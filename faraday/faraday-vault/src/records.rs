//! A slot's plaintext (`docs/VAULT.md` §7): the format byte, typed
//! records, the end record, and zeros to the slot's length.
//!
//! A record is a type byte, a `u32` body length and the body; a body is
//! fields, each a number, a `u16` length and the bytes. What each type
//! may hold is [`rules`]: a field it does not list, a required field
//! missing, a field repeated where it may not be, text that is not
//! UTF-8, or a value of the wrong length refuses the slot. So does an
//! unknown type: a build reads only what it knows.
//!
//! Every field's bytes are wiped when the record is dropped.

use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::bip39::{Language, Mnemonic};
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
}

/// The fields of each record type, by number.
pub mod field {
    /// Slot label: the label.
    pub const LABEL: u8 = 1;

    /// Bitcoin key: the `osk-backup` kind-1 or kind-2 payload.
    pub const KEY: u8 = 1;
    /// Bitcoin key: what the person calls it.
    pub const KEY_LABEL: u8 = 2;
    /// Bitcoin key: flags, bit 0 load at unlock.
    pub const KEY_FLAGS: u8 = 3;
    /// Bitcoin key: its BIP-39 passphrase, only when the person chose to
    /// store it.
    pub const KEY_PASSPHRASE: u8 = 4;

    /// Wallet: BIP-388 text or a descriptor.
    pub const WALLET: u8 = 1;
    /// Wallet: its name.
    pub const WALLET_NAME: u8 = 2;

    /// Note: the text.
    pub const NOTE: u8 = 1;
    /// Recovery sheet: the `osk-backup` kind-4 payload.
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
    /// Signing round: the secret state, as `osk-psbt` serialises it (for
    /// FROST, the carry file).
    pub const ROUND_STATE: u8 = 4;
    /// Signing round: set when the round has been used.
    pub const ROUND_USED: u8 = 5;

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
}

/// Bit 0 of a key's flags: load it into the session at unlock.
pub const LOAD_AT_UNLOCK: u8 = 1;

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
        }
    }
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
const WALLET_RULES: &[Rule] = &[rule(1, true, false, TEXT), rule(2, false, false, TEXT)];
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
                osk_backup::oskb::KIND_WORDS
            } else {
                osk_backup::oskb::KIND_SEED
            };
            if osk_backup::oskb::payload_of(k, payload).is_none() {
                return Err(Fault::Value(self.kind, field::KEY));
            }
        }
        if self.kind == kind::SHEET
            && let Some(payload) = self.field(field::SHEET)
            && osk_backup::oskb::payload_of(osk_backup::oskb::KIND_SHEET, payload).is_none()
        {
            return Err(Fault::Value(self.kind, field::SHEET));
        }
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        1 + 4 + self.fields.iter().map(|f| 3 + f.bytes.len()).sum::<usize>()
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
        if self
            .records
            .iter()
            .filter(|r| r.kind == kind::SLOT_LABEL)
            .count()
            > 1
        {
            return Err(Error::Contents(Fault::TwoLabels));
        }
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
        let contents = Contents { records };
        if contents
            .records
            .iter()
            .filter(|r| r.kind == kind::SLOT_LABEL)
            .count()
            > 1
        {
            return Err(Fault::TwoLabels);
        }
        Ok(contents)
    }

    /// The records of one type, with their positions.
    pub fn of(&self, kind: u8) -> impl Iterator<Item = (usize, &Record)> {
        self.records
            .iter()
            .enumerate()
            .filter(move |(_, r)| r.kind == kind)
    }
}

/// A key's words as the `osk-backup` kind-1 payload a key record holds:
/// the word count, the wordlist, and the entropy padded to 32 bytes.
pub fn words_payload(m: &Mnemonic) -> Zeroizing<Vec<u8>> {
    let entropy = m.entropy();
    let bytes = entropy.expose().as_bytes();
    let lang = Language::ALL
        .iter()
        .position(|l| *l == m.language())
        .unwrap_or(0) as u8;
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(alloc::vec![0u8; 34]);
    out[0] = m.word_count() as u8;
    out[1] = lang;
    out[2..2 + bytes.len()].copy_from_slice(bytes);
    out
}

/// The words a key record holds, when it holds words rather than a seed.
pub fn words_of(record: &Record) -> Option<Mnemonic> {
    let payload = record.field(field::KEY)?;
    match osk_backup::oskb::payload_of(osk_backup::oskb::KIND_WORDS, payload)? {
        osk_backup::oskb::Opened::Words(m) => Some(m),
        _ => None,
    }
}
