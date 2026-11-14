//! The vault area's state and its types: the open vaults and the vault
//! screens' state ([`Vaults`]) with the item's scroll on a desktop
//! ([`VaultArea`]), the vault's actions, forms, summaries and the text
//! they are written in. Read through [`crate::Faraday::vaults`]; changed
//! through the methods here, and by the vault's own glue in
//! `vault/mod.rs` and its children.

use super::{SealSeen, SealWarn};
use crate::strings::EN;
use faraday_crypto::work::Compute;
use faraday_keys::vault::{OpenVaults, RecordView, Records, Room, VaultId};
use faraday_text::{Text, text};
use faraday_vault::records::{field, kind};
use faraday_vault::{self as fv, Cost, Record};
use std::collections::VecDeque;

use crate::work::{self, VaultAct};
use crate::{Action, Screen, flow};

pub use crate::action::is_hold;

/// The vault area.
pub(crate) struct VaultArea {
    pub(super) state: Vaults,
    item_offset: f32,
}

impl Default for VaultArea {
    fn default() -> VaultArea {
        let mut state = Vaults {
            // A new GPG key's term starts at GnuPG's default, never at
            // "never".
            gpg_years: crate::gpg::DEFAULT_YEARS,
            ..Vaults::default()
        };
        // The names new groups and titles take, set before the first edit.
        state.store.set_names(super::errors::names());
        VaultArea {
            state,
            item_offset: 0.0,
        }
    }
}

/// Reading the area is reading its [`Vaults`]: `self.vaults.open` reads
/// as the accessor [`VaultArea::vaults`] does. Nothing is changed through
/// it.
impl std::ops::Deref for VaultArea {
    type Target = Vaults;
    fn deref(&self) -> &Vaults {
        &self.state
    }
}

impl VaultArea {
    /// How many loads of seeds have been started.
    pub(crate) fn loads_started(&self) -> u32 {
        self.state.loads_started
    }

    /// Whether the press now run again has loaded vault `v` already.
    pub(crate) fn replaying(&self, v: usize) -> bool {
        self.state.replayed.contains(&v)
    }

    /// Notes the key a load for one key record added, and gives the one
    /// noted before.
    pub(crate) fn key_added_swap(&mut self, fp: Option<[u8; 4]>) -> Option<[u8; 4]> {
        std::mem::replace(&mut self.state.key_added, fp)
    }

    /// The press now being acted on is `action`, until it is put back:
    /// returns the one it replaces.
    pub(crate) fn acting_swap(&mut self, action: Option<crate::Action>) -> Option<crate::Action> {
        std::mem::replace(&mut self.state.acting, action)
    }

    /// The open vaults and the vault screens' state.
    pub(crate) fn vaults(&self) -> &Vaults {
        &self.state
    }

    /// Design units scrolled past in Vault contents' item, on a desktop.
    pub(crate) fn item_offset(&self) -> f32 {
        self.item_offset
    }

    /// Sets the item's scroll.
    pub(crate) fn set_item_offset(&mut self, y: f32) {
        self.item_offset = y;
    }

    /// Asks for the one-pass measurement, once: it goes to the worker at
    /// the next tick ([`Faraday::vault_tick`]), and is not asked again while
    /// it is out or once its figure is in.
    pub(crate) fn measure(&mut self) {
        if self.state.ms_per_unit.is_none() && self.state.measure == Measure::Idle {
            self.state.measure = Measure::Asked;
            if self.state.working.is_none() {
                self.state.drawn = false;
            }
        }
    }

    /// The field typing at `focus` goes into.
    fn text_box(&mut self, focus: Focus) -> Option<&mut TextBox> {
        match focus {
            Focus::Passphrase => Some(&mut self.state.passphrase),
            Focus::YubiKey => self.state.yubikey.as_mut(),
            Focus::Phrase(i, second) => self
                .state
                .create
                .as_mut()?
                .phrases
                .get_mut(i)
                .map(|(a, b)| if second { b } else { a }),
            Focus::Field(k) => self
                .state
                .form
                .as_mut()?
                .fields
                .get_mut(k)
                .map(|f| &mut f.2),
            Focus::Dice => self.state.dice.typing(),
            Focus::Rolls | Focus::Search(_) => None,
            Focus::Prompt => self.state.prompt.as_mut().map(|p| &mut p.text),
            Focus::PromptAgain => self.state.prompt.as_mut().map(|p| &mut p.again),
            Focus::Name => self.state.create.as_mut().map(|c| &mut c.name),
        }
    }

    /// Types `c` into the field at `focus`; false when no field is there.
    pub(crate) fn type_char(&mut self, focus: Focus, c: char) -> bool {
        match self.text_box(focus) {
            Some(b) => {
                b.text.push(c);
                true
            }
            None => false,
        }
    }

    /// Takes the last character out of the field at `focus`; false when no
    /// field is there.
    pub(crate) fn backspace(&mut self, focus: Focus) -> bool {
        match self.text_box(focus) {
            Some(b) => {
                b.text.pop();
                true
            }
            None => false,
        }
    }

    /// How long one Argon2id pass over 64 MiB takes here, ms, set as if
    /// measured.
    #[cfg(any(test, feature = "testkit"))]
    pub(crate) fn set_speed(&mut self, ms: Option<u64>) {
        self.state.ms_per_unit = ms;
    }

    /// Swaps open vaults `a` and `b` in the list, for the tests' kit.
    #[cfg(any(test, feature = "testkit"))]
    pub(crate) fn swap_open(&mut self, a: usize, b: usize) {
        self.state.open.swap(a, b);
    }

    /// Changes the vaults' summaries as `f` says, for the tests' kit.
    #[cfg(any(test, feature = "testkit"))]
    pub(crate) fn change_summaries<R>(&mut self, f: impl FnOnce(&mut Vec<VaultSummary>) -> R) -> R {
        f(&mut self.state.summaries)
    }

    /// Costs what [`Vaults::seals_write`] costs. Open vault `i`'s plans take
    /// the seal counts of the other vaults.
    pub(crate) fn seals_write(&mut self, c: &Compute, i: usize) {
        self.state.seals_write(c, i);
    }

    /// Costs what [`Vaults::seals_sealed`] costs. Open vault `i` was sealed
    /// into `bytes`.
    pub(crate) fn seals_sealed(&mut self, c: &Compute, i: usize, bytes: &[u8]) {
        self.state.seals_sealed(c, i, bytes);
    }

    /// Open vault `v` was just unlocked.
    pub(crate) fn seals_unlocked(&mut self, v: usize) {
        self.state.seals_unlocked(v);
    }

    /// Writes each fresh summary over the one kept for its vault, keeping
    /// the seal counts seen since power-on.
    pub(crate) fn merge_summaries(&mut self, fresh: Vec<VaultSummary>) {
        for mut s in fresh {
            match self.state.summaries.iter_mut().find(|o| o.id == s.id) {
                Some(o) => {
                    s.seal = o.seal.take();
                    s.copies = std::mem::take(&mut o.copies);
                    *o = s;
                }
                None => self.state.summaries.push(s),
            }
        }
        self.state.seals_planned_sync();
    }

    /// The file of the vault with identifier `id` is a copy of it the
    /// session has seen, whose SHA-256 is `digest`.
    pub(crate) fn copy_seen(&mut self, id: &[u8; 32], digest: [u8; 32]) {
        self.state.copy_seen(id, digest);
    }

    /// The summary of the vault a file in Files is a copy of, told by the
    /// file's SHA-256 ([`crate::Item::sha`]) being among the summary's
    /// copies: the only link between a file and a vault before the file
    /// is opened. `None` for a file never opened or sealed here.
    pub(crate) fn summary_of_file(&self, sha: Option<[u8; 32]>) -> Option<&VaultSummary> {
        let sha = sha?;
        self.state
            .summaries
            .iter()
            .find(|s| s.copies.contains(&sha))
    }

    /// The identifier of the vault a file in Files is a copy of
    /// ([`VaultArea::summary_of_file`]); `None` for a file never opened
    /// or sealed here.
    pub(crate) fn vault_id_of_file(&self, sha: Option<[u8; 32]>) -> Option<[u8; 32]> {
        self.summary_of_file(sha).map(|s| s.id)
    }

    /// Whether two files in Files are known to be copies of one vault:
    /// both SHA-256 are among one summary's copies.
    pub(crate) fn same_vault_files(&self, a: Option<[u8; 32]>, b: Option<[u8; 32]>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => {
                a == b
                    || self
                        .state
                        .summaries
                        .iter()
                        .any(|s| s.copies.contains(&a) && s.copies.contains(&b))
            }
            _ => false,
        }
    }

    /// Changes open vault `id`'s records through `change` ([`Vaults::edit`]).
    pub(crate) fn edit<T>(&mut self, id: VaultId, change: impl FnOnce(&mut OpenVaults) -> T) -> T {
        self.state.edit(id, change)
    }

    /// As [`Self::edit`], for a change that may be refused ([`Vaults::edit_or`]).
    pub(crate) fn edit_or<T, E>(
        &mut self,
        id: VaultId,
        change: impl FnOnce(&mut OpenVaults) -> Result<T, E>,
    ) -> Result<T, E> {
        self.state.edit_or(id, change)
    }

    /// Open vault `id` changed ([`Vaults::changed`]).
    pub(crate) fn changed(&mut self, id: VaultId) {
        self.state.changed(id);
    }

    /// Costs what [`Vaults::add_memory`] costs. The signed-amount memory's
    /// records `new` go into open vault `o`.
    #[cfg(feature = "testkit")]
    pub(crate) fn add_amounts_now(&mut self, c: &Compute, o: usize, new: Vec<Record>) {
        self.state.add_memory(c, o, &[], new);
    }

    /// A change of open vault `id` counted by [`Vaults::changed`] was taken
    /// back ([`Vaults::unchanged`]).
    pub(crate) fn unchanged(&mut self, id: VaultId) {
        self.state.unchanged(id);
    }

    /// Open vault `o` has one more change not saved.
    pub(crate) fn bump(&mut self, o: usize) {
        self.state.open[o].changes += 1;
    }

    /// Whether the record moved into open vault `o` that waits for its
    /// seal is the backup subject `subject`: a flow does not take it out.
    pub(crate) fn holds_moved_subject(&self, o: usize, subject: &str) -> bool {
        self.state
            .open
            .get(o)
            .is_some_and(|open| self.state.moved_subject_in(open.id, subject))
    }

    /// Whether an item moved into open vault `o` waits for its seal: the
    /// vault is not closed unsealed.
    pub(crate) fn holds_moved(&self, o: usize) -> bool {
        self.state
            .open
            .get(o)
            .is_some_and(|open| self.state.moved_target(open.id))
    }

    /// Whether an item moved out of open vault `o` waits for its target's
    /// seal: the vault is not closed, or its last file would be taken
    /// with the item only in the target's memory.
    pub(crate) fn moved_from(&self, o: usize) -> bool {
        self.state
            .open
            .get(o)
            .is_some_and(|open| !self.state.moved_waits_for(open.id).is_empty())
    }

    /// Closes open vault `o` and shows the first. It does not seal: the
    /// caller makes sure no item moved into it waits for its seal.
    pub(crate) fn close_at(&mut self, o: usize) {
        self.state.close(o);
        self.state.current = 0;
    }

    /// Shows open vault `o`.
    pub(crate) fn set_current(&mut self, o: usize) {
        self.state.current = o;
    }

    /// Shows kind `c` in the contents.
    pub(crate) fn set_category(&mut self, c: usize) {
        self.state.category = c;
    }

    /// Shows kind `c` in the contents as its tab does: on a small panel
    /// (`compact`) the list `offset` scrolls is kept for the kind left and
    /// set to where kind `c` was left.
    pub(crate) fn show_category(&mut self, c: usize, offset: &mut f32, compact: bool) {
        let v = &mut self.state;
        let c = c.min(CATEGORIES.len() - 1);
        if compact && c != v.category {
            if let Some(kept) = v.category_scroll.get_mut(v.category) {
                *kept = *offset;
            }
            *offset = v.category_scroll.get(c).copied().unwrap_or(0.0);
        }
        v.category = c;
    }

    /// Shows kind `c` in the contents, as chosen: listed while it holds
    /// nothing.
    pub(crate) fn choose_category(&mut self, c: usize) {
        self.state.category = c;
        self.state.category_chosen = true;
    }

    /// Points at item `n` of kind `cat` in open vault `o`.
    pub(crate) fn point_at(&mut self, o: usize, cat: usize, n: usize) {
        let v = &mut self.state;
        v.current = o;
        v.category = cat;
        v.category_chosen = true;
        v.item[cat] = n;
    }

    /// The item pointed at opened, with no form, prompt or way back.
    pub(crate) fn open_item(&mut self) {
        let v = &mut self.state;
        v.item_open = true;
        v.form = None;
        v.saving = false;
        v.prompt = None;
        v.back_to = None;
    }

    /// Vault `at` chosen to unlock.
    pub(crate) fn set_pick(&mut self, at: usize) {
        self.state.pick = at;
    }

    /// The contents as the way back left them.
    pub(crate) fn restore_view(
        &mut self,
        current: usize,
        category: usize,
        chosen: bool,
        item: [usize; 7],
        item_open: bool,
    ) {
        let v = &mut self.state;
        v.current = current;
        v.category = category;
        v.category_chosen = chosen;
        v.item = item;
        v.item_open = item_open;
    }

    /// Where typing goes.
    pub(crate) fn set_focus(&mut self, focus: Option<Focus>) {
        self.state.focus = focus;
    }

    /// Where an unlock or a creation started from another flow returns.
    pub(crate) fn set_back_to(&mut self, back: Option<Screen>) {
        self.state.back_to = back;
    }

    /// Create a vault closed, typing nowhere.
    pub(crate) fn drop_create(&mut self) {
        self.state.create = None;
        self.state.focus = None;
    }

    /// Create a vault closed, typing nowhere, with no way back.
    pub(crate) fn leave_create(&mut self) {
        self.state.back_to = None;
        self.drop_create();
    }

    /// Create a vault opened on `form`, returning to `back`, typing at
    /// `focus`.
    pub(crate) fn begin_create(&mut self, form: CreateForm, back: Screen, focus: Option<Focus>) {
        // A form started anew is no item's: a wait is of its own form.
        self.state.copy_wait = None;
        self.state.create = Some(form);
        self.state.back_to = Some(back);
        self.state.just_made = None;
        self.state.focus = focus;
    }

    /// How far Create a vault is scrolled, units, while it is open.
    pub(crate) fn create_scroll_y(&self) -> Option<f32> {
        self.state.create.as_ref().map(|c| c.scroll.y)
    }

    /// Create a vault scrolled to `y` units, while it is open.
    pub(crate) fn set_create_scroll_y(&mut self, y: f32) {
        if let Some(c) = self.state.create.as_mut() {
            c.scroll.y = y;
        }
    }

    /// Sets Create a vault's scroll, as the drawing pass noted it.
    pub(crate) fn set_create_scroll(&mut self, scroll: crate::flow::Scroll) {
        if let Some(c) = self.state.create.as_mut() {
            c.scroll = scroll;
        }
    }

    /// Vault `at` chosen to unlock, with no error, returning to `back`,
    /// typing at `focus`.
    pub(crate) fn pick_unlock(&mut self, at: usize, back: Option<Screen>, focus: Option<Focus>) {
        self.state.pick = at;
        self.state.unlock_error = None;
        self.state.back_to = back;
        self.state.focus = focus;
    }

    /// The key file chosen and the YubiKey secret typed, taken away.
    pub(crate) fn clear_unlock_parts(&mut self) {
        self.state.unlock_key = None;
        self.state.yubikey = None;
        self.state.secret_error = None;
        self.state.key_pick = 0;
        self.state.store.clear_made_parts();
    }

    /// The key file of a vault just made, held for the unlock at once.
    pub(crate) fn set_made_parts(&mut self, parts: Option<faraday_keys::UnlockParts>) {
        self.state.store.hold_made_parts(parts);
    }

    /// The unlock passphrase wiped, shown hidden, with no error.
    pub(crate) fn clear_unlock(&mut self) {
        self.clear_unlock_parts();
        self.state.passphrase.clear();
        self.state.typed_shown = false;
        self.state.unlock_error = None;
    }

    /// The unlock passphrase wiped.
    pub(crate) fn clear_passphrase(&mut self) {
        self.state.passphrase.clear();
    }

    /// Vault `at` chosen to unlock with `passphrase`, returning to
    /// `back`: the caller defers the unlock ([`Faraday::vault_defer`]).
    pub(crate) fn unlock_with(&mut self, at: usize, passphrase: &str, back: Screen) {
        self.state.pick = at;
        self.state.passphrase.set(passphrase);
        self.state.unlock_error = None;
        self.state.back_to = Some(back);
    }

    /// Work waiting for its frame: done once a frame is drawn.
    pub(crate) fn start_work(&mut self, work: Work) {
        self.state.working = Some(work);
        self.state.drawn = false;
    }

    /// No work waits or runs.
    #[cfg(feature = "testkit")]
    pub(crate) fn end_work(&mut self) {
        self.state.working = None;
    }

    /// A frame has been drawn since the work was asked for.
    pub(crate) fn set_drawn(&mut self) {
        self.state.drawn = true;
    }

    /// The press now held began at `now`, ms.
    pub(crate) fn pressed(&mut self, now: u64) {
        self.state.pressed_at = now;
    }

    /// An item's form opened on `form`, not saving, typing at its first
    /// field.
    pub(crate) fn open_form(&mut self, form: Form) {
        self.state.form = Some(form);
        self.state.saving = false;
        self.state.focus = Some(Focus::Field(0));
    }

    /// The item's form closed, typing nowhere.
    pub(crate) fn close_form(&mut self) {
        self.state.form = None;
        self.state.focus = None;
    }

    /// A new GPG key's term, in years.
    pub(crate) fn set_gpg_years(&mut self, years: u32) {
        self.state.gpg_years = years;
    }

    /// Every secret the item page shows hidden again, with what was held
    /// of them: the SeedQR, a key's words, a paperkey and the entry's
    /// code (each wiped as it drops).
    pub(crate) fn hide_shown(&mut self) {
        self.state.drop_shown();
    }

    /// The secrets shown hidden once the place they were shown is left,
    /// `on_page` whether the item page is on screen with no sheet over it
    /// ([`Vaults::shown`]'s invariant). A walk of the record list while
    /// one is shown.
    pub(crate) fn tidy_shown(&mut self, on_page: bool) {
        let left = match self.state.shown {
            Some(s) => self.state.shown_here(on_page) != Some(s.at),
            None => true,
        };
        if left {
            self.hide_item_secrets();
        }
        // A file's text on the signing page goes with the page, or with
        // a sheet (the idle warning, the Lock sheet) coming over it.
        if !self.state.signing || !on_page {
            self.state.sign_shown.clear();
        }
    }

    /// The item page's secret fields, the SeedQR, a key's words and a
    /// paperkey hidden at once, each wiped as it drops. The entry's code
    /// is the refresh's to keep or drop (`Faraday::vault_code_refresh`):
    /// dropped here, it would be made again after every input.
    pub(crate) fn hide_item_secrets(&mut self) {
        let code = self.state.code.take();
        self.hide_shown();
        self.state.code = code;
    }

    /// Those and Create a vault's passphrases, hidden at once by a lock
    /// (the eye of Create a vault stays where the person left it
    /// otherwise: [`Self::tidy_shown`] leaves it).
    pub(crate) fn hide_secrets(&mut self) {
        self.hide_item_secrets();
        if let Some(c) = self.state.create.as_mut() {
            c.shown = false;
        }
        self.state.sign_shown.clear();
    }

    /// Whether the signing page shows the text of the file with `hash`.
    pub(crate) fn sign_text_shown(&self, hash: &[u8; 32]) -> bool {
        self.state.sign_shown.contains(hash)
    }

    /// The eye of a file's text on the signing page: shows it, or hides it
    /// again.
    pub(crate) fn toggle_sign_text(&mut self, hash: [u8; 32]) {
        match self.state.sign_shown.iter().position(|h| *h == hash) {
            Some(at) => {
                self.state.sign_shown.swap_remove(at);
            }
            None => self.state.sign_shown.push(hash),
        }
    }

    /// Costs a parse of a key's words, or a paperkey's making, when the
    /// eye shows it. The eye of secret field `n` on the item page: shows
    /// it, or hides it again. The first secret of a key record (its
    /// words) or of a GPG key (its paperkey) is made here and held while
    /// it shows (`docs/SPEED.md` §3.3); a hidden one is dropped.
    pub(crate) fn toggle_shown(&mut self, c: &Compute, on_page: bool, n: usize) {
        self.tidy_shown(on_page);
        let (Some(here), Some(bit)) = (
            self.state.shown_here(on_page),
            u32::try_from(n).ok().and_then(|n| 1u64.checked_shl(n)),
        ) else {
            return;
        };
        let s = self.state.shown.get_or_insert(Shown {
            at: here,
            fields: 0,
        });
        s.fields ^= bit;
        let on = s.fields & bit != 0;
        if s.fields == 0 && self.state.seedqr.is_none() {
            self.state.shown = None;
        }
        if n != 0 {
            return;
        }
        if !on {
            self.state.words = None;
            self.state.paperkey = None;
            return;
        }
        let state = &mut self.state;
        let Some(r) = state.records(here.vault).get(here.record) else {
            return;
        };
        use faraday_keys::{Part, Reveal};
        let (words, paperkey) = match r.kind() {
            kind::KEY => (
                state.store.words_of(c, r, Reveal::VaultEntry(Part::Words)),
                None,
            ),
            kind::GPG => (None, crate::gpg::paperkey_shown(&state.store, r)),
            _ => (None, None),
        };
        state.words = words.map(|m| (here.record, m));
        state.paperkey = paperkey.map(|p| (here.record, p));
    }

    /// Costs a parse of the words and a QR encoding when it shows. The
    /// eye of Show SeedQR: makes the selected key record's code, kept
    /// while it is shown (`docs/SPEED.md` §3.3: drawing makes no code),
    /// or drops it.
    pub(crate) fn toggle_seedqr(&mut self, c: &Compute, on_page: bool) {
        self.tidy_shown(on_page);
        if self.state.seedqr.take().is_some() {
            if self.state.shown.is_some_and(|s| s.fields == 0) {
                self.state.shown = None;
            }
            return;
        }
        let Some(here) = self.state.shown_here(on_page) else {
            return;
        };
        let Some(r) = self
            .state
            .records(here.vault)
            .get(here.record)
            .filter(|r| r.kind() == kind::KEY)
        else {
            return;
        };
        let code = self
            .state
            .store
            .words_of(c, r, faraday_keys::Reveal::SeedQr)
            .filter(|m| m.language() == faraday_bip::bip39::Language::English)
            .and_then(|m| faraday_codec::seedqr::encode_seedqr(c, &m).ok());
        if let Some(mx) = code {
            self.state.seedqr = Some((here.record, mx));
            self.state.shown.get_or_insert(Shown {
                at: here,
                fields: 0,
            });
        }
    }

    /// Save to a stick closed.
    pub(crate) fn clear_stick_save(&mut self) {
        self.state.stick_save = None;
    }

    /// This computer's clock, as the shell said it at `at_ms`.
    pub(crate) fn set_clock(&mut self, unix_secs: u64, at_ms: u64) {
        self.state.unix_secs = Some(unix_secs);
        self.state.clock_at_ms = at_ms;
    }

    /// An item moved from open vault `from` into `to` waits for `to`'s seal
    /// (a test's way to leave what the Move dialog does).
    #[cfg(feature = "testkit")]
    pub(crate) fn moved_pair(&mut self, from: VaultId, to: VaultId) {
        self.state.moved_add(from, to, None);
    }

    /// Costs what [`Vaults::lock_blocks`] costs. Remembers the vaults the
    /// Lock sheet names, as it opens.
    pub(crate) fn name_lock(&mut self, c: &Compute) {
        self.state.name_lock(c);
    }

    /// Remembers the vaults the Lock sheet on show names, from what the
    /// frame's warm read: lookups only.
    pub(crate) fn name_lock_seen(&mut self) {
        self.state.name_lock_seen();
    }

    /// Free memory, MiB, as the shell said it (a shell with no reader, and
    /// the test kit, say it this way).
    pub(crate) fn set_memory_free(&mut self, mib: u32) {
        let before = self.state.memory_free_mib;
        self.state.memory_free_mib = Some(mib);
        self.forget_short_reads(before);
        self.tell_copy_room();
    }

    /// When the free figure is no longer `before`, drops what was read of a
    /// seal that memory was short for: it was read against the old figure,
    /// and is read again at the next warm. A seal that stopped for memory
    /// stays recorded (`Stuck::Short`) whatever free memory reads. Not a
    /// draw.
    fn forget_short_reads(&mut self, before: Option<u32>) {
        if before == self.state.memory_free_mib {
            return;
        }
        for o in &mut self.state.open {
            let short = o
                .read
                .read
                .get()
                .is_some_and(|r| r.refusal.is_some() || matches!(r.past, Past::Short(_)));
            if short {
                o.read.read = std::cell::OnceCell::new();
            }
        }
    }

    /// Tells the store how long a document a copy may be made of, from the
    /// free memory kept: a count that would copy more is not made (memory
    /// is short; nothing is decided from the bound).
    fn tell_copy_room(&mut self) {
        self.state
            .store
            .set_copy_room(self.state.memory_free_mib.map(copy_room_of));
    }

    /// The function that reads what this computer has free now. The store
    /// reads it too, before each count that copies a document.
    pub(crate) fn set_memory_reader(&mut self, read: fn() -> Option<u32>) {
        self.state.memory_reader = Some(read);
        self.state
            .store
            .set_copy_reader(move || read().map(copy_room_of));
    }

    /// Costs a read of the free memory: on macOS the process `vm_stat`,
    /// elsewhere one small file. Reads what this computer has free now, for the check about to be made and for the screens that show
    /// it. Without a reader the figure the shell said stands; a reader that
    /// cannot say leaves no figure, and the checks then run as for a
    /// computer that does not say.
    pub(crate) fn read_free(&mut self) -> Option<u32> {
        if let Some(read) = self.state.memory_reader {
            let before = self.state.memory_free_mib;
            self.state.memory_free_mib = read();
            self.forget_short_reads(before);
        }
        self.tell_copy_room();
        self.state.memory_free_mib
    }

    /// Whether free memory is read from this computer, not told.
    pub(crate) fn reads_free(&self) -> bool {
        self.state.memory_reader.is_some()
    }

    /// What the Create and Unlock screens show of the free figure: the
    /// most unlock memory this computer gives and whether the Create form's
    /// cost is over what is free. A lookup.
    pub(crate) fn memory_shown(&self) -> (u32, bool) {
        let over = match (self.memory_free_mib, self.form_cost()) {
            (Some(free), Some((_, mem, _))) => vault_need_mib(mem, new_file_len()) > free,
            _ => false,
        };
        (self.here_limit(), over)
    }

    /// The clock's offset from this computer's, set by the person.
    pub(crate) fn set_clock_offset(&mut self, offset: Option<i64>) {
        self.state.clock_offset = offset;
    }

    /// The clock was checked, or not.
    pub(crate) fn set_clock_checked(&mut self, checked: bool) {
        self.state.clock_checked = checked;
    }

    /// The vaults' summaries, as the previous process kept them.
    pub(crate) fn set_summaries(&mut self, summaries: Vec<super::VaultSummary>) {
        self.state.summaries = summaries;
    }

    /// Secure Boot's signing closed, its images not shown.
    pub(crate) fn sb_reset(&mut self) {
        self.state.sb_sign = None;
        self.state.sb_images = false;
    }

    /// What Secure Boot's check found.
    pub(crate) fn set_sb_checked(&mut self, line: Text) {
        self.state.sb_checked = Some(line);
    }

    /// Everything in open vault `o` chosen to load.
    pub(crate) fn load_all(&mut self, o: usize) {
        if let Some(v) = self.state.open.get_mut(o) {
            v.skip.clear();
        }
    }

    /// What open vault `o` leaves out of a load, replaced by `skip`; the
    /// old set returned.
    pub(crate) fn swap_skip(
        &mut self,
        o: usize,
        skip: std::collections::BTreeSet<usize>,
    ) -> std::collections::BTreeSet<usize> {
        std::mem::replace(&mut self.state.open[o].skip, skip)
    }

    /// The open vaults of salt `salt` named `from` renamed `to`, as their
    /// file was.
    pub(crate) fn rename_open(&mut self, salt: &[u8; 32], from: &str, to: &str) {
        for v in &mut self.state.open {
            if v.vault_id() == *salt && v.name == from {
                v.name = Text::of(to);
            }
        }
    }

    /// Practice's passphrase put in Create a vault's phrase `i` (its repeat
    /// when `again`).
    pub(crate) fn fill_phrase(&mut self, i: usize, again: bool, text: &str) {
        if let Some(c) = self.state.create.as_mut()
            && let Some((a, b)) = c.phrases.get_mut(i)
        {
            if again { b } else { a }.set(text);
        }
        self.state.unlock_error = None;
    }

    /// Practice's passphrase put in the unlock field.
    pub(crate) fn fill_unlock(&mut self, text: &str) {
        self.state.passphrase.set(text);
        self.state.unlock_error = None;
    }

    /// Practice's passphrase put in the prompt.
    pub(crate) fn fill_prompt(&mut self, text: &str) {
        if let Some(p) = self.state.prompt.as_mut() {
            p.text.set(text);
            if p.pair() {
                p.again.set(text);
            }
            p.error = None;
        }
    }

    /// Practice's passphrase put in the form's field `k`.
    pub(crate) fn fill_field(&mut self, k: usize, text: &str) {
        if let Some(f) = self.state.form.as_mut()
            && let Some(x) = f.fields.get_mut(k)
        {
            x.2.set(text);
        }
    }
}

/// The fewest characters a KeePass export's passphrase may have.
pub(crate) const KDBX_PASSPHRASE_MIN: usize = 8;

/// What a one-line prompt in the contents is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// A BIP-39 passphrase to load the selected key with.
    KeyPassphrase,
    /// A BIP-39 passphrase for seed record r of open vault v, checked
    /// against the keys a wallet is missing (F62).
    ForWallet(usize, usize),
    /// The passphrase a KeePass export of the selected entry is sealed
    /// under.
    Kdbx,
}

/// A one-line prompt in the contents' detail column.
pub struct Prompt {
    /// What it is for.
    pub purpose: Purpose,
    /// What is typed.
    pub text: TextBox,
    /// The passphrase typed again, where the prompt makes one (`pair`).
    pub again: TextBox,
    /// Why the last try failed.
    pub error: Option<Text>,
}

impl Prompt {
    /// Whether the passphrase is entered twice: one made for a KeePass
    /// export, or a BIP-39 one typed for a key, which nothing checks. A
    /// wallet's missing key is checked against the wallet and stays single.
    pub fn pair(&self) -> bool {
        !matches!(self.purpose, Purpose::ForWallet(..))
    }
}

/// A line of typing, wiped when dropped, cleared or moved.
#[derive(Default)]
pub struct TextBox {
    /// What is typed.
    pub text: crate::secret_text::SecretText,
}

impl TextBox {
    pub(crate) fn clear(&mut self) {
        self.text.clear();
    }
    pub(crate) fn set(&mut self, s: &str) {
        self.text.set(s);
    }
}

pub use crate::fresh::roller::{Act as RollerAct, PerClick as RollerPerClick};

/// Where typing goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The unlock passphrase.
    Passphrase,
    /// The unlock's YubiKey secret.
    YubiKey,
    /// Create's passphrase n, first or second field.
    Phrase(usize, bool),
    /// The form's field n.
    Field(usize),
    /// The dice rolls.
    Dice,
    /// The roller's typed number of rolls per click.
    Rolls,
    /// The prompt.
    Prompt,
    /// The prompt's passphrase typed again.
    PromptAgain,
    /// Create's name for the vault.
    Name,
    /// A search bar ([`super::SearchIn`]).
    Search(super::SearchIn),
}

/// One vault file the Inbox or Outbox holds.
pub struct VaultFile {
    /// Its file name.
    pub name: Text,
    /// It waits in the Outbox (made or sealed here), rather than having
    /// come from a stick.
    pub in_outbox: bool,
    /// What its header and length state, read once when it came in.
    pub facts: fv::file::Facts,
    /// Its SHA-256, which links it to a vault through the summaries'
    /// copies ([`VaultArea::summary_of_file`]).
    pub sha: Option<[u8; 32]>,
    /// Its length in bytes.
    pub len: usize,
    /// The open vault it is, when one is open.
    pub open: Option<usize>,
    /// The names of the other files in Files that are known copies of
    /// this vault (their SHA-256 are among its summary's copies), which
    /// it is listed once for.
    pub copies: Vec<Text>,
}

/// A wallet in an open vault that holds some of its seeds too.
#[derive(Debug, Clone)]
pub struct VaultWallet {
    /// The wallet's record.
    pub record: usize,
    /// Its name.
    pub name: Text,
    /// The records of its seeds in the same vault.
    pub keys: Vec<usize>,
    /// Signatures it needs.
    pub needed: usize,
    /// Keys it has.
    pub total: usize,
    /// It and those seeds are all loaded.
    pub done: bool,
    /// Chosen for the next load.
    pub chosen: bool,
}

/// A key or wallet in an open vault, as its load list shows it.
#[derive(Debug, Clone)]
pub struct VaultRow {
    /// Its record's place in the slot.
    pub record: usize,
    /// Its label or name.
    pub name: Text,
    /// A seed's fingerprint, a wallet's shape.
    pub detail: Text,
    /// A seed rather than a wallet.
    pub seed: bool,
    /// The session has it already.
    pub loaded: bool,
    /// Chosen for the next load.
    pub chosen: bool,
}

/// A wallet description a vault record holds, read once
/// ([`Vaults::described`]): a wallet record's wallet, a plan's, a
/// wallet share's. Public data only.
#[derive(Debug, Clone)]
pub struct Described {
    /// The description, parsed.
    pub policy: faraday_bip::policy::WalletPolicy,
    /// [`crate::wallet::same_wallet`] of it: what tells two descriptions
    /// of one wallet apart from another wallet's.
    pub same: Text,
    /// Its descriptor checksum.
    pub checksum: Text,
    /// Its shape as the session names one: "Single key", "2 of 3".
    pub shape: Text,
}

impl Described {
    /// What is kept of a parsed description.
    pub(super) fn of(policy: faraday_bip::policy::WalletPolicy) -> Described {
        Described {
            same: Text::of(&crate::wallet::same_wallet(&policy)),
            checksum: Text::of(policy.checksum()),
            shape: Text::of(&crate::vault::screen::policy_shape(policy.clone())),
            policy,
        }
    }
}

/// The most key records' fingerprints kept ([`Vaults::fingerprints`]); the
/// next one drops the oldest. More than the key records of the open vaults
/// together, since every open vault's key records are worked out at its
/// unlock and a copy or move that judges them would otherwise find one
/// dropped and work it out again.
pub(crate) const FINGERPRINTS_KEPT: usize = 1024;
/// The most key records' account keys kept ([`Vaults::accounts`]); the
/// next one drops the oldest.
pub(crate) const ACCOUNTS_KEPT: usize = 256;

/// What an open vault keeps read of its public records
/// (`docs/SPEED.md` §2 rules 2 and 4, §3.3): the wallet description each
/// record holds, by record place, and the version of the records it was
/// read from.
///
/// Invariant: a vault's `Read` is what its records hold as they are.
/// Every change to the records goes through [`Vaults::edit`], which drops
/// it; the next ask reads them again, and every frame reads them before it
/// draws ([`Vaults::warm`] from `before_frame`), so drawing only looks
/// them up. [`OpenVaults::version`] says whether a `Read` is the records'
/// own: it is checked at every lookup in a debug build, and
/// `a_wallet_record_changed_in_an_open_vault_shows_as_it_now_is` changes the records
/// to test it.
pub(super) struct Read {
    /// [`OpenVaults::version`] of the records when this was read.
    version: u64,
    /// The wallet description each holds.
    described: Vec<Option<Described>>,
    /// Whether the next seal is past the ceiling ([`Vaults::past_ceiling`]).
    past: Past,
    /// Why this computer cannot seal it now for memory
    /// ([`Vaults::seal_refusal`]); read only for a vault with changes.
    refusal: Option<Text>,
}

/// Whether the plaintext of a seal is past the ceiling, decided on the exact
/// count only.
pub(super) enum Past {
    /// It is not: under it by the bound or by the count.
    No,
    /// It is, taking this many bytes (`usize::MAX` for a vault the seal
    /// cannot write).
    By(usize),
    /// Not known: the exact count needs a copy of the document that what is
    /// free does not hold. What memory needs, in words; nothing is past
    /// anything, trimmed or lost.
    Short(Text),
}

/// What stops a lock's seals ([`Vaults::lock_blocks_seen`]): every vault a
/// press of Lock without saving would lose, each with its line.
#[derive(Default)]
pub struct LockBlocks {
    /// The vaults named, in the order of the open vaults.
    pub named: Vec<VaultId>,
    /// One line for each: past the ceiling, short of memory (with what it
    /// needs), or holding an item moved to another named vault.
    pub lines: Vec<Text>,
    /// Some named vault is past the ceiling or cannot be written: a lock
    /// that tries again cannot go on, only one that lets them go. A vault
    /// named only for an item moved to a short one does not count.
    pub loses: bool,
    /// Some named vault is short of memory.
    pub short: bool,
}

/// Why a seal that was tried stopped for a vault, remembered so that the
/// Lock sheet names it (`stuck_of`).
pub(crate) enum Stuck {
    /// The seal ran out of memory half way, with what it needed and what
    /// was free in `why`: kept, whatever free memory reads after, until the
    /// vault changes, a seal of it succeeds, or it is let go or closed.
    Short { why: Text },
    /// The store gave no job to seal the vault: kept until it changes.
    Unwritable,
}

/// Why a seal cannot be made.
pub(crate) struct Cannot {
    /// What stops it, in words.
    pub(crate) why: Text,
    /// Memory is short, the changes are not past the ceiling: the vault stays
    /// open with its changes and a lock loses nothing.
    pub(crate) short: bool,
}

impl Vaults {
    /// Costs what [`Vaults::selected_place`] costs. The selected record
    /// of the kind shown in the open vault on the contents screen, by its
    /// place among the vault's records: among those its search shows.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_place()
    }

    /// Costs what [`Vaults::words_id`] costs. The label of the open vault
    /// with a key record of the words whose id is `id`, from any list.
    pub(crate) fn words_vault(&self, c: &Compute, id: &[u8; 32]) -> Option<Text> {
        self.open.iter().find_map(|v| {
            let mut keys = self.records(v.id).of(kind::KEY);
            keys.any(|(i, _)| self.words_id(c, v.id, i) == Some(id))
                .then(|| v.label(self.store()))
        })
    }

    /// The label of the open vault holding a key record whose id
    /// ([`faraday_keys::vault::key_record_id`]) is `id`: where a key of a
    /// master seed (codex32, SLIP-39) is kept. A hash per key record.
    pub(crate) fn seed_vault(&self, id: [u8; 32]) -> Option<Text> {
        self.open.iter().find_map(|v| {
            let mut keys = self.records(v.id).of(kind::KEY);
            keys.any(|(_, r)| faraday_keys::vault::key_record_id(r) == Some(id))
                .then(|| v.label(self.store()))
        })
    }

    /// Where the item page's secrets are now ([`ShownAt`]), `on_page`
    /// whether Vault contents is on screen with no sheet over it: the
    /// selected record with its own page shown, `None` anywhere else. A
    /// walk of the record list.
    pub(crate) fn shown_here(&self, on_page: bool) -> Option<ShownAt> {
        if !on_page
            || self.form.is_some()
            || self.prompt.is_some()
            || self.saving
            || self.signing
            || self.sb_images
        {
            return None;
        }
        Some(ShownAt {
            vault: self.current_id()?,
            category: self.category,
            record: self.selected_index()?,
            item_open: self.item_open,
        })
    }

    /// The item page's secret fields shown here now, a bit each; none
    /// where they were shown elsewhere. A walk of the record list.
    pub(crate) fn fields_shown(&self, on_page: bool) -> u64 {
        match (self.shown, self.shown_here(on_page)) {
            (Some(s), Some(here)) if s.at == here => s.fields,
            _ => 0,
        }
    }

    /// The selected key record's words, while its eye shows them here
    /// (made at the press, [`VaultArea::toggle_shown`]). A walk of the
    /// record list.
    pub(crate) fn words_shown(&self, on_page: bool) -> Option<&faraday_bip::bip39::Mnemonic> {
        let here = self.shown_here(on_page)?;
        self.shown
            .filter(|s| s.at == here)
            .and(self.words.as_ref())
            .filter(|(at, _)| *at == here.record)
            .map(|(_, m)| &**m)
    }

    /// The selected GPG key's paperkey, while its eye shows it here. A
    /// walk of the record list.
    pub(crate) fn paperkey_shown(&self, on_page: bool) -> Option<&str> {
        let here = self.shown_here(on_page)?;
        self.shown
            .filter(|s| s.at == here)
            .and(self.paperkey.as_ref())
            .filter(|(at, _)| *at == here.record)
            .map(|(_, p)| p.as_str())
    }

    /// The codes screen's rows as held while it is on ([`CodesHeld`]).
    pub(crate) fn codes_held(&self) -> Option<&CodesHeld> {
        self.codes.as_ref()
    }

    /// The selected entry's TOTP code as held for the page, while the
    /// page is on screen ([`CodeHeld`]). A walk of the record list.
    pub(crate) fn code_held(&self, on_page: bool) -> Option<&CodeHeld> {
        let here = self.shown_here(on_page)?;
        self.code.as_ref().filter(|h| h.at == here)
    }

    /// The selected key's SeedQR, while its eye shows it here. A walk of
    /// the record list.
    pub(crate) fn seedqr_shown(&self, on_page: bool) -> Option<&faraday_codec::qr::QrMatrix> {
        let here = self.shown_here(on_page)?;
        self.shown
            .filter(|s| s.at == here)
            .and(self.seedqr.as_ref())
            .filter(|(at, _)| *at == here.record)
            .map(|(_, m)| m)
    }

    /// Costs a read of each record put (through `fits`) and, only where
    /// they do not fit, a document write (through `used`). Why `records`
    /// do not fit open vault `id`: the bytes they take and the room left
    /// under the ceiling; None when they fit.
    pub(crate) fn no_room(&self, c: &Compute, id: VaultId, records: &[Record]) -> Option<Text> {
        let refs: Vec<&Record> = records.iter().collect();
        match self.store.fit(c, id, &[], &refs) {
            Room::Fits => return None,
            Room::Short(len) => return Some(self.short_text(id, len)),
            Room::Full => {}
        }
        let room = self.slot_limit(id).saturating_sub(self.records(id).used(c));
        let need = Text::num(records.iter().map(|r| r.size()).sum::<usize>() as u64);
        Some(Text::fill(
            EN.vaults_those_entries,
            &[&need, &Text::num(room as u64)],
        ))
    }

    /// The most plaintext a seal of open vault `id` may take: the ceiling
    /// (`docs/VAULT.md` §2), or less where a store was told to lower it.
    pub fn slot_limit(&self, id: VaultId) -> usize {
        self.store.limit_of(id).unwrap_or(fv::file::CEILING)
    }

    /// Costs what `fit` costs for no change and, only where the vault
    /// does not fit, a document write (through `counted`). Whether the
    /// plaintext a seal of open vault `id` would take is past the ceiling,
    /// decided on the exact count only ([`Past`]).
    fn past_ceiling(&self, c: &Compute, id: VaultId) -> Past {
        let none: &[&Record] = &[];
        match self.store.fit(c, id, &[], none) {
            Room::Fits => Past::No,
            Room::Short(len) => Past::Short(self.short_text(id, len)),
            Room::Full => match self.records(id).counted(c) {
                Some(n) => Past::By(n),
                None => Past::Short(self.short_text(id, self.store.held(id).map_or(0, |h| h.0))),
            },
        }
    }

    /// Costs a read of the free memory (a process on macOS) and a walk of
    /// the records, no copy. What memory is short for: a copy of a document of `len`
    /// bytes, or the whole seal where that needs more, against what is free
    /// now. "Needs 1.6 GiB. This computer has 1 GiB free."  Reads the costs
    /// the vault has kept.
    pub(crate) fn short_text(&self, id: VaultId, len: usize) -> Text {
        let free = self
            .memory_reader
            .map_or(self.memory_free_mib, |read| read())
            .unwrap_or(0);
        let need = match (self.store.next_cost(id), self.store.held(id)) {
            (Some(cost), Some((document, attachments))) => need_mib(Need::Seal {
                memory_mib: cost.memory_kib / 1024,
                document,
                attachments,
            }),
            _ => 0,
        };
        Text::fill(
            EN.vaults_needs_this_computer,
            &[&mib_text(need.max(copy_need_mib(len))), &mib_text(free)],
        )
    }

    /// Costs what [`Vaults::past_ceiling`] costs. Why open vault `i`
    /// cannot be sealed: its changes are past the ceiling (so a lock would
    /// lose them), or this computer has not the memory to count them
    /// (nothing is lost). None when it can be.
    pub(crate) fn cannot_seal(&self, c: &Compute, i: usize) -> Option<Cannot> {
        let v = self.open.get(i)?;
        if v.changes == 0 {
            return None;
        }
        self.cannot_seal_with(i, &self.past_ceiling(c, v.id))
    }

    /// What [`Vaults::cannot_seal`] says, from what [`Vaults::warm`] read
    /// before the frame: lookups only, for drawing. None until read.
    pub(crate) fn cannot_seal_seen(&self, i: usize) -> Option<Cannot> {
        let v = self.open.get(i)?;
        let read = v.read.read.get()?;
        debug_assert_eq!(
            read.version,
            self.store.version(v.id),
            "an edit of the records that did not go through Vaults::edit"
        );
        self.cannot_seal_with(i, &read.past)
    }

    /// Why open vault `i` cannot be sealed when its seal is `past` the
    /// ceiling ([`Vaults::past_ceiling`]).
    fn cannot_seal_with(&self, i: usize, past: &Past) -> Option<Cannot> {
        let v = self.open.get(i)?;
        if v.changes == 0 {
            return None;
        }
        let past = match past {
            Past::No => return None,
            Past::Short(why) => {
                return Some(Cannot {
                    why: why.clone(),
                    short: true,
                });
            }
            Past::By(n) => *n,
        };
        let why = if past == usize::MAX {
            // The store reads a vault it cannot write as full: it is not
            // a number of bytes over.
            Text::fill(EN.vault_lock_unwritable, &[&v.name])
        } else {
            let slot = self.slot_limit(v.id);
            if past <= slot {
                return None;
            }
            Text::fill(
                EN.vault_cannot_seal,
                &[&v.name, &Text::num((past - slot) as u64)],
            )
        };
        Some(Cannot { why, short: false })
    }

    /// Costs what [`Vaults::cannot_seal`] costs, for each open vault. Why
    /// the first open vault that cannot be sealed cannot be. Reads only.
    pub fn seal_blocked(&self, c: &Compute) -> Option<Text> {
        (0..self.open.len()).find_map(|i| self.cannot_seal(c, i).map(|n| n.why))
    }

    /// Costs what [`Vaults::cannot_seal`] costs, for each open vault.
    /// Whether a lock would lose a vault's changes: one is past the
    /// ceiling. Memory short loses nothing.
    pub fn seal_loses(&self, c: &Compute) -> bool {
        (0..self.open.len()).any(|i| self.cannot_seal(c, i).is_some_and(|n| !n.short))
    }

    /// Drops what was read of each open vault's seal (past the ceiling, or
    /// short of memory), to be read again at the next warm, against the
    /// memory free then. Called where a lock stops, on the change path.
    pub(crate) fn reread_seals(&mut self) {
        for o in &mut self.open {
            o.read.read = std::cell::OnceCell::new();
        }
    }

    /// What stops a lock's seals, from what was read before the frame:
    /// lookups only, for drawing. Each vault a press of Lock without saving
    /// would lose: past the ceiling, short of memory with what it needs, or
    /// with an item moved to one of those.
    pub fn lock_blocks_seen(&self) -> LockBlocks {
        self.lock_blocks_of(|i, v| {
            let cannot = self.cannot_seal_seen(i);
            let refusal = v.read.read.get().and_then(|r| r.refusal.clone());
            (cannot, refusal)
        })
    }

    /// Costs what [`Vaults::cannot_seal`] and [`Vaults::seal_refusal`] cost,
    /// for each open vault with changes. What [`Vaults::lock_blocks_seen`]
    /// says, worked out now.
    pub(crate) fn lock_blocks(&self, c: &Compute) -> LockBlocks {
        self.lock_blocks_of(|i, v| {
            let cannot = self.cannot_seal(c, i);
            let refusal = if cannot.is_none() {
                self.seal_refusal(c, v.id)
            } else {
                None
            };
            (cannot, refusal)
        })
    }

    /// The blocks of the open vaults with changes, from `find`: why each
    /// cannot be sealed and why its seal is refused for memory.
    fn lock_blocks_of(
        &self,
        find: impl Fn(usize, &OpenVault) -> (Option<Cannot>, Option<Text>),
    ) -> LockBlocks {
        let mut out = LockBlocks::default();
        let mut lines: Vec<(VaultId, Text)> = Vec::new();
        for (i, v) in self.open.iter().enumerate() {
            if v.changes == 0 {
                continue;
            }
            let line = match find(i, v) {
                (Some(n), _) if !n.short => {
                    out.loses = true;
                    n.why
                }
                (Some(Cannot { why: need, .. }), _) | (None, Some(need)) => {
                    out.short = true;
                    Text::fill(EN.vault_lock_short, &[&v.name, &need])
                }
                // A seal that stopped part way, not found by the estimates.
                (None, None) => match self.stuck_of(v.id) {
                    Some(Stuck::Short { why, .. }) => {
                        out.short = true;
                        Text::fill(EN.vault_lock_short, &[&v.name, why])
                    }
                    Some(Stuck::Unwritable) => {
                        out.loses = true;
                        Text::fill(EN.vault_lock_unwritable, &[&v.name])
                    }
                    None => continue,
                },
            };
            lines.push((v.id, line));
        }
        // A vault with an item moved to a named one goes with it: its last
        // sealed file holds the item, the target's seal does not.
        loop {
            let more = self.open.iter().find(|v| {
                !lines.iter().any(|(id, _)| *id == v.id)
                    && self
                        .moved_waits_for(v.id)
                        .iter()
                        .any(|t| lines.iter().any(|(id, _)| id == t))
            });
            let Some(v) = more else { break };
            let target = self
                .moved_waits_for(v.id)
                .into_iter()
                .filter(|t| lines.iter().any(|(id, _)| id == t))
                .find_map(|t| self.open.iter().find(|o| o.id == t))
                .map_or_else(Text::new, |o| o.name.clone());
            // Not `loses`: the target it waits for is named for what stops
            // it, and where that is only memory, Lock tries again.
            lines.push((v.id, Text::fill(EN.vault_lock_moved, &[&v.name, &target])));
        }
        // The open vaults' order.
        lines.sort_by_key(|(id, _)| self.open.iter().position(|o| o.id == *id));
        out.named = lines.iter().map(|(id, _)| *id).collect();
        out.lines = lines.into_iter().map(|(_, l)| l).collect();
        out
    }

    /// A lookup. How the last seal of open vault `id` stopped, while the
    /// vault's records are as they were.
    fn stuck_of(&self, id: VaultId) -> Option<&Stuck> {
        let version = self.store.version(id);
        self.stuck
            .iter()
            .find(|(i, v, _)| *i == id && *v == version)
            .map(|(_, _, s)| s)
    }

    /// Whether open vault `id` has a short-of-memory seal remembered, its
    /// records as they were.
    pub(crate) fn has_short(&self, id: VaultId) -> bool {
        matches!(self.stuck_of(id), Some(Stuck::Short { .. }))
    }

    /// Forgets how the last seal of open vault `id` stopped.
    pub(crate) fn clear_stuck(&mut self, id: VaultId) {
        self.stuck.retain(|(i, _, _)| *i != id);
    }

    /// Remembers that the seal of open vault `id` stopped as `why` says, so
    /// that the Lock sheet names it though the estimates find it fits.
    pub(crate) fn set_stuck(&mut self, id: VaultId, why: Stuck) {
        let version = self.store.version(id);
        self.stuck.retain(|(i, _, _)| *i != id);
        self.stuck.push((id, version, why));
    }

    /// Costs what [`Vaults::lock_blocks`] costs. Remembers the vaults the
    /// Lock sheet names, as it is opened, for the press of Lock without
    /// saving to let go and no others. Change path, never a draw.
    pub(crate) fn name_lock(&mut self, c: &Compute) {
        self.lock_named = self.lock_blocks(c).named;
    }

    /// What [`Vaults::name_lock`] does, from what [`Vaults::warm`] has read:
    /// lookups only. The frame before a Lock sheet is drawn.
    pub(crate) fn name_lock_seen(&mut self) {
        self.lock_named = self.lock_blocks_seen().named;
    }

    /// What [`Vaults::seal_loses`] says from what was read before the
    /// frame: lookups only, for drawing.
    pub fn seal_loses_seen(&self) -> bool {
        (0..self.open.len()).any(|i| self.cannot_seal_seen(i).is_some_and(|n| !n.short))
    }

    /// Costs a read of the free memory, a read of each record changed, each
    /// time it checks the fit, and, past the ceiling by the bound, a document
    /// write for each pass. The memory's records `new` (signed amounts and
    /// nonce records) go into open vault `i`, after the nonce records of
    /// the transactions `replace` are taken out, and the oldest records
    /// leave while the vault holds over [`CAP`] nonce entries, oldest
    /// first, and, only while it is past the ceiling, for room: a nonce
    /// record before any signed amounts (`docs/VAULT.md` §7, types 10 and
    /// 17). Every nonce record put in this call is kept, so a seal that
    /// puts more than the cap leaves the vault over it until a later seal.
    pub(crate) fn add_memory(
        &mut self,
        c: &Compute,
        i: usize,
        replace: &[[u8; 32]],
        new: Vec<Record>,
    ) {
        let Some(v) = self
            .open
            .get(i)
            .filter(|_| !new.is_empty() || !replace.is_empty())
        else {
            return;
        };
        let id = v.id;
        let puts_nonces = new.iter().any(|r| r.kind == kind::NONCES);
        let new_txs: Vec<[u8; 32]> = new
            .iter()
            .filter(|r| r.kind == kind::NONCES)
            .filter_map(|r| r.field(faraday_vault::records::field::NONCES_TX))
            .filter_map(|t| <[u8; 32]>::try_from(t).ok())
            .collect();
        self.edit(id, |s| {
            for tx in replace {
                let at = s.records(id).of(kind::NONCES).find_map(|(k, r)| {
                    (r.public(faraday_vault::records::field::NONCES_TX) == Some(&tx[..]))
                        .then_some(k)
                });
                if let Some(k) = at {
                    s.remove(id, k);
                }
            }
            for r in new {
                s.insert(id, r);
            }
            if puts_nonces {
                let entries = |s: &OpenVaults| {
                    s.records(id)
                        .of(kind::NONCES)
                        .map(|(_, r)| {
                            r.public_all(faraday_vault::records::field::NONCES_ENTRY)
                                .count()
                        })
                        .sum::<usize>()
                };
                while entries(s) > crate::memory::nonces::CAP {
                    // Only older records go, oldest first; those put in
                    // this call stay, so the vault may remain over the cap
                    // until a later seal.
                    let oldest = s
                        .records(id)
                        .of(kind::NONCES)
                        .find(|(_, r)| {
                            !r.public(faraday_vault::records::field::NONCES_TX)
                                .is_some_and(|t| new_txs.iter().any(|p| &p[..] == t))
                        })
                        .map(|(k, _)| k);
                    match oldest {
                        Some(k) if s.remove(id, k) => {}
                        _ => break,
                    }
                }
            }
            // Trimmed only on an exact count past the ceiling, one count
            // for each pass: a vault the seal refuses for another reason
            // (the count says `usize::MAX`), or that memory is too short to
            // count (`None`), loses nothing here; the seal says why it waits.
            let limit = s.limit_of(id).unwrap_or(fv::file::CEILING);
            while s.read_only(id).is_none()
                && s.records(id)
                    .counted(c)
                    .is_some_and(|n| n != usize::MAX && n > limit)
            {
                let oldest = s
                    .records(id)
                    .iter()
                    .find(|(_, r)| r.kind() == kind::NONCES)
                    .or_else(|| {
                        s.records(id)
                            .iter()
                            .find(|(_, r)| r.kind() == kind::AMOUNTS)
                    });
                match oldest.map(|(k, _)| k) {
                    Some(k) if s.remove(id, k) => {}
                    _ => break,
                }
            }
        });
        self.changed(id);
    }

    /// The open vaults' records, for reading. Every change goes through
    /// [`Vaults::edit`].
    pub fn store(&self) -> &OpenVaults {
        &self.store
    }

    /// Open vault `id`'s records, for reading.
    pub fn records(&self, id: VaultId) -> Records<'_> {
        self.store.records(id)
    }

    /// Changes open vault `id`'s records with `change`, which has the
    /// store to edit, and drops what was read of them ([`Read`]). The one
    /// way the records of an open vault change.
    pub fn edit<T>(&mut self, id: VaultId, change: impl FnOnce(&mut OpenVaults) -> T) -> T {
        let out = change(&mut self.store);
        // A secret changed in place leaves no old code showing.
        self.code = None;
        self.codes = None;
        if let Some(o) = self.open.iter_mut().find(|o| o.id == id) {
            o.read = Kept::default();
        }
        out
    }

    /// As [`Vaults::edit`], for a change that may be refused: what was
    /// read is dropped only when `change` gives `Ok`, so a refusal leaves
    /// the vault as it was.
    pub fn edit_or<T, E>(
        &mut self,
        id: VaultId,
        change: impl FnOnce(&mut OpenVaults) -> Result<T, E>,
    ) -> Result<T, E> {
        let out = change(&mut self.store)?;
        Ok(self.edit(id, |_| out))
    }

    /// Counts one change not yet sealed in open vault `id`.
    pub fn changed(&mut self, id: VaultId) {
        if let Some(o) = self.open.iter_mut().find(|o| o.id == id) {
            o.changes += 1;
            o.read.read = std::cell::OnceCell::new();
        }
    }

    /// Takes back one change counted by [`Vaults::changed`] for a change
    /// that was itself taken back.
    pub fn unchanged(&mut self, id: VaultId) {
        if let Some(o) = self.open.iter_mut().find(|o| o.id == id) {
            o.changes = o.changes.saturating_sub(1);
            o.read.read = std::cell::OnceCell::new();
        }
    }

    /// The open vault on the contents screen.
    pub fn current_id(&self) -> Option<VaultId> {
        self.open.get(self.current).map(|o| o.id)
    }

    /// Costs what [`Vaults::read`] costs the first time after a change, a
    /// lookup after, and for each wallet description and each key record's
    /// words a parse. Reads open
    /// vault `id`'s records, and finds what the search bars hold in them
    /// ([`super::search::Found`]), unless that is done already.
    pub fn warm(&self, c: &Compute, id: VaultId) {
        let _ = self.read(c, id);
        let _ = self.shown(id, 0);
    }

    /// Costs a read of the free memory and, past the bound's ceiling or
    /// where a seal's bound is over what is free, a copy of the document and a
    /// count of it ([`Vaults::past_ceiling`], [`Vaults::seal_refusal`]), a
    /// parse of each description, once after a change. What is read of open
    /// vault `id`'s records.
    fn read(&self, c: &Compute, id: VaultId) -> Option<&Read> {
        let o = self.open.iter().find(|o| o.id == id)?;
        let read = o.read.read.get_or_init(|| {
            // The key records' words are read with the descriptions, once.
            self.store.warm(c, id);
            let past = self.past_ceiling(c, id);
            // What memory refuses is asked only where the vault is not
            // already stopped by the ceiling or by a count it cannot make:
            // the sheet and the seal say that first.
            let stopped = self
                .open
                .iter()
                .position(|v| v.id == id)
                .is_some_and(|i| self.cannot_seal_with(i, &past).is_some());
            Read {
                version: self.store.version(id),
                refusal: if o.changes > 0 && !stopped {
                    self.seal_refusal(c, id)
                } else {
                    None
                },
                past,
                described: self
                    .store
                    .records(id)
                    .iter()
                    .map(|(_, r)| {
                        let f = match r.kind() {
                            kind::WALLET => field::WALLET,
                            kind::PLAN => field::PLAN_WALLET,
                            kind::WALLET_SHARE => field::SHARE_WALLET,
                            _ => return None,
                        };
                        let policy = crate::wallet::read_wallet(c, r.public_text(f)?).ok()?;
                        Some(Described::of(policy))
                    })
                    .collect(),
            }
        });
        debug_assert_eq!(
            read.version,
            self.store.version(id),
            "an edit of the records that did not go through Vaults::edit"
        );
        Some(read)
    }

    /// Costs what `warm` costs. The wallet description record `i` of open
    /// vault `id` holds, read: a wallet record's wallet, a plan's, a
    /// wallet share's; `None` for another record or one whose description
    /// does not read. A lookup once the records are read.
    pub fn described(&self, c: &Compute, id: VaultId, i: usize) -> Option<&Described> {
        self.read(c, id)?.described.get(i)?.as_ref()
    }

    /// Costs what `described` costs. The wallet description record `r`
    /// holds, read, when `r` is an open vault's own record (told by where
    /// it is, not by what it says); `None` for a copy.
    pub fn described_rec(&self, c: &Compute, r: RecordView<'_>) -> Option<&Described> {
        let (id, i) = r.home()?;
        self.described(c, id, i)
    }

    /// Costs what `described` costs. Each record of kind `k` of open vault
    /// `id` with the wallet description it holds, read.
    pub fn described_of(
        &self,
        c: &Compute,
        id: VaultId,
        k: u8,
    ) -> impl Iterator<Item = (usize, RecordView<'_>, &Described)> {
        self.store
            .records(id)
            .of(k)
            .filter_map(move |(i, r)| self.described(c, id, i).map(|d| (i, r, d)))
    }

    /// Costs what `warm` costs. How many words key record `r`'s seed has,
    /// as its open vault keeps it read; a record that is no open vault's
    /// own (a copy) is read afresh, which costs a parse.
    pub fn record_word_count(&self, c: &Compute, r: RecordView<'_>) -> Option<usize> {
        if let Some((id, _)) = r.home() {
            self.warm(c, id);
        }
        self.store.record_word_count(c, r)
    }

    /// Costs what `warm` costs. The word list key record `r`'s words are
    /// in, as its open vault keeps it read (public, kept with the
    /// count); a record that is no open vault's own is read afresh,
    /// which costs a parse.
    pub fn record_language(
        &self,
        c: &Compute,
        r: RecordView<'_>,
    ) -> Option<faraday_bip::bip39::Language> {
        if let Some((id, _)) = r.home() {
            self.warm(c, id);
        }
        self.store.record_language(c, r)
    }

    /// Costs what `warm` costs. Key record `r`'s words'
    /// [`crate::wallet::WordsId`], as its open vault keeps it; a record
    /// that is no open vault's own (a copy) is read afresh, which costs a
    /// parse.
    pub fn record_words_id(
        &self,
        c: &Compute,
        r: RecordView<'_>,
    ) -> Option<crate::wallet::WordsId> {
        if let Some((id, _)) = r.home() {
            self.warm(c, id);
        }
        self.store.record_words_id(c, r)
    }

    /// Costs what `warm` costs. Key record `i` of open vault `id`'s words'
    /// [`crate::wallet::WordsId`]; a lookup once the records are read.
    pub fn words_id(&self, c: &Compute, id: VaultId, i: usize) -> Option<&[u8; 32]> {
        self.warm(c, id);
        self.store.words_id(c, id, i)
    }

    /// Costs what `warm` costs. How many words key record `i` of open
    /// vault `id` has; a lookup once the records are read.
    pub fn word_count(&self, c: &Compute, id: VaultId, i: usize) -> Option<usize> {
        self.warm(c, id);
        self.store.word_count(c, id, i)
    }
}

/// What a form field of a record of type `kind` is, when it holds a
/// secret: the part a reveal names.
pub(super) fn form_part(kind_: u8, number: u8) -> Option<faraday_keys::Part> {
    use faraday_keys::Part;
    match (kind_, number) {
        (kind::ENTRY, field::PASSWORD) => Some(Part::Password),
        (kind::ENTRY, field::NOTES) => Some(Part::EntryNotes),
        (kind::ENTRY, field::TOTP) => Some(Part::Totp),
        (kind::NOTE, field::NOTE) => Some(Part::Note),
        _ => None,
    }
}

/// A key record's identity, kept once worked out: the record's id (a
/// hash of its payload and passphrase), its fingerprint, and its master
/// key's [`crate::wallet::master_id`].
pub type KeyIdentity = ([u8; 32], Text, [u8; 32]);

/// A key record's account key at a path, kept once worked out: the
/// record's id, the path as text, and the account key's public key and
/// chain code.
pub(crate) type KeyAccount = ([u8; 32], Text, Option<[u8; 65]>);

/// One open slot of one vault: what the screens say of it. Its slot key
/// and records are in [`Vaults::store`], under [`OpenVault::id`].
pub struct OpenVault {
    /// The file name it came under, and goes back under.
    pub name: Text,
    /// Which slot of [`Vaults::store`] it is.
    pub id: VaultId,
    /// What the header and length of the file it was opened from stated,
    /// copied from the store when it opened: public, and the same for as
    /// long as it is open.
    pub(super) facts: fv::file::Facts,
    /// The vault's identifier, from the store when it opened.
    pub(super) vault_id: [u8; 32],
    /// Changes not yet sealed.
    pub changes: usize,
    /// Keys and wallets left out of the next load, by record position.
    pub skip: std::collections::BTreeSet<usize>,
    /// Its seeds and wallets listed one by one on Files, under its
    /// wallets with their seeds.
    pub each_shown: bool,
    /// What is read of its records and what its searches find
    /// ([`Kept`]); dropped by every edit.
    pub(super) read: Kept,
    /// The SHA-256 of the slot as the file it was opened from holds it,
    /// kept at unlock ([`super::seals`]).
    pub(super) seal_digest: [u8; 32],
    /// What the seal count says of the copy opened, kept at unlock.
    pub(super) seal_warn: Option<SealWarn>,
    /// Why it is open to read only, as the head says it; `None` when it
    /// can be edited. Kept at unlock.
    pub(super) read_only: Option<&'static str>,
    /// What the file's size says, as the head shows it: kept at unlock.
    pub(super) size: Text,
    /// The head's line for settings other than Faraday writes, kept at
    /// unlock; empty when they are Faraday's.
    pub(super) settings: Text,
}

/// What an open vault keeps worked out from its records: what is read of
/// them ([`Read`]) and what the search bars find in them
/// ([`super::search::Found`]), each dropped when what it comes from
/// changes.
#[derive(Default)]
pub(super) struct Kept {
    pub(super) read: std::cell::OnceCell<Read>,
    pub(super) found: std::cell::OnceCell<super::search::Found>,
}

impl OpenVault {
    /// The slot's label, or "Vault". `store` is the [`Vaults::store`] it
    /// is in.
    pub fn label(&self, store: &OpenVaults) -> Text {
        store
            .records(self.id)
            .of(kind::SLOT_LABEL)
            .next()
            .and_then(|(_, r)| r.public_text(field::LABEL))
            .map_or_else(|| Text::of(EN.vaults_vault), Text::of)
    }

    /// What the header and length of the file it was opened from stated.
    pub fn facts(&self) -> &fv::file::Facts {
        &self.facts
    }

    /// The head's line of why the vault is open to read only, or `None`
    /// when it can be edited. A lock closes a read-only vault without
    /// sealing it.
    pub fn read_only(&self) -> Option<&'static str> {
        self.read_only
    }

    /// The size the file shows, or that another program saved it.
    pub fn size_text(&self) -> &str {
        &self.size
    }

    /// "Saved in AES-256 / Argon2d / gzip", or empty when the settings are
    /// Faraday's.
    pub fn settings_text(&self) -> &str {
        &self.settings
    }

    /// The vault's identifier: its root group's UUID in hex, kept for as
    /// long as it is open.
    pub fn vault_id(&self) -> [u8; 32] {
        self.vault_id
    }

    /// The SHA-256 of the copy this vault was opened from, or was last
    /// sealed into.
    pub(crate) fn copy_digest(&self) -> [u8; 32] {
        self.seal_digest
    }
}

/// Create a vault's form.
pub struct CreateForm {
    /// The open step.
    pub open: Option<u8>,
    /// Steps closed as done, by `vstep`.
    pub done: [bool; vstep::COUNT],
    /// Customise has opened the three cards the size stands for.
    pub customise: bool,
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The machines chosen, in `MACHINES` order.
    pub on: [bool; 5],
    /// This computer is not one the vault is sized for: it is chosen
    /// until unticked, for a vault that asks less than it can give.
    pub not_here: bool,
    /// The preset chosen, `CUSTOM` for Custom; `None` takes the
    /// suggestion.
    pub preset: Option<usize>,
    /// A custom cost's memory, MiB.
    pub memory: u32,
    /// A custom cost's passes.
    pub passes: u32,
    /// The vault's name, which its file is named after.
    pub name: TextBox,
    /// Each passphrase, typed twice.
    pub phrases: Vec<(TextBox, TextBox)>,
    /// For each passphrase the dice made, what they made and its bits:
    /// its strength holds while the field still holds those words.
    pub from_dice: Vec<Option<(Text, f32, crate::fresh::roller::Sim)>>,
    /// The passphrases are shown in the clear, not masked: a person
    /// cannot accept one on faith, only on seeing it.
    pub shown: bool,
    /// What stops the vault being made.
    pub error: Option<Text>,
    /// The key file the vault will need, if one was added.
    pub key: KeyChoice,
}

/// The key file Create a vault is asked to make the vault need. Nothing
/// is made or read until **Create vault**.
#[derive(Clone, PartialEq, Eq, Default)]
pub enum KeyChoice {
    /// No key file.
    #[default]
    None,
    /// A key file drawn at Create.
    Make,
    /// This file in Files, read at Create.
    File(Text),
}

impl Default for CreateForm {
    fn default() -> Self {
        // The size has a default, so the flow opens on Name and
        // passphrases (`DESIGN.md` §4.14).
        let mut done = [false; vstep::COUNT];
        done[usize::from(vstep::PRESET)] = true;
        let (_, preset, on) = SIZES[DEFAULT_SIZE];
        CreateForm {
            open: Some(vstep::PHRASES),
            done,
            customise: false,
            scroll: flow::Scroll::default(),
            on,
            not_here: false,
            preset: Some(preset),
            memory: 512,
            passes: 3,
            name: TextBox::default(),
            phrases: vec![(TextBox::default(), TextBox::default())],
            from_dice: vec![None],
            shown: false,
            error: None,
            key: KeyChoice::None,
        }
    }
}

impl CreateForm {
    /// A new form: on the default size, or on Light where the computer does
    /// not say what memory it has free and makes only that
    /// (`docs/VAULT.md` §3.1).
    pub fn starting(memory_free_mib: Option<u32>) -> Self {
        let mut form = Self::default();
        if memory_free_mib.is_none() {
            form.preset = Some(0);
        }
        form
    }

    /// The steps shown, in order: the preset and the passphrases, or, once
    /// Customise is pressed, the two cards the preset stands for in its
    /// place.
    pub fn steps(&self) -> &'static [u8] {
        if self.customise {
            &[vstep::WHERE, vstep::COST, vstep::PHRASES]
        } else {
            &[vstep::PRESET, vstep::PHRASES]
        }
    }
}

/// Create a vault's steps, plugged into Back, Forward and the choices
/// that move on (F103, F104).
pub(crate) struct CreateVaultSteps;

/// The one [`CreateVaultSteps`].
pub(crate) static CREATE_VAULT_STEPS: CreateVaultSteps = CreateVaultSteps;

impl crate::trail::StepFlow for CreateVaultSteps {
    fn steps(&self, app: &crate::Faraday) -> Option<crate::trail::Steps> {
        let c = app.vaults.create.as_ref()?;
        let mut s = crate::trail::Steps::new(c.steps().to_vec(), c.open);
        // The vault being made takes no Back.
        s.running = app.vaults.working == Some(Work::Create);
        Some(s)
    }

    fn open(&self, _: &crate::Faraday, k: u8) -> Action {
        Action::Vault(VaultAction::CStep(k))
    }

    fn name(&self, _: &crate::Faraday, k: u8) -> &'static str {
        super::screen::VSTEPS[usize::from(k)]
    }

    fn moves_on(&self, app: &crate::Faraday, chosen: Action) -> Option<Action> {
        app.vaults.create.as_ref()?;
        let step = match chosen {
            Action::Vault(VaultAction::CSizeRow(_)) => vstep::PRESET,
            // Custom opens its two rows, under Continue.
            Action::Vault(VaultAction::CPreset(k)) if k < CUSTOM => vstep::COST,
            _ => return None,
        };
        Some(Action::Vault(VaultAction::CNext(step)))
    }
}

/// The form adding or editing an item.
pub struct Form {
    /// The record type.
    pub kind: u8,
    /// The record being edited, by position; `None` adds one.
    pub record: Option<usize>,
    /// The fields: number, label, typing, shown masked.
    pub fields: Vec<(u8, &'static str, TextBox, bool)>,
}

impl Form {
    /// Whether what is typed into it is secret: an entry's or a note's
    /// form. A name, a label or a description is not.
    pub fn holds_secrets(&self) -> bool {
        matches!(self.kind, kind::ENTRY | kind::NOTE | PASSPHRASE_FORM)
    }
}

/// The `Form::kind` of Change passphrase: two masked fields, the new
/// passphrase and the same again. Not a record type.
pub const PASSPHRASE_FORM: u8 = 0xF0;

/// Work that waits for a frame saying it is under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    /// Unlock the chosen vault.
    Unlock,
    /// Make the vault the form describes.
    Create,
    /// Make the key the backup flow's form described.
    FlowMake,
    /// Write the KeePass export the prompt is for.
    Kdbx,
}

/// Where the one-pass measurement (one Argon2id pass over 64 MiB, for the
/// unlock estimates) is: it is asked for once and runs once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Measure {
    /// Not asked for, or its figure is in.
    #[default]
    Idle,
    /// Asked for; it starts at the next tick.
    Asked,
    /// Out on the worker.
    Out,
}

/// The machines ticked in an open vault's Memory dialog: this computer
/// unless `not_here`, and those of `MACHINES` in `on`. A default has this
/// computer alone.
#[derive(Default)]
struct MemoryTicks {
    not_here: bool,
    on: [bool; 5],
}

/// The app's vaults.
#[derive(Default)]
pub struct Vaults {
    /// The open slots, as the screens say them.
    pub open: Vec<OpenVault>,
    /// The open slots' keys and records, held by faraday-keys: each of
    /// [`Vaults::open`] is a slot of it, by its `id`. They are made and
    /// closed together ([`Vaults::close`], [`Vaults::close_all`]). Read
    /// through [`Vaults::store`]; changed through [`Vaults::edit`].
    pub(super) store: OpenVaults,
    /// The copies this device sealed this power-on: a vault's identifier,
    /// the digest of the copy opened and the digest of the copy its seal
    /// wrote. RAM only ([`Vaults::copy_descends`]).
    pub(super) sealed_from: Vec<([u8; 32], [u8; 32], [u8; 32])>,
    /// Create a vault, while it is on screen.
    pub create: Option<CreateForm>,
    /// The machines ticked in the Memory dialog, its own.
    memory_ticks: MemoryTicks,
    /// The vault chosen to unlock, by its place in the list.
    pub pick: usize,
    /// The unlock passphrase.
    pub passphrase: TextBox,
    /// Why the last unlock failed.
    pub unlock_error: Option<Text>,
    /// Unlock's page is to bring a refusal or failure just said into
    /// view, once ([`crate::vault::screen`]'s small panel).
    pub unlock_follow: std::cell::Cell<bool>,
    /// The key file chosen to unlock with: its name in Files. Nothing of
    /// it is read until Unlock.
    pub unlock_key: Option<Text>,
    /// The YubiKey secret field, while it is shown.
    pub yubikey: Option<TextBox>,
    /// What the YubiKey secret was refused for, under its field.
    pub secret_error: Option<Text>,
    /// The file the key file dialog has marked, by its place in the list.
    pub key_pick: usize,
    /// The key file dialog is Create a vault's: its Choose sets the form's
    /// key file, not Unlock's.
    pub key_for_create: bool,
    /// The sheet shows Create's two ways to have a key file, not the list.
    pub key_menu: bool,
    /// The open vault's ⋮ menu or Key file dialog, while it is up (the
    /// key file sheet, `vault::more`).
    pub more: Option<super::more::More>,
    /// The file of Files the open vault's pending key file was read from,
    /// by vault: it leaves Files once the seal that writes it succeeds.
    pub key_chosen: Option<(VaultId, Text)>,
    /// The Outbox name of the `.keyx` made for a vault's pending key file,
    /// until a seal writes that key or the key is replaced or taken back.
    pub key_outbox: Vec<(VaultId, Text)>,
    /// Whether the unlock last tried used a key file and a YubiKey secret.
    pub unlock_tried: (bool, bool),
    /// The open vault on the contents screen.
    pub current: usize,
    /// The kind shown.
    pub category: usize,
    /// The kind shown was chosen, by the person or the flow that opened
    /// the contents, rather than set at unlock: it is listed even while it
    /// holds nothing.
    pub category_chosen: bool,
    /// The item shown in each kind.
    pub item: [usize; 7],
    /// On a small panel, where each kind's list was scrolled to when it
    /// was last left: a kind shown again shows that place.
    pub category_scroll: [f32; 7],
    /// On a small panel, the chosen item is the page, not the list.
    pub item_open: bool,
    /// Adding or editing an item.
    pub form: Option<Form>,
    /// Saving session keys or wallets into the vault.
    pub saving: bool,
    /// Where typing goes.
    pub focus: Option<Focus>,
    /// Free memory, MiB, as last read (or said).
    pub memory_free_mib: Option<u32>,
    /// How the shell reads what is free now.
    pub memory_reader: Option<fn() -> Option<u32>>,
    /// When the figure was last read for the screens, ms.
    pub memory_read_at: u64,
    /// One Argon2id pass over 64 MiB, ms, as measured here.
    pub ms_per_unit: Option<u64>,
    /// Where the one-pass measurement is.
    pub(super) measure: Measure,
    /// Work waiting for its frame.
    pub working: Option<Work>,
    /// The seals of a lock under way ([`super::SealRun`]).
    pub(crate) sealing: Option<super::SealRun>,
    /// The vaults the Lock sheet on show names: what a press of its Lock
    /// without saving lets go ([`Vaults::name_lock`]).
    pub(crate) lock_named: Vec<VaultId>,
    /// Vaults whose seal stopped part way, by the version of their records
    /// then (`stuck_of`).
    pub(crate) stuck: Vec<(VaultId, u64, Stuck)>,
    /// A frame has been drawn since the work was asked for.
    pub(crate) drawn: bool,
    /// When the press now held began, ms.
    pub(crate) pressed_at: u64,
    /// The vault just made, by file name, until it is unlocked or the
    /// person leaves the list.
    pub just_made: Option<Text>,
    /// The wallet a vault load left out and why (name, reason), until the
    /// load's toast has told the person.
    pub(crate) refused: Option<(Text, Text)>,
    /// The press being acted on, to be run again when a load it started
    /// has answered ([`crate::vault::load_job`]).
    pub(crate) acting: Option<crate::Action>,
    /// A press is run again after its load: it starts no load again of the
    /// vaults whose seeds were loaded for it (by their places).
    pub(crate) replayed: Vec<usize>,
    /// How many loads of seeds were started: a press that starts one stops
    /// where it is, and says so by this moving.
    pub(crate) loads_started: u32,
    /// The key a load for one key record added, until its press takes it.
    pub(crate) key_added: Option<[u8; 4]>,
    /// Why the last load left a key out, as the person is told.
    pub(crate) key_refused: Option<Text>,
    /// Where an unlock or a creation started from another flow returns.
    pub back_to: Option<Screen>,
    /// Create a vault was opened from Home: Back goes to Home, not the
    /// list.
    pub from_home: bool,
    /// Dice being rolled for a passphrase (its number is the entry's
    /// tag): the rolls, the list and the roller beside them (Faraday
    /// Demo).
    pub dice: crate::dice_entry::DiceEntry,
    /// The prompt in the contents.
    pub prompt: Option<Prompt>,
    /// This computer's clock, Unix seconds, as the shell last said.
    pub unix_secs: Option<u64>,
    /// When the shell last said it, on the app's own ticks, ms.
    pub(crate) clock_at_ms: u64,
    /// A time set by hand (`docs/TOTP.md` §1.2): the Unix seconds the
    /// person set minus the shell's clock at that moment. `None` until
    /// the first Set or Keep this time this power-on.
    pub(crate) clock_offset: Option<i64>,
    /// Set or Keep this time has been pressed this power-on
    /// (`docs/TOTP.md` §1.3). The desktop app is always checked, once its
    /// clock is known.
    pub(crate) clock_checked: bool,
    /// What is remembered of each vault seen open since power-on.
    pub summaries: Vec<VaultSummary>,
    /// An unlock stopped at the older-copy dialog ([`OpenHold`]), boxed:
    /// a summary is large and the hold is rare.
    pub older: Option<Box<super::OpenHold>>,
    /// An idle lock let a vault go with changes that could not be sealed:
    /// the Locked sheet says so.
    pub(crate) unsaved: bool,
    /// An idle lock could not allocate a chosen new memory step and sealed
    /// a vault at the file's own: the Locked sheet says so.
    pub(crate) step_dropped: bool,
    /// Records waiting at the dialog that a vault already holds a related
    /// passphrase or share ([`super::related::SaveHold`]).
    pub save_hold: Option<Box<super::related::SaveHold>>,
    /// The expiry chosen for a GPG key being made or renewed, in years;
    /// 0 for never.
    pub gpg_years: u32,
    /// Choosing Inbox files for the selected GPG key to sign.
    pub signing: bool,
    /// The signable Inbox files ticked to sign, by name and SHA-256: a
    /// file replaced by a stick visit is no longer ticked. Set to its
    /// default (the only file ticked, several none) whenever the page
    /// opens.
    pub sign_ticked: Vec<(Text, [u8; 32])>,
    /// The small text files, by SHA-256, whose text the signing page shows
    /// now: each starts hidden behind its eye, and a lock, the idle
    /// warning and leaving the page hide them again.
    pub sign_shown: Vec<[u8; 32]>,
    /// The fingerprint of the key the signing page was opened for.
    pub sign_key: Option<[u8; 20]>,
    /// The files the signing page hashed, so that it hashes each once.
    pub sign_seen: std::cell::RefCell<Vec<crate::gpg::Seen>>,
    /// The lines the signing page broke each small text file into, kept
    /// with the text and what it was broken from (`docs/FRAMES.md` §7).
    pub sign_wraps: std::cell::RefCell<Vec<crate::gpg::SignWrap>>,
    /// The term chosen to renew the selected GPG key for, in years; none
    /// until the person chooses.
    pub renew_years: Option<u32>,
    /// The reason chosen for a revocation, by GnuPG's menu number.
    pub revoke_reason: u8,
    /// What is typed into passphrase fields is shown in the clear.
    pub typed_shown: bool,
    /// The owner name Secure Boot keys are being made under.
    pub sb_owner: Text,
    /// Enrolment adds the owner's keys alone, not Microsoft's.
    pub sb_own_only: bool,
    /// The Inbox's EFI images are shown, to sign or check.
    pub sb_images: bool,
    /// The image picked to sign, by Inbox position, with its release check
    /// then (F134), for the page; the hold checks the Inbox again.
    pub sb_sign: Option<(usize, crate::secure_boot::Checked)>,
    /// What the last check said.
    pub sb_checked: Option<Text>,
    /// Key fingerprints worked out for the contents screen, by the
    /// SHA-256 of the key's payload and the passphrase stored with it:
    /// deriving one takes 2048 rounds. In memory only.
    /// With each, the key's [`crate::wallet::master_id`], which says it
    /// is a given key where the fingerprint only finds it.
    pub fingerprints: std::cell::RefCell<Vec<KeyIdentity>>,
    /// Key records' account keys at a wallet's path, by the record's id
    /// and the path: public key and chain code, `None` where it could not
    /// be derived. What confirms a record is a wallet slot's key
    /// ([`crate::vault::screen::record_is_policy_key`]). In memory only.
    pub(crate) accounts: std::cell::RefCell<Vec<KeyAccount>>,
    /// The selected key's SeedQR while its eye shows it, by record
    /// index: made at the press that shows it, dropped at the press that
    /// hides it and with [`Vaults::shown`].
    pub(crate) seedqr: Option<(usize, faraday_codec::qr::QrMatrix)>,
    /// The selected key record's words while its eye shows them, by
    /// record index: made at the press that shows them, dropped at the
    /// press that hides them and with [`Vaults::shown`] (F195).
    pub(crate) words: Option<(usize, faraday_keys::Held<faraday_bip::bip39::Mnemonic>)>,
    /// The selected GPG key's paperkey while its eye shows it, held and
    /// dropped as `words` is.
    pub(crate) paperkey: Option<(usize, faraday_keys::Held<faraday_text::Text>)>,
    /// The code of the entry on the page, held while the page is open
    /// and made once for each period ([`CodeHeld`]). It shows with no eye,
    /// so it outlives [`Vaults::shown`]: the frame after the page is left
    /// drops it (`Faraday::vault_code_refresh`), as do an edit and a lock.
    pub(crate) code: Option<CodeHeld>,
    /// The codes screen's rows, held while the screen is on
    /// ([`CodesHeld`]); dropped on leaving it, at a lock, and by every
    /// edit and close.
    pub(crate) codes: Option<CodesHeld>,
    /// The item page's secrets shown by their eyes, and where. Invariant:
    /// `None`, and `seedqr` with it, once the place it names is left
    /// (another screen, vault, kind, record or page of the item, a sheet
    /// over it, a lock); kept so by [`crate::Faraday::vault_shown_tidy`]
    /// after every input, the codes, words and paperkey held for what is
    /// shown dropped with it (the codes by `before_frame`), and drawing
    /// shows a secret only where it was shown.
    pub(crate) shown: Option<Shown>,
    /// Save to a stick's popup, while it is open.
    pub stick_save: Option<crate::vault::out::StickSave>,
    /// The Load sheet an unlock shows, while it is up (F62).
    pub load: Option<crate::vault::load::LoadSheet>,
    /// The sheet of seeds kept with no passphrase, while it is up (F62).
    pub pass_seeds: Option<crate::vault::load::PassSeeds>,
    /// The sheet of vaults an item can be copied to, while it is up.
    pub(super) copy_to: Option<Box<super::copy::CopyTo>>,
    /// An item held for a vault to unlock or make, to go into it then.
    pub(super) copy_wait: Option<Box<super::copy::CopyWait>>,
    /// The save the Keep sheet waits to make.
    pub(super) keep_save: Option<super::keep::KeepSave>,
    /// The dialog before a codex32 set or a BIP-85 origin is dropped from a
    /// seed, while it is up.
    pub(super) trim: Option<super::trim::Trim>,
    /// The dialog saying an item does not fit a QR vault, while it is up.
    pub(super) qr_no: Option<super::qr_save::QrNo>,
    /// Items moved and not yet sealed in their target, as (original,
    /// target): the original is not sealed before the target, or the item
    /// would be in neither new file (`docs/VAULT.md` §7.2).
    moved: Vec<(VaultId, VaultId)>,
    /// The backup subject (a key's identity, as a flow knows it) of each
    /// moved record that is one, as (target, subject): a flow dropping a
    /// place spares that record in the target, which holds it for no one
    /// else.
    moved_subjects: Vec<(VaultId, Text)>,
    /// What the search bars hold ([`super::search`]).
    pub(super) search: super::search::Searches,
}

/// Where the item page's secrets were shown, on Vault contents: the open vault,
/// the kind and the record, with the item's page itself on screen (no
/// form, prompt, save, signing or images panel over it, and no sheet).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShownAt {
    pub(crate) vault: VaultId,
    pub(crate) category: usize,
    pub(crate) record: usize,
    pub(crate) item_open: bool,
}

impl ShownAt {
    /// The same record of the same vault and kind, the item's page open
    /// or not (it shows beside the list on a wide panel).
    pub(crate) fn same_record(self, other: ShownAt) -> bool {
        Self {
            item_open: other.item_open,
            ..self
        } == other
    }
}

/// What the entry on the page gives for its TOTP secret, worked out
/// outside drawing (`Faraday::vault_code_refresh`) and read by it: the
/// result at `counter`'s period, and where. The seconds left are not
/// held: drawing works them out from the clock.
pub(crate) struct CodeHeld {
    /// The page it was made for.
    pub(crate) at: ShownAt,
    /// The period it was made in, as seconds on the clock over the
    /// period's length; `None` with no checked clock.
    pub(crate) counter: Option<u64>,
    /// The period's length in seconds; 0 where there is no code.
    pub(crate) period: u64,
    /// A code (wiped as it drops), Set the time, HOTP or Unreadable.
    pub(crate) result: Result<Option<faraday_keys::totp::Shown>, faraday_keys::totp::NotTotp>,
}

impl CodeHeld {
    /// Whether the code made is still the one for `now` on `at`: the
    /// same entry, its page open or not.
    pub(crate) fn current(&self, at: ShownAt, now: Option<u64>) -> bool {
        self.at.same_record(at)
            && match (&self.result, now) {
                (Err(_), _) => true,
                (Ok(None), None) => true,
                (Ok(Some(_)), Some(n)) => self.period > 0 && self.counter == Some(n / self.period),
                _ => false,
            }
    }
}

/// What one listed entry of the codes screen gives for its TOTP secret,
/// worked out outside drawing (`Faraday::vault_codes_refresh`) and read
/// by it, as [`CodeHeld`] holds the entry page's. The seconds left are
/// not held: drawing works them out from the clock.
pub(crate) struct ListedCode {
    pub(crate) vault: VaultId,
    pub(crate) record: usize,
    /// The period it was made in, as seconds on the clock over the
    /// period's length; `None` with no clock.
    pub(crate) counter: Option<u64>,
    /// The period's length in seconds; 0 where there is no code.
    pub(crate) period: u64,
    /// The account its URI's label names, public.
    pub(crate) account: Option<faraday_text::Text>,
    /// `None` for a record with no row (not an entry); else a code
    /// (wiped as it drops), Set the time, HOTP or Unreadable.
    pub(crate) result:
        Option<Result<Option<faraday_keys::totp::Shown>, faraday_keys::totp::NotTotp>>,
}

impl ListedCode {
    /// Whether what was made is still what `now` gives.
    pub(crate) fn current(&self, now: Option<u64>) -> bool {
        match (&self.result, now) {
            (None | Some(Err(_)), _) => true,
            (Some(Ok(None)), None) => true,
            (Some(Ok(Some(_))), Some(n)) => {
                self.period > 0 && self.counter == Some(n / self.period)
            }
            _ => false,
        }
    }
}

/// The codes screen's rows as held: one [`ListedCode`] for each entry
/// listed, wiped as it drops.
#[derive(Default)]
pub(crate) struct CodesHeld(pub(crate) Vec<ListedCode>);

impl CodesHeld {
    /// The held result of record `record` of vault `vault`.
    pub(crate) fn of(&self, vault: VaultId, record: usize) -> Option<&ListedCode> {
        self.0
            .iter()
            .find(|l| l.vault == vault && l.record == record)
    }
}

/// The item page's secrets shown by their eyes ([`Vaults::shown`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shown {
    /// Where they were shown.
    pub(crate) at: ShownAt,
    /// Which secret fields of the record, a bit each by their order on
    /// the page.
    pub(crate) fields: u64,
}

/// `parts` with `sep` between them.
pub(crate) fn join_text(parts: &[Text], sep: &str) -> Text {
    let mut out = Text::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        out.push_str(part);
    }
    out
}

/// Memory in MiB as a person reads it.
pub fn mib_text(mib: u32) -> Text {
    if mib >= 1024 {
        if mib.is_multiple_of(1024) {
            Text::fill(EN.vaults_gib, &[&Text::num(u64::from(mib / 1024))])
        } else {
            Text::fill(EN.vaults_gib, &[&text!("{:.1}", f64::from(mib) / 1024.0)])
        }
    } else {
        Text::fill(EN.vaults_mib, &[&Text::num(u64::from(mib))])
    }
}

/// A file size as a person reads it: [`crate::screens::size_text`].
pub fn file_text(len: usize) -> Text {
    Text::of(&crate::screens::size_text(len as u64))
}

/// A cost as a person reads it.
pub fn cost_text(cost: &Cost) -> Text {
    text!(
        "{} · {} {}",
        &*mib_text(cost.memory_kib / 1024),
        cost.passes,
        if cost.passes == 1 {
            EN.vaults_pass
        } else {
            EN.vaults_passes
        }
    )
}

impl Vaults {
    /// Closes open vault `i`: its slot key and records are wiped with it.
    /// The pairs it is in are met and go, with what is recorded of the
    /// moved records. Closed unsealed, a target would let go an item held
    /// only in memory, and an original would lose the last file that holds
    /// the item: so no caller closes an unsealed vault that a pair names,
    /// as target or as original (the seal, which closes after its file is
    /// written and after its targets'; the flow's `close_at` of a dropped
    /// place and of `flow_bind_ids`, which skip such a vault or need no
    /// change in it; `seal_close` of a vault with no change in a lock of
    /// one; a hold's Keep locked, a vault with no pair yet). A person's
    /// Lock without sealing closes every vault at once (`close_all`),
    /// originals with their targets.
    pub fn close(&mut self, i: usize) {
        let gone = self.open.remove(i);
        self.stuck.retain(|(id, _, _)| *id != gone.id);
        self.moved.retain(|&(o, t)| o != gone.id && t != gone.id);
        // What is recorded of a moved record goes with its last pair.
        let moved = &self.moved;
        self.moved_subjects
            .retain(|(t, _)| moved.iter().any(|p| p.1 == *t));
        self.store.close(gone.id);
        self.codes = None;
    }

    /// An item moved from `from` into `to` waits for `to`'s seal.
    /// `subject` is the record's identity for a backup flow, if it is one.
    pub(crate) fn moved_add(&mut self, from: VaultId, to: VaultId, subject: Option<Text>) {
        self.moved.push((from, to));
        if let Some(s) = subject {
            self.moved_subjects.push((to, s));
        }
    }

    /// The record of backup subject `subject` left vault `id` (deleted, or
    /// moved on): it is no longer one moved there.
    pub(crate) fn moved_forget(&mut self, id: VaultId, subject: Option<Text>) {
        if let Some(s) = subject {
            self.moved_subjects.retain(|(t, x)| !(*t == id && *x == s));
        }
    }

    /// The vaults `id` must be sealed after: the targets of the items
    /// moved out of it and not yet sealed in them.
    pub(crate) fn moved_waits_for(&self, id: VaultId) -> Vec<VaultId> {
        self.moved
            .iter()
            .filter(|p| p.0 == id)
            .map(|p| p.1)
            .collect()
    }

    /// Whether the record moved into vault `id` and waiting for its seal
    /// is backup subject `subject`.
    pub(crate) fn moved_subject_in(&self, id: VaultId, subject: &str) -> bool {
        self.moved_subjects
            .iter()
            .any(|(t, s)| *t == id && s == subject)
    }

    /// Whether any item was moved into `id` and waits for its seal.
    pub(crate) fn moved_target(&self, id: VaultId) -> bool {
        self.moved.iter().any(|p| p.1 == id)
    }

    /// Whether moving an item from `from` into `to` would make a ring of
    /// vaults each waiting for the next (no order seals them all safely):
    /// whether `from` is reached from `to` by following every pair.
    pub(crate) fn moved_ring(&self, from: VaultId, to: VaultId) -> bool {
        let mut seen = vec![to];
        let mut at = 0;
        while let Some(&id) = seen.get(at) {
            if id == from {
                return true;
            }
            for next in self.moved_waits_for(id) {
                if !seen.contains(&next) {
                    seen.push(next);
                }
            }
            at += 1;
        }
        false
    }

    /// `ids` with each vault after the ones it waits for.
    pub(crate) fn moved_order(&self, ids: Vec<VaultId>) -> VecDeque<VaultId> {
        fn visit(
            v: &Vaults,
            id: VaultId,
            all: &[VaultId],
            done: &mut Vec<VaultId>,
            out: &mut VecDeque<VaultId>,
        ) {
            if done.contains(&id) {
                return;
            }
            done.push(id);
            for &(o, t) in &v.moved {
                if o == id && all.contains(&t) {
                    visit(v, t, all, done, out);
                }
            }
            out.push_back(id);
        }
        let mut out = VecDeque::new();
        let mut done = Vec::new();
        for &id in &ids {
            visit(self, id, &ids, &mut done, &mut out);
        }
        out
    }

    /// Drops the slot an unlock adopted and refused, its key and records
    /// wiped with it.
    pub(super) fn drop_adopted(&mut self, id: VaultId) {
        self.store.close(id);
    }

    /// Every secret shown and what was held of it dropped, each wiped:
    /// the SeedQR, a key's words, a paperkey and the entry's code.
    pub(crate) fn drop_shown(&mut self) {
        self.shown = None;
        self.seedqr = None;
        self.words = None;
        self.paperkey = None;
        self.code = None;
    }

    /// At a lock: the signing page's kept texts (the files' text and the
    /// lines broken from it) are wiped and dropped.
    pub(crate) fn drop_sign_text(&self) {
        crate::gpg::SignWrap::drop_all(&self.sign_wraps);
        crate::gpg::Seen::drop_all(&self.sign_seen);
    }

    #[cfg(any(test, feature = "testkit"))]
    /// How many files' texts the signing page keeps: the lines broken from
    /// each small text file and the files it remembers.
    pub(crate) fn sign_text_kept(&self) -> usize {
        crate::gpg::SignWrap::kept_count(&self.sign_wraps) + self.sign_seen.borrow().len()
    }

    /// Closes every open vault, at a lock what is shown with them.
    pub fn close_all(&mut self) {
        self.drop_shown();
        self.sign_shown.clear();
        self.codes = None;
        self.moved.clear();
        self.moved_subjects.clear();
        self.open.clear();
        self.stuck.clear();
        self.store.clear();
        self.search = super::search::Searches::default();
    }

    /// Seconds to unlock at a cost on this computer, when measured.
    pub fn seconds(&self, memory_mib: u32, passes: u32) -> Option<f64> {
        self.ms_per_unit
            .map(|ms| ms as f64 * f64::from(memory_mib) / 64.0 * f64::from(passes) / 1000.0)
    }

    /// "about 1.8 s", or that it is not measured yet.
    pub fn time_text(&self, memory_mib: u32, passes: u32) -> Text {
        match self.seconds(memory_mib, passes) {
            Some(s) if s < 10.0 => Text::fill(EN.vaults_about_s, &[&text!("{s:.1}")]),
            Some(s) => Text::fill(EN.vaults_about_s, &[&Text::num(s.round() as u64)]),
            None => Text::of(EN.vaults_not_measured),
        }
    }

    /// Create's cost: memory MiB and passes.
    pub fn form_cost(&self) -> Option<(usize, u32, u32)> {
        let c = self.create.as_ref()?;
        let k = c.preset.unwrap_or_else(|| self.suggested());
        Some(match k {
            CUSTOM => (CUSTOM, c.memory, c.passes),
            k => (k, PRESETS[k].1, PRESETS[k].2),
        })
    }

    /// Whether typing at `focus` is a secret: a passphrase, a
    /// passphrase's dice, the prompt's passphrase, or an entry's or a
    /// note's field. A vault's name, a label and a search are not.
    pub(crate) fn focus_is_secret(&self, focus: Focus) -> bool {
        match focus {
            Focus::Passphrase
            | Focus::YubiKey
            | Focus::Phrase(..)
            | Focus::Dice
            | Focus::Prompt
            | Focus::PromptAgain => true,
            Focus::Field(_) => self.form.as_ref().is_some_and(Form::holds_secrets),
            Focus::Rolls | Focus::Search(_) | Focus::Name => false,
        }
    }

    /// Why this computer does not unlock a vault of `memory_mib` unlock
    /// memory and a file of `len` bytes, by what it has free; `None` when it
    /// does. Not knowing what it has free, it unlocks only Light's, which
    /// opens anywhere (`docs/VAULT.md` §3.1). Noted ([`work::refused`]).
    pub(crate) fn unlock_memory_refusal(&self, cost: Cost, len: usize) -> Option<Text> {
        let refusal = self.unlock_refusal(cost, len);
        work::refused(
            VaultAct::Unlocked,
            cost.memory_kib / 1024,
            cost.passes,
            refusal,
        )
    }

    /// [`Vaults::unlock_memory_refusal`] without the note: a lookup, for
    /// drawing and for what a step is allowed.
    pub(crate) fn unlock_refusal(&self, cost: Cost, len: usize) -> Option<Text> {
        self.open_refusal(cost.memory_kib / 1024, len)
    }

    /// The same test for any file of `len` bytes whose key takes
    /// `memory_mib` MiB to work out, a QR vault code's included. A lookup.
    pub(crate) fn open_refusal(&self, memory_mib: u32, len: usize) -> Option<Text> {
        let need = vault_need_mib(memory_mib, len);
        match self.memory_free_mib {
            Some(free) if need > free => Some(Text::fill(
                EN.vaults_needs_this_computer,
                &[&mib_text(need), &mib_text(free)],
            )),
            None if memory_mib > LIGHT_MIB => Some(Text::fill(
                EN.vaults_memory_unknown_unlock,
                &[&mib_text(LIGHT_MIB), &mib_text(memory_mib)],
            )),
            _ => None,
        }
    }

    /// Why open vault `id`'s next seal may not use the memory step
    /// `kib`: this computer could not unlock the file it makes, by the
    /// test Unlock applies ([`Vaults::unlock_memory_refusal`]). A lookup.
    pub(crate) fn step_refusal(&self, id: VaultId, kib: u32) -> Option<Text> {
        let store = self.store();
        let facts = store.facts(id)?;
        let mut cost = store.next_cost(id)?;
        cost.memory_kib = kib;
        self.unlock_refusal(cost, facts.len)
    }

    /// Costs a walk of the records changed and no more where the bound's
    /// figure fits what is free or Argon2id's memory alone is over it; else
    /// a copy of the document and a count of it, with a read of the free
    /// memory ([`OpenVaults::held_exact`]). Why open vault `id` cannot be sealed
    /// here: this computer does not have free what [`Need::Seal`] says its
    /// next seal takes, at the document and attachments it opened with, what
    /// was added since counted, and the Argon2id memory the seal uses. `None`
    /// when it does, and when this computer does not say what it has free:
    /// the vault opened here, and a figure it does not give is no ground to
    /// keep a vault from being saved. The bound only says "certainly fits":
    /// where the exact count cannot be made because the memory free then does
    /// not hold the copy, the bound's figure is the one shown, and the seal
    /// is refused, never allowed.
    pub(crate) fn seal_refusal(&self, c: &Compute, id: VaultId) -> Option<Text> {
        let free = self.memory_free_mib?;
        let store = self.store();
        let cost = store.next_cost(id)?;
        let need_of = |(document, attachments): (usize, usize)| {
            need_mib(Need::Seal {
                memory_mib: cost.memory_kib / 1024,
                document,
                attachments,
            })
        };
        let held = store.held(id)?;
        let mut need = need_of(held);
        // The exact count copies the document: it is made only where what
        // is free also holds that copy, so that a computer short of memory
        // is not asked for more to find out. Short at the second reading
        // keeps the bound's figure, and the refusal.
        // Nor where the Argon2id memory and the app alone are more than is
        // free: no count of the document can change that answer.
        if need > free
            && need_of((0, 0)) <= free
            && copy_need_mib(held.0) <= free
            && let Some(exact) = store.held_exact(c, id)
        {
            need = need_of(exact);
        }
        (need > free).then(|| {
            Text::fill(
                EN.vaults_needs_this_computer,
                &[&mib_text(need), &mib_text(free)],
            )
        })
    }

    /// Why a file of `size` bytes is not read in, by what this computer has
    /// free ([`Need::Read`]); `None` for a file that is not a vault's, and
    /// when it fits. Not knowing what it has free, a vault file of over 4
    /// MiB is not read (`docs/VAULT.md` §3.1). A lookup.
    pub fn read_refusal(&self, is_vault: bool, size: u64) -> Option<Text> {
        const UNKNOWN_FREE_MAX: u64 = 4 * 1024 * 1024;
        if !is_vault {
            return None;
        }
        let need = need_mib(Need::Read {
            len: usize::try_from(size).unwrap_or(usize::MAX),
        });
        match self.memory_free_mib {
            Some(free) if need > free => Some(Text::fill(
                EN.vaults_needs_this_computer,
                &[&mib_text(need), &mib_text(free)],
            )),
            None if size > UNKNOWN_FREE_MAX => Some(Text::of(EN.visit_file_memory_unknown)),
            _ => None,
        }
    }

    /// Why the Create form's cost is not made here, by what this computer
    /// has free; `None` when it is. Not knowing what it has free, it makes
    /// only Light's (`docs/VAULT.md` §3.1). Noted ([`work::refused`]).
    pub(crate) fn create_memory_refusal(&self) -> Option<Text> {
        let (_, mem, passes) = self.form_cost()?;
        let refusal = match self.memory_free_mib {
            Some(free) if vault_need_mib(mem, new_file_len()) > free => {
                Some(Text::of(EN.vaults_choose_a_cost))
            }
            None if mem > LIGHT_MIB => Some(Text::fill(
                EN.vaults_memory_unknown_create,
                &[&mib_text(LIGHT_MIB)],
            )),
            _ => None,
        };
        work::refused(VaultAct::Made, mem, passes, refusal)
    }

    /// The vault a stretch job opened joins the store (no Argon2id run).
    pub(super) fn adopt(
        &mut self,
        stretched: faraday_keys::job::Stretched,
    ) -> Result<(VaultId, fv::file::Facts), fv::file::Error> {
        self.store.adopt(stretched)
    }

    /// The most unlock memory this computer gives a vault, MiB: the
    /// largest custom memory that fits in what it has free beside twice
    /// the file and the app ([`vault_need_mib`]), at most 4 GiB, which is
    /// as much as a vault needs; Light's, which opens anywhere, when it
    /// does not say (`docs/VAULT.md` §3.1).
    pub fn here_limit(&self) -> u32 {
        self.here_limit_for(new_file_len())
    }

    /// What this computer gives a vault whose file is `len` bytes.
    pub fn here_limit_for(&self, len: usize) -> u32 {
        let Some(free) = self.memory_free_mib else {
            return LIGHT_MIB;
        };
        CUSTOM_MEMORY
            .iter()
            .copied()
            .filter(|m| *m <= MAX_UNLOCK_MIB && vault_need_mib(*m, len) <= free)
            .max()
            .unwrap_or(CUSTOM_MEMORY[0])
    }

    /// The machine that gives a vault the least unlock memory among
    /// those chosen, label and MiB: this computer what it has free (see
    /// `here_limit`), any other half of its memory. With none chosen,
    /// this computer.
    pub fn weakest(&self) -> (Text, u32) {
        match self.create.as_ref() {
            Some(c) => Self::weakest_of(self.here_limit(), c.not_here, &c.on),
            None => Self::weakest_of(self.here_limit(), false, &[false; 5]),
        }
    }

    /// The weakest of the machines ticked, as Create states it: `here` is
    /// what this computer gives, `not_here` drops it, `on` are the
    /// machines in `MACHINES` order. With none ticked, this computer.
    pub fn weakest_of(here: u32, not_here: bool, on: &[bool; 5]) -> (Text, u32) {
        let here = (Text::of(EN.vaults_this_computer_name), here);
        let mut w: Option<(Text, u32)> = (!not_here).then_some(here.clone());
        for (k, m) in MACHINES.iter().enumerate() {
            if on[k] && w.as_ref().is_none_or(|w| m.3 / 2 < w.1) {
                w = Some((Text::of(m.0), m.3 / 2));
            }
        }
        w.unwrap_or(here)
    }

    /// What this computer gives open vault `id`: by its own size, never
    /// the Create form's.
    pub fn memory_here_limit(&self, id: VaultId) -> u32 {
        let len = self.store().facts(id).map_or_else(new_file_len, |f| f.len);
        self.here_limit_for(len)
    }

    /// The Memory dialog's weakest machine over its own ticks, for open
    /// vault `id`.
    pub fn memory_weakest(&self, id: VaultId) -> (Text, u32) {
        let t = &self.memory_ticks;
        Self::weakest_of(self.memory_here_limit(id), t.not_here, &t.on)
    }

    /// The Memory dialog's ticks: `(not_here, on)`, `on` in `MACHINES`
    /// order. Not Create's, and not kept between openings.
    pub fn memory_ticks(&self) -> (bool, [bool; 5]) {
        (self.memory_ticks.not_here, self.memory_ticks.on)
    }

    /// Tick or untick this computer in the Memory dialog.
    pub(crate) fn memory_toggle_here(&mut self) {
        self.memory_ticks.not_here = !self.memory_ticks.not_here;
    }

    /// Tick or untick machine `k` in the Memory dialog; out of range does
    /// nothing.
    pub(crate) fn memory_toggle(&mut self, k: usize) {
        if let Some(t) = self.memory_ticks.on.get_mut(k) {
            *t = !*t;
        }
    }

    /// The Memory dialog opens with this computer alone.
    pub(crate) fn memory_ticks_reset(&mut self) {
        self.memory_ticks = MemoryTicks::default();
    }

    /// The largest preset the weakest machine gives the memory for.
    pub fn suggested(&self) -> usize {
        let (_, limit) = self.weakest();
        (0..PRESETS.len())
            .rev()
            .find(|&k| PRESETS[k].1 <= limit)
            .unwrap_or(0)
    }

    /// The size row chosen, while the preset and machines are one of
    /// `SIZES`.
    pub fn size_row(&self) -> Option<usize> {
        let c = self.create.as_ref()?;
        SIZES
            .iter()
            .position(|(_, p, on)| c.preset == Some(*p) && c.on == *on && !c.not_here)
    }

    /// Whether a passphrase pair is typed the same twice, and not empty.
    pub fn phrase_ok(c: &CreateForm, i: usize) -> bool {
        let (a, b) = &c.phrases[i];
        crate::screens::pass_pair::ready(a.text.as_str(), b.text.as_str())
    }

    /// Whether a passphrase pair differs: Create waits while one does.
    pub fn phrases_differ(c: &CreateForm) -> bool {
        c.phrases
            .iter()
            .any(|(a, b)| crate::screens::pass_pair::differ(a.text.as_str(), b.text.as_str()))
    }

    /// Passphrase `i`'s own bits, known only while it is the words the
    /// dice made: a typed passphrase's strength is not measured, nor is
    /// Practice's `faraday`, which stands in for the rolled words.
    pub fn phrase_bits(c: &CreateForm, i: usize) -> Option<f32> {
        let (made, bits, _) = c.from_dice.get(i)?.as_ref()?;
        let (a, b) = c.phrases.get(i)?;
        let same = made.as_str() == a.text.as_str() && made.as_str() == b.text.as_str();
        (same && made.as_str() != crate::demo::PASSPHRASE).then_some(*bits)
    }
}

/// The wallet name record `r` keeps in field `number`, as one line of
/// text ([`crate::wallet::wallet_name`]); `or` where it keeps none.
pub(crate) fn wallet_name_in<'a>(
    r: RecordView<'a>,
    number: u8,
    or: &'a str,
) -> std::borrow::Cow<'a, str> {
    crate::wallet::wallet_name(r.public_text(number).unwrap_or(or))
}

/// Longest name a vault's file takes, before `.kdbx`.
pub const NAME_MAX: usize = 40;

/// Longest name written in a vault's document, in bytes: what the person
/// typed, cut at a character boundary (`docs/VAULT.md` §7).
pub const LABEL_MAX: usize = 64;

/// The file name, less `.kdbx`, a vault named `typed` is written under:
/// letters, digits, hyphens and underscores, spaces made hyphens,
/// anything else left out; `vault` when that leaves nothing, and a
/// Windows device name (`con`, `com1`) with `-vault` after it.
pub fn vault_stem(typed: &str) -> Text {
    /// Whether Windows takes a name for a device, in any case.
    fn windows_device(name: &str) -> bool {
        let n = name.as_bytes();
        let numbered = |prefix: &[u8]| {
            n.len() == 4 && n[..3].eq_ignore_ascii_case(prefix) && matches!(n[3], b'1'..=b'9')
        };
        ["con", "prn", "aux", "nul"]
            .iter()
            .any(|d| d.eq_ignore_ascii_case(name))
            || numbered(b"com")
            || numbered(b"lpt")
    }

    let mut out = Text::new();
    for c in typed.trim().chars() {
        if out.chars().count() >= NAME_MAX {
            break;
        }
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c.is_whitespace() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        Text::of("vault")
    } else if windows_device(out) {
        // A name Windows keeps for a device (`con`, `aux`, `com1`) cannot
        // be a file's.
        text!("{out}-vault")
    } else {
        Text::of(out)
    }
}

/// Whether a form field takes several lines, Enter starting a new one: a
/// note's text and an entry's notes.
pub fn multiline(kind: u8, number: u8) -> bool {
    matches!(
        (kind, number),
        (kind::NOTE, field::NOTE) | (kind::ENTRY, field::NOTES)
    )
}

/// What the lock, write-out and power-off sheets list (§4.5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeptWiped {
    /// Kept: one line per vault with its currency, then the files For
    /// the stick.
    pub kept: Vec<Text>,
    /// Wiped from memory, or lost at power-off: one line each, `true` for
    /// what is nowhere else, and for a key or wallet in no open vault
    /// while one is open, the Save that puts it in that vault.
    pub wiped: Vec<(Text, bool, Option<Action>)>,
}

/// A vault file's currency (`docs/history/SIMPLIFY.md` §3.5), computed from the
/// receipt's hashes, never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Currency {
    /// Sealed For the stick, and no receipt names it.
    NeverWritten,
    /// The last receipt that names it wrote the bytes it has now: on the
    /// stick of this label.
    Current(Text),
    /// Open with unsaved changes, or sealed bytes that differ from the
    /// receipt's.
    Changed,
    /// Read from a stick and not changed since, the stick not known.
    Unchanged,
}

impl Currency {
    /// The line a person reads: "Never written", "On STICK · current",
    /// "Changed since written", "Unchanged".
    pub fn line(&self) -> Text {
        match self {
            Currency::NeverWritten => Text::of(EN.vaults_never_written),
            Currency::Current(label) => Text::fill(EN.vaults_on_current, &[label]),
            Currency::Changed => Text::of(EN.vaults_changed_since),
            Currency::Unchanged => Text::of(EN.vaults_unchanged),
        }
    }
}

/// What the app remembers of a vault seen open, across a lock
/// (`docs/VAULT.md` §11): names, fingerprints and master ids, never
/// anything that spends. Kept in the kept state, so it is gone at power-off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VaultSummary {
    /// The file it was opened from.
    pub file: Text,
    /// The vault's identifier: its root group's UUID in hex
    /// (`docs/VAULT.md` §7). A file has no identifier Files can read, so
    /// a file is linked to a vault by [`VaultSummary::copies`].
    pub id: [u8; 32],
    /// The SHA-256 of every copy of the vault the session has seen: the
    /// file opened and each seal (`docs/VAULT.md` §6). A file on a stick
    /// is a copy of the vault when its SHA-256 is among them.
    pub copies: Vec<[u8; 32]>,
    /// The open slot's name.
    pub name: Text,
    /// Its wallets: name and shape.
    pub wallets: Vec<(Text, Text)>,
    /// Its wallets' descriptor checksums, in the same order.
    pub sums: Vec<Text>,
    /// The descriptor checksums of the wallets it keeps a plan for.
    pub planned: Vec<Text>,
    /// Its keys' fingerprints.
    pub keys: Vec<Text>,
    /// Its keys' [`crate::wallet::master_id`]s, in the same order: what
    /// says a seed is one of its keys while it is locked, where the
    /// fingerprint only finds a candidate (F65). `None` for a key a
    /// summary kept before the ids were.
    pub ids: Vec<Option<[u8; 32]>>,
    /// How many entries it holds.
    pub entries: usize,
    /// Its backup maps' lines: where each place is and what it holds
    /// (record type 11, field 4).
    pub map: Vec<Text>,
    /// When it was last seen open, Unix seconds, when the clock is known.
    pub seen: Option<u64>,
    /// The highest seal count seen of it since power-on
    /// (`docs/VAULT.md` §11).
    pub seal: Option<SealSeen>,
}

impl VaultSummary {
    /// The line a locked vault's row and Home's Unlock lead read under
    /// its name: "Savings 2 of 3 · key 9a6a2580 · 12 entries · seen
    /// 14:02". `tz_min` is the device's time zone (`docs/TOTP.md` §2.3).
    pub fn line(&self, tz_min: i16) -> Text {
        let mut parts: Vec<Text> = Vec::new();
        // One or two wallets by name and the shape's first part ("2 of
        // 3", "Single key"); more, by count.
        if self.wallets.len() <= 2 {
            for (name, shape) in &self.wallets {
                let short = shape.split(" · ").next().unwrap_or(shape);
                parts.push(text!("{} {short}", &**name));
            }
        } else {
            parts.push(Text::fill(
                EN.vaults_wallets_n,
                &[&Text::num(self.wallets.len() as u64)],
            ));
        }
        // The same for keys: one or two by fingerprint, more by count.
        match self.keys.len() {
            0 => {}
            1 => parts.push(Text::fill(EN.vaults_key, &[&self.keys[0]])),
            2 => parts.push(Text::fill(
                EN.vaults_keys_inline,
                &[&join_text(&self.keys, ", ")],
            )),
            n => parts.push(Text::fill(EN.vaults_keys, &[&Text::num(n as u64)])),
        }
        match self.entries {
            0 => {}
            1 => parts.push(Text::of(EN.vaults_1_entry)),
            n => parts.push(Text::fill(EN.vaults_entries_n, &[&Text::num(n as u64)])),
        }
        if let Some(t) = self.seen {
            parts.push(Text::fill(
                EN.vaults_seen,
                &[&time_of_day(local_secs(t, tz_min))],
            ));
        }
        if parts.is_empty() {
            return Text::of(EN.vaults_empty_line);
        }
        join_text(&parts, " · ")
    }
}

/// A time of day from local Unix-style seconds ([`local_secs`]), hh:mm.
pub fn time_of_day(unix_secs: u64) -> Text {
    let s = unix_secs % 86_400;
    text!("{:02}:{:02}", s / 3600, (s / 60) % 60)
}

/// [`time_of_day`], with seconds: hh:mm:ss, for Settings' and Set the
/// time's Now row.
pub fn time_of_day_secs(unix_secs: u64) -> Text {
    let s = unix_secs % 86_400;
    text!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

/// A record type 11 map line (field 4) with a leading place name
/// replaced by "Place n": a vault written before place names moved out
/// of this field named the place there; `names` are the same record's
/// field 3 values, in place order. A line already naming the place by
/// number, or one for a vault, files, software or an away seed, is
/// returned unchanged.
pub(crate) fn redact_place_name(line: &str, names: &[Text]) -> Text {
    for (p, name) in names.iter().enumerate() {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(name).and_then(|r| r.strip_prefix(':')) {
            return Text::fill(EN.vaults_place, &[&Text::num(p as u64 + 1), rest]);
        }
    }
    Text::of(line)
}

/// A clock reading below this is not known: the Pi has no real-time
/// clock and starts at 1970 (`docs/TOTP.md` §1.1), the same floor
/// `faraday/shells/stick/src/files.rs` uses for file dates.
pub const CLOCK_FLOOR: u64 = 1_577_836_800;

/// Unix seconds turned into local seconds for a time zone offset east of
/// UTC, in minutes (`docs/TOTP.md` §2.3). Every date or time drawn on
/// screen or written for the person goes through this; what goes into a
/// format (a signature's time, the TOTP counter) stays UTC.
pub fn local_secs(unix_secs: u64, tz_min: i16) -> u64 {
    (unix_secs as i64 + i64::from(tz_min) * 60).max(0) as u64
}

/// A time zone offset, minutes east of UTC, as `+02:00`, `-03:30` or
/// `+00:00` (`docs/TOTP.md` §2.1 and §2.2): the sign always written, two
/// digits each of hours and minutes.
pub fn tz_offset_text(tz_min: i16) -> Text {
    let sign = if tz_min < 0 { '-' } else { '+' };
    let abs = tz_min.unsigned_abs();
    text!("{sign}{:02}:{:02}", abs / 60, abs % 60)
}

/// `s` as a time zone offset `tz_offset_text` writes: the sign, two
/// digits of hours, `:`, two digits of minutes a multiple of 15, in
/// range −12:00 to +14:00.
pub fn parse_tz_offset(s: &str) -> Option<i16> {
    let b = s.as_bytes();
    if b.len() != 6 || b[3] != b':' {
        return None;
    }
    let sign: i16 = match b[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hh: i16 = s.get(1..3)?.parse().ok()?;
    let mm: i16 = s.get(4..6)?.parse().ok()?;
    if !matches!(mm, 0 | 15 | 30 | 45) {
        return None;
    }
    let total = sign * (hh * 60 + mm);
    (-720..=840).contains(&total).then_some(total)
}

/// Light's unlock memory, MiB: the cost that opens anywhere, and the most
/// a computer that does not say what memory it has free unlocks or makes
/// (`docs/VAULT.md` §3.1).
pub const LIGHT_MIB: u32 = PRESETS[0].1;

/// The presets of `docs/VAULT.md` §3.1: name, memory in MiB, passes, in
/// order of memory.
pub const PRESETS: [(&str, u32, u32); 5] = [
    (EN.vaults_light, 64, 3),
    (EN.vaults_standard, 256, 3),
    (EN.vaults_medium, 512, 3),
    (EN.vaults_strong, 1024, 4),
    (EN.vaults_maximum, 2048, 2),
];

/// The place after the presets that stands for a custom cost.
pub const CUSTOM: usize = PRESETS.len();

/// Create a vault's two sizes (`docs/history/SIMPLIFY.md` §3.1): the label, the
/// preset it takes, and the machines it is for besides this one, in
/// `MACHINES` order. The second is the default.
pub const SIZES: [(&str, usize, [bool; 5]); 2] = [
    (EN.vaults_pcs_only, 2, [true, false, false, false, false]),
    (
        EN.vaults_pcs_and_a_raspberry,
        1,
        [true, false, false, true, false],
    ),
];

/// The size a new vault starts on: PCs and a Raspberry Pi.
pub const DEFAULT_SIZE: usize = 1;

/// The length of a new vault's file: one step of plaintext (`docs/VAULT.md`
/// §2).
pub fn new_file_len() -> usize {
    fv::file::file_len(fv::file::STEP)
}

/// The machines a vault may be opened on, besides this one: label, short
/// name, memory shown, memory in MiB.
pub const MACHINES: [(&str, &str, &str, u32); 5] = [
    (
        EN.vaults_other_pcs,
        EN.vaults_other_pcs_inline,
        "4 GB",
        4096,
    ),
    (EN.vaults_raspberry_pi_5, EN.vaults_pi_5, "4 GB", 4096),
    (EN.vaults_raspberry_pi_4, EN.vaults_pi_4, "2 GB", 2048),
    (EN.vaults_raspberry_pi_3, EN.vaults_pi_3, "1 GB", 1024),
    (EN.vaults_pi_zero_2_w, EN.vaults_pi_zero_2_w, "512 MB", 512),
];

/// The most unlock memory This computer is suggested for, MiB: more
/// buys a vault little.
pub const MAX_UNLOCK_MIB: u32 = 4096;
/// What the app itself takes beside a vault's unlock memory and file,
/// MiB: a slot's plaintext and the app.
pub const APP_MIB: u32 = 100;

/// What a vault file needs of the memory a computer has free at each step
/// that takes memory in proportion to the file (F206 point 4).
#[derive(Clone, Copy, Debug)]
pub enum Need {
    /// Reading a file in: the file and a copy beside it.
    Read {
        /// The file's length in bytes.
        len: usize,
    },
    /// Unlocking a file.
    Unlock {
        /// The Argon2id memory, MiB.
        memory_mib: u32,
        /// The file's length in bytes.
        len: usize,
    },
    /// Inflating a compressed body once it is decrypted: the buffer and the
    /// file beside it.
    Inflate {
        /// The plaintext length the body states, in bytes.
        plain: usize,
    },
    /// Parsing the XML document of an inflated body: the document and the
    /// attachments beside it.
    Parse {
        /// The XML document's length, in bytes.
        document: usize,
        /// The attachments' length together, in bytes.
        attachments: usize,
    },
    /// Sealing a vault.
    Seal {
        /// The Argon2id memory of the seal, MiB.
        memory_mib: u32,
        /// The XML document's length when the vault opened, in bytes.
        document: usize,
        /// The attachments' length together when the vault opened, in bytes.
        attachments: usize,
    },
}

/// How many times the document a seal takes beyond what the opened vault
/// holds at rest, the free figure being read with the vault open: a seal's
/// peak is 28.7 times a 240 MiB document of real entries (the opened vault,
/// the text written and the document read back from it; the copy with the
/// changes is dropped before that), and the opened vault holds 13.7 of
/// those at rest (a 100 MiB document of real entries: 1,367 MiB resident
/// after the open, 2,813 MiB at the seal's peak), so 15.0, rounded up. A
/// factor only falls, by a measurement (F206 point 4).
const SEAL_DOCUMENT_TIMES: u64 = 16;
/// How many times the attachments a seal takes beyond what the opened vault
/// holds at rest: the peak is 3.2 times one 240 MiB attachment (the opened
/// vault, the file made and the body read back from it) and the opened
/// vault holds 1.0 of those at rest (a 100 MiB attachment: 118 MiB resident
/// after the open, 334 MiB at the seal's peak), so 2.2, rounded up.
const SEAL_ATTACHMENT_TIMES: u64 = 3;

/// How many times the document an open holds at its peak, the file and
/// the body beside it included: 15.7 on a vault of entries. An attachment
/// takes twice its length (the file and the body).
const OPEN_DOCUMENT_TIMES: u64 = 16;
/// How many times the attachments an open holds at its peak.
const OPEN_ATTACHMENT_TIMES: u64 = 2;

/// The longest document a copy may be made of, bytes, where `free` MiB is
/// free ([`OPEN_DOCUMENT_TIMES`] times it is what a copy takes).
fn copy_room_of(free: u32) -> usize {
    (u64::from(free) * 1024 * 1024 / OPEN_DOCUMENT_TIMES) as usize
}

/// The memory a copy of a document of `document` bytes takes, MiB: the
/// exact count of a seal's size makes one, and it takes what an opened
/// document takes ([`OPEN_DOCUMENT_TIMES`]).
fn copy_need_mib(document: usize) -> u32 {
    let bytes = (OPEN_DOCUMENT_TIMES).saturating_mul(document as u64);
    u32::try_from(bytes.div_ceil(1024 * 1024)).unwrap_or(u32::MAX)
}

/// The memory a computer needs free for `step`, MiB (a seal's Argon2id is
/// freed before it builds, so a seal needs the larger of the two): the one
/// function the checks before a file is read, before an unlock starts and
/// before a seal starts all ask (`docs/VAULT.md` §3.1, F206 point 4).
pub fn need_mib(step: Need) -> u32 {
    const MIB: u64 = 1024 * 1024;
    let mib = |bytes: u64| u32::try_from(bytes.div_ceil(MIB)).unwrap_or(u32::MAX);
    let times = |n: u64, len: usize| n.saturating_mul(len as u64);
    match step {
        Need::Read { len } => mib(times(2, len)),
        // Twice the file to the nearest MiB, as a figure a person was shown.
        Need::Unlock { memory_mib, len } => memory_mib
            .saturating_add(u32::try_from((times(2, len) + MIB / 2) / MIB).unwrap_or(u32::MAX))
            .saturating_add(APP_MIB),
        Need::Inflate { plain } => mib(times(2, plain)).saturating_add(APP_MIB),
        Need::Parse {
            document,
            attachments,
        } => mib(times(OPEN_DOCUMENT_TIMES, document)
            .saturating_add(times(OPEN_ATTACHMENT_TIMES, attachments)))
        .saturating_add(APP_MIB),
        Need::Seal {
            memory_mib,
            document,
            attachments,
        } => memory_mib
            .max(mib(times(SEAL_DOCUMENT_TIMES, document)
                .saturating_add(times(SEAL_ATTACHMENT_TIMES, attachments))))
            .saturating_add(APP_MIB),
    }
}

/// The memory a machine needs free to open a file of `len` bytes at this
/// unlock memory, MiB: [`Need::Unlock`].
pub fn vault_need_mib(memory_mib: u32, len: usize) -> u32 {
    need_mib(Need::Unlock { memory_mib, len })
}

/// A custom cost's memory choices, MiB.
pub const CUSTOM_MEMORY: [u32; 8] = [64, 128, 256, 512, 1024, 2048, 3072, 4096];
/// A custom cost's pass choices.
pub const CUSTOM_PASSES: [u32; 7] = [2, 3, 4, 5, 6, 8, 10];

/// The kinds a vault's contents are listed by, with the record types each
/// shows.
pub const CATEGORIES: [(&str, &[u8]); 6] = [
    (
        EN.vaults_bitcoin_keys,
        &[kind::KEY, kind::PASSPHRASE, kind::CODEX32_SHARE],
    ),
    (EN.vaults_wallets, &[kind::WALLET, kind::WALLET_SHARE]),
    (EN.vaults_entries, &[kind::ENTRY]),
    (EN.vaults_notes, &[kind::NOTE, kind::SHEET]),
    (EN.vaults_gpg_keys, &[kind::GPG]),
    (EN.vaults_secure_boot, &[kind::SECURE_BOOT]),
];

/// The bits an unlock cost adds to every passphrase (`docs/VAULT.md`
/// §3.1): one guess is memory × passes of Argon2id's 1 KiB block steps,
/// and the cost adds the base-2 logarithm of that count. Doubling the
/// memory or the passes adds one bit. An estimate, shown with ≈: it
/// counts work, and leaves out how much harder memory is than time for
/// an attacker's hardware.
pub fn cost_bits(memory_mib: u32, passes: u32) -> f32 {
    (f64::from(memory_mib) * 1024.0 * f64::from(passes)).log2() as f32
}

/// How long "Hold to delete" is held before it deletes, in ms.
pub const HOLD_MS: u64 = 1200;

/// The steps of Create a vault.
pub mod vstep {
    /// Where it will be opened.
    pub const WHERE: u8 = 0;
    /// The Argon2id cost.
    pub const COST: u8 = 1;
    /// The passphrases.
    pub const PHRASES: u8 = 2;
    /// One of two presets, standing for the first two steps until
    /// Customise opens them.
    pub const PRESET: u8 = 3;
    /// How many steps there are.
    pub const COUNT: usize = 4;
}

/// The wallet slot a seed is loaded from a vault for: its descriptor key
/// is what says a vault's record is that slot's seed (F65).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotOf {
    /// Slot k of loaded wallet w.
    Loaded(usize, usize),
    /// Slot k of the wallet in open vault v's wallet record r.
    Vault(usize, usize, usize),
    /// A key no descriptor names with a path: a silent payments
    /// wallet's, found by its fingerprint.
    Bare,
}

/// Everything a vault screen can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultAction {
    /// Open a vault from the list: its contents when open, else Unlock.
    Open(usize),
    /// Start Create a vault.
    Create,
    /// Start Create a vault from another flow, which it returns to.
    CreateFrom(Screen),
    /// Unlock vault n from another flow, which it returns to.
    OpenFrom(usize, Screen),
    /// The vault list, to unlock one of several for another flow, which
    /// the unlock returns to.
    ListFrom(Screen),
    /// Back from the vault list to the flow that made a vault.
    Back,
    /// Load the seed with this master fingerprint from whichever open
    /// vault holds it, as the key of wallet slot [`SlotOf`]: a record is
    /// loaded only when it is that slot's key.
    LoadKeyOf([u8; 4], SlotOf),
    /// Load the chosen wallets of open vault v with their seeds in it.
    LoadWithKeys(usize),
    /// From the vault's chart: load wallet record r of open vault v, with
    /// its seeds there where keys may load, and open its card.
    OpenWalletOf(usize, usize),
    /// From the vault's chart: load wallet record r of open vault v as
    /// [`VaultAction::OpenWalletOf`] does, and open its backup's
    /// checklist.
    ChecklistOf(usize, usize),
    /// Show or hide open vault v's seeds and wallets one by one.
    EachShown(usize),
    /// Choose or drop record n of open vault v for the next load.
    Choose(usize, usize),
    /// Load every key and wallet of open vault v.
    LoadAll(usize),
    /// Load the chosen keys and wallets of open vault v.
    LoadChosen(usize),
    /// Choose every key and wallet of open vault v not loaded yet, or
    /// none when all are chosen.
    ChooseAll(usize),
    /// Open or close a Create step.
    CStep(u8),
    /// Close a Create step as done.
    CNext(u8),
    /// Choose or drop a machine the vault will be opened on.
    CMachine(usize),
    /// Choose or drop this computer as one the vault is sized for.
    CHere,
    /// In Memory, choose or drop a machine the vault will be opened on.
    MemoryMachine(usize),
    /// In Memory, choose or drop this computer.
    MemoryHere,
    /// Choose a preset, or `CUSTOM`.
    CPreset(usize),
    /// Choose one of the two sizes, by its place in `SIZES`.
    CSizeRow(usize),
    /// Open the two cards the two presets stand for: where it opens and
    /// the unlock cost.
    CCustomise,
    /// A custom cost's memory.
    CMemory(usize),
    /// A custom cost's passes.
    CPasses(usize),
    /// Type into passphrase n, its first or second field.
    CFocus(usize, bool),
    /// Type the vault's name.
    CName,
    /// Show or hide every passphrase being typed.
    CShow,
    /// Make the vault.
    CGo,
    /// Choose the vault to unlock.
    Pick(usize),
    /// Type the passphrase.
    FocusPassphrase,
    /// Unlock.
    Unlock,
    /// Show an open vault's contents.
    Show(usize),
    /// A kind in the contents.
    Category(usize),
    /// An item of the kind.
    Item(usize),
    /// Show secret field n of the item, or hide it again: its eye.
    Reveal(usize),
    /// Held to delete the item.
    HoldDelete,
    /// Held to delete the files chosen on the Delete files sheet, a vault
    /// among them (`docs/FLOWS.md` decisions 4 and 61).
    HoldWipe,
    /// A key loads at unlock, or stops loading.
    ToggleLoad,
    /// Load the item into the session.
    Load,
    /// Edit the item.
    Edit,
    /// Add an item of the current kind.
    Add,
    /// Add an item of kind n: the contents' row for that kind.
    AddKind(usize),
    /// An empty vault's Write it to a stick: Files.
    WriteOut,
    /// Save session key n into the vault.
    SaveKey(usize),
    /// Save session wallet n into the vault.
    SaveWallet(usize),
    /// Type into the form's field n.
    FocusField(usize),
    /// Keep the form.
    FormSave,
    /// Leave the form.
    FormCancel,
    /// Rename the open vault's slot.
    Rename,
    /// Open Change passphrase on the open vault.
    ChangePassphrase,
    /// Open Memory on the open vault.
    MemoryAsk,
    /// Take memory step n of `STEPS_KIB` for the next seal.
    MemoryStep(usize),
    /// Show what is typed into a passphrase field in the clear, or mask
    /// it again: Unlock's, a prompt's, and an entry's secret fields.
    ShowTyped,
    /// Import the entries in an Inbox text file.
    ImportEntries(usize),
    /// Keep Inbox file n in the open vault as what it is: a text file as a
    /// note, words as a key, a descriptor as a wallet.
    AddFile(usize),
    /// Import the entries in every Inbox entries file.
    ImportAllEntries,
    /// Scan a TOTP setup code into a new entry.
    ScanEntry,
    /// Roll dice for passphrase n.
    Dice(usize),
    /// Choose which of the three EFF lists to roll from, by index into
    /// [`faraday_bip::diceware::List::ALL`].
    DiceList(u8),
    /// Fill the passphrase with the rolled words.
    DiceUse,
    /// Put the dice away.
    DiceClose,
    /// A key of the dice's pad: a face, 1 to 6, or 0 for Backspace.
    DiceKey(u8),
    /// A press on the roller beside the dice entry (Faraday Demo).
    Roller(crate::fresh::roller::Act),
    /// The selected entry out as a KeePass database, under a passphrase
    /// asked for first.
    ExportKdbx,
    /// Load the selected key with a BIP-39 passphrase typed now.
    LoadWithPassphrase,
    /// Save session key n into the vault with its BIP-39 passphrase.
    SaveKeyWithPassphrase(usize),
    /// Type into the prompt.
    FocusPrompt,
    /// Type into the prompt's second field.
    FocusPromptAgain,
    /// Do what the prompt asks.
    PromptGo,
    /// Leave the prompt.
    PromptCancel,
    /// A search bar's press ([`super::SearchAct`]).
    Search(super::SearchAct),
    /// A GPG key's expiry, in years; 0 for never.
    GpgYears(u32),
    /// The selected GPG key's public certificate to the Outbox.
    GpgExport,
    /// The revocation certificate kept with the selected GPG key, through
    /// the secret sheet.
    GpgExportRevocation,
    /// The page that asks why the selected GPG key is revoked
    /// (`gpg --generate-revocation`).
    GpgRevocationPage,
    /// The reason chosen on that page, by GnuPG's menu number 0 to 3.
    GpgReason(u8),
    /// The new term chosen for the selected GPG key's renewal, in years;
    /// 0 for never.
    GpgRenewTerm(u32),
    /// Held to mark the selected GPG key revoked.
    GpgHoldRevoke,
    /// New self-signatures with the term chosen.
    GpgRenew,
    /// Choose Inbox files for the selected GPG key to sign.
    GpgSignPick,
    /// Tick or untick Inbox file n to sign.
    GpgTick(usize),
    /// Show the text of listed file n on the signing page, or hide it
    /// again: its eye.
    SignText(usize),
    /// Tick every signable file, or untick them all when every one is
    /// already ticked.
    GpgTickAll,
    /// Sign a file's **Sign**: the sheet over the page with the files
    /// ticked, the time and the hold (F63). Signs nothing.
    GpgSign,
    /// Held on that sheet to sign every ticked file with the selected GPG
    /// key, one signature each to the Outbox.
    GpgHoldSign,
    /// Choose the Windows-compatible enrolment policy, or own keys only.
    SbOwnOnly(bool),
    /// The enrolment files to the Outbox.
    SbEnrol,
    /// Show the Inbox's EFI images, to sign or check.
    SbImages,
    /// Look at image n before signing it.
    SbPick(usize),
    /// Held to sign the image looked at.
    SbHoldSign,
    /// Check image n's signature.
    SbCheck(usize),
    /// On a small panel, back from an item, or what was being added or
    /// made, to the vault's list.
    ItemBack,
    /// Show the selected key's SeedQR (`docs/BACKUP-FLOW.md` §17.7), or
    /// hide it again: its eye.
    ShowSeedQr,
    /// The selected wallet's descriptor as the Wallets page shows it.
    WalletQr,
    /// The selected entry's TOTP secret as its `otpauth://` URI, on the QR
    /// sheet marked Secret (`docs/QR.md` §7.1).
    TotpQr,
    /// The selected GPG key's paperkey as one static code on the QR sheet
    /// marked Secret (`docs/GPG-PAPERKEY.md` §12, F215).
    PaperkeyQr,
    /// The Load sheet (F62): tick or untick wallet record or Keys-card
    /// key record n.
    LoadTick(usize),
    /// The Load sheet's Unselect all, or Select all once none is ticked.
    LoadTickAll,
    /// The Load sheet: key record n loads with its passphrase number m
    /// (from 0) of those kept for it.
    LoadPassphrase(usize, usize),
    /// The Load sheet's Load ticked.
    LoadTicked,
    /// The Load sheet's Not now: nothing loads, the vault stays open.
    LoadNotNow,
    /// The dialog "a passphrase for this key is already in {vault}": save
    /// the record too.
    SaveAlsoContinue,
    /// The same dialog's Cancel: nothing is saved.
    SaveAlsoCancel,
    /// The item page's **Copy to another vault…**: the sheet of vaults.
    CopyTo,
    /// The item page's **Move to another vault…**: the same sheet, for a
    /// move.
    MoveTo,
    /// The seed page's **Remove codex32 set** or **Remove BIP-85 origin**
    /// (the field's number): the dialog before it.
    TrimAsk(u8),
    /// That dialog's **Remove**.
    TrimGo,
    /// That dialog's **Cancel**.
    TrimCancel,
    /// The item page's **Save as a QR vault**: the QR vault tool at its
    /// Passphrase step, or the dialog saying the item does not fit.
    QrSave,
    /// That dialog's **Close**.
    QrNoClose,
    /// The open vault's ⋮.
    MoreAsk,
    /// The ⋮'s **Key file**: its dialog.
    VKeyAsk,
    /// That dialog's **Make a key file**.
    VKeyMake,
    /// That dialog's **Choose a file**: the list of files.
    VKeyChoose,
    /// That dialog's **Remove key file**.
    VKeyRemove,
    /// The ⋮'s **Remove YubiKey secret** (or **Keep** where a removal waits): the dialog.
    YubiKeyRemoveAsk,
    /// Unlock's **Add a key file**, and the chip: the dialog of files.
    KeyFileAsk,
    /// That dialog's file n marked.
    KeyFilePick(usize),
    /// That dialog's **Choose**.
    KeyFileChoose,
    /// That dialog's **Cancel**.
    KeyFileCancel,
    /// The chip's ×: the key file taken back.
    KeyFileDrop,
    /// Unlock's **Add a YubiKey secret**: its field.
    YubiKeyAdd,
    /// The field's ×: the secret taken away.
    YubiKeyDrop,
    /// The YubiKey secret field pressed.
    FocusYubiKey,
    /// Create a vault's **Add a key file**, and its chip: the dialog.
    CKeyAsk,
    /// That dialog's **Make a key file**.
    CKeyMake,
    /// That dialog's **Choose a file**: the list of files.
    CKeyChoose,
    /// That dialog's **Cancel**.
    CKeyCancel,
    /// The chip's ×: the key file taken back.
    CKeyDrop,
    /// A vault on that sheet pressed, by its place in the list.
    CopyPick(usize),
    /// On that sheet: what a seed carries, its choice n, kept or left out.
    CopyKeep(usize),
    /// The dialog before a copy that leaves data behind, or a move:
    /// **Copy**, **Move**.
    CopyGo,
    /// The Keep sheet: the seed with that fingerprint, its field `number`, ticked or unticked.
    KeepTick([u8; 4], u8),
    /// The Keep sheet's **Save**.
    KeepSave,
    /// The Keep sheet's **Cancel**: nothing is saved.
    KeepCancel,
    /// Session key `k` saved with the choices as they stand, with its
    /// passphrase when `true`: the backup flow, after its own sheet.
    SaveKeyKept(usize, bool, super::keep::Scope),
    /// The sheet's or the dialog's Cancel: nothing is copied.
    CopyCancel,
    /// The older-copy dialog's **Open**: the unlock goes on (`docs/VAULT.md` §11).
    OlderOpen,
    /// Its **Keep locked**: the copy closes, nothing of it read.
    OlderBack,
    /// From wallet n short of a key: the seeds open vaults keep without
    /// a passphrase, one to load with a passphrase typed now (F62).
    PassphraseKey(usize),
    /// On that sheet: seed record r of open vault v, its field.
    PassphrasePick(usize, usize),
    /// On that sheet: back from a seed's field to the list.
    PassphraseList,
    /// Save to a stick: the popup of the selected key's or wallet's files.
    SaveToStick,
    /// Tick or untick the popup's file n.
    SaveTick(u8),
    /// The ticked files to the Outbox, for the stick.
    SaveToStickGo,
}

impl Vaults {
    /// Costs a PBKDF2 and two elliptic-curve multiplications the first
    /// time. A key record's fingerprint and [`crate::wallet::master_id`] on
    /// `network`, worked out once per key and kept.
    pub(crate) fn key_identity(
        &self,
        c: &Compute,
        network: faraday_bip::keys::Network,
        r: RecordView<'_>,
    ) -> Option<(Text, [u8; 32])> {
        let id = faraday_keys::vault::key_record_id(r)?;
        self.kept_identity(id, || faraday_keys::vault::key_identity(c, network, r))
    }

    /// Keeps the identities a vault's unlock worked out in its job, by
    /// key record id, as `kept_identity` keeps them: no work. Ids already
    /// kept stay as they are.
    pub(crate) fn keep_identities(&self, found: &[faraday_keys::job::KeyIdentity]) {
        for (id, fp, mid) in found {
            self.kept_identity(*id, || Some((*fp, *mid)));
        }
    }

    /// Costs what `key_identity` costs. A key record's fingerprint on
    /// `network`, worked out once per key and kept.
    pub(crate) fn key_fingerprint(
        &self,
        c: &Compute,
        network: faraday_bip::keys::Network,
        r: RecordView<'_>,
    ) -> Option<Text> {
        self.key_identity(c, network, r).map(|(fp, _)| fp)
    }

    /// The identity kept for key record id `id`, or `work`'s, kept by the
    /// id once worked out, by whoever asks first.
    pub(crate) fn kept_identity(
        &self,
        id: [u8; 32],
        work: impl FnOnce() -> Option<(faraday_bip::keys::Fingerprint, [u8; 32])>,
    ) -> Option<(Text, [u8; 32])> {
        if let Some((_, fp, mid)) = self.fingerprints.borrow().iter().find(|(k, ..)| *k == id) {
            return Some((fp.clone(), *mid));
        }
        let (fingerprint, mid) = work()?;
        let fp = Text::of(&crate::wallet::fp_text(fingerprint));
        let mut kept = self.fingerprints.borrow_mut();
        if kept.len() >= FINGERPRINTS_KEPT {
            kept.remove(0);
        }
        kept.push((id, fp.clone(), mid));
        Some((fp, mid))
    }

    /// Costs what `described` and `key_fingerprint` cost. The keys and
    /// wallets open vault `v` holds, each with whether `session` has it
    /// already and whether it is chosen for the next load. Reads its wallet
    /// records as it keeps them read ([`Vaults::described`]).
    pub(crate) fn rows(
        &self,
        c: &Compute,
        session: &crate::wallet::Session,
        v: usize,
    ) -> Vec<VaultRow> {
        let Some(open) = self.open.get(v) else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        for (i, r) in self.records(open.id).of(kind::WALLET) {
            let described = self.described(c, open.id, i);
            // A MuSig2 wallet two of whose keys are one seed's can never
            // load: not listed.
            if described.is_some_and(|d| crate::wallet::musig_one_seed(&d.policy)) {
                continue;
            }
            let loaded =
                described.is_some_and(|d| session.wallets.iter().any(|w| w.same() == &*d.same));
            rows.push(VaultRow {
                record: i,
                name: Text::of(&wallet_name_in(r, field::WALLET_NAME, EN.vaults_wallet)),
                detail: described.map(|d| d.shape.clone()).unwrap_or_default(),
                seed: false,
                loaded,
                chosen: !open.skip.contains(&i),
            });
        }
        for (i, r) in self.records(open.id).of(kind::KEY) {
            let fp = self
                .key_fingerprint(c, session.network(), r)
                .unwrap_or_default();
            let loaded = session
                .keys
                .all()
                .any(|k| crate::wallet::fp_text(k.fingerprint) == *fp);
            rows.push(VaultRow {
                record: i,
                name: r
                    .public_text(field::KEY_LABEL)
                    .map(Text::of)
                    .or_else(|| crate::bip85::record_child_mark(r))
                    .unwrap_or_else(|| Text::of(EN.vaults_seed)),
                detail: fp,
                seed: true,
                loaded,
                chosen: !open.skip.contains(&i),
            });
        }
        rows
    }

    /// Costs a parse of each key's words. `session`'s keys and wallets
    /// that no open vault holds, by label and by name (`docs/VAULT.md`
    /// §6.3: locking lists what was not saved).
    pub(crate) fn unsaved(
        &self,
        c: &Compute,
        session: &crate::wallet::Session,
    ) -> (Vec<Text>, Vec<Text>) {
        let held_wallets: Vec<Text> = self
            .open
            .iter()
            .flat_map(|v| {
                self.described_of(c, v.id, kind::WALLET)
                    .map(|(_, _, d)| Text::of(&d.policy.to_descriptor()))
            })
            .collect();
        let keys = session
            .keys
            .all()
            .enumerate()
            .filter(|(i, _)| !self.store().holds_session_words(c, &session.keys, *i))
            .map(|(_, k)| Text::of(&k.label))
            .collect();
        let wallets = session
            .wallets
            .iter()
            .filter(|w| !held_wallets.contains(&Text::of(&w.policy().to_descriptor())))
            .map(|w| Text::of(&w.name))
            .collect();
        (keys, wallets)
    }

    /// Costs a read of each open vault's wallet records: `session`'s
    /// wallets as lines of Wiped from memory (`Faraday::kept_wiped_now`),
    /// each with where else it is (`in_files` for a file in the Inbox or
    /// Outbox) and, while a vault is open, its Save when in none.
    pub(crate) fn wallets_wiped(
        &self,
        c: &Compute,
        session: &crate::wallet::Session,
        in_files: &dyn Fn(&str) -> bool,
        wiped: &mut Vec<(Text, bool, Option<Action>)>,
    ) {
        let in_vault = |descriptor: &str| {
            self.open.iter().find_map(|v| {
                self.described_of(c, v.id, kind::WALLET)
                    .any(|(_, _, d)| d.policy.to_descriptor() == descriptor)
                    .then(|| v.label(self.store()))
            })
        };
        let can_save = self.current_id().is_some();
        for (i, w) in session.wallets.iter().enumerate() {
            let vault = in_vault(&w.policy().to_descriptor());
            let save =
                (can_save && vault.is_none()).then_some(Action::Vault(VaultAction::SaveWallet(i)));
            let (state, nowhere) = match vault {
                Some(v) => (Text::fill(EN.vaults_kept_in_vault, &[&v]), false),
                None if in_files(&w.source) => (Text::of(EN.vaults_in_files), false),
                None => (Text::of(EN.vaults_nowhere_else), true),
            };
            wiped.push((text!("{} · {}", &*w.name, &*state), nowhere, save));
        }
    }
}

/// The head's size line of a file: the size its length shows, or that
/// another program saved it (no padding) with its length in KiB.
pub(super) fn size_line(facts: &fv::file::Facts) -> Text {
    match facts.size {
        Some(n) => Text::fill(EN.vault_holds_up_to, &[&file_text(n)]),
        None => Text::fill(
            EN.vault_saved_by_other,
            &[&Text::fill(
                EN.vault_kib,
                &[&Text::num(facts.len as u64 / 1024)],
            )],
        ),
    }
}

/// The head's line for settings other than Faraday writes, else empty:
/// the version too when it is not 4.0 (`docs/VAULT.md` §7.1).
pub(super) fn settings_line(facts: &fv::file::Facts) -> Text {
    use fv::kdbx::{Cipher, Compression, Kdf};
    if !facts.other_settings() {
        return Text::new();
    }
    let h = &facts.header;
    let cipher = match h.cipher {
        Cipher::Aes256 => EN.vault_setting_aes256,
        Cipher::ChaCha20 => EN.vault_setting_chacha20,
    };
    let kdf = match h.kdf {
        Kdf::Argon2d => EN.vault_setting_argon2d,
        Kdf::Argon2id => EN.vault_setting_argon2id,
    };
    let zip = match h.compression {
        Compression::Gzip => EN.vault_setting_gzip,
        Compression::None => EN.vault_setting_uncompressed,
    };
    // The version is named where it is what differs.
    let version = if h.minor == 0 {
        Text::new()
    } else {
        text!(
            "{} / ",
            &*Text::fill(EN.vault_setting_version, &[&Text::num(u64::from(h.minor))])
        )
    };
    Text::fill(
        EN.vault_other_settings,
        &[&text!("{}{cipher} / {kdf} / {zip}", &*version)],
    )
}

/// The head's line of why a vault is open to read only.
pub(super) fn read_only_line(why: &fv::file::ReadOnly) -> &'static str {
    match why {
        fv::file::ReadOnly::Newer(_) => EN.vault_read_only_newer,
        fv::file::ReadOnly::Ambiguous(_) => EN.vault_read_only_settings,
    }
}
