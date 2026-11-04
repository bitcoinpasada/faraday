//! A lock's seals as jobs on the worker (`docs/VAULT.md` §5, `docs/WORKERS.md`
//! §6): one open vault at a time, since a plan's seal counts are written
//! from the vaults sealed before it. The front half draws the seal's
//! randomness and makes the job, which marks the vault sealing in the
//! store; the back half puts the file in the Outbox, closes the vault and
//! goes on to the next. A seal that fails keeps the vault open with its
//! changes and ends the lock. When the last is done a lock of every vault
//! finishes (the wipes), and a lock of one closes that vault and goes on
//! with what was waiting for it.

use std::collections::VecDeque;

use faraday_crypto::work::Compute;
use faraday_keys::job::{SealJob, Sealed};
use faraday_keys::vault::{SEAL_RANDOM_LEN, VaultId};
use zeroize::Zeroizing;

use super::errors;
use crate::strings::EN;
use crate::worker::Job;
use crate::{Command, Faraday};

/// The seals of a lock, waiting and running. Kept in [`super::Vaults`].
pub(crate) struct SealRun {
    /// Vaults not yet sealed, by id.
    queue: VecDeque<VaultId>,
    /// The vault whose job is out, with its file name.
    running: Option<(VaultId, String)>,
    /// A lock of every vault (which then finishes the lock), else of one.
    all: bool,
    /// What a lock of one goes on with when its vault is closed.
    after: After,
}

/// What waits for a lock of one vault to finish.
#[derive(Clone, Copy)]
pub(crate) enum After {
    Nothing,
    /// Make a vault from the one just locked, on this screen's behalf.
    CreateFrom(crate::Screen),
}

/// A vault's seal to run on the worker, with its file name.
pub(crate) struct SealWork {
    job: SealJob,
    id: VaultId,
    name: String,
}

/// A seal's answer: the file (encrypted, so public) or why there is none.
pub(crate) struct SealDone {
    sealed: Sealed,
    id: VaultId,
    name: String,
}

impl SealWork {
    /// Costs an Argon2id run and a document write ([`SealJob::run`]).
    pub(crate) fn run(self, c: &Compute) -> SealDone {
        SealDone {
            sealed: self.job.run(c),
            id: self.id,
            name: self.name,
        }
    }
}

/// Whether a lock's seals are under way.
pub(crate) fn sealing(app: &Faraday) -> bool {
    app.vaults.state.sealing.is_some()
}

/// Seals every open vault with changes into the Outbox, then finishes
/// the lock (`docs/history/PLAN.md` §5.3). A vault nobody changed or
/// that is read-only is not rewritten: a file's bytes change only when
/// there is something to keep, which is all a second copy can compare
/// (`docs/VAULT.md` §9). A vault whose seal cannot be made (the seed or
/// the answers have not arrived) is closed unsealed.
pub(crate) fn vault_seal_all(app: &mut Faraday) {
    // What each open vault holds, remembered across the lock.
    app.vault_summaries_refresh();
    // A held key's record says so before the vault goes
    // (`docs/history/NEW-WALLET.md` §14.3).
    app.held_sweep();
    let queue = app.vaults.open.iter().map(|o| o.id).collect();
    app.vaults.state.sealing = Some(SealRun {
        queue,
        running: None,
        all: true,
        after: After::Nothing,
    });
    seal_next(app);
}

/// Locks open vault `i` alone: sealed into the Outbox when changed,
/// what it holds remembered, and closed. The rest stay open. A
/// changed vault stays open, its changes kept, when the seed has not
/// arrived or its changes do not fit: then this returns false. It
/// also returns false while the seal runs on the worker: the vault is
/// closed when it is done. `after` is done once the vault is closed,
/// at once or when the seal ends; never when it stays open.
pub(crate) fn vault_lock_one(app: &mut Faraday, i: usize, after: After) -> bool {
    if sealing(app) {
        return false;
    }
    let Some(v) = app.vaults.open.get(i) else {
        return true;
    };
    let (id, changed) = (v.id, v.changes > 0);
    if changed && !app.seeded() {
        app.out.command(faraday_shell_api::Command::RequestEntropy);
        app.toast(EN.seed_missing);
        return false;
    }
    let c = app.compute();
    if let Some(why) = app.vaults.cannot_seal(&c, i) {
        app.toast(&why);
        return false;
    }
    app.vault_summaries_refresh();
    app.held_sweep();
    app.vaults.state.sealing = Some(SealRun {
        queue: VecDeque::from([id]),
        running: None,
        all: false,
        after,
    });
    seal_next(app);
    !sealing(app)
}

/// The next vault of the queue: its plans' counts written, the
/// signed-amount memory added, and its job started. The lock ends
/// when the queue is empty. Re-entered by the back half, which an
/// inline worker runs before `start` returns.
fn seal_next(app: &mut Faraday) {
    loop {
        let Some(run) = app.vaults.state.sealing.as_mut() else {
            return;
        };
        let all = run.all;
        let Some(id) = run.queue.pop_front() else {
            return seal_end(app);
        };
        let Some(i) = app.vaults.open.iter().position(|o| o.id == id) else {
            continue;
        };
        // Its plans take the counts of the vaults sealed before it
        // (`docs/VAULT-HARDENING.md` §5.6).
        let c = app.compute();
        app.vaults.seals_write(&c, i);
        seal_memory(app, i);
        let read_only = app.vaults.store().read_only(id).is_some();
        if read_only || app.vaults.open[i].changes == 0 {
            if !all {
                seal_close(app, id);
            }
            continue;
        }
        let Some(work) = seal_work(app, id) else {
            if !all {
                seal_stop(app);
            }
            continue;
        };
        let line = EN.vault_sealing;
        if let Some(run) = app.vaults.state.sealing.as_mut() {
            run.running = Some((id, work.name.clone()));
        }
        app.start(Job::Seal(Box::new(work)), line);
        return;
    }
}

/// The signed-amount memory goes into open vault `i`, the oldest
/// records leaving first when the file is full (`docs/VAULT.md` §7,
/// type 10).
fn seal_memory(app: &mut Faraday, i: usize) {
    if !app.memory.seals() {
        return;
    }
    let Some(id) = app.vaults.open.get(i).map(|o| o.id) else {
        return;
    };
    let have: Vec<[u8; 32]> = app
        .vaults
        .records(id)
        .of(faraday_vault::records::kind::AMOUNTS)
        .filter_map(|(_, r)| crate::memory::from_record(r).map(|e| e.0))
        .collect();
    let new = app.memory.records_lacking(&have);
    let c = app.compute();
    app.vaults.state.add_amounts(&c, i, new);
}

/// The front half: open vault `id`'s seal made into a job, which takes
/// the vault's job. `None` where the session's answers
/// have not arrived (and the shell is asked), or the store gave no job.
fn seal_work(app: &mut Faraday, id: VaultId) -> Option<SealWork> {
    if !app.seeded() {
        app.out.command(faraday_shell_api::Command::RequestEntropy);
        return None;
    }
    let n = app.vaults.store().uuid_bytes(id)?;
    let random = app.fresh_bytes(b"vault seal", SEAL_RANDOM_LEN)?;
    let mut seal = Zeroizing::new([0u8; SEAL_RANDOM_LEN]);
    seal.copy_from_slice(&random);
    let uuids = if n == 0 {
        Zeroizing::new(Vec::new())
    } else {
        app.fresh_bytes(b"vault entries", n)?
    };
    let now = app.clock_now();
    let name = app.vaults.open.iter().find(|o| o.id == id)?.name.clone();
    let job = app.vaults.state.store.seal_job(id, &seal, &uuids, now)?;
    Some(SealWork { job, id, name })
}

/// A lock of one that could not seal: the vault stays open.
fn seal_stop(app: &mut Faraday) {
    app.vaults.state.sealing = None;
    app.toast(EN.seed_missing);
}

/// Closes open vault `id` after a lock of it alone.
fn seal_close(app: &mut Faraday, id: VaultId) {
    if let Some(i) = app.vaults.open.iter().position(|o| o.id == id) {
        app.vaults.state.close(i);
    }
    app.vaults.state.current = 0;
    app.vaults.state.item_open = false;
    app.vaults.state.form = None;
}

/// The queue is empty: a lock of every vault closes them and goes on
/// with the wipes; a lock of one is done.
fn seal_end(app: &mut Faraday) {
    let Some(run) = app.vaults.state.sealing.take() else {
        return;
    };
    if run.all {
        app.vaults.state.close_all();
        app.vaults.state.load = None;
        app.vaults.state.pass_seeds = None;
        lock_finish(app);
    } else if let After::CreateFrom(screen) = run.after {
        app.vault_act(super::VaultAction::CreateFrom(screen));
    }
}

/// The back half: the file to the Outbox, and the next vault.
pub(crate) fn sealed(app: &mut Faraday, done: SealDone) {
    let SealDone { sealed, id, name } = done;
    let all = match app.vaults.state.sealing.as_mut() {
        Some(run) => {
            run.running = None;
            run.all
        }
        // A lock ended the run: nothing is left to go on with.
        None => return,
    };
    match sealed.into_file() {
        Ok(bytes) => {
            app.put_outbox(&name, bytes.clone());
            if let Some(i) = app.vaults.open.iter().position(|o| o.id == id) {
                let c = app.compute();
                app.vaults.seals_sealed(&c, i, &bytes);
                // The keys waiting for their copies that this sealing holds.
                app.flow_sealed(i, &name, &bytes);
            }
            app.vaults.state.store.seal_ended(id, true);
        }
        // The vault stays open with its changes, the lock goes no
        // further (the rest of the queue is not sealed, nothing is
        // wiped), and the person is told.
        Err(why) => {
            app.vaults.state.store.seal_ended(id, false);
            app.vaults.state.sealing = None;
            app.out.command(Command::Draw);
            app.toast(&errors::error_text(&why));
            return;
        }
    }
    if !all {
        seal_close(app, id);
    }
    seal_next(app);
}

/// The rest of a lock, after the last seal: the wipes.
fn lock_finish(app: &mut Faraday) {
    // What the boot stick brought and was not imported is not kept:
    // its bytes are wiped as it drops.
    app.import.clear_all();
    // Nothing secret copied in outlives the process that read it
    // (`docs/history/PLAN.md` §5.3): the next process is clean, and these would be
    // in its memory and in `/run/faraday`. They are still on the stick
    // they came from.
    // A picture goes as what its codes hold, and as Other when none read.
    app.inbox.retain(|i| !i.dropped_at_lock());
    app.session.wipe();
    app.own_wipe();
    app.wallets.clear_spend();
    app.backup.close();
    app.message.clear();
    app.create.clear();
    app.restore.clear();
    app.keygen.forget();
    app.wordlist.clear();
    app.bip85.clear();
    app.explore.clear();
    app.secrets.clear();
    app.entry.reset();
    app.save_boxes_for_lock();
    app.restart = true;
    app.out.commands.push_back(Command::Exit);
}
