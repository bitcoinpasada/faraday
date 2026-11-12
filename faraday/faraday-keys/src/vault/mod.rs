//! The open vaults: each one's opened file and its records
//! (`docs/KEYS.md` §4 step 3).
//!
//! A vault is opened here ([`OpenVaults::unlock`]) and sealed by a job
//! this store makes ([`OpenVaults::seal_job`]), which marks the vault
//! sealing until [`OpenVaults::seal_ended`].
//! The opened file, with the passphrase it seals under, is private to this
//! module and the job. The caller holds a [`VaultId`], which tells open
//! vaults apart and says nothing else.
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

use std::cell::{Cell, OnceCell};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use faraday_bip::bip39::Mnemonic;
use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::keys::{Fingerprint, Network};
use faraday_crypto::work::{Compute, Work};
use faraday_intake::secret_buf::SecretText;
use faraday_intake::wallet::Refusal;
use faraday_vault::file::Change;
use faraday_vault::kdbx::items::{self, Body, Damage, Fault, Place, Unreadable, Unwritable};
use faraday_vault::layouts;
use faraday_vault::records::{self, field, kind};
use faraday_vault::{self as fv, Record};
pub use items::Related;
use zeroize::Zeroizing;

use crate::identity::{Material, fingerprint};
use crate::session::{Held, Keys, Part, Reveal, Revealed, WordsId, words_id};

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
    /// The revocation certificate made with the key, as packets. Secret:
    /// whoever has it can revoke the key (F32).
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
/// - ROUND: the round's state (secret nonces);
/// - GPG: the primary key's and the subkey's seeds, and the revocation
///   certificate kept with them;
/// - SECURE_BOOT: the PK's, KEK's and db's private keys;
/// - CODEX32_SHARE: the string.
///
/// Public, in full: the slot label, wallets, sheets, amounts, plans and
/// wallet shares; a key's label, flags, codex32 set's facts and the
/// BIP-85 key it is recorded as a child of; a
/// codex32 share's key fingerprint; a passphrase's key
/// fingerprint; an entry's title, username and URL; a round's scheme,
/// transaction hash, key fingerprint and used flag; a GPG key's dates, user IDs, certificate
/// and revoked date; a Secure Boot record's owner and certificates.
pub const SECRET_FIELDS: &[(u8, u8)] = &[
    (kind::KEY, field::KEY),
    (kind::KEY, field::KEY_PASSPHRASE),
    (kind::PASSPHRASE, field::PASSPHRASE),
    (kind::NOTE, field::NOTE),
    (kind::ENTRY, field::PASSWORD),
    (kind::ENTRY, field::NOTES),
    (kind::ENTRY, field::TOTP),
    (kind::ROUND, field::ROUND_STATE),
    (kind::GPG, gpg_field::PRIMARY),
    (kind::GPG, gpg_field::SUBKEY),
    (kind::GPG, gpg_field::REVOCATION),
    (kind::SECURE_BOOT, sb_field::PK_KEY),
    (kind::SECURE_BOOT, sb_field::KEK_KEY),
    (kind::SECURE_BOOT, sb_field::DB_KEY),
    (kind::CODEX32_SHARE, field::CODEX32_STRING),
];

/// Whether field `field` of a record of type `kind` is secret
/// ([`SECRET_FIELDS`]).
pub fn is_secret(kind: u8, field: u8) -> bool {
    SECRET_FIELDS.contains(&(kind, field))
}

/// Which open vault. Public: it tells vaults apart and opens nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VaultId(u32);

/// A vault's seal count: how many times it has been written, and the
/// time of the last seal (Unix seconds by the sealing computer's clock;
/// `None` when it was not set or is not readable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seals {
    /// How many times the vault has been written.
    pub count: u64,
    /// The last seal's time.
    pub last: Option<u64>,
}

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
/// in hand, or one an operation of this crate made and holds. Only the
/// edits of [`OpenVaults`] take the record out of it.
pub trait IntoRecord: Fit {
    /// The record.
    fn into_record(self) -> Record;
    /// The record held, to wait (a dialog's pending list) with its
    /// secrets still guarded.
    fn staged(self) -> Staged;
    /// The bytes the record takes in a slot.
    fn size(&self) -> usize;
}

/// Proof of being inside this crate: only it can make one, so only it
/// reads the record of a [`Fit`].
pub struct Inside(());

/// What can be measured against a slot or looked up for a related
/// record without the record leaving its guard: [`OpenVaults::fits`] and
/// [`OpenVaults::relates`] take it.
pub trait Fit {
    /// The record; for this crate's use.
    fn record_of(&self, _: Inside) -> &Record;
}

impl Fit for Record {
    fn record_of(&self, _: Inside) -> &Record {
        self
    }
}

impl Fit for Held<Record> {
    fn record_of(&self, _: Inside) -> &Record {
        self
    }
}

impl Fit for MadeKey {
    fn record_of(&self, _: Inside) -> &Record {
        &self.0
    }
}

impl Fit for &MadeKey {
    fn record_of(&self, _: Inside) -> &Record {
        &self.0
    }
}

impl Fit for Staged {
    fn record_of(&self, _: Inside) -> &Record {
        self.record()
    }
}

impl IntoRecord for Record {
    fn into_record(self) -> Record {
        self
    }
    fn staged(self) -> Staged {
        Staged::Made(self)
    }
    fn size(&self) -> usize {
        Record::size(self)
    }
}

impl IntoRecord for Held<Record> {
    fn into_record(self) -> Record {
        self.release()
    }
    fn staged(self) -> Staged {
        Staged::Held(self)
    }
    fn size(&self) -> usize {
        self.view().size()
    }
}

/// A key made here from fresh entropy (a GPG or Secure Boot key), as
/// the record every vault of a plan takes. Its secrets never leave: it is
/// neither `Clone` nor `Debug`, shows its public fields through
/// [`MadeKey::view`], and each vault takes a copy made inside this crate
/// (`IntoRecord for &MadeKey`).
pub struct MadeKey(Record);

impl MadeKey {
    /// Holds `record`.
    pub(crate) fn of(record: Record) -> MadeKey {
        MadeKey(record)
    }

    /// The record's public fields.
    pub fn view(&self) -> RecordView<'_> {
        RecordView::of(&self.0)
    }

    /// The bytes the record takes in a slot.
    pub fn size(&self) -> usize {
        self.view().size()
    }
}

impl IntoRecord for &MadeKey {
    fn into_record(self) -> Record {
        self.0.clone()
    }
    fn staged(self) -> Staged {
        Staged::Made(self.0.clone())
    }
    fn size(&self) -> usize {
        MadeKey::size(self)
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

    /// The record itself, for this crate's measures.
    pub(crate) fn record(&self) -> &Record {
        match self {
            Staged::Made(r) => r,
            Staged::Held(h) => h,
        }
    }
}

impl std::fmt::Debug for Staged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Staged(..)")
    }
}

impl IntoRecord for Staged {
    fn into_record(self) -> Record {
        match self {
            Staged::Made(r) => r,
            Staged::Held(h) => h.release(),
        }
    }
    fn staged(self) -> Staged {
        self
    }
    fn size(&self) -> usize {
        Staged::size(self)
    }
}

/// One write a seal makes, owned, so that a job can carry it.
pub(crate) enum Op {
    /// An entry written: a new one takes `uuid` and, in its kind's group,
    /// `group_uuid` if the group is new.
    Put {
        uuid: [u8; 16],
        group_uuid: [u8; 16],
        new: bool,
        record: Record,
    },
    /// An entry taken out.
    Remove([u8; 16]),
    /// A plan or signed amounts taken out, by key.
    RemoveCustom(String),
    /// A plan written at its number.
    Plan { n: u32, record: Record },
    /// Signed amounts written.
    Signed(Record),
    /// A transaction's nonce record written.
    Nonces(Record),
    /// The vault renamed.
    Label(String),
}

/// What an entry of a record's kind holds for the vault crate: a GPG key
/// is written from its armored form.
fn body_of(record: &Record) -> Option<Body> {
    if record.kind != kind::GPG {
        return Some(Body::Record(record.clone()));
    }
    let created = record
        .field(gpg_field::CREATED)
        .and_then(|b| <[u8; 4]>::try_from(b).ok())
        .map(u32::from_le_bytes);
    let at = record
        .field(gpg_field::CERTIFICATE)
        .and_then(faraday_pgp::signature_time)
        .or(created)
        .unwrap_or(0);
    crate::gpg::gpg_key_of(record, at).map(Body::Gpg)
}

/// Costs a PBKDF2 and an elliptic-curve multiplication for each key with
/// no label, when `exact`, and what a GPG key's armor costs. Gives `f` the
/// changes `ops` make, as the vault crate takes them, with the time `now`
/// written as each entry's last modification. With `exact` false a key's
/// title is a stand-in of the same length.
pub(crate) fn with_changes<T>(
    c: &Compute,
    names: &Names,
    ops: &[Op],
    exact: bool,
    now: Option<u64>,
    f: impl FnOnce(&[Change<'_>]) -> Result<T, fv::file::Error>,
) -> Result<T, fv::file::Error> {
    struct Prepared {
        title: String,
        body: Body,
    }
    let prepared: Vec<Option<Prepared>> = ops
        .iter()
        .map(|op| match op {
            Op::Put { record, .. } => body_of(record).map(|body| Prepared {
                title: names.title(c, record, exact),
                body,
            }),
            _ => None,
        })
        .collect();
    let when = now.and_then(|n| i64::try_from(n).ok());
    let mut changes: Vec<Change<'_>> = Vec::with_capacity(ops.len());
    for (op, p) in ops.iter().zip(&prepared) {
        changes.push(match op {
            Op::Put {
                uuid,
                group_uuid,
                record,
                ..
            } => {
                // A record with no form to write is a refusal, never a
                // change left out.
                let p = p
                    .as_ref()
                    .ok_or(fv::file::Error::Unwritable(Unwritable::Record(record.kind)))?;
                Change::Put {
                    place: Place {
                        uuid: *uuid,
                        group: names.group(record.kind),
                        group_uuid: *group_uuid,
                        title: &p.title,
                        now: when,
                    },
                    body: &p.body,
                }
            }
            Op::Remove(uuid) => Change::Remove(*uuid),
            Op::RemoveCustom(key) => Change::RemoveCustom(key),
            Op::Plan { n, record } => Change::Plan { n: *n, record },
            Op::Signed(record) => Change::Signed(record),
            Op::Nonces(record) => Change::Nonces(record),
            Op::Label(label) => Change::Label(label),
        });
    }
    f(&changes)
}

/// An open vault's records, as the caller may read them. Cheap to copy.
#[derive(Clone, Copy)]
pub struct Records<'a> {
    id: VaultId,
    contents: &'a [Record],
    slot: Option<(&'a Slot, &'a Names)>,
}

impl<'a> Records<'a> {
    /// How many records.
    pub fn len(&self) -> usize {
        self.contents.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }

    /// Record `i`.
    pub fn get(&self, i: usize) -> Option<RecordView<'a>> {
        let id = self.id;
        self.contents.get(i).map(|record| RecordView {
            record,
            home: Some((id, i)),
        })
    }

    /// Every record with its place.
    pub fn iter(&self) -> impl Iterator<Item = (usize, RecordView<'a>)> + use<'a> {
        let id = self.id;
        self.contents.iter().enumerate().map(move |(i, record)| {
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

    /// Costs a read of each record changed and, only past the ceiling by
    /// [`Slot::bound`], a read of the free memory and a count of the
    /// document, the first time after a change (a lookup after, except where
    /// memory is short: the count is tried again each time). The bytes of
    /// plaintext the next seal takes:
    /// an upper bound under the ceiling, exactly past it
    /// (`faraday_vault::file::Opened::used`); 0 for a vault that is
    /// read-only or not held. Where memory is short for the exact count
    /// ([`Records::counted`]) the bound: a figure to show, never to decide
    /// from.
    pub fn used(&self, c: &Compute) -> usize {
        self.slot.map_or(0, |(s, names)| s.used(c, names))
    }

    /// Costs what [`Records::used`] costs, and a read of the free memory
    /// before a copy of the document is made. The bytes of plaintext the
    /// next seal takes, as [`Records::used`] says them but never the
    /// bound past the ceiling: `None` when the exact count cannot be made
    /// because what is free does not hold the copy of the document.
    pub fn counted(&self, c: &Compute) -> Option<usize> {
        self.slot.map_or(Some(0), |(s, names)| s.counted(c, names))
    }

    /// Costs what [`Records::counted`] costs, with its read of the free
    /// memory. Whether the seal refuses to
    /// write this vault as it is (a record it cannot write, a file it
    /// cannot make): then its size says nothing, and taking records out
    /// to make room cannot help unless the refused one goes.
    pub fn unwritable(&self, c: &Compute) -> bool {
        self.counted(c) == Some(usize::MAX)
    }
}

/// The names Faraday writes into a vault in the language the app is in
/// (`docs/VAULT.md` §7): one group per kind, and the titles of the items
/// that have none of their own. `{fingerprint}` and `{for}` in a title
/// stand for the key's fingerprint and the fingerprint a secret is for.
/// The caller sets them once ([`OpenVaults::set_names`]).
#[derive(Debug, Clone, Default)]
pub struct Names {
    /// Group of seeds.
    pub group_keys: String,
    /// Group of wallets.
    pub group_wallets: String,
    /// Group of notes.
    pub group_notes: String,
    /// Group of recovery sheets.
    pub group_sheets: String,
    /// Group of GPG keys.
    pub group_gpg: String,
    /// Group of Secure Boot keys.
    pub group_secure_boot: String,
    /// Group of wallet shares.
    pub group_wallet_shares: String,
    /// Group of passphrases.
    pub group_passphrases: String,
    /// Group of codex32 shares.
    pub group_shares: String,
    /// A key with no label: "Key {fingerprint}".
    pub title_key: String,
    /// A note.
    pub title_note: String,
    /// A passphrase: "Passphrase for {for}".
    pub title_passphrase: String,
    /// A codex32 share: "Codex32 share for {for}".
    pub title_share: String,
    /// Secure Boot keys.
    pub title_secure_boot: String,
}

impl Names {
    /// What the longest group name and the longest title take as markup,
    /// with the hex of a fingerprint among them: the bound of the
    /// names one record's entry may carry.
    pub(crate) fn room(&self) -> usize {
        let longest = |l: &[&String]| l.iter().map(|s| s.len()).max().unwrap_or(0);
        let groups = longest(&[
            &self.group_keys,
            &self.group_wallets,
            &self.group_notes,
            &self.group_sheets,
            &self.group_gpg,
            &self.group_secure_boot,
            &self.group_wallet_shares,
            &self.group_passphrases,
            &self.group_shares,
        ]);
        let titles = longest(&[
            &self.title_key,
            &self.title_note,
            &self.title_passphrase,
            &self.title_share,
            &self.title_secure_boot,
        ]);
        BOUND_TEXT * (groups + titles) + BOUND_FIELD
    }

    /// The group a record of kind `k` is written in; none for a person's
    /// entry, which goes in the root.
    pub(crate) fn group(&self, k: u8) -> &str {
        match k {
            kind::KEY => &self.group_keys,
            kind::WALLET => &self.group_wallets,
            kind::NOTE => &self.group_notes,
            kind::SHEET => &self.group_sheets,
            kind::GPG => &self.group_gpg,
            kind::SECURE_BOOT => &self.group_secure_boot,
            kind::WALLET_SHARE => &self.group_wallet_shares,
            kind::PASSPHRASE => &self.group_passphrases,
            kind::CODEX32_SHARE => &self.group_shares,
            _ => "",
        }
    }

    /// Costs a PBKDF2 and an elliptic-curve multiplication for a key with
    /// no label, when `exact` (a label is the title, so a key with one costs nothing). The title written where the record has none
    /// of its own; with `exact` false a key's fingerprint is a stand-in of
    /// the same length.
    pub(crate) fn title(&self, c: &Compute, r: &Record, exact: bool) -> String {
        let hex = |n: u8| {
            r.field(n)
                .map(|b| b.iter().map(|x| format!("{x:02x}")).collect::<String>())
                .unwrap_or_default()
        };
        match r.kind {
            kind::KEY => {
                let labelled = r.text(field::KEY_LABEL).is_some_and(|t| !t.is_empty());
                let fp = if exact && !labelled {
                    with_material(c, r, |m, p| fingerprint(c, m, p, Network::Mainnet))
                        .map(crate::identity::fp_text)
                } else {
                    None
                };
                self.title_key.replace(
                    "{fingerprint}",
                    &fp.unwrap_or_else(|| "00000000".to_string()),
                )
            }
            kind::NOTE => self.title_note.clone(),
            kind::SECURE_BOOT => self.title_secure_boot.clone(),
            kind::PASSPHRASE => self
                .title_passphrase
                .replace("{for}", &hex(field::PASSPHRASE_KEY)),
            kind::CODEX32_SHARE => self.title_share.replace("{for}", &hex(field::CODEX32_KEY)),
            _ => String::new(),
        }
    }
}

/// What a record was read from, so that a seal writes it back there.
pub(crate) enum Origin {
    /// An entry: its UUID, the names of its groups, its kind as read.
    Entry {
        uuid: [u8; 16],
        path: Arc<[Arc<str>]>,
        kind: u8,
        /// Its place in `read().items`.
        item: usize,
    },
    /// A backup plan or signed amounts in the database's custom data.
    Custom { key: String },
    /// The vault's label.
    Label,
    /// A record made here.
    New,
}

/// Costs a count of the document's bytes, with no copy of it and no
/// buffer. The plaintext a seal takes of `opened` with no change, exactly;
/// `usize::MAX` when a seal cannot write it as it is (a read-only vault
/// among them: no change is made to one).
pub(crate) fn base_of(opened: &mut fv::file::Opened) -> usize {
    opened.base_used().unwrap_or(usize::MAX)
}

/// The app's view of an opened file, built on the worker that opened it.
pub(crate) struct Built {
    pub(crate) records: Vec<Record>,
    pub(crate) origin: Vec<Origin>,
    pub(crate) unreadable: Vec<Unreadable>,
}

/// Costs the Ed25519 signatures that read each GPG key. The records of an
/// opened file: the label, then each entry that reads, then each plan and
/// signed amounts; a GPG key not in Faraday's own shape goes on the
/// unreadable list and is not touched by a seal.
pub(crate) fn build(opened: &fv::file::Opened) -> Built {
    let read = opened.read();
    let mut built = Built {
        records: vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, read.label.as_bytes())],
        origin: vec![Origin::Label],
        unreadable: read.unreadable.clone(),
    };
    for (item, it) in read.items.iter().enumerate() {
        let record = match &it.body {
            Body::Record(r) => Some(r.clone()),
            Body::Gpg(g) => crate::gpg::record_of(g),
        };
        match record {
            Some(r) => {
                built.records.push(r);
                built.origin.push(Origin::Entry {
                    uuid: it.uuid,
                    path: Arc::clone(&it.path),
                    kind: it.body.kind(),
                    item,
                });
            }
            None => {
                let title = match &it.body {
                    Body::Gpg(g) => g.title.clone(),
                    Body::Record(_) => String::new(),
                };
                built.unreadable.push(Unreadable {
                    title,
                    uuid: Some(it.uuid),
                    what: Damage {
                        at: "key".to_string(),
                        fault: Fault::Value,
                    },
                });
            }
        }
    }
    for cu in &read.custom {
        built.records.push(cu.record.clone());
        built.origin.push(Origin::Custom {
            key: cu.key.clone(),
        });
    }
    built
}

/// What a read of the records leaves out or adds, to ask what a seal
/// would take without making the edit.
struct Overlay<'a> {
    /// Records taken out, by place.
    remove: &'a [usize],
    /// Records put in last.
    add: &'a [&'a Record],
    /// Records changed in place.
    edit: &'a [(usize, &'a Record)],
}

impl Overlay<'_> {
    const NONE: Overlay<'static> = Overlay {
        remove: &[],
        add: &[],
        edit: &[],
    };
}

/// What is worked out of one key record's words, once per change: how
/// many words, and what tells them from other words ([`WordsId`]).
struct Fact {
    words: usize,
    /// The words' list: public, as the count is.
    language: faraday_bip::bip39::Language,
    id: WordsId,
}

/// Whether records of kind `k` are entries.
fn is_entry_kind(k: u8) -> bool {
    matches!(
        k,
        kind::KEY
            | kind::WALLET
            | kind::NOTE
            | kind::SHEET
            | kind::GPG
            | kind::SECURE_BOOT
            | kind::WALLET_SHARE
            | kind::PASSPHRASE
            | kind::CODEX32_SHARE
            | kind::ENTRY
    )
}

/// What a removed record leaves to take out at the seal.
fn drop_op(o: &Origin, ops: &mut Vec<Op>) {
    match o {
        Origin::Entry { uuid, .. } => ops.push(Op::Remove(*uuid)),
        Origin::Custom { key } => ops.push(Op::RemoveCustom(key.clone())),
        // The name taken out is a name cleared.
        Origin::Label => ops.push(Op::Label(String::new())),
        Origin::New => {}
    }
}

/// A record with no place in the file yet.
///
/// A kind with no place in the file, or no number left for a plan, is a
/// refusal: the seal does not leave the change out.
fn fresh_op(rec: &Record, plan: &mut Option<u32>, ops: &mut Vec<Op>) -> Result<(), Unwritable> {
    match rec.kind {
        kind::PLAN => {
            let n = plan.ok_or(Unwritable::Plan)?;
            ops.push(Op::Plan {
                n,
                record: rec.clone(),
            });
            *plan = n.checked_add(1);
        }
        kind::AMOUNTS => ops.push(Op::Signed(rec.clone())),
        kind::NONCES => ops.push(Op::Nonces(rec.clone())),
        kind::SLOT_LABEL => ops.push(Op::Label(rec.text(field::LABEL).unwrap_or("").to_string())),
        k if is_entry_kind(k) => ops.push(Op::Put {
            uuid: [0; 16],
            group_uuid: [0; 16],
            new: true,
            record: rec.clone(),
        }),
        k => return Err(Unwritable::Record(k)),
    }
    Ok(())
}

/// What a record present says to write: nothing when it is as read.
fn put_op(
    o: &Origin,
    rec: &Record,
    dirty: bool,
    plan: &mut Option<u32>,
    ops: &mut Vec<Op>,
) -> Result<(), Unwritable> {
    match o {
        Origin::Entry { uuid, kind, .. } if is_entry_kind(rec.kind) && rec.kind == *kind => {
            if dirty {
                ops.push(Op::Put {
                    uuid: *uuid,
                    group_uuid: [0; 16],
                    new: false,
                    record: rec.clone(),
                });
            }
        }
        Origin::Custom { key }
            if rec.kind == kind::AMOUNTS && key.starts_with("faraday.signed.") =>
        {
            if dirty {
                // The key names the transaction: one changed to another
                // leaves the old key to go.
                let tx = rec.field(field::AMOUNTS_TX).unwrap_or(&[]);
                let now: String = tx.iter().map(|b| format!("{b:02x}")).collect();
                if key.strip_prefix("faraday.signed.") != Some(now.as_str()) {
                    drop_op(o, ops);
                }
                ops.push(Op::Signed(rec.clone()));
            }
        }
        Origin::Custom { key }
            if rec.kind == kind::NONCES && key.starts_with("faraday.nonces.") =>
        {
            if dirty {
                let tx = rec.field(field::NONCES_TX).unwrap_or(&[]);
                let now: String = tx.iter().map(|b| format!("{b:02x}")).collect();
                if key.strip_prefix("faraday.nonces.") != Some(now.as_str()) {
                    drop_op(o, ops);
                }
                ops.push(Op::Nonces(rec.clone()));
            }
        }
        Origin::Custom { key } if rec.kind == kind::PLAN => {
            match key
                .strip_prefix("faraday.plan.")
                .and_then(|n| n.parse().ok())
            {
                Some(n) => {
                    if dirty {
                        ops.push(Op::Plan {
                            n,
                            record: rec.clone(),
                        });
                    }
                }
                None => {
                    drop_op(o, ops);
                    fresh_op(rec, plan, ops)?;
                }
            }
        }
        Origin::Label if rec.kind == kind::SLOT_LABEL => {
            if dirty {
                fresh_op(rec, plan, ops)?;
            }
        }
        _ => {
            drop_op(o, ops);
            fresh_op(rec, plan, ops)?;
        }
    }
    Ok(())
}

/// What a seal's own values (the seal count and time, the format) take
/// beyond the file as read, with room to spare.
const BOUND_SEAL: usize = 1024;
/// What one record's entry, its group, its UUID, times and the names of
/// its fields take beyond its text: with the titles, the markup and the
/// custom data of a record of any kind, well under it.
const BOUND_RECORD: usize = 4096;
/// What one field of a record takes beyond its text: its key and tags.
const BOUND_FIELD: usize = 64;
/// How many bytes of document one byte of a record's text takes, at the
/// most: markup escaped is 5 (`&amp;`), a protected value 4/3 in
/// Base64, a byte as hex 2, and a payload as words (a seed) under 7; a GPG
/// user ID is written as the title (escaped, 5) and again inside the
/// protected armor (about 1.78), so 6.78, which the bound takes as 7.
const BOUND_TEXT: usize = 7;
/// What a GPG user ID takes beyond its text: the armor holds the user ID's
/// own self-signature (a packet of about 150 bytes, 1.78 times in Base64,
/// about 260 measured), however short the ID. A record may hold many.
const BOUND_USER_ID: usize = 512;

/// An upper bound on the bytes of document a seal writes for `r`,
/// whatever the record holds: [`BOUND_TEXT`] times each field's bytes, a
/// constant for each field and for the record, and its title.
fn bound_of(names: &Names, r: &Record) -> usize {
    let title = names.room();
    r.fields.iter().fold(BOUND_RECORD + title, |n, f| {
        let more = if r.kind == kind::GPG && f.number == gpg_field::USER_ID {
            BOUND_USER_ID
        } else {
            0
        };
        n + BOUND_TEXT * f.bytes.len() + BOUND_FIELD + more
    })
}

/// One open file of one vault.
struct Slot {
    id: VaultId,
    /// The SHA-256 of the file as it was opened: tells copies apart. The
    /// file itself is not kept: a vault may be 128 MiB.
    digest: [u8; 32],
    /// The plaintext a seal takes with no change, exactly, worked out once
    /// when the file opened; `usize::MAX` when a seal cannot write the
    /// file as it is. What [`Slot::bound`] adds the changes to.
    base: usize,
    /// The file opened. Shared with a seal job, which never changes it.
    opened: Arc<fv::file::Opened>,
    /// The app's view: the label, the entries, the plans and signed amounts.
    records: Vec<Record>,
    /// Where each record was read from, by place.
    origin: Vec<Origin>,
    /// Whether each record was changed since it was read, by place.
    dirty: Vec<bool>,
    /// Whether each record was changed and never checked for the seal, by
    /// place: set by an edit that runs no check, cleared when a check
    /// passes. The next check takes these in with its own changes, so an
    /// edit is checked as one change however many came before.
    unchecked: Vec<Cell<bool>>,
    /// What was read and has been taken out: removed at the seal.
    removed: Vec<Origin>,
    /// What does not read: the file's own, and the GPG keys this store
    /// could not take.
    unreadable: Vec<Unreadable>,
    /// Moves with every edit of the records.
    version: u64,
    /// What is worked out of each key record's words, by record place,
    /// beside `records`; dropped for a record when it is changed, so an
    /// edit of one record leaves the others read.
    facts: Vec<OnceCell<Option<Fact>>>,
    /// The plaintext the next seal takes, when it is an exact count or a
    /// bound under the ceiling (never one cut short by memory); dropped by
    /// every edit.
    used: OnceCell<usize>,
    /// How many bytes of UUIDs the next seal takes ([`OpenVaults::uuid_bytes`]),
    /// 0 when it refuses; dropped by every edit.
    uuids: OnceCell<usize>,
    /// A seal job holds the file: every edit refuses until it ends.
    sealing: bool,
    /// The most plaintext a seal may take.
    limit: usize,
    /// The longest document a copy may be made of, from what is free
    /// ([`OpenVaults::set_copy_room`]); none for no limit known.
    copy_room: Arc<CopyRoom>,
    /// The passphrase the next seal writes, when it is not the one the file
    /// opened with; wiped as it drops or is replaced (never exposed).
    new_passphrase: Option<SecretText>,
    /// The memory step, in KiB, the next seal writes when it is not the
    /// file's own.
    new_step: Option<u32>,
    /// The key file's key the next seal writes, when it is not the file's
    /// own: `Some(None)` writes none. Wiped as it drops or is replaced;
    /// never exposed, but as the document [`OpenVaults::key_file_document`].
    new_key_file: Option<Option<Zeroizing<[u8; 32]>>>,
    /// Whether `new_key_file` is a key Faraday drew
    /// ([`OpenVaults::make_key_file`]) and not one given.
    key_made: bool,
    /// Whether the document of a made `new_key_file` was given
    /// ([`OpenVaults::key_file_document`]), which it is once only: until
    /// then no seal writes it.
    key_taken: bool,
    /// Whether the next seal leaves the YubiKey's secret out.
    drop_yubikey: bool,
}

impl Slot {
    /// Whether the key file made for the next seal waits for its document
    /// to be taken.
    fn key_pending(&self) -> bool {
        self.key_made && !self.key_taken
    }

    /// Every edit ends here: the version moves and what was worked out of
    /// the records is dropped.
    fn changed(&mut self) {
        self.version += 1;
        self.used = OnceCell::new();
        self.uuids = OnceCell::new();
    }

    /// Record `i` was changed with no check of it.
    fn touch(&mut self, i: usize) {
        if let Some(d) = self.dirty.get_mut(i) {
            *d = true;
        }
        if let Some(f) = self.facts.get_mut(i) {
            *f = OnceCell::new();
        }
        if let Some(u) = self.unchecked.get(i) {
            u.set(true);
        }
        self.changed();
    }

    /// Record `i` was changed after a check of it passed.
    fn touch_checked(&mut self, i: usize) {
        self.touch(i);
        if let Some(u) = self.unchecked.get(i) {
            u.set(false);
        }
    }

    /// The UUIDs the checks give new entries and groups: a count, skipping
    /// any an entry of the file has, so a crafted file cannot make every
    /// new record collide.
    fn stand_ins(&self) -> impl FnMut() -> [u8; 16] + '_ {
        let mut n = 0u64;
        move || loop {
            n += 1;
            let mut u = [0u8; 16];
            u[..8].copy_from_slice(&n.to_le_bytes());
            if !self.opened.holds_entry(&u) {
                return u;
            }
        }
    }

    /// Whether the vault may not be changed: read-only, or sealing.
    fn locked(&self) -> bool {
        self.sealing || self.opened.read_only().is_some()
    }

    /// The most plaintext a seal may take: the ceiling, or less where a
    /// store was told to lower it ([`OpenVaults::lower_limit`]).
    fn limit(&self) -> usize {
        self.limit
    }

    /// An upper bound on the plaintext the next seal takes with `ov`
    /// applied: the file's own `base`, the room a seal's own values may
    /// take, and [`bound_of`] each record changed since it was read or
    /// changed by `ov`. A record taken out is not subtracted: the bound
    /// only errs high. Cheap: it reads no document.
    fn bound(&self, names: &Names, ov: &Overlay<'_>) -> usize {
        let mut n = self.base.saturating_add(BOUND_SEAL);
        for (i, r) in self.records.iter().enumerate() {
            if ov.remove.contains(&i) {
                continue;
            }
            let r = match ov.edit.iter().find(|(j, _)| *j == i) {
                Some((_, r)) => *r,
                None if self.dirty[i] => r,
                None => continue,
            };
            n = n.saturating_add(bound_of(names, r));
        }
        for r in ov.add {
            n = n.saturating_add(bound_of(names, r));
        }
        n
    }

    /// Costs a parse of a wallet's text for each record `ov` changes, and
    /// past the bound a read of the free memory, a copy of the document, a
    /// count of it and a read of each item put. Whether the vault, with `ov` applied, may be
    /// sealed: `Ok`; or why not, `Full` saying the bytes over the ceiling.
    /// Under the ceiling by [`Slot::bound`] it checks the changes `ov`
    /// makes, each from the one entry it touches, and each earlier change
    /// that no check has passed yet, once (the others were checked when
    /// they were made, and the seal checks them all), and counts no
    /// document; past it, the exact size decides, so the bound
    /// never refuses a vault that fits and never lets one past.
    fn room(&self, c: &Compute, names: &Names, ov: &Overlay<'_>) -> Result<(), NoRoom> {
        if self.bound(names, ov) <= self.limit() {
            let ops = self.ops(ov, true, &mut self.stand_ins())?;
            with_changes(c, names, &ops, false, Some(0), |changes| {
                self.opened.check(c, changes)
            })?;
            // What was never checked has passed with this, except what the
            // overlay takes out: its removal is only asked about, and may
            // never happen.
            for (i, u) in self.unchecked.iter().enumerate() {
                if !ov.remove.contains(&i) {
                    u.set(false);
                }
            }
            return Ok(());
        }
        match self.used_with(c, names, ov)? {
            n if n <= self.limit() => Ok(()),
            n => {
                // The count of the vault as it is is the one `counted`
                // gives: kept, so that asking for it after this does not
                // make the copy of the document again.
                if ov.remove.is_empty() && ov.add.is_empty() && ov.edit.is_empty() {
                    let _ = self.used.set(n);
                }
                Err(NoRoom::File(fv::file::Error::Full {
                    over: n - self.limit(),
                }))
            }
        }
    }

    /// Costs a parse of key record `i`'s words and a hash, the first time
    /// after that record is changed. What is worked out of them; `None`
    /// for another record, or a key record that holds a seed rather than
    /// words.
    fn fact(&self, c: &Compute, i: usize) -> Option<&Fact> {
        self.facts
            .get(i)?
            .get_or_init(|| {
                let r = self.records.get(i)?;
                let m = (r.kind == kind::KEY)
                    .then(|| record_words(c, r))
                    .flatten()?;
                Some(Fact {
                    words: m.word_count(),
                    language: m.language(),
                    id: words_id(c, &m),
                })
            })
            .as_ref()
    }

    /// Costs what `fact` costs for each record not read since it was
    /// changed.
    fn read_facts(&self, c: &Compute) {
        for i in 0..self.records.len() {
            self.fact(c, i);
        }
    }

    /// The writes a seal makes: what changed, with the UUIDs new entries
    /// and groups take from `next`, in this order: each new entry, then
    /// one group for each kind among them.
    fn ops(
        &self,
        ov: &Overlay<'_>,
        overlay_only: bool,
        next: &mut dyn FnMut() -> [u8; 16],
    ) -> Result<Vec<Op>, Unwritable> {
        let mut ops = Vec::new();
        // What was taken out goes first: a record taken out and put again
        // under its key is written after the key is cleared, not before.
        if !overlay_only {
            for o in &self.removed {
                drop_op(o, &mut ops);
            }
        }
        // The next plan number, when the file has one to give: only a new
        // plan needs it.
        let mut plan = items::next_plan(self.opened.document()).ok();
        for (i, rec) in self.records.iter().enumerate() {
            if ov.remove.contains(&i) {
                drop_op(&self.origin[i], &mut ops);
                continue;
            }
            let (rec, dirty) = match ov.edit.iter().find(|(j, _)| *j == i) {
                Some((_, r)) => (*r, true),
                None if overlay_only && self.unchecked[i].get() => (rec, true),
                None if overlay_only => {
                    // Not checked now, but it takes the plan number it
                    // takes at the seal.
                    if rec.kind == kind::PLAN {
                        put_op(
                            &self.origin[i],
                            rec,
                            self.dirty[i],
                            &mut plan,
                            &mut Vec::new(),
                        )?;
                    }
                    continue;
                }
                None => (rec, self.dirty[i]),
            };
            put_op(&self.origin[i], rec, dirty, &mut plan, &mut ops)?;
        }
        for rec in ov.add {
            fresh_op(rec, &mut plan, &mut ops)?;
        }
        for op in ops.iter_mut() {
            if let Op::Put {
                uuid, new: true, ..
            } = op
            {
                *uuid = next();
            }
        }
        let mut groups: Vec<(u8, [u8; 16])> = Vec::new();
        for op in ops.iter_mut() {
            if let Op::Put {
                group_uuid,
                new: true,
                record,
                ..
            } = op
                && record.kind != kind::ENTRY
            {
                *group_uuid = match groups.iter().find(|(k, _)| *k == record.kind) {
                    Some((_, u)) => *u,
                    None => {
                        let u = next();
                        groups.push((record.kind, u));
                        u
                    }
                };
            }
        }
        Ok(ops)
    }

    /// Costs a walk of the records (the bound). The length of the document a
    /// copy would be made of with `ov` applied: the larger of its opened length and what the bound
    /// gives less the attachments.
    fn copy_len(&self, names: &Names, ov: &Overlay<'_>) -> usize {
        let (document, attachments) = self.opened.held();
        let bound = self.bound(names, ov);
        if bound == usize::MAX {
            document
        } else {
            document.max(bound.saturating_sub(attachments))
        }
    }

    /// Costs a read of the free memory, a copy of the document, a count of
    /// it and a read of each item put. The plaintext a seal would take with
    /// `ov` applied, as an exact count, made only where the memory free at
    /// this moment holds the copy; else [`NoRoom::Short`], and no bound
    /// stands in for it.
    fn used_with(&self, c: &Compute, names: &Names, ov: &Overlay<'_>) -> Result<usize, NoRoom> {
        let len = self.copy_len(names, ov);
        if self.copy_room.fresh().is_some_and(|room| len > room) {
            return Err(NoRoom::Short(len));
        }
        // Each new entry and group gets a UUID of its own, as the seal's:
        // two with one would count as one.
        let ops = self.ops(ov, false, &mut self.stand_ins())?;
        Ok(with_changes(c, names, &ops, false, Some(0), |changes| {
            self.opened.used(c, changes)
        })?)
    }

    /// Costs a parse of a wallet's text for each record changed and, only
    /// past [`Slot::bound`], what [`Slot::used_with`] costs, the first time
    /// after a change. Under the ceiling by the bound it is the bound, with
    /// no copy of the document and no write of it (F206 point 8); past it,
    /// the exact count; `None` when that count cannot be made because
    /// memory is short (not cached). A vault the seal cannot write (a
    /// change it refuses, a file it cannot make) reads as full,
    /// `usize::MAX`: its room shows empty, never as free. A read-only vault
    /// has no changes to hold: 0.
    fn counted(&self, c: &Compute, names: &Names) -> Option<usize> {
        if let Some(n) = self.used.get() {
            return Some(*n);
        }
        let bound = self.bound(names, &Overlay::NONE);
        let counted = if bound <= self.limit() {
            self.room(c, names, &Overlay::NONE).map(|()| bound)
        } else {
            self.used_with(c, names, &Overlay::NONE)
        };
        let n = match counted {
            Ok(n) => n,
            Err(NoRoom::File(fv::file::Error::ReadOnly)) => 0,
            Err(NoRoom::File(_)) => usize::MAX,
            Err(NoRoom::Short(_)) => return None,
        };
        Some(*self.used.get_or_init(|| n))
    }

    /// Costs what [`Slot::counted`] costs. The count, or where memory is
    /// short the bound, which errs high: a figure to show, never to decide
    /// from.
    fn used(&self, c: &Compute, names: &Names) -> usize {
        self.counted(c, names)
            .unwrap_or_else(|| self.bound(names, &Overlay::NONE))
    }
}

/// How a count of a vault's plaintext was refused.
#[derive(Debug)]
enum NoRoom {
    /// The file refuses it ([`fv::file::Error`]).
    File(fv::file::Error),
    /// What is free does not hold the copy of the document the exact count
    /// takes, of this many bytes: memory is short. The vault is not past
    /// anything; nothing is decided from the bound.
    Short(usize),
}

impl From<fv::file::Error> for NoRoom {
    fn from(e: fv::file::Error) -> Self {
        NoRoom::File(e)
    }
}

impl From<Unwritable> for NoRoom {
    fn from(e: Unwritable) -> Self {
        NoRoom::File(e.into())
    }
}

/// Whether a vault with some records put in and others taken out has room
/// ([`OpenVaults::fit`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Room {
    /// It fits: the seal takes it.
    Fits,
    /// It does not: past the ceiling by an exact count, read-only, or a
    /// record the seal cannot write.
    Full,
    /// Whether it fits is not known: the copy of the document of this many
    /// bytes that the exact count takes is more than this computer has free.
    /// Nothing is refused or lost for it but the edit asked about.
    Short(usize),
}

/// The longest document a copy may be made of, as this computer has free
/// memory, shared by the store and its slots. Where a reader is set, a
/// count that copies the document reads it first, so the room is the
/// memory free at that moment.
struct CopyRoom {
    /// The last room, in bytes; [`CopyRoom::NONE`] for none known.
    last: AtomicUsize,
    reader: Mutex<Option<CopyReader>>,
}

/// What reads the room a copy may take now.
type CopyReader = Box<dyn Fn() -> Option<usize> + Send + Sync>;

impl Default for CopyRoom {
    /// No room known, so no copy is refused.
    fn default() -> Self {
        Self {
            last: AtomicUsize::new(Self::NONE),
            reader: Mutex::new(None),
        }
    }
}

impl CopyRoom {
    const NONE: usize = usize::MAX;

    fn get(&self) -> Option<usize> {
        match self.last.load(Ordering::Relaxed) {
            Self::NONE => None,
            n => Some(n),
        }
    }

    fn set(&self, room: Option<usize>) {
        self.last
            .store(room.unwrap_or(Self::NONE), Ordering::Relaxed);
    }

    /// Costs a read of the free memory when a reader is set. The room now.
    fn fresh(&self) -> Option<usize> {
        if let Some(read) = self.reader.lock().ok().as_deref().and_then(Option::as_ref) {
            self.set(read());
        }
        self.get()
    }
}

/// The open vaults. Each is held from its unlock until it is closed or
/// its seal ends in a file: its records, and the document it was read
/// from, are wiped when it goes. The file itself is not kept, only its
/// digest.
///
/// Invariant: a [`VaultId`] this store has made names its slot until
/// [`OpenVaults::close`], [`OpenVaults::clear`] or [`OpenVaults::seal_ended`]
/// (with a file made) drops it, and the opened file does not change. A slot's
/// [`OpenVaults::version`] moves with every edit of its records, and only
/// then; the facts worked out of the key records' words are dropped with
/// it.
#[derive(Default)]
pub struct OpenVaults {
    slots: Vec<Slot>,
    next: u32,
    names: Names,
    /// Less than the ceiling, when told ([`OpenVaults::lower_limit`]).
    limit: Option<usize>,
    /// What the slots are told of the room for a copy.
    copy_room: Arc<CopyRoom>,
    /// The made vault's unlock parts, kept for its unlock
    /// ([`OpenVaults::hold_made_parts`]); wiped by [`OpenVaults::clear`].
    pub(crate) made_parts: Option<crate::parts::UnlockParts>,
}

type Codex32 = faraday_bip::codex32::Codex32;

impl From<NoRoom> for NotSet {
    fn from(e: NoRoom) -> Self {
        match e {
            NoRoom::File(fv::file::Error::Full { .. }) => NotSet::Full,
            NoRoom::Short(len) => NotSet::Short(len),
            NoRoom::File(_) => NotSet::Unwritable,
        }
    }
}

/// Costs a seed and its fingerprint for each combination of strings, at
/// most 16 (one when no index has two strings). A seed of the first `need`
/// indices of a codex32 set that has the fingerprint `fp`, with the set's
/// strings as they made it; `None` for more combinations, or when none
/// makes `fp`.
fn codex32_pick<'a>(
    c: &Compute,
    fp: [u8; 4],
    need: usize,
    list: &'a [(Codex32, Zeroizing<String>)],
    others: &'a [(Codex32, Zeroizing<String>)],
) -> Option<(Zeroizing<Vec<u8>>, Vec<&'a str>)> {
    // The strings of the first `need` indices; where an index has two,
    // each is tried, the first combination whose seed has the fingerprint
    // the records name winning.
    let alts: Vec<Vec<&(Codex32, Zeroizing<String>)>> = list
        .iter()
        .take(need)
        .map(|e| {
            std::iter::once(e)
                .chain(
                    others
                        .iter()
                        .filter(|o| o.0.share_index() == e.0.share_index()),
                )
                .collect()
        })
        .collect();
    let sizes: Vec<usize> = alts.iter().map(Vec::len).collect();
    for pick in combinations(&sizes)? {
        let parts: Vec<Codex32> = pick
            .iter()
            .zip(&alts)
            .filter_map(|(&n, a)| Codex32::parse(&a[n].1).ok())
            .collect();
        let Ok(seed) = crate::operations::codex32_seed(&parts) else {
            continue;
        };
        if crate::identity::fingerprint_of_seed(c, &seed) == Some(Fingerprint(fp)) {
            let texts = list
                .iter()
                .enumerate()
                .map(|(i, (_, t))| match pick.get(i) {
                    Some(&n) => alts[i][n].1.as_str(),
                    None => t.as_str(),
                })
                .collect();
            return Some((seed, texts));
        }
    }
    None
}

/// A seed and the strings that made it.
type Picked = (Zeroizing<Vec<u8>>, Vec<Zeroizing<String>>);

/// Costs what `codex32_pick` costs, for each of at most 16 choices of
/// which strings join `fixed`. Whether the strings of `pool` of the set
/// `fixed[0]` is of (its identifier, threshold and length; never a
/// secret), at indices none of `fixed` has, make with `fixed` a seed of
/// fingerprint `fp`: the first string of each index is tried, then, for an
/// index with two or more, the others, and a wrong string early in `pool`
/// does not hide a right one after it. `None` where fewer than `need`
/// indices are here; else the seed and the strings that made it.
pub(crate) fn codex32_find(
    c: &Compute,
    fp: [u8; 4],
    need: usize,
    fixed: &[&str],
    pool: &[&str],
) -> Option<Option<Picked>> {
    let parse = |t: &str| {
        Some((
            Codex32::parse(t).ok()?,
            Zeroizing::new(t.to_ascii_lowercase()),
        ))
    };
    let fixed: Vec<(Codex32, Zeroizing<String>)> = fixed.iter().filter_map(|t| parse(t)).collect();
    let first = &fixed.first()?.0;
    let mut list: Vec<(Codex32, Zeroizing<String>)> = Vec::new();
    let mut others: Vec<(Codex32, Zeroizing<String>)> = Vec::new();
    for t in pool {
        let Some((s, text)) = parse(t) else {
            continue;
        };
        let i = s.share_index();
        if s.identifier() != first.identifier()
            || s.threshold() != first.threshold()
            || s.payload().len() != first.payload().len()
            || s.is_secret()
            || fixed.iter().any(|(x, _)| x.share_index() == i)
            || list
                .iter()
                .chain(&others)
                .any(|(x, y)| x.share_index() == i && **y == *text)
        {
            continue;
        }
        if list.iter().any(|(x, _)| x.share_index() == i) {
            others.push((s, text));
        } else {
            list.push((s, text));
        }
    }
    let want = need.checked_sub(fixed.len())?;
    if list.len() < want {
        return None;
    }
    // The choices of `want` of the indices, the earlier first.
    let mut at: Vec<usize> = (0..want).collect();
    for _ in 0..16 {
        let chosen: Vec<(Codex32, Zeroizing<String>)> = fixed
            .iter()
            .chain(at.iter().map(|&n| &list[n]))
            .filter_map(|(_, t)| parse(t))
            .collect();
        if let Some((seed, texts)) = codex32_pick(c, fp, need, &chosen, &others) {
            let texts = texts
                .into_iter()
                .map(|t| Zeroizing::new(t.to_string()))
                .collect();
            return Some(Some((seed, texts)));
        }
        // The next choice: the last place that can move, moved on.
        let Some(p) = (0..want).rev().find(|&p| at[p] < list.len() - (want - p)) else {
            break;
        };
        at[p] += 1;
        for q in p + 1..want {
            at[q] = at[q - 1] + 1;
        }
    }
    Some(None)
}

/// Every way to pick one of each of `sizes` things, as the choice made at
/// each place; `None` for more than 16 ways.
fn combinations(sizes: &[usize]) -> Option<Vec<Vec<usize>>> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new()];
    for &n in sizes {
        if out.len().checked_mul(n)? > 16 {
            return None;
        }
        out = out
            .into_iter()
            .flat_map(|p| {
                (0..n).map(move |k| {
                    let mut q = p.clone();
                    q.push(k);
                    q
                })
            })
            .collect();
    }
    Some(out)
}

/// Why a form was not written: the vault has no room for it, or the seal
/// would refuse what it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotSet {
    /// The vault does not have room for the record.
    Full,
    /// The record is one the seal cannot write.
    Unwritable,
    /// What is free does not hold the copy of the document, of this many
    /// bytes, that finding out takes: memory is short.
    Short(usize),
}

/// Sets field `number` of `r` as [`Record::set`] does, except that a
/// required field left empty stays as an empty field: a note with no text
/// and an entry with no title are still records.
fn set_field(r: &mut Record, number: u8, bytes: &[u8]) {
    r.set(number, bytes);
    keep_required(r, &[number]);
}

/// Puts back, empty, each field of `emptied` that the record requires.
fn keep_required(r: &mut Record, emptied: &[u8]) {
    while let Err(records::Fault::Missing(_, n)) = r.check() {
        if !emptied.contains(&n) {
            break;
        }
        r.push(n, &[]);
        r.fields.sort_by_key(|f| f.number);
    }
}

/// The random bytes one seal draws (`faraday_vault::kdbx::RANDOM_LEN`).
pub const SEAL_RANDOM_LEN: usize = fv::kdbx::RANDOM_LEN;

/// The set a codex32 share belongs to (its threshold, its set's identifier,
/// its index and its key): the grouping `faraday_vault`'s `read` gives `related` by. A
/// lookup: the string's header is read in place, either case.
fn share_set_of(r: &Record) -> Option<Vec<u8>> {
    if r.kind != kind::CODEX32_SHARE {
        return None;
    }
    let (k, id, index) = codex32_header(r)?;
    let key = r.field(field::CODEX32_KEY)?;
    Some(
        [
            &[k, index.to_ascii_lowercase()][..],
            &id.to_ascii_lowercase(),
            key,
        ]
        .concat(),
    )
}

/// Unix seconds of an RFC 3339 time in UTC as `faraday.sealed` is written
/// (`2026-10-28T14:02:00Z`).
fn unix_of_rfc3339(t: &str) -> Option<u64> {
    let b = t.as_bytes();
    if b.len() != 20
        || [
            (4, b'-'),
            (7, b'-'),
            (10, b'T'),
            (13, b':'),
            (16, b':'),
            (19, b'Z'),
        ]
        .iter()
        .any(|&(i, ch)| b[i] != ch)
    {
        return None;
    }
    let n = |a: usize, z: usize| -> Option<i64> {
        let f = b.get(a..z)?;
        if !f.iter().all(u8::is_ascii_digit) {
            return None;
        }
        Some(f.iter().fold(0, |v, &c| v * 10 + i64::from(c - b'0')))
    };
    let (y, m, d) = (n(0, 4)?, n(5, 7)?, n(8, 10)?);
    let (h, mi, s) = (n(11, 13)?, n(14, 16)?, n(17, 19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let days = faraday_crypto::date::days_from_civil(y, m as u32, d as u32);
    // A day past its month's end (2026-02-31) names no day.
    if faraday_crypto::date::civil_from_days(days) != (y, m as u32, d as u32) {
        return None;
    }
    u64::try_from(days * 86_400 + h * 3_600 + mi * 60 + s).ok()
}

mod edits;
mod reveals;
mod seal;
#[cfg(test)]
mod tests;

/// How every BIP-85 derivation path begins, its purpose: a line of an
/// entry's notes so written is no part of what a KeePass export carries
/// (F172).
const BIP85_PATH: &str = "m/83696968'";

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

/// When the revocation certificate a GPG record keeps was made: a public
/// date, read from its signature packet. `None` for a record that keeps
/// none.
pub fn gpg_revocation_kept_at(r: RecordView<'_>) -> Option<u32> {
    if r.kind() != kind::GPG {
        return None;
    }
    faraday_pgp::signature_time(r.raw().field(gpg_field::REVOCATION)?)
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

/// A copy of the record's key bytes, no signature made: the key a GPG
/// record (type 7) holds, its seeds wiped when it drops. `None` for a record without a signing subkey.
pub(crate) fn gpg_key(r: &Record) -> Option<faraday_pgp::Key> {
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

/// Costs a SHA-256 over the record's payload and passphrase. A key
/// record's id: the SHA-256 of its payload and the passphrase stored with
/// it, by which what is worked out of it is kept. `None` for a record
/// with no key.
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

/// Costs a SHA-256 over the record's string and key. A codex32 share
/// record's id: the SHA-256 of its string and the fingerprint it names, by
/// which a share is known as it arrived. `None` for a record with no
/// share.
pub fn share_record_id(r: RecordView<'_>) -> Option<[u8; 32]> {
    use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
    if r.kind() != kind::CODEX32_SHARE {
        return None;
    }
    let r = r.raw();
    let mut engine = sha256::Hash::engine();
    engine.input(r.field(field::CODEX32_STRING)?);
    engine.input(&[0]);
    engine.input(r.field(field::CODEX32_KEY).unwrap_or_default());
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

/// A codex32 share record's header, the set's plain facts it states: its
/// threshold, its identifier and its share index (lower case), read from
/// the string's first nine characters (`ms1`, then one each, four, one).
/// No more of the string. `None` for no such record. A lookup.
pub fn codex32_share_header(r: RecordView<'_>) -> Option<(u8, [u8; 4], u8)> {
    (r.kind() == kind::CODEX32_SHARE).then(|| codex32_header(r.raw()))?
}

/// The identifier of the codex32 set a key record keeps (lower case), the
/// plain fact the record states; `None` for a record that is not a key or
/// keeps no set. No key material. A lookup.
pub fn codex32_key_set_identifier(r: RecordView<'_>) -> Option<[u8; 4]> {
    (r.kind() == kind::KEY)
        .then(|| codex32_set_of(r.raw()))?
        .map(|f| f.identifier.map(|b| b.to_ascii_lowercase()))
}

/// Whether `r` is a codex32 share record of the key with fingerprint
/// `fp`, of the set `set` (its threshold and identifier, either case), at
/// share index `index` where one is given (a bech32 character, lower
/// case). A lookup.
pub fn codex32_share_is(
    r: RecordView<'_>,
    fp: [u8; 4],
    (k, id): (u8, [u8; 4]),
    index: Option<u8>,
) -> bool {
    r.public(field::CODEX32_KEY) == Some(&fp[..])
        && codex32_share_header(r).is_some_and(|(rk, rid, j)| {
            rk == k
                && rid.eq_ignore_ascii_case(&id)
                && index.is_none_or(|x| x == j.to_ascii_lowercase())
        })
}

fn codex32_header(r: &Record) -> Option<(u8, [u8; 4], u8)> {
    let b = r.field(field::CODEX32_STRING)?;
    let head = b.get(..9)?;
    let k = head[3].checked_sub(b'0').filter(|k| *k <= 9)?;
    let mut id = [0u8; 4];
    id.copy_from_slice(&head[4..8]);
    Some((k, id, head[8]))
}

/// The codex32 set a key record keeps, its plain facts
/// (`docs/CODEX32.md` §9.4); `None` where it keeps none.
fn codex32_set_of(r: &Record) -> Option<fv::codex32::SetFacts> {
    fv::codex32::set_of(r.field(field::KEY_CODEX32)?)
}

/// The BIP-85 key a key record says it is a child seed of (F166);
/// `None` where it says none.
fn child_of_of(r: &Record) -> Option<fv::child::ChildOf> {
    fv::child::child_of(r.field(field::KEY_CHILD)?)
}

/// The master seed a key record holds, when it holds a seed rather than
/// words.
fn seed_of(r: &Record) -> Option<Zeroizing<Vec<u8>>> {
    match layouts::payload_of(layouts::KIND_SEED, r.field(field::KEY)?)? {
        layouts::Opened::Seed(s) => Some(s),
        _ => None,
    }
}

/// A wallet recovery sheet record's descriptor, name and note as text.
/// Empty for a record that holds none. A sheet is public in full.
pub fn sheet_parts(r: RecordView<'_>) -> (String, String, String) {
    match r
        .public(field::SHEET)
        .and_then(|p| layouts::payload_of(layouts::KIND_SHEET, p))
    {
        Some(layouts::Opened::Sheet(s)) => (
            String::from_utf8_lossy(&s.descriptor).into_owned(),
            String::from_utf8_lossy(&s.name).into_owned(),
            String::from_utf8_lossy(&s.note).into_owned(),
        ),
        _ => Default::default(),
    }
}

/// Costs an Argon2id run at `cost`, as `faraday_vault::file::create` does.
/// Makes a vault: the root group `id`, named `label`, sealed under
/// `passphrase`, stamped as the first seal at `now`. The file is
/// encrypted, so public.
pub fn create(
    c: &Compute,
    id: [u8; 16],
    label: &str,
    cost: fv::Cost,
    passphrase: &str,
    random: &[u8; SEAL_RANDOM_LEN],
    now: Option<u64>,
) -> Result<Vec<u8>, fv::file::Error> {
    fv::file::create(c, id, label, cost, passphrase, random, now)
}

/// Costs an Argon2id run at the file's stated cost, thrown away: the time
/// a vault's unlock takes on this computer, measured. A pass that could
/// not run (the memory is not there) is the error, with no time.
pub fn measure(c: &Compute, facts: &fv::file::Facts) -> Result<(), fv::file::Error> {
    fv::file::measure(c, facts)
}
