//! Collect on the open spend: the signatures another device put on a copy
//! of the transaction, added and checked on the worker (`docs/WORKERS.md`
//! §2.1, §6).
//!
//! The job is given the file's bytes, a copy of the spend, the MuSig2
//! round's public view (no nonce) and a copy of the session
//! ([`crate::wallet::Session::job_copy`]): the keys' masters shared the way
//! a signing job shares them, and the wallets with what they kept. It runs
//! the functions the main thread ran, and its answer brings back the spend,
//! what was worked out of it ([`Fresh`]) and the part file's plan. The copy
//! of the session, and the keys with it, is dropped on the worker before the
//! answer is sent. Back or a lock meanwhile drops the answer, and so does
//! a spend that no longer holds the PSBT the job was given: nothing is
//! merged then.

use super::fresh::{Fresh, worked_out};
use super::{PartPlan, part_name, part_plan};
use crate::strings::EN;
use crate::wallet::{Session, Spend};
use crate::worker::Job;
use crate::{Faraday, wallet::Wallet};
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_psbt::{MusigSessionView, Psbt};

/// A Collect to run on the worker, owned.
pub(crate) struct CollectWork {
    /// The Inbox file's bytes and name.
    bytes: Vec<u8>,
    name: String,
    spend: Spend,
    /// The MuSig2 round's public facts, when the spend holds a round.
    view: Option<MusigSessionView>,
    /// The spend was not worked out for the session as it is.
    stale: bool,
    session: Session,
    /// The part file's name, whether this spend put one before, and the
    /// bytes waiting in the Outbox under it.
    part: (String, bool),
    waiting: Option<Vec<u8>>,
    /// Came while the Spend tab's spend is under way.
    spend_backup: bool,
}

/// The part file's plan and what it was made against.
pub(crate) struct Planned {
    pub(crate) plan: PartPlan,
    pub(crate) name: String,
    pub(crate) waiting: Option<Vec<u8>>,
}

/// A Collect's answer.
pub(crate) struct CollectDone {
    /// The spend's PSBT as the job was given it: the answer is applied
    /// only to a spend that still holds exactly this.
    input: Psbt,
    spend: Spend,
    /// The file's name, when it held nothing the spend did not have.
    nothing: Option<String>,
    /// What the merge refused, if it did.
    error: Option<String>,
    fresh: Option<Fresh>,
    part: Option<Planned>,
    spend_backup: bool,
    /// The wallets as the job left them, for what they kept.
    wallets: Vec<Wallet>,
}

impl CollectWork {
    /// Costs what `Spend::collect`, `worked_out` and `part_plan` cost. The
    /// file merged into the spend, the spend worked out anew where it
    /// changed or the session did, and the part file planned, as the press
    /// did on the main thread.
    pub(crate) fn run(self, c: &Compute) -> CollectDone {
        let Self {
            bytes,
            name,
            mut spend,
            view,
            stale,
            mut session,
            part: (part_name, recorded),
            waiting,
            spend_backup,
        } = self;
        let input = spend.psbt.clone();
        let (added, error) = match spend.collect(c, &session, &bytes, &name) {
            Ok(added) => (added, None),
            Err(e) => (Vec::<Fingerprint>::new(), Some(e)),
        };
        let merged = spend.psbt != input;
        let nothing = (error.is_none() && added.is_empty() && !merged).then_some(name);
        let fresh =
            (merged || stale).then(|| worked_out(c, &session, &mut spend, view.as_ref(), true));
        let part = fresh
            .as_ref()
            .filter(|_| spend.psbt != input)
            .map(|f| Planned {
                plan: part_plan(
                    c,
                    &spend,
                    f.complete,
                    (&part_name, recorded),
                    waiting.as_deref(),
                ),
                name: part_name.clone(),
                waiting: waiting.clone(),
            });
        // No key's `Arc` rides in the answer: the keys go before it does.
        let wallets = std::mem::take(&mut session.wallets);
        drop(session);
        CollectDone {
            input,
            spend,
            nothing,
            error,
            fresh,
            part,
            spend_backup,
            wallets,
        }
    }
}

/// Starts the Collect of Files' PSBT `index` into the open spend, on the
/// worker.
pub(crate) fn start_collect(app: &mut Faraday, index: usize) {
    start(app, index, false);
}

/// Starts the Collect of a PSBT that came while the Spend tab's spend is
/// under way: the tab's open card follows the spend's when it ends.
pub(crate) fn start_collect_on_spend_backup(app: &mut Faraday, index: usize) {
    start(app, index, true);
}

fn start(app: &mut Faraday, index: usize, spend_backup: bool) {
    let Some(item) = app.inbox.get(index) else {
        return;
    };
    let (bytes, name) = (item.bytes.clone(), item.name.clone());
    let Some(s) = app.wallets.spend() else {
        return;
    };
    let inputs = app.session.inspect_inputs();
    let (part, recorded) = part_name(s);
    let waiting = app
        .outbox
        .iter()
        .find(|i| i.name == part)
        .map(|i| i.bytes.clone());
    let work = CollectWork {
        bytes,
        name,
        spend: s.spend.clone(),
        view: s.musig.as_ref().map(|m| m.view()),
        stale: !s.inspected_for(&inputs),
        session: app.session.job_copy(),
        part: (part, recorded),
        waiting,
        spend_backup,
    };
    app.start(Job::Collect(Box::new(work)), EN.working_collecting);
}

/// Costs what `part_out` costs where the Outbox changed under the job. A
/// Collect's answer: merged into the open spend and ended as the press
/// ended. Dropped where the spend is gone or holds another PSBT than the
/// job was given.
pub(crate) fn collected(app: &mut Faraday, done: CollectDone) {
    let CollectDone {
        input,
        spend,
        nothing,
        error,
        fresh,
        part,
        spend_backup,
        wallets,
    } = done;
    let Some(s) = app.wallets.spend_mut() else {
        return;
    };
    if s.spend.psbt != input {
        return;
    }
    for (kept, theirs) in app.session.wallets.iter().zip(wallets) {
        kept.take_kept(theirs);
    }
    s.take_spend(spend);
    // `nothing` leaves the error as it was; the end says it.
    if error.is_some() {
        s.error = error;
    } else if nothing.is_none() {
        s.error = None;
    }
    if let Some(f) = fresh {
        s.take(f);
    }
    super::collect_end(app, nothing.as_ref(), part);
    if spend_backup {
        app.spend_backup_step_moved(false);
    }
}
