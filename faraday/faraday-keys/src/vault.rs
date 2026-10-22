//! The open vaults: each one's slot key and decrypted records
//! (`docs/KEYS.md` §4 step 3).
//!
//! A vault is opened here ([`OpenVaults::unlock`]) and sealed here
//! ([`OpenVaults::seal`]). The slot key that seals it again is private to
//! this module, and so is the vault file the other slots are copied from.
//! The caller holds a [`VaultId`], which tells open vaults apart and says
//! nothing else.
//!
//! The records stay readable to the caller through one accessor,
//! [`OpenVaults::vault_records`], so that faraday-core's flows go on as
//! they are. That accessor is temporary: step 3b replaces it with
//! operations and `reveal`, and the records become private.
//!
//! The store is generic over the records' type `R`. faraday-core keeps a
//! cache of what it has read of the records beside them (the wallet
//! descriptions), so its own type, which reads as [`Contents`], is what is
//! stored.

use std::collections::BTreeSet;
use std::ops::DerefMut;

use faraday_backup::oskb;
use faraday_bip::bip39::Mnemonic;
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::{Compute, Work};
use faraday_intake::wallet::Refusal;
use faraday_vault::records::{self, field, kind};
use faraday_vault::{self as fv, Contents, Header, Record};
use zeroize::Zeroizing;

use crate::identity::{Material, fingerprint};
use crate::session::{Held, Keys, Reveal};

/// The random bytes one KDBX database needs.
pub const KDBX_SEED_LEN: usize = faraday_backup::kdbx::SEED_LEN;

/// A recovery sheet's three parts, as they were sealed.
pub use faraday_backup::oskb::Sheet;

/// What the records of an open vault are held as: they read and change as
/// [`Contents`], and are made from it.
pub trait Records: DerefMut<Target = Contents> + From<Contents> {}

impl<T> Records for T where T: DerefMut<Target = Contents> + From<Contents> {}

/// Which open vault. Public: it tells vaults apart and opens nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VaultId(u32);

/// One open slot of one vault.
struct Slot<R> {
    id: VaultId,
    /// The file as it was opened: the other slots are copied from it.
    file: Vec<u8>,
    /// The slot and its key.
    opened: fv::Opened,
    /// The slot's decrypted records.
    records: R,
}

/// The open vaults. Each is held from its unlock until it is closed, with
/// its slot key; both are wiped when it goes.
///
/// Invariant: a [`VaultId`] this store has made names its slot until
/// [`OpenVaults::close`] or [`OpenVaults::clear`] drops it, and the slot's
/// header does not change.
pub struct OpenVaults<R> {
    slots: Vec<Slot<R>>,
    next: u32,
}

impl<R> Default for OpenVaults<R> {
    fn default() -> Self {
        OpenVaults {
            slots: Vec::new(),
            next: 0,
        }
    }
}

impl<R: Records> OpenVaults<R> {
    fn slot(&self, id: VaultId) -> Option<&Slot<R>> {
        self.slots.iter().find(|s| s.id == id)
    }

    /// Costs an Argon2id run over the vault's memory, as
    /// `faraday_vault::open` does. Opens the slot `passphrase` opens in
    /// `file` and keeps it, with the slot key and its records. Gives its
    /// id and the file's header, which is public. The passphrase is lent,
    /// and the caller wipes its copy.
    pub fn unlock(
        &mut self,
        c: &Compute,
        file: Vec<u8>,
        passphrase: &[u8],
    ) -> Result<(VaultId, Header), fv::Error> {
        let (opened, contents) = fv::open(c, &file, passphrase)?;
        let header = *opened.header();
        let id = VaultId(self.next);
        self.next += 1;
        self.slots.push(Slot {
            id,
            file,
            opened,
            records: R::from(contents),
        });
        Ok((id, header))
    }

    /// Closes open vault `id`: its slot key and records are wiped.
    pub fn close(&mut self, id: VaultId) {
        self.slots.retain(|s| s.id != id);
    }

    /// Closes every open vault.
    pub fn clear(&mut self) {
        self.slots.clear();
    }

    /// A lookup. Open vault `id` with its slot's records changed, sealed
    /// under a fresh `nonce`: the vault file, with every other slot copied
    /// from the file it was opened from. `None` where it does not seal
    /// (the contents do not fit, or the file is another's). The bytes are
    /// encrypted, so public.
    pub fn sealed(&self, id: VaultId, nonce: &[u8; fv::NONCE_LEN]) -> Option<Vec<u8>> {
        let s = self.slot(id)?;
        s.opened.seal(&s.file, &s.records, nonce).ok()
    }

    /// A lookup. Open vault `id`'s records.
    ///
    /// Temporary: step 3b of `docs/KEYS.md` removes this and the next,
    /// when faraday-core's reads and edits of the records become
    /// operations of this crate.
    ///
    /// Panics for an id this store has not made or has closed, which the
    /// caller's own list of open vaults rules out.
    pub fn vault_records(&self, id: VaultId) -> &R {
        match self.slot(id) {
            Some(s) => &s.records,
            None => unreachable!(),
        }
    }

    /// A lookup. Open vault `id`'s records, to change. Temporary, with
    /// [`OpenVaults::vault_records`].
    pub fn vault_records_mut(&mut self, id: VaultId) -> &mut R {
        match self.slots.iter_mut().find(|s| s.id == id) {
            Some(s) => &mut s.records,
            None => unreachable!(),
        }
    }

    /// Costs a parse of each key record's words in `set`, and what
    /// `add_words_with` and `add_seed` cost for each. Loads the key
    /// records of open vault `id` whose places `set` names into `keys`:
    /// the words (with the passphrase stored beside them, or one an open
    /// vault keeps alone for them) or the master seed. Returns the
    /// fingerprints of the keys that loaded; a key the store refuses is
    /// left out.
    pub fn load_from_vault(
        &self,
        c: &Compute,
        keys: &mut Keys,
        id: VaultId,
        set: &BTreeSet<usize>,
    ) -> Vec<Fingerprint> {
        let Some(slot) = self.slot(id) else {
            return Vec::new();
        };
        let mut words: Vec<(Zeroizing<String>, String)> = Vec::new();
        let mut seeds: Vec<(Zeroizing<Vec<u8>>, String)> = Vec::new();
        let mut passphrases: Vec<Zeroizing<String>> = Vec::new();
        for (i, r) in slot.records.of(kind::KEY) {
            if !set.contains(&i) {
                continue;
            }
            let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
            if let Some(m) = record_words(c, r) {
                // Stored with the key, or kept alone in an open vault.
                let pass = match r.text(field::KEY_PASSPHRASE) {
                    Some(p) => Zeroizing::new(p.to_string()),
                    None => self.passphrase_kept_for(c, keys, &m).unwrap_or_default(),
                };
                words.push((phrase_of(&m), label));
                passphrases.push(pass);
            } else if let Some(seed) = seed_of(r) {
                seeds.push((seed, label));
            }
        }
        let mut loaded = Vec::new();
        for ((w, label), p) in words.iter().zip(&passphrases) {
            if let Ok(fp) = keys.add_words_with(c, w, p, label, None) {
                loaded.push(fp);
            }
        }
        for (seed, label) in &seeds {
            if let Ok(fp) = keys.add_seed(c, seed, label) {
                loaded.push(fp);
            }
        }
        loaded
    }

    /// Costs a parse of the record's words, and what `add_mnemonic` or
    /// `add_seed` costs. Loads key record `at` of open vault `id` into
    /// `keys`. `None` where the record is no key record, or holds neither
    /// words nor a seed.
    pub fn load_key_record(
        &self,
        c: &Compute,
        keys: &mut Keys,
        id: VaultId,
        at: usize,
    ) -> Option<Result<Fingerprint, Refusal>> {
        let r = self.slot(id)?.records.records.get(at)?;
        if r.kind != kind::KEY {
            return None;
        }
        let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
        match record_words(c, r) {
            Some(m) => {
                // Stored with the key, or kept alone in an open vault
                // (`docs/BACKUP-FLOW.md` §17.12 item 4).
                let p = match r.text(field::KEY_PASSPHRASE) {
                    Some(p) => Zeroizing::new(p.to_string()),
                    None => self.passphrase_kept_for(c, keys, &m).unwrap_or_default(),
                };
                Some(keys.add_mnemonic(c, &m, &p, &label, None))
            }
            None => seed_of(r).map(|seed| keys.add_seed(c, &seed, &label)),
        }
    }

    /// Costs a fingerprint for each passphrase record. The passphrase an
    /// open vault keeps alone (a type 14 record,
    /// `docs/BACKUP-FLOW.md` §17.12 item 4) for the key the words `m`
    /// make with it, found by the fingerprint the record names.
    pub fn passphrase_kept_for(
        &self,
        c: &Compute,
        keys: &Keys,
        m: &Mnemonic,
    ) -> Option<Zeroizing<String>> {
        for slot in &self.slots {
            for (_, r) in slot.records.of(kind::PASSPHRASE) {
                let (Some(fp), Some(p)) =
                    (r.field(field::PASSPHRASE_KEY), r.text(field::PASSPHRASE))
                else {
                    continue;
                };
                let Some(got) = fingerprint(c, Material::Words(m), p, keys.network()) else {
                    continue;
                };
                if got.0[..] == *fp {
                    return Some(Zeroizing::new(p.to_string()));
                }
            }
        }
        None
    }
}

/// Costs a parse of the words. The words key record `r` holds, when it
/// holds words.
fn record_words(c: &Compute, r: &Record) -> Option<Mnemonic> {
    c.count(Work::WordsParse);
    records::words_of(r)
}

/// A mnemonic's words, space-separated, in wiped memory.
fn phrase_of(m: &Mnemonic) -> Zeroizing<String> {
    let lang = m.language();
    let mut out = faraday_intake::secret_buf::room();
    for (k, &i) in m.indices().iter().enumerate() {
        if k > 0 {
            out.push(' ');
        }
        out.push_str(lang.word(i));
    }
    out
}

/// The master seed a key record holds, when it holds a seed rather than
/// words. Temporary, with [`OpenVaults::vault_records`]: the seed is a
/// secret lent to faraday-core's fingerprinting of key records.
pub fn seed_of(r: &Record) -> Option<Zeroizing<Vec<u8>>> {
    match oskb::payload_of(oskb::KIND_SEED, r.field(field::KEY)?)? {
        oskb::Opened::Seed(s) => Some(s),
        _ => None,
    }
}

/// A wallet recovery sheet record's descriptor, name and note as text.
/// Empty for a record that holds none.
pub fn sheet_parts(r: &Record) -> (String, String, String) {
    match r
        .field(field::SHEET)
        .and_then(|p| oskb::payload_of(oskb::KIND_SHEET, p))
    {
        Some(oskb::Opened::Sheet(s)) => (
            String::from_utf8_lossy(&s.descriptor).into_owned(),
            String::from_utf8_lossy(&s.name).into_owned(),
            String::from_utf8_lossy(&s.note).into_owned(),
        ),
        _ => Default::default(),
    }
}

/// Costs what `faraday_vault::create` costs (an Argon2id run per
/// passphrase). Makes a vault: one to four passphrases, each with the
/// contents of its own slot, at `cost` with slots of `slot_len`, with
/// everything random drawn from `seed`. The file is encrypted, so public.
pub fn create(
    c: &Compute,
    slot_len: u32,
    cost: fv::Cost,
    passphrases: &[&[u8]],
    contents: &[Contents],
    seed: &[u8; 32],
) -> Result<Vec<u8>, fv::Error> {
    fv::create(c, slot_len, cost, passphrases, contents, seed)
}

/// Costs an Argon2id run at `header`'s cost, thrown away: the time a
/// vault's unlock takes on this computer, measured.
pub fn measure(c: &Compute, header: &Header) {
    let _ = fv::derive(c, header, b"measure");
}

/// What a `.oskb` backup held, made into a vault record.
pub struct BackupRecord {
    /// The record: a key (words or a seed), a note or a sheet.
    pub record: Held<Record>,
    /// It is a key record, not a note or a sheet.
    pub key: bool,
}

/// Costs an Argon2id run (through `open_payload`) and what a key's words
/// cost to write. Opens the `.oskb` backup `bytes` under `passphrase` and
/// makes what it held into a vault record, a key labelled `label` or a
/// note or a sheet. `Ok(None)` for a sheet that cannot be made a record.
/// The record is key material (`Reveal::VaultEntry`), so it comes back
/// held.
pub fn open_backup(
    c: &Compute,
    bytes: &[u8],
    passphrase: &[u8],
    label: &str,
    _why: Reveal,
) -> Result<Option<BackupRecord>, oskb::Error> {
    let (record, key) = match oskb::open_payload(c, bytes, passphrase)? {
        oskb::Opened::Words(m) => (
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&m))
                .with(field::KEY_LABEL, label.as_bytes()),
            true,
        ),
        oskb::Opened::Seed(seed) => {
            let mut payload = Zeroizing::new(vec![0u8; 1 + oskb::MAX_SEED]);
            payload[0] = seed.len() as u8;
            payload[1..1 + seed.len()].copy_from_slice(&seed);
            (
                Record::new(kind::KEY)
                    .with(field::KEY, &payload)
                    .with(field::KEY_LABEL, label.as_bytes()),
                true,
            )
        }
        oskb::Opened::Note(text) => (Record::new(kind::NOTE).with(field::NOTE, &text), false),
        oskb::Opened::Sheet(sheet) => {
            match oskb::sheet_payload(&sheet.descriptor, &sheet.name, &sheet.note) {
                Some(payload) => (Record::new(kind::SHEET).with(field::SHEET, &payload), false),
                None => return Ok(None),
            }
        }
    };
    Ok(Some(BackupRecord {
        record: Held::of(record),
        key,
    }))
}

/// What a `.oskb` backup held, when it is loaded into the session.
pub enum Added {
    /// A key's words or a master seed: added to the keys, or refused.
    Key(Result<Fingerprint, Refusal>),
    /// A wallet's recovery sheet: its parts, for the wallets to add.
    Sheet(Sheet),
    /// A note, which loads nothing.
    Note,
}

impl Keys {
    /// Costs an Argon2id run (through `open_payload`) and what
    /// `add_words` or `add_seed` costs. Opens the `.oskb` backup `bytes`
    /// under `passphrase` and adds the key it holds, labelled `label`. A
    /// sheet is handed back for the wallets, and a note loads nothing.
    pub fn add_backup(
        &mut self,
        c: &Compute,
        bytes: &[u8],
        passphrase: &[u8],
        label: &str,
    ) -> Result<Added, oskb::Error> {
        Ok(match oskb::open_payload(c, bytes, passphrase)? {
            oskb::Opened::Words(m) => Added::Key(self.add_words(c, &phrase_of(&m), label, None)),
            oskb::Opened::Seed(s) => Added::Key(self.add_seed(c, &s, label)),
            oskb::Opened::Sheet(sheet) => Added::Sheet(sheet),
            oskb::Opened::Note(_) => Added::Note,
        })
    }
}

/// Costs a KDBX 4 database's Argon2id run. A KeePass database holding one
/// entry (`title`, `notes` and the secret `password`), sealed under
/// `passphrase`, with its randomness from `seed`. The bytes are
/// encrypted, so public; `None` where the entry does not fit.
pub fn write_kdbx(
    c: &Compute,
    title: &[u8],
    notes: &[u8],
    password: &[u8],
    passphrase: &[u8],
    seed: &[u8; KDBX_SEED_LEN],
) -> Option<Vec<u8>> {
    let item = faraday_backup::kdbx::Item {
        title,
        notes,
        secret: password,
    };
    faraday_backup::kdbx::write(c, &[item], passphrase, faraday_backup::DEVICE_PARAMS, seed)
}
