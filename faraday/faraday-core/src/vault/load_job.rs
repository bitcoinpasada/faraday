//! The seeds a Load sheet press loads are stretched as a job on the worker
//! (`docs/WORKERS.md` §2.1, §6): each key's BIP-39 stretch, its master key
//! and its id. The front half reads the open vault's records into a
//! faraday-keys [`KeysJob`], which holds the words; the back half takes the
//! keys it worked out, on the main thread, adds them to the session and adds
//! the wallets that were ticked with them.

use std::collections::BTreeSet;

use faraday_crypto::work::Compute;
use faraday_keys::job::{KeysJob, KeysLoaded};
use faraday_keys::vault::VaultId;

use crate::Faraday;
use crate::strings::EN;
use crate::worker::Job;

/// A wallet to add after the keys: its description and its name. Public.
pub(crate) type LoadWallet = (faraday_bip::policy::WalletPolicy, String);

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
    let c = app.compute();
    let Some((job, wallets)) = app.vault_load_front(&c, v, set) else {
        return;
    };
    if job.is_empty() {
        let n = app.vault_load_wallets(&c, wallets);
        app.load_sheet_loaded(v, 0, n);
        return;
    }
    let Some(id) = app.vaults.open.get(v).map(|o| o.id) else {
        return;
    };
    let work = LoadKeysWork {
        job,
        plan: Plan { v, id, wallets },
    };
    app.start(Job::KeysLoad(Box::new(work)), EN.working_load_seeds);
}

/// The load's answer, on the main thread. Dropped, and wiped, where the
/// sheet is no longer up for the vault it was started on.
pub(crate) fn loaded(app: &mut Faraday, done: LoadKeysDone) {
    let LoadKeysDone { loaded, plan } = done;
    let c = app.compute();
    let still = app.vaults.load_sheet_vault() == Some(plan.v)
        && app.vaults.open.get(plan.v).map(|o| o.id) == Some(plan.id);
    if !still {
        return;
    }
    let keys = app.session.keys.adopt_loaded(&c, loaded).len();
    let wallets = app.vault_load_wallets(&c, plan.wallets);
    app.load_sheet_loaded(plan.v, keys, wallets);
}
