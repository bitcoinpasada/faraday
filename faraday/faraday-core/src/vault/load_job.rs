//! The seeds a Load sheet press loads are stretched as a job on the worker
//! (`docs/WORKERS.md` §2.1, §6): each key's BIP-39 stretch, its master key
//! and its id. The front half reads the open vault's records into a
//! faraday-keys [`KeysJob`], which holds the words; the back half takes the
//! keys it worked out, on the main thread, adds them to the session and adds
//! the wallets that were ticked with them.

use std::collections::BTreeSet;

use faraday_crypto::work::Compute;
use faraday_keys::job::{Derived, KeysJob, KeysLoaded};
use faraday_keys::vault::VaultId;

use crate::Faraday;
use crate::strings::EN;
use crate::worker::Job;

/// A wallet to add after the keys: its description and its name. Public.
pub(crate) type LoadWallet = (faraday_bip::policy::WalletPolicy, String);

/// What a load goes on with once its keys and wallets are added: the
/// counts of the keys that joined and the wallets that did.
pub(crate) type Then = Box<dyn FnOnce(&mut Faraday, usize, usize) + Send>;

/// A load of seeds to run on the worker.
pub(crate) struct LoadKeysWork {
    job: KeysJob,
    plan: Plan,
}

/// What the load is for, kept with its job and handed back with the
/// answer.
struct Plan {
    /// The open vault's place and id, to find the sheet still for it.
    v: usize,
    id: VaultId,
    wallets: Vec<LoadWallet>,
    /// The answer is for the Load sheet of the vault, which must be up.
    sheet: bool,
    then: Then,
}

/// A load's answer, with what it was for.
pub(crate) struct LoadKeysDone {
    loaded: KeysLoaded,
    plan: Plan,
}

impl LoadKeysWork {
    /// Costs what [`KeysJob::run`] costs.
    pub(crate) fn run(self, c: &Compute) -> LoadKeysDone {
        LoadKeysDone {
            loaded: self.job.run(c),
            plan: self.plan,
        }
    }
}

/// Costs what `vault_load_front` costs. A press of Load ticked on the sheet
/// of open vault `v`: its seeds are stretched in a job, and its wallets
/// are added when the answer comes. A load of wallets alone has nothing
/// to stretch and is done at once.
pub(crate) fn start(app: &mut Faraday, v: usize, set: &BTreeSet<usize>) {
    let then: Then = Box::new(move |app, keys, wallets| app.load_sheet_loaded(v, keys, wallets));
    begin(app, v, set, Kind::Sheet, then);
}

/// Costs what [`start`] costs. A load of the records `set` names of open
/// vault `v`, then `then` with the counts, on the main thread: at once for
/// wallets alone, else when the job answers (a stale answer is dropped,
/// `then` never run). Nothing starts of a vault for a press run again after its
/// load of it ([`replay`]): it finds what the first load left.
pub(crate) fn start_then(app: &mut Faraday, v: usize, set: &BTreeSet<usize>, then: Then) {
    begin(app, v, set, Kind::Plain, then);
}

/// Costs what [`start`] costs. As [`start_then`] with `then` run first,
/// and the press being acted on run again after it, with what the load
/// left: `true` when a job started, and the press stops where it is;
/// `false`: nothing started, the press goes on. A press that is not
/// being acted on ends with `then` alone.
pub(crate) fn start_replay(app: &mut Faraday, v: usize, set: &BTreeSet<usize>, then: Then) -> bool {
    let Some(action) = app.vaults.state.acting else {
        return begin(app, v, set, Kind::Plain, then);
    };
    // The vaults this press has loaded, this one with them: the press run
    // again loads none of them a second time.
    let mut seen = app.vaults.state.replayed.clone();
    seen.push(v);
    let both: Then = Box::new(move |app, keys, wallets| {
        then(app, keys, wallets);
        replay(app, action, seen);
    });
    begin(app, v, set, Kind::Replay, both)
}

/// Costs what the press costs, run again with its seeds loaded: runs
/// `action` again, its load done: a load it asks for of a vault in
/// `seen` starts no job.
fn replay(app: &mut Faraday, action: crate::Action, seen: Vec<usize>) {
    let was = std::mem::replace(&mut app.vaults.state.replayed, seen);
    app.act(action);
    app.vaults.state.replayed = was;
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Sheet,
    Plain,
    Replay,
}

/// Costs the store read into a job, and for a load of wallets alone their
/// policies added. Whether a job was started.
fn begin(app: &mut Faraday, v: usize, set: &BTreeSet<usize>, kind: Kind, then: Then) -> bool {
    if app.vaults.state.replayed.contains(&v) {
        return false;
    }
    let c = app.compute();
    let Some((mut job, wallets)) = app.vault_load_front(&c, v, set) else {
        return false;
    };
    let Some(id) = app.vaults.open.get(v).map(|o| o.id) else {
        return false;
    };
    for (policy, _) in &wallets {
        job.check_at(policy);
    }
    if job.is_empty() {
        let n = app.vault_load_wallets(&c, wallets, &Derived::default());
        if kind != Kind::Replay {
            then(app, 0, n);
        }
        return false;
    }
    let work = LoadKeysWork {
        job,
        plan: Plan {
            v,
            id,
            wallets,
            sheet: kind == Kind::Sheet,
            then,
        },
    };
    app.vaults.state.loads_started = app.vaults.state.loads_started.wrapping_add(1);
    app.start(Job::KeysLoad(Box::new(work)), EN.working_load_seeds);
    true
}

/// The load's answer, on the main thread. Dropped, and wiped, where the
/// vault is no longer open as it was, or the sheet is no longer up for
/// the vault it was started on.
pub(crate) fn loaded(app: &mut Faraday, done: LoadKeysDone) {
    let LoadKeysDone { loaded, plan } = done;
    let c = app.compute();
    let still = (!plan.sheet || app.vaults.load_sheet_vault() == Some(plan.v))
        && app.vaults.open.get(plan.v).map(|o| o.id) == Some(plan.id);
    if !still {
        return;
    }
    let (added, derived, refused) = app.session.keys.adopt_loaded_derived(&c, loaded);
    app.vaults.state.key_refused = refused.first().map(|e| e.text());
    let wallets = app.vault_load_wallets(&c, plan.wallets, &derived);
    (plan.then)(app, added.len(), wallets);
}
