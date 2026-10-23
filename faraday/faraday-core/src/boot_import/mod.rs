//! The boot import (`docs/history/PLAN.md` §5.4; F60): the first look at the boot
//! stick in a power-on copies every file on it into memory, and the sheet
//! lists each with a checkbox, every one ticked. **Import** brings in what
//! is ticked: the files From the stick, their wallets and keys through the
//! Inbox's loader (`Faraday::load_found_in`), and the vaults into Files,
//! locked, each then offered its Unlock in turn with **Cancel**; an
//! unlock shows the vault's Load sheet (F62, `vault_load`) before the
//! next. **Later** brings in nothing. Until then nothing read enters Files: what is not
//! imported is dropped, its bytes wiped as each [`Item`] drops.

pub(crate) mod screen;

use crate::strings::EN;
use crate::strings::fill;
use faraday_crypto::work::Compute;
use std::collections::BTreeSet;

use crate::inbox::SeedSource;
use crate::vault::{Focus, VaultAction};
use crate::wallet::FileKind;
use crate::{
    Action, Faraday, Item, Screen, Sheet, StickInfo, StorageCommand, StorageEvent, stem, stick_kind,
};

/// What the import sheet can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportAction {
    /// Show the sheet over Home.
    Open,
    /// Later: close it. Nothing is imported; what was read stays in
    /// memory for the sheet to open again.
    Later,
    /// Tick or untick file n of [`ImportView::files`].
    File(usize),
    /// Unselect all while any is ticked; Select all once none is.
    All,
    /// Import what is ticked.
    Go,
    /// Unlock the vault the sheet offers, with the passphrase typed.
    Submit,
    /// Leave the vault the sheet offers locked in Files, and go on to the
    /// next.
    Cancel,
}

/// Where one file of the boot stick stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Staged {
    /// Asked for; not answered yet.
    Reading,
    /// Read into memory.
    Read,
    /// A vault, read into the Inbox.
    Vault,
    /// A picture whose QR codes were read: what they held, or why
    /// nothing came of them.
    Codes(String),
    /// A kind Faraday does not read: listed, not read.
    NotRead,
    /// The read failed, or the stick went before it was answered.
    Failed(String),
}

/// One file of the boot stick.
#[derive(Debug, Clone)]
pub struct StagedFile {
    /// Its name on the stick.
    pub name: String,
    /// Its size on the stick.
    pub size: u64,
    /// Where it stands.
    pub state: Staged,
}

/// The boot stick's files, held in memory until the import.
#[derive(Default)]
pub struct ImportState {
    /// The stick's label.
    pub label: String,
    /// The stick's id, as the shell gave it.
    pub stick: String,
    /// Every file on it but the settings file, in the stick's order.
    pub files: Vec<StagedFile>,
    /// What the files read as: one item for a file, a vault's included,
    /// and one for each code a picture held.
    items: Vec<Item>,
    /// The file each item came from, by place in `files`.
    item_file: Vec<usize>,
    /// Files the person unticked, by name: every other is ticked.
    skip: BTreeSet<String>,
    /// Once imported: the vaults brought in, by file name, each waiting
    /// for its Unlock or Cancel in turn. `None` while the sheet picks.
    pub unlocking: Option<Vec<String>>,
    /// How many wallets were loaded before Import: the landing after the
    /// last unlock counts every wallet the import brought.
    pub(crate) wallets_before: usize,
}

/// A file the sheet lists.
#[derive(Debug, Clone)]
pub struct ImportFile {
    /// Its name on the stick.
    pub name: String,
    /// What it is, or what its codes held.
    pub line: String,
    /// Ticked for the import.
    pub chosen: bool,
    /// It can be ticked: read, and something Faraday reads.
    pub enabled: bool,
}

/// A vault the import brought into Files, waiting for its Unlock.
#[derive(Debug, Clone)]
pub struct ImportVault {
    /// Its place in [`Faraday::vault_files`].
    pub file: usize,
    /// Its file's name.
    pub name: String,
    /// The name it goes by: what it was called when last seen open, or
    /// its file's name.
    pub title: String,
    /// Its size.
    pub len: usize,
}

/// What the import sheet shows.
#[derive(Debug, Clone, Default)]
pub struct ImportView {
    /// After Import: the vaults it brought still locked, in the order
    /// they are offered.
    pub vaults: Vec<ImportVault>,
    /// While the sheet picks: every file, each with its checkbox.
    pub files: Vec<ImportFile>,
}

/// What the boot stick brought, counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportCount {
    /// Files read into memory.
    pub copied: usize,
    /// Files still being read.
    pub reading: usize,
    /// Files not read: kinds Faraday does not read, or failed reads.
    pub not_read: usize,
    /// Vault files.
    pub vaults: usize,
    /// Wallets, once each.
    pub wallets: usize,
    /// Keys, once each.
    pub keys: usize,
    /// Files read that are none of these.
    pub other: usize,
    /// Files read that are, or whose codes hold, a PSBT.
    pub psbts: usize,
}

impl ImportCount {
    /// The sheet's one line of what was read: "1 vault · 2 PSBTs · 4
    /// other files", leaving out what there is none of.
    pub fn line(&self) -> String {
        let n =
            |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
        let other = self.copied - self.vaults - self.psbts;
        let parts: Vec<String> = [
            (
                self.vaults,
                EN.boot_import_vault_inline,
                EN.boot_import_vaults,
            ),
            (self.psbts, EN.boot_import_psbt, EN.boot_import_psbts),
            (other, EN.boot_import_other_file, EN.boot_import_other_files),
        ]
        .into_iter()
        .filter(|(k, ..)| *k > 0)
        .map(|(k, one, many)| n(k, one, many))
        .collect();
        if parts.is_empty() {
            EN.boot_import_no_files.to_string()
        } else {
            parts.join(" · ")
        }
    }
}

/// A picture's file name.
fn is_png(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".png")
}

/// `name`, or `stem-2.ext`, … when `taken` says it is in use.
fn free_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(name) {
        return name.to_string();
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (2..1000)
        .map(|n| format!("{stem}-{n}{ext}"))
        .find(|n| !taken(n))
        .unwrap_or_else(|| name.to_string())
}

impl ImportState {
    /// The item places a seed source among the items names.
    fn source_items(src: &SeedSource) -> Vec<usize> {
        match src {
            SeedSource::Words(k) => vec![*k],
            SeedSource::WordsWith(k, p) => vec![*k, *p],
            SeedSource::Slip39(ks) | SeedSource::Codex32(ks) => ks.clone(),
            SeedSource::Loaded => Vec::new(),
        }
    }

    /// The place of the file named `name`, while it waits for its read.
    fn waiting(&self, name: &str) -> Option<usize> {
        self.files
            .iter()
            .position(|f| f.name == name && f.state == Staged::Reading)
    }

    /// Adds a plain file's item, ahead of any picture's codes already
    /// staged: a seed found both as a file and in a picture is labelled
    /// from the file, whichever order the stick's reads come back in.
    fn push_plain_item(&mut self, item: Item, file: usize) {
        let at = self
            .item_file
            .iter()
            .position(|&g| is_png(&self.files[g].name))
            .unwrap_or(self.items.len());
        self.items.insert(at, item);
        self.item_file.insert(at, file);
    }
}

impl Faraday {
    /// The boot stick has arrived for the first time this power-on with
    /// files on it: every one is asked for, the sheet comes up over Home.
    pub(crate) fn import_start(&mut self, boot: &StickInfo) {
        let mut state = ImportState {
            label: boot.label.clone(),
            stick: boot.id.clone(),
            ..ImportState::default()
        };
        for (name, size) in &boot.files {
            if crate::stick::settings::is_file(name) {
                continue;
            }
            let readable = stick_kind(name).is_some();
            state.files.push(StagedFile {
                name: name.clone(),
                size: *size,
                state: if readable {
                    Staged::Reading
                } else {
                    Staged::NotRead
                },
            });
            if !readable {
                continue;
            }
            let (stick, name) = (boot.id.clone(), name.clone());
            self.out.storage_out.push_back(if is_png(&name) {
                StorageCommand::ReadQr { stick, name }
            } else {
                StorageCommand::Read { stick, name }
            });
        }
        self.import = Some(state);
        self.after_visit = None;
        self.screen = Screen::Home;
        // A device waiting to be believed is asked about first
        // (`docs/history/PLAN.md` §4.6); this sheet comes up after.
        if matches!(self.sheet, Some(Sheet::NewInput)) {
            self.import_under_input = true;
        } else {
            self.sheet = Some(Sheet::Import);
        }
    }

    /// Takes the storage answers the import asked for and hands every
    /// other event back.
    pub(crate) fn import_event(&mut self, event: StorageEvent) -> Option<StorageEvent> {
        match event {
            StorageEvent::Read { stick, name, bytes } if self.import_waits(Some(&stick), &name) => {
                self.import_read(&name, bytes)
            }
            StorageEvent::ReadFailed {
                stick,
                name,
                reason,
            } if self.import_waits(Some(&stick), &name) => {
                if let Some(imp) = self.import.as_mut()
                    && let Some(f) = imp.waiting(&name)
                {
                    imp.files[f].state = Staged::Failed(reason);
                }
            }
            // The picture's bytes are not kept: what its codes hold is
            // imported, and the picture is copied on a visit.
            StorageEvent::QrRead { name, payloads, .. } if self.import_waits(None, &name) => {
                self.import_codes(&name, payloads)
            }
            other => return Some(other),
        }
        None
    }

    /// Whether the import waits for `name` from `stick` (any stick for a
    /// picture's codes, whose answer names none).
    fn import_waits(&self, stick: Option<&str>, name: &str) -> bool {
        self.import.as_ref().is_some_and(|i| {
            stick.is_none_or(|s| s == i.stick)
                && i.waiting(name).is_some()
                && (stick.is_some() || is_png(name))
        })
    }

    /// A file the import asked for, read: held here, a vault too, until
    /// the person chooses.
    fn import_read(&mut self, name: &str, bytes: Vec<u8>) {
        let c = self.compute();
        let Some(imp) = self.import.as_mut() else {
            return;
        };
        let Some(f) = imp.waiting(name) else {
            return;
        };
        let item = Item::new(&c, name, bytes);
        imp.files[f].state = if item.kind == FileKind::Vault {
            Staged::Vault
        } else {
            Staged::Read
        };
        imp.push_plain_item(item, f);
    }

    /// The codes a picture the import asked for holds: each becomes an
    /// item remembered as coming from it, a SeedQR as its words.
    fn import_codes(&mut self, name: &str, payloads: Vec<Vec<u8>>) {
        let c = self.compute();
        let Some(f) = self.import.as_ref().and_then(|i| i.waiting(name)) else {
            return;
        };
        let stem = stem(name);
        let mut got: Vec<Item> = Vec::new();
        let mut note: Option<String> = None;
        // Read as the visit reads an image: as the camera reads, with a
        // reading of its own so a transfer in parts started elsewhere is
        // not mixed in.
        let codes = payloads.len();
        let scan = self.scan.replace(crate::ScanState::default());
        for mut p in payloads {
            // A SeedQR's digits are a seed: wiped once read as words.
            if let Some(words) = crate::seedqr_words(&p) {
                zeroize::Zeroize::zeroize(&mut p);
                got.push(Item::new(
                    &c,
                    &format!("{stem}-words.txt"),
                    words.as_bytes().to_vec(),
                ));
                continue;
            }
            if let Some((named, ext, data)) = self.read_code(zeroize::Zeroizing::new(p)) {
                let item = Item::new(&c, &named.unwrap_or_else(|| format!("{stem}.{ext}")), data);
                if item.kind == FileKind::Other {
                    note = Some(EN.boot_import_not_a_psbt.into());
                } else {
                    got.push(item);
                }
            }
        }
        let left = std::mem::replace(&mut self.scan, scan);
        if got.is_empty() && note.is_none() {
            note = left.and_then(|s| s.note.clone());
        }
        let Some(imp) = self.import.as_mut() else {
            return;
        };
        let said = if codes == 0 {
            EN.boot_import_no_qr_code.to_string()
        } else if got.is_empty() {
            note.unwrap_or_else(|| EN.boot_import_its_codes.to_string())
        } else {
            let kinds: Vec<String> = got
                .iter()
                .map(|i| crate::screens::kind_name(i.kind).to_string())
                .collect();
            let n = codes;
            fill(
                EN.boot_import_qr,
                &[
                    &n.to_string(),
                    (if n == 1 {
                        EN.boot_import_code
                    } else {
                        EN.boot_import_codes
                    }),
                    &kinds.join(", ").to_string(),
                ],
            )
        };
        imp.files[f].state = Staged::Codes(said);
        for item in got {
            let taken: Vec<String> = imp.items.iter().map(|i| i.name.clone()).collect();
            let mut item = item;
            item.name = free_name(&item.name, |n| taken.iter().any(|t| t == n));
            imp.items.push(item);
            imp.item_file.push(f);
        }
    }

    /// The boot stick was pulled: what it had not answered never will be.
    /// Nothing comes in by the pull (F60); a vault imported and waiting
    /// has its passphrase field ready.
    pub(crate) fn import_stick_gone(&mut self) {
        let present: Vec<String> = self.sticks.iter().map(|s| s.id.clone()).collect();
        let noun = self.medium.noun();
        let Some(imp) = self.import.as_mut() else {
            return;
        };
        if present.contains(&imp.stick) {
            return;
        }
        for f in imp.files.iter_mut() {
            if f.state == Staged::Reading {
                f.state = Staged::Failed(fill(EN.boot_import_the_was, &[noun]));
            }
        }
        if self.sheet == Some(Sheet::Import) && self.vaults.focus.is_none() {
            self.screen = Screen::Home;
            self.import_field_ready();
        }
    }

    /// The field of the vault the sheet offers, opened as the sheet comes
    /// up with the stick out. On a small panel it waits for a tap, which
    /// brings the keyboard up: until then the sheet has room for the rest.
    fn import_field_ready(&mut self) {
        let Some(v) = self.import_field() else {
            return;
        };
        if !self.may_load_keys() {
            return;
        }
        self.vaults.pick = v.file;
        self.vaults.back_to = None;
        self.vaults.unlock_error = None;
        self.vaults.focus = (!self.compact).then_some(Focus::Passphrase);
    }

    /// The vault whose Unlock the sheet offers: after Import, the first
    /// of those it brought still locked.
    pub fn import_field(&self) -> Option<ImportVault> {
        self.import_view()?.vaults.into_iter().next()
    }

    /// Costs a read of every file it brought that may describe a wallet or
    /// a seed. What the boot stick brought, counted for the sheet's first
    /// line. Kept in the drawing memo while a frame is drawn
    /// (`docs/SPEED.md` §3.3).
    ///
    /// Works out [`crate::drawing::View::import_count`].
    pub fn import_count_now(&self, c: &Compute) -> Option<ImportCount> {
        let imp = self.import.as_ref()?;
        let found = self.found_in(c, &imp.items);
        let mut c = ImportCount::default();
        let mut known: BTreeSet<usize> = BTreeSet::new();
        for w in found.to_load() {
            c.wallets += 1;
            known.extend(w.files.iter().filter_map(|k| imp.item_file.get(*k)));
        }
        for s in found.seeds.iter().filter(|s| !s.loaded) {
            c.keys += 1;
            for src in &s.sources {
                for k in ImportState::source_items(src) {
                    known.extend(imp.item_file.get(k));
                }
            }
        }
        for (f, file) in imp.files.iter().enumerate() {
            match &file.state {
                Staged::Reading => c.reading += 1,
                Staged::NotRead | Staged::Failed(_) => c.not_read += 1,
                Staged::Vault => {
                    c.copied += 1;
                    c.vaults += 1;
                }
                Staged::Read | Staged::Codes(_) => {
                    c.copied += 1;
                    if imp
                        .item_file
                        .iter()
                        .zip(&imp.items)
                        .any(|(g, i)| *g == f && i.kind == FileKind::Psbt)
                    {
                        c.psbts += 1;
                    }
                    let wallet_or_key = known.contains(&f)
                        || imp.item_file.iter().zip(&imp.items).any(|(g, i)| {
                            *g == f && matches!(i.kind, FileKind::Wallet | FileKind::Share)
                        });
                    if !wallet_or_key {
                        c.other += 1;
                    }
                }
            }
        }
        Some(c)
    }

    /// What the import sheet lists: while it picks, every file with its
    /// checkbox; after Import, the vaults it brought still locked.
    pub fn import_view(&self) -> Option<ImportView> {
        let imp = self.import.as_ref()?;
        let mut view = ImportView::default();
        if let Some(names) = &imp.unlocking {
            let files = self.vault_files();
            for name in names {
                let Some(i) = files.iter().position(|f| f.name == *name) else {
                    continue;
                };
                let vf = &files[i];
                if vf.open.is_some() {
                    continue;
                }
                view.vaults.push(ImportVault {
                    file: i,
                    name: vf.name.clone(),
                    title: self
                        .vault_summary(vf)
                        .map_or_else(|| vf.name.clone(), |s| s.name.clone()),
                    len: vf.len,
                });
            }
            return Some(view);
        }
        for (f, file) in imp.files.iter().enumerate() {
            let size = crate::screens::size_text(file.size as usize);
            let (line, enabled) = match &file.state {
                Staged::Reading => (fill(EN.boot_import_being_read, &[&size.to_string()]), false),
                Staged::NotRead => (fill(EN.boot_import_not_a_kind, &[&size.to_string()]), false),
                Staged::Failed(why) => (fill(EN.boot_import_not_read, &[&why.to_string()]), false),
                Staged::Vault => (fill(EN.boot_import_vault_n, &[&size.to_string()]), true),
                Staged::Codes(said) => (said.clone(), imp.item_file.contains(&f)),
                Staged::Read => {
                    let kind = imp
                        .item_file
                        .iter()
                        .position(|g| *g == f)
                        .and_then(|k| imp.items.get(k))
                        .map_or(FileKind::Other, |i| i.kind);
                    (crate::screens::kind_line(kind, file.size as usize), true)
                }
            };
            view.files.push(ImportFile {
                name: file.name.clone(),
                line,
                chosen: enabled && !imp.skip.contains(&file.name),
                enabled,
            });
        }
        Some(view)
    }

    /// Runs an import sheet action.
    pub(crate) fn import_act(&mut self, a: ImportAction) {
        use ImportAction as I;
        match a {
            I::Open => {
                if self.import.is_some() {
                    self.vaults.back_to = None;
                    self.screen = Screen::Home;
                    self.sheet = Some(Sheet::Import);
                    self.vaults.focus = None;
                    self.import_field_ready();
                }
            }
            I::Later => {
                self.sheet = None;
                self.vaults.passphrase.clear();
                self.vaults.typed_shown = false;
                self.vaults.focus = None;
                self.vaults.unlock_error = None;
            }
            I::File(i) => {
                let Some(f) = self
                    .import_view()
                    .and_then(|v| v.files.get(i).filter(|f| f.enabled).cloned())
                else {
                    return;
                };
                if let Some(imp) = self.import.as_mut()
                    && !imp.skip.remove(&f.name)
                {
                    imp.skip.insert(f.name);
                }
            }
            I::All => {
                let Some(view) = self.import_view() else {
                    return;
                };
                let any = view.files.iter().any(|f| f.chosen);
                if let Some(imp) = self.import.as_mut() {
                    if any {
                        imp.skip
                            .extend(view.files.into_iter().filter(|f| f.enabled).map(|f| f.name));
                    } else {
                        imp.skip.clear();
                    }
                }
            }
            I::Go => self.import_go(),
            I::Submit => {
                let Some(v) = self.import_field() else {
                    return;
                };
                if !self.may_load_keys() {
                    return;
                }
                self.vaults.pick = v.file;
                self.vault_act(VaultAction::Unlock);
            }
            I::Cancel => {
                let Some(v) = self.import_field() else {
                    return;
                };
                if let Some(names) = self.import.as_mut().and_then(|i| i.unlocking.as_mut()) {
                    names.retain(|n| *n != v.name);
                }
                self.vaults.passphrase.clear();
                self.vaults.typed_shown = false;
                self.vaults.unlock_error = None;
                self.vaults.focus = None;
                self.import_next();
            }
        }
    }

    /// Imports what is ticked (F60): the files From the stick, with the
    /// wallets and keys they hold through the Inbox's loader (a seed's
    /// words with the stick still in load as it is pulled, as Import and
    /// load's do); the vaults into Files, locked, each then offered its
    /// Unlock. The rest is wiped.
    pub(crate) fn import_go(&mut self) {
        let Some(imp) = self.import.as_mut() else {
            return;
        };
        if imp.unlocking.is_some() {
            return;
        }
        let label = imp.label.clone();
        let ticked: BTreeSet<usize> = (0..imp.files.len())
            .filter(|&f| !imp.skip.contains(&imp.files[f].name) && imp.item_file.contains(&f))
            .collect();
        // Nothing ticked: Import is disabled, and the choice stays open.
        if ticked.is_empty() {
            return;
        }
        let items = std::mem::take(&mut imp.items);
        let item_file = std::mem::take(&mut imp.item_file);
        let mut vaults: Vec<String> = Vec::new();
        let mut plain: Vec<Item> = Vec::new();
        for (item, f) in items.into_iter().zip(item_file) {
            if !ticked.contains(&f) {
                continue;
            }
            if item.kind == FileKind::Vault {
                self.vault_read_from(&item.name, &item.bytes, &label);
                self.inbox.retain(|i| i.name != item.name);
                vaults.push(item.name.clone());
                self.inbox.push(item);
            } else {
                let mut item = item;
                let taken = |n: &str| plain.iter().any(|p: &Item| p.name == n);
                let name = self.free_inbox_name(&item.name);
                item.name = free_name(&name, taken);
                plain.push(item);
            }
        }
        let (keys_before, wallets_before) = (self.session.keys.len(), self.session.wallets.len());
        if let Some(imp) = self.import.as_mut() {
            imp.wallets_before = wallets_before;
        }
        // Every wallet and seed they hold; with the stick in, the seeds,
        // of any kind, wait for the pull.
        self.load_found_in(&plain, &BTreeSet::new(), &|_| true);
        if !self.may_load_keys() {
            self.import_load_after
                .extend(plain.iter().map(|i| i.name.clone()));
        }
        self.inbox.extend(plain);
        self.save_boxes();
        self.refresh_spend();
        self.wallet = 0;
        let keys = self.session.keys.len() - keys_before;
        let wallets = self.session.wallets.len() - wallets_before;
        let count =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        self.toast(&fill(
            EN.boot_import_imported,
            &[
                &count(
                    wallets,
                    EN.boot_import_wallet_inline,
                    EN.boot_import_wallets_inline,
                )
                .to_string(),
                &count(keys, EN.boot_import_key, EN.boot_import_keys).to_string(),
                &count(
                    ticked.len(),
                    EN.boot_import_file,
                    EN.boot_import_files_inline,
                )
                .to_string(),
            ],
        ));
        if let Some(imp) = self.import.as_mut() {
            imp.unlocking = Some(vaults);
        }
        self.import_next();
    }

    /// The stick is pulled: every seed the files Import brought in with
    /// it still in hold loads now (F60), whatever its kind, and the
    /// wallets that name them. Returns how many keys loaded.
    pub(crate) fn import_load_after_pull(&mut self) -> usize {
        if self.import_load_after.is_empty() || !self.may_load_keys() {
            return 0;
        }
        let names = std::mem::take(&mut self.import_load_after);
        let items: Vec<Item> = self
            .inbox
            .iter()
            .filter(|i| names.contains(&i.name))
            .cloned()
            .collect();
        let (_, keys) = self.load_found_in(&items, &BTreeSet::new(), &|_| true);
        if keys > 0 {
            self.refresh_spend();
        }
        keys
    }

    /// After Import, an Unlock or a Cancel: the next vault's field, or,
    /// with none left, the sheet closed and the import over.
    fn import_next(&mut self) {
        if self.import_field().is_some() {
            self.screen = Screen::Home;
            self.sheet = Some(Sheet::Import);
            self.vaults.focus = None;
            self.import_field_ready();
            return;
        }
        self.import = None;
        if self.sheet == Some(Sheet::Import) {
            self.sheet = None;
            self.screen = Screen::Home;
        }
        self.vaults.passphrase.clear();
        if self.vaults.focus == Some(Focus::Passphrase) {
            self.vaults.focus = None;
        }
    }

    /// After Import, with every vault it brought open (unlocked from
    /// Vaults, say): the import is over.
    pub(crate) fn import_settle(&mut self) {
        let waiting = self.import.as_ref().is_some_and(|i| i.unlocking.is_some());
        if waiting && self.import_field().is_none() {
            self.import = None;
        }
    }

    /// Vault `v` just unlocked on the sheet: its Load sheet (F62), or,
    /// when it holds no wallet and no key, the next vault at once.
    /// Returns whether the import is over with nothing left on screen.
    pub(crate) fn import_unlocked(&mut self, v: usize) -> bool {
        let before = self
            .import
            .as_ref()
            .map_or(self.session.wallets.len(), |i| i.wallets_before);
        if self.load_sheet_offer(v, true, before) {
            return false;
        }
        self.import_after_unlock()
    }

    /// After a vault's unlock on the sheet, and its Load sheet if it had
    /// one: the next vault's field, or the import over. Returns whether
    /// it is over.
    pub(crate) fn import_after_unlock(&mut self) -> bool {
        self.import_next();
        self.import.is_none()
    }
}

/// The press that opens the import sheet.
pub const OPEN: Action = Action::Import(ImportAction::Open);

/// The press that closes it for later: Later.
pub const LATER: Action = Action::Import(ImportAction::Later);
