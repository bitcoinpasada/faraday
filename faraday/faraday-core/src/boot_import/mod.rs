//! The boot import (`docs/history/PLAN.md` §5.4; F60, F130): the first
//! look at the boot stick in a power-on lists its files by name and size,
//! nothing of them read, and the sheet lists each with a checkbox, every
//! one ticked, with what it is by its name alone. **Import** reads the
//! ticked files, and those alone, then brings in what they held: the
//! files From the stick, their wallets and keys through the Inbox's
//! loader (`Faraday::load_found_in`), and the vaults into Files, locked,
//! each then offered its Unlock in turn with **Cancel**; an unlock shows
//! the vault's Load sheet (F62, `vault_load`) before the next. **Later**
//! reads nothing and brings in nothing. With the stick pulled before
//! Import the sheet asks for it back; its return brings the sheet up again.
//! What is read is held only until the reads are done, its bytes wiped as
//! each [`Item`] drops.

pub(crate) mod screen;

use crate::strings::EN;
use crate::strings::fill;
use std::collections::BTreeSet;

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
    /// Later: close it. Nothing is read or imported; the list stays for
    /// the sheet to open again.
    Later,
    /// Tick or untick file n of [`ImportView::files`].
    File(usize),
    /// Unselect all while any is ticked; Select all once none is.
    All,
    /// Read what is ticked, and import what it holds.
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
    /// Listed by its name and size alone: nothing of it read.
    Listed,
    /// Ticked when Import was pressed, and asked for; not answered yet.
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

/// The boot stick's files: their names and sizes until Import, then what
/// the ticked ones held until it is done.
#[derive(Default)]
pub struct ImportState {
    /// The stick's label.
    pub label: String,
    /// The stick's id, as the shell gave it.
    pub stick: String,
    /// Every file on it but the settings file, in the stick's order.
    pub files: Vec<StagedFile>,
    /// Import was pressed: the ticked files were asked for, and the
    /// ticks are fixed.
    asked: bool,
    /// What the files Import asked for read as: one item for a file, a
    /// vault's included, and one for each code a picture held.
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
    /// It can be ticked: Import not yet pressed, and a kind Faraday
    /// reads.
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

/// The boot stick's files, counted for the sheet's line: by their names
/// until Import, then how many of those it asked for are read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportCount {
    /// Vault files, by their names.
    pub vaults: usize,
    /// PSBTs, by their names.
    pub psbts: usize,
    /// Other files Faraday reads, by their names.
    pub other: usize,
    /// After Import: the files it asked for.
    pub asked: usize,
    /// After Import: those still being read.
    pub reading: usize,
}

impl ImportCount {
    /// The sheet's one line: before Import, "1 vault · 2 PSBTs · 4 other
    /// files, by their names", leaving out what there is none of; after,
    /// "3 of 7 files read".
    pub fn line(&self) -> String {
        if self.asked > 0 {
            return fill(
                EN.boot_import_files_read,
                &[
                    &(self.asked - self.reading).to_string(),
                    &self.asked.to_string(),
                ],
            );
        }
        let n =
            |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
        let parts: Vec<String> = [
            (
                self.vaults,
                EN.boot_import_vault_inline,
                EN.boot_import_vaults,
            ),
            (self.psbts, EN.boot_import_psbt, EN.boot_import_psbts),
            (
                self.other,
                EN.boot_import_other_file,
                EN.boot_import_other_files,
            ),
        ]
        .into_iter()
        .filter(|(k, ..)| *k > 0)
        .map(|(k, one, many)| n(k, one, many))
        .collect();
        if parts.is_empty() {
            EN.boot_import_no_files.to_string()
        } else {
            fill(EN.boot_import_by_names, &[&parts.join(" · ")])
        }
    }
}

/// A file's extension, lower case: all the sheet knows of what it is
/// before Import.
fn ext(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// A picture's file name.
fn is_png(name: &str) -> bool {
    ext(name) == "png"
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

/// The boot import's area: what the boot stick brought at its first
/// look, the files whose seeds load once the stick is pulled, and whether
/// its sheet waits under the new-device sheet.
#[derive(Default)]
pub(crate) struct BootImportArea {
    state: Option<ImportState>,
    /// Files the boot import brought in with the stick still in, by name:
    /// the seeds they hold load once it is pulled (F60).
    load_after: BTreeSet<String>,
    /// The boot import's sheet waits under the new-device sheet, and
    /// comes up once every waiting device is decided.
    under_input: bool,
}

impl BootImportArea {
    /// What the boot stick brought, while it waits to be imported.
    pub(crate) fn state(&self) -> Option<&ImportState> {
        self.state.as_ref()
    }

    /// Whether an import waits.
    pub(crate) fn open(&self) -> bool {
        self.state.is_some()
    }

    /// The import, to change it.
    pub(crate) fn state_mut(&mut self) -> Option<&mut ImportState> {
        self.state.as_mut()
    }

    /// Starts an import of `state`.
    pub(crate) fn set(&mut self, state: ImportState) {
        self.state = Some(state);
    }

    /// Ends the import, leaving the files that wait for the pull.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }

    /// Ends the import and forgets the files that wait for the pull.
    pub(crate) fn clear_all(&mut self) {
        self.state = None;
        self.load_after.clear();
    }

    /// Whether the sheet waits under the new-device sheet.
    pub(crate) fn set_under_input(&mut self, on: bool) {
        self.under_input = on;
    }

    /// Takes the wait under the new-device sheet, whether there was one.
    pub(crate) fn take_under_input(&mut self) -> bool {
        std::mem::take(&mut self.under_input)
    }

    /// Notes `names` as files whose seeds load once the stick is pulled.
    pub(crate) fn hold_for_pull(&mut self, names: impl IntoIterator<Item = String>) {
        self.load_after.extend(names);
    }

    /// Whether no file waits for the pull.
    pub(crate) fn load_after_is_empty(&self) -> bool {
        self.load_after.is_empty()
    }

    /// Takes the files that wait for the pull.
    pub(crate) fn take_load_after(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.load_after)
    }
}

impl Faraday {
    /// What the boot stick brought, while it waits to be imported.
    pub fn import(&self) -> Option<&ImportState> {
        self.import.state()
    }

    /// The boot stick has arrived for the first time this power-on with
    /// files on it: they are listed by the names and sizes its listing
    /// gave, none of them read (F130), and the sheet comes up over Home.
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
            state.files.push(StagedFile {
                name: name.clone(),
                size: *size,
                state: if stick_kind(name).is_some() {
                    Staged::Listed
                } else {
                    Staged::NotRead
                },
            });
        }
        self.import.set(state);
        self.import_sheet_up();
    }

    /// The import's sheet over Home, or waiting under the new-device
    /// sheet.
    fn import_sheet_up(&mut self) {
        *self.visit.after_visit_mut() = None;
        self.screen = Screen::Home;
        // A device waiting to be believed is asked about first
        // (`docs/history/PLAN.md` §4.6); this sheet comes up after.
        if matches!(self.sheet, Some(Sheet::NewInput)) {
            self.import.set_under_input(true);
        } else {
            self.sheet = Some(Sheet::Import);
        }
    }

    /// A stick arrived in a clean process: whether Home stays on screen
    /// instead of a stick visit. It does for the boot stick alone at its
    /// first look, with the import over it when it holds anything; and
    /// for the boot stick alone put back while its import waits to be
    /// pressed, which makes it the import's stick again and brings the
    /// sheet up over Home (F130).
    pub(crate) fn import_keeps_home(&mut self, first_look: bool, boot: Option<&StickInfo>) -> bool {
        if !self.stick.iter().all(|s| s.boot) {
            return false;
        }
        if first_look {
            return self.screen == Screen::Home;
        }
        let Some(b) = boot else {
            return false;
        };
        let Some(imp) = self.import.state_mut().filter(|i| !i.asked) else {
            return false;
        };
        imp.stick = b.id.clone();
        self.import_sheet_up();
        true
    }

    /// Whether the import's stick is in.
    fn import_stick_in(&self) -> bool {
        self.import
            .state()
            .is_some_and(|i| self.stick.iter().any(|s| s.id == i.stick))
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
                if let Some(imp) = self.import.state_mut()
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
        self.import_if_read();
        None
    }

    /// Whether the import waits for `name` from `stick` (any stick for a
    /// picture's codes, whose answer names none).
    fn import_waits(&self, stick: Option<&str>, name: &str) -> bool {
        self.import.state().is_some_and(|i| {
            stick.is_none_or(|s| s == i.stick)
                && i.waiting(name).is_some()
                && (stick.is_some() || is_png(name))
        })
    }

    /// A file Import asked for, read: held here, a vault too, until every
    /// file it asked for is answered.
    fn import_read(&mut self, name: &str, bytes: Vec<u8>) {
        let c = self.compute();
        let Some(imp) = self.import.state_mut() else {
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
        let Some(f) = self.import.state().and_then(|i| i.waiting(name)) else {
            return;
        };
        let stem = stem(name);
        let mut got: Vec<Item> = Vec::new();
        let mut note: Option<String> = None;
        // Read as the visit reads an image: as the camera reads, with a
        // reading of its own so a transfer in parts started elsewhere is
        // not mixed in.
        let codes = payloads.len();
        let scan = self.camera.swap(Some(crate::ScanState::default()));
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
        let left = self.camera.swap(scan);
        if got.is_empty() && note.is_none() {
            note = left.and_then(|s| s.note.clone());
        }
        let Some(imp) = self.import.state_mut() else {
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

    /// The boot stick was pulled: what it had not answered never will be,
    /// and Import goes on with what was read. Nothing comes in by the pull
    /// before Import (F60); a vault imported and waiting has its passphrase
    /// field ready.
    pub(crate) fn import_stick_gone(&mut self) {
        if self.import_stick_in() {
            return;
        }
        let noun = self.medium.noun();
        let Some(imp) = self.import.state_mut() else {
            return;
        };
        for f in imp.files.iter_mut() {
            if f.state == Staged::Reading {
                f.state = Staged::Failed(fill(EN.boot_import_the_was, &[noun]));
            }
        }
        self.import_if_read();
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
        let focus = (!self.compact).then_some(Focus::Passphrase);
        self.vaults.pick_unlock(v.file, None, focus);
    }

    /// The vault whose Unlock the sheet offers: after Import, the first
    /// of those it brought still locked.
    pub fn import_field(&self) -> Option<ImportVault> {
        self.import_view()?.vaults.into_iter().next()
    }

    /// What the boot stick holds, counted for the sheet's line: by the
    /// files' names before Import, then how many of those it asked for
    /// are read.
    pub fn import_count(&self) -> Option<ImportCount> {
        let imp = self.import.state()?;
        let mut c = ImportCount::default();
        for file in &imp.files {
            match &file.state {
                Staged::NotRead => {}
                Staged::Listed if !imp.asked => match ext(&file.name).as_str() {
                    "ofv" => c.vaults += 1,
                    "psbt" => c.psbts += 1,
                    _ => c.other += 1,
                },
                Staged::Listed => {}
                Staged::Reading => {
                    c.asked += 1;
                    c.reading += 1;
                }
                Staged::Read | Staged::Vault | Staged::Codes(_) | Staged::Failed(_) => {
                    c.asked += 1;
                }
            }
        }
        Some(c)
    }

    /// Whether Import was pressed and its reads are not all answered.
    pub fn import_reading(&self) -> bool {
        self.import
            .state()
            .is_some_and(|i| i.asked && i.unlocking.is_none())
    }

    /// Whether Import can be pressed: the list still picks, something is
    /// ticked, and the stick is in to be read.
    pub fn import_may_go(&self) -> bool {
        self.import_stick_in()
            && self.import.state().is_some_and(|i| {
                !i.asked
                    && i.files
                        .iter()
                        .any(|f| f.state == Staged::Listed && !i.skip.contains(&f.name))
            })
    }

    /// What the import sheet lists: while it picks, every file with its
    /// checkbox; while Import reads, the files it asked for; after, the
    /// vaults it brought still locked.
    pub fn import_view(&self) -> Option<ImportView> {
        let imp = self.import.state()?;
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
            let line = match &file.state {
                // Once Import is pressed, the files it did not ask for
                // are not listed.
                Staged::Listed | Staged::NotRead if imp.asked => continue,
                // What it is by its name alone: nothing of it is read.
                Staged::Listed => fill(
                    EN.boot_import_by_name,
                    &[stick_kind(&file.name).unwrap_or_default(), &size],
                ),
                Staged::NotRead => fill(EN.boot_import_not_a_kind, &[&size]),
                Staged::Reading => fill(EN.boot_import_being_read, &[&size]),
                Staged::Failed(why) => fill(EN.boot_import_not_read, &[why]),
                Staged::Vault => fill(EN.boot_import_vault_n, &[&size]),
                Staged::Codes(said) => said.clone(),
                Staged::Read => {
                    let kind = imp
                        .item_file
                        .iter()
                        .position(|g| *g == f)
                        .and_then(|k| imp.items.get(k))
                        .map_or(FileKind::Other, |i| i.kind);
                    crate::screens::kind_line(kind, file.size as usize)
                }
            };
            let enabled = !imp.asked && file.state == Staged::Listed;
            view.files.push(ImportFile {
                name: file.name.clone(),
                line,
                chosen: imp.asked || (enabled && !imp.skip.contains(&file.name)),
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
                if self.import.state().is_some() {
                    self.vaults.set_back_to(None);
                    self.screen = Screen::Home;
                    self.sheet = Some(Sheet::Import);
                    self.vaults.set_focus(None);
                    self.import_field_ready();
                }
            }
            I::Later => {
                self.sheet = None;
                self.vaults.clear_unlock();
                self.vaults.set_focus(None);
            }
            I::File(i) => {
                let Some(f) = self
                    .import_view()
                    .and_then(|v| v.files.get(i).filter(|f| f.enabled).cloned())
                else {
                    return;
                };
                if let Some(imp) = self.import.state_mut()
                    && !imp.skip.remove(&f.name)
                {
                    imp.skip.insert(f.name);
                }
            }
            I::All => {
                let Some(view) = self
                    .import_view()
                    .filter(|v| v.files.iter().any(|f| f.enabled))
                else {
                    return;
                };
                let any = view.files.iter().any(|f| f.chosen);
                if let Some(imp) = self.import.state_mut() {
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
                self.vaults.set_pick(v.file);
                self.vault_act(VaultAction::Unlock);
            }
            I::Cancel => {
                let Some(v) = self.import_field() else {
                    return;
                };
                if let Some(names) = self.import.state_mut().and_then(|i| i.unlocking.as_mut()) {
                    names.retain(|n| *n != v.name);
                }
                self.vaults.clear_unlock();
                self.vaults.set_focus(None);
                self.import_next();
            }
        }
    }

    /// Import pressed (F130): the ticked files, and those alone, are asked
    /// of the stick; once every one is answered, what they held comes in
    /// ([`Faraday::import_if_read`]). Nothing ticked, or the stick out:
    /// Import is disabled, and the choice stays open.
    pub(crate) fn import_go(&mut self) {
        if !self.import_may_go() {
            return;
        }
        let Some(imp) = self.import.state_mut() else {
            return;
        };
        imp.asked = true;
        for f in imp.files.iter_mut() {
            if f.state != Staged::Listed || imp.skip.contains(&f.name) {
                continue;
            }
            f.state = Staged::Reading;
            let (stick, name) = (imp.stick.clone(), f.name.clone());
            self.out.storage_out.push_back(if is_png(&name) {
                StorageCommand::ReadQr { stick, name }
            } else {
                StorageCommand::Read { stick, name }
            });
        }
    }

    /// Once every file Import asked for is answered, brings in what they
    /// held (F60): the files From the stick, with the wallets and keys they
    /// hold through the Inbox's loader (a seed's words with the stick
    /// still in load as it is pulled, as Import and load's do); the vaults
    /// into Files, locked, each then offered its Unlock.
    fn import_if_read(&mut self) {
        let Some(imp) = self.import.state_mut() else {
            return;
        };
        if !imp.asked
            || imp.unlocking.is_some()
            || imp.files.iter().any(|f| f.state == Staged::Reading)
        {
            return;
        }
        let label = imp.label.clone();
        let failed = imp
            .files
            .iter()
            .filter(|f| matches!(f.state, Staged::Failed(_)))
            .count();
        let items = std::mem::take(&mut imp.items);
        let item_file = std::mem::take(&mut imp.item_file);
        let files: BTreeSet<usize> = item_file.iter().copied().collect();
        let mut vaults: Vec<String> = Vec::new();
        let mut plain: Vec<Item> = Vec::new();
        // A file whose name Files holds with other bytes comes in beside
        // it under a numbered name, said in the toast; one Files holds
        // with the same bytes is read for what it gives, not kept twice.
        let mut renamed: Vec<String> = Vec::new();
        let mut already: BTreeSet<String> = BTreeSet::new();
        for mut item in items {
            let again = self
                .inbox
                .iter()
                .any(|i| i.name == item.name && i.bytes == item.bytes);
            if item.kind == FileKind::Vault {
                if again {
                    self.inbox.retain(|i| i.name != item.name);
                } else {
                    let name = self.free_inbox_name(&item.name);
                    if name != item.name {
                        renamed.push(name.clone());
                    }
                    item.name = name;
                }
                self.vault_read_from(&item.name, &item.bytes, &label);
                vaults.push(item.name.clone());
                self.inbox.push(item);
            } else if again {
                already.insert(item.name.clone());
                plain.push(item);
            } else {
                let taken = |n: &str| plain.iter().any(|p: &Item| p.name == n);
                let name = free_name(&self.free_inbox_name(&item.name), taken);
                if name != item.name {
                    renamed.push(name.clone());
                }
                item.name = name;
                plain.push(item);
            }
        }
        let (keys_before, wallets_before) = (self.session.keys.len(), self.session.wallets.len());
        if let Some(imp) = self.import.state_mut() {
            imp.wallets_before = wallets_before;
        }
        // Every wallet and seed they hold; with the stick in, the seeds,
        // of any kind, wait for the pull.
        self.load_found_in(&plain, &BTreeSet::new(), &|_| true);
        if !self.may_load_keys() {
            self.import
                .hold_for_pull(plain.iter().map(|i| i.name.clone()));
        }
        self.inbox
            .extend(plain.into_iter().filter(|i| !already.contains(&i.name)));
        self.save_boxes();
        self.refresh_spend();
        self.wallets.set_wallet(0);
        let keys = self.session.keys.len() - keys_before;
        let wallets = self.session.wallets.len() - wallets_before;
        self.import_toast(wallets, keys, files.len(), failed, &renamed);
        if let Some(imp) = self.import.state_mut() {
            imp.unlocking = Some(vaults);
        }
        // The sheet put off while the files were read stays off: Home
        // leads with the vault's Unlock.
        if self.sheet == Some(Sheet::Import) {
            self.import_next();
        } else {
            self.import_settle();
        }
    }

    /// What Import brought in, said: "Imported 1 wallet · 2 keys · 6
    /// files", "· 1 not read" when a file it asked for was not, and the
    /// names of the files kept beside others of their names.
    fn import_toast(
        &mut self,
        wallets: usize,
        keys: usize,
        files: usize,
        failed: usize,
        renamed: &[String],
    ) {
        let count =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        let mut said = fill(
            EN.boot_import_imported,
            &[
                &count(
                    wallets,
                    EN.boot_import_wallet_inline,
                    EN.boot_import_wallets_inline,
                ),
                &count(keys, EN.boot_import_key, EN.boot_import_keys),
                &count(files, EN.boot_import_file, EN.boot_import_files_inline),
            ],
        );
        if failed > 0 {
            said = format!(
                "{said} · {}",
                fill(EN.boot_import_n_not_read, &[&failed.to_string()])
            );
        }
        if !renamed.is_empty() {
            said = fill(EN.boot_import_kept_as, &[&said, &renamed.join(", ")]);
        }
        self.toast(&said);
    }

    /// The stick is pulled: every seed the files Import brought in with
    /// it still in hold loads now (F60), whatever its kind, and the
    /// wallets that name them. Returns how many keys loaded.
    pub(crate) fn import_load_after_pull(&mut self) -> usize {
        if self.import.load_after_is_empty() || !self.may_load_keys() {
            return 0;
        }
        let names = self.import.take_load_after();
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
            self.vaults.set_focus(None);
            self.import_field_ready();
            return;
        }
        self.import.clear();
        if self.sheet == Some(Sheet::Import) {
            self.sheet = None;
            self.screen = Screen::Home;
        }
        self.vaults.clear_passphrase();
        if self.vaults.focus == Some(Focus::Passphrase) {
            self.vaults.set_focus(None);
        }
    }

    /// After Import, with every vault it brought open (unlocked from
    /// Vaults, say): the import is over.
    pub(crate) fn import_settle(&mut self) {
        let waiting = self.import.state().is_some_and(|i| i.unlocking.is_some());
        if waiting && self.import_field().is_none() {
            self.import.clear();
        }
    }

    /// Vault `v` just unlocked on the sheet: its Load sheet (F62), or,
    /// when it holds no wallet and no key, the next vault at once.
    /// Returns whether the import is over with nothing left on screen.
    pub(crate) fn import_unlocked(&mut self, v: usize) -> bool {
        let before = self
            .import
            .state()
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
        self.import.state().is_none()
    }
}

/// The press that opens the import sheet.
pub const OPEN: Action = Action::Import(ImportAction::Open);

/// The press that closes it for later: Later.
pub const LATER: Action = Action::Import(ImportAction::Later);
