//! A vault's Argon2id runs as jobs on the worker (`docs/WORKERS.md` §6,
//! `docs/KEYS.md` §2): opening a vault (an Unlock on the Vaults tab, the
//! Spend tab, the boot import, a backup flow), making one, and writing a
//! KeePass export. The one-pass measurement of this computer's speed
//! runs there too, as a quiet job. The front
//! half reads the state and builds the job, the passphrase moved into
//! faraday-keys' [`StretchJob`]; the back half takes the answer, on the
//! main thread, and the open vault joins the store there.

use faraday_crypto::work::Compute;
use faraday_keys::UnlockParts;
use faraday_keys::job::{Budget, KdbxJob, Parsed, StretchJob, Stretched};
use faraday_keys::vault::SEAL_RANDOM_LEN;
use faraday_vault::file::STEPS_KIB;
use faraday_vault::{Cost, records::field, records::kind};
use zeroize::Zeroizing;

use super::unlock_keys;
use super::{APP_MIB, Need, VaultAction, Work, errors, need_mib, vault_stem};
use crate::strings::{EN, fill};
use crate::work::{self, VaultAct, VaultEnded};
use crate::worker::Job;
use crate::{Faraday, Later, Screen};

/// What a vault stretch is for, kept with its job and handed back with
/// the answer.
pub(crate) enum Plan {
    /// Vault file `name` opened; `for_import` when the boot import's
    /// sheet asked for it.
    /// `sha` is the SHA-256 of the bytes the job opens, which is the copy
    /// the open vault is a copy of.
    Unlock {
        name: String,
        sha: Option<[u8; 32]>,
        for_import: bool,
    },
    /// The vault the Create form describes, named `typed`; `first` is the
    /// first passphrase, kept for a backup flow that unlocks the vault it
    /// made at once. Wiped as it drops.
    Create {
        typed: String,
        first: Zeroizing<String>,
        /// The vault's root group UUID, which names it (`docs/VAULT.md` §7).
        id: [u8; 16],
        /// The key file the vault needs: a file chosen from Files, read
        /// at the press, or one drawn then.
        key: Made,
    },
}

/// The key file a vault being made needs.
pub(crate) enum Made {
    /// None.
    No,
    /// A key file drawn at the press: its `.keyx` comes out of the job.
    Drawn,
    /// This file of Files, which leaves Files with the vault made.
    Chosen(String),
}

/// A vault stretch to run on the worker.
pub(crate) struct StretchWork {
    job: StretchJob,
    act: VaultAct,
    cost: Cost,
    plan: Plan,
}

/// A vault stretch's answer, with what it was for.
pub(crate) struct StretchDone {
    stretched: Stretched,
    plan: Plan,
}

impl StretchWork {
    /// Costs an Argon2id run at the job's cost, the stretch noted ([`work::note_vault`]).
    pub(crate) fn run(self, c: &Compute) -> StretchDone {
        let from = work::vault_clock();
        let stretched = self.job.run(c);
        let took = from.map(|t| t.elapsed());
        let error = stretched.error();
        work::note_vault_took(
            self.act,
            self.cost,
            VaultEnded::of_error(error.as_ref()),
            took,
        );
        StretchDone {
            stretched,
            plan: self.plan,
        }
    }
}

/// Leaves a vault stretch for the tick after a frame that says it is
/// under way (`docs/SPEED.md` §9); the job starts then
/// ([`Faraday::vault_work`]).
pub(crate) fn defer(app: &mut Faraday, work: Work) {
    if app.later.is_some() {
        app.later_now();
    }
    app.vaults.state.working = Some(work);
    app.later = Some(Later::Vault(work, app.worker.inline()));
    app.worker.set_drawn(false);
    app.out.redraw();
    app.out.commands.push_back(crate::Command::Draw);
}

/// Starts the job the frame drawn first was for.
pub(crate) fn work(app: &mut Faraday, work: Work) {
    let started = match work {
        Work::Unlock => unlock_start(app),
        Work::Create => create_start(app),
        Work::Kdbx => kdbx_start(app),
        _ => false,
    };
    if !started {
        ended(app);
    }
}

/// The vault job is over, or was never started: the screens stop
/// saying it runs.
pub(crate) fn ended(app: &mut Faraday) {
    if matches!(
        app.vaults.state.working,
        Some(Work::Unlock | Work::Create | Work::Kdbx)
    ) {
        app.vaults.state.working = None;
    }
}

/// The job that opens `file` under `passphrase`, with the memory it may take
/// worked out from what the computer has free (`docs/VAULT.md` §2): the
/// bytes the document's structures may take, and for a compressed file what
/// its plaintext may take, each against the same function as Unlock's test
/// ([`need_mib`]). A computer that gives no figure leaves the file's
/// own length to decide: the reader's limit per byte of it.
fn open_job(
    app: &Faraday,
    file: Vec<u8>,
    passphrase: crate::secret_text::SecretText,
) -> StretchJob {
    let network = app.session.network();
    let Some(free) = app.vaults.memory_free_mib else {
        return StretchJob::open(file, passphrase).with_identities(network);
    };
    const MIB: usize = 1024 * 1024;
    // What is free beside the app. The copy of the file this job holds is
    // already out of it (free is read after the copy was made) and is
    // counted by the open once, with the body, the document and the text
    // copies; the rest is what the document's structures, the items and
    // the index may take.
    let room = (free as usize)
        .saturating_mul(MIB)
        .saturating_sub(APP_MIB as usize * MIB);
    let budget: Budget = Box::new(move |plain| {
        let need = need_mib(Need::Inflate { plain });
        if need > free { Err(need) } else { Ok(()) }
    });
    // Once the body is whole, the document's length and the attachments'
    // are known before the document is parsed. What the parse check
    // counted is also the most the open takes, however much is free: the
    // Parse cap is the smaller room of the two (this and `room` above),
    // and the file is inside it, so that a crafted document cannot take
    // more than it counted.
    let parsed: Parsed = Box::new(move |document, attachments| {
        let need = need_mib(Need::Parse {
            document,
            attachments,
        });
        if need > free {
            Err(need)
        } else {
            Ok(Some(
                (need.saturating_sub(APP_MIB) as usize).saturating_mul(MIB),
            ))
        }
    });
    StretchJob::open_within(file, passphrase, Some(room), Some(budget), Some(parsed))
        .with_identities(network)
}

/// The front half of an unlock: the chosen vault's file and the
/// typed passphrase, moved, go to the worker. False when nothing
/// started.
fn unlock_start(app: &mut Faraday) -> bool {
    let files = app.vault_files();
    let Some(f) = files.get(app.vaults.pick) else {
        return false;
    };
    let Some((bytes, sha)) = app.vault_bytes(f) else {
        return false;
    };
    let name = f.name.clone();
    let cost = f.facts.header.cost;
    // Unlocked on the boot import's sheet: the unlock imports.
    let for_import = app.sheet == Some(crate::Sheet::Import) && app.import.state().is_some();
    // The key file and the secret are read for this try and wiped as the
    // job ends; the chip and the field stay for another.
    // A vault just made unlocks at once with the key file it was made with.
    let made = app.vaults.state.store.take_made_parts();
    let Ok(components) = made.map_or_else(|| super::unlock_keys::parts(app), Ok) else {
        return false;
    };
    app.vaults.state.unlock_tried = (components.has_key_file(), components.has_yubikey());
    let passphrase = std::mem::take(&mut app.vaults.state.passphrase.text);
    // The unlock and its parse check are held to what is free now.
    app.vaults.read_free();
    // An open is never refused by `keyed`; if it were, nothing starts.
    let Ok(job) = open_job(app, bytes, passphrase).keyed(components) else {
        return false;
    };
    app.start(
        Job::Stretch(Box::new(StretchWork {
            job,
            act: VaultAct::Unlocked,
            cost,
            plan: Plan::Unlock {
                name,
                sha,
                for_import,
            },
        })),
        EN.working_vault_open,
    );
    true
}

/// The create form says the system has not given enough randomness yet.
fn say_no_randomness(app: &mut Faraday) {
    if let Some(c) = app.vaults.state.create.as_mut() {
        c.error = Some(EN.no_randomness.to_string());
    }
}

/// The front half of Create: the form's passphrase is copied into the
/// job (the form keeps it for a Back or an error) and wiped with it.
/// False when nothing started.
fn create_start(app: &mut Faraday) -> bool {
    // Everything random in a vault comes from fresh system entropy of
    // its own; a vault is never made before it has arrived.
    if app.fresh.pool_is_empty() {
        app.out.command(faraday_shell_api::Command::RequestEntropy);
        say_no_randomness(app);
        return false;
    }
    let Some((_, mem, passes)) = app.vaults.form_cost() else {
        return false;
    };
    let Some(id) = app.fresh_bytes(b"vault id", 16) else {
        say_no_randomness(app);
        return false;
    };
    let Some(random) = app.fresh_bytes(b"vault seal", SEAL_RANDOM_LEN) else {
        say_no_randomness(app);
        return false;
    };
    let mut id16 = [0u8; 16];
    id16.copy_from_slice(&id);
    let mut seal = Zeroizing::new([0u8; SEAL_RANDOM_LEN]);
    seal.copy_from_slice(&random);
    let now = app.clock_now();
    // A key file Create makes is drawn now, at the press; a chosen one is
    // read now. A refused one was said by the press before this.
    let drawn = match app.vaults.create.as_ref().map(|c| c.key.clone()) {
        Some(super::KeyChoice::Make) => {
            let Some(bytes) = app.fresh_bytes(b"vault key file", 32) else {
                say_no_randomness(app);
                return false;
            };
            let mut key = Zeroizing::new([0u8; 32]);
            key.copy_from_slice(&bytes);
            Some(key)
        }
        _ => None,
    };
    let chosen = match super::create_key::chosen(app) {
        Ok(chosen) => chosen,
        Err(e) => {
            if let Some(c) = app.vaults.state.create.as_mut() {
                c.error = Some(e);
            }
            return false;
        }
    };
    let Some(form) = app.vaults.create.as_ref() else {
        return false;
    };
    // The memory is the largest step the file takes at or below what the
    // form states; its lanes are the format's.
    let memory_kib = STEPS_KIB
        .iter()
        .copied()
        .filter(|k| *k <= mem.saturating_mul(1024))
        .max()
        .unwrap_or(STEPS_KIB[0]);
    let cost = Cost {
        memory_kib,
        passes,
        lanes: 4,
    };
    let first = Zeroizing::new(
        form.phrases
            .first()
            .map(|(a, _)| a.text.to_string())
            .unwrap_or_default(),
    );
    let phrase = crate::secret_text::SecretText::of(&first);
    // The vault's name is the database's name: 64 bytes at most
    // (`docs/VAULT.md` §7).
    let label = match form.name.text.trim() {
        "" => EN.vaults_vault,
        typed => {
            let end = (0..=typed.len().min(super::area::LABEL_MAX))
                .rev()
                .find(|&i| typed.is_char_boundary(i))
                .unwrap_or(0);
            &typed[..end]
        }
    };
    let job = StretchJob::make(id16, label.into(), cost, phrase, seal, now);
    let (job, key) = match (drawn, chosen) {
        (Some(random), _) => (job.making_key_file(&random), Made::Drawn),
        (None, Some((name, parts))) => match job.keyed(parts) {
            Ok(job) => (job, Made::Chosen(name)),
            // Only a YubiKey's secret is refused, and Create offers none.
            Err(_) => return false,
        },
        (None, None) => (job, Made::No),
    };
    let typed = form.name.text.to_string();
    app.start(
        Job::Stretch(Box::new(StretchWork {
            job,
            act: VaultAct::Made,
            cost,
            plan: Plan::Create {
                typed,
                first,
                id: id16,
                key,
            },
        })),
        EN.working_vault_make,
    );
    true
}

/// Costs what `unlocked` or `created` costs: no Argon2id run, which ran on
/// the worker. The back half: a stretch's answer, applied on the main
/// thread.
pub(crate) fn stretched(app: &mut Faraday, done: StretchDone) {
    ended(app);
    let StretchDone { stretched, plan } = done;
    match plan {
        Plan::Unlock {
            name,
            sha,
            for_import,
        } => unlocked(app, stretched, (name, sha), for_import),
        Plan::Create {
            typed,
            first,
            id,
            key,
        } => created(app, stretched, (&typed, &first), id, key),
    }
}

/// Costs what `OpenVaults::adopt` costs: no Argon2id run, which ran on the
/// worker. The back half of an unlock, applied on the main thread.
fn unlocked(
    app: &mut Faraday,
    stretched: Stretched,
    (name, sha): (String, Option<[u8; 32]>),
    for_import: bool,
) {
    // A stick put in while the stretch ran: the press's own refusal.
    // The answer is dropped, and wiped as it drops.
    if !app.may_load_keys() {
        drop(stretched);
        app.vaults.state.unlock_error =
            Some(fill(EN.vaults_remove_to_unlock, &[app.medium.noun()]));
        app.vaults.unlock_follow.set(true);
        return;
    }
    let before = app.vaults.open.len();
    // The key records' fingerprints and master ids, worked out in the job.
    let identities = stretched.identities().to_vec();
    match app.vaults.state.adopt(stretched) {
        Ok((id, facts)) => {
            let vault_id = app.vaults.state.store().vault_id(id).unwrap_or_default();
            // The same vault from another copy: refused, naming the one open.
            if let Some(open) = app.vaults.open.iter().find(|o| o.vault_id == vault_id) {
                let said = fill(
                    EN.vault_already_open,
                    &[&open.label(app.vaults.state.store()), &open.name],
                );
                app.vaults.state.drop_adopted(id);
                app.vaults.state.unlock_error = Some(said);
                return;
            }
            let read_only = app
                .vaults
                .state
                .store()
                .read_only(id)
                .as_ref()
                .map(super::area::read_only_line);
            let (size, settings) = (
                super::area::size_line(&facts),
                super::area::settings_line(&facts),
            );
            app.vaults.keep_identities(&identities);
            app.vaults.state.typed_shown = false;
            app.vaults.state.focus = None;
            app.vaults.state.open.push(super::OpenVault {
                name,
                id,
                facts,
                vault_id,
                changes: 0,
                skip: Default::default(),
                each_shown: false,
                read: Default::default(),
                seal_digest: [0; 32],
                seal_warn: None,
                read_only,
                size,
                settings,
            });
            app.vaults.state.current = app.vaults.open.len() - 1;
            // Nothing of the key file or the secret is remembered, and the
            // key file's copy in Files is gone.
            super::unlock_keys::used(app);
            app.held_unlock_clear(app.vaults.current, sha);
            if !app.vault_unlock_held(app.vaults.current, sha, for_import) {
                app.vault_unlock_go_on(for_import);
            }
        }
        Err(e) => {
            app.vaults.state.unlock_error = Some(match e {
                // What was tried says what to try again.
                faraday_vault::file::Error::Kdbx(faraday_vault::kdbx::Error::Passphrase) => {
                    match app.vaults.unlock_tried {
                        (false, false) => EN.vault_error_wrong,
                        (true, false) => EN.vault_error_wrong_key_file,
                        (false, true) => EN.vault_error_wrong_yubikey,
                        (true, true) => EN.vault_error_wrong_both,
                    }
                    .to_string()
                }
                e => errors::stopped_text(&e, &name, app.vaults.memory_free_mib),
            });
            app.vaults.unlock_follow.set(true);
        }
    }
    if app.vaults.open.len() > before && app.vaults.older.is_none() {
        app.vault_unlock_flow();
    }
}

fn created(
    app: &mut Faraday,
    stretched: Stretched,
    (typed, first): (&str, &str),
    id: [u8; 16],
    key: Made,
) {
    // A stick put in while the vault was made: the press's own refusal.
    if !app.stick.is_empty() {
        drop(stretched);
        let said = fill(EN.vaults_remove_the_first, &[app.medium.noun()]);
        if let Some(c) = app.vaults.state.create.as_mut() {
            c.error = Some(said);
        }
        return;
    }
    // A key file Create made is kept with the vault or not at all: its
    // KeePass document must reach the Outbox, else no vault is kept.
    let document = match key {
        Made::Drawn => {
            let held = stretched.key_file_document(faraday_keys::Reveal::OutboxFile(
                faraday_keys::Part::KeyFile,
            ));
            match held.map(|h| h.clone()) {
                Some(d) => Some(d),
                None if stretched.error().is_some() => None,
                None => {
                    if let Some(c) = app.vaults.state.create.as_mut() {
                        c.error = Some(EN.vault_key_file_not_kept.to_string());
                    }
                    return;
                }
            }
        }
        _ => None,
    };
    match stretched.into_made() {
        Ok(file) => {
            let name = app.free_vault_name(&vault_stem(typed));
            // The components the vault is unlocked with at once, if a
            // waiting copy or backup flow does that.
            let mut parts = match (&key, &document) {
                (Made::Chosen(f), _) => unlock_keys::file_parts(app, f).ok(),
                (Made::Drawn, Some(d)) => UnlockParts::read(Some(d), None).ok(),
                _ => None,
            };
            if matches!(key, Made::Drawn) {
                let Some(d) = document else {
                    return;
                };
                let keyx = format!("{}.keyx", name.strip_suffix(".kdbx").unwrap_or(&name));
                let item = crate::Item::new(&app.compute(), &keyx, d.to_vec());
                app.outbox_secret(crate::secrets::OutReason::KeyFile, item);
            }
            if let Made::Chosen(f) = &key {
                // The file is the vault's now: gone from Files, wiped as it drops.
                app.inbox.retain(|i| i.name != *f);
            }
            app.put_outbox(&name, file);
            // The file is a copy of the vault it makes: nothing in the
            // file says which, so the session's summary does.
            let sha = app
                .outbox
                .iter()
                .find(|it| it.name == name)
                .and_then(|it| it.sha);
            if let Some(sha) = sha {
                let mut vault_id = [0u8; 32];
                for (k, b) in id.iter().enumerate() {
                    vault_id[2 * k] = b"0123456789abcdef"[usize::from(b >> 4)];
                    vault_id[2 * k + 1] = b"0123456789abcdef"[usize::from(b & 15)];
                }
                app.vaults.merge_summaries(vec![super::area::VaultSummary {
                    file: name.clone(),
                    id: vault_id,
                    copies: vec![sha],
                    name: name.clone(),
                    ..Default::default()
                }]);
            }
            // Made for an item that waits to go in it: unlocked at once, the
            // item put in when it is open.
            if super::copy::made(app, &name, id, (first, &mut parts)) {
                app.toast(EN.vaults_vault_created);
                return;
            }
            // Made inside a backup flow: unlocked at once with the
            // passphrase just typed twice, never leaving the flow.
            if app.flow_vault_made(&name, (first, &mut parts)) {
                app.toast(EN.vaults_vault_created);
                return;
            }
            app.vaults.state.just_made = Some(name.clone());
            app.vaults.state.create = None;
            app.vaults.state.focus = None;
            app.screen = Screen::Vaults;
            app.toast(EN.vaults_vault_created);
            // Unlock, the new vault picked (FLOWS decision 10): its
            // passphrase is typed once more, then the flow it was made
            // for goes on, or its contents open.
            if let Some(i) = app.vault_files().iter().position(|f| f.name == name) {
                app.vault_act(VaultAction::Open(i));
            }
        }
        Err(e) => {
            if let Some(c) = app.vaults.state.create.as_mut() {
                c.error = Some(errors::error_text(&e));
            }
        }
    }
}

/// A KeePass export to write on the worker, with the name its file takes.
pub(crate) struct KdbxWork {
    job: KdbxJob,
    name: String,
}

/// A KeePass export's answer: the file (encrypted, so public, written
/// as a new vault), or why there is none.
pub(crate) struct KdbxDone {
    file: Result<Vec<u8>, faraday_vault::file::Error>,
    name: String,
}

impl KdbxWork {
    /// Costs an Argon2id run, the new vault's.
    pub(crate) fn run(self, c: &Compute) -> KdbxDone {
        KdbxDone {
            file: self.job.run(c),
            name: self.name,
        }
    }
}

/// The front half of the KeePass export: the selected item, put in a new
/// vault sealed under the prompt's passphrase, which is copied into the job
/// (the prompt keeps it for an error) and wiped with it. The vault's
/// identifier, seal and entry identifiers are drawn as Create a vault and
/// a seal draw them. False when nothing started.
fn kdbx_start(app: &mut Faraday) -> bool {
    let Some(at) = app.vault_selected_index() else {
        return false;
    };
    let Some(id) = app.vaults.current_id() else {
        return false;
    };
    let Some(title) = app
        .vaults
        .records(id)
        .get(at)
        .filter(|r| r.kind() == kind::ENTRY)
        .map(|r| {
            r.public_text(field::TITLE)
                .unwrap_or(EN.vaults_entry)
                .to_string()
        })
    else {
        return false;
    };
    let Some(passphrase) = app
        .vaults
        .prompt
        .as_ref()
        .map(|p| crate::secret_text::SecretText::of(&p.text.text))
    else {
        return false;
    };
    let (Some(vid), Some(random), Some(uuids)) = (
        app.fresh_bytes(b"vault id", 16),
        app.fresh_bytes(b"vault seal", SEAL_RANDOM_LEN),
        app.fresh_bytes(b"vault entries", 32),
    ) else {
        if let Some(p) = app.vaults.state.prompt.as_mut() {
            p.error = Some(EN.no_randomness.to_string());
        }
        return false;
    };
    let mut vid16 = [0u8; 16];
    vid16.copy_from_slice(&vid);
    let mut seal = Zeroizing::new([0u8; SEAL_RANDOM_LEN]);
    seal.copy_from_slice(&random);
    let mut u32s = [0u8; 32];
    u32s.copy_from_slice(&uuids);
    let now = app.clock_now();
    // The memory step the prompt offers is the first; passes and lanes are
    // the format's.
    let cost = Cost {
        memory_kib: STEPS_KIB[0],
        passes: 3,
        lanes: 4,
    };
    let (out, name) = crate::bip85::kdbx_out(&app.outbox, &title);
    let Some(job) = app.vaults.store().kdbx_job(
        (id, at),
        out,
        (vid16, EN.vaults_vault, now),
        passphrase,
        cost,
        seal,
        u32s,
    ) else {
        return false;
    };
    app.start(
        Job::Kdbx(Box::new(KdbxWork {
            job,
            name: String::from(&*name),
        })),
        EN.vault_sealing,
    );
    true
}

/// The back half: the file to the Outbox, sealed, so it goes as it is.
pub(crate) fn kdbx_written(app: &mut Faraday, done: KdbxDone) {
    ended(app);
    let KdbxDone { file, name } = done;
    match file {
        Ok(file) => {
            app.vaults.state.prompt = None;
            app.vaults.state.focus = None;
            app.put_outbox(&name, file);
            app.toast(&fill(
                EN.vaults_waits_sealed,
                &[&name.to_string(), app.medium.for_the()],
            ));
        }
        Err(why) => {
            if let Some(p) = app.vaults.state.prompt.as_mut() {
                p.error = Some(match why {
                    faraday_vault::file::Error::Full { .. } => {
                        EN.vaults_entry_too_large_kdbx.to_string()
                    }
                    e => errors::error_text(&e),
                });
            }
        }
    }
}

/// The one Argon2id pass over 64 MiB that tells the unlock estimates how
/// fast this computer is, to run on the worker without a busy screen.
pub(crate) struct MeasureWork;

/// The measurement's answer: the milliseconds the pass took.
pub(crate) struct MeasureDone {
    /// None where the pass could not run: no figure is recorded.
    ms: Option<u64>,
}

impl MeasureWork {
    /// Costs an Argon2id run of one pass over 64 MiB, thrown away. The
    /// figure is the run's own time, on the worker, and is noted for the
    /// dev card ([`work::note_vault_took`]).
    pub(crate) fn run(self, c: &Compute) -> MeasureDone {
        let cost = Cost {
            memory_kib: 64 * 1024,
            passes: 1,
            lanes: 1,
        };
        let facts = faraday_vault::file::Facts {
            header: faraday_vault::kdbx::Header {
                minor: 0,
                cipher: faraday_vault::kdbx::Cipher::ChaCha20,
                compression: faraday_vault::kdbx::Compression::None,
                kdf: faraday_vault::kdbx::Kdf::Argon2id,
                cost,
                public_custom_data: None,
            },
            len: 0,
            size: Some(faraday_vault::file::STEP),
        };
        let from = std::time::Instant::now();
        let ran = faraday_keys::vault::measure(c, &facts);
        let took = from.elapsed();
        work::note_vault_took(
            VaultAct::Measured,
            cost,
            match &ran {
                Ok(()) => VaultEnded::Ran,
                Err(faraday_vault::file::Error::Memory(_)) => VaultEnded::NoMemory,
                Err(_) => VaultEnded::Refused,
            },
            work::vault_clock().map(|_| took),
        );
        MeasureDone {
            ms: ran
                .ok()
                .map(|()| u64::try_from(took.as_millis()).unwrap_or(u64::MAX).max(1)),
        }
    }
}

/// Sends the measurement to the worker, quiet: it has no busy screen, makes
/// no job stale and is not lost or started again by a lock, a Back or
/// another job, which waits its turn behind it. Inline it is run and
/// applied at once.
pub(crate) fn measure_start(app: &mut Faraday) {
    app.vaults.state.measure = super::area::Measure::Out;
    if app.worker.inline() {
        let c = app.compute();
        let done = MeasureWork.run(&c);
        measured(app, done);
    } else {
        app.worker.send_quiet(Job::Measure(Box::new(MeasureWork)));
    }
}

/// The back half: the figure goes in if none is yet (it is taken once).
/// Returns whether anything changed.
pub(crate) fn measured(app: &mut Faraday, done: MeasureDone) -> bool {
    app.vaults.state.measure = super::area::Measure::Idle;
    if app.vaults.state.ms_per_unit.is_some() {
        return false;
    }
    let Some(ms) = done.ms else {
        return false;
    };
    app.vaults.state.ms_per_unit = Some(ms);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pass that could not run leaves no time to show.
    #[test]
    fn a_failed_measuring_pass_records_no_figure() {
        let mut app = Faraday::new();
        assert!(!measured(&mut app, MeasureDone { ms: None }));
        assert_eq!(app.vaults.state.ms_per_unit, None);
        assert!(measured(&mut app, MeasureDone { ms: Some(900) }));
        assert_eq!(app.vaults.state.ms_per_unit, Some(900));
    }
}
