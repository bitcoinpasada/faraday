//! A vault's Argon2id runs as jobs on the worker (`docs/WORKERS.md` §6,
//! `docs/KEYS.md` §2): opening a vault (an Unlock on the Vaults tab, the
//! Spend tab, the boot import, a backup flow) and making one. The front
//! half reads the state and builds the job, the passphrase moved into
//! faraday-keys' [`StretchJob`]; the back half takes the answer, on the
//! main thread, and the open vault joins the store there.

use faraday_crypto::work::Compute;
use faraday_keys::job::{StretchJob, Stretched};
use faraday_vault::{Contents, Cost, Record, records::field, records::kind};
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
    Unlock { name: String, for_import: bool },
    /// The vault the Create form describes, named `typed`; `first` is the
    /// first passphrase, kept for a backup flow that unlocks the vault it
    /// made at once. Wiped as it drops.
    Create {
        typed: String,
        first: Zeroizing<String>,
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
    /// Costs an Argon2id run at the job's cost for each passphrase, the
    /// stretch noted ([`work::note_vault`]).
    pub(crate) fn run(self, c: &Compute) -> StretchDone {
        let stretched = self.job.run(c);
        work::note_vault(self.act, self.cost, VaultEnded::of_error(stretched.error()));
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
    app.out.dirty = true;
    app.out.commands.push_back(crate::Command::Draw);
}

/// Starts the job the frame drawn first was for.
pub(crate) fn work(app: &mut Faraday, work: Work) {
    let started = match work {
        Work::Unlock => unlock_start(app),
        Work::Create => create_start(app),
        _ => false,
    };
    if !started {
        ended(app);
    }
}

/// The vault job is over, or was never started: the screens stop
/// saying it runs.
pub(crate) fn ended(app: &mut Faraday) {
    if matches!(app.vaults.state.working, Some(Work::Unlock | Work::Create)) {
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
    let Some(bytes) = app.vault_bytes(f) else {
        return false;
    };
    let name = f.name.clone();
    let cost = f.header.cost;
    // Unlocked on the boot import's sheet: the unlock imports.
    let for_import = app.sheet == Some(crate::Sheet::Import) && app.import.state().is_some();
    let passphrase = std::mem::take(&mut app.vaults.state.passphrase.text);
    app.start(
        Job::Stretch(Box::new(StretchWork {
            job: StretchJob::open(bytes, passphrase),
            act: VaultAct::Unlocked,
            cost,
            plan: Plan::Unlock { name, for_import },
        })),
        EN.working_vault_open,
    );
    true
}

/// The front half of Create: the form's passphrases are copied into
/// the job (the form keeps them for a Back or an error) and wiped
/// with it. False when nothing started.
fn create_start(app: &mut Faraday) -> bool {
    // Everything random in a vault comes from fresh system entropy of
    // its own; a vault is never made before it has arrived.
    if app.fresh.pool_is_empty() {
        app.out.command(faraday_shell_api::Command::RequestEntropy);
        if let Some(c) = app.vaults.state.create.as_mut() {
            c.error = Some(EN.vaults_no_randomness.to_string());
        }
        return false;
    }
    let Some((_, mem, passes)) = app.vaults.form_cost() else {
        return false;
    };
    let Some(seed) = app.fresh(b"vault create") else {
        return false;
    };
    let seed = Zeroizing::new(seed);
    let Some(form) = app.vaults.create.as_ref() else {
        return false;
    };
    let cost = Cost {
        memory_kib: mem * 1024,
        passes,
        lanes: 1,
    };
    let first = Zeroizing::new(
        form.phrases
            .first()
            .map(|(a, _)| a.text.to_string())
            .unwrap_or_default(),
    );
    let phrases = form
        .phrases
        .iter()
        .map(|(a, _)| crate::secret_text::SecretText::of(&a.text))
        .collect();
    // Every slot starts with the vault's name: a slot opened under
    // pressure says nothing of the others (`docs/VAULT.md` §9, F57).
    let label = match form.name.text.trim() {
        "" => EN.vaults_vault,
        typed => {
            // A slot's label holds 64 bytes (`docs/VAULT.md` §7).
            let end = (0..=typed.len().min(64))
                .rev()
                .find(|&i| typed.is_char_boundary(i))
                .unwrap_or(0);
            &typed[..end]
        }
    };
    let contents: Vec<Contents> = form
        .phrases
        .iter()
        .map(|_| Contents {
            records: vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, label.as_bytes())],
        })
        .collect();
    let job = StretchJob::make(form.slot, cost, phrases, contents, seed);
    let typed = form.name.text.to_string();
    app.start(
        Job::Stretch(Box::new(StretchWork {
            job,
            act: VaultAct::Made,
            cost,
            plan: Plan::Create { typed, first },
        })),
        EN.working_vault_make,
    );
    true
}

/// The back half: a stretch's answer, applied on the main thread.
pub(crate) fn stretched(app: &mut Faraday, done: StretchDone) {
    ended(app);
    let StretchDone { stretched, plan } = done;
    match plan {
        Plan::Unlock { name, for_import } => unlocked(app, stretched, name, for_import),
        Plan::Create { typed, first } => created(app, stretched, &typed, &first),
    }
}

fn unlocked(app: &mut Faraday, stretched: Stretched, name: String, for_import: bool) {
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
        Ok((id, header)) => {
            app.vaults.state.typed_shown = false;
            app.vaults.state.focus = None;
            app.vaults.state.open.push(super::OpenVault {
                name,
                id,
                header,
                changes: 0,
                skip: Default::default(),
                each_shown: false,
                read: Default::default(),
                seal_digest: [0; 32],
                seal_warn: None,
            });
            app.vaults.state.current = app.vaults.open.len() - 1;
            if !app.vault_unlock_held(app.vaults.current, for_import) {
                app.vault_unlock_go_on(for_import);
            }
        }
        Err(e) => {
            app.vaults.state.unlock_error = Some(errors::error_text(e));
        }
    }
    if app.vaults.open.len() > before && app.vaults.older.is_none() {
        app.vault_unlock_flow();
    }
}

fn created(app: &mut Faraday, stretched: Stretched, typed: &str, first: &str) {
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
                c.error = Some(errors::error_text(e));
            }
        }
    }
}
