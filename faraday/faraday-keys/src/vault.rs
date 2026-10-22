//! The open vaults: each one's slot key and decrypted records
//! (`docs/KEYS.md` §4 step 3).
//!
//! A vault is opened here ([`OpenVaults::unlock`]) and sealed here
//! ([`OpenVaults::sealed`]). The slot key that seals it again is private to
//! this module, and so is the vault file the other slots are copied from.
//! The caller holds a [`VaultId`], which tells open vaults apart and says
//! nothing else.
//!
//! **What the caller sees of the records.** Every record field is public
//! or secret, by [`SECRET_FIELDS`]. The caller reads a record through a
//! [`RecordView`]: its public fields ([`RecordView::public`]), whether a
//! secret field is there ([`RecordView::has`]) and how long it is
//! ([`RecordView::len`]), never its bytes. A secret leaves only through
//! [`OpenVaults::reveal`] or an operation that takes its [`Reveal`]
//! reason. A record changes only through the edits of [`OpenVaults`]; a
//! secret field is written only from input the caller already holds
//! ([`OpenVaults::set_secret`], [`OpenVaults::set_fields`]) or copied
//! inside this crate ([`OpenVaults::copy_record`]).
//!
//! The store keeps a change counter per vault ([`OpenVaults::version`]),
//! which moves with every edit, so that what the caller keeps of a vault's
//! public records can be told to be in step.

use std::cell::OnceCell;
use std::collections::BTreeSet;

use faraday_backup::oskb;
use faraday_bip::bip39::Mnemonic;
use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::keys::{Fingerprint, Network};
use faraday_crypto::work::{Compute, Work};
use faraday_intake::wallet::Refusal;
use faraday_vault::records::{self, field, kind};
use faraday_vault::{self as fv, Contents, Header, Record};
use zeroize::Zeroizing;

use crate::identity::{Material, fingerprint};
use crate::session::{Held, Keys, Part, Reveal, Revealed, WordsId, words_id};

/// The random bytes one KDBX database needs.
pub const KDBX_SEED_LEN: usize = faraday_backup::kdbx::SEED_LEN;

/// A recovery sheet's three parts, as they were sealed.
pub use faraday_backup::oskb::Sheet;

/// The fields of a GPG record (type 7), by number
/// (`docs/VAULT.md` §7).
pub mod gpg_field {
    /// The primary key's seed. Secret.
    pub const PRIMARY: u8 = 1;
    /// When the primary key was made (`u32`).
    pub const CREATED: u8 = 2;
    /// The signing subkey's seed. Secret.
    pub const SUBKEY: u8 = 3;
    /// When the subkey was made (`u32`).
    pub const SUBKEY_CREATED: u8 = 4;
    /// A user ID (repeatable).
    pub const USER_ID: u8 = 5;
    /// The expiry, seconds after creation (`u32`).
    pub const EXPIRY: u8 = 6;
    /// The public certificate, as packets.
    pub const CERTIFICATE: u8 = 7;
    /// The revocation certificate made with the key, as packets.
    pub const REVOCATION: u8 = 8;
    /// When the key was marked revoked (`u32`).
    pub const REVOKED: u8 = 9;
}

/// The fields of a Secure Boot record (type 8), by number.
pub mod sb_field {
    /// The owner GUID.
    pub const OWNER: u8 = 1;
    /// The PK's private key. Secret.
    pub const PK_KEY: u8 = 2;
    /// The PK's certificate.
    pub const PK_CERT: u8 = 3;
    /// The KEK's private key. Secret.
    pub const KEK_KEY: u8 = 4;
    /// The KEK's certificate.
    pub const KEK_CERT: u8 = 5;
    /// The db's private key. Secret.
    pub const DB_KEY: u8 = 6;
    /// The db's certificate.
    pub const DB_CERT: u8 = 7;
}

/// Which record fields are secret: the one table the whole workspace
/// reads. A field not listed is public. Each entry is a record type and a
/// field number (`docs/VAULT.md` §7):
///
/// - KEY: the payload (words or seed) and the BIP-39 passphrase;
/// - PASSPHRASE: the passphrase;
/// - NOTE: the text;
/// - ENTRY: the password, the TOTP secret and the notes;
/// - APK_KEY: the private key;
/// - ROUND: the round's state (secret nonces);
/// - GPG: the primary key's and the subkey's seeds;
/// - SECURE_BOOT: the PK's, KEK's and db's private keys.
///
/// Public, in full: the slot label, wallets, sheets, amounts, plans and
/// wallet shares; a key's label and flags; a passphrase's key
/// fingerprint; an entry's title, username and URL; an APK key's
/// certificate and package; a round's scheme, transaction hash, key
/// fingerprint and used flag; a GPG key's dates, user IDs, certificate,
/// revocation certificate and revoked date; a Secure Boot record's owner
/// and certificates.
pub const SECRET_FIELDS: &[(u8, u8)] = &[
    (kind::KEY, field::KEY),
    (kind::KEY, field::KEY_PASSPHRASE),
    (kind::PASSPHRASE, field::PASSPHRASE),
    (kind::NOTE, field::NOTE),
    (kind::ENTRY, field::PASSWORD),
    (kind::ENTRY, field::NOTES),
    (kind::ENTRY, field::TOTP),
    (kind::APK_KEY, field::APK_KEY),
    (kind::ROUND, field::ROUND_STATE),
    (kind::GPG, gpg_field::PRIMARY),
    (kind::GPG, gpg_field::SUBKEY),
    (kind::SECURE_BOOT, sb_field::PK_KEY),
    (kind::SECURE_BOOT, sb_field::KEK_KEY),
    (kind::SECURE_BOOT, sb_field::DB_KEY),
];

/// Whether field `field` of a record of type `kind` is secret
/// ([`SECRET_FIELDS`]).
pub fn is_secret(kind: u8, field: u8) -> bool {
    SECRET_FIELDS.contains(&(kind, field))
}

/// Which open vault. Public: it tells vaults apart and opens nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VaultId(u32);

/// A record, as the caller may read it: its public fields, and whether a
/// secret field is there and how long. Cheap to copy. It borrows the
/// record (in an open vault, or one held beside it), so it cannot outlive
/// an edit.
#[derive(Clone, Copy)]
pub struct RecordView<'a> {
    record: &'a Record,
    home: Option<(VaultId, usize)>,
}

impl<'a> RecordView<'a> {
    /// A view of a record that is in no vault: one the caller built, or
    /// one held beside the vaults.
    pub fn of(record: &'a Record) -> RecordView<'a> {
        RecordView { record, home: None }
    }

    /// The record's type (`kind`).
    pub fn kind(&self) -> u8 {
        self.record.kind
    }

    /// Where the record is, when it is an open vault's own: the vault and
    /// its place there.
    pub fn home(&self) -> Option<(VaultId, usize)> {
        self.home
    }

    /// Public field `number`'s bytes; `None` where the record has no such
    /// field, and for a secret field ([`is_secret`]).
    pub fn public(&self, number: u8) -> Option<&'a [u8]> {
        if is_secret(self.record.kind, number) {
            return None;
        }
        self.record.field(number)
    }

    /// Public field `number` as text, when it reads as UTF-8.
    pub fn public_text(&self, number: u8) -> Option<&'a str> {
        self.public(number)
            .and_then(|b| std::str::from_utf8(b).ok())
    }

    /// Every public field with this number, in order (for the repeatable
    /// ones). Empty for a secret field.
    pub fn public_all(&self, number: u8) -> impl Iterator<Item = &'a [u8]> + use<'a> {
        let secret = is_secret(self.record.kind, number);
        self.record
            .fields
            .iter()
            .filter(move |f| !secret && f.number == number)
            .map(|f| f.bytes.as_slice())
    }

    /// Whether the record has field `number`, secret or not.
    pub fn has(&self, number: u8) -> bool {
        self.record.field(number).is_some()
    }

    /// How many bytes field `number` has, secret or not.
    pub fn len(&self, number: u8) -> Option<usize> {
        self.record.field(number).map(<[u8]>::len)
    }

    /// Whether a key or wallet record starts ticked on the Load sheet an
    /// unlock shows.
    pub fn loads_at_unlock(&self) -> bool {
        records::loads_at_unlock(self.record)
    }

    /// The bytes the record takes in a slot.
    pub fn size(&self) -> usize {
        self.record.size()
    }

    /// A copy of the record, when it has no secret field: a plan, a
    /// wallet, a share. `None` for any other.
    pub fn to_public(&self) -> Option<Record> {
        self.record
            .fields
            .iter()
            .all(|f| !is_secret(self.record.kind, f.number))
            .then(|| self.record.clone())
    }

    /// Whether the record has exactly the fields of `other`, in order,
    /// the same bytes in each: the record as it would be written. For
    /// records of a type with no secret field.
    pub fn same_fields(&self, other: &Record) -> bool {
        self.record.kind == other.kind
            && self.record.fields.len() == other.fields.len()
            && self
                .record
                .fields
                .iter()
                .zip(&other.fields)
                .all(|(a, b)| a.number == b.number && *a.bytes == *b.bytes)
    }

    /// The record itself, for the operations of this crate.
    pub(crate) fn raw(&self) -> &'a Record {
        self.record
    }
}

impl Held<Record> {
    /// A view of the held record: its public fields.
    pub fn view(&self) -> RecordView<'_> {
        RecordView::of(self)
    }
}

/// What can be made a record of an open vault: a record built from input
/// in hand, or one an operation of this crate made and holds.
pub trait IntoRecord {
    /// The record.
    fn into_record(self) -> Record;
}

impl IntoRecord for Record {
    fn into_record(self) -> Record {
        self
    }
}

impl IntoRecord for Held<Record> {
    fn into_record(self) -> Record {
        self.release()
    }
}

/// A record waiting to go into a vault: built by the caller from public
/// values, or copied out of an open vault by this crate and held here
/// (its secrets are never the caller's to read).
pub enum Staged {
    /// A record the caller built.
    Made(Record),
    /// A record copied out of a vault.
    Held(Held<Record>),
}

impl Staged {
    /// The record's public fields.
    pub fn view(&self) -> RecordView<'_> {
        match self {
            Staged::Made(r) => RecordView::of(r),
            Staged::Held(h) => h.view(),
        }
    }

    /// The bytes the record takes in a slot.
    pub fn size(&self) -> usize {
        self.view().size()
    }
}

impl IntoRecord for Staged {
    fn into_record(self) -> Record {
        match self {
            Staged::Made(r) => r,
            Staged::Held(h) => h.release(),
        }
    }
}

/// An open vault's records, as the caller may read them. Cheap to copy.
#[derive(Clone, Copy)]
pub struct Records<'a> {
    id: VaultId,
    contents: &'a Contents,
}

static NO_RECORDS: Contents = Contents {
    records: Vec::new(),
};

impl<'a> Records<'a> {
    /// How many records.
    pub fn len(&self) -> usize {
        self.contents.records.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.contents.records.is_empty()
    }

    /// Record `i`.
    pub fn get(&self, i: usize) -> Option<RecordView<'a>> {
        let id = self.id;
        self.contents.records.get(i).map(|record| RecordView {
            record,
            home: Some((id, i)),
        })
    }

    /// Every record with its place.
    pub fn iter(&self) -> impl Iterator<Item = (usize, RecordView<'a>)> + use<'a> {
        let id = self.id;
        self.contents
            .records
            .iter()
            .enumerate()
            .map(move |(i, record)| {
                (
                    i,
                    RecordView {
                        record,
                        home: Some((id, i)),
                    },
                )
            })
    }

    /// The records of one type, with their places.
    pub fn of(&self, k: u8) -> impl Iterator<Item = (usize, RecordView<'a>)> + use<'a> {
        self.iter().filter(move |(_, r)| r.kind() == k)
    }

    /// The bytes the records take in a slot, with the format byte and the
    /// end record.
    pub fn used(&self) -> usize {
        self.contents.used()
    }
}

/// What is worked out of one key record's words, once per change: how
/// many words, and what tells them from other words ([`WordsId`]).
struct Fact {
    words: usize,
    id: WordsId,
}

/// One open slot of one vault.
struct Slot {
    id: VaultId,
    /// The file as it was opened: the other slots are copied from it.
    file: Vec<u8>,
    /// The slot and its key.
    opened: fv::Opened,
    /// The slot's decrypted records.
    records: Contents,
    /// Moves with every edit of the records.
    version: u64,
    /// What is worked out of each key record's words, by record place;
    /// dropped by every edit.
    facts: OnceCell<Vec<Option<Fact>>>,
}

impl Slot {
    /// Every edit ends here: the version moves and what was worked out of
    /// the records is dropped.
    fn changed(&mut self) {
        self.version += 1;
        self.facts = OnceCell::new();
    }

    /// Costs a parse of each key record's words and a hash of each, the
    /// first time after a change.
    fn facts(&self, c: &Compute) -> &Vec<Option<Fact>> {
        self.facts.get_or_init(|| {
            self.records
                .records
                .iter()
                .map(|r| {
                    let m = (r.kind == kind::KEY)
                        .then(|| record_words(c, r))
                        .flatten()?;
                    Some(Fact {
                        words: m.word_count(),
                        id: words_id(c, &m),
                    })
                })
                .collect()
        })
    }
}

/// The open vaults. Each is held from its unlock until it is closed, with
/// its slot key; both are wiped when it goes.
///
/// Invariant: a [`VaultId`] this store has made names its slot until
/// [`OpenVaults::close`] or [`OpenVaults::clear`] drops it, and the slot's
/// header does not change. A slot's [`OpenVaults::version`] moves with
/// every edit of its records, and only then; the facts worked out of the
/// key records' words are dropped with it.
#[derive(Default)]
pub struct OpenVaults {
    slots: Vec<Slot>,
    next: u32,
}

/// The vault does not have room for an edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Full;

impl OpenVaults {
    fn slot(&self, id: VaultId) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }

    fn slot_mut(&mut self, id: VaultId) -> Option<&mut Slot> {
        self.slots.iter_mut().find(|s| s.id == id)
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
            records: contents,
            version: 0,
            facts: OnceCell::new(),
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

    // ---- reading ----

    /// A lookup. Open vault `id`'s records, for reading. Empty for an id
    /// this store has not made or has closed.
    pub fn records(&self, id: VaultId) -> Records<'_> {
        Records {
            id,
            contents: self.slot(id).map_or(&NO_RECORDS, |s| &s.records),
        }
    }

    /// A lookup. How many edits open vault `id`'s records have had since
    /// it was unlocked: what a caller's record of the vault is kept in
    /// step by. Zero for a vault this store does not hold.
    pub fn version(&self, id: VaultId) -> u64 {
        self.slot(id).map_or(0, |s| s.version)
    }

    /// A lookup. Whether open vault `id` still has room after records
    /// `remove` (by place) are taken out and records of the sizes `add`
    /// are put in.
    pub fn fits(&self, id: VaultId, remove: &[usize], add: &[usize]) -> bool {
        let Some(s) = self.slot(id) else {
            return false;
        };
        let mut gone: Vec<usize> = remove.to_vec();
        gone.sort_unstable();
        gone.dedup();
        let freed: usize = gone
            .iter()
            .filter_map(|&i| s.records.records.get(i))
            .map(Record::size)
            .sum();
        s.records.used() - freed + add.iter().sum::<usize>() <= s.opened.header().slot_len as usize
    }

    // ---- edits ----

    /// Puts `record` last in open vault `id`; its place. `None` for a
    /// vault this store does not hold.
    pub fn insert(&mut self, id: VaultId, record: impl IntoRecord) -> Option<usize> {
        let s = self.slot_mut(id)?;
        s.records.records.push(record.into_record());
        s.changed();
        Some(s.records.records.len() - 1)
    }

    /// Puts `record` at place `at` in open vault `id`, the records from
    /// there on moving up.
    pub fn insert_at(&mut self, id: VaultId, at: usize, record: impl IntoRecord) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let at = at.min(s.records.records.len());
        s.records.records.insert(at, record.into_record());
        s.changed();
        true
    }

    /// Puts `record` in place of record `i` of open vault `id`.
    pub fn replace(&mut self, id: VaultId, i: usize, record: impl IntoRecord) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.records.get_mut(i) else {
            return false;
        };
        *r = record.into_record();
        s.changed();
        true
    }

    /// Takes record `i` out of open vault `id`.
    pub fn remove(&mut self, id: VaultId, i: usize) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        if i >= s.records.records.len() {
            return false;
        }
        s.records.records.remove(i);
        s.changed();
        true
    }

    /// Takes out of open vault `id` every record `gone` says goes; how
    /// many went.
    pub fn remove_where(
        &mut self,
        id: VaultId,
        mut gone: impl FnMut(RecordView<'_>) -> bool,
    ) -> usize {
        let Some(s) = self.slot_mut(id) else {
            return 0;
        };
        let before = s.records.records.len();
        let mut at = 0;
        s.records.records.retain(|record| {
            let view = RecordView {
                record,
                home: Some((id, at)),
            };
            at += 1;
            !gone(view)
        });
        let n = before - s.records.records.len();
        if n > 0 {
            s.changed();
        }
        n
    }

    /// Sets public field `number` of record `i` of open vault `id`; an
    /// empty `bytes` takes the field out. `false` for a secret field
    /// ([`is_secret`]), which only [`OpenVaults::set_secret`] writes, or
    /// no such record.
    pub fn set_public(&mut self, id: VaultId, i: usize, number: u8, bytes: &[u8]) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.records.get_mut(i) else {
            return false;
        };
        if is_secret(r.kind, number) {
            return false;
        }
        r.set(number, bytes);
        s.changed();
        true
    }

    /// Sets a repeatable public field of record `i` of open vault `id` to
    /// `values`, in order, the values it had gone. `false` for a secret
    /// field or no such record.
    pub fn set_public_all(&mut self, id: VaultId, i: usize, number: u8, values: &[&[u8]]) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.records.get_mut(i) else {
            return false;
        };
        if is_secret(r.kind, number) {
            return false;
        }
        r.fields.retain(|f| f.number != number);
        for v in values {
            r.push(number, v);
        }
        s.changed();
        true
    }

    /// Sets a repeatable public field of every record in `at` of open
    /// vault `id` to `values`, as [`OpenVaults::set_public_all`] does; or,
    /// when the vault would then not fit its slot, none of them
    /// ([`Full`]).
    pub fn set_public_all_in(
        &mut self,
        id: VaultId,
        at: &[usize],
        number: u8,
        values: &[&[u8]],
    ) -> Result<(), Full> {
        let s = self.slot_mut(id).ok_or(Full)?;
        let mut grown = s.records.clone();
        for &i in at {
            let r = grown.records.get_mut(i).ok_or(Full)?;
            if is_secret(r.kind, number) {
                return Err(Full);
            }
            r.fields.retain(|f| f.number != number);
            for v in values {
                r.push(number, v);
            }
        }
        if grown.used() > s.opened.header().slot_len as usize {
            return Err(Full);
        }
        s.records = grown;
        s.changed();
        Ok(())
    }

    /// Sets secret field `number` of record `i` of open vault `id` to
    /// input the caller holds (a typed password), which the caller wipes
    /// when it drops; an empty `bytes` takes the field out. `false` for a
    /// public field or no such record.
    pub fn set_secret(
        &mut self,
        id: VaultId,
        i: usize,
        number: u8,
        bytes: &Zeroizing<Vec<u8>>,
    ) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.records.get_mut(i) else {
            return false;
        };
        if !is_secret(r.kind, number) {
            return false;
        }
        r.set(number, bytes);
        s.changed();
        true
    }

    /// Sets whether key or wallet record `i` of open vault `id` loads at
    /// unlock, keeping its other flags.
    pub fn set_loads_at_unlock(&mut self, id: VaultId, i: usize, loads: bool) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.records.get_mut(i) else {
            return false;
        };
        if !matches!(r.kind, kind::KEY | kind::WALLET) {
            return false;
        }
        records::set_loads_at_unlock(r, loads);
        s.changed();
        true
    }

    /// Writes a form into open vault `id`: record `at` (or, with `None`
    /// or a place with no record, a new record of type `kind`) gets each
    /// of `fields`, a field with no bytes taken out, as the caller typed
    /// them. A record that does not fit afterwards is left as it was.
    /// Gives the record's place.
    pub fn set_fields(
        &mut self,
        id: VaultId,
        at: Option<usize>,
        k: u8,
        fields: &[(u8, &[u8])],
    ) -> Result<usize, Full> {
        let s = self.slot_mut(id).ok_or(Full)?;
        let slot_len = s.opened.header().slot_len as usize;
        let place = at.filter(|&i| i < s.records.records.len());
        let mut record = match place {
            Some(i) => s.records.records[i].clone(),
            None => Record::new(k),
        };
        for (n, bytes) in fields {
            record.set(*n, bytes);
        }
        let old = place.map_or(0, |i| s.records.records[i].size());
        if s.records.used() - old + record.size() > slot_len {
            return Err(Full);
        }
        let i = match place {
            Some(i) => {
                s.records.records[i] = record;
                i
            }
            None => {
                s.records.records.push(record);
                s.records.records.len() - 1
            }
        };
        s.changed();
        Ok(i)
    }

    /// Copies record `i` of open vault `from` into open vault `to`, last;
    /// its place there. With `without`, the copy lacks that field (a
    /// key's stored passphrase). The secrets are copied here, and are
    /// never in the caller's hands.
    pub fn copy_record(
        &mut self,
        from: VaultId,
        i: usize,
        to: VaultId,
        without: Option<u8>,
    ) -> Option<usize> {
        let held = self.copy_out(from, i, without)?;
        self.insert(to, held)
    }

    /// A copy of record `i` of open vault `id`, held until it is dropped:
    /// readable as a [`RecordView`], and put into a vault with
    /// [`OpenVaults::insert`]. With `without`, the copy lacks that field.
    pub fn copy_out(&self, id: VaultId, i: usize, without: Option<u8>) -> Option<Held<Record>> {
        let mut r = self.slot(id)?.records.records.get(i)?.clone();
        if let Some(n) = without {
            r.fields.retain(|f| f.number != n);
        }
        Some(Held::of(r))
    }

    /// A copy of record `i` of open vault `id`, staged to go into a vault
    /// ([`OpenVaults::insert`]); with `without`, lacking that field.
    pub fn stage(&self, id: VaultId, i: usize, without: Option<u8>) -> Option<Staged> {
        self.copy_out(id, i, without).map(Staged::Held)
    }

    /// A passphrase record (type 14) for the key whose fingerprint is `fp`,
    /// made from the passphrase key record `i` of open vault `id` stores,
    /// staged to go into a vault. `None` for a record that stores none.
    pub fn stage_passphrase(&self, id: VaultId, i: usize, fp: &[u8; 4]) -> Option<Staged> {
        let r = self.slot(id)?.records.records.get(i)?;
        if r.kind != kind::KEY {
            return None;
        }
        let p = r.field(field::KEY_PASSPHRASE)?;
        Some(Staged::Held(Held::of(
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, fp)
                .with(field::PASSPHRASE, p),
        )))
    }

    // ---- facts worked out of the key records' words ----

    /// Costs a parse and a hash of every key record's words in open vault
    /// `id`, the first time after a change, a lookup after. Works out what
    /// `word_count` and `words_id` answer.
    pub fn warm(&self, c: &Compute, id: VaultId) {
        if let Some(s) = self.slot(id) {
            s.facts(c);
        }
    }

    /// Costs what `warm` costs. How many words key
    /// record `i` has; `None` for another record, or a key record that
    /// holds a seed rather than words.
    pub fn word_count(&self, c: &Compute, id: VaultId, i: usize) -> Option<usize> {
        self.slot(id)?.facts(c).get(i)?.as_ref().map(|f| f.words)
    }

    /// Costs what `word_count` costs. What tells key record `i`'s words
    /// from other words ([`WordsId`]); `None` as for `word_count`. A
    /// lookup once the vault's key records are read.
    pub fn words_id(&self, c: &Compute, id: VaultId, i: usize) -> Option<&[u8; 32]> {
        self.slot(id)?.facts(c).get(i)?.as_ref().map(|f| &*f.id)
    }

    /// Costs what `word_count` costs for a record in an open vault, a
    /// parse of the words for any other. How many words key record `r`'s
    /// seed has.
    pub fn record_word_count(&self, c: &Compute, r: RecordView<'_>) -> Option<usize> {
        match r.home() {
            Some((id, i)) if self.slot(id).is_some() => self.word_count(c, id, i),
            _ => record_words(c, r.raw()).map(|m| m.word_count()),
        }
    }

    /// Costs what `words_id` costs for a record in an open vault, a parse
    /// of the words and a hash for any other. Key record `r`'s words'
    /// [`WordsId`].
    pub fn record_words_id(&self, c: &Compute, r: RecordView<'_>) -> Option<WordsId> {
        match r.home() {
            Some((id, i)) if self.slot(id).is_some() => {
                self.words_id(c, id, i).map(|x| Zeroizing::new(*x))
            }
            _ => record_words(c, r.raw()).map(|m| words_id(c, &m)),
        }
    }

    // ---- key records and the session ----

    /// Costs what `record_words_id` and `Keys::words_id` cost. Whether key
    /// record `r` holds the words of session key `key`, from whichever
    /// list: told by the hash of each one's entropy, all 32 bytes.
    pub fn same_words(&self, c: &Compute, keys: &Keys, r: RecordView<'_>, key: usize) -> bool {
        keys.words_id(c, key)
            .is_some_and(|id| self.record_words_id(c, r).is_some_and(|x| *x == *id))
    }

    /// Whether key record `r` stores the BIP-39 passphrase session key `k`
    /// has loaded with.
    pub fn stores_passphrase_of(&self, r: RecordView<'_>, keys: &Keys, k: usize) -> bool {
        r.raw()
            .text(field::KEY_PASSPHRASE)
            .is_some_and(|p| keys.passphrase_is(k, p))
    }

    /// Whether passphrase record `r` (type 14) holds the passphrase of
    /// session key `k`; a record with none holds that of a key with none.
    pub fn passphrase_record_is(&self, r: RecordView<'_>, keys: &Keys, k: usize) -> bool {
        match r.raw().text(field::PASSPHRASE) {
            Some(t) => keys.passphrase_is(k, t),
            None => keys.info(k).is_some_and(|key| !key.has_passphrase),
        }
    }

    /// Costs a parse of each open vault's key records' words (see
    /// `word_count`). Whether any key record of any open vault holds the
    /// words of session key `i`, as the payload a key record keeps.
    pub fn holds_session_words(&self, c: &Compute, keys: &Keys, i: usize) -> bool {
        let held: Vec<&[u8]> = self
            .slots
            .iter()
            .flat_map(|s| s.records.of(kind::KEY))
            .filter_map(|(_, r)| r.field(field::KEY))
            .collect();
        keys.words_payload_among(c, i, &held)
    }

    /// Costs a parse of the record's words and, with `accept`, a PBKDF2
    /// and a master key for the fingerprint, then what `add_mnemonic`
    /// costs. Loads key record `at` of open vault `id` into `keys` with
    /// `passphrase`, in place of the one stored with it. With `accept`,
    /// the key it gives is loaded only when `accept` takes its
    /// fingerprint; otherwise the fingerprint is handed back
    /// ([`KeyLoad::Refused`]) and nothing loads. `None` where the record
    /// holds no words.
    pub fn load_key_with_passphrase(
        &self,
        c: &Compute,
        keys: &mut Keys,
        at: (VaultId, usize),
        passphrase: &str,
        accept: Option<&dyn Fn(Fingerprint) -> bool>,
    ) -> Option<KeyLoad> {
        let r = self.slot(at.0)?.records.records.get(at.1)?;
        if r.kind != kind::KEY {
            return None;
        }
        let m = record_words(c, r)?;
        if let Some(accept) = accept {
            let fp = fingerprint(c, Material::Words(&m), passphrase, keys.network())?;
            if !accept(fp) {
                return Some(KeyLoad::Refused(fp));
            }
        }
        let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
        Some(KeyLoad::Done(
            keys.add_mnemonic(c, &m, passphrase, &label, None),
        ))
    }

    /// Costs a parse of the words. Key record `r`'s words, held until
    /// they are dropped, for the reason `why` (a SeedQR picture, a file to
    /// a stick, the words on screen). `None` for a record that holds a
    /// seed rather than words.
    pub fn words_of(&self, c: &Compute, r: RecordView<'_>, _why: Reveal) -> Option<Held<Mnemonic>> {
        record_words(c, r.raw()).map(Held::of)
    }

    /// Costs what `words_of` costs for words. Field `number` of record `i`
    /// of open vault `id`, lent while the guard lives, for the reason
    /// `why`: a key's words (a parse of them) or stored passphrase, a
    /// passphrase record's passphrase, a note's text, an entry's
    /// password, TOTP secret or notes. Empty where the field is not
    /// secret, is not there, or `why` names another part.
    pub fn reveal(
        &self,
        c: &Compute,
        id: VaultId,
        i: usize,
        number: u8,
        why: Reveal,
    ) -> Revealed<'_> {
        let Some(r) = self.slot(id).and_then(|s| s.records.records.get(i)) else {
            return Revealed::lent("");
        };
        if !is_secret(r.kind, number) {
            return Revealed::lent("");
        }
        let part = why.part();
        let lent = |p: Part| -> &str {
            if part == p {
                r.text(number).unwrap_or("")
            } else {
                ""
            }
        };
        match (r.kind, number) {
            (kind::KEY, field::KEY) if matches!(part, Part::Words | Part::SeedQr) => {
                match record_words(c, r) {
                    Some(m) => Revealed::made(phrase_of(&m)),
                    None => Revealed::lent(""),
                }
            }
            (kind::KEY, field::KEY_PASSPHRASE) | (kind::PASSPHRASE, field::PASSPHRASE) => {
                Revealed::lent(lent(Part::Passphrase))
            }
            (kind::NOTE, field::NOTE) => Revealed::lent(lent(Part::Note)),
            (kind::ENTRY, field::PASSWORD) => Revealed::lent(lent(Part::Password)),
            (kind::ENTRY, field::NOTES) => Revealed::lent(lent(Part::EntryNotes)),
            (kind::ENTRY, field::TOTP) => Revealed::lent(lent(Part::Totp)),
            _ => Revealed::lent(""),
        }
    }

    // ---- entries ----

    /// Cheap: one HMAC. The code entry `i` of open vault `id` gives at
    /// `now`, and the account its TOTP secret's label names; `None` for a
    /// record with no TOTP secret. `now` is `None` while the clock is not
    /// checked: `Ok(None)` where the secret reads and there is no code to
    /// show yet. The code is revealed for `VaultEntry(Part::Totp)`.
    pub fn totp(&self, id: VaultId, i: usize, now: Option<u64>, _why: Reveal) -> Option<Totp> {
        let r = self.slot(id)?.records.records.get(i)?;
        if r.kind != kind::ENTRY {
            return None;
        }
        let t = r.text(field::TOTP)?;
        Some(Totp {
            account: fv::entries::otpauth_names(t)
                .map(|(_, a)| a)
                .filter(|a| !a.is_empty()),
            code: crate::totp::shown(t, now),
        })
    }

    /// Costs a KDBX 4 database's Argon2id run. A KeePass database holding
    /// entry `i` of open vault `id` (its title, its notes with its
    /// username and URL put before them under `words`, and its password),
    /// sealed under `passphrase`, with its randomness from `seed`. The
    /// bytes are encrypted, so public. `None` where the record is no
    /// entry or the entry does not fit.
    pub fn kdbx_entry(
        &self,
        c: &Compute,
        at: (VaultId, usize),
        words: &KdbxWords<'_>,
        passphrase: &[u8],
        seed: &[u8; KDBX_SEED_LEN],
    ) -> Option<Vec<u8>> {
        let (id, i) = at;
        let r = self.slot(id)?.records.records.get(i)?;
        if r.kind != kind::ENTRY {
            return None;
        }
        let title = r.text(field::TITLE).unwrap_or(words.title);
        let mut notes = String::new();
        for (label, f) in [(words.username, field::USERNAME), (words.url, field::URL)] {
            if let Some(v) = r.text(f) {
                notes.push_str(&format!("{label}: {v}\n"));
            }
        }
        if let Some(n) = r.text(field::NOTES) {
            notes.push_str(n);
        }
        let notes = Zeroizing::new(notes);
        write_kdbx(
            c,
            title.as_bytes(),
            notes.as_bytes(),
            r.field(field::PASSWORD).unwrap_or(&[]),
            passphrase,
            seed,
        )
    }

    // ---- rounds ----

    /// The carry section the first unused round of any open vault bound to
    /// transaction `tx` holds, when it reads. A round is kept across a
    /// lock; its state is the secret nonces of a signing.
    pub fn round_section(
        &self,
        tx: &[u8; 32],
    ) -> Option<Held<faraday_psbt::threshold::CarrySection>> {
        for s in &self.slots {
            for (_, r) in s.records.of(kind::ROUND) {
                if r.field(field::ROUND_TX) == Some(&tx[..])
                    && r.field(field::ROUND_USED).is_none()
                    && let Some(state) = r.field(field::ROUND_STATE)
                    && let Some(section) = crate::carry::section_of(state)
                {
                    return Some(Held::of(section));
                }
            }
        }
        None
    }

    // ---- GPG keys ----

    /// Costs Ed25519 signatures. The public certificate (packets) of GPG
    /// record `i` of open vault `id`, signed at `at`, with its expiry
    /// `expiry` when given.
    pub fn gpg_certificate(
        &self,
        id: VaultId,
        i: usize,
        at: u32,
        expiry: Option<u32>,
    ) -> Option<Vec<u8>> {
        let mut key = gpg_key(self.slot(id)?.records.records.get(i)?)?;
        if let Some(e) = expiry {
            key.expiry = e;
        }
        Some(crate::gpg::certificate(&key, at))
    }

    /// Costs an Ed25519 signature. A revocation certificate (packets) for
    /// GPG record `i` of open vault `id`.
    pub fn gpg_revocation(
        &self,
        id: VaultId,
        i: usize,
        at: u32,
        reason: faraday_pgp::Reason,
        text: &str,
    ) -> Option<Vec<u8>> {
        let key = gpg_key(self.slot(id)?.records.records.get(i)?)?;
        Some(crate::gpg::revocation(&key, at, reason, text))
    }

    /// A revocation certificate of GPG record `i` of open vault `id` as the
    /// file GnuPG keeps it in. Secret: whoever has it can revoke the key.
    pub fn gpg_revocation_file(
        &self,
        id: VaultId,
        i: usize,
        revocation: &[u8],
    ) -> Option<Zeroizing<String>> {
        let key = gpg_key(self.slot(id)?.records.records.get(i)?)?;
        Some(Zeroizing::new(crate::gpg::revocation_file(
            &key, revocation,
        )))
    }

    /// Costs an Ed25519 signature. The signature over `data` by GPG record
    /// `i` of open vault `id`'s signing subkey, at `at`, armoured.
    pub fn gpg_sign_armored(&self, id: VaultId, i: usize, data: &[u8], at: u32) -> Option<String> {
        let key = gpg_key(self.slot(id)?.records.records.get(i)?)?;
        Some(crate::gpg::sign_armored(&key, data, at))
    }

    /// The paperkey text of GPG record `r`: secret, made only to be shown,
    /// wiped when dropped (`Reveal::GpgPaperkey`).
    pub fn gpg_paperkey(&self, r: RecordView<'_>, _why: Reveal) -> Option<Held<Zeroizing<String>>> {
        let key = gpg_key(r.raw())?;
        Some(Held::of(crate::gpg::paperkey(&key)))
    }

    // ---- APK and Secure Boot keys ----

    /// Costs RSA signatures over the whole file. APK `src` (read as `apk`)
    /// signed by APK key record `i` of open vault `id`, the signing
    /// blinded from `seed` (fresh bytes of the system's). `None` for a
    /// record that is no APK key.
    pub fn apk_sign(
        &self,
        id: VaultId,
        i: usize,
        seed: &[u8; 32],
        src: &mut dyn faraday_apk::Source,
        apk: &faraday_apk::Apk,
    ) -> Option<Result<Vec<u8>, faraday_apk::Error>> {
        let r = self.slot(id)?.records.records.get(i)?;
        if r.kind != kind::APK_KEY {
            return None;
        }
        Some(crate::apk::sign(
            seed,
            src,
            apk,
            r.field(field::APK_KEY)?,
            r.field(field::APK_CERT)?,
        ))
    }

    /// Costs RSA signatures. The enrolment files, by name, for Secure Boot
    /// record `i` of open vault `id` under `policy`, the signing blinded
    /// from `blind_seed`. `None` for a record that is no readable set of
    /// owner keys.
    pub fn sb_enrolment(
        &self,
        id: VaultId,
        i: usize,
        blind_seed: &[u8; 32],
        policy: faraday_sb::Policy,
        now: u64,
    ) -> Option<Result<Enrolment, faraday_sb::Error>> {
        let keys = sb_keys(self.slot(id)?.records.records.get(i)?)?;
        Some(crate::secure_boot::enrolment(
            blind_seed, &keys, policy, now,
        ))
    }

    /// Costs an RSA signature. The image `pe` signed by the db key of
    /// Secure Boot record `i` of open vault `id`, the signing blinded from
    /// `blind_seed`. `None` for a record that is no readable set of owner
    /// keys.
    pub fn sb_sign_image(
        &self,
        id: VaultId,
        i: usize,
        blind_seed: &[u8; 32],
        pe: &[u8],
    ) -> Option<Result<Vec<u8>, faraday_sb::Error>> {
        let keys = sb_keys(self.slot(id)?.records.records.get(i)?)?;
        Some(crate::secure_boot::sign_image(blind_seed, pe, &keys))
    }

    // ---- loading keys into the session ----

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

    // ---- for tests ----

    /// Open vault `id`'s records as they are held, for tests (feature
    /// `master-for-tests`, which no shipped binary has).
    #[cfg(feature = "master-for-tests")]
    pub fn contents_for_tests(&self, id: VaultId) -> &Contents {
        match self.slot(id) {
            Some(s) => &s.records,
            None => &NO_RECORDS,
        }
    }

    /// Open vault `id`'s records, to change, for tests (feature
    /// `master-for-tests`). The vault's version moves, as for any edit.
    #[cfg(feature = "master-for-tests")]
    pub fn contents_for_tests_mut(&mut self, id: VaultId) -> &mut Contents {
        let s = self.slot_mut(id).expect("an open vault");
        s.changed();
        &mut s.records
    }
}

/// The words a KeePass export puts around an entry's own text.
pub struct KdbxWords<'a> {
    /// The title of an entry that has none.
    pub title: &'a str,
    /// The label before the username in the notes.
    pub username: &'a str,
    /// The label before the URL in the notes.
    pub url: &'a str,
}

/// What loading a key record with a passphrase came to.
pub enum KeyLoad {
    /// The key it gives is not one `accept` took: its fingerprint, and
    /// nothing loaded.
    Refused(Fingerprint),
    /// The key was added to the session, or refused by it.
    Done(Result<Fingerprint, Refusal>),
}

/// Secure Boot enrolment files, by name.
pub type Enrolment = Vec<(String, Vec<u8>)>;

/// A TOTP entry's code and account, as [`OpenVaults::totp`] gives them.
pub struct Totp {
    /// The account the secret's `otpauth://` label names, when it has one.
    pub account: Option<String>,
    /// The code, or why there is none: `Ok(None)` while the clock is not
    /// checked.
    pub code: Result<Option<crate::totp::Shown>, crate::totp::NotTotp>,
}

/// Costs two Ed25519 public keys. A GPG key's public facts, worked out of
/// its record: its fingerprints, dates and user IDs. `None` for a record
/// without a signing subkey, which is kept and shown by its user ID alone.
pub fn gpg_facts(r: RecordView<'_>) -> Option<GpgFacts> {
    let key = gpg_key(r.raw())?;
    Some(GpgFacts {
        fingerprint: key.fingerprint(),
        subkey_fingerprint: key.subkey_fingerprint(),
        created: key.created,
        subkey_created: key.subkey_created,
        user_ids: key.user_ids.clone(),
        expiry: key.expiry,
    })
}

/// A GPG key's public facts ([`gpg_facts`]).
#[derive(Debug, Clone)]
pub struct GpgFacts {
    /// The primary key's fingerprint.
    pub fingerprint: [u8; 20],
    /// The signing subkey's fingerprint.
    pub subkey_fingerprint: [u8; 20],
    /// When the primary key was made.
    pub created: u32,
    /// When the subkey was made.
    pub subkey_created: u32,
    /// The user IDs.
    pub user_ids: Vec<String>,
    /// Seconds after creation the key expires; 0 for never.
    pub expiry: u32,
}

fn u32_of(r: &Record, n: u8) -> Option<u32> {
    Some(u32::from_le_bytes(r.field(n)?.try_into().ok()?))
}

/// The key a GPG record (type 7) holds, its seeds wiped when it drops.
/// `None` for a record without a signing subkey.
fn gpg_key(r: &Record) -> Option<faraday_pgp::Key> {
    if r.kind != kind::GPG {
        return None;
    }
    let primary = r.field(gpg_field::PRIMARY)?;
    let subkey = r.field(gpg_field::SUBKEY)?;
    let created = u32_of(r, gpg_field::CREATED)?;
    crate::gpg::key_of(
        primary,
        created,
        subkey,
        u32_of(r, gpg_field::SUBKEY_CREATED).unwrap_or(created),
        r.fields
            .iter()
            .filter(|x| x.number == gpg_field::USER_ID)
            .map(|x| String::from_utf8_lossy(&x.bytes).into_owned())
            .collect(),
        u32_of(r, gpg_field::EXPIRY).unwrap_or(0),
    )
}

/// The owner keys a Secure Boot record (type 8) holds.
fn sb_keys(r: &Record) -> Option<faraday_sb::Keys> {
    if r.kind != kind::SECURE_BOOT {
        return None;
    }
    let pair = |key: u8, cert: u8| -> Option<faraday_sb::Pair> {
        Some(faraday_sb::Pair {
            key: Zeroizing::new(r.field(key)?.to_vec()),
            cert: r.field(cert)?.to_vec(),
        })
    };
    crate::secure_boot::keys_of(
        r.field(sb_field::OWNER)?,
        pair(sb_field::PK_KEY, sb_field::PK_CERT)?,
        pair(sb_field::KEK_KEY, sb_field::KEK_CERT)?,
        pair(sb_field::DB_KEY, sb_field::DB_CERT)?,
    )
}

// ---- key records, by view ----

/// A key record's id: the SHA-256 of its payload and the passphrase
/// stored with it, by which what is worked out of it is kept. `None` for
/// a record with no key. Costs a hash.
pub fn key_record_id(r: RecordView<'_>) -> Option<[u8; 32]> {
    use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
    let r = r.raw();
    // By the payload and the passphrase stored with it: the same words
    // with a passphrase are another key.
    let payload = r.field(field::KEY)?;
    let mut engine = sha256::Hash::engine();
    engine.input(payload);
    if let Some(p) = r.field(field::KEY_PASSPHRASE) {
        engine.input(&[0]);
        engine.input(p);
    }
    Some(sha256::Hash::from_engine(engine).to_byte_array())
}

/// Costs a parse of the words for a record that holds them. Runs `f` on
/// what a key record is made from, its words (with the passphrase stored
/// beside them) or its master seed.
fn with_material<T>(
    c: &Compute,
    r: &Record,
    f: impl FnOnce(Material<'_>, &str) -> Option<T>,
) -> Option<T> {
    match record_words(c, r) {
        Some(m) => f(
            Material::Words(&m),
            r.text(field::KEY_PASSPHRASE).unwrap_or(""),
        ),
        None => {
            let seed = seed_of(r)?;
            f(Material::Seed(&seed), "")
        }
    }
}

/// Costs a PBKDF2 and two elliptic-curve multiplications. A key record's
/// fingerprint and [`master_id`] on `network`, worked out fresh.
pub fn key_identity(
    c: &Compute,
    network: Network,
    r: RecordView<'_>,
) -> Option<(Fingerprint, [u8; 32])> {
    with_material(c, r.raw(), |material, passphrase| {
        crate::identity::identity(c, material, passphrase, network)
    })
}

/// Costs a PBKDF2 and a key derivation. A key record's account key at
/// `path` on `network`: its public key and chain code, which do not change
/// with the network, worked out fresh.
pub fn key_account(
    c: &Compute,
    network: Network,
    r: RecordView<'_>,
    path: &DerivationPath,
) -> Option<[u8; 65]> {
    with_material(c, r.raw(), |material, passphrase| {
        crate::identity::xpub_at(c, material, passphrase, network, path)
    })
    .map(|x| account_bytes(&x))
}

/// An extended public key's public key and chain code, which do not
/// change with the network it is written for. A lookup.
pub fn account_bytes(x: &faraday_bip::bitcoin::bip32::Xpub) -> [u8; 65] {
    let mut out = [0u8; 65];
    out[..33].copy_from_slice(&x.public_key.serialize());
    out[33..].copy_from_slice(x.chain_code.as_bytes());
    out
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
/// words.
fn seed_of(r: &Record) -> Option<Zeroizing<Vec<u8>>> {
    match oskb::payload_of(oskb::KIND_SEED, r.field(field::KEY)?)? {
        oskb::Opened::Seed(s) => Some(s),
        _ => None,
    }
}

/// A wallet recovery sheet record's descriptor, name and note as text.
/// Empty for a record that holds none. A sheet is public in full.
pub fn sheet_parts(r: RecordView<'_>) -> (String, String, String) {
    match r
        .public(field::SHEET)
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
