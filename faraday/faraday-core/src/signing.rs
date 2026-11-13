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

pub(crate) mod collect;
pub(crate) mod fresh;
pub(crate) mod read;

use crate::strings::EN;
use crate::strings::fill;
use crate::wallet::{self, fp_text};
use crate::worker::Job;
use crate::{Faraday, spend_opens_round};
use faraday_crypto::work::Compute;
use faraday_psbt::Psbt;
use zeroize::Zeroizing;

/// A signing to run on the worker: the job, what the press that started
/// it needs to finish, and what the spend needs worked out after it.
pub(crate) struct SignWork {
    pub(crate) job: faraday_keys::job::SignJob,
    pub(crate) from: Origin,
    after: After,
}

/// What a signing's answer is worked out against: a copy of the session
/// ([`crate::wallet::Session::job_copy`]), the spend as it was, and the part
/// file's name and the bytes waiting under it in the Outbox.
struct After {
    session: wallet::Session,
    spend: wallet::Spend,
    part: (String, bool),
    waiting: Option<Zeroizing<Vec<u8>>>,
}

/// A signing's answer.
pub(crate) struct SignDone {
    /// The MuSig2 round as it stands, with its nonces.
    round: Option<faraday_psbt::MusigSession>,
    /// What the signing left and what was worked out of it, or the error
    /// text.
    worked: Result<Worked, String>,
    pub(crate) from: Origin,
}

/// The spend a signing left, worked out on the worker.
struct Worked {
    spend: wallet::Spend,
    fresh: fresh::Fresh,
    part: Option<collect::Planned>,
    /// The pass only opened a MuSig2 round: a nonce and no signature.
    nonce_only: bool,
    /// The wallets as the job left them, for what they kept.
    wallets: Vec<wallet::Wallet>,
}

/// The press the job belongs to.
pub(crate) struct Origin {
    /// The spend's PSBT as the job was given it: the answer is applied
    /// only to a spend that still holds exactly this.
    pub(crate) input: Psbt,
    /// The spend's PSBT before the press began.
    pub(crate) before: Vec<u8>,
}

impl SignWork {
    /// Costs what `faraday_psbt::sign`, `worked_out` and `part_plan` cost.
    /// Signs, and where it did, leaves the signature on the copy of the
    /// spend (before anything reads it), works the spend out and plans the
    /// part file, as the press did on the main thread.
    pub(crate) fn run(self, c: &Compute) -> SignDone {
        let Self { job, from, after } = self;
        let signing = job.run(c);
        let view = signing.round_view();
        let (psbt, round, result) = signing.into_parts();
        let worked = match result {
            Err(e) => Err(String::from(&*crate::review::sign_error_text(&e))),
            Ok(r) => {
                let fps = crate::review::signers_of(&r);
                let After {
                    mut session,
                    mut spend,
                    part: (name, recorded),
                    waiting,
                } = after;
                let nonce_only = fps.is_empty() && view.is_some();
                spend.take_signed(psbt, fps, r.nonces_written, nonce_only);
                let fresh = fresh::worked_out(c, &session, &mut spend, view.as_ref(), true);
                let part = (nonce_only || spend.psbt.to_bytes() != from.before).then(|| {
                    collect::Planned {
                        plan: part_plan(
                            c,
                            &spend,
                            fresh.complete,
                            (&name, recorded),
                            waiting.as_ref().map(|w| w.as_slice()),
                        ),
                        name,
                        waiting,
                    }
                });
                // No key's `Arc` rides in the answer: the keys go before it does.
                let wallets = std::mem::take(&mut session.wallets);
                drop(session);
                Ok(Worked {
                    spend,
                    fresh,
                    part,
                    nonce_only,
                    wallets,
                })
            }
        };
        SignDone {
            round,
            worked,
            from,
        }
    }
}

/// Signs the open spend with the keys held here, on the worker. The
/// refusals come first and answer at once.
///
/// Every route here requires the steps before Sign done: if one is not,
/// the first of them opens and nothing is signed. The press that only
/// opens a MuSig2 round (Go first, on the Nonces card) requires the
/// steps before Nonces.
pub(crate) fn sign_here(app: &mut Faraday) {
    // The checks below read the inspection the job will sign from, not
    // one made before the session changed.
    app.refresh_spend();
    if app.wallets.stop_at_undone_step() {
        return;
    }
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
        app.wallets.signers_shown_with(fill(
            EN.sign_here_backup_first,
            &[&waiting.join(", ").to_string()],
        ));
        return;
    }
    // A MuSig2 round opened here draws its secret nonces
    // from a seed of this pass's own; a pass with no such input
    // draws nothing and needs none.
    let opens_round = app
        .wallets
        .spend()
        .is_some_and(|s| spend_opens_round(&s.inspection));
    let seed = app.sign_seed();
    if seed.is_none() && opens_round {
        app.wallets.set_spend_error(EN.no_randomness.to_string());
        return;
    }
    // The same transaction stating other amounts than when it was
    // signed here is refused, with no override (`docs/WALLETS.md` §3.3).
    if app
        .wallets
        .spend()
        .is_some_and(|s| app.amounts_refused(&s.spend.psbt))
    {
        app.wallets
            .set_spend_error(EN.sign_here_this_transaction.to_string());
        return;
    }
    let Some((made, before)) = app.wallets.sign_job(&app.session, &c, seed.as_ref()) else {
        return;
    };
    match made {
        Ok(job) => sign_start(app, job, before),
        Err(e) => sign_end(app, &before, Err(e)),
    }
}

/// Starts `job`, the signing of the open spend, whose PSBT was
/// `before` the press began.
pub(crate) fn sign_start(app: &mut Faraday, job: faraday_keys::job::SignJob, before: Vec<u8>) {
    let Some(s) = app.wallets.spend() else {
        return;
    };
    let (part, recorded) = part_name(s);
    let waiting = app
        .outbox
        .iter()
        .find(|i| i.name == part)
        .map(|i| i.bytes.clone());
    let from = Origin {
        input: s.spend.psbt.clone(),
        before,
    };
    let after = After {
        session: app.session.job_copy(),
        spend: s.spend.clone(),
        part: (part, recorded),
        waiting,
    };
    app.start(
        Job::Sign(Box::new(SignWork { job, from, after })),
        EN.working_signing,
    );
}

/// A signing's answer. Dropped, with the round and the PSBT it holds,
/// where the spend is gone or holds another PSBT.
pub(crate) fn signed_spend(app: &mut Faraday, done: SignDone) {
    let SignDone {
        round,
        worked,
        from,
    } = done;
    if !app.wallets.keep_round(&from.input, round) {
        return;
    }
    sign_end(app, &from.before, worked);
}

/// What a signing leaves on the open spend, as the press did when it
/// signed in place: the answer worked out on the worker is moved in.
fn sign_end(app: &mut Faraday, before: &[u8], worked: Result<Worked, String>) {
    let mut remember: Option<Psbt> = None;
    let mut nonce_only = false;
    let mut part = None;
    if !app.wallets.end_spent_round() {
        return;
    }
    match worked {
        Ok(w) => {
            nonce_only = w.nonce_only;
            remember = app.wallets.take_signed(w.spend, w.fresh, nonce_only);
            part = w.part;
            for (kept, theirs) in app.session.wallets.iter().zip(w.wallets) {
                kept.take_kept(theirs);
            }
        }
        Err(e) => app.wallets.set_spend_error(e),
    }
    if let Some(p) = remember {
        app.remember_amounts(&p);
        app.save_boxes();
    }
    if nonce_only {
        app.refresh_spend();
        part_put(app, part);
        app.wallets.open_step_followed(wallet::step::NONCES);
        app.toast(EN.sign_here_this_device);
        return;
    }
    app.refresh_spend();
    if let Some(planned) = part {
        part_put(app, Some(planned));
    } else if app
        .wallets
        .spend()
        .is_some_and(|s| s.spend.psbt.to_bytes() != before)
    {
        part_out(app);
    }
    app.wallets.open_after_sign();
}

/// Costs what `part_out` costs where the Outbox changed under the job. The end of
/// Collect on the main thread, the job's answer kept: the refresh finds
/// the spend worked out already, and the part file the job planned is put
/// by moves alone, unless the Outbox no longer holds what the plan was
/// made against, which is worked out again here.
pub(crate) fn collect_end(
    app: &mut Faraday,
    nothing: Option<&String>,
    part: Option<collect::Planned>,
) {
    if let Some(name) = nothing {
        app.wallets.collect_found_nothing(name);
    }
    app.refresh_spend();
    part_put(app, part);
    app.wallets.open_after_collect();
}

/// Costs what `part_out` costs where the Outbox changed under the job. Puts
/// the part file a job planned, by moves alone, unless the Outbox no longer
/// holds what the plan was made against, which is worked out again here.
fn part_put(app: &mut Faraday, part: Option<collect::Planned>) {
    let Some(planned) = part else {
        return;
    };
    let kept = app.wallets.spend().is_some_and(|s| {
        let (name, _) = part_name(s);
        let waiting = app.outbox.iter().find(|i| i.name == name);
        name == planned.name
            && waiting.map(|w| w.bytes.as_slice()) == planned.waiting.as_ref().map(|w| w.as_slice())
    });
    if kept {
        part_apply(app, planned.plan);
    } else {
        part_out(app);
    }
}

/// The file a spend puts its part under, and whether it put one before:
/// the name recorded, else the fixed one. A lookup.
pub(crate) fn part_name(s: &wallet::SpendState) -> (String, bool) {
    match &s.out_names.part {
        Some(name) => (name.clone(), true),
        None => (
            format!("{}-part.psbt", crate::result_stem(&s.spend.source).as_str()),
            false,
        ),
    }
}

/// What the part file of a spend becomes ([`part_plan`]), for [`part_apply`]
/// to do with moves only.
pub(crate) enum PartPlan {
    /// A spend that needs no more: the file of this name is taken back from
    /// the Outbox, if it is there.
    Take(String),
    /// A spend that needs no more, whose file is not this spend's to take.
    Keep,
    /// The PSBT could not be written.
    NotWritten,
    /// The same transaction as the file waiting under the name: replaces it.
    Replace(String, Box<crate::Item>),
    /// A different transaction, or none waiting: beside any file of the name.
    Beside(crate::visit_taken::Beside),
}

/// Costs a serialisation of the PSBT and a decode of it and of the file
/// waiting, and what `beside_plan` costs. What goes to the Outbox for the
/// stick for the other signers while more are needed (the PSBT with what
/// this pass added), or comes back out of it once the spend is `complete`.
/// `name` is the file this spend put before (`recorded`), else the fixed
/// name; `waiting` is the bytes of the Outbox file of that name, if any.
pub(crate) fn part_plan(
    c: &Compute,
    spend: &wallet::Spend,
    complete: bool,
    (name, recorded): (&str, bool),
    waiting: Option<&[u8]>,
) -> PartPlan {
    if complete {
        // The part file this spend sent is not the answer of a spend that
        // needs no more: it is taken back. Another file of its name stays.
        // A spend opened again has no name recorded, and the Outbox outlives
        // it: the fixed name goes only if it holds this transaction.
        let ours = recorded
            || waiting
                .is_some_and(|w| unsigned_tx_of(c, w).as_ref() == Some(spend.psbt.unsigned_tx()));
        return if ours {
            PartPlan::Take(name.to_string())
        } else {
            PartPlan::Keep
        };
    }
    let Ok(bytes) = spend.out_bytes() else {
        return PartPlan::NotWritten;
    };
    // A pass of the same transaction replaces the file waiting; a
    // different transaction of the name goes beside it.
    let waiting_tx = waiting.and_then(|w| unsigned_tx_of(c, w));
    let same = waiting_tx.is_some() && waiting_tx == unsigned_tx_of(c, &bytes);
    if same {
        PartPlan::Replace(name.to_string(), Box::new(crate::Item::new(c, name, bytes)))
    } else {
        PartPlan::Beside(crate::visit_taken::beside_plan(
            c,
            name,
            Zeroizing::new(bytes),
            waiting,
        ))
    }
}

/// Moves only. Does what [`part_plan`] made: the Outbox's file put, replaced
/// or taken back, and the name the spend sent it under recorded.
pub(crate) fn part_apply(app: &mut Faraday, plan: PartPlan) {
    // A spend held back by the nonce history gives out no file for the
    // next signer, as it saves and shows nothing else.
    if app.wallets.spend().is_some_and(|s| s.spend.nonce_hold)
        && matches!(plan, PartPlan::Replace(..) | PartPlan::Beside(_))
    {
        return;
    }
    let name = match plan {
        PartPlan::Take(name) => {
            if app.outbox.iter().any(|i| i.name == name) {
                app.outbox.retain(|i| i.name != name);
                app.save_boxes();
            }
            return;
        }
        PartPlan::Keep => return,
        PartPlan::NotWritten => return app.toast(EN.spend_v2_not_written),
        PartPlan::Replace(name, item) => {
            app.put_item(*item);
            name
        }
        PartPlan::Beside(plan) => String::from(&*app.beside_apply(plan)),
    };
    app.wallets.set_part_name(name);
}

/// Costs what `part_plan` costs. The part file of the open spend, worked
/// out and put in a row.
pub(crate) fn part_out(app: &mut Faraday) {
    let c = app.compute();
    let Some(s) = app.wallets.spend() else {
        return;
    };
    let (name, recorded) = part_name(s);
    let waiting = app.outbox.iter().find(|i| i.name == name);
    let plan = part_plan(
        &c,
        &s.spend,
        s.complete,
        (&name, recorded),
        waiting.map(|w| w.bytes.as_slice()),
    );
    part_apply(app, plan);
}

/// Costs a decode of a PSBT. The unsigned transaction a PSBT file (either
/// version) carries, `None` for bytes that are not one.
fn unsigned_tx_of(c: &Compute, bytes: &[u8]) -> Option<faraday_psbt::bitcoin::Transaction> {
    if let Ok(p) = faraday_psbt::Psbt::parse_bytes(c, bytes) {
        return Some(p.unsigned_tx().clone());
    }
    faraday_psbt::v2::PsbtV2::parse_bytes(c, bytes)
        .ok()?
        .unsigned_tx()
        .ok()
}
