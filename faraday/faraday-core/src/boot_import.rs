//! The boot import (`PLAN.md` §5.4): the first look at the boot stick in a
//! power-on copies every file on it into memory, the person pulls the
//! stick, unlocks the vaults they want, chooses the wallets, keys and
//! files to bring in, and one press imports them and wipes the rest.
//!
//! Vault files go into the Inbox as soon as they are read, so that the
//! vault screens unlock them as they unlock any other: they are
//! ciphertext. Everything else waits here, outside the Inbox, until the
//! import: the wallets and keys chosen load through the Inbox's loader
//! and the vaults' (`Faraday::load_found_in`, `Faraday::vault_load_set`),
//! the files chosen move into the Inbox, and the rest is dropped, its
//! bytes wiped as each [`Item`] drops.

use std::collections::BTreeSet;

use faraday_vault::records::{field, kind};

use crate::inbox::SeedSource;
use crate::vaults::VaultAction;
use crate::wallet::{FileKind, Session, Wallet, fp_text};
use crate::{
    Action, Faraday, Item, Screen, Sheet, StickInfo, StorageCommand, StorageEvent, stem, stick_kind,
};

/// What the import sheet can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportAction {
    /// Show the sheet over Home.
    Open,
    /// Close it; what was copied stays in memory.
    Later,
    /// Choose or drop wallet n of [`ImportView::wallets`].
    Wallet(usize),
    /// Choose or drop key n of [`ImportView::keys`].
    Key(usize),
    /// Choose or drop file n of [`ImportView::files`] for the Inbox.
    File(usize),
    /// Unlock vault n of [`ImportView::vaults`].
    Unlock(usize),
    /// Import what is chosen and wipe the rest.
    Go,
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
    /// What the files read as: one item for a file, one for each code a
    /// picture held. Vault files are in the Inbox, not here.
    items: Vec<Item>,
    /// The file each item came from, by place in `files`.
    item_file: Vec<usize>,
    /// Wallets left out of the import, by descriptor.
    skip_wallets: BTreeSet<String>,
    /// Keys left out of the import, by fingerprint.
    skip_keys: BTreeSet<String>,
    /// Files chosen for the Inbox, by name.
    to_inbox: BTreeSet<String>,
}

/// A wallet the import offers.
#[derive(Debug, Clone)]
pub struct ImportWallet {
    /// Its name.
    pub name: String,
    /// Its descriptor, in the one form wallets are compared in.
    pub descriptor: String,
    /// Its shape: single key, or m of n and the script.
    pub shape: String,
    /// Its keys' fingerprints.
    pub keys: Vec<String>,
    /// Of those, how many a seed here, in an open vault or loaded gives.
    pub here: usize,
    /// Signatures it needs.
    pub needed: usize,
    /// The files that carry it and its keys.
    pub files: Vec<String>,
    /// Chosen for the import.
    pub chosen: bool,
}

impl ImportWallet {
    /// The keys here reach the signatures it needs.
    pub fn can_sign(&self) -> bool {
        self.here > 0 && self.here >= self.needed
    }

    /// Whether it can sign here, how many keys it still needs, or that it
    /// is watch-only.
    pub fn status(&self) -> String {
        let n = self.keys.len().max(1);
        if self.can_sign() {
            "Can sign".to_string()
        } else if self.here > 0 {
            let more = self.needed - self.here;
            format!("{} of {n} keys here · {more} more needed", self.here)
        } else {
            "Watch-only".to_string()
        }
    }
}

/// A key the import offers that no wallet here uses.
#[derive(Debug, Clone)]
pub struct ImportKey {
    /// Its master fingerprint.
    pub fingerprint: String,
    /// The files it comes from.
    pub files: Vec<String>,
    /// Chosen for the import.
    pub chosen: bool,
}

/// A file the import offers for the Inbox.
#[derive(Debug, Clone)]
pub struct ImportFile {
    /// Its name on the stick.
    pub name: String,
    /// What it is, or what its codes held.
    pub line: String,
    /// Chosen for the Inbox.
    pub chosen: bool,
    /// It can be chosen or dropped: not unread, and not an open vault's.
    pub enabled: bool,
}

/// A vault file the boot stick brought.
#[derive(Debug, Clone)]
pub struct ImportVault {
    /// Its place in [`Faraday::vault_files`].
    pub file: usize,
    /// Its name.
    pub name: String,
    /// The open slot's label, when it is open.
    pub open: Option<String>,
    /// Its size.
    pub len: usize,
}

/// What the import sheet shows.
#[derive(Debug, Clone, Default)]
pub struct ImportView {
    /// The vaults the stick brought.
    pub vaults: Vec<ImportVault>,
    /// The wallets, once each.
    pub wallets: Vec<ImportWallet>,
    /// The keys no wallet here uses.
    pub keys: Vec<ImportKey>,
    /// Every file, for the Inbox.
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

/// Adds `name` to `list` once.
fn push_once(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|n| n == name) {
        list.push(name.to_string());
    }
}

impl ImportState {
    /// The names of the files item places in `ks` came from.
    fn names_of(&self, ks: &[usize]) -> Vec<String> {
        let mut out = Vec::new();
        for k in ks {
            if let Some(f) = self.item_file.get(*k).and_then(|f| self.files.get(*f)) {
                push_once(&mut out, &f.name);
            }
        }
        out
    }

    /// The item places a seed source among the items names.
    fn source_items(src: &SeedSource) -> Vec<usize> {
        match src {
            SeedSource::Words(k) => vec![*k],
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
            if crate::stick_settings::is_file(name) {
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
            self.storage_out.push_back(if is_png(&name) {
                StorageCommand::ReadQr { stick, name }
            } else {
                StorageCommand::Read { stick, name }
            });
        }
        self.import = Some(state);
        self.after_visit = None;
        self.screen = Screen::Home;
        self.sheet = Some(Sheet::Import);
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
            StorageEvent::QrRead { name, payloads } if self.import_waits(None, &name) => {
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

    /// A file the import asked for, read.
    fn import_read(&mut self, name: &str, bytes: Vec<u8>) {
        let Some(imp) = self.import.as_mut() else {
            return;
        };
        let Some(f) = imp.waiting(name) else {
            return;
        };
        let item = Item::new(name, bytes);
        match item.kind {
            // A vault goes into the Inbox, where the vault screens unlock
            // it.
            FileKind::Vault => {
                imp.files[f].state = Staged::Vault;
                imp.to_inbox.insert(name.to_string());
                self.inbox.retain(|i| i.name != name);
                self.inbox.push(item);
                self.save_boxes();
            }
            kind => {
                if kind == FileKind::Psbt {
                    imp.to_inbox.insert(name.to_string());
                }
                imp.files[f].state = Staged::Read;
                imp.push_plain_item(item, f);
            }
        }
    }

    /// The codes a picture the import asked for holds: each becomes an
    /// item remembered as coming from it, a SeedQR as its words.
    fn import_codes(&mut self, name: &str, payloads: Vec<Vec<u8>>) {
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
                    &format!("{stem}-words.txt"),
                    words.as_bytes().to_vec(),
                ));
                continue;
            }
            if let Some((named, ext, data)) = self.read_code(p) {
                let item = Item::new(&named.unwrap_or_else(|| format!("{stem}.{ext}")), data);
                if item.kind == FileKind::Other {
                    note = Some("Not a PSBT, a descriptor, an xpub or a signed message".into());
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
            "No QR code found".to_string()
        } else if got.is_empty() {
            note.unwrap_or_else(|| "Its codes are not complete".to_string())
        } else {
            let kinds: Vec<String> = got
                .iter()
                .map(|i| crate::screens::kind_name(i.kind).to_string())
                .collect();
            let n = codes;
            format!(
                "{n} QR {} · {}",
                if n == 1 { "code" } else { "codes" },
                kinds.join(", ")
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
    pub(crate) fn import_stick_gone(&mut self) {
        let present: Vec<String> = self.sticks.iter().map(|s| s.id.clone()).collect();
        if let Some(imp) = self.import.as_mut()
            && !present.contains(&imp.stick)
        {
            for f in imp.files.iter_mut() {
                if f.state == Staged::Reading {
                    f.state = Staged::Failed("the stick was removed before it was read".into());
                }
            }
        }
    }

    /// The boot stick the import came from is still attached.
    pub fn import_stick_present(&self) -> bool {
        self.import
            .as_ref()
            .is_some_and(|i| self.sticks.iter().any(|s| s.id == i.stick))
    }

    /// What the boot stick brought, counted for the sheet's first line.
    pub fn import_count(&self) -> Option<ImportCount> {
        let imp = self.import.as_ref()?;
        let found = self.found_in(&imp.items);
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

    /// What the import sheet lists: the stick's vaults, the wallets and
    /// keys found in its files and in the open vaults, and every file.
    pub fn import_view(&self) -> Option<ImportView> {
        let imp = self.import.as_ref()?;
        let found = self.found_in(&imp.items);
        let mut view = ImportView::default();

        // The vaults the stick brought, in the Inbox now.
        let stick_vaults: Vec<&str> = imp
            .files
            .iter()
            .filter(|f| f.state == Staged::Vault)
            .map(|f| f.name.as_str())
            .collect();
        for (i, vf) in self.vault_files().iter().enumerate() {
            if vf.in_outbox || !stick_vaults.contains(&vf.name.as_str()) {
                continue;
            }
            view.vaults.push(ImportVault {
                file: i,
                name: vf.name.clone(),
                open: vf
                    .open
                    .and_then(|o| self.vaults.open.get(o))
                    .map(|o| o.label()),
                len: vf.len,
            });
        }

        // Every key here: the stick's seeds, the open vaults' and the
        // session's, by fingerprint, with where each comes from.
        let mut keys_here: Vec<(String, Vec<String>, bool)> = Vec::new();
        let key_from = |keys: &mut Vec<(String, Vec<String>, bool)>,
                        fp: String,
                        from: Vec<String>,
                        loaded: bool| {
            match keys.iter_mut().find(|(k, ..)| *k == fp) {
                Some((_, files, l)) => {
                    for n in from {
                        push_once(files, &n);
                    }
                    *l |= loaded;
                }
                None => keys.push((fp, from, loaded)),
            }
        };
        for s in &found.seeds {
            let mut from = Vec::new();
            for src in &s.sources {
                for n in imp.names_of(&ImportState::source_items(src)) {
                    push_once(&mut from, &n);
                }
            }
            key_from(&mut keys_here, fp_text(s.fingerprint), from, s.loaded);
        }
        for open in &self.vaults.open {
            for (_, r) in open.contents.of(kind::KEY) {
                if let Some(fp) = crate::vault_screens::key_fingerprint(self, r) {
                    let loaded = self
                        .session
                        .keys
                        .iter()
                        .any(|k| fp_text(k.master.fingerprint()) == fp);
                    key_from(&mut keys_here, fp, vec![open.name.clone()], loaded);
                }
            }
        }

        // The wallets: the stick's files first, then the open vaults';
        // once each by descriptor, leaving out those loaded already.
        let loaded_desc: BTreeSet<String> = self
            .session
            .wallets
            .iter()
            .map(|w| crate::wallet::same_wallet(&w.policy))
            .collect();
        let add_wallet = |view: &mut ImportView, w: ImportWallet| {
            if loaded_desc.contains(&w.descriptor) {
                return;
            }
            match view
                .wallets
                .iter_mut()
                .find(|v| v.descriptor == w.descriptor)
            {
                Some(v) => {
                    for n in &w.files {
                        push_once(&mut v.files, n);
                    }
                }
                None => view.wallets.push(w),
            }
        };
        let shares: Vec<usize> = imp
            .items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == FileKind::Share)
            .map(|(k, _)| k)
            .collect();
        for fw in found.to_load() {
            let files = if fw.from_shares {
                imp.names_of(&shares)
            } else {
                imp.names_of(&fw.files)
            };
            add_wallet(
                &mut view,
                ImportWallet {
                    name: fw.name.clone(),
                    descriptor: fw.descriptor.clone(),
                    shape: fw.shape.clone(),
                    keys: fw.keys.iter().map(|k| fp_text(*k)).collect(),
                    here: 0,
                    needed: fw.needed,
                    files,
                    chosen: !imp.skip_wallets.contains(&fw.descriptor),
                },
            );
        }
        for open in &self.vaults.open {
            for (_, r) in open.contents.of(kind::WALLET) {
                let Ok(policy) = crate::wallet::read_wallet(r.text(field::WALLET).unwrap_or(""))
                else {
                    continue;
                };
                let wallet = Wallet {
                    name: r.text(field::WALLET_NAME).unwrap_or("Wallet").to_string(),
                    policy,
                    source: String::new(),
                };
                let descriptor = crate::wallet::same_wallet(&wallet.policy);
                let keys: Vec<String> = wallet
                    .policy
                    .keys()
                    .iter()
                    .filter_map(|k| k.fingerprint())
                    .map(fp_text)
                    .collect();
                add_wallet(
                    &mut view,
                    ImportWallet {
                        chosen: !imp.skip_wallets.contains(&descriptor),
                        name: wallet.name.clone(),
                        shape: Session::shape(&wallet),
                        needed: crate::wallet::needed(&wallet),
                        descriptor,
                        keys,
                        here: 0,
                        files: vec![open.name.clone()],
                    },
                );
            }
        }
        // Each wallet's keys here, and the files they come from.
        for w in view.wallets.iter_mut() {
            let mut keys: Vec<String> = Vec::new();
            for k in &w.keys {
                push_once(&mut keys, k);
            }
            w.keys = keys;
            for k in &w.keys {
                if let Some((_, from, _)) = keys_here.iter().find(|(fp, ..)| fp == k) {
                    w.here += 1;
                    for n in from {
                        push_once(&mut w.files, n);
                    }
                }
            }
        }

        // The keys no wallet here uses, and not loaded already.
        for (fp, from, loaded) in &keys_here {
            if *loaded || view.wallets.iter().any(|w| w.keys.contains(fp)) {
                continue;
            }
            view.keys.push(ImportKey {
                fingerprint: fp.clone(),
                files: from.clone(),
                chosen: !imp.skip_keys.contains(fp),
            });
        }

        // Every file, for the Inbox.
        for (f, file) in imp.files.iter().enumerate() {
            let size = crate::screens::kind_line(FileKind::Other, file.size as usize);
            let size = size.trim_start_matches("File · ").to_string();
            let open_vault = file.state == Staged::Vault
                && view
                    .vaults
                    .iter()
                    .any(|v| v.name == file.name && v.open.is_some());
            let (line, enabled) = match &file.state {
                Staged::Reading => (format!("Being read · {size}"), false),
                Staged::NotRead => (format!("Not a kind of file Faraday reads · {size}"), false),
                Staged::Failed(why) => (format!("Not read: {why}"), false),
                Staged::Vault if open_vault => (format!("Vault, open · {size}"), false),
                Staged::Vault => (format!("Vault · {size}"), true),
                Staged::Codes(said) => {
                    let any = imp.item_file.contains(&f);
                    (said.clone(), any)
                }
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
                chosen: open_vault || (enabled && imp.to_inbox.contains(&file.name)),
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
                }
            }
            I::Later => self.sheet = None,
            I::Wallet(i) => {
                let Some(d) = self
                    .import_view()
                    .and_then(|v| v.wallets.get(i).map(|w| w.descriptor.clone()))
                else {
                    return;
                };
                if let Some(imp) = self.import.as_mut()
                    && !imp.skip_wallets.remove(&d)
                {
                    imp.skip_wallets.insert(d);
                }
            }
            I::Key(i) => {
                let Some(fp) = self
                    .import_view()
                    .and_then(|v| v.keys.get(i).map(|k| k.fingerprint.clone()))
                else {
                    return;
                };
                if let Some(imp) = self.import.as_mut()
                    && !imp.skip_keys.remove(&fp)
                {
                    imp.skip_keys.insert(fp);
                }
            }
            I::File(i) => {
                let Some(f) = self
                    .import_view()
                    .and_then(|v| v.files.get(i).filter(|f| f.enabled).cloned())
                else {
                    return;
                };
                if let Some(imp) = self.import.as_mut()
                    && !imp.to_inbox.remove(&f.name)
                {
                    imp.to_inbox.insert(f.name);
                }
            }
            I::Unlock(i) => {
                let Some(v) = self.import_view().and_then(|v| v.vaults.get(i).cloned()) else {
                    return;
                };
                if v.open.is_none() {
                    self.sheet = None;
                    self.vault_act(VaultAction::OpenFrom(v.file, Screen::Home));
                }
            }
            I::Go => self.import_go(),
        }
    }

    /// Imports what is chosen: the wallets with their keys, the keys on
    /// their own, the files for the Inbox; and wipes the rest.
    fn import_go(&mut self) {
        if !self.may_load_keys() {
            return;
        }
        let Some(view) = self.import_view() else {
            return;
        };
        let Some(imp) = self.import.take() else {
            return;
        };
        let (keys_before, wallets_before) = (self.session.keys.len(), self.session.wallets.len());
        let chosen: Vec<&ImportWallet> = view.wallets.iter().filter(|w| w.chosen).collect();
        let mut want: BTreeSet<String> = chosen.iter().flat_map(|w| w.keys.clone()).collect();
        want.extend(
            view.keys
                .iter()
                .filter(|k| k.chosen)
                .map(|k| k.fingerprint.clone()),
        );
        let descriptors: BTreeSet<&str> = chosen.iter().map(|w| w.descriptor.as_str()).collect();
        // The stick's own files, through the Inbox's loader.
        let skip: BTreeSet<String> = self
            .found_in(&imp.items)
            .wallets
            .iter()
            .filter(|w| !descriptors.contains(w.descriptor.as_str()))
            .map(|w| w.descriptor.clone())
            .collect();
        self.load_found_in(&imp.items, &skip, &|fp| want.contains(&fp_text(fp)));
        // The open vaults', through the vaults' own.
        for v in 0..self.vaults.open.len() {
            let open = &self.vaults.open[v];
            let mut set = BTreeSet::new();
            for (i, r) in open.contents.of(kind::WALLET) {
                let chosen = crate::wallet::read_wallet(r.text(field::WALLET).unwrap_or(""))
                    .is_ok_and(|p| descriptors.contains(crate::wallet::same_wallet(&p).as_str()));
                if chosen {
                    set.insert(i);
                }
            }
            for (i, r) in open.contents.of(kind::KEY) {
                if crate::vault_screens::key_fingerprint(self, r).is_some_and(|f| want.contains(&f))
                {
                    set.insert(i);
                }
            }
            if !set.is_empty() {
                self.vault_load_set(v, &set);
            }
        }
        // The files chosen, into the Inbox; a vault not chosen out of it.
        let ImportState {
            files,
            items,
            item_file,
            to_inbox,
            ..
        } = imp;
        let mut moved = 0;
        for f in &files {
            if f.state == Staged::Vault && !view.files.iter().any(|v| v.name == f.name && v.chosen)
            {
                self.inbox.retain(|i| i.name != f.name);
            }
        }
        let mut counted: BTreeSet<usize> = BTreeSet::new();
        for (item, f) in items.into_iter().zip(item_file) {
            let Some(file) = files.get(f) else {
                continue;
            };
            if !to_inbox.contains(&file.name) {
                continue;
            }
            let mut item = item;
            item.name = self.free_inbox_name(&item.name);
            self.inbox.push(item);
            if counted.insert(f) {
                moved += 1;
            }
        }
        moved += files
            .iter()
            .filter(|f| {
                f.state == Staged::Vault && view.files.iter().any(|v| v.name == f.name && v.chosen)
            })
            .count();
        self.save_boxes();
        self.refresh_spend();
        self.sheet = None;
        self.screen = Screen::Home;
        self.wallet = 0;
        let keys = self.session.keys.len() - keys_before;
        let wallets = self.session.wallets.len() - wallets_before;
        let count =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        self.toast(&format!(
            "Imported {} · {} · {}",
            count(wallets, "wallet", "wallets"),
            count(keys, "key", "keys"),
            count(moved, "file", "files")
        ));
    }
}

/// The press that opens the import sheet.
pub const OPEN: Action = Action::Import(ImportAction::Open);

/// The press that closes it for later.
pub const LATER: Action = Action::Import(ImportAction::Later);
