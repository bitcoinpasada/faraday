//! Signing a spend's PSBT on this device: the press, the job on the worker
//! and its answer (`docs/WORKERS.md` §6, `docs/KEYS.md` §2).
//!
//! The refusals before a signature (the backup hold of F31, no randomness,
//! other amounts than when it was signed here, no key) are all on the main
//! thread, before the job starts. The job signs a copy of the PSBT with the
//! keys the store shares ([`faraday_keys::job::SignJob`]); its answer is
//! applied only while it is live and the spend still holds the PSBT the job
//! was given. Back or a lock meanwhile drops the answer: the PSBT stays as
//! it was, nothing goes to the Outbox, and the MuSig2 round that moved into
//! the job is dropped with its nonces (`docs/KEYS.md` §2).

pub(crate) mod read;

use crate::strings::EN;
use crate::strings::fill;
use crate::wallet::{self, Signed, fp_text};
use crate::worker::Job;
use crate::{Faraday, spend_opens_round};
use faraday_crypto::work::Compute;
use faraday_psbt::Psbt;

/// A signing to run on the worker: the job and what the press that started
/// it needs to finish.
pub(crate) struct SignWork {
    pub(crate) job: faraday_keys::job::SignJob,
    pub(crate) from: Origin,
}

/// A signing's answer.
pub(crate) struct SignDone {
    pub(crate) signing: faraday_keys::job::Signing,
    pub(crate) from: Origin,
}

/// The press the job belongs to.
pub(crate) struct Origin {
    /// The spend's PSBT as the job was given it: the answer is applied
    /// only to a spend that still holds exactly this.
    pub(crate) input: Vec<u8>,
    /// The spend's PSBT before the press began.
    pub(crate) before: Vec<u8>,
    /// Whether it is the MuSig2 aggregation after Collect, not Sign.
    pub(crate) collect: bool,
}

impl SignWork {
    /// Costs what `faraday_psbt::sign` costs.
    pub(crate) fn run(self, c: &Compute) -> SignDone {
        SignDone {
            signing: self.job.run(c),
            from: self.from,
        }
    }
}

/// Signs the open spend with the keys held here, on the worker. The
/// refusals come first and answer at once.
pub(crate) fn sign_here(app: &mut Faraday) {
    let c = app.compute();
    // A key made here signs nothing until its backup is done
    // (`docs/history/NEW-WALLET.md` §14.3).
    let held = app.held_keys_now(&c);
    let waiting: Vec<String> = app
        .wallets
        .spend()
        .map(|s| {
            s.inspection
                .participating_keys
                .iter()
                .filter(|fp| held.contains(fp))
                .map(|fp| fp_text(*fp))
                .collect()
        })
        .unwrap_or_default();
    if !waiting.is_empty() {
        if let Some(s) = app.wallets.spend_mut() {
            s.error = Some(fill(
                EN.sign_here_backup_first,
                &[&waiting.join(", ").to_string()],
            ));
            s.open = Some(wallet::step::SIGNERS);
            s.follow = true;
        }
        return;
    }
    // A MuSig2 round opened here draws its secret nonces
    // from a seed of this pass's own; a pass with no such input
    // draws nothing and needs none.
    let opens_round = app
        .wallets
        .spend()
        .is_some_and(|s| spend_opens_round(&s.inspection));
    let seed = match app.sign_seed() {
        Some(seed) => seed,
        None if opens_round => {
            if let Some(s) = app.wallets.spend_mut() {
                s.error = Some(EN.sign_here_no_randomness.to_string());
            }
            return;
        }
        None => zeroize::Zeroizing::new([0u8; 32]),
    };
    // The same transaction stating other amounts than when it was
    // signed here is refused, with no override (`docs/WALLETS.md` §3.3).
    if app
        .wallets
        .spend()
        .is_some_and(|s| app.amounts_refused(&s.spend.psbt))
    {
        if let Some(s) = app.wallets.spend_mut() {
            s.error = Some(EN.sign_here_this_transaction.to_string());
        }
        return;
    }
    let Some(s) = app.wallets.spend_mut() else {
        return;
    };
    let psbt = s.spend.psbt.clone();
    let before = s.spend.psbt.to_bytes();
    match app.session.sign_spend_job(&c, &psbt, &seed, s) {
        Ok(job) => sign_start(app, job, before, false),
        Err(e) => sign_end(app, &before, false, (psbt, Err(e))),
    }
}

/// Starts `job`, the signing of the open spend, whose PSBT was
/// `before` the press began.
pub(crate) fn sign_start(
    app: &mut Faraday,
    job: faraday_keys::job::SignJob,
    before: Vec<u8>,
    collect: bool,
) {
    let input = app
        .wallets
        .spend()
        .map(|s| s.spend.psbt.to_bytes())
        .unwrap_or_default();
    let from = Origin {
        input,
        before,
        collect,
    };
    app.start(
        Job::Sign(Box::new(SignWork { job, from })),
        EN.working_signing,
    );
}

/// A signing's answer. Dropped, with the round and the PSBT it holds,
/// where the spend is gone or holds another PSBT.
pub(crate) fn signed_spend(app: &mut Faraday, done: SignDone) {
    let SignDone { signing, from } = done;
    let Some(s) = app.wallets.spend_mut() else {
        return;
    };
    if s.spend.psbt.to_bytes() != from.input {
        return;
    }
    let result = crate::review::signed(&mut s.musig, signing);
    sign_end(app, &from.before, from.collect, result);
}

/// What a signing leaves on the open spend, as the press did when it
/// signed in place.
fn sign_end(
    app: &mut Faraday,
    before: &[u8],
    collect: bool,
    (psbt, signed): (Psbt, Result<Signed, String>),
) {
    let mut remember: Option<Psbt> = None;
    let mut nonce_only = false;
    let Some(s) = app.wallets.spend_mut() else {
        return;
    };
    // A session whose every nonce has signed is over: the next pass
    // over this transaction opens a new one, never the same nonce.
    if s.musig.as_ref().is_some_and(|m| m.is_empty()) {
        s.musig = None;
    }
    let signed = signed.map(|out| out.by);
    if collect {
        if signed.is_ok() {
            s.spend.psbt = psbt;
        }
        collect_end(app, before, None);
        // The part file sent before the job is not the answer of a
        // spend the aggregation completed.
        if let Some(s) = app.wallets.spend().filter(|s| s.complete) {
            let name = format!("{}-part.psbt", crate::result_stem(&s.spend.source));
            app.outbox.retain(|i| i.name != name);
            app.save_boxes();
        }
        return;
    }
    match signed {
        Ok(fps) if fps.is_empty() && s.musig.is_some() => {
            // MuSig2 round one: this device's nonce, and no signature.
            s.spend.psbt = psbt;
            s.error = None;
            nonce_only = true;
        }
        Ok(fps) => {
            remember = Some(psbt.clone());
            s.spend.psbt = psbt;
            s.spend.signed_here.extend(fps);
            s.done[5] = true;
            s.error = None;
        }
        Err(e) => s.error = Some(e),
    }
    if let Some(p) = remember {
        app.remember_amounts(&p);
        app.save_boxes();
    }
    if nonce_only {
        app.refresh_spend();
        app.part_out();
        if let Some(s) = app.wallets.spend_mut() {
            s.open = Some(wallet::step::NONCES);
            s.follow = true;
        }
        app.toast(EN.sign_here_this_device);
        return;
    }
    app.refresh_spend();
    if app
        .wallets
        .spend()
        .is_some_and(|s| s.spend.psbt.to_bytes() != before)
    {
        app.part_out();
    }
    if let Some(s) = app.wallets.spend_mut() {
        s.follow = true;
        if s.complete {
            s.done[wallet::step::COLLECT as usize] = true;
            s.open = Some(wallet::step::FINISH);
        } else if s.steps.contains(&wallet::step::COLLECT) {
            s.open = Some(wallet::step::COLLECT);
        } else {
            s.open = Some(wallet::step::FINISH);
        }
    }
}

/// Collect's MuSig2 aggregation: the merged PSBT is refreshed and sent to
/// the Outbox first, so a Back or a lock meanwhile leaves the spend as
/// consistent as a Collect with nothing to aggregate; the answer then
/// adds only what the aggregation changes.
pub(crate) fn collect_aggregate(app: &mut Faraday, job: faraday_keys::job::SignJob, before: &[u8]) {
    let merged = app
        .wallets
        .spend()
        .map(|s| s.spend.psbt.to_bytes())
        .unwrap_or_default();
    collect_end(app, before, None);
    sign_start(app, job, merged, true);
}

/// The end of Collect, once the MuSig2 aggregation after it (if any)
/// is done.
pub(crate) fn collect_end(app: &mut Faraday, before: &[u8], nothing: Option<&String>) {
    if let (Some(name), Some(s)) = (nothing, app.wallets.spend_mut()) {
        s.error = Some(fill(EN.collect_has_nothing, &[name]));
    }
    app.refresh_spend();
    if app
        .wallets
        .spend()
        .is_some_and(|s| s.spend.psbt.to_bytes() != before)
    {
        app.part_out();
    }
    if let Some(s) = app.wallets.spend_mut() {
        s.follow = true;
        if s.complete {
            s.done[wallet::step::COLLECT as usize] = true;
            s.open = Some(wallet::step::FINISH);
        }
    }
}
