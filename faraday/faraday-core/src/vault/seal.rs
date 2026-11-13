//! A lock's seals as jobs on the worker (`docs/VAULT.md` §5, `docs/WORKERS.md`
//! §6): one open vault at a time, since a plan's seal counts are written
//! from the vaults sealed before it. The front half draws the seal's
//! randomness and makes the job, which marks the vault sealing in the
//! store; the back half puts the file in the Outbox, closes the vault and
//! goes on to the next. A vault that cannot be sealed ends a person's lock
//! (one that may not lose changes) with that vault open and its changes
//! kept, the vaults sealed before it closed, nothing wiped and the Lock
//! sheet showing; an idle lock or Lock without sealing lets it go and
//! goes on. When the last is done a lock of every vault ends what the
//! flows hold and finishes (the wipes), and a lock of one closes that
//! vault and goes on with what was waiting for it. What a lock does that
//! cannot be undone is only what the seals themselves need: the running
//! job's answer is dropped and every secret on show goes off the screen
//! ([`lock_secrets_off_screen`], the lock's list); what was typed is
//! kept, to be finished if the lock stops.

use std::collections::VecDeque;

use faraday_crypto::work::Compute;
use faraday_keys::job::{SealJob, Sealed};
use faraday_keys::vault::{SEAL_RANDOM_LEN, VaultId};
use zeroize::Zeroizing;

use super::Stuck;
use super::errors;
use crate::strings::{EN, fill};
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
    /// Whether a vault that cannot be sealed may be let go with its
    /// changes lost (an idle lock, Lock without sealing). A lock of every
    /// vault that may not stops instead and asks.
    may_lose: bool,
    /// With `may_lose`, the vaults the Lock sheet named when the person
    /// pressed Lock without saving: only these are let go. Any other vault
    /// that cannot be sealed stops the run and the sheet shows again with
    /// what it now names. None: every vault may be let go (an idle lock for
    /// what is past the ceiling, a lock with no randomness to seal with).
    named: Option<Vec<VaultId>>,
    /// What a lock of one goes on with when its vault is closed.
    after: After,
    /// The vault whose new memory step was dropped for a retried seal; the
    /// Locked sheet says so only once that seal has been made.
    step_dropped: Option<VaultId>,
    /// A Lock pressed while this seal of one vault ran, to be done when it
    /// ends: `Some(may_lose)`, whether it may let go what cannot be sealed.
    lock_after: Option<bool>,
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

/// Whether a lock is under way that waits for a seal to end: nothing
/// secret is put on show again until it has gone through (the TOTP codes
/// are not made).
pub(crate) fn lock_pending(app: &Faraday) -> bool {
    app.vaults
        .state
        .sealing
        .as_ref()
        .is_some_and(|r| r.all || r.lock_after.is_some())
}

/// Takes every secret on show off the screen, its bytes wiped as it
/// drops; what is typed and not yet shown stays where it is held. Called
/// by a lock, first, and by the idle warning. The code on show (it may be
/// an entry's TOTP secret, `docs/QR.md` §7.1), an open entry's revealed
/// fields, a key's SeedQR and words on the item page, Create a vault's
/// passphrases, the backup checklist's seed and codex32
/// string with its QR, the backup chart's passphrase sheet, New key's
/// words and codex32 strings, the child of
/// BIP-85, Explore's words and passphrase, the Convert result, the QR,
/// Krux code and core wallet tools, a Krux code's contents, Lightning's
/// private key, Vanity's passphrase, the Inbox's passphrase and a scanned
/// text shown unmasked. The one place that says what a secret on show is;
/// a screen that shows a secret only while its eye is open is put away
/// here, and a new one is added here.
pub(crate) fn secrets_off_screen(app: &mut Faraday) {
    app.qr.close();
    app.explore_hide();
    crate::convert::hide(app);
    crate::kef_tool::hide(app);
    crate::qr_tool::hide(app);
    crate::core_wallet::tool::hide(app);
    app.lightning.hide_secrets();
    app.vanity.hide_secrets();
    app.inbox.hide_secrets();
    app.scan_card_hide();
    app.vaults.hide_secrets();
    app.backup.hide_secrets();
    app.keygen.hide_secrets();
    app.bip85.hide_secrets();
    app.seed_read.hide_secrets();
    app.glance.set_reveal(false);
}

/// What a lock takes off the screen: [`secrets_off_screen`] and the
/// checklist's Seed XOR parts, which the idle warning keeps (they come
/// from fresh randomness, cannot be shown again and have no eye).
pub(crate) fn lock_secrets_off_screen(app: &mut Faraday) {
    secrets_off_screen(app);
    app.backup.drop_paper();
}

/// Remembers a lock asked for while a seal runs, to be done when it ends
/// (the seal finishes first). A lock of every vault already under way is
/// that lock: nothing is remembered. Every secret on show closes at once,
/// whatever the lock waits for ([`secrets_off_screen`]).
pub(crate) fn lock_after_seal(app: &mut Faraday, may_lose: bool) {
    lock_secrets_off_screen(app);
    if let Some(run) = app.vaults.state.sealing.as_mut().filter(|r| !r.all) {
        run.lock_after = Some(run.lock_after.unwrap_or(false) || may_lose);
    }
}

/// Seals every open vault with changes into the Outbox, then finishes
/// the lock (`docs/VAULT.md`). A vault nobody changed or
/// that is read-only is not rewritten: a file's bytes change only when
/// there is something to keep, which is all a second copy can compare
/// (`docs/VAULT.md` §9). A vault whose seal cannot be made (the seed or
/// the answers have not arrived, or whose changes do not fit) is let go
/// unsealed when `may_lose`, its changes lost and said so on the Locked
/// sheet; else the lock stops with it open and the Lock sheet shows.
pub(crate) fn vault_seal_all(app: &mut Faraday, may_lose: bool) {
    // This lock has lost nothing yet: an earlier process's warning is not
    // repeated by a lock that loses none.
    app.vaults.clear_changes_lost();
    app.vaults.clear_step_dropped();
    // What each open vault holds, remembered across the lock.
    app.vault_summaries_refresh();
    // A held key's record says so before the vault goes
    // (`docs/history/NEW-WALLET.md` §14.3).
    app.held_sweep();
    // An original after the target its item was moved to.
    let ids = app.vaults.open.iter().map(|o| o.id).collect();
    let queue = app.vaults.state.moved_order(ids);
    // A person's Lock without saving lets go what the sheet named and no
    // more; the memory free may have changed since it was drawn, so what the
    // run finds short that it did not name is not lost but shown again.
    let named = (may_lose
        && app.sheet == Some(crate::Sheet::LockAsk)
        && !app.seal_waits()
        && app.idle_locked_after().is_none())
    .then(|| app.vaults.state.lock_named.clone());
    app.vaults.state.sealing = Some(SealRun {
        queue,
        running: None,
        all: true,
        may_lose,
        named,
        after: After::Nothing,
        step_dropped: None,
        lock_after: None,
    });
    seal_next(app);
}

/// Costs what `seal_next` costs for the one vault. Locks open vault `i`
/// alone: sealed into the Outbox when changed,
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
        app.toast(EN.no_randomness);
        return false;
    }
    // The item moved out of it is in no sealed file until its target's.
    let waits = app.vaults.state.moved_waits_for(id);
    if let Some(&target) = waits.first() {
        let name = app
            .vaults
            .open
            .iter()
            .find(|o| o.id == target)
            .map_or_else(String::new, |o| o.label(app.vaults.store()));
        app.toast(&fill(EN.vault_moved_lock_first, &[&name]));
        return false;
    }
    let c = app.compute();
    if let Some(cannot) = app.vaults.cannot_seal(&c, i) {
        app.toast(&cannot.why);
        return false;
    }
    app.vault_summaries_refresh();
    app.held_sweep();
    app.vaults.state.sealing = Some(SealRun {
        queue: VecDeque::from([id]),
        running: None,
        all: false,
        may_lose: false,
        named: None,
        after,
        step_dropped: None,
        lock_after: None,
    });
    seal_next(app);
    !sealing(app)
}

/// Costs what `seal_memory` costs and the seal job's start; its Argon2id
/// run is on the worker. The next vault of the queue: its plans' counts written, the
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
        // Whether this vault may be let go with its changes lost.
        let may_lose = run.may_lose && run.named.as_ref().is_none_or(|n| n.contains(&id));
        let Some(i) = app.vaults.open.iter().position(|o| o.id == id) else {
            continue;
        };
        // What this computer has free now, not what it had at an earlier
        // seal of a lock or at the unlock: each seal is checked against
        // the memory free when it starts.
        app.vaults.read_free();
        // A target of an item moved out of this vault is still open (a
        // sealed one is closed): it was let go unsealed. This vault goes
        // with it when the lock may lose changes, its last sealed file
        // still holding the item; else the lock stops and asks.
        if all
            && app
                .vaults
                .state
                .moved_waits_for(id)
                .iter()
                .any(|t| app.vaults.open.iter().any(|o| o.id == *t))
        {
            if may_lose {
                app.vaults.set_changes_lost();
                continue;
            }
            return seal_stopped(app, None);
        }
        // Its plans take the counts of the vaults sealed before it
        // (`docs/VAULT.md` §11).
        // A read-only vault is closed as it is: nothing written to it.
        let read_only = app.vaults.store().read_only(id).is_some();
        if !read_only {
            let c = app.compute();
            app.vaults.seals_write(&c, i);
            seal_memory(app, i);
            seal_nonces(app, i);
        }
        if read_only || (app.vaults.open[i].changes == 0 && !app.vaults.store().rekeyed(id)) {
            if !all {
                seal_close(app, id);
            }
            continue;
        }
        // A vault whose changes are past the ceiling is let go unsealed only
        // when the person chose Lock without sealing or an idle lock had no
        // one to ask; else the lock stops and asks. Memory short for the
        // count is not past anything: it goes on to the seal's own check
        // below, which keeps the vault open with its changes.
        if all {
            let c = app.compute();
            if app.vaults.cannot_seal(&c, i).is_some_and(|n| !n.short) {
                if may_lose {
                    app.vaults.set_changes_lost();
                    continue;
                }
                return seal_stopped(app, None);
            }
        }
        // This computer must have free what the seal takes, before it takes
        // any (F206 point 4). Short of it the vault stays open with its
        // changes and the reason shows, in an idle lock too, which would
        // lose them if the seal ran out of memory half way. Only a person's
        // Lock without sealing lets the changes go.
        let c = app.compute();
        if let Some(why) = app.vaults.seal_refusal(&c, id) {
            if all && may_lose && app.idle_locked_after().is_none() {
                app.vaults.set_changes_lost();
                continue;
            }
            app.toast(&why);
            if all {
                return seal_stopped(app, None);
            }
            app.vaults.state.sealing = None;
            app.out.command(Command::Draw);
            return;
        }
        let work = match seal_work(app, id) {
            Ok(work) => work,
            // A lock of one stays open with its changes and the person is
            // told; a lock of every vault that may lose changes lets it go
            // and goes on to the wipes, else it stops and asks.
            Err(why) => {
                if !all {
                    seal_stop(app, why);
                } else if may_lose {
                    app.vaults.set_changes_lost();
                } else {
                    // The sheet names it, as it loses changes if let go.
                    if matches!(why, NoJob::Unwritable) {
                        app.vaults.state.set_stuck(id, Stuck::Unwritable);
                    }
                    return seal_stopped(app, None);
                }
                continue;
            }
        };
        let line = EN.vault_sealing;
        if let Some(run) = app.vaults.state.sealing.as_mut() {
            run.running = Some((id, work.name.clone()));
        }
        app.start(Job::Seal(Box::new(work)), line);
        return;
    }
}

/// Costs what `OpenVaults::fits` costs for each record put. The
/// signed-amount memory goes into open vault `i`, the oldest
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
    app.vaults.state.add_memory(&c, i, &[], new);
}

/// Costs what `Vaults::add_memory` costs, one walk of the vault's nonce
/// records and unreadable values, and a set lookup for each waiting entry.
/// The transactions waiting in the nonce history go into open
/// vault `i`: for each whose entries the vault lacks, its record becomes
/// the union of the vault's own and the waiting entries, the newest. Only
/// what was checked here is written, never a record read from a vault.
/// Records that do not read, and a read-only vault, are left as written
/// (`docs/FLOWS.md` "Nonce history").
fn seal_nonces(app: &mut Faraday, i: usize) {
    if !app.memory.nonces.kept() {
        return;
    }
    let Some(id) = app.vaults.open.get(i).map(|o| o.id) else {
        return;
    };
    if app.vaults.store().read_only(id).is_some() {
        return;
    }
    use crate::memory::nonces::NonceEntry;
    use std::collections::{BTreeSet, HashMap, HashSet};
    // One walk of the vault's records and of its unreadable values.
    let mut held: HashMap<[u8; 32], (Vec<NonceEntry>, BTreeSet<NonceEntry>)> = HashMap::new();
    for (_, r) in app
        .vaults
        .records(id)
        .of(faraday_vault::records::kind::NONCES)
    {
        let Some(tx) = r
            .public(faraday_vault::records::field::NONCES_TX)
            .and_then(|t| <[u8; 32]>::try_from(t).ok())
        else {
            continue;
        };
        let (list, set) = held.entry(tx).or_default();
        for e in r
            .public_all(faraday_vault::records::field::NONCES_ENTRY)
            .filter_map(NonceEntry::from_bytes)
        {
            if set.insert(e) {
                list.push(e);
            }
        }
    }
    let unreadable: HashSet<&str> = app
        .vaults
        .store()
        .unreadable(id)
        .iter()
        .filter(|u| u.uuid.is_none())
        .map(|u| u.title.as_str())
        .collect();
    let mut replace = Vec::new();
    let mut new = Vec::new();
    for (tx, waiting) in app.memory.nonces.pending() {
        // A value that does not read under this very key is left as
        // written, and the transaction is not written to this vault.
        if unreadable.contains(NonceEntry::title_of(tx).as_str()) {
            continue;
        }
        let (mut list, mut set) = held.get(tx).cloned().unwrap_or_default();
        let mut added = false;
        for e in waiting {
            if set.insert(*e) {
                list.push(*e);
                added = true;
            }
        }
        if !added {
            continue;
        }
        replace.push(*tx);
        new.push(NonceEntry::record_of(tx, &list));
    }
    let c = app.compute();
    app.vaults.state.add_memory(&c, i, &replace, new);
}

/// Why a seal could not be made into a job.
enum NoJob {
    /// The session's answers have not arrived (the shell is asked).
    Seed,
    /// The store cannot write this vault.
    Unwritable,
}

/// Costs one SHA-256 per 32 bytes of `n`, with no `Compute`. `n` bytes for
/// the UUIDs of new entries and groups, from one fresh answer: SHA-256
/// over a domain string, the answer and a counter, block by block. The UUIDs need to differ, not to be secret, so one answer
/// serves any number of them.
fn uuid_bytes_from(answer: &[u8], n: usize) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::with_capacity(n + 32));
    let mut input = Zeroizing::new(Vec::with_capacity(24 + answer.len() + 4));
    input.extend_from_slice(b"faraday vault seal uuids");
    input.extend_from_slice(answer);
    input.extend_from_slice(&0u32.to_be_bytes());
    let mut counter = 0u32;
    while out.len() < n {
        let tail = input.len() - 4;
        input[tail..].copy_from_slice(&counter.to_be_bytes());
        let block = Zeroizing::new(faraday_crypto::sha256(&input));
        out.extend_from_slice(&*block);
        counter += 1;
    }
    out.truncate(n);
    out
}

/// The front half: open vault `id`'s seal made into a job, which takes
/// the vault's job. Why not where the session's answers have not arrived
/// (and the shell is asked), or the store gave no job.
fn seal_work(app: &mut Faraday, id: VaultId) -> Result<SealWork, NoJob> {
    if !app.seeded() {
        app.out.command(faraday_shell_api::Command::RequestEntropy);
        return Err(NoJob::Seed);
    }
    let n = app.vaults.store().uuid_bytes(id).ok_or(NoJob::Unwritable)?;
    let random = app
        .fresh_bytes(b"vault seal", SEAL_RANDOM_LEN)
        .ok_or(NoJob::Seed)?;
    let mut seal = Zeroizing::new([0u8; SEAL_RANDOM_LEN]);
    seal.copy_from_slice(&random);
    let uuids = if n == 0 {
        Zeroizing::new(Vec::new())
    } else {
        let answer = app.fresh_bytes(b"vault entries", 32).ok_or(NoJob::Seed)?;
        uuid_bytes_from(&answer, n)
    };
    let now = app.clock_now();
    let name = app
        .vaults
        .open
        .iter()
        .find(|o| o.id == id)
        .ok_or(NoJob::Unwritable)?
        .name
        .clone();
    let job = app
        .vaults
        .state
        .store
        .seal_job(id, &seal, &uuids, now)
        .ok_or(NoJob::Unwritable)?;
    Ok(SealWork { job, id, name })
}

/// A lock of one that could not seal: the vault stays open.
fn seal_stop(app: &mut Faraday, why: NoJob) {
    app.vaults.state.sealing = None;
    app.toast(match why {
        NoJob::Seed => EN.no_randomness,
        NoJob::Unwritable => EN.vaults_cannot_write,
    });
}

/// Costs a read of the free memory (a process on macOS) and what
/// `lock_blocks` costs: for each changed vault a walk of its records and,
/// where the figures call for it, a document write (`cannot_seal`) or a copy
/// and count of it (`seal_refusal`). A lock met a vault
/// it may not let go, or cannot seal: the run ends, that vault and the ones
/// after it stay open with their changes, nothing is wiped, and the Lock
/// sheet shows, naming what it names now (a Lock without saving that named
/// vaults before this one lets go only those).
fn seal_stopped(app: &mut Faraday, short: Option<(VaultId, faraday_vault::file::Error, String)>) {
    app.vaults.state.sealing = None;
    // What the sheet says is read again against the memory free now.
    let free = app.vaults.read_free();
    // A seal that ran out of memory says what this computer has free now.
    // A person's press that finds it short again would bring back the same
    // sheet: the sheet names it, as short of memory (an idle lock only
    // tells, and corrects the text of an entry a person's Lock left).
    if let Some((id, why, name)) = short {
        let text = errors::stopped_text(&why, &name, free);
        app.toast(&text);
        if app.idle_locked_after().is_none() || app.vaults.state.has_short(id) {
            app.vaults.state.set_stuck(id, Stuck::Short { why: text });
        }
    }
    app.vaults.state.reread_seals();
    // The lock did not happen: a later idle period warns and tries again,
    // and a Lock the person presses is theirs, not the idle lock's.
    if app.idle.locking().is_some() {
        app.idle.set_locking(None);
        app.idle.input(app.now_ms);
    }
    if app.seal_waits() {
        app.out.command(faraday_shell_api::Command::RequestEntropy);
    }
    let c = app.compute();
    app.vaults.state.name_lock(&c);
    app.sheet = Some(crate::Sheet::LockAsk);
    app.out.command(Command::Draw);
}

/// What a lock does to flows before the wipes, once every seal is done
/// (the vaults are still open): a lock before Make a new wallet's keys are
/// kept anywhere ends it, as its Cancel does: nothing is left to come back
/// to (`docs/history/NEW-WALLET.md` §18); the flows keep where they are.
fn lock_flows(app: &mut Faraday) {
    let c = app.compute();
    if app.lock_ends_create(&c) {
        app.create_drop();
    }
    app.flow_before_lock();
}

/// Closes open vault `id` after a seal.
fn seal_close(app: &mut Faraday, id: VaultId) {
    if let Some(i) = app.vaults.open.iter().position(|o| o.id == id) {
        app.vaults.state.close(i);
    }
    app.nonces_sync_open();
    app.vaults.state.current = 0;
    app.vaults.state.item_open = false;
    app.vaults.state.form = None;
    drop_vault_dialog(app);
}

/// Change passphrase and Memory belong to the vault they were opened
/// on: once it is closed they close with it, the typed text gone.
fn drop_vault_dialog(app: &mut Faraday) {
    use crate::Sheet::{VaultMemory, VaultPassphrase};
    if matches!(app.sheet, Some(VaultPassphrase | VaultMemory)) {
        app.sheet = None;
    }
}

/// The queue is empty: a lock of every vault closes them and goes on
/// with the wipes; a lock of one is done.
fn seal_end(app: &mut Faraday) {
    let Some(run) = app.vaults.state.sealing.take() else {
        return;
    };
    if run.all {
        // Flows end while the vaults are still open, as the lock began.
        lock_flows(app);
        app.vaults.state.close_all();
        app.nonces_sync_open();
        app.vaults.state.form = None;
        drop_vault_dialog(app);
        app.vaults.state.load = None;
        app.vaults.state.pass_seeds = None;
        lock_finish(app);
    } else if let Some(may_lose) = run.lock_after {
        // A Lock pressed meanwhile goes first: Create a vault does not
        // open just before it.
        app.lock_after_seal_ended(may_lose);
    } else if let After::CreateFrom(screen) = run.after {
        app.vault_act(super::VaultAction::CreateFrom(screen));
    }
}

/// A test's wish that the next seal of a vault with a new memory step
/// pending fails as if the step could not be allocated.
#[cfg(feature = "testkit")]
static FAIL_STEP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// A test's wish that the seal retried without the step fails as well.
#[cfg(feature = "testkit")]
static FAIL_RETRY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(feature = "testkit")]
use std::sync::atomic::Ordering;

/// Makes the next seal of a vault with a new memory step pending fail as
/// if this computer could not allocate it.
#[cfg(feature = "testkit")]
pub(crate) fn fail_next_step_seal() {
    FAIL_STEP.store(true, Ordering::SeqCst);
}

/// [`fail_next_step_seal`], and the seal retried without the step fails too.
#[cfg(feature = "testkit")]
pub(crate) fn fail_next_step_seal_and_retry() {
    FAIL_STEP.store(true, Ordering::SeqCst);
    FAIL_RETRY.store(true, Ordering::SeqCst);
}

/// The name the seal of open vault `id` takes in the Outbox: `name`
/// where no file waits under it or the file is an earlier copy of this same
/// vault (told by its SHA-256 among the copies the session remembers), else
/// the first name beside it that is free in Files too or held by such a copy.
/// A file that cannot be told to be a copy of this vault is not one, and
/// while the vault open is older than a copy seen, or sealed as often as
/// one, only the file it was opened from is an earlier copy.
fn seal_file_name(app: &Faraday, id: VaultId, name: &str) -> String {
    let Some(open) = app.vaults.open.iter().find(|o| o.id == id) else {
        return name.to_string();
    };
    let others_in = |files: &[crate::Item]| -> Vec<String> {
        files
            .iter()
            .filter(|i| {
                let copy = app.vaults.vault_id_of_file(i.sha) == Some(open.vault_id);
                !copy || open.seal_warn.is_some() && i.sha != Some(open.seal_digest)
            })
            .map(|i| i.name.to_ascii_lowercase())
            .collect()
    };
    let mut taken = others_in(&app.outbox);
    if !taken.contains(&name.to_ascii_lowercase()) {
        return name.to_string();
    }
    taken.extend(others_in(&app.inbox));
    crate::visit_taken::free_name(&taken, name)
}

/// The back half: the file to the Outbox, and the next vault.
pub(crate) fn sealed(app: &mut Faraday, done: SealDone) {
    let SealDone { sealed, id, name } = done;
    let all = match app.vaults.state.sealing.as_mut() {
        Some(run) => {
            run.running = None;
            run.all
        }
        // The run is gone: the vault is let go as it is, its changes
        // kept, so that it can be edited and sealed again.
        None => {
            app.vaults.state.store.seal_ended(id, false);
            return;
        }
    };
    let result = sealed.into_file();
    #[cfg(feature = "testkit")]
    let result = if if app.vaults.store().has_step(id) {
        FAIL_STEP.swap(false, Ordering::SeqCst)
    } else {
        FAIL_RETRY.swap(false, Ordering::SeqCst)
    } {
        Err(faraday_vault::file::Error::Memory(1))
    } else {
        result
    };
    match result {
        Ok(bytes) => {
            // Over an earlier copy of this vault, else beside the file that
            // has the name (`docs/VAULT.md` §5): the name really taken is
            // the one the rest uses.
            let name = seal_file_name(app, id, &name);
            app.vaults.state.file_named(id, &name);
            // The file is read where it is, then moved into the Outbox: it
            // is not copied (F206 point 6).
            if let Some(i) = app.vaults.open.iter().position(|o| o.id == id) {
                let c = app.compute();
                app.vaults.seals_sealed(&c, i, &bytes);
                // The marks this lock made are the written file's.
                app.held_sealed(i, app.vaults.open[i].seal_digest);
                // The keys waiting for their copies that this sealing holds.
                app.flow_sealed(i, &name, &bytes);
            }
            app.put_outbox(&name, bytes);
            if app
                .vaults
                .state
                .sealing
                .as_ref()
                .and_then(|r| r.step_dropped)
                == Some(id)
            {
                app.vaults.set_step_dropped();
            }
            app.vaults.state.store.seal_ended(id, true);
            super::more::sealed(app, id);
            app.vaults.state.clear_stuck(id);
            // Sealed, so no longer open, in a lock of every vault too: a
            // later vault's failure leaves this one closed, its file in
            // the Outbox, not open and empty.
            seal_close(app, id);
        }
        // An idle lock that could not allocate the new memory step seals
        // again at the file's own cost: the changes are kept, the step is
        // not.
        Err(faraday_vault::file::Error::Memory(_))
            if all && app.idle_locked_after().is_some() && app.vaults.store().has_step(id) =>
        {
            app.vaults.state.store.seal_ended(id, false);
            app.vaults.state.store.clear_step(id);
            if let Some(run) = app.vaults.state.sealing.as_mut() {
                run.step_dropped = Some(id);
                run.queue.push_front(id);
            }
            return seal_next(app);
        }
        // A seal that ran out of memory half way loses nothing by itself: the
        // vault stays open with its changes and the reason shows, in an idle
        // lock too. Only a person's Lock without saving that named this
        // vault lets it go; any other press shows the sheet again.
        Err(why @ faraday_vault::file::Error::Memory(_)) if all => {
            app.vaults.state.store.seal_ended(id, false);
            let named =
                app.vaults.state.sealing.as_ref().is_some_and(|r| {
                    r.may_lose && r.named.as_ref().is_none_or(|n| n.contains(&id))
                });
            if named && app.idle_locked_after().is_none() {
                app.vaults.set_changes_lost();
                return seal_next(app);
            }
            return seal_stopped(app, Some((id, why, name)));
        }
        // An idle lock has no one to ask: the changes are lost and the
        // lock goes on to the wipes.
        Err(_) if all && app.idle_locked_after().is_some() => {
            app.vaults.state.store.seal_ended(id, false);
            app.vaults.set_changes_lost();
            return seal_next(app);
        }
        // Else the vault stays open with its changes, the lock goes no
        // further (the rest of the queue is not sealed, nothing is
        // wiped), and the person is told.
        Err(why) => {
            app.vaults.state.store.seal_ended(id, false);
            let lock = app
                .vaults
                .state
                .sealing
                .take()
                .and_then(|run| run.lock_after);
            app.out.command(Command::Draw);
            app.toast(&errors::stopped_text(
                &why,
                &name,
                app.vaults.memory_free_mib,
            ));
            // A Lock pressed meanwhile is not dropped: it goes ahead, and
            // shows the Lock sheet if it would lose the vault's changes.
            if let Some(may_lose) = lock {
                app.lock_after_seal_ended(may_lose);
            }
            return;
        }
    }
    seal_next(app);
}

/// The rest of a lock, after the last seal: the wipes.
fn lock_finish(app: &mut Faraday) {
    // What the boot stick brought and was not imported is not kept:
    // its bytes are wiped as it drops.
    app.import.clear_all();
    // A vault write waiting for its name's answer is not kept.
    app.visit.name_taken_clear();
    // Nothing secret copied in outlives the process that read it
    // (`docs/VAULT.md`): the next process is clean, and these would be
    // in its memory and in `/run/faraday`. They are still on the stick
    // they came from.
    // A picture goes as what its codes hold, and as Other when none read.
    app.inbox.retain(|i| !i.dropped_at_lock());
    app.session.wipe();
    // The charts and the backup list kept across moved frames.
    app.kept.charts_drop();
    // The signing page's kept text of the Inbox's small files.
    app.vaults.drop_sign_text();
    app.own_wipe();
    app.wallets.clear_spend();
    app.seed_read.clear();
    app.decode.clear_check();
    app.decode.clear_compare();
    app.backup.close();
    app.message.clear();
    app.create.clear();
    app.restore.clear();
    app.keygen.forget();
    app.wordlist.clear();
    app.bip85.clear();
    app.tools.convert.clear();
    app.tools.qr_tool.clear();
    app.explore.clear();
    app.secrets.clear();
    app.entry.reset();
    // A made key file of a vault let go unsealed does not go with the boxes.
    super::more::drop_gone(app);
    app.save_boxes_for_lock();
    app.restart = true;
    app.out.commands.push_back(Command::Exit);
}
