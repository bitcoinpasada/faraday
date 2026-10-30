//! Vaults in the app (`docs/VAULT.md`, `docs/history/PLAN.md` §5.3 and §6.1): the
//! vaults the Inbox and Outbox hold, unlocking one slot of one, its
//! contents by kind, creating a vault, and sealing what changed when the
//! session locks.
//!
//! A vault file arrives in the Inbox on a stick visit, or is made here and
//! waits in the Outbox. Unlocking is never offered while a stick is
//! attached. An open slot is a secret: it counts for the stick rule, and
//! locking seals it into the Outbox when it changed. The slow part of
//! unlocking and creating, Argon2id, runs on the tick after a frame that
//! says it is working has been drawn.

mod area;
pub(crate) mod dice;
mod dice_screen;
pub(crate) mod errors;
pub mod load;
pub(crate) mod load_screen;
pub mod out;
pub(crate) mod panes_screen;
pub(crate) mod screen;
mod seals;
mod search;
mod strength;
mod summaries;

pub use area::*;
pub use seals::{OlderHold, SealFrom, SealOrder, SealSeen, SealWarn, times};
pub use search::{SearchAct, SearchIn};
pub(crate) use summaries::{summaries_decode, summaries_encode};

use crate::stick;
use crate::strings::EN;
use crate::strings::fill;
use faraday_crypto::work::Compute;
use faraday_keys::vault::{IntoRecord, RecordView, VaultId};
use faraday_vault::records::{self, field, kind};
use faraday_vault::{self as fv, Contents, Cost, Header, Record};
use std::collections::BTreeSet;
use zeroize::Zeroizing;

use crate::Sheet;
use crate::backup_flow::steps::Built;
use crate::wallet::fp_text;
use crate::work::{self, VaultAct, VaultEnded};
use crate::{Action, Faraday, FileKind, Item, Screen, gpg};
use faraday_bip::keys::Fingerprint;
use faraday_keys::vault::{GpgFacts as Facts, gpg_facts, gpg_field as f};
use faraday_pgp::{Armor, Reason, armor};
use std::rc::Rc;

impl Faraday {
    /// The open vaults and the vault screens' state.
    pub fn vaults(&self) -> &Vaults {
        self.vaults.vaults()
    }

    /// Costs what `described_rec` and `read_kept` cost. The wallet
    /// description record `r` holds, as its open vault keeps it read
    /// ([`Vaults::described_rec`]); a record that is no open vault's own (a
    /// copy) is read as a description no record keeps
    /// ([`Kept::read_kept`]), which costs a parse the first time.
    pub(crate) fn record_described(
        &self,
        c: &Compute,
        r: RecordView<'_>,
    ) -> Option<std::borrow::Cow<'_, Described>> {
        if let Some(d) = self.vaults.described_rec(c, r) {
            return Some(std::borrow::Cow::Borrowed(d));
        }
        let f = match r.kind() {
            kind::WALLET => field::WALLET,
            kind::PLAN => field::PLAN_WALLET,
            kind::WALLET_SHARE => field::SHARE_WALLET,
            _ => return None,
        };
        let policy = self.kept.read_kept(c, r.public_text(f)?)?;
        Some(std::borrow::Cow::Owned(Described::of(policy)))
    }
}

impl Faraday {
    /// Whether anything secret is in memory: a key, or an open vault.
    pub fn holds_secret(&self) -> bool {
        self.session.keys.holds_secret() || !self.vaults.open.is_empty()
    }

    /// Costs a parse of each key's words. The session's keys and wallets
    /// that no open vault holds, by label and by name (`docs/history/PLAN.md` §6.3:
    /// locking lists what was not saved). Kept in the drawing memo while a
    /// frame is drawn (`docs/SPEED.md` §3.3).
    ///
    /// Works out [`crate::drawing::View::unsaved`]. Costs a parse of each key's words.
    pub fn unsaved_now(&self, c: &Compute) -> (Vec<String>, Vec<String>) {
        self.vaults.unsaved(c, &self.session)
    }

    /// Costs what `lock_held`, `currency`, `words_id` and the rest it calls
    /// cost. What a lock, a write-out or a power-off keeps and what it
    /// wipes (`docs/history/SIMPLIFY.md` §4.5): each vault with its currency, the
    /// files For the stick, and each seed and wallet in memory with where
    /// else it is. At `power_off` a vault not written as it stands and the
    /// files For the stick are lost, not kept. Reads each seed's words and
    /// each open vault's wallets; kept for the frame while one is drawn
    /// (`docs/SPEED.md` §3.12).
    ///
    /// Reads each seed's words and each open vault's wallets: what
    /// [`crate::drawing::View::kept_wiped`] answers.
    pub fn kept_wiped_now(&self, c: &Compute, power_off: bool) -> KeptWiped {
        let mut out = KeptWiped::default();
        // Make a new wallet before its keys are kept anywhere ends with
        // the lock or the power-off (`docs/history/NEW-WALLET.md` §18).
        if self.lock_held(c) {
            out.wiped.push((EN.lock_held_ends.to_string(), true, None));
        }
        if power_off {
            for line in self.flows.power_lines() {
                out.wiped.push((line, true, None));
            }
        }
        for f in self.vault_files() {
            let open = f.open.and_then(|o| self.vaults.open.get(o));
            let title = open
                .map(|v| v.label(self.vaults.store()))
                .or_else(|| self.vault_summary(&f).map(|s| s.name.clone()))
                .unwrap_or_else(|| f.name.clone());
            let currency = self.currency(c, &f);
            let state = currency
                .as_ref()
                .map_or_else(|| self.medium.from_box().to_string(), Currency::line);
            let line = format!("{title} · {state}");
            let lost =
                power_off && matches!(currency, Some(Currency::NeverWritten | Currency::Changed));
            // A lock with no seed to seal with loses the changes.
            let unsealed = !power_off && !self.seeded() && open.is_some_and(|v| v.changes > 0);
            if unsealed {
                out.wiped.push((
                    format!("{title} · {}", EN.vaults_changes_not_sealed),
                    true,
                    None,
                ));
            } else if lost {
                out.wiped.push((line, true, None));
            } else {
                out.kept.push(line);
            }
        }
        let n = self.outbox.len();
        let files = format!(
            "{n} {} {}",
            if n == 1 {
                EN.vaults_file
            } else {
                EN.vaults_files
            },
            self.medium.for_the()
        );
        if power_off || self.mode_changing() {
            if n > 0 {
                out.wiped.push((files, true, None));
            }
        } else {
            out.kept.push(files);
        }
        // Where an open vault holds each seed: by its words' entropy from
        // any list (`Key::words_id`, `Vaults::words_id`), not parsed here;
        // a master seed's key by its record's id.
        // What is in no open vault can be saved into the one open now.
        let can_save = self.vaults.current_id().is_some();
        for (i, k) in self.session.keys.all().enumerate() {
            let vault = match self.session.keys.seed_record_id(i) {
                Some(id) => self.vaults.seed_vault(id),
                None => self
                    .session
                    .keys
                    .words_id(c, i)
                    .and_then(|w| self.vaults.words_vault(c, w)),
            };
            let save = (can_save
                && vault.is_none()
                && (k.has_words() || self.session.keys.holds_seed_bytes(i)))
            .then_some(Action::Vault(VaultAction::SaveKey(i)));
            let copy = self.seed_copy_checked(c, k.fingerprint.0);
            let (state, nowhere) = match (vault, copy) {
                (Some(v), true) => (fill(EN.vaults_in_copy_checked, &[&v.to_string()]), false),
                (Some(v), false) => (fill(EN.vaults_kept_in_vault, &[&v.to_string()]), false),
                (None, true) => (EN.vaults_copy_checked.to_string(), false),
                (None, false) => (EN.vaults_nowhere_else.to_string(), true),
            };
            // A key with no name of its own goes by "Seed {fingerprint}",
            // as the backup map writes it.
            let fp = crate::wallet::fp_text(k.fingerprint);
            let name = if k.label.is_empty() || k.label.eq_ignore_ascii_case(&fp) {
                fill(EN.vaults_seed_n, &[&fp.to_string()])
            } else {
                k.label.clone()
            };
            out.wiped.push((format!("{name} · {state}"), nowhere, save));
        }
        let in_files = |source: &str| {
            self.inbox
                .iter()
                .chain(self.outbox.iter())
                .any(|i| i.name == source)
        };
        self.vaults
            .wallets_wiped(c, &self.session, &in_files, &mut out.wiped);
        // What is nowhere else first, where it is read.
        out.wiped.sort_by_key(|(_, nowhere, _)| !nowhere);
        out
    }

    /// The vaults the Inbox and Outbox hold: one row per vault, the
    /// Outbox's copy standing for a vault that is in both.
    pub fn vault_files(&self) -> Vec<VaultFile> {
        let mut out: Vec<VaultFile> = Vec::new();
        for (in_outbox, items) in [(true, &self.outbox), (false, &self.inbox)] {
            for it in items.iter().filter(|i| i.kind == FileKind::Vault) {
                let Ok(header) = fv::read_header(&it.bytes) else {
                    continue;
                };
                // A copy of a vault listed already, under another name.
                if let Some(v) = out.iter_mut().find(|v| v.header.salt == header.salt) {
                    v.copies.push(it.name.clone());
                    continue;
                }
                let open = self
                    .vaults
                    .open
                    .iter()
                    .position(|o| o.header().salt == header.salt);
                out.push(VaultFile::new(it, in_outbox, header, open));
            }
        }
        // Open vaults whose file is in neither box still list.
        out
    }

    /// What a flow that needs a vault offers while none is open, coming
    /// back to `back`: Make a vault when there is no vault file, Unlock
    /// {name} when one is locked, Unlock a vault, through the list, when
    /// several are. `None` while a vault is open.
    pub fn vault_way(&self, back: Screen) -> Option<(String, crate::Action)> {
        if self.vaults.open.get(self.vaults.current).is_some() {
            return None;
        }
        Some(self.vault_unlock_way(back).unwrap_or_else(|| {
            (
                EN.vaults_make_a_vault.to_string(),
                crate::Action::Vault(VaultAction::CreateFrom(back)),
            )
        }))
    }

    /// The way to a vault already on a stick read this power-on, whether
    /// or not another is open: Unlock {name} for the one locked, Unlock a
    /// vault, through the list, for several. `None` with none locked.
    pub fn vault_unlock_way(&self, back: Screen) -> Option<(String, crate::Action)> {
        let files = self.vault_files();
        let locked: Vec<usize> = (0..files.len())
            .filter(|&i| files[i].open.is_none())
            .collect();
        let (label, a) = match locked[..] {
            [] => return None,
            [k] => (
                fill(EN.vaults_unlock, &[&files[k].name.to_string()]),
                VaultAction::OpenFrom(k, back),
            ),
            _ => (
                EN.vaults_unlock_way.to_string(),
                VaultAction::ListFrom(back),
            ),
        };
        Some((label, crate::Action::Vault(a)))
    }

    /// The flow a vault is being made or unlocked for, by name: the way
    /// back's label and the "Then:" on Create a vault.
    pub fn back_name(&self, s: Screen) -> &'static str {
        match s {
            Screen::Create => EN.vaults_make_a_new_wallet,
            Screen::Restore => EN.vaults_restore_an_existing,
            Screen::Family => EN.vaults_spend,
            Screen::Backup => EN.vaults_back_up_a_wallet,
            Screen::BackupFlow => self
                .flow()
                .map_or(crate::strings::EN.backup_flow_heading, |f| f.kind.name()),
            Screen::Bip85 => EN.vaults_child_seeds,
            Screen::Lightning => EN.vaults_lightning_node,
            Screen::Home if self.import.state().is_some() => EN.vaults_import,
            Screen::VaultContents => crate::catalog::TILES
                .iter()
                .find(|t| t.go == crate::catalog::Go::VaultCategory(self.vaults.category))
                .map_or(
                    CATEGORIES[self.vaults.category.min(CATEGORIES.len() - 1)].0,
                    |t| t.name,
                ),
            _ => crate::compact::screen_name(s, self.medium),
        }
    }

    /// Where the way back from making or unlocking a vault for `s` goes,
    /// by name: Tools for a vault category with no vault open, as
    /// [`VaultAction::Back`] goes there; else the flow.
    pub fn back_link_name(&self, s: Screen) -> &'static str {
        if s == Screen::VaultContents && self.vaults.open.get(self.vaults.current).is_none() {
            EN.vaults_tools
        } else {
            self.back_name(s)
        }
    }

    /// The secret sheet closes while a vault is made or unlocked for it;
    /// its secret is kept, and the sheet opens again when the vault does.
    fn secret_sheet_waits(&mut self) {
        if self.sheet == Some(crate::Sheet::SecretOut) {
            self.sheet = None;
        }
    }

    fn vault_bytes(&self, f: &VaultFile) -> Option<Vec<u8>> {
        let items = if f.in_outbox {
            &self.outbox
        } else {
            &self.inbox
        };
        items
            .iter()
            .find(|i| i.name == f.name)
            .map(|i| i.bytes.clone())
    }

    /// A seal's nonce (`docs/VAULT.md` §5, F158): a fresh answer for "vault
    /// seal" when one waits, else the seed with a count, so a lock never
    /// waits; `None` before the seed, and the shell is asked again.
    pub(crate) fn vault_seal_nonce(&mut self) -> Option<[u8; 32]> {
        if !self.fresh.seeded() {
            self.out.command(faraday_shell_api::Command::RequestEntropy);
            return None;
        }
        if let Some(fresh) = self.fresh(b"vault seal") {
            return Some(fresh);
        }
        self.vaults.state.draws += 1;
        let what = [b"faraday vault draw".as_slice(), b"nonce"].concat();
        self.fresh.draw(&what, &self.vaults.draws.to_le_bytes())
    }

    /// Runs a vault action.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn vault_act(&mut self, a: VaultAction) {
        let c = self.compute();
        use VaultAction as V;
        // A small panel's contents page changes to another page: it starts
        // at its top. A kind's tab keeps its own place (`show_category`).
        if matches!(
            a,
            V::Item(_) | V::ItemBack | V::Add | V::AddKind(_) | V::Edit | V::Rename | V::Search(_)
        ) {
            self.list_offset = 0.0;
        }
        match a {
            V::Search(s) => self.vaults.search_act(s),
            V::Open(i) => {
                let files = self.vault_files();
                if let Some(f) = files.get(i) {
                    match f.open {
                        Some(o) => {
                            self.vaults.state.current = o;
                            self.vaults.state.item_open = false;
                            self.screen = Screen::VaultContents;
                        }
                        None => {
                            self.vaults.state.pick = i;
                            self.vaults.state.unlock_error = None;
                            self.vaults.state.focus =
                                self.may_load_keys().then_some(Focus::Passphrase);
                            self.screen = Screen::Unlock;
                            // The import sheet's way back is set afresh by
                            // `OpenFrom` below each time; one left from
                            // an Unlock abandoned by the sidebar does not
                            // linger. Create's and Restore's stay: a vault
                            // just made there is opened through here.
                            if self.vaults.back_to == Some(Screen::Home) {
                                self.vaults.state.back_to = None;
                            }
                            self.vaults.measure();
                        }
                    }
                }
            }
            V::Create | V::CreateFrom(_) => {
                self.vaults.state.back_to = match a {
                    V::CreateFrom(s) => Some(s),
                    _ => None,
                };
                self.vaults.state.from_home = self.screen == Screen::Home;
                self.secret_sheet_waits();
                self.vaults.state.create = Some(CreateForm::default());
                self.vaults.state.just_made = None;
                // It opens on Name and passphrases: the first passphrase
                // takes the typing, as Continue into that card does.
                self.vaults.state.focus = self.may_load_keys().then_some(Focus::Phrase(0, false));
                self.screen = Screen::CreateVault;
                self.vaults.measure();
            }
            V::OpenFrom(i, back) => {
                self.vault_act(V::Open(i));
                if self.screen == Screen::Unlock {
                    self.vaults.state.back_to = Some(back);
                    self.secret_sheet_waits();
                }
            }
            V::ListFrom(back) => {
                self.vaults.state.back_to = Some(back);
                self.vaults.state.just_made = None;
                self.secret_sheet_waits();
                self.screen = Screen::Vaults;
            }
            V::EachShown(v) => {
                if let Some(o) = self.vaults.state.open.get_mut(v) {
                    o.each_shown = !o.each_shown;
                }
            }
            V::LoadKeyOf(fp, at) => {
                let found = self
                    .slot_of(&c, fp, at)
                    .and_then(|slot| self.vault_key_for(&c, &slot));
                let fp = faraday_bip::keys::Fingerprint(fp);
                if let Some((v, i)) = found {
                    let (k, _) = self.vault_load_set(v, &BTreeSet::from([i]));
                    if k > 0 {
                        self.toast(&fill(
                            EN.vaults_loaded_note,
                            &[
                                &crate::wallet::fp_text(fp).to_string(),
                                &self.vaults.open[v].name.to_string(),
                            ],
                        ));
                    }
                }
            }
            V::LoadWithKeys(v) => {
                let mut set = BTreeSet::new();
                let open: Vec<VaultWallet> = self
                    .vault_wallets_with_keys(&c, v)
                    .into_iter()
                    .filter(|w| !w.done)
                    .collect();
                // One wallet has no checkbox: the button is its own.
                let one = open.len() == 1;
                for w in open {
                    if w.chosen || one {
                        set.insert(w.record);
                        set.extend(w.keys);
                    }
                }
                let (keys, wallets) = self.vault_load_set(v, &set);
                if self.screen == Screen::Family {
                    self.family_settle();
                    self.family_vault_loaded();
                    self.family_opened();
                }
                self.toast(&fill(
                    EN.vaults_and_loaded,
                    &[
                        &wallets.to_string(),
                        (if wallets == 1 {
                            EN.vaults_wallet_inline
                        } else {
                            EN.vaults_wallets_inline
                        }),
                        &keys.to_string(),
                        (if keys == 1 {
                            EN.vaults_seed_inline
                        } else {
                            EN.vaults_seeds
                        }),
                    ],
                ));
            }
            V::OpenWalletOf(v, r) => {
                if let Some(w) = self.vault_load_wallet(v, r) {
                    self.act(Action::OpenWallet(w));
                }
            }
            V::ChecklistOf(v, r) => {
                if let Some(w) = self.vault_load_wallet(v, r) {
                    self.act(Action::BackupChecklist(w));
                }
            }
            V::Back => {
                self.vaults.state.just_made = None;
                self.flow_create_left();
                if let Some(s) = self.vaults.state.back_to.take() {
                    self.vaults.state.passphrase.clear();
                    self.vaults.state.typed_shown = false;
                    self.vaults.state.unlock_error = None;
                    self.vaults.state.create = None;
                    self.vaults.state.dice = None;
                    self.vaults.state.focus = None;
                    // The way back from making or unlocking a vault for a
                    // secret is its Cancel: the secret is dropped.
                    self.secret_cancel();
                    // A vault category from Tools with no vault open has
                    // nothing to show: back to Tools.
                    self.screen = if s == Screen::VaultContents
                        && self.vaults.open.get(self.vaults.current).is_none()
                    {
                        Screen::Catalog
                    } else {
                        s
                    };
                }
            }
            V::Choose(v, r) => {
                if let Some(o) = self.vaults.state.open.get_mut(v)
                    && !o.skip.remove(&r)
                {
                    o.skip.insert(r);
                }
            }
            V::ChooseAll(v) => {
                let rows = self.vault_rows_now(&c, v);
                if let Some(o) = self.vaults.state.open.get_mut(v) {
                    let open: Vec<usize> = rows
                        .iter()
                        .filter(|r| !r.loaded)
                        .map(|r| r.record)
                        .collect();
                    if open.iter().all(|r| !o.skip.contains(r)) {
                        o.skip.extend(open);
                    } else {
                        for r in open {
                            o.skip.remove(&r);
                        }
                    }
                }
            }
            V::LoadAll(v) | V::LoadChosen(v) => {
                let all = matches!(a, V::LoadAll(_));
                let (keys, wallets) = self.vault_load(v, all);
                self.wallets.set_wallet(0);
                // The list stays on screen with what loaded marked; the
                // Spend tab goes on to what it needs next.
                if self.screen == Screen::Family {
                    self.family_settle();
                    self.family_vault_loaded();
                    self.family_opened();
                }
                self.toast(&fill(
                    EN.vaults_and_loaded,
                    &[
                        &keys.to_string(),
                        (if keys == 1 {
                            EN.vaults_seed_inline
                        } else {
                            EN.vaults_seeds
                        }),
                        &wallets.to_string(),
                        (if wallets == 1 {
                            EN.vaults_wallet_inline
                        } else {
                            EN.vaults_wallets_inline
                        }),
                    ],
                ));
            }
            V::CStep(k) => {
                if let Some(c) = self.vaults.state.create.as_mut() {
                    c.open = if c.open == Some(k) { None } else { Some(k) };
                    c.scroll.follow = true;
                    if c.open == Some(vstep::PHRASES) {
                        self.vaults.state.focus = Some(Focus::Phrase(0, false));
                    } else if matches!(self.vaults.focus, Some(Focus::Phrase(..) | Focus::Name)) {
                        // Typing goes nowhere on a card with no field.
                        self.vaults.state.focus = None;
                    }
                }
            }
            V::CNext(k) => {
                if let Some(c) = self.vaults.state.create.as_mut()
                    && usize::from(k) < vstep::COUNT
                {
                    c.done[k as usize] = true;
                    c.open = c.steps().iter().copied().find(|&i| !c.done[i as usize]);
                    c.scroll.follow = true;
                    c.error = None;
                    if c.open == Some(vstep::PHRASES) {
                        self.vaults.state.focus = Some(Focus::Phrase(0, false));
                    }
                }
            }
            V::CHere => {
                if let Some(c) = self.vaults.state.create.as_mut() {
                    c.not_here = !c.not_here;
                    c.error = None;
                }
            }
            V::CMachine(k) => {
                if let Some(c) = self.vaults.state.create.as_mut()
                    && k < MACHINES.len()
                {
                    c.on[k] = !c.on[k];
                    c.error = None;
                }
            }
            V::CPreset(k) => {
                if let Some(c) = self.vaults.state.create.as_mut() {
                    c.preset = Some(k.min(CUSTOM));
                    c.error = None;
                }
            }
            V::CSizeRow(k) => {
                if let Some(c) = self.vaults.state.create.as_mut()
                    && let Some(&(_, preset, on)) = SIZES.get(k)
                {
                    c.preset = Some(preset);
                    c.on = on;
                    c.not_here = false;
                    c.error = None;
                }
            }
            V::CCustomise => {
                if let Some(c) = self.vaults.state.create.as_mut() {
                    // The three cards open on the size's values, to be
                    // walked through in order.
                    c.customise = true;
                    for k in [vstep::WHERE, vstep::COST, vstep::SIZE] {
                        c.done[usize::from(k)] = false;
                    }
                    if matches!(
                        self.vaults.state.focus,
                        Some(Focus::Phrase(..) | Focus::Name)
                    ) {
                        self.vaults.state.focus = None;
                    }
                    c.open = Some(vstep::WHERE);
                    c.scroll.follow = true;
                    c.error = None;
                }
            }
            V::CMemory(k) => {
                if let Some(c) = self.vaults.state.create.as_mut() {
                    c.memory = CUSTOM_MEMORY[k.min(CUSTOM_MEMORY.len() - 1)];
                    c.preset = Some(CUSTOM);
                }
            }
            V::CPasses(k) => {
                if let Some(c) = self.vaults.state.create.as_mut() {
                    c.passes = CUSTOM_PASSES[k.min(CUSTOM_PASSES.len() - 1)];
                    c.preset = Some(CUSTOM);
                }
            }
            V::CSize(k) => {
                if let Some(c) = self.vaults.state.create.as_mut() {
                    c.slot = fv::SLOT_SIZES[k.min(3)];
                }
            }
            V::CAddPhrase => {
                if let Some(c) = self.vaults.state.create.as_mut()
                    && c.phrases.len() < fv::SLOTS
                {
                    c.phrases.push((TextBox::default(), TextBox::default()));
                    c.from_dice.push(None);
                    self.vaults.state.focus = Some(Focus::Phrase(c.phrases.len() - 1, false));
                }
            }
            V::CRemovePhrase(i) => {
                if let Some(c) = self.vaults.state.create.as_mut()
                    && c.phrases.len() > 1
                    && i < c.phrases.len()
                {
                    c.phrases.remove(i);
                    if i < c.from_dice.len() {
                        c.from_dice.remove(i);
                    }
                    self.vaults.state.focus = None;
                }
            }
            V::CFocus(i, second) => {
                self.vaults.state.focus = self.may_load_keys().then_some(Focus::Phrase(i, second));
            }
            V::CName => self.vaults.state.focus = Some(Focus::Name),
            V::CShow => {
                if let Some(c) = self.vaults.state.create.as_mut() {
                    c.shown = !c.shown;
                }
            }
            V::CGo => self.vault_create_check(),
            V::Pick(i) => {
                self.vaults.state.pick = i;
                self.vaults.state.unlock_error = None;
                self.vaults.state.focus = self.may_load_keys().then_some(Focus::Passphrase);
            }
            V::FocusPassphrase => {
                self.vaults.state.focus = self.may_load_keys().then_some(Focus::Passphrase);
            }
            V::Unlock => self.vault_unlock_check(),
            V::Show(i) => {
                self.gpg_page_reset();
                if i < self.vaults.open.len() {
                    self.vaults.state.current = i;
                    self.vaults.state.form = None;
                    self.vaults.state.saving = false;
                    self.vaults.state.item_open = false;
                    self.vaults.set_item_offset(0.0);
                }
            }
            V::Category(k) => {
                self.gpg_page_reset();
                self.vaults
                    .show_category(k, &mut self.list_offset, self.compact);
                self.vaults.set_item_offset(0.0);
                self.vaults.state.category_chosen = true;
                self.vaults.state.form = None;
                self.vaults.state.saving = false;
                self.vaults.state.item_open = false;
            }
            V::Item(k) => {
                self.gpg_page_reset();
                self.vaults.state.item[self.vaults.category] = k;
                self.vaults.set_item_offset(0.0);
                self.vaults.state.form = None;
                self.vaults.state.saving = false;
                self.vaults.state.item_open = true;
                // A key made before revocations were kept gets its one
                // now, on first use of its page.
                self.gpg_keep_revocation();
            }
            V::ItemBack => {
                self.gpg_page_reset();
                self.vaults.state.item_open = false;
                self.vaults.state.form = None;
                self.vaults.state.prompt = None;
                self.vaults.state.saving = false;
                self.vaults.state.sb_images = false;
                self.vaults.state.sb_sign = None;
                self.vaults.state.focus = None;
            }
            V::Reveal(n) => self.vaults.toggle_shown(self.vault_on_page(), n),
            V::ShowSeedQr => self.vaults.toggle_seedqr(&c, self.vault_on_page()),
            V::WalletQr | V::TotpQr => self.vault_qr_act(a),
            V::SaveToStick => self.vault_stick_save_open(),
            V::SaveTick(k) => {
                if let Some(f) = self
                    .vaults
                    .state
                    .stick_save
                    .as_mut()
                    .and_then(|s| s.files.get_mut(usize::from(k)))
                {
                    f.ticked = !f.ticked;
                }
            }
            V::SaveToStickGo => self.vault_stick_save_ask(),
            V::LoadTick(_) | V::LoadTickAll | V::LoadTicked | V::LoadNotNow => {
                self.load_sheet_act(a)
            }
            V::OlderOpen | V::OlderBack => self.vault_older_act(a),
            V::PassphraseKey(w) => self.pass_seeds_open(w),
            V::PassphrasePick(v, r) => self.pass_seed_pick(v, r),
            V::PassphraseList => self.pass_seed_list(),
            V::HoldDelete => self.vault_delete(),
            V::HoldWipe => self.delete_go(),
            V::ToggleLoad => {
                // A key that loads with a wallet of this vault has no
                // setting of its own (F62).
                let Some(at) = self.vault_selected_index() else {
                    return;
                };
                if self.vault_key_wallet(&c, self.vaults.current, at).is_some() {
                    return;
                }
                if let Some(id) = self.vaults.current_id()
                    && let Some(loads) = self
                        .vaults
                        .records(id)
                        .get(at)
                        .filter(|r| matches!(r.kind(), kind::KEY | kind::WALLET))
                        .map(|r| r.loads_at_unlock())
                {
                    self.vaults
                        .edit(id, |s| s.set_loads_at_unlock(id, at, !loads));
                    self.vaults.changed(id);
                }
            }
            V::Load if !stick::refuses(self, false) => self.vault_load_selected(),
            V::Edit if !stick::refuses(self, true) => self.vault_form(true),
            V::PromptGo if !stick::refuses(self, false) => self.vault_prompt_go(),
            V::Load | V::Edit | V::PromptGo => {}
            V::AddKind(k) => {
                self.vault_act(V::Category(k));
                self.vault_act(V::Add);
            }
            V::WriteOut => self.screen = Screen::Files,
            V::Add => {
                let kinds = CATEGORIES[self.vaults.category].1;
                // A key that cannot be made again from seed words is made
                // by the backup flow, which decides where it will be kept
                // first (`docs/BACKUP-FLOW.md`).
                if let Some(k) = crate::backup_flow::Kind::of_category(self.vaults.category) {
                    self.vaults.state.form = None;
                    return self.flow_begin(k);
                }
                self.vaults.state.sb_images = false;
                self.vaults.state.sb_sign = None;
                if kinds.contains(&kind::KEY) || kinds.contains(&kind::WALLET) {
                    self.vaults.state.saving = !self.vaults.saving;
                    self.vaults.state.form = None;
                } else {
                    self.vault_form(false);
                }
            }
            V::SaveKey(k) => self.vault_save_key(k),
            V::SaveWallet(k) => self.vault_save_wallet(k),
            V::FocusField(k) => stick::focus(self, Focus::Field(k)),
            V::FormSave => self.vault_form_save(),
            V::FormCancel => {
                self.vaults.state.form = None;
                self.vaults.state.focus = None;
            }
            V::Rename => {
                let label = self
                    .vaults
                    .open
                    .get(self.vaults.current)
                    .map(|o| o.label(self.vaults.store()));
                if let Some(label) = label {
                    let mut name = TextBox::default();
                    name.set(&label);
                    self.vaults.state.form = Some(Form {
                        kind: kind::SLOT_LABEL,
                        record: None,
                        fields: vec![(field::LABEL, EN.vaults_name, name, false)],
                    });
                    self.vaults.state.saving = false;
                    stick::focus(self, Focus::Field(0));
                }
            }
            V::ShowTyped => self.vaults.state.typed_shown = !self.vaults.typed_shown,
            V::ImportEntries(k) => self.vault_import_entries(&[k]),
            V::AddFile(k) => self.vault_add_file(k),
            V::ImportAllEntries => {
                let all: Vec<usize> = self
                    .inbox
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.kind == FileKind::Entries)
                    .map(|(k, _)| k)
                    .collect();
                self.vault_import_entries(&all);
            }
            V::ScanEntry => {
                let mut scan = crate::ScanState::default();
                scan.purpose = crate::ScanPurpose::VaultEntry;
                self.camera.open(scan);
                self.sheet = Some(crate::Sheet::Scan);
                self.out.command(faraday_shell_api::Command::CameraOn);
            }
            V::LoadWithPassphrase => {
                self.vaults.state.prompt = Some(Prompt {
                    purpose: Purpose::KeyPassphrase,
                    text: TextBox::default(),
                    error: None,
                });
                stick::focus(self, Focus::Prompt);
            }
            V::SaveKeyWithPassphrase(k) => self.vault_save_key_with(k, true),
            V::FocusPrompt => stick::focus(self, Focus::Prompt),
            V::ExportKdbx => {
                self.vaults.state.prompt = Some(Prompt {
                    purpose: Purpose::Kdbx,
                    text: TextBox::default(),
                    error: None,
                });
                stick::focus(self, Focus::Prompt);
            }
            V::PromptCancel => {
                self.vaults.state.prompt = None;
                self.vaults.state.focus = None;
            }
            // A key made in the backup flow signs nothing until every place
            // of its plan is written: the action opens the flow.
            V::GpgSignPick
            | V::GpgSign
            | V::GpgHoldSign
            | V::GpgRenew
            | V::SbEnrol
            | V::SbPick(_)
            | V::SbHoldSign
                if self.selected_waits().is_some() =>
            {
                self.flow_resume_selected();
            }
            V::GpgYears(y) => self.vaults.state.gpg_years = y,
            V::GpgExport => self.gpg_export(),
            V::GpgExportRevocation => self.gpg_export_revocation(),
            V::GpgRevocationPage => {
                self.gpg_keep_revocation();
                self.vault_form_for_revocation();
            }
            V::GpgReason(n) => self.vaults.state.revoke_reason = n.min(3),
            V::GpgRenewTerm(y) => self.vaults.state.renew_years = Some(y),
            V::GpgHoldRevoke => self.gpg_mark_revoked(),
            V::GpgRenew => {
                if !self.time_asked() {
                    self.gpg_renew();
                }
            }
            V::GpgSignPick => {
                self.vaults.state.signing = !self.vaults.signing;
                self.vaults.state.form = None;
                self.vaults.state.sign_key = self.gpg_fingerprint();
                if self.vaults.signing {
                    self.gpg_default_ticks();
                } else {
                    self.vaults.state.sign_ticked.clear();
                }
            }
            V::GpgTick(k) => self.gpg_tick(k),
            V::GpgTickAll => self.gpg_tick_all(),
            // Sign opens the sheet; only its hold signs, and it reads the
            // time the sheet shows rather than asking for it (F63).
            V::GpgSign => self.gpg_sign_sheet_open(),
            V::GpgHoldSign => self.gpg_sign_sheet_hold(),
            V::SbOwnOnly(on) => self.vaults.state.sb_own_only = on,
            V::SbEnrol => {
                if !self.time_asked() {
                    self.sb_enrol();
                }
            }
            V::SbImages => {
                self.vaults.state.sb_images = !self.vaults.sb_images;
                self.vaults.state.sb_sign = None;
                self.vaults.state.sb_checked = None;
            }
            V::SbPick(k) => self.vaults.state.sb_sign = self.sb_release_now(k).map(|r| (k, r)),
            V::SbHoldSign => {
                if let Some((k, _)) = self.vaults.sb_sign {
                    self.sb_sign(k);
                }
            }
            V::SbCheck(k) => self.sb_check(k),
            // The dice entry's and the roller's.
            _ => dice::act(self, a),
        }
    }

    fn vault_import_entries(&mut self, files: &[usize]) {
        if self.vaults.open.is_empty() {
            self.toast(EN.vaults_unlock_a_vault);
            return;
        }
        let mut records = Vec::new();
        for &k in files {
            if let Some(it) = self.inbox.get(k) {
                records.extend(fv::entries::parse(&String::from_utf8_lossy(&it.bytes)));
            }
        }
        let n = records.len();
        let Some(id) = self.vaults.current_id() else {
            return;
        };
        if let Some(why) = self.vaults.no_room(id, &records) {
            self.toast(&why);
            return;
        }
        self.vaults.edit(id, |s| {
            for r in records {
                s.insert(id, r);
            }
        });
        if n > 0 {
            self.vaults.changed(id);
        }
        self.vaults.state.category = 2;
        self.vaults.state.form = None;
        self.vaults.state.saving = false;
        self.screen = Screen::VaultContents;
        self.toast(&fill(
            EN.vaults_added,
            &[
                &n.to_string(),
                (if n == 1 {
                    EN.vaults_entry_inline
                } else {
                    EN.vaults_entries_inline
                }),
            ],
        ));
    }

    /// A TOTP setup code the camera read, into a new entry's form; text of
    /// several entries, into the vault.
    pub(crate) fn vault_scanned_entry(&mut self, text: &str) -> Result<(), String> {
        let t = text.trim();
        // Scanned from the codes screen (`docs/TOTP.md` §5.3): back to
        // it, with the new row in the list. Scanned from the Add… menu
        // or the entries list's own button: back to Vault contents, as
        // before.
        let back = if self.screen == Screen::Codes {
            Screen::Codes
        } else {
            Screen::VaultContents
        };
        if let Some((title, account)) = fv::entries::otpauth_names(t) {
            self.vaults.state.category = 2;
            self.vault_form(false);
            if let Some(f) = self.vaults.state.form.as_mut() {
                for (n, _, b, _) in f.fields.iter_mut() {
                    match *n {
                        field::TITLE => b.set(&title),
                        field::USERNAME => b.set(&account),
                        field::TOTP => b.set(t),
                        _ => {}
                    }
                }
            }
            self.screen = back;
            return Ok(());
        }
        if fv::entries::looks_like_entries(t) {
            let records = fv::entries::parse(t);
            if let Some(id) = self.vaults.current_id() {
                let n = records.len();
                if let Some(why) = self.vaults.no_room(id, &records) {
                    self.toast(&why);
                    return Err(why);
                }
                self.vaults.edit(id, |s| {
                    for r in records {
                        s.insert(id, r);
                    }
                });
                self.vaults.changed(id);
                self.vaults.state.category = 2;
                self.screen = back;
                self.toast(&fill(
                    EN.vaults_added,
                    &[
                        &n.to_string(),
                        (if n == 1 {
                            EN.vaults_entry_inline
                        } else {
                            EN.vaults_entries_inline
                        }),
                    ],
                ));
                return Ok(());
            }
        }
        Err(EN.vaults_not_a_totp_setup.to_string())
    }

    fn vault_prompt_go(&mut self) {
        let c = self.compute();
        let Some(p) = self.vaults.state.prompt.as_mut() else {
            return;
        };
        match p.purpose {
            Purpose::ForWallet(v, r) => self.pass_seed_try(v, r),
            Purpose::Kdbx => {
                if p.text.text.chars().count() < 8 {
                    p.error = Some(EN.vaults_a_passphrase.to_string());
                } else {
                    self.vaults.state.working = Some(Work::Kdbx);
                    self.vaults.state.drawn = false;
                }
            }
            Purpose::KeyPassphrase => {
                let passphrase = Zeroizing::new(p.text.text.to_string());
                let Some(at) = self.vault_selected_index() else {
                    return;
                };
                let Some(id) = self.vaults.current_id() else {
                    return;
                };
                let Some(faraday_keys::vault::KeyLoad::Done(loaded)) =
                    self.vaults.store().load_key_with_passphrase(
                        &c,
                        &mut self.session.keys,
                        (id, at),
                        &passphrase,
                        None,
                    )
                else {
                    return;
                };
                match loaded {
                    Ok(fp) => {
                        self.vaults.state.prompt = None;
                        self.vaults.state.focus = None;
                        self.refresh_spend();
                        self.toast(&fill(
                            EN.vaults_key_is_loaded,
                            &[&crate::wallet::fp_text(fp).to_string()],
                        ));
                    }
                    Err(e) => {
                        if let Some(p) = self.vaults.state.prompt.as_mut() {
                            p.error = Some(e.text());
                        }
                    }
                }
            }
        }
    }

    /// What a tick does for vaults: runs work whose frame has been drawn,
    /// finishes a measurement, and fires a hold. Returns whether to redraw.
    pub(crate) fn vault_tick(&mut self, now_ms: u64) -> bool {
        let c = self.compute();
        let mut redraw = false;
        if let Some(t0) = self.vaults.state.measure_from.take() {
            self.vaults.state.ms_per_unit = Some(now_ms.saturating_sub(t0).max(1));
            redraw = true;
        }
        if let Some(work) = self.vaults.working
            && self.vaults.drawn
        {
            self.vaults.state.working = None;
            match work {
                Work::Measure => {
                    let header = Header {
                        cost: Cost {
                            memory_kib: 64 * 1024,
                            passes: 1,
                            lanes: 1,
                        },
                        slot_len: fv::SLOT_SIZES[0],
                        salt: [0; 32],
                    };
                    self.vaults.state.measure_from = Some(now_ms);
                    faraday_keys::vault::measure(&c, &header);
                    work::note_vault(VaultAct::Measured, header.cost, VaultEnded::Ran);
                }
                Work::Unlock => self.vault_unlock(),
                Work::FlowMake => self.flow_make_work(),
                Work::Kdbx => self.vault_kdbx(),
                Work::Create => self.vault_create(),
            }
            redraw = true;
        }
        // A hold acts on the tick that completes it.
        if let Some((held, _)) = self.pointer.pressed()
            && is_hold(held)
        {
            if now_ms.saturating_sub(self.vaults.pressed_at) >= HOLD_MS {
                self.pointer.set_pressed(None);
                match held {
                    Action::Vault(v) => self.vault_act(v),
                    Action::Flow(f) => self.flow_act(f),
                    _ => self.hold_done(held, &c),
                }
            }
            redraw = true;
        }
        redraw | dice::tick(self, now_ms) | self.vault_older_settle()
    }

    fn vault_unlock_check(&mut self) {
        let files = self.vault_files();
        let Some(f) = files.get(self.vaults.pick) else {
            return;
        };
        let error = if !self.may_load_keys() {
            Some(fill(EN.vaults_remove_to_unlock, &[self.medium.noun()]))
        } else if self.vaults.passphrase.text.is_empty() {
            Some(EN.vaults_type_the_passphrase.to_string())
        } else {
            self.vaults
                .unlock_memory_refusal(f.header.cost, f.header.slot_len)
        };
        self.vaults.state.unlock_error = error;
        if self.vaults.unlock_error.is_none() {
            self.vaults.state.working = Some(Work::Unlock);
            self.vaults.state.drawn = false;
        }
    }

    fn vault_unlock(&mut self) {
        let before = self.vaults.open.len();
        self.vault_unlock_inner();
        if self.vaults.open.len() > before && self.vaults.older.is_none() {
            self.vault_unlock_flow();
        }
    }

    /// Unlocked inside a backup flow: it goes on, and loads what it
    /// needs itself, so no Load sheet waits over it (F62).
    pub(crate) fn vault_unlock_flow(&mut self) {
        let screen = self.screen;
        self.flow_vault_opened();
        let from_tab = self.vaults.load.as_ref().is_some_and(|l| !l.boot);
        if self.screen != screen && from_tab && self.sheet == Some(crate::Sheet::UnlockLoad) {
            self.vaults.state.load = None;
            self.sheet = None;
        }
    }

    fn vault_unlock_inner(&mut self) {
        let c = self.compute();
        let files = self.vault_files();
        let Some(f) = files.get(self.vaults.pick) else {
            return;
        };
        let Some(bytes) = self.vault_bytes(f) else {
            return;
        };
        let name = f.name.clone();
        // Unlocked on the boot import's sheet: the unlock imports.
        let for_import = self.sheet == Some(crate::Sheet::Import) && self.import.state().is_some();
        let result = self.vaults.state.unlock_noted(&c, bytes, f.header.cost);
        match result {
            Ok((id, header)) => {
                self.vaults.state.passphrase.clear();
                self.vaults.state.typed_shown = false;
                self.vaults.state.focus = None;
                self.vaults.state.open.push(OpenVault {
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
                self.vaults.state.current = self.vaults.open.len() - 1;
                if !self.vault_unlock_held(self.vaults.current, for_import) {
                    self.vault_unlock_go_on(for_import);
                }
            }
            Err(e) => {
                self.vaults.state.passphrase.clear();
                self.vaults.state.unlock_error = Some(errors::error_text(e));
            }
        }
    }

    /// Costs what `vault_gather_codex32` and the Load sheet cost. The rest
    /// of an unlock once its copy is taken: at once, or from the
    /// older-copy dialog's **Open** (`docs/VAULT-HARDENING.md` §5.3).
    pub(crate) fn vault_unlock_go_on(&mut self, for_import: bool) {
        // Unlocked for a vault category from Tools: that category.
        // Otherwise the first kind it holds.
        if self.vaults.back_to != Some(Screen::VaultContents) {
            let held = |kinds: &[u8]| {
                self.vaults
                    .records(self.vaults.open[self.vaults.open.len() - 1].id)
                    .iter()
                    .any(|(_, r)| kinds.contains(&r.kind()))
            };
            // Its wallets first, each with its chart (§7.2).
            self.vaults.state.category = if held(CATEGORIES[1].1) {
                1
            } else {
                CATEGORIES
                    .iter()
                    .position(|(_, kinds)| held(kinds))
                    .unwrap_or(0)
            };
            self.vaults.state.category_chosen = false;
        } else {
            self.vaults.state.category_chosen = true;
        }
        self.vaults.state.item = [0; 7];
        // Keys and wallets set not to load at unlock start unchosen on the
        // Vaults tab's load list.
        let v = self.vaults.current;
        let skip = self
            .vaults
            .records(self.vaults.open[v].id)
            .iter()
            .filter(|(_, r)| matches!(r.kind(), kind::KEY | kind::WALLET) && !r.loads_at_unlock())
            .map(|(i, _)| i)
            .collect();
        self.vaults.state.open[v].skip = skip;
        self.amounts_from_vault(self.vaults.open[v].id);
        // The vault just made opens on its contents, empty, with what to do next.
        let made = self.vaults.state.just_made.take() == Some(self.vaults.open[v].name.clone());
        self.toast(&fill(
            EN.vaults_unlocked,
            &[&self.vaults.open[v].name.to_string()],
        ));
        self.vault_gather_codex32();
        if for_import {
            // Its Load sheet (F62); after it, the next vault the import
            // brought is offered, or, with none left, the landing (F60).
            self.vaults.state.back_to = None;
            let before = self
                .import
                .state()
                .map_or(self.session.wallets.len(), |i| i.wallets_before);
            let label = self.import.state().map(|i| i.label.clone());
            if self.import_unlocked(v) {
                let file = self.vaults.open[v].name.clone();
                self.unlock_landed(before, &file, label.as_deref());
            }
            return;
        }
        // Every vault the boot import brought unlocked elsewhere: the import is over (F60).
        self.import_settle();
        // Unlocked from the Spend tab, or for it: everything loads.
        let back = self.vaults.state.back_to.take();
        if self.screen == Screen::Family || back == Some(Screen::Family) {
            self.screen = Screen::Family;
            self.family_unlocked(v);
        } else if back.is_none() && !made {
            // The Vaults list's Unlock (`docs/history/NEW-WALLET.md` §11.1a,
            // F62): Files, where it landed before F62 when it loaded no
            // wallet, with the vault's Load sheet over it; a wallet loaded
            // lands on Wallets as the boot sheet's does. A vault opened from
            // inside a flow, or just made, keeps `back_to` or `made` and
            // takes the branch below instead, loading nothing here.
            self.screen = Screen::Files;
            let before = self.session.wallets.len();
            self.load_sheet_offer(v, false, before);
        } else {
            self.screen = back.unwrap_or(if made {
                Screen::VaultContents
            } else {
                Screen::Files
            });
        }
        // Landed on the codes screen with the clock not checked this
        // power-on: Set the time first (`docs/TOTP.md` §5.2), coming back
        // to the codes.
        if self.screen == Screen::Codes && !self.clock_checked() {
            self.open_set_time();
        }
        // Unlocked for the boot import: its sheet again, with the vault's
        // wallets and keys in it.
        if back == Some(Screen::Home) && self.import.state().is_some() {
            self.sheet = Some(crate::Sheet::Import);
        }
        // Unlocked for a secret: its sheet again, with the vault.
        if back.is_some() && self.secrets.out().is_some() {
            self.sheet = Some(crate::Sheet::SecretOut);
        }
    }

    /// After an unlock's Load sheet that loaded wallets
    /// (`docs/history/NEW-WALLET.md` §11.1, §11.1a, F62: the boot sheet's Unlock or
    /// the Vaults list's): Wallets, on
    /// the first loaded that can sign here, else the first loaded, and a
    /// toast naming what loaded from where. One that loaded none stays
    /// where it landed.
    pub(crate) fn unlock_landed(&mut self, before: usize, file: &str, stick: Option<&str>) {
        let loaded: Vec<usize> = (before..self.session.wallets.len()).collect();
        let Some(&first) = loaded.first() else {
            return;
        };
        if self.sheet.is_some() {
            return;
        }
        let signs = loaded.iter().copied().find(|&i| {
            let w = &self.session.wallets[i];
            self.session.slots(w).iter().any(|s| s.held_by.is_some())
        });
        self.wallets.set_wallet(signs.unwrap_or(first));
        self.screen = Screen::Wallets;
        self.wallets.set_card_offset(0.0);
        self.glance.set_open(false);
        // From the vault, or from the stick it came on when the stick's
        // own files brought some.
        let from = match stick {
            Some(s)
                if loaded
                    .iter()
                    .any(|&i| self.session.wallets[i].source != EN.vaults_vault) =>
            {
                s
            }
            _ => file,
        };
        let what = match loaded.len() {
            1 => self.session.wallets[first].name.clone(),
            n => fill(EN.vaults_wallets_n, &[&n.to_string()]),
        };
        self.toast(&fill(EN.vaults_loaded_note, &[&what.to_string(), from]));
    }

    /// Costs what `described` and `key_fingerprint` cost. The keys and
    /// wallets open vault `v` holds, each with whether the session has it
    /// already and whether it is chosen for the next load. Kept in the
    /// drawing memo while a frame is drawn (`docs/SPEED.md` §3.3). Reads
    /// its wallet records as it keeps them read ([`Vaults::described`]).
    ///
    /// Works out [`crate::drawing::View::vault_rows`] from the wallet records as the
    /// vault keeps them read ([`Vaults::described`]).
    pub fn vault_rows_now(&self, c: &Compute, v: usize) -> Vec<VaultRow> {
        self.vaults.rows(c, &self.session, v)
    }

    /// Costs a PBKDF2 and two elliptic-curve multiplications per key
    /// record, and a PBKDF2 and a key derivation per candidate, the first
    /// time. Where the seed of wallet slot `slot` is in the open vaults:
    /// the vault and its record. A record is that slot's seed when its key
    /// at the descriptor's path is the descriptor's key; the fingerprint
    /// only finds the candidate (F65). A slot with no descriptor key (a
    /// silent payments wallet's, a share) is found by fingerprint.
    pub fn vault_key_for(&self, c: &Compute, slot: &crate::wallet::Slot) -> Option<(usize, usize)> {
        let want = crate::wallet::fp_text(slot.fingerprint?);
        self.vaults.open.iter().enumerate().find_map(|(v, open)| {
            self.vaults
                .records(open.id)
                .of(kind::KEY)
                .find(|(_, r)| match &slot.key {
                    Some(k) => crate::vault::screen::record_is_policy_key(c, self, *r, k),
                    None => {
                        crate::vault::screen::key_fingerprint(c, self, *r).as_deref() == Some(&want)
                    }
                })
                .map(|(i, _)| (v, i))
        })
    }

    /// Costs a read of the wallet's keys. The slot [`SlotOf`] `at` names,
    /// with master fingerprint `fp`; `None` when that wallet or slot is
    /// gone or names another key.
    pub(crate) fn slot_of(
        &self,
        c: &Compute,
        fp: [u8; 4],
        at: SlotOf,
    ) -> Option<crate::wallet::Slot> {
        let fp = faraday_bip::keys::Fingerprint(fp);
        let slot = match at {
            SlotOf::Loaded(w, k) => {
                let wallet = self.session.wallets.get(w)?;
                self.session.slots(wallet).into_iter().nth(k)?
            }
            SlotOf::Vault(v, r, k) => {
                let d = self.vaults.described(c, self.vaults.open.get(v)?.id, r)?;
                let wallet = crate::wallet::Wallet::new(
                    String::new(),
                    d.policy.clone(),
                    String::new(),
                    crate::wallet::Origin::Named,
                );
                self.session.slots(&wallet).into_iter().nth(k)?
            }
            SlotOf::Bare => crate::wallet::Slot::bare(fp),
        };
        (slot.fingerprint == Some(fp)).then_some(slot)
    }

    /// Costs what `key_fingerprint`, `vault_rows_now` and `described_of`
    /// cost. The wallets open vault `v` holds together with some of their
    /// seeds, each with those seeds' records.
    pub fn vault_wallets_with_keys(&self, c: &Compute, v: usize) -> Vec<VaultWallet> {
        let Some(open) = self.vaults.open.get(v) else {
            return Vec::new();
        };
        let seeds: Vec<(usize, String)> = self
            .vaults
            .records(open.id)
            .of(kind::KEY)
            .filter_map(|(i, r)| crate::vault::screen::key_fingerprint(c, self, r).map(|f| (i, f)))
            .collect();
        let rows = self.vault_rows_now(c, v);
        let loaded = |i: usize| rows.iter().any(|r| r.record == i && r.loaded);
        let mut out = Vec::new();
        for (i, r, d) in self.vaults.described_of(c, open.id, kind::WALLET) {
            let policy = d.policy.clone();
            let name = wallet_name_in(r, field::WALLET_NAME, EN.vaults_wallet).into_owned();
            let wallet = crate::wallet::Wallet::new(
                name.clone(),
                policy,
                String::new(),
                crate::wallet::Origin::Named,
            );
            let fps: Vec<String> = wallet
                .policy()
                .keys()
                .iter()
                .filter_map(|k| k.fingerprint())
                .map(crate::wallet::fp_text)
                .collect();
            let keys: Vec<usize> = seeds
                .iter()
                .filter(|(_, f)| fps.contains(f))
                .map(|(k, _)| *k)
                .collect();
            if keys.is_empty() {
                continue;
            }
            out.push(VaultWallet {
                record: i,
                name,
                needed: crate::wallet::needed(&wallet),
                total: fps.len().max(1),
                done: loaded(i) && keys.iter().all(|&k| loaded(k)),
                chosen: !open.skip.contains(&i),
                keys,
            });
        }
        out
    }

    /// Loads the keys and wallets of an open vault into the session:
    /// every one, or the chosen ones.
    pub(crate) fn vault_load(&mut self, v: usize, all: bool) -> (usize, usize) {
        let Some(open) = self.vaults.open.get(v) else {
            return (0, 0);
        };
        let set: BTreeSet<usize> = (0..self.vaults.records(open.id).len())
            .filter(|i| all || !open.skip.contains(i))
            .collect();
        self.vault_load_set(v, &set)
    }

    /// Loads wallet record `r` of open vault `v`, with the seeds of it the
    /// vault keeps where keys may load. Returns the wallet's place in the
    /// session.
    pub(crate) fn vault_load_wallet(&mut self, v: usize, r: usize) -> Option<usize> {
        let c = self.compute();
        let open = self.vaults.open.get(v)?;
        self.vaults
            .records(open.id)
            .get(r)
            .filter(|x| x.kind() == kind::WALLET)?;
        let described = self.vaults.described(&c, open.id, r)?;
        let policy = described.policy.clone();
        let want = described.same.clone();
        let mut set = BTreeSet::from([r]);
        if self.may_load_keys() {
            let fps: Vec<String> = policy
                .keys()
                .iter()
                .filter_map(|k| k.fingerprint())
                .map(crate::wallet::fp_text)
                .collect();
            set.extend(
                self.vaults
                    .records(open.id)
                    .of(kind::KEY)
                    .filter_map(|(i, k)| {
                        crate::vault::screen::key_fingerprint(&c, self, k)
                            .filter(|f| fps.contains(f))
                            .map(|_| i)
                    }),
            );
        }
        let (keys, _) = self.vault_load_set(v, &set);
        if keys > 0 {
            self.toast(&fill(
                EN.vaults_loaded,
                &[
                    &keys.to_string(),
                    (if keys == 1 {
                        EN.vaults_seed_inline
                    } else {
                        EN.vaults_seeds
                    }),
                ],
            ));
        }
        self.session
            .wallets
            .iter()
            .position(|w| crate::wallet::same_wallet(w.policy()) == want)
    }

    /// Loads the records `set` names of open vault `v`: its keys and
    /// wallets among them.
    pub(crate) fn vault_load_set(&mut self, v: usize, set: &BTreeSet<usize>) -> (usize, usize) {
        let c = self.compute();
        let mut wallets = 0;
        let Some(open) = self.vaults.open.get(v) else {
            return (0, 0);
        };
        let mut descs: Vec<(faraday_bip::policy::WalletPolicy, String)> = Vec::new();
        let records = self.vaults.records(open.id);
        for (i, r) in records.of(kind::WALLET) {
            if !set.contains(&i) {
                continue;
            }
            // Read once when the records changed (`Vaults::described`),
            // not again here.
            if let Some(d) = self.vaults.described(&c, open.id, i) {
                descs.push((
                    d.policy.clone(),
                    wallet_name_in(r, field::WALLET_NAME, EN.vaults_wallet).into_owned(),
                ));
            }
        }
        let keys = self
            .vaults
            .store()
            .load_from_vault(&c, &mut self.session.keys, open.id, set)
            .len();
        let before = self.session.wallets.len();
        for (policy, name) in descs {
            let to = crate::wallet::Origin::Vault;
            let added = self
                .session
                .add_policy(&c, &name, policy, EN.vaults_vault, to);
            self.silent_refused(added);
        }
        wallets += self.session.wallets.len() - before;
        self.refresh_spend();
        (keys, wallets)
    }

    /// The selected entry as a KeePass database sealed under the prompt's
    /// passphrase, to the Outbox: sealed, so it goes as it is.
    fn vault_kdbx(&mut self) {
        let c = self.compute();
        let Some(at) = self.vault_selected_index() else {
            return;
        };
        let Some(id) = self.vaults.current_id() else {
            return;
        };
        let Some(title) = self
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
            return;
        };
        let Some(passphrase) = self
            .vaults
            .prompt
            .as_ref()
            .map(|p| Zeroizing::new(p.text.text.to_string()))
        else {
            return;
        };
        // The database's own randomness, every byte the format needs
        // taken fresh.
        let Some(draw) = self.fresh_bytes(b"kdbx", faraday_keys::vault::KDBX_SEED_LEN) else {
            if let Some(p) = self.vaults.state.prompt.as_mut() {
                p.error = Some(EN.vaults_no_randomness.to_string());
            }
            return;
        };
        let mut seed = Zeroizing::new([0u8; faraday_keys::vault::KDBX_SEED_LEN]);
        seed.copy_from_slice(&draw);
        let (out, name) = crate::bip85::kdbx_out(&self.outbox, &title);
        let words = faraday_keys::vault::KdbxWords {
            title: out,
            username: EN.vaults_username,
            url: EN.vaults_url,
        };
        match self
            .vaults
            .store()
            .kdbx_entry(&c, (id, at), &words, passphrase.as_bytes(), &seed)
        {
            Some(file) => {
                self.vaults.state.prompt = None;
                self.vaults.state.focus = None;
                self.put_outbox(&name, file);
                self.toast(&fill(
                    EN.vaults_waits_sealed,
                    &[&name.to_string(), self.medium.for_the()],
                ));
            }
            None => {
                if let Some(p) = self.vaults.state.prompt.as_mut() {
                    p.error = Some(EN.vaults_entry_too_large_kdbx.to_string());
                }
            }
        }
    }

    /// The position in the slot of the item selected in the kind shown.
    pub fn vault_selected_index(&self) -> Option<usize> {
        self.vaults.selected_place()
    }

    fn vault_delete(&mut self) {
        let Some(at) = self.vault_selected_index() else {
            return;
        };
        if let Some(id) = self.vaults.current_id() {
            self.vaults.edit(id, |s| s.remove(id, at));
            self.vaults.changed(id);
            self.toast(EN.vaults_deleted_toast);
        }
    }

    fn vault_load_selected(&mut self) {
        let c = self.compute();
        let Some(at) = self.vault_selected_index() else {
            return;
        };
        let Some(open) = self.vaults.open.get(self.vaults.current) else {
            return;
        };
        let id = open.id;
        let Some(r) = self.vaults.records(id).get(at) else {
            return;
        };
        match r.kind() {
            kind::KEY => {
                let Some(added) =
                    self.vaults
                        .store()
                        .load_key_record(&c, &mut self.session.keys, id, at)
                else {
                    return;
                };
                match added {
                    Ok(fp) => self.toast(&fill(
                        EN.vaults_key_is_loaded,
                        &[&crate::wallet::fp_text(fp).to_string()],
                    )),
                    Err(e) => self.toast(&e.text()),
                }
            }
            kind::WALLET => {
                let text = r.public_text(field::WALLET).unwrap_or("").to_string();
                let name = wallet_name_in(r, field::WALLET_NAME, EN.vaults_wallet).into_owned();
                match self.session.add_wallet(
                    &c,
                    &name,
                    &text,
                    EN.vaults_vault,
                    crate::wallet::Origin::Vault,
                ) {
                    Ok(i) => {
                        self.wallets.set_wallet(i);
                        self.screen = Screen::Wallets;
                    }
                    Err(e) => self.toast(&e.text()),
                }
            }
            _ => {}
        }
        self.refresh_spend();
    }

    /// The form that asks why the selected GPG key is revoked: its
    /// optional description, with the reason chosen by buttons. Its one
    /// field is number 3, which the Make form does not have.
    pub(crate) fn vault_form_for_revocation(&mut self) {
        self.vaults.state.signing = false;
        self.vaults.state.revoke_reason = 0;
        self.vaults.state.form = Some(Form {
            kind: kind::GPG,
            record: self.vault_selected_index(),
            fields: vec![(3, EN.vaults_description, TextBox::default(), false)],
        });
        self.vaults.state.saving = false;
        stick::focus(self, Focus::Field(0));
    }

    fn vault_form(&mut self, edit: bool) {
        let c = self.compute();
        let kinds = CATEGORIES[self.vaults.category].1;
        let at = if edit {
            self.vault_selected_index()
        } else {
            None
        };
        let id = self.vaults.current_id();
        let existing = at
            .zip(id)
            .and_then(|(i, id)| self.vaults.records(id).get(i));
        let k = existing.map(|r| r.kind()).unwrap_or(kinds[0]);
        let shape: &[(u8, &'static str, bool)] = match k {
            kind::ENTRY => &[
                (field::TITLE, EN.vaults_title, false),
                (field::USERNAME, EN.vaults_username, false),
                (field::PASSWORD, EN.vaults_password, true),
                (field::URL, EN.vaults_url, false),
                (field::NOTES, EN.vaults_notes, false),
                (field::TOTP, EN.vaults_totp_secret, true),
            ],
            kind::NOTE => &[(field::NOTE, EN.vaults_text, false)],
            _ => return,
        };
        let fields = shape
            .iter()
            .map(|(n, label, secret)| {
                let mut b = TextBox::default();
                if let (Some(r), Some(id), Some(i)) = (existing, id, at) {
                    match form_part(r.kind(), *n) {
                        // A secret goes into the form only to be edited.
                        Some(part) => {
                            let t = self.vaults.store().reveal(
                                &c,
                                id,
                                i,
                                *n,
                                faraday_keys::Reveal::VaultEntry(part),
                            );
                            if !t.is_empty() {
                                b.set(&t);
                            }
                        }
                        None => {
                            if let Some(t) = r.public_text(*n) {
                                b.set(t);
                            }
                        }
                    }
                }
                (*n, *label, b, *secret)
            })
            .collect();
        self.vaults.state.form = Some(Form {
            kind: k,
            record: at,
            fields,
        });
        self.vaults.state.saving = false;
        stick::focus(self, Focus::Field(0));
    }

    fn vault_form_save(&mut self) {
        // The backup flow's Make form is saved by Make the key.
        if self.screen == Screen::BackupFlow {
            return self.flow_act(crate::backup_flow::steps::FlowAction::Make);
        }
        let Some(form) = self.vaults.state.form.take() else {
            return;
        };
        self.vaults.state.focus = None;
        if form.kind == kind::GPG && self.time_asked() {
            self.vaults.state.form = Some(form);
            return;
        }
        if form.kind == kind::GPG && form.fields.first().is_some_and(|f| f.0 == 3) {
            let reason = faraday_pgp::Reason::from_menu(self.vaults.revoke_reason)
                .unwrap_or(faraday_pgp::Reason::Unspecified);
            let text = form.fields[0].2.text.to_string();
            self.gpg_make_revocation(reason, &text);
            return;
        }
        let Some(id) = self.vaults.current_id() else {
            return;
        };
        if form.kind == kind::SLOT_LABEL {
            let name = form.fields[0].2.text.trim().to_string();
            self.vaults.edit(id, |s| {
                s.remove_where(id, |r| r.kind() == kind::SLOT_LABEL);
                if !name.is_empty() {
                    let name: String = name.chars().take(64).collect();
                    let mut name = name;
                    while name.len() > 64 {
                        name.pop();
                    }
                    s.insert_at(
                        id,
                        0,
                        Record::new(kind::SLOT_LABEL).with(field::LABEL, name.as_bytes()),
                    );
                }
            });
            self.vaults.changed(id);
            return;
        }
        if form.fields[0].2.text.trim().is_empty() {
            let what = if form.kind == kind::NOTE {
                EN.vaults_note_empty
            } else {
                EN.vaults_an_entry_needs
            };
            self.vaults.state.form = Some(form);
            self.toast(what);
            return;
        }
        // The TOTP secret field, if any, goes through `normalise` with
        // the form's own Title and Username (`docs/TOTP.md` §6); checked
        // before anything moves `form` away.
        let title = form.fields[0].2.text.to_string();
        let account = form
            .fields
            .iter()
            .find(|(n, ..)| *n == field::USERNAME)
            .map(|(_, _, b, _)| b.text.to_string())
            .unwrap_or_default();
        let totp_text = form
            .fields
            .iter()
            .find(|(n, ..)| *n == field::TOTP)
            .map(|(_, _, b, _)| b.text.to_string());
        let totp_store: Result<Option<String>, ()> = match &totp_text {
            None => Ok(None),
            Some(t) if t.trim().is_empty() => Ok(None),
            Some(t) => faraday_keys::totp::normalise(t, &title, &account)
                .map(Some)
                .ok_or(()),
        };
        let Ok(totp_store) = totp_store else {
            self.vaults.state.form = Some(form);
            self.toast(EN.vaults_not_a_totp_key);
            return;
        };
        // The record takes each field as it was typed; one left empty is
        // taken out.
        let mut typed: Vec<(u8, &[u8])> = Vec::new();
        for (n, _, b, _) in &form.fields {
            if *n != field::TOTP {
                typed.push((*n, b.text.as_bytes()));
            }
        }
        if totp_text.is_some() {
            typed.push((
                field::TOTP,
                totp_store.as_deref().map_or(&b""[..], str::as_bytes),
            ));
        }
        // A new record must still fit in the slot.
        let was = form.record.filter(|&i| i < self.vaults.records(id).len());
        let placed = self
            .vaults
            .edit(id, |s| s.set_fields(id, form.record, form.kind, &typed));
        let Ok(place) = placed else {
            self.vaults.state.form = Some(form);
            self.toast(EN.vaults_slot_too_small);
            return;
        };
        if was.is_none() {
            self.vaults.state.select_place(place);
        }
        self.vaults.changed(id);
    }

    /// Inbox file k into the open vault, as the record its kind is.
    fn vault_add_file(&mut self, k: usize) {
        if self.vaults.open.is_empty() {
            self.toast(EN.vaults_unlock_a_vault);
            return;
        }
        let Some(it) = self.inbox.get(k) else {
            return;
        };
        let stem = crate::stem(&it.name);
        let text = Zeroizing::new(String::from_utf8_lossy(&it.bytes).into_owned());
        let (record, said, category) = match it.kind {
            FileKind::Text => {
                // A note's first line is its title.
                let note = Zeroizing::new(format!("{stem}\n{}", text.trim_end()));
                (
                    Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
                    fill(EN.vaults_is_in_the_vault_as_a_note, &[&it.name.to_string()]),
                    3,
                )
            }
            FileKind::Words => {
                let words = crate::forms::file_words(&text);
                let Ok(m) =
                    crate::forms::typed_mnemonic(faraday_bip::bip39::Language::English, &words)
                else {
                    return;
                };
                (
                    Record::new(kind::KEY)
                        .with(field::KEY, &records::words_payload(&m))
                        .with(field::KEY_LABEL, stem.as_bytes()),
                    fill(EN.vaults_file_added_as_key, &[&it.name.to_string()]),
                    0,
                )
            }
            FileKind::Wallet => (
                Record::new(kind::WALLET)
                    .with(field::WALLET, text.trim().as_bytes())
                    .with(field::WALLET_NAME, stem.as_bytes()),
                fill(EN.vaults_is_in_the_vault_as, &[&it.name.to_string()]),
                1,
            ),
            _ => return,
        };
        let before = self.vaults.open.get(self.vaults.current).map(|v| v.changes);
        self.vault_push(record, &said);
        if self.vaults.open.get(self.vaults.current).map(|v| v.changes) != before {
            self.vaults.state.category = category;
        }
    }

    fn vault_save_key(&mut self, k: usize) {
        self.vault_save_key_with(k, false);
    }

    fn vault_save_key_with(&mut self, k: usize, with_passphrase: bool) {
        if let Some((record, label)) = self.key_record(k, with_passphrase) {
            self.vault_push(
                record,
                &fill(EN.vaults_is_in_the_vault, &[&label.to_string()]),
            );
        }
    }

    /// Session key `k` as the vault's key record, ticked at unlock, and
    /// its label: what Save on Vaults writes, and the secret sheet for a
    /// seed. With its BIP-39 passphrase only when `with_passphrase`.
    pub(crate) fn key_record(
        &mut self,
        k: usize,
        with_passphrase: bool,
    ) -> Option<(faraday_keys::Held<Record>, String)> {
        let c = self.compute();
        let key = self.session.keys.info(k)?;
        if !key.has_words() && !self.session.keys.holds_seed_bytes(k) {
            self.toast(EN.vaults_this_key_has);
            return None;
        }
        // Made here and its backup not done: the vault says so, beside
        // the seed.
        // It loads at unlock (no flag says otherwise, F62).
        let pending = self
            .key_held_stored(&c, key.fingerprint)
            .then_some(records::BACKUP_PENDING);
        let label = key.label.clone();
        // A BIP-39 passphrase is stored only when the person asks
        // (`docs/history/PLAN.md` §6.3).
        let record = self.session.keys.key_record(
            &c,
            k,
            pending,
            with_passphrase,
            faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Words),
        )?;
        Some((record, label))
    }

    fn vault_save_wallet(&mut self, k: usize) {
        let Some(w) = self.session.wallets.get(k) else {
            return;
        };
        // A silent payments wallet is its record: it has no descriptor
        // that reads back as it.
        let text = match w.policy().silent() {
            Some(s) => s.to_text(),
            None => w.policy().to_descriptor_checksummed(),
        };
        let record = Record::new(kind::WALLET)
            .with(field::WALLET, text.as_bytes())
            .with(field::WALLET_NAME, w.name.as_bytes());
        let name = w.name.clone();
        self.vault_push(
            record,
            &fill(EN.vaults_is_in_the_vault, &[&name.to_string()]),
        );
    }

    pub(crate) fn vault_push(&mut self, record: impl IntoRecord, said: &str) {
        self.vault_push_in(self.vaults.current, record, said);
    }

    /// Adds `record` to open vault `v`, saying `said`. False when it does
    /// not fit in the vault's slot.
    pub(crate) fn vault_push_in(&mut self, v: usize, record: impl IntoRecord, said: &str) -> bool {
        let Some(id) = self.vaults.open.get(v).map(|o| o.id) else {
            return false;
        };
        if !self.vaults.store().fits(id, &[], &[record.size()]) {
            self.toast(EN.vaults_slot_too_small);
            return false;
        }
        self.vaults.edit(id, |s| s.insert(id, record));
        self.vaults.changed(id);
        self.toast(said);
        true
    }

    fn vault_create_check(&mut self) {
        let memory = self.vaults.create_memory_refusal();
        let medium = self.medium;
        let Some(c) = self.vaults.state.create.as_mut() else {
            return;
        };
        let error = if !self.stick.is_empty() {
            Some(fill(EN.vaults_remove_the_first, &[medium.noun()]))
        } else if let Some(n) = c
            .steps()
            .iter()
            .position(|&i| !c.done[i as usize] && i != vstep::PHRASES)
        {
            c.open = Some(c.steps()[n]);
            Some(fill(EN.vaults_finish_step, &[&(n + 1).to_string()]))
        } else if c.phrases.iter().any(|(a, _)| a.text.is_empty()) {
            c.open = Some(vstep::PHRASES);
            Some(EN.vaults_every_passphrase.to_string())
        } else if (0..c.phrases.len()).any(|i| !Vaults::phrase_ok(c, i)) {
            c.open = Some(vstep::PHRASES);
            Some(EN.vaults_each_passphrase.to_string())
        } else if (0..c.phrases.len())
            .any(|i| (i + 1..c.phrases.len()).any(|j| *c.phrases[i].0.text == *c.phrases[j].0.text))
        {
            c.open = Some(vstep::PHRASES);
            Some(EN.vaults_two_passphrases.to_string())
        } else {
            memory
        };
        c.error = error;
        if c.error.is_none() {
            c.done[vstep::PHRASES as usize] = true;
            self.vaults.state.working = Some(Work::Create);
            self.vaults.state.drawn = false;
        }
    }

    fn vault_create(&mut self) {
        let c = self.compute();
        // Everything random in a vault comes from fresh system entropy of
        // its own; a vault is never made before it has arrived.
        if self.fresh.pool_is_empty() {
            self.out.command(faraday_shell_api::Command::RequestEntropy);
            if let Some(c) = self.vaults.state.create.as_mut() {
                c.error = Some(EN.vaults_no_randomness.to_string());
            }
            return;
        }
        let Some((_, mem, passes)) = self.vaults.form_cost() else {
            return;
        };
        let Some(seed) = self.fresh(b"vault create") else {
            return;
        };
        let Some(form) = self.vaults.create.as_ref() else {
            return;
        };
        let cost = Cost {
            memory_kib: mem * 1024,
            passes,
            lanes: 1,
        };
        let first_phrase = Zeroizing::new(
            form.phrases
                .first()
                .map(|(a, _)| a.text.to_string())
                .unwrap_or_default(),
        );
        let phrases: Vec<&str> = form.phrases.iter().map(|(a, _)| &*a.text).collect();
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
        let contents: Vec<Contents> = phrases
            .iter()
            .map(|_| Contents {
                records: vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, label.as_bytes())],
            })
            .collect();
        let made = faraday_keys::vault::create(&c, form.slot, cost, &phrases, &contents, &seed);
        work::note_vault(VaultAct::Made, cost, VaultEnded::of(&made));
        match made {
            Ok(file) => {
                let name = self.free_vault_name(&vault_stem(&form.name.text));
                self.put_outbox(&name, file);
                // Made inside a backup flow: unlocked at once with the
                // passphrase just typed twice, never leaving the flow.
                if self.flow_vault_made(&name, &first_phrase) {
                    self.toast(EN.vaults_vault_created);
                    return;
                }
                self.vaults.state.just_made = Some(name.clone());
                self.vaults.state.create = None;
                self.vaults.state.focus = None;
                self.screen = Screen::Vaults;
                self.toast(EN.vaults_vault_created);
                // Unlock, the new vault picked (FLOWS decision 10): its
                // passphrase is typed once more, then the flow it was made
                // for goes on, or its contents open.
                if let Some(i) = self.vault_files().iter().position(|f| f.name == name) {
                    self.vault_act(VaultAction::Open(i));
                }
            }
            Err(e) => {
                if let Some(c) = self.vaults.state.create.as_mut() {
                    c.error = Some(errors::error_text(e));
                }
            }
        }
    }

    /// `stem.fvlt`, or the first `stem-n.fvlt` no box holds.
    pub(crate) fn free_vault_name(&self, stem: &str) -> String {
        let taken = |n: &str| {
            self.outbox
                .iter()
                .chain(self.inbox.iter())
                .any(|i| i.name.eq_ignore_ascii_case(n))
        };
        let first = format!("{stem}.fvlt");
        if !taken(&first) {
            return first;
        }
        (2..100)
            .map(|n| format!("{stem}-{n}.fvlt"))
            .find(|n| !taken(n))
            .unwrap_or_else(|| format!("{stem}-100.fvlt"))
    }

    /// Whether a lock would lose an open vault's changes: the session's
    /// seed, which every seal's nonce is drawn from, has not arrived.
    pub fn seal_waits(&self) -> bool {
        !self.seeded() && self.vaults.open.iter().any(|v| v.changes > 0)
    }

    /// Whether a lock would lose a changed vault's changes: the seed
    /// has not arrived, or a vault's records no longer fit its slot.
    pub fn lock_would_lose(&self) -> bool {
        self.seal_waits() || self.vaults.seal_blocked().is_some()
    }

    /// Seals every open vault with changes into the Outbox (`docs/history/PLAN.md`
    /// §5.3). A vault nobody changed is not rewritten: a slot's bytes
    /// change only when there is something to keep, which is all a second
    /// copy can compare (`docs/VAULT.md` §9). Before the session's seed
    /// arrived nothing is sealed: the lock that calls this asked first
    /// ([`Faraday::seal_waits`]).
    pub(crate) fn vault_seal_all(&mut self) {
        // What each open vault holds, remembered across the lock.
        self.vault_summaries_refresh();
        let mut sealed: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..self.vaults.open.len() {
            // Its plans take the counts of the vaults sealed before it
            // (`docs/VAULT-HARDENING.md` §5.6).
            let c = self.compute();
            self.vaults.seals_write(&c, i);
            sealed.extend(self.vault_sealed(i));
        }
        for (name, bytes) in sealed {
            self.put_outbox(&name, bytes);
        }
        self.vaults.state.close_all();
        self.vaults.state.load = None;
        self.vaults.state.pass_seeds = None;
    }

    /// Open vault `i`, with the signed-amount memory added, sealed: its
    /// name and bytes, or None when nobody changed it or the session's
    /// seed has not arrived.
    fn vault_sealed(&mut self, i: usize) -> Option<(String, Vec<u8>)> {
        // A held key's record says so before the vault goes
        // (`docs/history/NEW-WALLET.md` §14.3).
        self.held_sweep();
        // The signed-amount memory goes into every open vault that lacks
        // part of it, the oldest records leaving first when the slot is
        // full (`docs/VAULT.md` §7, type 10).
        if self.memory.seals()
            && let Some(id) = self.vaults.open.get(i).map(|o| o.id)
        {
            let have: Vec<[u8; 32]> = self
                .vaults
                .records(id)
                .of(kind::AMOUNTS)
                .filter_map(|(_, r)| crate::memory::from_record(r).map(|e| e.0))
                .collect();
            let new: Vec<Record> = self.memory.records_lacking(&have);
            self.vaults.state.add_amounts(i, new);
        }
        if self.vaults.open.get(i)?.changes == 0 {
            return None;
        }
        let draw = self.vault_seal_nonce()?;
        let mut nonce = [0u8; fv::NONCE_LEN];
        nonce.copy_from_slice(&draw[..fv::NONCE_LEN]);
        let now = self.clock_now();
        let id = self.vaults.open[i].id;
        let bytes = self.vaults.state.store.sealed(id, &nonce, now)?;
        let name = self.vaults.open[i].name.clone();
        let c = self.compute();
        self.vaults.seals_sealed(&c, i, &bytes);
        // The keys waiting for their copies that this sealing holds.
        self.flow_sealed(i, &name, &bytes);
        Some((name, bytes))
    }

    /// Locks open vault `i` alone: sealed into the Outbox when changed,
    /// what it holds remembered, and closed. The rest stay open. A
    /// changed vault stays open, its changes kept, while the session's
    /// seed has not arrived: then this returns false.
    pub(crate) fn vault_lock_one(&mut self, i: usize) -> bool {
        if i >= self.vaults.open.len() {
            return true;
        }
        if !self.seeded() && self.vaults.open[i].changes > 0 {
            self.out.command(faraday_shell_api::Command::RequestEntropy);
            self.toast(EN.seed_missing);
            return false;
        }
        if let Some(why) = self.vaults.cannot_seal(i) {
            self.toast(&why);
            return false;
        }
        self.vault_summaries_refresh();
        let c = self.compute();
        self.vaults.seals_write(&c, i);
        if let Some((name, bytes)) = self.vault_sealed(i) {
            self.put_outbox(&name, bytes);
        }
        self.vaults.state.close(i);
        self.vaults.state.current = 0;
        self.vaults.state.item_open = false;
        self.vaults.state.form = None;
        true
    }

    /// Typing on a vault screen. Returns whether the key was taken.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn vault_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        // The boot import's sheet has a passphrase field of its own.
        let import = self.sheet == Some(crate::Sheet::Import) && self.screen == Screen::Home;
        // So has the sheet of seeds kept with no passphrase (F62).
        let pass_seeds = self.sheet == Some(crate::Sheet::PassphraseSeeds);
        if !import
            && !pass_seeds
            && !matches!(
                self.screen,
                Screen::Unlock
                    | Screen::CreateVault
                    | Screen::VaultContents
                    | Screen::Family
                    | Screen::Codes
                    | Screen::BackupFlow
            )
        {
            return false;
        }
        let Some(focus) = self.vaults.focus else {
            return false;
        };
        // Nothing secret is typed while a stick is attached: the key is
        // taken and dropped.
        if stick::refuses_focus(self, focus) {
            return true;
        }
        if key == K::Escape {
            self.vaults.state.focus = None;
            // The backup flow's Make form is not left by Escape.
            if self.vaults.form.is_some() && self.screen != Screen::BackupFlow {
                self.vaults.state.form = None;
            }
            return true;
        }
        if matches!(focus, Focus::Dice | Focus::Rolls) {
            return dice::key(self, focus, key);
        }
        if focus == Focus::Prompt {
            if let Some(p) = self.vaults.state.prompt.as_mut() {
                match key {
                    K::Char(c) if !c.is_control() => p.text.text.push(c),
                    K::Backspace => {
                        p.text.text.pop();
                    }
                    K::Enter => self.vault_act(VaultAction::PromptGo),
                    _ => {}
                }
                if let Some(p) = self.vaults.state.prompt.as_mut() {
                    p.error = None;
                }
            }
            return true;
        }
        if key == K::Enter || key == K::Tab {
            match focus {
                Focus::Dice | Focus::Rolls | Focus::Prompt | Focus::Search(_) => {}
                Focus::Passphrase => {
                    if key == K::Enter && import {
                        self.import_act(crate::boot_import::ImportAction::Submit);
                    } else if key == K::Enter {
                        self.vault_act(VaultAction::Unlock);
                    }
                }
                Focus::Name => {
                    self.vaults.state.focus =
                        self.may_load_keys().then_some(Focus::Phrase(0, false));
                }
                Focus::Phrase(i, second) => {
                    let n = self.vaults.create.as_ref().map_or(0, |c| c.phrases.len());
                    self.vaults.state.focus = if !second {
                        Some(Focus::Phrase(i, true))
                    } else if i + 1 < n {
                        Some(Focus::Phrase(i + 1, false))
                    } else if key == K::Enter {
                        self.vault_act(VaultAction::CGo);
                        None
                    } else {
                        Some(Focus::Phrase(0, false))
                    };
                }
                Focus::Field(k) => {
                    let n = self.vaults.form.as_ref().map_or(0, |f| f.fields.len());
                    let lines = self
                        .vaults
                        .form
                        .as_ref()
                        .is_some_and(|f| multiline(f.kind, f.fields[k].0));
                    if key == K::Enter && lines {
                        if let Some(b) = self.vaults.text_box(focus) {
                            b.text.push('\n');
                        }
                    } else if key == K::Enter && (k + 1 >= n) {
                        self.vault_act(VaultAction::FormSave);
                    } else {
                        self.vaults.state.focus = Some(Focus::Field((k + 1) % n.max(1)));
                    }
                }
            }
            return true;
        }
        let Some(b) = self.vaults.text_box(focus) else {
            return false;
        };
        match key {
            K::Char(c) if !c.is_control() => {
                b.text.push(c);
            }
            K::Backspace => {
                b.text.pop();
            }
            _ => return false,
        }
        if let Some(c) = self.vaults.state.create.as_mut() {
            c.error = None;
        }
        self.vaults.state.unlock_error = None;
        true
    }
}

impl Faraday {
    /// Set or Keep this time has been pressed this power-on
    /// (`docs/TOTP.md` §1.3); the desktop app counts as checked whenever
    /// its clock is known.
    pub fn clock_checked(&self) -> bool {
        self.vaults.clock_checked || (self.online && self.clock_now().is_some())
    }

    /// Whether the screen shows the clock or a TOTP code live: Settings'
    /// and Set the time's Now, the codes, an open vault's entries, whose
    /// detail has the Code row, and Sign with {key}'s time.
    pub(crate) fn shows_clock(&self) -> bool {
        // Sign with {key}'s signature time (F63).
        if self.sheet == Some(crate::Sheet::GpgSign) {
            return true;
        }
        match self.screen {
            Screen::Settings | Screen::SetTime | Screen::Codes => true,
            Screen::VaultContents => self.vaults.category == 2,
            _ => false,
        }
    }

    /// Costs a hash of a whole file (through `hash`). Where vault file `f`
    /// stands against the last write (`docs/history/SIMPLIFY.md` §3.5). A vault
    /// copied in from a stick, unchanged, that no receipt names is current
    /// on the stick it came from when that is known, else unchanged.
    pub fn currency(&self, c: &Compute, f: &VaultFile) -> Option<Currency> {
        let changed = f
            .open
            .and_then(|o| self.vaults.open.get(o))
            .is_some_and(|v| v.changes > 0);
        let named = self.visit.receipt().as_ref().filter(|r| r.wrote(&f.name));
        match named {
            Some(r) => {
                let boxes = if f.in_outbox {
                    &self.outbox
                } else {
                    &self.inbox
                };
                let same = boxes
                    .iter()
                    .find(|i| i.name == f.name)
                    .is_some_and(|i| r.holds(&i.name, &i.hash(c)));
                Some(if same && !changed {
                    Currency::Current(r.label.clone())
                } else {
                    Currency::Changed
                })
            }
            None if f.in_outbox => Some(Currency::NeverWritten),
            None if changed => Some(Currency::Changed),
            // Read from a stick and not changed: on the stick it came
            // from, as it is, when that stick is known.
            // A vault file's hash was taken when it came in
            // ([`crate::Item::sha`]): drawing asks this on every frame.
            None if !self.visit.vault_from().iter().any(|(n, ..)| *n == f.name) => {
                Some(Currency::Unchanged)
            }
            None => {
                let hash = self
                    .inbox
                    .iter()
                    .find(|i| i.name == f.name)
                    .map(|i| i.hash(c));
                Some(
                    self.visit
                        .vault_from_find(|(n, h, _)| *n == f.name && Some(*h) == hash)
                        .map_or(Currency::Unchanged, |(.., label)| {
                            Currency::Current(label.clone())
                        }),
                )
            }
        }
    }

    /// Costs a hash of a whole file (through `currency`). The first vault
    /// changed since it was written, by the name it goes by: Home's lead
    /// rule 6 and the session strip's Write out.
    pub fn vault_changed(&self, c: &Compute) -> Option<String> {
        self.vault_files().iter().find_map(|f| {
            (self.currency(c, f) == Some(Currency::Changed)).then(|| {
                f.open
                    .and_then(|o| self.vaults.open.get(o))
                    .map(|v| v.label(self.vaults.store()))
                    .or_else(|| self.vault_summary(f).map(|s| s.name.clone()))
                    .unwrap_or_else(|| f.name.clone())
            })
        })
    }
}

/// Glue of the TOTP codes screen (`crate::codes`).
impl Faraday {
    /// Opens the codes screen (§5.2): no vault open leads to Create or
    /// Unlock first (handled by the tile and by `vault_way`); the clock
    /// not checked this power-on opens Set the time first. Both checks
    /// happen on the way in only. It opens at its top.
    pub(crate) fn open_codes(&mut self) {
        self.screen = Screen::Codes;
        self.list_offset = 0.0;
        if !self.clock_checked() {
            self.open_set_time();
        }
    }

    /// A row pressed: opens that entry in its vault (Vault contents,
    /// Entries, the record selected).
    pub(crate) fn open_code_row(&mut self, vault: usize, item: usize) {
        self.vaults.state.current = vault;
        self.vault_act(VaultAction::Category(2));
        let k = self.vaults.state.place_shown(item);
        self.vault_act(VaultAction::Item(k));
        self.screen = Screen::VaultContents;
    }

    /// Add an entry from the codes screen: the entry form, in the
    /// current vault, the codes screen still showing behind it.
    pub(crate) fn codes_add_entry(&mut self) {
        self.vaults.state.category = 2;
        self.vault_act(VaultAction::Add);
    }

    /// Handles the Codes screen's actions (`OpenCodes`, `CodesOpen`, `CodesAdd`).
    pub(crate) fn codes_act(&mut self, action: Action) {
        match action {
            Action::OpenCodes => self.open_codes(),
            Action::CodesOpen(v, item) => self.open_code_row(v, item),
            Action::CodesAdd => self.codes_add_entry(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

/// Glue of the vault files this session wrote.
impl Faraday {
    /// Costs a hash of a whole file (a SHA-256 pass). A vault in Files
    /// that this session unlocked or wrote: its salt is an open vault's, or
    /// its bytes are what the last write of this session put on a stick. A
    /// copy of any other vault never goes over a vault of the same size on
    /// a stick.
    pub(crate) fn vault_is_this_sessions(&self, c: &Compute, bytes: &[u8]) -> bool {
        let Ok(h) = faraday_vault::read_header(bytes) else {
            return false;
        };
        if self.vaults.open.iter().any(|v| v.header().salt == h.salt) {
            return true;
        }
        // Only a write of this session's own visit counts: the flow's
        // sealings do not, so that a later ordinary visit never writes the
        // flow's bytes over a newer copy on another stick.
        let hash = faraday_crypto::sha256_whole(c, bytes);
        self.visit
            .receipt()
            .as_ref()
            .is_some_and(|r| !r.by_flow && r.holds_hash(&hash))
    }
}

/// Glue of the vault's GPG keys (`crate::gpg`).
impl Faraday {
    fn clock(&self) -> Result<u32, String> {
        self.clock_now()
            .map(|t| t as u32)
            .ok_or_else(|| EN.gpg_this_computer_s.to_string())
    }

    /// The GPG key selected in the open vault: its vault, its record's
    /// place and what it says of itself in public. Costs two Ed25519
    /// public keys.
    fn gpg_selected(&self) -> Option<(VaultId, usize, Facts)> {
        let at = self.vault_selected_index()?;
        let id = self.vaults.open.get(self.vaults.current)?.id;
        let r = self.vaults.records(id).get(at)?;
        (r.kind() == kind::GPG).then(|| gpg_facts(r).map(|k| (id, at, k)))?
    }

    /// Record `at` of open vault `id`.
    fn gpg_record(&self, id: VaultId, at: usize) -> Option<RecordView<'_>> {
        self.vaults.records(id).get(at)
    }

    /// Puts a file in the Outbox unless a file of that name is there that
    /// `replaces` does not allow over: GnuPG's batch mode does not
    /// overwrite ("File exists") either.
    fn put_unless_taken(
        &mut self,
        name: &str,
        bytes: Vec<u8>,
        replaces: impl Fn(&Item) -> bool,
    ) -> bool {
        if self.outbox.iter().any(|i| i.name == name && !replaces(i)) {
            return false;
        }
        self.put_outbox(name, bytes);
        true
    }

    /// Makes a GPG key for the backup flow: an Ed25519 certification key
    /// and signing subkey from fresh entropy, one user ID, the chosen
    /// expiry. The record every vault of the plan takes and the public
    /// certificate For the stick; the revocation certificate, as GnuPG 2.1
    /// and later do, is kept with the key.
    pub(crate) fn gpg_build(&mut self, name: &str, email: &str) -> Result<Built, String> {
        let now = self.clock()?;
        // `gpg --quick-generate-key` takes one string, trims it, and
        // refuses only an empty one ("Invalid user ID"); it checks no
        // address and no length.
        let uid = match (name.trim(), email.trim()) {
            ("", "") => return Err(EN.gpg_a_key_needs_a_name.to_string()),
            (n, "") => n.to_string(),
            ("", e) => format!("<{e}>"),
            (n, e) => format!("{n} <{e}>"),
        };
        let (Some(primary), Some(subkey)) = (self.fresh(b"gpg primary"), self.fresh(b"gpg subkey"))
        else {
            return Err(EN.gpg_no_randomness.to_string());
        };
        let primary = Zeroizing::new(primary);
        let subkey = Zeroizing::new(subkey);
        let made = faraday_keys::gpg::create(
            &primary,
            &subkey,
            now,
            uid.clone(),
            self.vaults.gpg_years * gpg::YEAR,
        );
        let stem = gpg::file_stem(&made.facts);
        Ok(Built {
            record: made.key,
            subject: made.facts.fingerprint.to_vec(),
            name: if name.trim().is_empty() {
                uid
            } else {
                name.trim().to_string()
            },
            outbox: vec![(
                format!("{stem}.asc"),
                armor(Armor::PublicKey, &made.certificate).into_bytes(),
            )],
        })
    }

    /// The public key to the Outbox, unless a file of that name is there
    /// that is not a public key.
    pub(crate) fn gpg_put_public(&mut self, name: &str, bytes: Vec<u8>) {
        self.put_unless_taken(name, bytes, |i| {
            faraday_pgp::read_public_armor(&i.bytes) == Some(Armor::PublicKey)
        });
    }

    /// Makes the revocation certificate a record lacks, with the clock as
    /// it is: a key made before revocations were kept gets its one when
    /// its page is first used. Nothing is made while the clock is not
    /// known or not checked.
    pub(crate) fn gpg_keep_revocation(&mut self) {
        let Some((id, at, _)) = self.gpg_selected() else {
            return;
        };
        if self.gpg_record(id, at).is_none_or(|r| r.has(f::REVOCATION)) || !self.clock_checked() {
            return;
        }
        let Ok(now) = self.clock() else {
            return;
        };
        if self.vaults.edit(id, |s| s.gpg_keep_revocation(id, at, now)) {
            self.vaults.changed(id);
        }
    }

    /// The selected key's public certificate, armoured, with the key.
    fn gpg_certificate(&self) -> Option<(Facts, String)> {
        let (id, at, key) = self.gpg_selected()?;
        let r = self.gpg_record(id, at)?;
        let cert = match r.public(f::CERTIFICATE) {
            Some(c) => c.to_vec(),
            None => self.vaults.store().gpg_certificate(
                id,
                at,
                gpg::certified_at(r).unwrap_or(key.created),
                None,
            )?,
        };
        Some((key, armor(Armor::PublicKey, &cert)))
    }

    /// The selected key's public certificate to the Outbox.
    pub(crate) fn gpg_export(&mut self) {
        let Some((key, text)) = self.gpg_certificate() else {
            return;
        };
        let name = format!("{}.asc", gpg::file_stem(&key));
        if self.put_unless_taken(&name, text.into_bytes(), |i| {
            faraday_pgp::read_public_armor(&i.bytes) == Some(Armor::PublicKey)
        }) {
            self.toast_out(&name);
        } else {
            self.toast(&fill(EN.gpg_exists, &[&name.to_string()]));
        }
    }

    /// Costs what `gpg_files` and `text` cost and a file's hash: the QR view of the selected
    /// key's certificate or its signature over an Inbox file, with its picture's name and label.
    pub(crate) fn gpg_code(&self, c: &Compute, code: crate::Code) -> Result<crate::QrView, String> {
        use crate::Code;
        let (id, at, key) = self.gpg_selected().ok_or(EN.gpg_no_gpg_key_is_selected)?;
        let stem = gpg::file_stem(&key);
        let fp = fill(
            EN.gpg_fingerprint,
            &[&faraday_pgp::fingerprint_text(&key.fingerprint).to_string()],
        );
        let who = key.user_ids.first().cloned().unwrap_or_default();
        let (title, text, name, lines) = match code {
            Code::GpgKey => {
                let (_, text) = self
                    .gpg_certificate()
                    .ok_or(EN.gpg_no_gpg_key_is_selected)?;
                (
                    EN.gpg_gpg_public_key.to_string(),
                    text,
                    format!("{stem}.png"),
                    vec![who, fp, EN.gpg_public_checks_signatures.to_string()],
                )
            }
            Code::GpgSignature(prefix) => {
                // A key that waits for its backup signs nothing, a code of a
                // signature included (`docs/BACKUP-FLOW.md` §8).
                if self.selected_waits().is_some() {
                    return Err(EN.backup_flow_first.to_string());
                }
                let now = self.clock()?;
                gpg::usable(&key, self.gpg_record(id, at).and_then(gpg::revoked_at), now)?;
                let item = self
                    .gpg_files(c)
                    .into_iter()
                    .find(|f| f.hash[..8] == prefix)
                    .and_then(|f| self.inbox.get(f.index))
                    .filter(|i| crate::sha256_of(c, &i.bytes)[..8] == prefix) // hashed again
                    .ok_or(EN.gpg_file_gone)?;
                (
                    fill(EN.gpg_gpg_signature, &[&item.name.to_string()]),
                    self.vaults
                        .store()
                        .gpg_sign_armored(id, at, &item.bytes, now)
                        .ok_or(EN.gpg_no_gpg_key_is_selected)?,
                    format!("{}.asc.png", item.name),
                    vec![
                        fill(EN.gpg_signs, &[&item.name.to_string()]),
                        fill(EN.gpg_signer_line, &[&who.to_string()]),
                        fp,
                    ],
                )
            }
            _ => return Err(EN.gpg_not_a_gpg_file.to_string()),
        };
        Ok(crate::QrView::text(c, &title, &text)?.public(&name, lines))
    }

    /// Offers a revocation file through the secret sheet: whoever has it
    /// can revoke the key, so it is never a public file, a code or a
    /// picture.
    fn gpg_offer_revocation(&mut self, key: &Facts, file: &str) {
        let mut out = crate::secrets::SecretOut::new(
            gpg::rev_name(key),
            Zeroizing::new(file.as_bytes().to_vec()),
            EN.gpg_gpg_revocation,
            EN.gpg_revocation_warning,
        );
        // It is in the vault already.
        out.keep = crate::secrets::Keep::None;
        self.offer_secret(out);
    }

    /// The revocation certificate kept with the selected key, through the
    /// secret sheet as `{name}-{FPR}.rev`.
    pub(crate) fn gpg_export_revocation(&mut self) {
        self.gpg_keep_revocation();
        let Some((id, at, key)) = self.gpg_selected() else {
            return;
        };
        let Some(file) = self.vaults.store().gpg_revocation_file(
            id,
            at,
            faraday_keys::Reveal::OutboxFile(faraday_keys::Part::Revocation),
        ) else {
            return self.toast(EN.gpg_no_revocation_kept);
        };
        self.gpg_offer_revocation(&key, &file);
    }

    /// A new revocation certificate for the selected key, with the reason
    /// chosen (`gpg --generate-revocation`), through the secret sheet.
    /// The kept one is not replaced.
    pub(crate) fn gpg_make_revocation(&mut self, reason: Reason, text: &str) {
        let Some((id, at, key)) = self.gpg_selected() else {
            return;
        };
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        let Some(file) = self.vaults.store().gpg_new_revocation_file(
            id,
            at,
            now,
            reason,
            text.trim(),
            faraday_keys::Reveal::OutboxFile(faraday_keys::Part::Revocation),
        ) else {
            return;
        };
        self.gpg_offer_revocation(&key, &file);
    }

    /// The key is marked revoked, as importing its own revocation
    /// certificate marks it in GnuPG: it signs nothing and renews
    /// nothing after.
    pub(crate) fn gpg_mark_revoked(&mut self) {
        let Some((id, at, _)) = self.gpg_selected() else {
            return;
        };
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        if self.gpg_record(id, at).is_some_and(|r| !r.has(f::REVOKED)) {
            self.vaults
                .edit(id, |s| s.set_public(id, at, f::REVOKED, &now.to_le_bytes()));
            self.vaults.changed(id);
        }
        self.toast(EN.gpg_marked_revoked);
    }

    /// New self-signatures with the term chosen on the key's page,
    /// counted from now as `gpg --quick-set-expire` counts it; the
    /// record's certificate and the Outbox's change together.
    pub(crate) fn gpg_renew(&mut self) {
        if self.selected_waits().is_some() {
            return;
        }
        let Some((id, at, key)) = self.gpg_selected() else {
            return;
        };
        let Some(years) = self.vaults.renew_years else {
            return self.toast(EN.gpg_choose_the_term);
        };
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        let Some(r) = self.gpg_record(id, at) else {
            return;
        };
        let (revoked, certified) = (gpg::revoked_at(r), gpg::certified_at(r));
        if revoked.is_some() {
            return self.toast(&fill(
                EN.gpg_revoked,
                &[&gpg::date(u64::from(revoked.unwrap_or(0)))],
            ));
        }
        // GnuPG signs one second after the last signature when the clock
        // is within 4 seconds of it, and refuses a clock earlier than
        // that ("Time conflict"): an older signature would be ignored.
        let last = certified.unwrap_or(key.created);
        if last > now && last - now >= gpg::SKEW {
            return self.toast(EN.gpg_time_conflict);
        }
        let at_time = now.max(last.saturating_add(1));
        let expiry = match years {
            0 => 0,
            y => now.saturating_sub(key.created) + y * gpg::YEAR,
        };
        let Some(cert) = self
            .vaults
            .store()
            .gpg_certificate(id, at, at_time, Some(expiry))
        else {
            return;
        };
        let name = format!("{}.asc", gpg::file_stem(&key));
        self.vaults.edit(id, |s| {
            s.set_public(id, at, f::EXPIRY, &expiry.to_le_bytes());
            s.set_public(id, at, f::CERTIFICATE, &cert);
        });
        self.vaults.changed(id);
        self.vaults.state.renew_years = None;
        if self.put_unless_taken(&name, armor(Armor::PublicKey, &cert).into_bytes(), |i| {
            faraday_pgp::read_public_armor(&i.bytes) == Some(Armor::PublicKey)
        }) {
            self.toast(&fill(
                EN.gpg_renewed_waits,
                &[&name.to_string(), self.medium.for_the()],
            ));
        } else {
            self.toast(&fill(EN.gpg_exists, &[&name.to_string()]));
        }
    }

    /// Costs a hash of a whole file (through `hash`). The Inbox files a GPG
    /// key can sign, each with its hash and, when small, its text: a `.asc`
    /// file is a signature already, not something to sign again.
    pub(crate) fn gpg_files(&self, c: &Compute) -> Vec<gpg::GpgFile> {
        let mut seen = self.vaults.sign_seen.borrow_mut();
        let mut out = Vec::new();
        let mut keep = Vec::new();
        for (index, it) in self.inbox.iter().enumerate() {
            if it.name.ends_with(".asc") {
                continue;
            }
            let probe = gpg::probe_of(&it.bytes);
            let (hash, text) = match seen.iter().find(|s| s.name == it.name && s.probe == probe) {
                Some(s) => (s.hash, s.text.clone()),
                // Kept on the state (`sign_seen`) once worked out.
                None => (it.hash(c), gpg::small_text(&it.bytes)),
            };
            keep.push(gpg::Seen {
                name: it.name.clone(),
                probe,
                hash,
                text: text.clone(),
            });
            out.push(gpg::GpgFile {
                index,
                name: it.name.clone(),
                size: it.bytes.len(),
                hash,
                text,
            });
        }
        *seen = keep;
        out
    }

    /// Costs a hash of a whole file (through `gpg_files`). The code of the
    /// selected key's signature over the Inbox file called `name`: it names
    /// the file by its hash, as the signing page's buttons do.
    pub fn signature_code(&self, c: &Compute, name: &str) -> Option<crate::Code> {
        let f = self.gpg_files(c).into_iter().find(|f| f.name == name)?;
        Some(crate::Code::GpgSignature(f.hash[..8].try_into().ok()?))
    }

    /// Whether a listed file is ticked: by its name and hash, so a file a
    /// stick visit replaced is not.
    pub(crate) fn gpg_ticked(&self, f: &gpg::GpgFile) -> bool {
        self.vaults
            .sign_ticked
            .iter()
            .any(|(n, h)| *n == f.name && *h == f.hash)
    }

    /// The signing page's ticks at their default: the only signable file
    /// ticked, or none of several.
    pub(crate) fn gpg_default_ticks(&mut self) {
        let c = self.compute();
        let files = self.gpg_files(&c);
        self.vaults.state.sign_ticked = match files.as_slice() {
            [one] => vec![(one.name.clone(), one.hash)],
            _ => Vec::new(),
        };
    }

    /// Ticks or unticks the listed file at `k`.
    pub(crate) fn gpg_tick(&mut self, k: usize) {
        let c = self.compute();
        let Some(f) = self.gpg_files(&c).into_iter().nth(k) else {
            return;
        };
        let tick = (f.name, f.hash);
        match self.vaults.sign_ticked.iter().position(|t| *t == tick) {
            Some(at) => {
                self.vaults.state.sign_ticked.remove(at);
            }
            None => self.vaults.state.sign_ticked.push(tick),
        }
    }

    /// Ticks every listed file, or none when every one is ticked.
    pub(crate) fn gpg_tick_all(&mut self) {
        let c = self.compute();
        let files = self.gpg_files(&c);
        if files.iter().all(|f| self.gpg_ticked(f)) {
            self.vaults.state.sign_ticked.clear();
        } else {
            self.vaults.state.sign_ticked = files.into_iter().map(|f| (f.name, f.hash)).collect();
        }
    }

    /// Clears the signing page, its ticks and the other GPG pages when the
    /// vault, the category or the key selected changes.
    pub(crate) fn gpg_page_reset(&mut self) {
        if self.sheet == Some(crate::Sheet::GpgSign) {
            self.sheet = None;
        }
        self.vaults.state.signing = false;
        self.vaults.state.sign_ticked.clear();
        self.vaults.state.sign_key = None;
        self.vaults.state.renew_years = None;
        self.vaults.state.revoke_reason = 0;
    }

    /// The selected key's fingerprint.
    pub(crate) fn gpg_fingerprint(&self) -> Option<[u8; 20]> {
        self.gpg_selected().map(|(_, _, k)| k.fingerprint)
    }

    /// The selected key's first user ID, which Sign with {key} names it
    /// by.
    pub(crate) fn gpg_user_id(&self) -> Option<String> {
        let at = self.vault_selected_index()?;
        let r = self
            .vaults
            .records(self.vaults.open.get(self.vaults.current)?.id)
            .get(at)?;
        let id = r.public_all(f::USER_ID).next()?;
        Some(String::from_utf8_lossy(id).into_owned())
    }

    /// Costs a hash of a whole file (through `gpg_files`). The listed files
    /// ticked to sign, as Sign's sheet lists them. Hashes a file the page
    /// has not hashed yet.
    pub(crate) fn gpg_chosen(&self, c: &Compute) -> Vec<gpg::GpgFile> {
        self.gpg_files(c)
            .into_iter()
            .filter(|f| self.gpg_ticked(f))
            .collect()
    }

    /// Sign a file's **Sign**: opens **Sign with {key}** over the page
    /// when a listed file is ticked. Signs nothing and asks nothing: the
    /// sheet shows the time the signatures will carry (F63).
    pub(crate) fn gpg_sign_sheet_open(&mut self) {
        let c = self.compute();
        if !self.vaults.signing || self.vaults.sign_key != self.gpg_fingerprint() {
            return;
        }
        if self.gpg_chosen(&c).is_empty() {
            return;
        }
        self.sheet = Some(crate::Sheet::GpgSign);
    }

    /// The hold on **Sign with {key}**: signs the ticked files at the
    /// time the sheet shows, then closes it. With the clock not known
    /// the hold is disabled and signs nothing (F63). Nothing else signs:
    /// pressed with no sheet open, it does nothing.
    pub(crate) fn gpg_sign_sheet_hold(&mut self) {
        if self.sheet != Some(crate::Sheet::GpgSign) || self.clock_now().is_none() {
            return;
        }
        self.sheet = None;
        self.gpg_sign_ticked();
    }

    /// Signs every ticked file in one go, after the hold: a detached
    /// signature each, armoured, `{file}.asc` in the Outbox as `gpg
    /// --armor --detach-sign` names it. A file whose bytes are no longer
    /// those ticked is not signed. A signature is not written over a file
    /// of that name. One toast says how many were written.
    pub(crate) fn gpg_sign_ticked(&mut self) {
        let c = self.compute();
        if self.selected_waits().is_some() {
            return;
        }
        let Some((id, at, key)) = self.gpg_selected() else {
            return;
        };
        // The page was opened for another key: nothing is signed.
        if self.vaults.sign_key != Some(key.fingerprint) {
            self.gpg_page_reset();
            return;
        }
        let now = match self.clock() {
            Ok(t) => t,
            Err(e) => return self.toast(&e),
        };
        if let Err(e) = gpg::usable(&key, self.gpg_record(id, at).and_then(gpg::revoked_at), now) {
            return self.toast(&e);
        }
        let ticked = std::mem::take(&mut self.vaults.state.sign_ticked);
        let (mut written, mut changed) = (0usize, 0usize);
        let mut failure: Option<String> = None;
        for (name, hash) in ticked {
            let found = self
                .inbox
                .iter()
                .position(|i| i.name == name && crate::sha256_of(&c, &i.bytes) == hash);
            let Some(k) = found else {
                changed += 1;
                failure.get_or_insert_with(|| fill(EN.gpg_changed, &[&name.to_string()]));
                continue;
            };
            let out = format!("{name}.asc");
            if self.outbox.iter().any(|i| i.name == out) {
                failure.get_or_insert_with(|| fill(EN.gpg_exists, &[&out.to_string()]));
                continue;
            }
            let Some(sig) = self
                .vaults
                .store()
                .gpg_sign_armored(id, at, &self.inbox[k].bytes, now)
            else {
                continue;
            };
            self.put_outbox(&out, sig.into_bytes());
            written += 1;
        }
        let _ = changed;
        let text = match (written, failure) {
            (0, Some(e)) => e,
            (0, None) => return,
            (1, _) => fill(EN.gpg_1_signature_waits, &[self.medium.for_the()]),
            (n, _) => fill(
                EN.gpg_signatures_wait,
                &[&n.to_string(), self.medium.for_the()],
            ),
        };
        self.toast(&text);
    }
}

/// Glue of the Load sheet and Load with a passphrase (`load`).
impl Faraday {
    /// Costs what `key_fingerprint` and `described_of` cost. The first
    /// wallet of open vault `v` that key record `r` is a seed of, by name
    /// ([`Vaults::key_wallet`]).
    pub(crate) fn vault_key_wallet(&self, c: &Compute, v: usize, r: usize) -> Option<String> {
        self.vaults.key_wallet(c, self.session.network(), v, r)
    }

    /// Costs a parse of each open vault's wallet records. The seeds open
    /// vaults keep as words without a BIP-39 passphrase, not loaded, in no
    /// wallet of their vault: ones a wallet short of a key may use with a
    /// passphrase never stored (F62), each with its vault and line. Kept in
    /// the drawing memo while a frame is drawn (`docs/SPEED.md` §3.3); a
    /// press works it out afresh.
    ///
    /// Works out [`crate::drawing::View::vault_passphrase_seeds`]. Costs a parse of
    /// each open vault's wallet records.
    pub fn vault_passphrase_seeds_now(&self, c: &Compute) -> Rc<Vec<load::PassSeed>> {
        Rc::new(self.vaults.passphrase_seeds(c, &self.session))
    }

    /// **Load with a passphrase** from wallet `w` short of a key (F62):
    /// the sheet of the seeds kept with no passphrase, or, with one, its
    /// passphrase field at once.
    pub(crate) fn pass_seeds_open(&mut self, w: usize) {
        let c = self.compute();
        let seeds = self.vault_passphrase_seeds_now(&c);
        let Some(wallet) = self.session.wallets.get(w) else {
            return;
        };
        if seeds.is_empty() {
            return;
        }
        let missing: Vec<[u8; 4]> = self
            .session
            .slots(wallet)
            .iter()
            .filter(|s| s.held_by.is_none())
            .filter_map(|s| s.fingerprint.map(|f| f.0))
            .collect();
        self.vaults.state.pass_seeds = Some(load::PassSeeds {
            wallet: w,
            missing,
            picked: None,
        });
        self.vaults.state.prompt = None;
        self.sheet = Some(Sheet::PassphraseSeeds);
        if let [one] = seeds.as_slice() {
            self.pass_seed_pick(one.vault, one.record);
        }
    }

    /// Seed `r` of open vault `v` picked on the sheet: its passphrase
    /// field.
    pub(crate) fn pass_seed_pick(&mut self, v: usize, r: usize) {
        let Some(p) = self.vaults.state.pass_seeds.as_mut() else {
            return;
        };
        p.picked = Some((v, r));
        self.vaults.state.prompt = Some(crate::vault::Prompt {
            purpose: crate::vault::Purpose::ForWallet(v, r),
            text: Default::default(),
            error: None,
        });
        self.vaults.state.focus = self.may_load_keys().then_some(crate::vault::Focus::Prompt);
    }

    /// Back from a seed's field to the list.
    pub(crate) fn pass_seed_list(&mut self) {
        if let Some(p) = self.vaults.state.pass_seeds.as_mut() {
            p.picked = None;
        }
        self.vaults.state.prompt = None;
        self.vaults.state.focus = None;
    }

    /// The sheet closed: nothing typed is kept.
    pub(crate) fn pass_seeds_close(&mut self) {
        self.vaults.state.pass_seeds = None;
        self.vaults.state.prompt = None;
        self.vaults.state.focus = None;
        if self.sheet == Some(Sheet::PassphraseSeeds) {
            self.sheet = None;
        }
    }

    /// The passphrase typed for seed `r` of open vault `v`: the key it
    /// gives loads only when it is one the wallet is missing here; else
    /// nothing loads and the field says what key it gave. The passphrase
    /// is never stored. Costs a BIP-39 seed and a master key.
    pub(crate) fn pass_seed_try(&mut self, v: usize, r: usize) {
        let c = self.compute();
        if !self.may_load_keys() {
            return;
        }
        let Some(p) = self.vaults.pass_seeds.as_ref() else {
            return;
        };
        let (w, missing) = (p.wallet, p.missing.clone());
        let Some(id) = self.vaults.open.get(v).map(|o| o.id) else {
            return;
        };
        let passphrase = zeroize::Zeroizing::new(
            self.vaults
                .prompt
                .as_ref()
                .map(|p| p.text.text.to_string())
                .unwrap_or_default(),
        );
        let wanted = |fp: Fingerprint| missing.contains(&fp.0);
        let Some(loaded) = self.vaults.store().load_key_with_passphrase(
            &c,
            &mut self.session.keys,
            (id, r),
            &passphrase,
            Some(&wanted),
        ) else {
            return;
        };
        let loaded = match loaded {
            faraday_keys::vault::KeyLoad::Refused(fp) => {
                let wallet = self
                    .session
                    .wallets
                    .get(w)
                    .map_or_else(String::new, |x| x.name.clone());
                if let Some(p) = self.vaults.state.prompt.as_mut() {
                    p.text.clear();
                    p.error = Some(fill(EN.pass_seeds_not_a_key, &[&fp_text(fp), &wallet]));
                }
                return;
            }
            faraday_keys::vault::KeyLoad::Done(added) => added,
        };
        match loaded {
            Ok(fp) => {
                self.pass_seeds_close();
                self.refresh_spend();
                self.toast(&fill(EN.vaults_key_is_loaded, &[&fp_text(fp)]));
            }
            Err(e) => {
                if let Some(p) = self.vaults.state.prompt.as_mut() {
                    p.error = Some(e.text());
                }
            }
        }
    }

    /// Costs what `load_view_of` costs. What the sheet shows, kept in the
    /// drawing memo while a frame is drawn (`docs/SPEED.md` §3.3). Reads
    /// the vault's wallet records as it keeps them read; the key
    /// fingerprints are the ones the unlock worked out.
    ///
    /// Works out [`crate::drawing::View::vault_load_view`] from the state as it is.
    pub fn vault_load_view_now(&self, c: &Compute) -> Option<Rc<load::LoadView>> {
        let s = self.vaults.load.as_ref()?;
        let v = self.vaults.load_sheet_vault()?;
        Some(Rc::new(self.vaults.load_view(
            c,
            &self.session,
            v,
            &s.ticked,
        )))
    }

    /// Open vault `v` was just unlocked from the boot sheet (`boot`) or
    /// the Vaults tab: its Load sheet comes up, its cards ticked from
    /// each record's Load at unlock. Returns false, showing nothing, when
    /// the vault holds no wallet and no key, or the session holds all of
    /// them already. Reads the wallet records as the vault keeps them read ([`crate::vault::Vaults::described`]).
    pub(crate) fn load_sheet_offer(&mut self, v: usize, boot: bool, wallets_before: usize) -> bool {
        let c = self.compute();
        let Some(open) = self.vaults.open.get(v) else {
            return false;
        };
        let records: BTreeSet<usize> = self
            .vaults
            .records(open.id)
            .iter()
            .filter(|(_, r)| matches!(r.kind(), kind::KEY | kind::WALLET))
            .map(|(i, _)| i)
            .collect();
        if records.is_empty() {
            return false;
        }
        // Every record that would load at unlock; the view leaves out a
        // key on a wallet's card, which has no checkbox of its own, and
        // anything loaded already.
        let ticked: BTreeSet<usize> = records
            .into_iter()
            .filter(|&i| {
                self.vaults
                    .records(open.id)
                    .get(i)
                    .is_some_and(|r| r.loads_at_unlock())
            })
            .collect();
        // Everything it holds is in the session already: nothing to ask.
        if !self
            .vaults
            .load_view(&c, &self.session, v, &ticked)
            .tickable()
        {
            return false;
        }
        let Some(open) = self.vaults.open.get(v) else {
            return false;
        };
        self.vaults.state.load = Some(load::LoadSheet {
            name: open.name.clone(),
            boot,
            wallets_before,
            ticked,
        });
        self.sheet = Some(Sheet::UnlockLoad);
        true
    }

    /// Runs a press on the Load sheet.
    pub(crate) fn load_sheet_act(&mut self, a: VaultAction) {
        let c = self.compute();
        let Some(v) = self.vaults.load_sheet_vault() else {
            // Its vault is gone (locked): nothing is left to load.
            if self.sheet == Some(Sheet::UnlockLoad) {
                self.sheet = None;
            }
            self.vaults.state.load = None;
            return;
        };
        let Some(ticked) = self.vaults.load.as_ref().map(|s| s.ticked.clone()) else {
            return;
        };
        let view = self.vaults.load_view(&c, &self.session, v, &ticked);
        match a {
            VaultAction::LoadTick(r) => {
                if !view.tickable_records().contains(&r) {
                    return;
                }
                if let Some(s) = self.vaults.state.load.as_mut()
                    && !s.ticked.remove(&r)
                {
                    s.ticked.insert(r);
                }
            }
            VaultAction::LoadTickAll => {
                let all = view.tickable_records();
                if let Some(s) = self.vaults.state.load.as_mut() {
                    if view.any_ticked() {
                        for r in all {
                            s.ticked.remove(&r);
                        }
                    } else {
                        s.ticked.extend(all);
                    }
                }
            }
            VaultAction::LoadTicked => {
                if !view.any_ticked() || !self.may_load_keys() {
                    return;
                }
                let set = view.to_load();
                let (keys, wallets) = self.vault_load_set(v, &set);
                if wallets == 0 && keys > 0 {
                    self.toast(&fill(
                        EN.vaults_loaded,
                        &[
                            &keys.to_string(),
                            (if keys == 1 {
                                EN.vaults_seed_inline
                            } else {
                                EN.vaults_seeds
                            }),
                        ],
                    ));
                }
                self.load_sheet_close(v);
            }
            VaultAction::LoadNotNow => self.load_sheet_close(v),
            _ => {}
        }
    }

    /// The sheet is done with: the boot import goes on to its next vault
    /// or lands, as F60 has it; an unlock from the Vaults tab lands on
    /// Wallets when wallets loaded, else stays on Files.
    fn load_sheet_close(&mut self, v: usize) {
        let Some(s) = self.vaults.state.load.take() else {
            return;
        };
        if self.sheet == Some(Sheet::UnlockLoad) {
            self.sheet = None;
        }
        let file = self.vaults.open[v].name.clone();
        if s.boot {
            let label = self.import.state().map(|i| i.label.clone());
            if self.import_after_unlock() {
                self.unlock_landed(s.wallets_before, &file, label.as_deref());
            }
        } else {
            if self.screen == Screen::Unlock {
                self.screen = Screen::Files;
            }
            self.unlock_landed(s.wallets_before, &file, None);
        }
    }
}

/// Glue of what leaves a vault: Save to a stick, a SeedQR, a wallet's
/// files (`out`).
impl Faraday {
    /// The selected vault record, when it is of kind `k`.
    pub(crate) fn vault_record_of(
        &self,
        k: u8,
    ) -> Option<(usize, faraday_keys::vault::RecordView<'_>)> {
        let at = self.vault_selected_index()?;
        let v = self.vaults.open.get(self.vaults.current)?;
        self.vaults
            .records(v.id)
            .get(at)
            .filter(|r| r.kind() == k)
            .map(|r| (at, r))
    }

    /// The item page of Vault contents is on screen, no sheet over it:
    /// where its secrets may show ([`Vaults::shown_here`]).
    pub(crate) fn vault_on_page(&self) -> bool {
        self.screen == Screen::VaultContents && self.sheet.is_none()
    }

    /// After every input: the item page's secrets hidden once the place
    /// they were shown is left ([`Vaults::shown`]'s invariant).
    pub(crate) fn vault_shown_tidy(&mut self) {
        let on = self.vault_on_page();
        self.vaults.tidy_shown(on);
        self.vault_older_settle();
    }

    /// Runs `f` with the selected wallet record as a session wallet, by
    /// its index: the session's own when it holds the same wallet, else
    /// the record read in for `f` alone and taken out again, so the
    /// session is as it was. Reads the descriptor.
    pub(crate) fn with_vault_wallet<T>(
        &mut self,
        f: impl FnOnce(&mut Self, usize) -> T,
    ) -> Option<T> {
        let c = self.compute();
        let (_, r) = self.vault_record_of(kind::WALLET)?;
        let text = r.public_text(field::WALLET)?.to_string();
        let name = wallet_name_in(r, field::WALLET_NAME, EN.vaults_wallet).into_owned();
        let policy = crate::wallet::read_wallet(&c, &text).ok()?;
        let same = crate::wallet::same_wallet(&policy);
        if let Some(i) = self
            .session
            .wallets
            .iter()
            .position(|w| crate::wallet::same_wallet(w.policy()) == same)
        {
            return Some(f(self, i));
        }
        self.session.wallets.push(crate::wallet::Wallet::new(
            name,
            policy,
            EN.vaults_vault.to_string(),
            crate::wallet::Origin::Vault,
        ));
        let i = self.session.wallets.len() - 1;
        let out = f(self, i);
        self.session.wallets.truncate(i);
        Some(out)
    }

    /// Opens Save to a stick for the selected record, writing out the
    /// names of its files: for a key its words and its SeedQR, both
    /// ticked; for a wallet its descriptor, its QR pictures and the
    /// software files Back up makes, which are made now (QR pictures
    /// drawn), the descriptor alone ticked.
    pub(crate) fn vault_stick_save_open(&mut self) {
        let c = self.compute();
        let Some(at) = self.vault_selected_index() else {
            return;
        };
        let save = if self.vault_record_of(kind::KEY).is_some() {
            let Some((_, r)) = self.vault_record_of(kind::KEY) else {
                return;
            };
            let Some(m) = self.vaults.store().words_of(
                &c,
                r,
                faraday_keys::Reveal::BackupStick(faraday_keys::Part::Words),
            ) else {
                return;
            };
            let fp = crate::vault::screen::key_fingerprint(&c, self, r).unwrap_or_default();
            let mut files = Vec::new();
            for (label, what, form) in [
                (
                    EN.board_words,
                    out::SaveWhat::Words,
                    crate::plan::What::SeedFile(0),
                ),
                (
                    EN.board_seedqr,
                    out::SaveWhat::SeedQr,
                    crate::plan::What::SeedQrFile(0),
                ),
            ] {
                if what == out::SaveWhat::SeedQr
                    && m.language() != faraday_bip::bip39::Language::English
                {
                    continue;
                }
                if let Some(name) = crate::backup_flow::wallet::seed_stick_name(form, &fp) {
                    files.push(out::SaveFile {
                        label: label.to_string(),
                        what,
                        names: vec![name],
                        made: Vec::new(),
                        ticked: true,
                    });
                }
            }
            out::StickSave {
                record: at,
                seed: true,
                files,
            }
        } else {
            let Some(files) = self.with_vault_wallet(|app, i| app.wallet_save_files(i)) else {
                return;
            };
            out::StickSave {
                record: at,
                seed: false,
                files,
            }
        };
        self.vaults.state.stick_save = Some(save);
        self.sheet = Some(Sheet::VaultSave);
    }

    /// Makes session wallet `i`'s public files for the popup: the
    /// descriptor's text, its QR pictures, and each file of the software
    /// Back up writes for (Sparrow, the hardware signers, Nunchuk, Bitcoin
    /// Core) that is not the descriptor.
    fn wallet_save_files(&mut self, i: usize) -> Vec<out::SaveFile> {
        let Some(w) = self.session.wallets.get(i) else {
            return Vec::new();
        };
        let every = crate::plan::Answers {
            software: [true, true, true, true, false],
            ..Default::default()
        };
        let software = Self::backup_public_for(w, &every);
        let mut files = Vec::new();
        let mut add = |label: String, made: Vec<(String, Vec<u8>)>, ticked: bool| {
            if !made.is_empty() {
                files.push(out::SaveFile {
                    label,
                    what: out::SaveWhat::Public,
                    names: made.iter().map(|(n, _)| n.clone()).collect(),
                    made,
                    ticked,
                });
            }
        };
        add(
            EN.vault_descriptor.to_string(),
            self.public_made(i, 1),
            true,
        );
        add(
            EN.board_qr_picture.to_string(),
            self.public_made(i, 8),
            false,
        );
        for x in software.into_iter().filter(|x| *x != 1) {
            let name = match x {
                5 => EN.app_sparrow,
                2 => EN.app_coldcard_keystone,
                6 => EN.app_nunchuk,
                _ => EN.app_bitcoin_core,
            };
            add(
                fill(EN.board_software_file, &[name]),
                self.public_made(i, x),
                false,
            );
        }
        files
    }

    /// Writes the popup's ticked files to the Outbox, for the stick: a
    /// wallet's as they were made, a key's words and SeedQR picture made
    /// now from the record (a SeedQR drawn), as secrets let out
    /// unprotected. The popup closes.
    pub(crate) fn vault_stick_save_write(&mut self) {
        let c = self.compute();
        let Some(save) = self.vaults.state.stick_save.take() else {
            return;
        };
        self.sheet = None;
        let mut out: Vec<String> = Vec::new();
        let words = self
            .vaults
            .open
            .get(self.vaults.current)
            .and_then(|v| self.vaults.records(v.id).get(save.record))
            .filter(|r| r.kind() == kind::KEY)
            .and_then(|r| {
                let fp = crate::vault::screen::key_fingerprint(&c, self, r).unwrap_or_default();
                self.vaults
                    .store()
                    .words_of(
                        &c,
                        r,
                        faraday_keys::Reveal::OutboxFile(faraday_keys::Part::Words),
                    )
                    .map(|m| (m, fp))
            });
        for f in save.files.into_iter().filter(|f| f.ticked) {
            match f.what {
                out::SaveWhat::Public => {
                    for (name, bytes) in f.made {
                        self.put_outbox(&name, bytes);
                        out.push(name);
                    }
                }
                out::SaveWhat::Words | out::SaveWhat::SeedQr => {
                    let (Some((m, fp)), Some(name)) = (words.as_ref(), f.names.first()) else {
                        continue;
                    };
                    let bytes = if f.what == out::SaveWhat::Words {
                        Some(out::words_line(m))
                    } else {
                        crate::backup_flow::wallet::seedqr_png(&c, m, fp)
                    };
                    let Some(bytes) = bytes else {
                        continue;
                    };
                    self.outbox.retain(|it| it.name != *name);
                    let item = crate::Item::new(&c, name, bytes.to_vec());
                    self.outbox_secret(crate::secrets::OutReason::VaultStick, item);
                    out.push(name.clone());
                }
            }
        }
        self.save_boxes();
        match (out.as_slice(), save.seed) {
            ([], _) => {}
            ([name], true) => self.toast_waits(EN.secrets_waits_unprotected, &[name]),
            (more, true) => {
                self.toast_waits(EN.secrets_wait_unprotected, &[&more.len().to_string()]);
            }
            (names, false) => self.toast_out_files(names),
        }
    }
}
