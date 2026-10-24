//! A slot's plaintext (`docs/VAULT.md` §7): the format byte, typed
//! records, the end record, and zeros to the slot's length.
//!
//! A record is a type byte, a `u32` body length and the body; a body is
//! fields, each a number, a `u16` length and the bytes. What each type
//! may hold is [`rules`]: a field it does not list, a required field
//! missing, a field repeated where it may not be, text that is not
//! UTF-8, or a value of the wrong length refuses the slot. So does an
//! unknown type: a build reads only what it knows. So do two slot
//! labels, two passphrase records (type 14) for one key, and two signed
//! amounts records (type 10) for one transaction.
//!
//! Every field's bytes are wiped when the record is dropped.

use alloc::string::String;
use alloc::vec::Vec;

use faraday_bip::bip39::Mnemonic;
use zeroize::Zeroizing;

use crate::Error;

/// The slot format this build writes and reads.
pub const FORMAT: u8 = 1;

/// The record types (§7).
pub mod kind {
    /// The end record.
    pub const END: u8 = 0;
    /// The slot's label.
    pub const SLOT_LABEL: u8 = 1;
    /// A Bitcoin key.
    pub const KEY: u8 = 2;
    /// A wallet.
    pub const WALLET: u8 = 3;
    /// A note.
    pub const NOTE: u8 = 4;
    /// A wallet's recovery sheet.
    pub const SHEET: u8 = 5;
    /// An entry: a login, its password and its TOTP secret.
    pub const ENTRY: u8 = 6;
    /// A GPG key.
    pub const GPG: u8 = 7;
    /// Secure Boot keys.
    pub const SECURE_BOOT: u8 = 8;
    /// A MuSig2 or FROST round kept across a lock.
    pub const ROUND: u8 = 9;
    /// The amounts a transaction's inputs stated when it was signed.
    pub const AMOUNTS: u8 = 10;
    /// A wallet's backup plan.
    pub const PLAN: u8 = 11;
    // Type 12 was an APK signing key. It is retired: no build knows it,
    // and a slot holding one is refused like any unknown type
    // (`docs/VAULT.md` §7).
    /// A share of a multisig wallet's description, some keys left off
    /// (`docs/BACKUP-FLOW.md` §15.6).
    pub const WALLET_SHARE: u8 = 13;
    /// A key's BIP-39 passphrase kept without the key's words, in a vault
    /// that does not hold them (`docs/BACKUP-FLOW.md` §17.12 item 4).
    pub const PASSPHRASE: u8 = 14;
}

/// The fields of each record type, by number.
pub mod field {
    /// Slot label: the label.
    pub const LABEL: u8 = 1;

    /// Bitcoin key: the words or the seed payload (§7).
    pub const KEY: u8 = 1;
    /// Bitcoin key: what the person calls it.
    pub const KEY_LABEL: u8 = 2;
    /// Bitcoin key: flags, bit 0 not loaded at unlock
    /// ([`super::NO_LOAD_AT_UNLOCK`]), bit 1 its backup not done.
    pub const KEY_FLAGS: u8 = 3;
    /// Bitcoin key: its BIP-39 passphrase, only when the person chose to
    /// store it.
    pub const KEY_PASSPHRASE: u8 = 4;

    /// Wallet: BIP-388 text or a descriptor.
    pub const WALLET: u8 = 1;
    /// Wallet: its name.
    pub const WALLET_NAME: u8 = 2;
    /// Wallet: flags, bit 0 not loaded at unlock
    /// ([`super::NO_LOAD_AT_UNLOCK`]). Absent, it loads.
    pub const WALLET_FLAGS: u8 = 3;

    /// Note: the text.
    pub const NOTE: u8 = 1;
    /// Recovery sheet: the sheet payload (§7).
    pub const SHEET: u8 = 1;

    /// Entry: title.
    pub const TITLE: u8 = 1;
    /// Entry: username.
    pub const USERNAME: u8 = 2;
    /// Entry: password.
    pub const PASSWORD: u8 = 3;
    /// Entry: URL.
    pub const URL: u8 = 4;
    /// Entry: notes.
    pub const NOTES: u8 = 5;
    /// Entry: TOTP secret, an `otpauth://` URI.
    pub const TOTP: u8 = 6;

    /// Signing round: the scheme, 1 MuSig2 or 2 FROST.
    pub const ROUND_SCHEME: u8 = 1;
    /// Signing round: SHA-256 of the unsigned transaction it is bound to.
    pub const ROUND_TX: u8 = 2;
    /// Signing round: the fingerprint of the key that signed with it.
    pub const ROUND_KEY: u8 = 3;
    /// Signing round: the secret state, as `faraday-psbt` serialises it (for
    /// FROST, the carry file).
    pub const ROUND_STATE: u8 = 4;
    /// Signing round: set when the round has been used.
    pub const ROUND_USED: u8 = 5;

    /// Signed amounts: SHA-256 of the unsigned transaction.
    pub const AMOUNTS_TX: u8 = 1;
    /// Signed amounts: the amount one input stated (repeatable).
    pub const AMOUNTS_INPUT: u8 = 2;

    /// Backup plan: the wallet, as its wallet record holds it.
    pub const PLAN_WALLET: u8 = 1;
    /// Backup plan: the answers, a line each.
    pub const PLAN_ANSWERS: u8 = 2;
    /// Backup plan: a place's name, one per place in order (repeatable).
    pub const PLAN_PLACE: u8 = 3;
    /// Backup plan: what one place holds, a line (repeatable).
    pub const PLAN_HOLDS: u8 = 4;
    /// Backup plan: a note on a thing of its map, a line (repeatable): the
    /// date it was last checked here, a mark (lost, exposed), a key's
    /// holder, a vault's name. Never a secret.
    pub const PLAN_NOTE: u8 = 5;

    /// Wallet share: the share, as its `.txt` file holds it.
    pub const SHARE: u8 = 1;
    /// Wallet share: the wallet's name.
    pub const SHARE_WALLET_NAME: u8 = 2;
    /// Wallet share: which share, from 1.
    pub const SHARE_NUMBER: u8 = 3;
    /// Wallet share: how many shares.
    pub const SHARE_OF: u8 = 4;
    /// Wallet share: the wallet, as a plan's field 1 holds it, by which a
    /// share is told from another wallet's of the same name.
    pub const SHARE_WALLET: u8 = 5;

    /// Passphrase: the fingerprint of the key it is the passphrase of, the
    /// key's words with this passphrase (4 bytes).
    pub const PASSPHRASE_KEY: u8 = 1;
    /// Passphrase: the passphrase.
    pub const PASSPHRASE: u8 = 2;
}

/// Bit 0 of a key's or a wallet's flags: the Load sheet an unlock
/// shows starts it unticked (`docs/DECISIONS.md` F62). With the bit
/// clear, or no flags field, it starts ticked.
pub const NO_LOAD_AT_UNLOCK: u8 = 1;

/// Bit 1 of a key's flags: the key was made in Faraday and its backup is
/// not done; it signs nothing until it is. Never a secret.
pub const BACKUP_PENDING: u8 = 2;

/// Why a slot's contents were refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The first byte is not a slot format this build reads.
    Format,
    /// A record type this build does not know.
    UnknownType(u8),
    /// A record or field runs past what holds it.
    Truncated,
    /// Bytes after the end record are not zero.
    Trailing,
    /// A field a type does not list.
    UnknownField(u8, u8),
    /// A required field is missing.
    Missing(u8, u8),
    /// A field repeated where it may not be.
    Repeated(u8, u8),
    /// A value of the wrong length or not UTF-8.
    Value(u8, u8),
    /// More than one slot label.
    TwoLabels,
    /// Two records of a type that holds one per key or per transaction:
    /// a passphrase (type 14) for the key whose fingerprint `of` is, or
    /// signed amounts (type 10) for the transaction whose SHA-256 starts
    /// with `of`.
    Duplicate {
        /// The record type.
        kind: u8,
        /// The key's fingerprint, or the first four bytes of the
        /// transaction's SHA-256.
        of: [u8; 4],
    },
}

impl Fault {
    /// The fault in a few words.
    pub fn reason(self) -> String {
        use alloc::format;
        match self {
            Fault::Format => "a slot format this build does not read".into(),
            Fault::UnknownType(t) => format!("record type {t} is unknown to this build"),
            Fault::Truncated => "a record runs past the slot".into(),
            Fault::Trailing => "bytes after the last record".into(),
            Fault::UnknownField(t, f) => format!("field {f} is not part of record type {t}"),
            Fault::Missing(t, f) => format!("record type {t} lacks field {f}"),
            Fault::Repeated(t, f) => format!("field {f} of record type {t} appears twice"),
            Fault::Value(t, f) => format!("field {f} of record type {t} has a bad value"),
            Fault::TwoLabels => "two slot labels".into(),
            Fault::Duplicate { kind: ty, of } => {
                let of = u32::from_be_bytes(of);
                if ty == kind::PASSPHRASE {
                    format!("two passphrase records for key {of:08x}")
                } else {
                    format!("two signed amounts records for transaction {of:08x}…")
                }
            }
        }
    }
}

/// The one-per-slot rules across records: at most one slot label, one
/// passphrase record per key (§7, type 14) and one signed amounts record
/// per transaction (type 10). Each record is already checked, so the
/// fields the rules read are there and of their lengths.
fn check_across(records: &[Record]) -> Result<(), Fault> {
    if records
        .iter()
        .filter(|r| r.kind == kind::SLOT_LABEL)
        .count()
        > 1
    {
        return Err(Fault::TwoLabels);
    }
    for (ty, by) in [
        (kind::PASSPHRASE, field::PASSPHRASE_KEY),
        (kind::AMOUNTS, field::AMOUNTS_TX),
    ] {
        let mut seen: Vec<&[u8]> = Vec::new();
        for r in records.iter().filter(|r| r.kind == ty) {
            let Some(id) = r.field(by) else { continue };
            if seen.contains(&id) {
                let mut of = [0u8; 4];
                of.copy_from_slice(&id[..4]);
                return Err(Fault::Duplicate { kind: ty, of });
            }
            seen.push(id);
        }
    }
    Ok(())
}

/// What a field's bytes must be.
#[derive(Debug, Clone, Copy)]
enum Shape {
    /// UTF-8, at most this many bytes.
    Text(usize),
    /// Any bytes.
    Bytes,
    /// Exactly this many bytes.
    Exact(usize),
    /// One of two lengths.
    Either(usize, usize),
}

/// One field a record type allows.
#[derive(Debug, Clone, Copy)]
struct Rule {
    number: u8,
    required: bool,
    repeat: bool,
    shape: Shape,
}

const fn rule(number: u8, required: bool, repeat: bool, shape: Shape) -> Rule {
    Rule {
        number,
        required,
        repeat,
        shape,
    }
}

const TEXT: Shape = Shape::Text(u16::MAX as usize);

const SLOT_LABEL_RULES: &[Rule] = &[rule(1, true, false, Shape::Text(64))];
const KEY_RULES: &[Rule] = &[
    rule(1, true, false, Shape::Either(34, 65)),
    rule(2, false, false, TEXT),
    rule(3, false, false, Shape::Exact(1)),
    rule(4, false, false, TEXT),
];
const WALLET_RULES: &[Rule] = &[
    rule(1, true, false, TEXT),
    rule(2, false, false, TEXT),
    rule(3, false, false, Shape::Exact(1)),
];
const NOTE_RULES: &[Rule] = &[rule(1, true, false, TEXT)];
const SHEET_RULES: &[Rule] = &[rule(1, true, false, Shape::Bytes)];
const ENTRY_RULES: &[Rule] = &[
    rule(1, true, false, TEXT),
    rule(2, false, false, TEXT),
    rule(3, false, false, TEXT),
    rule(4, false, false, TEXT),
    rule(5, false, false, TEXT),
    rule(6, false, false, TEXT),
];
const GPG_RULES: &[Rule] = &[
    rule(1, true, false, Shape::Exact(32)),
    rule(2, true, false, Shape::Exact(4)),
    rule(3, false, false, Shape::Exact(32)),
    rule(4, false, false, Shape::Exact(4)),
    rule(5, false, true, TEXT),
    rule(6, false, false, Shape::Exact(4)),
    rule(7, false, false, Shape::Bytes),
    rule(8, false, false, Shape::Bytes),
    rule(9, false, false, Shape::Exact(4)),
];
const SECURE_BOOT_RULES: &[Rule] = &[
    rule(1, true, false, Shape::Exact(16)),
    rule(2, false, false, Shape::Bytes),
    rule(3, false, false, Shape::Bytes),
    rule(4, false, false, Shape::Bytes),
    rule(5, false, false, Shape::Bytes),
    rule(6, false, false, Shape::Bytes),
    rule(7, false, false, Shape::Bytes),
];
const ROUND_RULES: &[Rule] = &[
    rule(1, true, false, Shape::Exact(1)),
    rule(2, true, false, Shape::Exact(32)),
    rule(3, true, false, Shape::Exact(4)),
    rule(4, true, false, Shape::Bytes),
    rule(5, false, false, Shape::Exact(1)),
];
const AMOUNTS_RULES: &[Rule] = &[
    rule(1, true, false, Shape::Exact(32)),
    rule(2, true, true, Shape::Exact(8)),
];
const PLAN_RULES: &[Rule] = &[
    rule(1, true, false, TEXT),
    rule(2, false, false, TEXT),
    rule(3, false, true, Shape::Text(128)),
    rule(4, false, true, TEXT),
    rule(5, false, true, TEXT),
    // Field 6, an access layer's name, was written only by test builds
    // before the layers were withdrawn (`docs/DECISIONS.md` F38): still
    // allowed so such a record opens, and read by nothing.
    rule(6, false, true, Shape::Text(128)),
];

const WALLET_SHARE_RULES: &[Rule] = &[
    rule(1, true, false, TEXT),
    rule(2, false, false, TEXT),
    rule(3, false, false, Shape::Exact(1)),
    rule(4, false, false, Shape::Exact(1)),
    rule(5, false, false, TEXT),
];

const PASSPHRASE_RULES: &[Rule] = &[
    rule(1, true, false, Shape::Exact(4)),
    rule(2, true, false, TEXT),
];

/// The fields of each record type (§7).
fn rules(kind: u8) -> Option<&'static [Rule]> {
    Some(match kind {
        kind::SLOT_LABEL => SLOT_LABEL_RULES,
        kind::KEY => KEY_RULES,
        kind::WALLET => WALLET_RULES,
        kind::NOTE => NOTE_RULES,
        kind::SHEET => SHEET_RULES,
        kind::ENTRY => ENTRY_RULES,
        kind::GPG => GPG_RULES,
        kind::SECURE_BOOT => SECURE_BOOT_RULES,
        kind::ROUND => ROUND_RULES,
        kind::AMOUNTS => AMOUNTS_RULES,
        kind::PLAN => PLAN_RULES,
        kind::WALLET_SHARE => WALLET_SHARE_RULES,
        kind::PASSPHRASE => PASSPHRASE_RULES,
        _ => return None,
    })
}

/// One field of a record. Its bytes are wiped on drop.
#[derive(Clone)]
pub struct Field {
    /// The field's number.
    pub number: u8,
    /// Its bytes.
    pub bytes: Zeroizing<Vec<u8>>,
}

impl core::fmt::Debug for Field {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Field({}, {} bytes)", self.number, self.bytes.len())
    }
}

/// One record of a slot.
#[derive(Clone, Debug)]
pub struct Record {
    /// Its type (`kind`).
    pub kind: u8,
    /// Its fields, in the order they are written.
    pub fields: Vec<Field>,
}

impl Record {
    /// An empty record of a type.
    pub fn new(kind: u8) -> Record {
        Record {
            kind,
            fields: Vec::new(),
        }
    }

    /// Adds a field and returns the record.
    pub fn with(mut self, number: u8, bytes: &[u8]) -> Record {
        self.push(number, bytes);
        self
    }

    /// Adds a field.
    pub fn push(&mut self, number: u8, bytes: &[u8]) {
        self.fields.push(Field {
            number,
            bytes: Zeroizing::new(bytes.to_vec()),
        });
    }

    /// Sets a field that appears at most once, or removes it when
    /// `bytes` is empty and the field is optional.
    pub fn set(&mut self, number: u8, bytes: &[u8]) {
        self.fields.retain(|f| f.number != number);
        if !bytes.is_empty() {
            self.push(number, bytes);
            self.fields.sort_by_key(|f| f.number);
        }
    }

    /// The first field with this number.
    pub fn field(&self, number: u8) -> Option<&[u8]> {
        self.fields
            .iter()
            .find(|f| f.number == number)
            .map(|f| f.bytes.as_slice())
    }

    /// The first field with this number, as text.
    pub fn text(&self, number: u8) -> Option<&str> {
        self.field(number)
            .and_then(|b| core::str::from_utf8(b).ok())
    }

    /// Checks the record against its type's rules.
    pub fn check(&self) -> Result<(), Fault> {
        let rules = rules(self.kind).ok_or(Fault::UnknownType(self.kind))?;
        for f in &self.fields {
            let r = rules
                .iter()
                .find(|r| r.number == f.number)
                .ok_or(Fault::UnknownField(self.kind, f.number))?;
            let ok = match r.shape {
                Shape::Text(max) => f.bytes.len() <= max && core::str::from_utf8(&f.bytes).is_ok(),
                Shape::Bytes => f.bytes.len() <= u16::MAX as usize,
                Shape::Exact(n) => f.bytes.len() == n,
                Shape::Either(a, b) => f.bytes.len() == a || f.bytes.len() == b,
            };
            if !ok {
                return Err(Fault::Value(self.kind, f.number));
            }
        }
        for r in rules {
            let n = self.fields.iter().filter(|f| f.number == r.number).count();
            if r.required && n == 0 {
                return Err(Fault::Missing(self.kind, r.number));
            }
            if !r.repeat && n > 1 {
                return Err(Fault::Repeated(self.kind, r.number));
            }
        }
        if self.kind == kind::KEY
            && let Some(payload) = self.field(field::KEY)
        {
            let k = if payload.len() == 34 {
                crate::layouts::KIND_WORDS
            } else {
                crate::layouts::KIND_SEED
            };
            if crate::layouts::payload_of(k, payload).is_none() {
                return Err(Fault::Value(self.kind, field::KEY));
            }
        }
        if self.kind == kind::SHEET
            && let Some(payload) = self.field(field::SHEET)
            && crate::layouts::payload_of(crate::layouts::KIND_SHEET, payload).is_none()
        {
            return Err(Fault::Value(self.kind, field::SHEET));
        }
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        1 + 4 + self.fields.iter().map(|f| 3 + f.bytes.len()).sum::<usize>()
    }

    /// The bytes this record takes in a slot: its type, its length and its
    /// fields. Public: it says how big the record is, not what it holds.
    pub fn size(&self) -> usize {
        self.encoded_len()
    }
}

/// A slot's records.
#[derive(Clone, Debug, Default)]
pub struct Contents {
    /// The records, in the order they are written.
    pub records: Vec<Record>,
}

impl Contents {
    /// The plaintext of a slot `slot_len` long holding these records.
    pub fn encode(&self, slot_len: u32) -> Result<Zeroizing<Vec<u8>>, Error> {
        for r in &self.records {
            r.check().map_err(Error::Contents)?;
        }
        check_across(&self.records).map_err(Error::Contents)?;
        let len = slot_len as usize;
        if self.used() > len {
            return Err(Error::Full);
        }
        let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
        out.try_reserve_exact(len)
            .map_err(|_| Error::Memory((len / (1024 * 1024)) as u32 + 1))?;
        out.push(FORMAT);
        for r in &self.records {
            out.push(r.kind);
            let body: usize = r.encoded_len() - 5;
            out.extend_from_slice(&(body as u32).to_le_bytes());
            for f in &r.fields {
                out.push(f.number);
                out.extend_from_slice(&(f.bytes.len() as u16).to_le_bytes());
                out.extend_from_slice(&f.bytes);
            }
        }
        out.push(kind::END);
        out.resize(len, 0);
        Ok(out)
    }

    /// The bytes these records take in a slot, with the format byte and
    /// the end record.
    pub fn used(&self) -> usize {
        2 + self.records.iter().map(Record::encoded_len).sum::<usize>()
    }

    /// Reads a slot's plaintext.
    pub fn decode(plain: &[u8]) -> Result<Contents, Fault> {
        if plain.first() != Some(&FORMAT) {
            return Err(Fault::Format);
        }
        let mut at = 1usize;
        let mut records = Vec::new();
        loop {
            let t = *plain.get(at).ok_or(Fault::Truncated)?;
            at += 1;
            if t == kind::END {
                break;
            }
            if rules(t).is_none() {
                return Err(Fault::UnknownType(t));
            }
            let len = plain.get(at..at + 4).ok_or(Fault::Truncated)?;
            let len = u32::from_le_bytes([len[0], len[1], len[2], len[3]]) as usize;
            at += 4;
            let end = at.checked_add(len).ok_or(Fault::Truncated)?;
            let body = plain.get(at..end).ok_or(Fault::Truncated)?;
            let mut record = Record::new(t);
            let mut p = 0usize;
            while p < body.len() {
                let head = body.get(p..p + 3).ok_or(Fault::Truncated)?;
                let n = usize::from(u16::from_le_bytes([head[1], head[2]]));
                let bytes = body.get(p + 3..p + 3 + n).ok_or(Fault::Truncated)?;
                record.push(head[0], bytes);
                p += 3 + n;
            }
            record.check()?;
            records.push(record);
            at = end;
        }
        if plain[at..].iter().any(|&b| b != 0) {
            return Err(Fault::Trailing);
        }
        check_across(&records)?;
        Ok(Contents { records })
    }

    /// The records of one type, with their positions.
    pub fn of(&self, kind: u8) -> impl Iterator<Item = (usize, &Record)> {
        self.records
            .iter()
            .enumerate()
            .filter(move |(_, r)| r.kind == kind)
    }
}

/// The flags field of a key or wallet record, by its type.
fn flags_field(record: &Record) -> Option<u8> {
    match record.kind {
        kind::KEY => Some(field::KEY_FLAGS),
        kind::WALLET => Some(field::WALLET_FLAGS),
        _ => None,
    }
}

/// Whether a key or wallet record starts ticked on the Load sheet an
/// unlock shows: its flags' bit 0 clear, or no flags at all.
pub fn loads_at_unlock(record: &Record) -> bool {
    let Some(f) = flags_field(record) else {
        return false;
    };
    record
        .field(f)
        .and_then(|b| b.first())
        .is_none_or(|b| b & NO_LOAD_AT_UNLOCK == 0)
}

/// Sets whether a key or wallet record loads at unlock, keeping its other
/// flags; a record left with no flag set has no flags field.
pub fn set_loads_at_unlock(record: &mut Record, loads: bool) {
    let Some(f) = flags_field(record) else {
        return;
    };
    let old = record
        .field(f)
        .and_then(|b| b.first().copied())
        .unwrap_or(0);
    let new = if loads {
        old & !NO_LOAD_AT_UNLOCK
    } else {
        old | NO_LOAD_AT_UNLOCK
    };
    if new == 0 {
        record.set(f, &[]);
    } else {
        record.set(f, &[new]);
    }
}

pub use crate::layouts::words_payload;

/// The words a key record holds, when it holds words rather than a seed.
pub fn words_of(record: &Record) -> Option<Mnemonic> {
    let payload = record.field(field::KEY)?;
    match crate::layouts::payload_of(crate::layouts::KIND_WORDS, payload)? {
        crate::layouts::Opened::Words(m) => Some(m),
        _ => None,
    }
}
