//! The vault area's state and its types: the open vaults and the vault
//! screens' state ([`Vaults`]) with the item's scroll on a desktop
//! ([`VaultArea`]), the vault's actions, forms, summaries and the text
//! they are written in. Read through [`crate::Faraday::vaults`]; changed
//! through the methods here, and by the vault's own glue in
//! `vault/mod.rs` and its children.

use crate::strings::EN;
use crate::strings::fill;
use faraday_crypto::work::Compute;
use faraday_keys::vault::{OpenVaults, RecordView, Records, VaultId};
use faraday_vault::records::{field, kind};
use faraday_vault::{self as fv, Cost, Header, Record};

use crate::{Action, Screen, flow};

/// The vault area.
pub(crate) struct VaultArea {
    pub(super) state: Vaults,
    item_offset: f32,
}

impl Default for VaultArea {
    fn default() -> VaultArea {
        VaultArea {
            state: Vaults {
                // A new GPG key's term starts at GnuPG's default, never at
                // "never".
                gpg_years: crate::gpg::DEFAULT_YEARS,
                ..Vaults::default()
            },
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
    /// The open vaults and the vault screens' state.
    pub(crate) fn vaults(&self) -> &Vaults {
        &self.state
    }

    /// Design units scrolled past in Vault contents' item, on a desktop.
    pub(crate) fn item_offset(&self) -> f32 {
        self.item_offset
    }

    /// The item's scroll, to move it.
    pub(crate) fn item_offset_mut(&mut self) -> &mut f32 {
        &mut self.item_offset
    }

    /// Sets the item's scroll.
    pub(crate) fn set_item_offset(&mut self, y: f32) {
        self.item_offset = y;
    }

    /// Asks for the one-pass measurement, once.
    pub(crate) fn measure(&mut self) {
        if self.state.ms_per_unit.is_none() && self.state.working.is_none() {
            self.state.working = Some(Work::Measure);
            self.state.drawn = false;
        }
    }

    /// The field typing at `focus` goes into, to change it.
    pub(crate) fn text_box(&mut self, focus: Focus) -> Option<&mut TextBox> {
        match focus {
            Focus::Passphrase => Some(&mut self.state.passphrase),
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
            Focus::Dice => self.state.dice.as_mut().map(|d| &mut d.1),
            Focus::Rolls | Focus::Search(_) => None,
            Focus::Prompt => self.state.prompt.as_mut().map(|p| &mut p.text),
            Focus::Name => self.state.create.as_mut().map(|c| &mut c.name),
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

    /// The vaults' summaries, for the tests' kit to change.
    #[cfg(any(test, feature = "testkit"))]
    pub(crate) fn summaries_mut(&mut self) -> &mut Vec<VaultSummary> {
        &mut self.state.summaries
    }

    /// Changes open vault `id`'s records through `change`
    /// ([`Vaults::edit`]).
    pub(crate) fn edit<T>(&mut self, id: VaultId, change: impl FnOnce(&mut OpenVaults) -> T) -> T {
        self.state.edit(id, change)
    }

    /// Open vault `id` changed ([`Vaults::changed`]).
    pub(crate) fn changed(&mut self, id: VaultId) {
        self.state.changed(id);
    }

    /// Open vault `o` has one more change not saved.
    pub(crate) fn bump(&mut self, o: usize) {
        self.state.open[o].changes += 1;
    }

    /// Closes open vault `o` and shows the first.
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
        self.state.create = Some(form);
        self.state.back_to = Some(back);
        self.state.just_made = None;
        self.state.focus = focus;
    }

    /// Create a vault's scroll offset, to move it.
    pub(crate) fn create_scroll_y_mut(&mut self) -> Option<&mut f32> {
        self.state.create.as_mut().map(|c| &mut c.scroll.y)
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

    /// The unlock passphrase wiped, shown hidden, with no error.
    pub(crate) fn clear_unlock(&mut self) {
        self.state.passphrase.clear();
        self.state.typed_shown = false;
        self.state.unlock_error = None;
    }

    /// The unlock passphrase wiped.
    pub(crate) fn clear_passphrase(&mut self) {
        self.state.passphrase.clear();
    }

    /// Vault `at` unlocked with `passphrase` on the next drawn frame,
    /// returning to `back`.
    pub(crate) fn unlock_with(&mut self, at: usize, passphrase: &str, back: Screen) {
        self.state.pick = at;
        self.state.passphrase.set(passphrase);
        self.state.unlock_error = None;
        self.state.back_to = Some(back);
        self.start_work(Work::Unlock);
    }

    /// Work waiting for its frame: done once a frame is drawn.
    pub(crate) fn start_work(&mut self, work: Work) {
        self.state.working = Some(work);
        self.state.drawn = false;
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

    /// Every secret the item page shows hidden again, its SeedQR with
    /// them.
    pub(crate) fn hide_shown(&mut self) {
        self.state.shown = None;
        self.state.seedqr = None;
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
            self.hide_shown();
        }
    }

    /// The eye of secret field `n` on the item page: shows it, or hides
    /// it again.
    pub(crate) fn toggle_shown(&mut self, on_page: bool, n: usize) {
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
        if s.fields == 0 && self.state.seedqr.is_none() {
            self.state.shown = None;
        }
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

    /// Free memory, MiB, as the shell said it.
    pub(crate) fn set_memory_free(&mut self, mib: u32) {
        self.state.memory_free_mib = Some(mib);
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
    pub(crate) fn set_sb_checked(&mut self, line: String) {
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
            if v.header().salt == *salt && v.name == from {
                v.name = to.to_string();
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
    /// Why the last try failed.
    pub error: Option<String>,
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
    /// Create's name for the vault.
    Name,
    /// A search bar ([`super::SearchIn`]).
    Search(super::SearchIn),
}

/// One vault file the Inbox or Outbox holds.
pub struct VaultFile {
    /// Its file name.
    pub name: String,
    /// It waits in the Outbox (made or sealed here), rather than having
    /// come from a stick.
    pub in_outbox: bool,
    /// Its header.
    pub header: Header,
    /// Its length in bytes.
    pub len: usize,
    /// The open vault it is, when one of its slots is open.
    pub open: Option<usize>,
}

/// A wallet in an open vault that holds some of its seeds too.
#[derive(Debug, Clone)]
pub struct VaultWallet {
    /// The wallet's record.
    pub record: usize,
    /// Its name.
    pub name: String,
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
    pub name: String,
    /// A seed's fingerprint, a wallet's shape.
    pub detail: String,
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
    pub same: String,
    /// Its descriptor checksum.
    pub checksum: String,
    /// Its shape as the session names one: "Single key", "2 of 3".
    pub shape: String,
}

impl Described {
    /// What is kept of a parsed description.
    pub(super) fn of(policy: faraday_bip::policy::WalletPolicy) -> Described {
        Described {
            same: crate::wallet::same_wallet(&policy),
            checksum: policy.checksum().to_string(),
            shape: crate::vault::screen::policy_shape(policy.clone()),
            policy,
        }
    }
}

/// The most key records' fingerprints kept ([`Vaults::fingerprints`]),
/// far more than open vaults hold; the next one drops the oldest.
pub(crate) const FINGERPRINTS_KEPT: usize = 256;
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
    pub(crate) fn words_vault(&self, c: &Compute, id: &[u8; 32]) -> Option<String> {
        self.open.iter().find_map(|v| {
            let mut keys = self.records(v.id).of(kind::KEY);
            keys.any(|(i, _)| self.words_id(c, v.id, i) == Some(id))
                .then(|| v.label(self.store()))
        })
    }

    /// The label of the open vault holding a key record whose id
    /// ([`faraday_keys::vault::key_record_id`]) is `id`: where a key of a
    /// master seed (codex32, SLIP-39) is kept. A hash per key record.
    pub(crate) fn seed_vault(&self, id: [u8; 32]) -> Option<String> {
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

    /// Why `records` do not fit open vault `id`'s slot: the bytes they
    /// take and the room left; None when they fit.
    pub(crate) fn no_room(&self, id: VaultId, records: &[Record]) -> Option<String> {
        let sizes: Vec<usize> = records.iter().map(|r| r.size()).collect();
        if self.store.fits(id, &[], &sizes) {
            return None;
        }
        let slot = self
            .open
            .iter()
            .find(|o| o.id == id)
            .map_or(0, |o| o.header().slot_len);
        let room = (slot as usize).saturating_sub(self.records(id).used());
        let need = sizes.iter().sum::<usize>().to_string();
        Some(fill(EN.vaults_those_entries, &[&need, &room.to_string()]))
    }

    /// Why open vault `i` cannot be sealed: its changes are more than its
    /// slot holds, so a lock would lose them. None when it can be.
    pub(crate) fn cannot_seal(&self, i: usize) -> Option<String> {
        let v = self.open.get(i)?;
        let slot = v.header().slot_len as usize;
        let used = self.records(v.id).used();
        let over = (used - slot.min(used)).to_string();
        (v.changes > 0 && used > slot).then(|| fill(EN.vault_cannot_seal, &[&v.name, &over]))
    }

    /// Why the first open vault that cannot be sealed cannot be
    /// ([`Vaults::cannot_seal`]). Reads only.
    pub fn seal_blocked(&self) -> Option<String> {
        (0..self.open.len()).find_map(|i| self.cannot_seal(i))
    }

    /// The signed-amount memory's records `new` into open vault `i`, the
    /// oldest of them leaving first while the slot is over full
    /// (`docs/VAULT.md` §7, type 10).
    pub(crate) fn add_amounts(&mut self, i: usize, new: Vec<Record>) {
        let Some(v) = self.open.get(i).filter(|_| !new.is_empty()) else {
            return;
        };
        let (id, slot_len) = (v.id, v.header().slot_len as usize);
        self.edit(id, |s| {
            for r in new {
                s.insert(id, r);
            }
            while s.records(id).used() > slot_len {
                let oldest = s
                    .records(id)
                    .iter()
                    .find(|(_, r)| r.kind() == kind::AMOUNTS);
                match oldest.map(|(k, _)| k) {
                    Some(k) => {
                        s.remove(id, k);
                    }
                    None => break,
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
        if let Some(o) = self.open.iter_mut().find(|o| o.id == id) {
            o.read = Kept::default();
        }
        out
    }

    /// Counts one change not yet sealed in open vault `id`.
    pub fn changed(&mut self, id: VaultId) {
        if let Some(o) = self.open.iter_mut().find(|o| o.id == id) {
            o.changes += 1;
        }
    }

    /// The open vault on the contents screen.
    pub fn current_id(&self) -> Option<VaultId> {
        self.open.get(self.current).map(|o| o.id)
    }

    /// Costs a parse of each wallet description and of each key record's
    /// words the first time after a change, a lookup after. Reads open
    /// vault `id`'s records, and finds what the search bars hold in them
    /// ([`super::search::Found`]), unless that is done already.
    pub fn warm(&self, c: &Compute, id: VaultId) {
        let _ = self.read(c, id);
        let _ = self.shown(id, 0);
    }

    /// Costs what `warm` costs. What is read of open vault `id`'s records.
    fn read(&self, c: &Compute, id: VaultId) -> Option<&Read> {
        let o = self.open.iter().find(|o| o.id == id)?;
        let read = o.read.read.get_or_init(|| {
            // The key records' words are read with the descriptions, once.
            self.store.warm(c, id);
            Read {
                version: self.store.version(id),
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
pub type KeyIdentity = ([u8; 32], String, [u8; 32]);

/// A key record's account key at a path, kept once worked out: the
/// record's id, the path as text, and the account key's public key and
/// chain code.
pub(crate) type KeyAccount = ([u8; 32], String, Option<[u8; 65]>);

/// One open slot of one vault: what the screens say of it. Its slot key
/// and records are in [`Vaults::store`], under [`OpenVault::id`].
pub struct OpenVault {
    /// The file name it came under, and goes back under.
    pub name: String,
    /// Which slot of [`Vaults::store`] it is.
    pub id: VaultId,
    /// The header of the file it was opened from, copied from the store
    /// when it opened: public, and the same for as long as it is open.
    pub(super) header: Header,
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
    pub fn label(&self, store: &OpenVaults) -> String {
        store
            .records(self.id)
            .of(kind::SLOT_LABEL)
            .next()
            .and_then(|(_, r)| r.public_text(field::LABEL))
            .unwrap_or(EN.vaults_vault)
            .to_string()
    }

    /// The header of the file it was opened from.
    pub fn header(&self) -> &Header {
        &self.header
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
    /// The slot size.
    pub slot: u32,
    /// The vault's name, which its file is named after.
    pub name: TextBox,
    /// Each passphrase, typed twice.
    pub phrases: Vec<(TextBox, TextBox)>,
    /// For each passphrase the dice made, what they made and its bits:
    /// its strength holds while the field still holds those words.
    pub from_dice: Vec<Option<(zeroize::Zeroizing<String>, f32, crate::fresh::roller::Sim)>>,
    /// The passphrases are shown in the clear, not masked: a person
    /// cannot accept one on faith, only on seeing it.
    pub shown: bool,
    /// What stops the vault being made.
    pub error: Option<String>,
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
            slot: fv::DEFAULT_SLOT,
            name: TextBox::default(),
            phrases: vec![(TextBox::default(), TextBox::default())],
            from_dice: vec![None],
            shown: false,
            error: None,
        }
    }
}

impl CreateForm {
    /// The steps shown, in order: the size and the passphrases, or, once
    /// Customise is pressed, the three cards the size stands for in its
    /// place.
    pub fn steps(&self) -> &'static [u8] {
        if self.customise {
            &[vstep::WHERE, vstep::COST, vstep::SIZE, vstep::PHRASES]
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
            Action::Vault(VaultAction::CSize(_)) => vstep::SIZE,
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
        matches!(self.kind, kind::ENTRY | kind::NOTE)
    }
}

/// Work that waits for a frame saying it is under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    /// Unlock the chosen vault.
    Unlock,
    /// Make the vault the form describes.
    Create,
    /// Time one Argon2id pass over 64 MiB, for the unlock estimates.
    Measure,
    /// Make the key the backup flow's form described.
    FlowMake,
    /// Write the KeePass export the prompt is for.
    Kdbx,
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
    /// Create a vault, while it is on screen.
    pub create: Option<CreateForm>,
    /// The vault chosen to unlock, by its place in the list.
    pub pick: usize,
    /// The unlock passphrase.
    pub passphrase: TextBox,
    /// Why the last unlock failed.
    pub unlock_error: Option<String>,
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
    /// Free memory, MiB, as the shell last said.
    pub memory_free_mib: Option<u32>,
    /// One Argon2id pass over 64 MiB, ms, as measured here.
    pub ms_per_unit: Option<u64>,
    pub(super) measure_from: Option<u64>,
    /// Work waiting for its frame.
    pub working: Option<Work>,
    /// A frame has been drawn since the work was asked for.
    pub(crate) drawn: bool,
    /// When the press now held began, ms.
    pub(crate) pressed_at: u64,
    /// The vault just made, by file name, until it is unlocked or the
    /// person leaves the list.
    pub just_made: Option<String>,
    /// Where an unlock or a creation started from another flow returns.
    pub back_to: Option<Screen>,
    /// Create a vault was opened from Home: Back goes to Home, not the
    /// list.
    pub from_home: bool,
    /// Dice being rolled for a passphrase: which one, the rolls, and
    /// which of the three EFF lists.
    pub dice: Option<(usize, TextBox, faraday_bip::diceware::List)>,
    /// The roller beside the rolls (Faraday Demo): which of their
    /// positions this computer made.
    pub roller: crate::fresh::roller::Roller,
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
    /// The expiry chosen for a GPG key being made or renewed, in years;
    /// 0 for never.
    pub gpg_years: u32,
    /// Choosing Inbox files for the selected GPG key to sign.
    pub signing: bool,
    /// The signable Inbox files ticked to sign, by name and SHA-256: a
    /// file replaced by a stick visit is no longer ticked. Set to its
    /// default (the only file ticked, several none) whenever the page
    /// opens.
    pub sign_ticked: Vec<(String, [u8; 32])>,
    /// The fingerprint of the key the signing page was opened for.
    pub sign_key: Option<[u8; 20]>,
    /// The files the signing page hashed, so that it hashes each once.
    pub sign_seen: std::cell::RefCell<Vec<crate::gpg::Seen>>,
    /// The term chosen to renew the selected GPG key for, in years; none
    /// until the person chooses.
    pub renew_years: Option<u32>,
    /// The reason chosen for a revocation, by GnuPG's menu number.
    pub revoke_reason: u8,
    /// What is typed into passphrase fields is shown in the clear.
    pub typed_shown: bool,
    /// The owner name Secure Boot keys are being made under.
    pub sb_owner: String,
    /// Enrolment adds the owner's keys alone, not Microsoft's.
    pub sb_own_only: bool,
    /// The Inbox's EFI images are shown, to sign or check.
    pub sb_images: bool,
    /// The image being looked at before signing, by Inbox position.
    pub sb_sign: Option<usize>,
    /// What the last check said.
    pub sb_checked: Option<String>,
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
    pub(super) draws: u64,
    /// The selected key's SeedQR while its eye shows it, by record
    /// index: made at the press that shows it, dropped at the press that
    /// hides it and with [`Vaults::shown`].
    pub(crate) seedqr: Option<(usize, faraday_codec::qr::QrMatrix)>,
    /// The item page's secrets shown by their eyes, and where. Invariant:
    /// `None`, and `seedqr` with it, once the place it names is left
    /// (another screen, vault, kind, record or page of the item, a sheet
    /// over it, a lock); kept so by [`crate::Faraday::vault_shown_tidy`]
    /// after every input, and drawing shows a secret only where it was
    /// shown.
    pub(crate) shown: Option<Shown>,
    /// Save to a stick's popup, while it is open.
    pub stick_save: Option<crate::vault::out::StickSave>,
    /// The Load sheet an unlock shows, while it is up (F62).
    pub load: Option<crate::vault::load::LoadSheet>,
    /// The sheet of seeds kept with no passphrase, while it is up (F62).
    pub pass_seeds: Option<crate::vault::load::PassSeeds>,
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

/// The item page's secrets shown by their eyes ([`Vaults::shown`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shown {
    /// Where they were shown.
    pub(crate) at: ShownAt,
    /// Which secret fields of the record, a bit each by their order on
    /// the page.
    pub(crate) fields: u64,
}

/// Memory in MiB as a person reads it.
pub fn mib_text(mib: u32) -> String {
    if mib >= 1024 {
        if mib.is_multiple_of(1024) {
            fill(EN.vaults_gib, &[&(mib / 1024).to_string()])
        } else {
            fill(EN.vaults_gib, &[&format!("{:.1}", f64::from(mib) / 1024.0)])
        }
    } else {
        fill(EN.vaults_mib, &[&mib.to_string()])
    }
}

/// A slot size as a person reads it.
pub fn size_text(slot: u32) -> String {
    if slot >= 1_048_576 {
        fill(EN.vaults_mib, &[&(slot / 1_048_576).to_string()])
    } else {
        fill(EN.vaults_kib, &[&(slot / 1024).to_string()])
    }
}

/// A file size as a person reads it.
pub fn file_text(len: usize) -> String {
    if len >= 1_000_000 {
        fill(EN.vaults_mb, &[&format!("{:.1}", len as f64 / 1e6)])
    } else {
        fill(EN.vaults_kb, &[&len.div_ceil(1000).to_string()])
    }
}

/// A cost as a person reads it.
pub fn cost_text(cost: &Cost) -> String {
    format!(
        "{} · {} {}",
        mib_text(cost.memory_kib / 1024),
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
    pub fn close(&mut self, i: usize) {
        let gone = self.open.remove(i);
        self.store.close(gone.id);
    }

    /// Closes every open vault.
    pub fn close_all(&mut self) {
        self.open.clear();
        self.store.clear();
        self.search = super::search::Searches::default();
    }

    /// Seconds to unlock at a cost on this computer, when measured.
    pub fn seconds(&self, memory_mib: u32, passes: u32) -> Option<f64> {
        self.ms_per_unit
            .map(|ms| ms as f64 * f64::from(memory_mib) / 64.0 * f64::from(passes) / 1000.0)
    }

    /// "about 1.8 s", or that it is not measured yet.
    pub fn time_text(&self, memory_mib: u32, passes: u32) -> String {
        match self.seconds(memory_mib, passes) {
            Some(s) if s < 10.0 => fill(EN.vaults_about_s, &[&format!("{s:.1}")]),
            Some(s) => fill(EN.vaults_about_s, &[&(s.round() as u64).to_string()]),
            None => EN.vaults_not_measured.to_string(),
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
            Focus::Passphrase | Focus::Phrase(..) | Focus::Dice | Focus::Prompt => true,
            Focus::Field(_) => self.form.as_ref().is_some_and(Form::holds_secrets),
            Focus::Rolls | Focus::Search(_) | Focus::Name => false,
        }
    }

    /// The most unlock memory this computer gives a vault, MiB: the
    /// largest custom memory that fits in what it has free beside twice
    /// the file and the app ([`vault_need_mib`]), at most 4 GiB, which is
    /// as much as a vault needs; 4 GiB when it does not say.
    pub fn here_limit(&self) -> u32 {
        let Some(free) = self.memory_free_mib else {
            return MAX_UNLOCK_MIB;
        };
        let slot = self.create.as_ref().map_or(fv::DEFAULT_SLOT, |c| c.slot);
        CUSTOM_MEMORY
            .iter()
            .copied()
            .filter(|m| *m <= MAX_UNLOCK_MIB && vault_need_mib(*m, slot) <= free)
            .max()
            .unwrap_or(CUSTOM_MEMORY[0])
    }

    /// The machine that gives a vault the least unlock memory among
    /// those chosen, label and MiB: this computer what it has free (see
    /// `here_limit`), any other half of its memory. With none chosen,
    /// this computer.
    pub fn weakest(&self) -> (String, u32) {
        let here = (EN.vaults_this_computer_name.to_string(), self.here_limit());
        let Some(c) = self.create.as_ref() else {
            return here;
        };
        let mut w: Option<(String, u32)> = (!c.not_here).then_some(here.clone());
        for (k, m) in MACHINES.iter().enumerate() {
            if c.on[k] && w.as_ref().is_none_or(|w| m.3 / 2 < w.1) {
                w = Some((m.0.to_string(), m.3 / 2));
            }
        }
        w.unwrap_or(here)
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
        !a.text.is_empty() && *a.text == *b.text
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

/// Longest name a vault's file takes, before `.ofv`.
pub const NAME_MAX: usize = 40;

/// The file name, less `.ofv`, a vault named `typed` is written under:
/// letters, digits, hyphens and underscores, spaces made hyphens,
/// anything else left out; `vault` when that leaves nothing.
pub fn vault_stem(typed: &str) -> String {
    let mut out = String::new();
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
        "vault".to_string()
    } else {
        out.to_string()
    }
}

/// The words a list's dice rolls spell, for as many whole words as the
/// rolls make (`faraday-bip`'s diceware lists, in roll order).
pub fn dice_words(rolls: &str, list: faraday_bip::diceware::List) -> Vec<&'static str> {
    let per = list.dice_per_word();
    let digits: Vec<u8> = rolls
        .bytes()
        .filter(|b| (b'1'..=b'6').contains(b))
        .map(|b| b - b'0')
        .collect();
    digits
        .chunks_exact(per)
        .filter_map(|w| list.word(w))
        .collect()
}

/// The place in its list of each whole word the rolls make, in order:
/// the dice read as a number in base 6, first die highest.
pub fn dice_word_indices(rolls: &str, list: faraday_bip::diceware::List) -> Vec<u16> {
    let digits: Vec<u16> = rolls
        .bytes()
        .filter(|b| (b'1'..=b'6').contains(b))
        .map(|b| u16::from(b - b'1'))
        .collect();
    digits
        .chunks_exact(list.dice_per_word())
        .map(|w| w.iter().fold(0u16, |n, d| n * 6 + d))
        .collect()
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
    pub kept: Vec<String>,
    /// Wiped from memory, or lost at power-off: one line each, `true` for
    /// what is nowhere else, and for a key or wallet in no open vault
    /// while one is open, the Save that puts it in that vault.
    pub wiped: Vec<(String, bool, Option<Action>)>,
}

/// A vault file's currency (`docs/history/SIMPLIFY.md` §3.5), computed from the
/// receipt's hashes, never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Currency {
    /// Sealed For the stick, and no receipt names it.
    NeverWritten,
    /// The last receipt that names it wrote the bytes it has now: on the
    /// stick of this label.
    Current(String),
    /// Open with unsaved changes, or sealed bytes that differ from the
    /// receipt's.
    Changed,
    /// Read from a stick and not changed since, the stick not known.
    Unchanged,
}

impl Currency {
    /// The line a person reads: "Never written", "On STICK · current",
    /// "Changed since written", "Unchanged".
    pub fn line(&self) -> String {
        match self {
            Currency::NeverWritten => EN.vaults_never_written.to_string(),
            Currency::Current(label) => fill(EN.vaults_on_current, &[&label.to_string()]),
            Currency::Changed => EN.vaults_changed_since.to_string(),
            Currency::Unchanged => EN.vaults_unchanged.to_string(),
        }
    }
}

/// What the app remembers of a vault seen open, across a lock
/// (`docs/VAULT.md` §11): names, fingerprints and master ids, never
/// anything that spends. Kept in the kept state, so it is gone at power-off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VaultSummary {
    /// The file it was opened from.
    pub file: String,
    /// The file's salt, which is the vault's identity.
    pub salt: [u8; 32],
    /// The open slot's name.
    pub name: String,
    /// Its wallets: name and shape.
    pub wallets: Vec<(String, String)>,
    /// Its wallets' descriptor checksums, in the same order.
    pub sums: Vec<String>,
    /// The descriptor checksums of the wallets it keeps a plan for.
    pub planned: Vec<String>,
    /// Its keys' fingerprints.
    pub keys: Vec<String>,
    /// Its keys' [`crate::wallet::master_id`]s, in the same order: what
    /// says a seed is one of its keys while it is locked, where the
    /// fingerprint only finds a candidate (F65). `None` for a key a
    /// summary kept before the ids were.
    pub ids: Vec<Option<[u8; 32]>>,
    /// How many entries it holds.
    pub entries: usize,
    /// Its backup maps' lines: where each place is and what it holds
    /// (record type 11, field 4).
    pub map: Vec<String>,
    /// When it was last seen open, Unix seconds, when the clock is known.
    pub seen: Option<u64>,
}

impl VaultSummary {
    /// The line a locked vault's row and Home's Unlock lead read under
    /// its name: "Savings 2 of 3 · key 9a6a2580 · 12 entries · seen
    /// 14:02". `tz_min` is the device's time zone (`docs/TOTP.md` §2.3).
    pub fn line(&self, tz_min: i16) -> String {
        let mut parts: Vec<String> = Vec::new();
        // One or two wallets by name and the shape's first part ("2 of
        // 3", "Single key"); more, by count.
        if self.wallets.len() <= 2 {
            for (name, shape) in &self.wallets {
                let short = shape.split(" · ").next().unwrap_or(shape);
                parts.push(format!("{name} {short}"));
            }
        } else {
            parts.push(fill(
                EN.vaults_wallets_n,
                &[&self.wallets.len().to_string()],
            ));
        }
        // The same for keys: one or two by fingerprint, more by count.
        match self.keys.len() {
            0 => {}
            1 => parts.push(fill(EN.vaults_key, &[&self.keys[0].to_string()])),
            2 => parts.push(fill(
                EN.vaults_keys_inline,
                &[&self.keys.join(", ").to_string()],
            )),
            n => parts.push(fill(EN.vaults_keys, &[&n.to_string()])),
        }
        match self.entries {
            0 => {}
            1 => parts.push(EN.vaults_1_entry.to_string()),
            n => parts.push(fill(EN.vaults_entries_n, &[&n.to_string()])),
        }
        if let Some(t) = self.seen {
            parts.push(fill(
                EN.vaults_seen,
                &[&time_of_day(local_secs(t, tz_min)).to_string()],
            ));
        }
        if parts.is_empty() {
            return EN.vaults_empty_line.to_string();
        }
        parts.join(" · ")
    }
}

/// A time of day from local Unix-style seconds ([`local_secs`]), hh:mm.
pub fn time_of_day(unix_secs: u64) -> String {
    let s = unix_secs % 86_400;
    format!("{:02}:{:02}", s / 3600, (s / 60) % 60)
}

/// [`time_of_day`], with seconds: hh:mm:ss, for Settings' and Set the
/// time's Now row.
pub fn time_of_day_secs(unix_secs: u64) -> String {
    let s = unix_secs % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

/// One text field of the kept summaries: no tab or line break.
fn kept_field(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// A record type 11 map line (field 4) with a leading place name
/// replaced by "Place n": a vault written before place names moved out
/// of this field named the place there; `names` are the same record's
/// field 3 values, in place order. A line already naming the place by
/// number, or one for a vault, files, software or an away seed, is
/// returned unchanged.
pub(super) fn redact_place_name(line: &str, names: &[String]) -> String {
    for (p, name) in names.iter().enumerate() {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(name).and_then(|r| r.strip_prefix(':')) {
            return fill(EN.vaults_place, &[&(p + 1).to_string(), rest]);
        }
    }
    line.to_string()
}

/// The summaries as the kept state holds them: a `vault` line per
/// vault, then its `wallet`, `key` and `map` lines, fields split by tabs.
pub(crate) fn summaries_encode(all: &[VaultSummary]) -> Vec<u8> {
    let mut out = String::new();
    for s in all {
        let salt: String = s.salt.iter().map(|b| format!("{b:02x}")).collect();
        let seen = s.seen.map(|t| t.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "vault\t{salt}\t{}\t{}\t{}\t{seen}\n",
            kept_field(&s.file),
            kept_field(&s.name),
            s.entries
        ));
        for (name, shape) in &s.wallets {
            out.push_str(&format!(
                "wallet\t{}\t{}\n",
                kept_field(name),
                kept_field(shape)
            ));
        }
        for (i, k) in s.keys.iter().enumerate() {
            match s.ids.get(i).copied().flatten() {
                Some(id) => out.push_str(&format!(
                    "key\t{}\t{}\n",
                    kept_field(k),
                    crate::backups::hex32(&id)
                )),
                None => out.push_str(&format!("key\t{}\n", kept_field(k))),
            }
        }
        for c in &s.sums {
            out.push_str(&format!("sum\t{}\n", kept_field(c)));
        }
        for c in &s.planned {
            out.push_str(&format!("planned\t{}\n", kept_field(c)));
        }
        for m in &s.map {
            out.push_str(&format!("map\t{}\n", kept_field(m)));
        }
    }
    out.into_bytes()
}

/// The summaries read back; a line that does not read is skipped.
pub(crate) fn summaries_decode(bytes: &[u8]) -> Vec<VaultSummary> {
    let text = String::from_utf8_lossy(bytes);
    let mut out: Vec<VaultSummary> = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[..] {
            ["vault", salt, file, name, entries, seen] => {
                let mut s = VaultSummary::default();
                let bytes: Option<Vec<u8>> = (0..32)
                    .map(|i| {
                        salt.get(i * 2..i * 2 + 2)
                            .and_then(|h| u8::from_str_radix(h, 16).ok())
                    })
                    .collect();
                match bytes.and_then(|b| <[u8; 32]>::try_from(b).ok()) {
                    Some(b) if salt.len() == 64 => s.salt = b,
                    _ => continue,
                }
                s.file = file.to_string();
                s.name = name.to_string();
                s.entries = entries.parse().unwrap_or(0);
                s.seen = seen.parse().ok();
                out.push(s);
            }
            ["wallet", name, shape] => {
                if let Some(s) = out.last_mut() {
                    s.wallets.push((name.to_string(), shape.to_string()));
                }
            }
            ["key", fp] => {
                if let Some(s) = out.last_mut() {
                    s.keys.push(fp.to_string());
                    s.ids.push(None);
                }
            }
            ["key", fp, id] => {
                if let Some(s) = out.last_mut() {
                    s.keys.push(fp.to_string());
                    s.ids.push(crate::backups::read_hex32(id));
                }
            }
            ["sum", c] => {
                if let Some(s) = out.last_mut() {
                    s.sums.push(c.to_string());
                }
            }
            ["planned", c] => {
                if let Some(s) = out.last_mut() {
                    s.planned.push(c.to_string());
                }
            }
            ["map", m] => {
                if let Some(s) = out.last_mut() {
                    s.map.push(m.to_string());
                }
            }
            _ => {}
        }
    }
    out
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
pub fn tz_offset_text(tz_min: i16) -> String {
    let sign = if tz_min < 0 { '-' } else { '+' };
    let abs = tz_min.unsigned_abs();
    format!("{sign}{:02}:{:02}", abs / 60, abs % 60)
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
        0,
        [true, false, false, true, false],
    ),
];

/// The size a new vault starts on: PCs and a Raspberry Pi.
pub const DEFAULT_SIZE: usize = 1;

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

/// The memory a machine needs free to open a vault of this unlock memory
/// and slot size, MiB: the Argon2id memory, twice the file (the file and
/// a sealed copy beside it while it is written) and [`APP_MIB`]
/// (`docs/VAULT.md` §3.1).
pub fn vault_need_mib(memory_mib: u32, slot_len: u32) -> u32 {
    const MIB: u64 = 1024 * 1024;
    let twice = (2 * fv::file_len(slot_len) as u64 + MIB / 2) / MIB;
    memory_mib + twice as u32 + APP_MIB
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

/// Bits of a passphrase from dice counted strong: six words of the EFF
/// long list (`docs/VAULT.md` §3.1).
pub const STRONG_BITS: u32 = 77;

/// The words this list needs for a strong passphrase, said with where the
/// lists come from: text, since Faraday is offline.
pub fn dice_aim(list: faraday_bip::diceware::List) -> String {
    let n = (1..=20)
        .find(|&n| list.bits(n) >= STRONG_BITS)
        .unwrap_or(20);
    fill(
        EN.vaults_aim_for_words,
        &[&n.to_string(), &STRONG_BITS.to_string()],
    )
}

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
    /// The slot size.
    pub const SIZE: u8 = 2;
    /// The passphrases.
    pub const PHRASES: u8 = 3;
    /// The size: one of two presets, standing for the first three steps
    /// until Customise opens them.
    pub const PRESET: u8 = 4;
    /// How many steps there are.
    pub const COUNT: usize = 5;
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
    /// Choose a preset, or `CUSTOM`.
    CPreset(usize),
    /// Choose one of the two sizes, by its place in `SIZES`.
    CSizeRow(usize),
    /// Open the three cards the two sizes stand for: where it opens, the
    /// unlock cost and the space per passphrase.
    CCustomise,
    /// A custom cost's memory.
    CMemory(usize),
    /// A custom cost's passes.
    CPasses(usize),
    /// The slot size.
    CSize(usize),
    /// Another passphrase.
    CAddPhrase,
    /// Remove a passphrase.
    CRemovePhrase(usize),
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
    /// The Load sheet (F62): tick or untick wallet record or Keys-card
    /// key record n.
    LoadTick(usize),
    /// The Load sheet's Unselect all, or Select all once none is ticked.
    LoadTickAll,
    /// The Load sheet's Load ticked.
    LoadTicked,
    /// The Load sheet's Not now: nothing loads, the vault stays open.
    LoadNotNow,
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

/// Whether an action acts when held long enough rather than on release.
pub fn is_hold(a: Action) -> bool {
    matches!(
        a,
        Action::Vault(
            VaultAction::HoldDelete
                | VaultAction::HoldWipe
                | VaultAction::SbHoldSign
                | VaultAction::GpgHoldSign
                | VaultAction::GpgHoldRevoke
        ) | Action::Flow(crate::backup_flow::steps::FlowAction::Hold)
            | Action::HoldPowerOff
            | Action::HoldRestart
            | Action::SecretHold
    )
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
    ) -> Option<(String, [u8; 32])> {
        let id = faraday_keys::vault::key_record_id(r)?;
        self.kept_identity(id, || faraday_keys::vault::key_identity(c, network, r))
    }

    /// Costs what `key_identity` costs. A key record's fingerprint on
    /// `network`, worked out once per key and kept.
    pub(crate) fn key_fingerprint(
        &self,
        c: &Compute,
        network: faraday_bip::keys::Network,
        r: RecordView<'_>,
    ) -> Option<String> {
        self.key_identity(c, network, r).map(|(fp, _)| fp)
    }

    /// The identity kept for key record id `id`, or `work`'s, kept by the
    /// id once worked out, by whoever asks first.
    pub(crate) fn kept_identity(
        &self,
        id: [u8; 32],
        work: impl FnOnce() -> Option<(faraday_bip::keys::Fingerprint, [u8; 32])>,
    ) -> Option<(String, [u8; 32])> {
        if let Some((_, fp, mid)) = self.fingerprints.borrow().iter().find(|(k, ..)| *k == id) {
            return Some((fp.clone(), *mid));
        }
        let (fingerprint, mid) = work()?;
        let fp = crate::wallet::fp_text(fingerprint);
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
            let loaded = described.is_some_and(|d| {
                session
                    .wallets
                    .iter()
                    .any(|w| crate::wallet::same_wallet(w.policy()) == d.same)
            });
            rows.push(VaultRow {
                record: i,
                name: r
                    .public_text(field::WALLET_NAME)
                    .unwrap_or(EN.vaults_wallet)
                    .to_string(),
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
                .any(|k| crate::wallet::fp_text(k.fingerprint) == fp);
            rows.push(VaultRow {
                record: i,
                name: r
                    .public_text(field::KEY_LABEL)
                    .unwrap_or(EN.vaults_seed)
                    .to_string(),
                detail: fp,
                seed: true,
                loaded,
                chosen: !open.skip.contains(&i),
            });
        }
        rows
    }

    /// Costs a parse of each key's words. `session`'s keys and wallets
    /// that no open vault holds, by label and by name (`docs/history/PLAN.md`
    /// §6.3: locking lists what was not saved).
    pub(crate) fn unsaved(
        &self,
        c: &Compute,
        session: &crate::wallet::Session,
    ) -> (Vec<String>, Vec<String>) {
        let held_wallets: Vec<String> = self
            .open
            .iter()
            .flat_map(|v| {
                self.described_of(c, v.id, kind::WALLET)
                    .map(|(_, _, d)| d.policy.to_descriptor())
            })
            .collect();
        let keys = session
            .keys
            .all()
            .enumerate()
            .filter(|(i, _)| !self.store().holds_session_words(c, &session.keys, *i))
            .map(|(_, k)| k.label.clone())
            .collect();
        let wallets = session
            .wallets
            .iter()
            .filter(|w| !held_wallets.contains(&w.policy().to_descriptor()))
            .map(|w| w.name.clone())
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
        wiped: &mut Vec<(String, bool, Option<Action>)>,
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
                Some(v) => (fill(EN.vaults_kept_in_vault, &[&v.to_string()]), false),
                None if in_files(&w.source) => (EN.vaults_in_files.to_string(), false),
                None => (EN.vaults_nowhere_else.to_string(), true),
            };
            wiped.push((format!("{} · {state}", w.name), nowhere, save));
        }
    }
}
