//! A vault's Argon2id runs as jobs on the worker (`docs/WORKERS.md` §6,
//! `docs/KEYS.md` §2): opening a vault (an Unlock on the Vaults tab, the
//! Spend tab, the boot import, a backup flow), making one, and writing a
//! KeePass export. The front
//! half reads the state and builds the job, the passphrase moved into
//! faraday-keys' [`StretchJob`]; the back half takes the answer, on the
//! main thread, and the open vault joins the store there.

use faraday_crypto::work::Compute;
use faraday_keys::job::{KdbxJob, StretchJob, Stretched};
use faraday_keys::vault::SEAL_RANDOM_LEN;
use faraday_vault::file::STEPS_KIB;
use faraday_vault::{Cost, records::field, records::kind};
use zeroize::Zeroizing;

use super::{VaultAction, Work, errors, vault_stem};
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
    },
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
        let stretched = self.job.run(c);
        let error = stretched.error();
        work::note_vault(self.act, self.cost, VaultEnded::of_error(error.as_ref()));
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
    let passphrase = std::mem::take(&mut app.vaults.state.passphrase.text);
    app.start(
        Job::Stretch(Box::new(StretchWork {
            job: StretchJob::open(bytes, passphrase),
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
        c.error = Some(EN.vaults_no_randomness.to_string());
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
            let end = (0..=typed.len().min(64))
                .rev()
                .find(|&i| typed.is_char_boundary(i))
                .unwrap_or(0);
            &typed[..end]
        }
    };
    let job = StretchJob::make(id16, label.into(), cost, form.size, phrase, seal, now);
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
        Plan::Create { typed, first, id } => created(app, stretched, &typed, &first, id),
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
        return;
    }
    let before = app.vaults.open.len();
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
            if !app.vault_unlock_held(app.vaults.current, sha, for_import) {
                app.vault_unlock_go_on(for_import);
            }
        }
        Err(e) => {
            app.vaults.state.unlock_error = Some(errors::error_text(&e));
        }
    }
    if app.vaults.open.len() > before && app.vaults.older.is_none() {
        app.vault_unlock_flow();
    }
}

fn created(app: &mut Faraday, stretched: Stretched, typed: &str, first: &str, id: [u8; 16]) {
    // A stick put in while the vault was made: the press's own refusal.
    if !app.stick.is_empty() {
        drop(stretched);
        let said = fill(EN.vaults_remove_the_first, &[app.medium.noun()]);
        if let Some(c) = app.vaults.state.create.as_mut() {
            c.error = Some(said);
        }
        return;
    }
    match stretched.into_made() {
        Ok(file) => {
            let name = app.free_vault_name(&vault_stem(typed));
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
            // Made inside a backup flow: unlocked at once with the
            // passphrase just typed twice, never leaving the flow.
            if app.flow_vault_made(&name, first) {
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
        say_no_randomness(app);
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
        Job::Kdbx(Box::new(KdbxWork { job, name })),
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
