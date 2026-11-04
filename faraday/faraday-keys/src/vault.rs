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

use std::cell::OnceCell;
use std::collections::BTreeSet;
use std::sync::Arc;

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
pub use items::{Related, RelatedKind};
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

    /// Costs a document write and a read of each record changed, the first
    /// time after a change (a lookup after). The bytes of plaintext the
    /// next seal takes, exactly (`faraday_vault::file::Opened::used`);
    /// 0 for a vault that is read-only or not held.
    pub fn used(&self, c: &Compute) -> usize {
        self.slot.map_or(0, |(s, names)| s.used(c, names))
    }

    /// Costs what [`Records::used`] costs. Whether the seal refuses to
    /// write this vault as it is (a record it cannot write, a file it
    /// cannot make): then its size says nothing, and taking records out
    /// to make room cannot help unless the refused one goes.
    pub fn unwritable(&self, c: &Compute) -> bool {
        self.used(c) == usize::MAX
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
        path: Vec<String>,
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
                    path: it.path.clone(),
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

/// One open file of one vault.
struct Slot {
    id: VaultId,
    /// The file as it was opened: its digest tells copies apart.
    file: Vec<u8>,
    /// The file opened. Shared with a seal job, which never changes it.
    opened: Arc<fv::file::Opened>,
    /// The app's view: the label, the entries, the plans and signed amounts.
    records: Vec<Record>,
    /// Where each record was read from, by place.
    origin: Vec<Origin>,
    /// Whether each record was changed since it was read, by place.
    dirty: Vec<bool>,
    /// What was read and has been taken out: removed at the seal.
    removed: Vec<Origin>,
    /// What does not read: the file's own, and the GPG keys this store
    /// could not take.
    unreadable: Vec<Unreadable>,
    /// Moves with every edit of the records.
    version: u64,
    /// What is worked out of each key record's words, by record place;
    /// dropped by every edit.
    facts: OnceCell<Vec<Option<Fact>>>,
    /// The plaintext the next seal takes; dropped by every edit.
    used: OnceCell<usize>,
    /// How many bytes of UUIDs the next seal takes ([`OpenVaults::uuid_bytes`]),
    /// 0 when it refuses; dropped by every edit.
    uuids: OnceCell<usize>,
    /// A seal job holds the file: every edit refuses until it ends.
    sealing: bool,
    /// The passphrase the next seal writes, when it is not the one the file
    /// opened with; wiped as it drops or is replaced (never exposed).
    new_passphrase: Option<SecretText>,
    /// The memory step, in KiB, the next seal writes when it is not the
    /// file's own.
    new_step: Option<u32>,
}

impl Slot {
    /// Every edit ends here: the version moves and what was worked out of
    /// the records is dropped.
    fn changed(&mut self) {
        self.version += 1;
        self.facts = OnceCell::new();
        self.used = OnceCell::new();
        self.uuids = OnceCell::new();
    }

    /// Record `i` was changed.
    fn touch(&mut self, i: usize) {
        if let Some(d) = self.dirty.get_mut(i) {
            *d = true;
        }
        self.changed();
    }

    /// Whether the vault may not be changed: read-only, or sealing.
    fn locked(&self) -> bool {
        self.sealing || self.opened.read_only().is_some()
    }

    /// The most plaintext a seal may take.
    fn limit(&self) -> usize {
        self.opened
            .facts()
            .size
            .unwrap_or(fv::file::SIZES[fv::file::SIZES.len() - 1])
    }

    /// Costs a parse of each key record's words and a hash of each, the
    /// first time after a change.
    fn facts(&self, c: &Compute) -> &Vec<Option<Fact>> {
        self.facts.get_or_init(|| {
            self.records
                .iter()
                .map(|r| {
                    let m = (r.kind == kind::KEY)
                        .then(|| record_words(c, r))
                        .flatten()?;
                    Some(Fact {
                        words: m.word_count(),
                        language: m.language(),
                        id: words_id(c, &m),
                    })
                })
                .collect()
        })
    }

    /// The writes a seal makes: what changed, with the UUIDs new entries
    /// and groups take from `next`, in this order: each new entry, then
    /// one group for each kind among them.
    fn ops(
        &self,
        ov: &Overlay<'_>,
        next: &mut dyn FnMut() -> [u8; 16],
    ) -> Result<Vec<Op>, Unwritable> {
        let mut ops = Vec::new();
        // What was taken out goes first: a record taken out and put again
        // under its key is written after the key is cleared, not before.
        for o in &self.removed {
            drop_op(o, &mut ops);
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

    /// Costs a document write and a read of each item put. The plaintext
    /// a seal would take with `ov` applied.
    fn used_with(
        &self,
        c: &Compute,
        names: &Names,
        ov: &Overlay<'_>,
    ) -> Result<usize, fv::file::Error> {
        // Each new entry and group gets a UUID of its own, as the seal's:
        // two with one would count as one.
        let mut n = 0u64;
        let ops = self.ops(ov, &mut || {
            n += 1;
            let mut u = [0u8; 16];
            u[..8].copy_from_slice(&n.to_le_bytes());
            u
        })?;
        with_changes(c, names, &ops, false, Some(0), |changes| {
            self.opened.used(c, changes)
        })
    }

    /// Costs what [`Slot::used_with`] costs, the first time after a change.
    /// A vault the seal cannot write (a change it refuses, a file it cannot
    /// make) reads as full, `usize::MAX`: its room shows empty, never as
    /// free. A read-only vault has no changes to hold: 0.
    fn used(&self, c: &Compute, names: &Names) -> usize {
        *self
            .used
            .get_or_init(|| match self.used_with(c, names, &Overlay::NONE) {
                Ok(n) => n,
                Err(fv::file::Error::ReadOnly) => 0,
                Err(_) => usize::MAX,
            })
    }
}

/// The open vaults. Each is held from its unlock until it is closed or
/// its seal ends in a file, with the file it was opened from; both are wiped
/// when it goes.
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
}

/// The vault does not have room for an edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Full;

type Codex32 = faraday_bip::codex32::Codex32;

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

/// Items of a person's entries and the plans that group by one key: the
/// grouping `faraday_vault`'s `read` gives `related` by.
fn pair_of(r: &Record) -> Option<(RelatedKind, Vec<u8>)> {
    match r.kind {
        kind::PASSPHRASE => r
            .field(field::PASSPHRASE_KEY)
            .map(|k| (RelatedKind::Passphrases, k.to_vec())),
        kind::CODEX32_SHARE => {
            let index = *r.field(field::CODEX32_STRING)?.get(8)?;
            r.field(field::CODEX32_KEY)
                .map(|k| (RelatedKind::Shares, [&[index][..], k].concat()))
        }
        _ => None,
    }
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
    let n = |a: usize, z: usize| t.get(a..z)?.parse::<i64>().ok();
    let (y, m, d) = (n(0, 4)?, n(5, 7)?, n(8, 10)?);
    let (h, mi, s) = (n(11, 13)?, n(14, 16)?, n(17, 19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + h * 3_600 + mi * 60 + s).ok()
}

impl OpenVaults {
    fn slot(&self, id: VaultId) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }

    /// The slot to edit: none for an id not held or a read-only vault.
    fn slot_mut(&mut self, id: VaultId) -> Option<&mut Slot> {
        self.slots
            .iter_mut()
            .find(|s| s.id == id)
            .filter(|s| !s.locked())
    }

    /// Sets the names new groups and titles take, for every vault. Set
    /// once, before the first edit.
    pub fn set_names(&mut self, names: Names) {
        self.names = names;
        for s in &mut self.slots {
            s.used = OnceCell::new();
            s.uuids = OnceCell::new();
        }
    }

    /// Costs an Argon2id run over the vault's memory, as
    /// `faraday_vault::file::open` does, and the reading of each GPG key.
    /// Opens `file` under `passphrase` and keeps it, with its records.
    /// Gives its id and the file's facts, which are public. The
    /// passphrase is lent, and the caller wipes its copy.
    pub fn unlock(
        &mut self,
        c: &Compute,
        file: Vec<u8>,
        passphrase: &str,
    ) -> Result<(VaultId, fv::file::Facts), fv::file::Error> {
        let opened = fv::file::open(c, &file, passphrase)?;
        let built = build(&opened);
        Ok(self.keep(file, Arc::new(opened), built))
    }

    /// Keeps the file a [`crate::job::StretchJob`] opened, on the thread
    /// that holds the store: no Argon2id run. Gives its id and the file's
    /// facts, or why the job could not open it. An answer dropped unkept
    /// is wiped.
    pub fn adopt(
        &mut self,
        stretched: crate::job::Stretched,
    ) -> Result<(VaultId, fv::file::Facts), fv::file::Error> {
        let slot = stretched.into_opened()?;
        let crate::job::OpenedSlot {
            file,
            opened,
            built,
        } = *slot;
        Ok(self.keep(file, opened, built))
    }

    fn keep(
        &mut self,
        file: Vec<u8>,
        opened: Arc<fv::file::Opened>,
        built: Built,
    ) -> (VaultId, fv::file::Facts) {
        let facts = opened.facts().clone();
        let id = VaultId(self.next);
        self.next += 1;
        let Built {
            records,
            origin,
            unreadable,
        } = built;
        self.slots.push(Slot {
            id,
            file,
            opened,
            dirty: vec![false; records.len()],
            records,
            origin,
            removed: Vec::new(),
            unreadable,
            version: 0,
            facts: OnceCell::new(),
            used: OnceCell::new(),
            uuids: OnceCell::new(),
            sealing: false,
            new_passphrase: None,
            new_step: None,
        });
        (id, facts)
    }

    /// Closes open vault `id`: its records are wiped.
    pub fn close(&mut self, id: VaultId) {
        self.slots.retain(|s| s.id != id);
    }

    /// Closes every open vault.
    pub fn clear(&mut self) {
        self.slots.clear();
    }

    /// A lookup once read, which [`OpenVaults::warm`] does before a frame.
    /// How many bytes of `uuids` [`OpenVaults::seal_job`] takes for open
    /// vault `id`: 16 for each new entry and for each kind's group among
    /// them. `None` for an id this store does not hold. The first read after
    /// an edit works the seal's writes out, which clones each record
    /// changed (its bytes in wiping types) and counts the UUIDs they take;
    /// the answer is kept until the next edit.
    pub fn uuid_bytes(&self, id: VaultId) -> Option<usize> {
        let s = self.slot(id)?;
        Some(*s.uuids.get_or_init(|| {
            let mut n = 0;
            // A seal that refuses draws nothing: its job carries the refusal.
            let ok = s
                .ops(&Overlay::NONE, &mut || {
                    n += 1;
                    [0; 16]
                })
                .is_ok();
            if ok { n * 16 } else { 0 }
        }))
    }

    /// Makes the job that seals open vault `id`, which stays in the
    /// store, marked sealing (every edit refuses) until
    /// [`OpenVaults::seal_ended`]. The job's file is written with the changes: its changes written, the seal count one higher with the
    /// time `now` (Unix seconds, `None` when the clock is not set), under
    /// `random` and with the new entries' and groups' UUIDs from `uuids`
    /// ([`OpenVaults::uuid_bytes`] long). `None`, and the vault not
    /// marked, for an id this store does not hold, a read-only or already
    /// sealing vault or too few `uuids`.
    pub fn seal_job(
        &mut self,
        id: VaultId,
        random: &[u8; SEAL_RANDOM_LEN],
        uuids: &[u8],
        now: Option<u64>,
    ) -> Option<crate::job::SealJob> {
        let at = self.slots.iter().position(|s| s.id == id)?;
        let s = &self.slots[at];
        if s.locked() {
            return None;
        }
        let mut chunks = uuids.chunks_exact(16);
        let mut taken = 0;
        let mut short = false;
        let ops = s.ops(&Overlay::NONE, &mut || {
            taken += 1;
            match chunks.next().and_then(|u| <[u8; 16]>::try_from(u).ok()) {
                Some(u) => u,
                None => {
                    short = true;
                    [0; 16]
                }
            }
        });
        if short && ops.is_ok() {
            return None;
        }
        let rekey = (
            s.new_passphrase
                .as_ref()
                .map(|p| SecretText::of(p.as_str())),
            s.new_step,
        );
        // A change the seal cannot write is its refusal, not a change left out.
        let ops = ops.map_err(fv::file::Error::from);
        let s = &mut self.slots[at];
        s.sealing = true;
        Some(crate::job::SealJob::new(
            Arc::clone(&s.opened),
            ops,
            self.names.clone(),
            rekey,
            random,
            now,
        ))
    }

    /// Takes `passphrase` (moved in, wiped as it is replaced or the vault
    /// closes) as the one the next seal of open vault `id` writes, in place
    /// of the one the file was opened with. It counts as a change: the
    /// version moves. Refused, and the passphrase wiped, for an empty one,
    /// a vault this store does not hold and a read-only or sealing one.
    pub fn set_passphrase(&mut self, id: VaultId, passphrase: SecretText) -> bool {
        if passphrase.as_str().is_empty() {
            return false;
        }
        let Some(s) = self.slots.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if s.locked() {
            return false;
        }
        s.new_passphrase = Some(passphrase);
        s.changed();
        true
    }

    /// Takes `memory_kib` as the memory step the next seal of open vault
    /// `id` writes, up or down (passes stay the file's or 3, over 4
    /// lanes). It counts as a change. Refused for a step that is not one of
    /// [`fv::file::STEPS_KIB`], an unknown vault and a read-only or sealing
    /// one.
    pub fn set_step(&mut self, id: VaultId, memory_kib: u32) -> bool {
        if !fv::file::STEPS_KIB.contains(&memory_kib) {
            return false;
        }
        let Some(s) = self.slots.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if s.locked() {
            return false;
        }
        s.new_step = Some(memory_kib);
        s.changed();
        true
    }

    /// A lookup. The cost the next seal of open vault `id` writes: its
    /// memory step is the one taken ([`OpenVaults::set_step`]) or else the
    /// file's own rounded up (§7.1, a file a KeePass app saved is written
    /// in Faraday's form). `None` for an id this store does not hold.
    pub fn next_cost(&self, id: VaultId) -> Option<fv::Cost> {
        let s = self.slot(id)?;
        Some(s.opened.next_cost_at(s.new_step))
    }

    /// A lookup. Whether open vault `id` has a new passphrase or a new
    /// memory step waiting for its seal: a change, so a lock seals it.
    pub fn rekeyed(&self, id: VaultId) -> bool {
        self.slot(id)
            .is_some_and(|s| s.new_passphrase.is_some() || s.new_step.is_some())
    }

    /// A seal job of open vault `id` ended: with `ok` the vault is closed
    /// (its file was made); else it stays open with its changes and may
    /// be edited and sealed again.
    pub fn seal_ended(&mut self, id: VaultId, ok: bool) {
        if ok {
            self.slots.retain(|s| s.id != id);
        } else if let Some(s) = self.slots.iter_mut().find(|s| s.id == id) {
            s.sealing = false;
        }
    }

    /// A lookup. Open vault `id`'s identifier (the root group's UUID in
    /// hex) and its seal count with the time of the last seal, as the file
    /// was opened. `None` for an id this store does not hold.
    pub fn seal_state(&self, id: VaultId) -> Option<([u8; 32], Seals)> {
        let s = self.slot(id)?;
        let read = s.opened.read();
        Some((
            s.opened.vault_id(),
            Seals {
                count: read.seals,
                last: read.sealed.as_deref().and_then(unix_of_rfc3339),
            },
        ))
    }

    /// Costs a SHA-256 over the whole file (up to 4 MiB, counted as a file
    /// hash). The digest of open vault `id`'s file as it was opened: what
    /// tells two seals of one count apart. `None` for an id this store
    /// does not hold.
    pub fn opened_digest(&self, c: &Compute, id: VaultId) -> Option<[u8; 32]> {
        Some(faraday_crypto::sha256_whole(c, &self.slot(id)?.file))
    }

    // ---- reading ----

    /// A lookup. Open vault `id`'s records, for reading. Empty for an id
    /// this store has not made or has closed.
    pub fn records(&self, id: VaultId) -> Records<'_> {
        let s = self.slot(id);
        Records {
            id,
            contents: s.map_or(&[][..], |s| &s.records),
            slot: s.map(|s| (s, &self.names)),
        }
    }

    /// A lookup. How many edits open vault `id`'s records have had since
    /// it was unlocked: what a caller's record of the vault is kept in
    /// step by. Zero for a vault this store does not hold.
    pub fn version(&self, id: VaultId) -> u64 {
        self.slot(id).map_or(0, |s| s.version)
    }

    /// A lookup. What open vault `id`'s file does not read: items and
    /// keys with where and what, and GPG keys not in Faraday's shape.
    /// Written back as read. No secrets.
    pub fn unreadable(&self, id: VaultId) -> &[Unreadable] {
        self.slot(id).map_or(&[][..], |s| &s.unreadable)
    }

    /// A lookup. Why open vault `id` may not be changed, if it may not:
    /// every edit refuses while this is `Some`.
    pub fn read_only(&self, id: VaultId) -> Option<fv::file::ReadOnly> {
        self.slot(id)?.opened.read_only()
    }

    /// A lookup. What the clear header of open vault `id`'s file states.
    pub fn facts(&self, id: VaultId) -> Option<&fv::file::Facts> {
        Some(self.slot(id)?.opened.facts())
    }

    /// A lookup. How many entries of open vault `id` have history and how
    /// many were in the recycle bin, which the next seal leaves out.
    pub fn history(&self, id: VaultId) -> Option<(usize, usize)> {
        let read = self.slot(id)?.opened.read();
        Some((read.history, read.recycle_bin))
    }

    /// A lookup. Open vault `id`'s identifier: the root group's UUID in
    /// hex.
    pub fn vault_id(&self, id: VaultId) -> Option<[u8; 32]> {
        Some(self.slot(id)?.opened.vault_id())
    }

    /// A lookup. The names of the groups record `i` of open vault `id`
    /// was read from, under the root; empty for the root, a new record or
    /// one not an entry.
    pub fn path(&self, id: VaultId, i: usize) -> &[String] {
        match self.slot(id).and_then(|s| s.origin.get(i)) {
            Some(Origin::Entry { path, .. }) => path,
            _ => &[],
        }
    }

    /// A lookup. Record `i`'s place among the records of open vault `id`
    /// that hold a related secret (passphrases for one key, codex32 shares
    /// of one index and key), in record order; `None` for one that is
    /// alone or holds none.
    pub fn related(&self, id: VaultId, i: usize) -> Option<Related> {
        let s = self.slot(id)?;
        let (kind, key) = pair_of(s.records.get(i)?)?;
        let (mut of, mut place) = (0, 0);
        for (j, r) in s.records.iter().enumerate() {
            if pair_of(r).is_some_and(|(k, v)| k == kind && v == key) {
                of += 1;
                if j <= i {
                    place += 1;
                }
            }
        }
        (of > 1).then_some(Related { kind, place, of })
    }

    /// Costs a document write and a read of each item put (what
    /// [`Records::used`] costs). Whether open vault `id` still has room
    /// after records `remove` (by place) are taken out and the records
    /// `add` are put in: the exact plaintext the seal would take, against
    /// the file's size. False for a vault that is read-only or not held.
    pub fn fits<R: Fit>(&self, c: &Compute, id: VaultId, remove: &[usize], add: &[&R]) -> bool {
        let Some(s) = self.slot(id) else {
            return false;
        };
        let add: Vec<&Record> = add.iter().map(|r| r.record_of(Inside(()))).collect();
        let add = &add[..];
        let ov = Overlay {
            remove,
            add,
            edit: &[],
        };
        s.used_with(c, &self.names, &ov)
            .is_ok_and(|n| n <= s.limit())
    }

    // ---- edits ----

    /// Puts `record` last in open vault `id`; its place. `None` for a
    /// vault this store does not hold or that is read-only.
    pub fn insert(&mut self, id: VaultId, record: impl IntoRecord) -> Option<usize> {
        let s = self.slot_mut(id)?;
        s.records.push(record.into_record());
        s.origin.push(Origin::New);
        s.dirty.push(true);
        s.changed();
        Some(s.records.len() - 1)
    }

    /// Puts `record` at place `at` in open vault `id`, the records from
    /// there on moving up.
    pub fn insert_at(&mut self, id: VaultId, at: usize, record: impl IntoRecord) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let at = at.min(s.records.len());
        s.records.insert(at, record.into_record());
        s.origin.insert(at, Origin::New);
        s.dirty.insert(at, true);
        s.changed();
        true
    }

    /// Puts `record` in place of record `i` of open vault `id`.
    pub fn replace(&mut self, id: VaultId, i: usize, record: impl IntoRecord) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        *r = record.into_record();
        s.touch(i);
        true
    }

    /// Takes record `i` out of open vault `id`.
    pub fn remove(&mut self, id: VaultId, i: usize) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        if i >= s.records.len() {
            return false;
        }
        s.records.remove(i);
        s.dirty.remove(i);
        s.removed.push(s.origin.remove(i));
        s.changed();
        true
    }

    /// Takes out of open vault `id` every record `gone` says goes; how
    /// many went. `gone` is asked once per record, in order; the `home` of
    /// the view it is given is the record's place now, shifted down by the
    /// records that went before it, not its place before the call (count
    /// the calls for that).
    pub fn remove_where(
        &mut self,
        id: VaultId,
        mut gone: impl FnMut(RecordView<'_>) -> bool,
    ) -> usize {
        let Some(s) = self.slot_mut(id) else {
            return 0;
        };
        let mut n = 0;
        let mut at = 0;
        while at < s.records.len() {
            let view = RecordView {
                record: &s.records[at],
                home: Some((id, at)),
            };
            if gone(view) {
                s.records.remove(at);
                s.dirty.remove(at);
                s.removed.push(s.origin.remove(at));
                n += 1;
            } else {
                at += 1;
            }
        }
        if n > 0 {
            s.changed();
        }
        n
    }

    /// Sets public field `number` of record `i` of open vault `id`; an
    /// empty `bytes` takes the field out. `false` for a secret field
    /// ([`is_secret`]), which only [`OpenVaults::set_secret`] writes, or
    /// no such record. No check of the vault's size: for what a seal or
    /// the plan's own bookkeeping writes; a person's edit is
    /// [`OpenVaults::set_publics_in`].
    pub fn set_public(&mut self, id: VaultId, i: usize, number: u8, bytes: &[u8]) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        if is_secret(r.kind, number) {
            return false;
        }
        set_field(r, number, bytes);
        s.touch(i);
        true
    }

    /// Sets a repeatable public field of record `i` of open vault `id` to
    /// `values`, in order, the values it had gone. `false` for a secret
    /// field or no such record. No check of the vault's size: a person's
    /// edit is [`OpenVaults::set_public_all_in`].
    pub fn set_public_all(&mut self, id: VaultId, i: usize, number: u8, values: &[&[u8]]) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        if is_secret(r.kind, number) {
            return false;
        }
        r.fields.retain(|f| f.number != number);
        for v in values {
            r.push(number, v);
        }
        s.touch(i);
        true
    }

    /// Costs what [`OpenVaults::fits`] costs. Sets each of `fields` (a
    /// public field's number and bytes; empty takes it out) of record `i`
    /// of open vault `id` together, as [`OpenVaults::set_public`] does; or,
    /// when the vault would then not fit its size, none of them ([`Full`]).
    /// A secret field, no such record or a vault read-only is refused the
    /// same way.
    pub fn set_publics_in(
        &mut self,
        c: &Compute,
        id: VaultId,
        i: usize,
        fields: &[(u8, &[u8])],
    ) -> Result<(), Full> {
        self.edit_record_in(c, id, i, |r| {
            for &(number, bytes) in fields {
                if is_secret(r.kind, number) {
                    return Err(Full);
                }
                set_field(r, number, bytes);
            }
            Ok(())
        })
    }

    /// Costs what [`OpenVaults::fits`] costs. Record `i` of open vault
    /// `id` as `edit` changes it, kept only when the vault then fits its
    /// size; otherwise nothing changes ([`Full`]).
    fn edit_record_in(
        &mut self,
        c: &Compute,
        id: VaultId,
        i: usize,
        edit: impl FnOnce(&mut Record) -> Result<(), Full>,
    ) -> Result<(), Full> {
        let s = self.slot(id).filter(|s| !s.locked()).ok_or(Full)?;
        let mut r = s.records.get(i).ok_or(Full)?.clone();
        edit(&mut r)?;
        let edit = [(i, &r)];
        let ov = Overlay {
            remove: &[],
            add: &[],
            edit: &edit,
        };
        if s.used_with(c, &self.names, &ov)
            .map_or(true, |n| n > s.limit())
        {
            return Err(Full);
        }
        let s = self.slot_mut(id).ok_or(Full)?;
        s.records[i] = r;
        s.touch(i);
        Ok(())
    }

    /// Costs what [`OpenVaults::fits`] costs. Sets a repeatable public
    /// field of every record in `at` of open vault `id` to `values`, as
    /// [`OpenVaults::set_public_all`] does; or, when the vault would then
    /// not fit its size, none of them ([`Full`]).
    pub fn set_public_all_in(
        &mut self,
        c: &Compute,
        id: VaultId,
        at: &[usize],
        number: u8,
        values: &[&[u8]],
    ) -> Result<(), Full> {
        let s = self.slot(id).filter(|s| !s.locked()).ok_or(Full)?;
        let mut grown: Vec<(usize, Record)> = Vec::new();
        for &i in at {
            if grown.iter().any(|(j, _)| *j == i) {
                continue;
            }
            let mut r = s.records.get(i).ok_or(Full)?.clone();
            if is_secret(r.kind, number) {
                return Err(Full);
            }
            r.fields.retain(|f| f.number != number);
            for v in values {
                r.push(number, v);
            }
            grown.push((i, r));
        }
        let edit: Vec<(usize, &Record)> = grown.iter().map(|(i, r)| (*i, r)).collect();
        let ov = Overlay {
            remove: &[],
            add: &[],
            edit: &edit,
        };
        if s.used_with(c, &self.names, &ov)
            .map_or(true, |n| n > s.limit())
        {
            return Err(Full);
        }
        let s = self.slot_mut(id).ok_or(Full)?;
        for (i, r) in grown {
            s.records[i] = r;
            s.touch(i);
        }
        Ok(())
    }

    /// Sets secret field `number` of record `i` of open vault `id` to
    /// input the caller holds (a typed password), which the caller wipes
    /// when it drops; an empty `bytes` takes the field out. `false` for a
    /// public field or no such record. No check of the vault's size: a
    /// person's edit is [`OpenVaults::set_fields`].
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
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        if !is_secret(r.kind, number) {
            return false;
        }
        set_field(r, number, bytes);
        s.touch(i);
        true
    }

    /// Sets whether key or wallet record `i` of open vault `id` loads at
    /// unlock, keeping its other flags.
    pub fn set_loads_at_unlock(&mut self, id: VaultId, i: usize, loads: bool) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        if !matches!(r.kind, kind::KEY | kind::WALLET) {
            return false;
        }
        records::set_loads_at_unlock(r, loads);
        s.touch(i);
        true
    }

    /// Costs what [`OpenVaults::fits`] costs. Writes a form into open
    /// vault `id`: record `at` (or, with `None` or a place with no
    /// record, a new record of type `kind`) gets each of `fields`, a field
    /// with no bytes taken out, as the caller typed them. A record that
    /// does not fit afterwards is left as it was ([`Full`]). Gives the
    /// record's place.
    pub fn set_fields(
        &mut self,
        c: &Compute,
        id: VaultId,
        at: Option<usize>,
        k: u8,
        fields: &[(u8, &[u8])],
    ) -> Result<usize, NotSet> {
        let s = self
            .slot(id)
            .filter(|s| !s.locked())
            .ok_or(NotSet::Unwritable)?;
        let place = at.filter(|&i| i < s.records.len());
        let mut record = match place {
            Some(i) => s.records[i].clone(),
            None => Record::new(k),
        };
        for (n, bytes) in fields {
            record.set(*n, bytes);
        }
        let emptied: Vec<u8> = fields
            .iter()
            .filter(|(_, b)| b.is_empty())
            .map(|(n, _)| *n)
            .collect();
        keep_required(&mut record, &emptied);
        let ov = match place {
            Some(i) => Overlay {
                remove: &[],
                add: &[],
                edit: &[(i, &record)],
            },
            None => Overlay {
                remove: &[],
                add: &[&record],
                edit: &[],
            },
        };
        match s.used_with(c, &self.names, &ov) {
            Ok(n) if n <= s.limit() => {}
            Ok(_) | Err(fv::file::Error::Full { .. }) => return Err(NotSet::Full),
            Err(_) => return Err(NotSet::Unwritable),
        }
        let s = self.slot_mut(id).ok_or(NotSet::Unwritable)?;
        let i = match place {
            Some(i) => {
                s.records[i] = record;
                s.touch(i);
                i
            }
            None => {
                s.records.push(record);
                s.origin.push(Origin::New);
                s.dirty.push(true);
                s.changed();
                s.records.len() - 1
            }
        };
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
        let mut r = self.slot(id)?.records.get(i)?.clone();
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
        let r = self.slot(id)?.records.get(i)?;
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
    /// `id`, and a read of each record changed (for the seal's UUIDs), the
    /// first time after a change, a lookup after. Works out what
    /// `word_count`, `words_id` and `uuid_bytes` answer.
    pub fn warm(&self, c: &Compute, id: VaultId) {
        if let Some(s) = self.slot(id) {
            s.facts(c);
            let _ = self.uuid_bytes(id);
        }
    }

    /// Costs what `warm` costs. How many words key
    /// record `i` has; `None` for another record, or a key record that
    /// holds a seed rather than words.
    pub fn word_count(&self, c: &Compute, id: VaultId, i: usize) -> Option<usize> {
        self.slot(id)?.facts(c).get(i)?.as_ref().map(|f| f.words)
    }

    /// Costs what `word_count` costs. The word list key record `i`'s
    /// words are in; `None` as for `word_count`. Public, kept with the
    /// count.
    pub fn language(
        &self,
        c: &Compute,
        id: VaultId,
        i: usize,
    ) -> Option<faraday_bip::bip39::Language> {
        self.slot(id)?.facts(c).get(i)?.as_ref().map(|f| f.language)
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

    /// Costs what `word_count` costs for a record in an open vault, a
    /// parse of the words for any other. The word list key record `r`'s
    /// words are in.
    pub fn record_language(
        &self,
        c: &Compute,
        r: RecordView<'_>,
    ) -> Option<faraday_bip::bip39::Language> {
        match r.home() {
            Some((id, i)) if self.slot(id).is_some() => self.language(c, id, i),
            _ => record_words(c, r.raw()).map(|m| m.language()),
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
            .is_some_and(|p| keys.passphrase_matches(k, p))
    }

    /// Whether passphrase record `r` (type 14) holds the passphrase of
    /// session key `k`; a record with none holds that of a key with none.
    pub fn passphrase_record_is(&self, r: RecordView<'_>, keys: &Keys, k: usize) -> bool {
        match r.raw().text(field::PASSPHRASE) {
            Some(t) => keys.passphrase_matches(k, t),
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
            .flat_map(|s| s.records.iter().filter(|r| r.kind == kind::KEY))
            .filter_map(|r| r.field(field::KEY))
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
        let r = self.slot(at.0)?.records.get(at.1)?;
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
        let loaded = keys.add_mnemonic(c, &m, passphrase, &label, None);
        if loaded.is_ok() {
            keys.set_child_of(keys.len() - 1, child_of_of(r));
        }
        Some(KeyLoad::Done(loaded))
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
        let Some(r) = self.slot(id).and_then(|s| s.records.get(i)) else {
            return Revealed::lent("");
        };
        if !is_secret(r.kind, number) {
            return Revealed::lent("");
        }
        let Some(part) = why.part() else {
            return Revealed::lent("");
        };
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
        let s = self.slot(id)?;
        let r = s.records.get(i)?;
        if r.kind != kind::ENTRY {
            return None;
        }
        let account = |t: &str| {
            fv::entries::otpauth_names(t)
                .map(|(_, a)| a)
                .filter(|a| !a.is_empty())
        };
        // An entry as read has its TOTP worked out over every field it
        // may be in; one changed here is read from its `otp` field, and
        // from the same fields as when read where it has none (Faraday
        // writes only `otp` and never rewrites a field it did not make).
        // An `otp` field taken out here leaves no code: the entry is no
        // longer read from its other fields.
        let had_otp = |item: usize| {
            s.opened.read().items.get(item).is_some_and(
                |it| matches!(&it.body, Body::Record(o) if o.field(field::TOTP).is_some()),
            )
        };
        let own = s.dirty[i]
            && (r.field(field::TOTP).is_some()
                || matches!(s.origin.get(i), Some(Origin::Entry { item, .. }) if had_otp(*item)));
        if let (false, Some(Origin::Entry { item, .. })) = (own, s.origin.get(i)) {
            let read = s.opened.read().items.get(*item)?.totp.as_ref()?;
            return Some(Totp {
                account: r.text(field::TOTP).and_then(account),
                code: match read {
                    Ok(t) => Ok(crate::totp::shown_of(t, now)),
                    Err(e) => Err(*e),
                },
            });
        }
        let t = r.text(field::TOTP)?;
        Some(Totp {
            account: account(t),
            code: crate::totp::shown(t, now),
        })
    }

    /// A vault holding one item to write on another thread, for a KeePass
    /// app (`docs/VAULT.md` §8): record `at` as it is kept, a person's
    /// entry in the root group or an item in its kind's group, in a new
    /// vault `id` named `label`, under `passphrase`, moved in, at `cost`,
    /// with `random` for the seal and `uuids` for the entry and its group.
    /// A line of the notes that is a BIP-85 path is left out, so a password
    /// BIP-85 made goes without its index (F172), and the title is `title`.
    /// `None` where the record is no entry or item.
    #[expect(
        clippy::too_many_arguments,
        reason = "each is a value the new file states; the caller draws them all"
    )]
    pub fn kdbx_job(
        &self,
        at: (VaultId, usize),
        title: &str,
        (id, label, now): ([u8; 16], &str, Option<u64>),
        passphrase: faraday_intake::secret_buf::SecretText,
        cost: fv::Cost,
        random: Zeroizing<[u8; SEAL_RANDOM_LEN]>,
        uuids: [u8; 32],
    ) -> Option<crate::job::KdbxJob> {
        let (vid, i) = at;
        let r = self.slot(vid)?.records.get(i)?;
        let mut record = r.clone();
        if record.kind == kind::ENTRY {
            record.set(field::TITLE, title.as_bytes());
            if let Some(n) = r.text(field::NOTES) {
                let kept: String = n
                    .split_inclusive('\n')
                    .filter(|l| !l.trim_start().starts_with(BIP85_PATH))
                    .collect();
                record.set(field::NOTES, kept.as_bytes());
            }
        }
        body_of(&record)?;
        let mut u = [0u8; 16];
        let mut g = [0u8; 16];
        u.copy_from_slice(&uuids[..16]);
        g.copy_from_slice(&uuids[16..]);
        Some(crate::job::KdbxJob::new(
            record,
            self.names.clone(),
            (id, label.into(), now),
            (u, g),
            passphrase,
            cost,
            random,
        ))
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
        let mut key = gpg_key(self.slot(id)?.records.get(i)?)?;
        if let Some(e) = expiry {
            key.expiry = e;
        }
        Some(crate::gpg::certificate(&key, at))
    }

    /// Costs an Ed25519 signature and what [`OpenVaults::fits`] costs.
    /// Makes the revocation certificate GPG record `i` of open vault `id`
    /// keeps, signed at `at` with no reason, and keeps it in the record,
    /// as GnuPG keeps one at key generation. `Ok(false)` for a record that
    /// is no GPG key or keeps one already; `Err` where the vault would not
    /// fit its size with it, and nothing is kept ([`Full`]).
    pub fn gpg_keep_revocation(
        &mut self,
        c: &Compute,
        id: VaultId,
        i: usize,
        at: u32,
    ) -> Result<bool, Full> {
        let Some(r) = self.slot(id).and_then(|s| s.records.get(i)) else {
            return Ok(false);
        };
        if r.field(gpg_field::REVOCATION).is_some() {
            return Ok(false);
        }
        let Some(key) = gpg_key(r) else {
            return Ok(false);
        };
        let rev = Zeroizing::new(crate::gpg::revocation(
            &key,
            at,
            faraday_pgp::Reason::Unspecified,
            "",
        ));
        self.edit_record_in(c, id, i, |r| {
            r.set(gpg_field::REVOCATION, &rev);
            Ok(())
        })?;
        Ok(true)
    }

    /// Costs what `gpg_key` costs. The revocation certificate GPG record
    /// `i` of open vault `id` keeps, as the file GnuPG keeps it in, for `why` (`Reveal::OutboxFile` of
    /// `Part::Revocation`). `None` for a record that keeps none, or a
    /// reason that names another part.
    pub fn gpg_revocation_file(
        &self,
        id: VaultId,
        i: usize,
        why: Reveal,
    ) -> Option<Held<Zeroizing<String>>> {
        if why.part() != Some(Part::Revocation) {
            return None;
        }
        let r = self.slot(id)?.records.get(i)?;
        let rev = r.field(gpg_field::REVOCATION)?;
        let key = gpg_key(r)?;
        Some(Held::of(Zeroizing::new(crate::gpg::revocation_file(
            &key, rev,
        ))))
    }

    /// Costs an Ed25519 signature. A new revocation certificate for GPG
    /// record `i` of open vault `id`, signed at `at` for `reason` and
    /// `text` (`gpg --generate-revocation`), as the file GnuPG writes, for
    /// `why` (`Reveal::OutboxFile` of `Part::Revocation`). The kept one
    /// is not replaced. `None` for a record that is no GPG key, or a
    /// reason that names another part.
    pub fn gpg_new_revocation_file(
        &self,
        id: VaultId,
        i: usize,
        at: u32,
        reason: faraday_pgp::Reason,
        text: &str,
        why: Reveal,
    ) -> Option<Held<Zeroizing<String>>> {
        if why.part() != Some(Part::Revocation) {
            return None;
        }
        let key = gpg_key(self.slot(id)?.records.get(i)?)?;
        let rev = Zeroizing::new(crate::gpg::revocation(&key, at, reason, text));
        Some(Held::of(Zeroizing::new(crate::gpg::revocation_file(
            &key, &rev,
        ))))
    }

    /// Costs an Ed25519 signature. The signature over `data` by GPG record
    /// `i` of open vault `id`'s signing subkey, at `at`, armoured.
    pub fn gpg_sign_armored(&self, id: VaultId, i: usize, data: &[u8], at: u32) -> Option<String> {
        let key = gpg_key(self.slot(id)?.records.get(i)?)?;
        Some(crate::gpg::sign_armored(&key, data, at))
    }

    /// Costs what `gpg_key` costs and a paperkey encoding. The paperkey
    /// text of GPG record `r`: secret, made only to be shown,
    /// wiped when dropped (`Reveal::GpgPaperkey`).
    pub fn gpg_paperkey(&self, r: RecordView<'_>, _why: Reveal) -> Option<Held<Zeroizing<String>>> {
        let key = gpg_key(r.raw())?;
        Some(Held::of(crate::gpg::paperkey(&key)))
    }

    // ---- Secure Boot keys ----

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
        let keys = sb_keys(self.slot(id)?.records.get(i)?)?;
        Some(crate::secure_boot::enrolment(
            blind_seed, &keys, policy, now,
        ))
    }

    /// Costs a SHA-256 of `pe` and an RSA signature. The boot program `pe`
    /// signed by the db key of Secure Boot record `i` of open vault `id`,
    /// the signing blinded from `blind_seed`, only when it is the file the
    /// checked `release` names (F134). `None` for a record that is no
    /// readable set of owner keys.
    pub fn sb_sign_image(
        &self,
        id: VaultId,
        i: usize,
        blind_seed: &[u8; 32],
        release: &faraday_sb::release::Release,
        pe: &[u8],
    ) -> Option<Result<Vec<u8>, faraday_sb::Error>> {
        let keys = sb_keys(self.slot(id)?.records.get(i)?)?;
        Some(crate::secure_boot::sign_image(
            blind_seed, pe, &keys, release,
        ))
    }

    // ---- loading keys into the session ----

    /// Costs a parse of each key record's words in `set`, and what
    /// `add_words_with` and `add_seed` cost for each. Loads the key
    /// records of open vault `id` whose places `set` names into `keys`:
    /// the words (with the passphrase stored beside them, or the one an
    /// open vault keeps alone for them that `chosen` names for the
    /// record's place, the first by default) or the master seed. Returns the
    /// fingerprints of the keys that loaded; a key the store refuses is
    /// left out.
    pub fn load_from_vault(
        &self,
        c: &Compute,
        keys: &mut Keys,
        id: VaultId,
        set: &BTreeSet<usize>,
        chosen: &std::collections::BTreeMap<usize, usize>,
    ) -> Vec<Fingerprint> {
        let Some(slot) = self.slot(id) else {
            return Vec::new();
        };
        type Set = Option<fv::codex32::SetFacts>;
        type Child = Option<fv::child::ChildOf>;
        let mut words: Vec<(Zeroizing<String>, String, Set, Child)> = Vec::new();
        let mut seeds: Vec<(Zeroizing<Vec<u8>>, String, Set)> = Vec::new();
        let mut passphrases: Vec<Zeroizing<String>> = Vec::new();
        for (i, r) in slot
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| r.kind == kind::KEY)
        {
            if !set.contains(&i) {
                continue;
            }
            let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
            let set = codex32_set_of(r);
            if let Some(m) = record_words(c, r) {
                // Stored with the key, or kept alone in an open vault.
                let pass = match r.text(field::KEY_PASSPHRASE) {
                    Some(p) => Zeroizing::new(p.to_string()),
                    None => self
                        .passphrase_kept_for(c, keys, &m, chosen.get(&i).copied().unwrap_or(0))
                        .unwrap_or_default(),
                };
                words.push((phrase_of(&m), label, set, child_of_of(r)));
                passphrases.push(pass);
            } else if let Some(seed) = seed_of(r) {
                seeds.push((seed, label, set));
            }
        }
        let mut loaded = Vec::new();
        for ((w, label, set, child), p) in words.iter().zip(&passphrases) {
            if let Ok(fp) = keys.add_words_with(c, w, p, label, None) {
                keys.set_codex32(keys.len() - 1, set.clone());
                keys.set_child_of(keys.len() - 1, *child);
                loaded.push(fp);
            }
        }
        for (seed, label, set) in &seeds {
            if let Ok(fp) = keys.add_seed(c, seed, label) {
                keys.set_codex32(keys.len() - 1, set.clone());
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
        let r = self.slot(id)?.records.get(at)?;
        if r.kind != kind::KEY {
            return None;
        }
        let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
        let loaded = match record_words(c, r) {
            Some(m) => {
                // Stored with the key, or kept alone in an open vault
                // (`docs/BACKUP-FLOW.md` §17.12 item 4).
                let p = match r.text(field::KEY_PASSPHRASE) {
                    Some(p) => Zeroizing::new(p.to_string()),
                    None => self.passphrase_kept_for(c, keys, &m, 0).unwrap_or_default(),
                };
                Some(keys.add_mnemonic(c, &m, &p, &label, None))
            }
            None => seed_of(r).map(|seed| keys.add_seed(c, &seed, &label)),
        };
        if matches!(loaded, Some(Ok(_))) {
            keys.set_codex32(keys.len() - 1, codex32_set_of(r));
            keys.set_child_of(keys.len() - 1, child_of_of(r));
        }
        loaded
    }

    /// Costs a parse of each codex32 share record's string and, for each
    /// set enough of them make, an interpolation, an HMAC-SHA512 and what
    /// `add_seed` costs. The strings the share records of every open vault
    /// keep (`docs/VAULT.md` §7 type 15), gathered towards their keys
    /// (`docs/CODEX32.md` §9.5, Getting the seed back), as the codex32 form
    /// gathers typed ones: one set, distinct indices. A key the session
    /// holds keeps the strings of its set ([`Keys::keep_codex32`]). A key
    /// it does not hold loads once `k` strings of one set are in across
    /// the open vaults and the seed they make has the fingerprint the
    /// records name; its set's facts are the strings found. Returns the
    /// fingerprints of the keys loaded.
    pub fn gather_codex32(&self, c: &Compute, keys: &mut Keys) -> Vec<Fingerprint> {
        use faraday_bip::codex32::Codex32;
        use fv::codex32::{SetFacts, State};
        // Each set, by the fingerprint the records name and its
        // identifier: its strings, one per index, and the other strings
        // of an index it has more than one of.
        type Strings = Vec<(Codex32, Zeroizing<String>)>;
        type Set = ([u8; 4], Strings, Strings);
        let mut sets: Vec<Set> = Vec::new();
        for slot in &self.slots {
            for r in slot
                .records
                .iter()
                .filter(|r| r.kind == kind::CODEX32_SHARE)
            {
                let (Some(text), Some(fp)) = (
                    r.text(field::CODEX32_STRING),
                    r.field(field::CODEX32_KEY)
                        .and_then(|b| <[u8; 4]>::try_from(b).ok()),
                ) else {
                    continue;
                };
                let Ok(share) = Codex32::parse(text) else {
                    continue;
                };
                let at = match sets.iter().position(|(f, list, _)| {
                    *f == fp
                        && list
                            .first()
                            .is_some_and(|(x, _)| x.identifier() == share.identifier())
                }) {
                    Some(at) => at,
                    None => {
                        sets.push((fp, Vec::new(), Vec::new()));
                        sets.len() - 1
                    }
                };
                let (_, list, others) = &mut sets[at];
                let text = Zeroizing::new(text.to_string());
                let same = |(x, t): &(Codex32, Zeroizing<String>)| {
                    x.share_index() == share.share_index() && t.eq_ignore_ascii_case(&text)
                };
                if list.iter().any(&same) || others.iter().any(&same) {
                    continue;
                }
                if list
                    .iter()
                    .any(|(x, _)| x.share_index() == share.share_index())
                {
                    others.push((share, text));
                } else {
                    list.push((share, text));
                }
            }
        }
        let mut loaded = Vec::new();
        for (fp, list, others) in sets {
            if let Some(k) = keys.position_of(fp) {
                let texts: Vec<&str> = list
                    .iter()
                    .chain(&others)
                    .map(|(_, t)| t.as_str())
                    .collect();
                keys.keep_codex32(c, k, &texts);
                continue;
            }
            let Some((first, _)) = list.first() else {
                continue;
            };
            let need = usize::from(first.threshold());
            if need < 2 || list.len() < need {
                continue;
            }
            let Some((seed, texts)) = codex32_pick(c, fp, need, &list, &others) else {
                continue;
            };
            let Ok(got) = keys.add_seed(c, &seed, "") else {
                continue;
            };
            let at = keys.len() - 1;
            let facts = SetFacts {
                threshold: first.threshold(),
                identifier: first.identifier(),
                length: u8::try_from(texts.first().map_or(0, |t| t.len())).unwrap_or(0),
                unknown: 0,
                strings: list
                    .iter()
                    .map(|(x, _)| (x.share_index(), State::Placed))
                    .collect(),
            };
            keys.set_codex32(at, Some(facts));
            keys.keep_codex32(c, at, &texts);
            loaded.push(got);
        }
        loaded
    }

    /// Costs a parse of each codex32 share record's string. The share strings
    /// every open vault keeps of the set `typed` is of (its identifier,
    /// threshold and length), at indices none of `typed` has, as their
    /// text (`docs/CODEX32.md` §9.5), grouped by the key each record
    /// names: one group per fingerprint, every distinct string in it
    /// (an index may have two, and the right one is found by the seed
    /// they make). Strings of two seeds may share an identifier, a
    /// threshold and a length; only one group's strings are ever
    /// combined. A record that names no key is in none. None where
    /// `typed` is empty or the secret alone.
    fn codex32_more_of(
        &self,
        typed: &[faraday_bip::codex32::Codex32],
    ) -> Vec<([u8; 4], Vec<Zeroizing<String>>)> {
        use faraday_bip::codex32::Codex32;
        let Some(first) = typed.first().filter(|f| f.threshold() >= 2) else {
            return Vec::new();
        };
        // Each key's strings, by fingerprint.
        type Group = ([u8; 4], Vec<Zeroizing<String>>);
        let mut out: Vec<Group> = Vec::new();
        for r in self.slots.iter().flat_map(|s| s.records.iter()) {
            if r.kind != kind::CODEX32_SHARE {
                continue;
            }
            let (Some(text), Some(fp)) = (
                r.text(field::CODEX32_STRING),
                r.field(field::CODEX32_KEY)
                    .and_then(|b| <[u8; 4]>::try_from(b).ok()),
            ) else {
                continue;
            };
            let Ok(s) = Codex32::parse(text) else {
                continue;
            };
            let i = s.share_index();
            let same = s.identifier() == first.identifier()
                && s.threshold() == first.threshold()
                && s.payload().len() == first.payload().len();
            if !same || s.is_secret() || typed.iter().any(|t| t.share_index() == i) {
                continue;
            }
            let at = match out.iter().position(|(f, _)| *f == fp) {
                Some(at) => at,
                None => {
                    out.push((fp, Vec::new()));
                    out.len() - 1
                }
            };
            let group = &mut out[at].1;
            if !group.iter().any(|t| t.eq_ignore_ascii_case(text)) {
                group.push(Zeroizing::new(text.to_ascii_lowercase()));
            }
        }
        out
    }

    /// Costs what `Keys::codex32_of_seed` costs, and a parse of each
    /// codex32 share record's string. Whether `string`, a share whose
    /// header is that of key `i`'s codex32 set, is of key `i`'s seed
    /// (`docs/CODEX32.md` §9.5), with the strings of its set this session
    /// holds for the key and the open vaults keep: `None` where fewer than
    /// `k` are here to rebuild it.
    pub fn codex32_of_seed(
        &self,
        c: &Compute,
        keys: &Keys,
        i: usize,
        string: &faraday_bip::codex32::Codex32,
    ) -> Option<bool> {
        let more = self.codex32_of_key(keys, i, string)?;
        keys.codex32_of_seed(c, i, string, &more)
    }

    /// Costs what `Keys::keep_codex32` costs, and a parse of each codex32
    /// share record's string. Keeps `string`, a share whose header is
    /// that of key `i`'s codex32 set, with key `i` where it is proved of
    /// the key's seed from the strings this session holds for the key
    /// and the open vaults keep (`docs/CODEX32.md` §9.5); a string not
    /// proved is not kept. Whether it was kept.
    pub fn keep_codex32(
        &self,
        c: &Compute,
        keys: &mut Keys,
        i: usize,
        string: &faraday_bip::codex32::Codex32,
    ) -> bool {
        let Some(more) = self.codex32_of_key(keys, i, string) else {
            return false;
        };
        let text = Zeroizing::new(string.encode().as_str().to_string());
        keys.keep_codex32_among(c, i, &[text.as_str()], &more) > 0
    }

    /// Costs a parse of each codex32 share record's string. The open vaults'
    /// strings of the set `string` is of that belong to key `i`'s own
    /// seed: strings of the same set header kept for another seed never
    /// join it. `None` for no such key.
    fn codex32_of_key(
        &self,
        keys: &Keys,
        i: usize,
        string: &faraday_bip::codex32::Codex32,
    ) -> Option<Vec<Zeroizing<String>>> {
        let fp = keys.info(i)?.fingerprint.0;
        Some(
            self.codex32_more_of(std::slice::from_ref(string))
                .into_iter()
                .find(|(f, _)| *f == fp)
                .map(|(_, g)| g)
                .unwrap_or_default(),
        )
    }

    /// Costs a parse of each codex32 share record's string. How many strings
    /// of the set `typed` is of the open vaults add to it: at indices
    /// none of `typed` has, one per index, of the one key whose records
    /// hold the most (`docs/CODEX32.md` §9.5). Public: a count.
    pub fn codex32_count(&self, typed: &[faraday_bip::codex32::Codex32]) -> usize {
        self.codex32_more_of(typed)
            .iter()
            .map(|(_, g)| {
                let mut at: Vec<u8> = g
                    .iter()
                    .filter_map(|t| t.as_bytes().get(8).copied())
                    .collect();
                at.sort_unstable();
                at.dedup();
                at.len()
            })
            .max()
            .unwrap_or(0)
    }

    /// Costs a parse of each codex32 share record's string and, for each
    /// key whose records make up `k` with those typed, and each of at
    /// most 16 choices among its strings, an interpolation, an HMAC-SHA512
    /// and a fingerprint, and what `add_seed` costs.
    /// Strings typed into the codex32 form gathered with the open vaults'
    /// strings of their set (`docs/CODEX32.md` §9.5), one key's records
    /// at a time: once `typed` and those make `k`, and the seed they make
    /// has the fingerprint the records name, it loads as a key labelled
    /// `label`, its set's facts the indices of both and its strings kept
    /// ([`Keys::keep_codex32`]). `None` while fewer than `k` are in; a
    /// refusal where no key's records make their own key with those
    /// typed, or the key does not load.
    pub fn codex32_with_typed(
        &self,
        c: &Compute,
        keys: &mut Keys,
        typed: &[faraday_bip::codex32::Codex32],
        label: &str,
    ) -> Option<Result<Fingerprint, Refusal>> {
        use fv::codex32::{SetFacts, State};
        let first = typed.first()?;
        let need = usize::from(first.threshold());
        if need < 2 || typed.len() >= need {
            return None;
        }
        // Each key's records with those typed, until one makes the key
        // its records name; never strings of two keys together.
        let typed_texts: Vec<Zeroizing<String>> = typed
            .iter()
            .map(|s| Zeroizing::new(s.encode().as_str().to_string()))
            .collect();
        let fixed: Vec<&str> = typed_texts.iter().map(|t| t.as_str()).collect();
        let mut tried = false;
        let mut found = None;
        for (fp, more) in self.codex32_more_of(typed) {
            let pool: Vec<&str> = more.iter().map(|t| t.as_str()).collect();
            let Some(made) = codex32_find(c, fp, need, &fixed, &pool) else {
                continue;
            };
            tried = true;
            if let Some((seed, picked)) = made {
                // What made the key, then the key's other strings at
                // indices none of those has.
                let mut texts = typed_texts.clone();
                texts.extend(picked.into_iter().skip(typed.len()));
                for t in more.iter() {
                    let at = t.as_bytes().get(8);
                    if !texts.iter().any(|x| x.as_bytes().get(8) == at) {
                        texts.push(t.clone());
                    }
                }
                found = Some((texts, seed));
                break;
            }
        }
        let Some((texts, seed)) = found else {
            return tried.then(|| {
                Err(Refusal::Words(
                    faraday_strings::EN.c32_vault_other_seed.to_string(),
                ))
            });
        };
        let fp = match keys.add_seed(c, &seed, label) {
            Ok(fp) => fp,
            Err(e) => return Some(Err(e)),
        };
        let at = keys.len() - 1;
        // Those typed are on paper; those in a vault are placed there.
        let strings = texts
            .iter()
            .enumerate()
            .filter_map(|(n, t)| Some((*t.as_bytes().get(8)?, n)))
            .map(|(i, n)| {
                (
                    i,
                    if n < typed.len() {
                        State::Made
                    } else {
                        State::Placed
                    },
                )
            })
            .collect();
        keys.set_codex32(
            at,
            Some(SetFacts {
                threshold: first.threshold(),
                identifier: first.identifier(),
                length: u8::try_from(texts[0].len()).unwrap_or(0),
                unknown: 0,
                strings,
            }),
        );
        let refs: Vec<&str> = texts.iter().map(|t| t.as_str()).collect();
        keys.keep_codex32(c, at, &refs);
        Some(Ok(fp))
    }

    /// Whether open vault `id` keeps a codex32 share record of the key
    /// with fingerprint `fp` at share index `index` (a bech32 character,
    /// lower case) of the set `set` (its threshold and identifier): a
    /// string of another set of the same key is not it. A lookup.
    pub fn holds_codex32(&self, id: VaultId, fp: [u8; 4], set: (u8, [u8; 4]), index: u8) -> bool {
        self.slot(id).is_some_and(|s| {
            s.records
                .iter()
                .any(|r| codex32_share_is(RecordView::of(r), fp, set, Some(index)))
        })
    }

    /// Costs a fingerprint for each passphrase record. The passphrase
    /// records (type 14) the open vaults keep alone for the key the words
    /// `m` make with them, found by the fingerprint each names: the vault
    /// and the place, in the order the vaults are held.
    pub fn passphrases_kept_for(
        &self,
        c: &Compute,
        keys: &Keys,
        m: &Mnemonic,
    ) -> Vec<(VaultId, usize)> {
        let mut found = Vec::new();
        for slot in &self.slots {
            for (i, r) in slot
                .records
                .iter()
                .enumerate()
                .filter(|(_, r)| r.kind == kind::PASSPHRASE)
            {
                let (Some(fp), Some(p)) =
                    (r.field(field::PASSPHRASE_KEY), r.text(field::PASSPHRASE))
                else {
                    continue;
                };
                let Some(got) = fingerprint(c, Material::Words(m), p, keys.network()) else {
                    continue;
                };
                if got.0[..] == *fp {
                    found.push((slot.id, i));
                }
            }
        }
        found
    }

    /// A lookup. What open vault `id` already holds that `r` would be
    /// related to (a passphrase for the same key, a share of the same
    /// index and key): `None` where `r` is no such record, nothing like it
    /// is held, or the same record is.
    pub fn relates(&self, id: VaultId, r: &impl Fit) -> Option<RelatedKind> {
        let s = self.slot(id)?;
        let r = r.record_of(Inside(()));
        let (kind, key) = pair_of(r)?;
        let mut held = false;
        for o in &s.records {
            if pair_of(o).is_some_and(|(k, v)| k == kind && v == key) {
                // The same secret again is no second one.
                let secret = if kind == RelatedKind::Passphrases {
                    field::PASSPHRASE
                } else {
                    field::CODEX32_STRING
                };
                if o.field(secret) == r.field(secret) {
                    return None;
                }
                held = true;
            }
        }
        held.then_some(kind)
    }

    /// Costs what [`Records::passphrases_kept_for`] costs, and a parse of
    /// the record's words. The passphrase records the open vaults keep
    /// alone for key record `i` of open vault `id`: none where the record
    /// holds its own passphrase or is no words.
    pub fn passphrases_of_key(
        &self,
        c: &Compute,
        keys: &Keys,
        id: VaultId,
        i: usize,
    ) -> Vec<(VaultId, usize)> {
        let Some(r) = self.slot(id).and_then(|s| s.records.get(i)) else {
            return Vec::new();
        };
        if r.kind != kind::KEY || r.text(field::KEY_PASSPHRASE).is_some() {
            return Vec::new();
        }
        record_words(c, r).map_or_else(Vec::new, |m| self.passphrases_kept_for(c, keys, &m))
    }

    /// Costs what [`Records::passphrases_kept_for`] costs. The passphrase
    /// an open vault keeps alone for the key the words `m` make: the
    /// `chosen`th of those found (the first when it names none).
    pub fn passphrase_kept_for(
        &self,
        c: &Compute,
        keys: &Keys,
        m: &Mnemonic,
        chosen: usize,
    ) -> Option<Zeroizing<String>> {
        let found = self.passphrases_kept_for(c, keys, m);
        let (id, i) = *found.get(chosen).or_else(|| found.first())?;
        let r = self.slot(id)?.records.get(i)?;
        Some(Zeroizing::new(r.text(field::PASSPHRASE)?.to_string()))
    }

    // ---- for tests ----

    /// Open vault `id`'s records as they are held, for tests (feature
    /// `master-for-tests`, which no shipped binary has).
    #[cfg(feature = "master-for-tests")]
    pub fn contents_for_tests(&self, id: VaultId) -> &[Record] {
        match self.slot(id) {
            Some(s) => &s.records,
            None => &[],
        }
    }

    /// Open vault `id`'s records, to change, for tests (feature
    /// `master-for-tests`). The vault's version moves, as for any edit.
    #[cfg(feature = "master-for-tests")]
    pub fn contents_for_tests_mut(&mut self, id: VaultId) -> &mut Vec<Record> {
        let s = self
            .slots
            .iter_mut()
            .find(|s| s.id == id)
            .expect("an open vault");
        s.changed();
        s.dirty.iter_mut().for_each(|d| *d = true);
        &mut s.records
    }
}

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
/// Makes a vault: the root group `id`, named `label`, sealed at `size`
/// under `passphrase`, stamped as the first seal at `now`. The file is
/// encrypted, so public.
#[expect(
    clippy::too_many_arguments,
    reason = "each is a value the new file states; the caller draws them all"
)]
pub fn create(
    c: &Compute,
    id: [u8; 16],
    label: &str,
    cost: fv::Cost,
    size: usize,
    passphrase: &str,
    random: &[u8; SEAL_RANDOM_LEN],
    now: Option<u64>,
) -> Result<Vec<u8>, fv::file::Error> {
    fv::file::create(c, id, label, cost, size, passphrase, random, now)
}

/// Costs an Argon2id run at the file's stated cost, thrown away: the time
/// a vault's unlock takes on this computer, measured.
pub fn measure(c: &Compute, facts: &fv::file::Facts) {
    let _ = fv::file::measure(c, facts);
}
