//! The text inside a vault (`docs/VAULT.md` §7, §7.1, §2, §2.1): the
//! TOML document of a file vault and the top-level keys of a QR vault,
//! written from and read into the vault's records ([`Record`], whose
//! fields are the bytes the other modules of this crate lay out).
//!
//! - [`write`] gives a file vault's document, each item in its published
//!   text form, in the order of §7's table, padded with line feeds to
//!   the vault's [`Size`]: one canonical form.
//! - [`read`] takes any TOML v1.0.0 document §7 allows (another key
//!   order, comments, white space, another string form, `[[seed]]` or
//!   `seed = [{…}]`) and refuses what §7.1 says to refuse, each refusal
//!   naming the kind of item, its place among the items of that kind and
//!   the key ([`Refusal`]). The records come out grouped as §7's table
//!   lists them, so two spellings of one vault read as one.
//! - [`write_qr`] and [`read_qr`] are the QR vault's one item, padded to
//!   [`QR_LEN`]; [`form`] says which of the two a plaintext is.
//!
//! GPG keys are not read yet: a `[[gpg]]` item and a GPG record are each
//! refused as [`Why::GpgNotRead`]. A signing round (type 9) is not
//! carried and is refused on writing.
//!
//! Every string is kept in a buffer that is wiped when it drops; what
//! `toml_parser` and the parsers of words and policies keep is not
//! (`docs/deps/toml_parser.md`).

mod tree;

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use faraday_bip::base64;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

use crate::child::{self, ChildOf};
use crate::codex32::{self, SetFacts, State};
use crate::layouts::{self, Opened};
use crate::records::{Fault, Record, field, kind};
use crate::{Contents, Seals, totp};
use tree::{Arr, Table, Val};

/// The most a vault's contents are, in bytes (§2).
pub const MAX_LEN: usize = 4 * 1024 * 1024;

/// The length of every QR vault's contents (§2.1): the line of a
/// 24-word seed of the longest words, `words = "…"` and a line feed: 9
/// bytes, 215 of words, 2.
pub const QR_LEN: usize = 226;

/// The longest BIP-39 passphrase a QR vault holds, in bytes of its TOML
/// text, so that its two lines are no longer than [`QR_LEN`]: the
/// passphrase's line is 13 bytes beside the text, its quotes and line
/// feed 3, and the `for` line 17: 13 + 3 + 17 = 33.
pub const QR_PASSPHRASE_MAX: usize = QR_LEN - 33;

/// The longest note a QR vault holds, in bytes of its TOML text (escapes
/// counted as written, so a quote or a backslash is 2 bytes): the line is
/// `note = "` of 8 bytes, the text, then the closing quote and the line
/// feed, 2: 8 + 2 = 10 beside the text.
pub const QR_NOTE_MAX: usize = QR_LEN - 10;

/// The sizes of a file vault's contents (§2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// 64 KiB.
    Kib64,
    /// 256 KiB, the default.
    Kib256,
    /// 1 MiB.
    Mib1,
    /// 4 MiB.
    Mib4,
}

impl Size {
    /// The contents' length in bytes.
    pub const fn bytes(self) -> usize {
        match self {
            Size::Kib64 => 64 * 1024,
            Size::Kib256 => 256 * 1024,
            Size::Mib1 => 1024 * 1024,
            Size::Mib4 => MAX_LEN,
        }
    }

    /// The smallest size that holds `len` bytes.
    pub const fn holding(len: usize) -> Option<Size> {
        if len <= 64 * 1024 {
            Some(Size::Kib64)
        } else if len <= 256 * 1024 {
            Some(Size::Kib256)
        } else if len <= 1024 * 1024 {
            Some(Size::Mib1)
        } else if len <= MAX_LEN {
            Some(Size::Mib4)
        } else {
            None
        }
    }
}

/// A file vault's contents: its `vault` table and its items.
#[derive(Clone, Debug)]
pub struct Document {
    /// The vault identifier (§7), 16 bytes drawn once when the vault is
    /// made.
    pub id: [u8; 16],
    /// The seal count and the time of the last seal.
    pub seals: Seals,
    /// The items. The vault's label is a slot label record (type 1) and
    /// is `vault.name`.
    pub contents: Contents,
}

/// Which of the two forms a plaintext is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// A file vault: it has a `vault` table.
    File,
    /// A QR vault: it has none.
    Qr,
}

/// Why contents were refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// Not TOML v1.0.0, at this line (from 1).
    Syntax(usize),
    /// Not UTF-8.
    NotUtf8,
    /// Longer than [`MAX_LEN`], or than the size written to.
    TooLong,
    /// A file vault with no `vault` table.
    NoVault,
    /// The `vault` table is not the first.
    VaultNotFirst,
    /// A table or key §7 does not name.
    Unknown,
    /// A value of the wrong type.
    WrongType,
    /// A required key is missing.
    Missing,
    /// Two keys of which one is allowed, or one that goes with another
    /// kind of value.
    Exclusive,
    /// `format` is not 1.
    Format,
    /// A seal count that is not 1 or more.
    Seals,
    /// Words that are not a valid BIP-39 phrase of their list.
    BadWords,
    /// Hex that is not lower case, or not of its length.
    BadHex,
    /// A policy or descriptor that does not parse.
    BadWallet,
    /// A codex32 set's facts, or a codex32 string, that are not a set or
    /// share this layout holds.
    BadCodex32,
    /// BIP-85 facts that are not child facts this layout holds.
    BadChild,
    /// A PEM block other than the one the key names.
    BadPem,
    /// An `otp` that is not an `otpauth://` URI Faraday reads.
    BadOtp,
    /// A fingerprint, a GUID, an identifier, a state, a list name or a
    /// date-time that is not one.
    BadValue,
    /// A number outside its range.
    Range,
    /// Two items of a kind that holds one per key or per transaction.
    Twice,
    /// A GPG key: not read yet.
    GpgNotRead,
    /// A record that is not carried (a signing round) or whose fields
    /// the layouts refuse.
    NotCarried,
    /// An item a QR vault does not hold, or a QR vault of the wrong length.
    NotQr,
}

/// A refusal: what, in which item and at which key. `item` is the kind's
/// TOML name (empty for the document), `index` counts the items of that
/// kind from 0, and `key` is the key (empty when it is the item).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// What was wrong.
    pub why: Why,
    /// The table: `seed`, `wallet`, `vault` and so on.
    pub item: &'static str,
    /// Its place among the tables of that name.
    pub index: usize,
    /// The key, as the document or the Spec names it.
    pub key: String,
}

fn refuse(why: Why) -> Refusal {
    Refusal {
        why,
        item: "",
        index: 0,
        key: String::new(),
    }
}

/// The items' TOML names in §7's order, with their record types.
const ORDER: [(u8, &str); 12] = [
    (kind::KEY, "seed"),
    (kind::WALLET, "wallet"),
    (kind::NOTE, "note"),
    (kind::SHEET, "sheet"),
    (kind::ENTRY, "entry"),
    (kind::GPG, "gpg"),
    (kind::SECURE_BOOT, "secure_boot"),
    (kind::AMOUNTS, "signed"),
    (kind::PLAN, "plan"),
    (kind::WALLET_SHARE, "wallet_share"),
    (kind::PASSPHRASE, "passphrase"),
    (kind::CODEX32_SHARE, "share"),
];

/// The names of the word lists other than English (§7).
const LISTS: [(Language, &str); 10] = [
    (Language::English, "english"),
    (Language::Japanese, "japanese"),
    (Language::Korean, "korean"),
    (Language::Spanish, "spanish"),
    (Language::ChineseSimplified, "chinese-simplified"),
    (Language::ChineseTraditional, "chinese-traditional"),
    (Language::French, "french"),
    (Language::Italian, "italian"),
    (Language::Czech, "czech"),
    (Language::Portuguese, "portuguese"),
];

const STATES: [(State, &str); 5] = [
    (State::Made, "made"),
    (State::Placed, "placed"),
    (State::Lost, "lost"),
    (State::Exposed, "exposed"),
    (State::Destroyed, "destroyed"),
];

/// The PEM keys of a Secure Boot item: the TOML key, the record field and
/// the PEM label, in §7's order.
const PEMS: [(&str, u8, &str); 6] = [
    ("pk_key", 2, "PRIVATE KEY"),
    ("kek_key", 4, "PRIVATE KEY"),
    ("db_key", 6, "PRIVATE KEY"),
    ("pk_cert", 3, "CERTIFICATE"),
    ("kek_cert", 5, "CERTIFICATE"),
    ("db_cert", 7, "CERTIFICATE"),
];

// ---------------------------------------------------------------- writing

/// The document being written, held to a length that was reserved whole,
/// so that no growth leaves a copy behind.
struct Out {
    bytes: Zeroizing<Vec<u8>>,
    limit: usize,
    full: bool,
}

impl Out {
    fn new(limit: usize) -> Result<Out, Refusal> {
        let mut bytes: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
        bytes
            .try_reserve_exact(limit)
            .map_err(|_| refuse(Why::TooLong))?;
        Ok(Out {
            bytes,
            limit,
            full: false,
        })
    }

    fn raw(&mut self, s: &[u8]) {
        if self.bytes.len() + s.len() > self.limit {
            self.full = true;
        } else {
            self.bytes.extend_from_slice(s);
        }
    }

    fn s(&mut self, s: &str) {
        self.raw(s.as_bytes());
    }

    fn num(&mut self, n: u64) {
        let mut buf = [0u8; 20];
        let mut i = buf.len();
        let mut n = n;
        loop {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        self.raw(&buf[i..]);
    }

    fn hex(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.raw(&[HEX[usize::from(b >> 4)], HEX[usize::from(b & 15)]]);
        }
    }

    /// A basic string: `"` and `\` escaped, control characters as escapes
    /// and everything else as written.
    fn text(&mut self, s: &str) {
        self.raw(b"\"");
        for ch in s.chars() {
            match ch {
                '"' => self.s("\\\""),
                '\\' => self.s("\\\\"),
                '\n' => self.s("\\n"),
                '\t' => self.s("\\t"),
                '\r' => self.s("\\r"),
                '\u{8}' => self.s("\\b"),
                '\u{c}' => self.s("\\f"),
                c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                    self.s("\\u00");
                    self.hex(&[c as u8]);
                }
                c => {
                    let mut b = [0u8; 4];
                    self.s(c.encode_utf8(&mut b));
                }
            }
        }
        self.raw(b"\"");
    }

    fn line(&mut self, key: &str, f: impl FnOnce(&mut Out)) {
        self.s(key);
        self.s(" = ");
        f(self);
        self.raw(b"\n");
    }

    fn strs(&mut self, items: &[&str]) {
        self.raw(b"[");
        for (i, s) in items.iter().enumerate() {
            if i > 0 {
                self.s(", ");
            }
            self.text(s);
        }
        self.raw(b"]");
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn at(item: &'static str, index: usize, key: &str, why: Why) -> Refusal {
    Refusal {
        why,
        item,
        index,
        key: key.into(),
    }
}

/// Writes a file vault's contents: the TOML document of `doc`, padded
/// with line feeds to `size`.
pub fn write(doc: &Document, size: Size) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    let mut out = Out::new(size.bytes())?;
    for r in &doc.contents.records {
        if r.kind == kind::SEALS {
            return Err(refuse(Why::Seals));
        }
        if r.kind != kind::SLOT_LABEL && !ORDER.iter().any(|&(k, _)| k == r.kind) {
            return Err(refuse(Why::NotCarried));
        }
    }
    if doc.seals.count == 0 || doc.seals.count > i64::MAX as u64 {
        return Err(at("vault", 0, "seals", Why::Seals));
    }
    if doc.seals.last.is_some_and(|t| t > MAX_TIME) {
        return Err(at("vault", 0, "sealed", Why::BadValue));
    }
    out.s("[vault]\n");
    out.line("format", |o| o.num(1));
    out.line("id", |o| {
        o.raw(b"\"");
        o.hex(&doc.id);
        o.raw(b"\"");
    });
    let mut labels = doc.contents.of(kind::SLOT_LABEL);
    if let Some((_, r)) = labels.next() {
        if labels.next().is_some() {
            return Err(at("vault", 0, "name", Why::Twice));
        }
        r.check()
            .map_err(|_| at("vault", 0, "name", Why::BadValue))?;
        let name = r.text(field::LABEL).unwrap_or("");
        out.line("name", |o| o.text(name));
    }
    out.line("seals", |o| o.num(doc.seals.count));
    if let Some(t) = doc.seals.last {
        out.line("sealed", |o| write_date(o, t));
    }
    for &(k, name) in &ORDER {
        for (index, (_, r)) in doc.contents.of(k).enumerate() {
            r.check().map_err(|f| at(name, index, "", fault_why(f)))?;
            out.s("\n[[");
            out.s(name);
            out.s("]]\n");
            write_item(&mut out, k, name, index, r)?;
        }
    }
    across(&doc.contents.records)?;
    if out.full {
        return Err(refuse(Why::TooLong));
    }
    let mut bytes = out.bytes;
    bytes.resize(size.bytes(), b'\n');
    Ok(bytes)
}

fn fault_why(f: Fault) -> Why {
    match f {
        Fault::Seals => Why::Seals,
        Fault::Duplicate { .. } | Fault::TwoLabels => Why::Twice,
        _ => Why::BadValue,
    }
}

/// The last second a four-digit year holds: 9999-12-31T23:59:59Z.
const MAX_TIME: u64 = 253_402_300_799;

fn write_date(o: &mut Out, t: u64) {
    let (days, rem) = (t / 86_400, t % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let two = |o: &mut Out, n: u64| {
        o.raw(&[b'0' + (n / 10) as u8, b'0' + (n % 10) as u8]);
    };
    let y = y as u64;
    two(o, y / 100);
    two(o, y % 100);
    o.raw(b"-");
    two(o, m as u64);
    o.raw(b"-");
    two(o, d as u64);
    o.raw(b"T");
    two(o, rem / 3600);
    o.raw(b":");
    two(o, rem % 3600 / 60);
    o.raw(b":");
    two(o, rem % 60);
    o.raw(b"Z");
}

fn text_of(r: &Record, n: u8) -> &str {
    r.text(n).unwrap_or("")
}

fn opt_text(o: &mut Out, r: &Record, n: u8, key: &str) {
    if r.field(n).is_some() {
        o.line(key, |o| o.text(text_of(r, n)));
    }
}

fn write_item(
    o: &mut Out,
    k: u8,
    name: &'static str,
    index: usize,
    r: &Record,
) -> Result<(), Refusal> {
    let bad = |key: &str, why: Why| at(name, index, key, why);
    match k {
        kind::KEY => w_key(o, name, index, r)?,
        kind::WALLET => w_wallet(o, name, index, r)?,
        kind::NOTE => o.line("text", |o| o.text(text_of(r, field::NOTE))),
        kind::SHEET => w_sheet(o, name, index, r)?,
        kind::ENTRY => w_entry(o, name, index, r)?,
        kind::GPG => return Err(bad("", Why::GpgNotRead)),
        kind::SECURE_BOOT => w_secure_boot(o, name, index, r)?,
        kind::AMOUNTS => w_amounts(o, name, index, r)?,
        kind::PLAN => w_plan(o, name, index, r)?,
        kind::WALLET_SHARE => w_wallet_share(o, name, index, r)?,
        kind::PASSPHRASE => w_passphrase(o, name, index, r)?,
        kind::CODEX32_SHARE => w_codex32_share(o, name, index, r)?,
        _ => return Err(bad("", Why::NotCarried)),
    }
    Ok(())
}

fn write_words(o: &mut Out, m: &Mnemonic) {
    o.raw(b"\"");
    for (i, idx) in m.indices().iter().enumerate() {
        if i > 0 {
            o.raw(b" ");
        }
        o.s(m.language().word(*idx));
    }
    o.raw(b"\"");
}

fn write_set(o: &mut Out, f: &SetFacts) {
    o.s("{k = ");
    o.num(u64::from(f.threshold));
    o.s(", id = \"");
    o.raw(&f.identifier);
    o.s("\", lengths = [");
    o.num(u64::from(f.length));
    o.s("], unknown = ");
    o.num(u64::from(f.unknown));
    o.s(", shares = [");
    for (i, (c, st)) in f.strings.iter().enumerate() {
        if i > 0 {
            o.s(", ");
        }
        o.s("{index = \"");
        o.raw(&[*c]);
        o.s("\", state = \"");
        o.s(STATES
            .iter()
            .find(|(s, _)| s == st)
            .map_or("made", |(_, n)| n));
        o.s("\"}");
    }
    o.s("]}");
}

fn write_guid(o: &mut Out, g: &[u8]) {
    // UEFI keeps the first three fields little-endian.
    let rev = |a: &[u8]| -> Vec<u8> { a.iter().rev().copied().collect() };
    o.hex(&rev(&g[0..4]));
    o.raw(b"-");
    o.hex(&rev(&g[4..6]));
    o.raw(b"-");
    o.hex(&rev(&g[6..8]));
    o.raw(b"-");
    o.hex(&g[8..10]);
    o.raw(b"-");
    o.hex(&g[10..16]);
}

/// A PEM block as a multi-line basic string.
fn write_pem(o: &mut Out, label: &str, der: &[u8]) {
    let mut b64: Zeroizing<Vec<u8>> = Zeroizing::new(alloc::vec![0u8; der.len().div_ceil(3) * 4]);
    let n = base64::encode_into(der, &mut b64);
    for pad in b64.iter_mut().skip(n) {
        *pad = b'=';
    }
    o.s("\"\"\"\n-----BEGIN ");
    o.s(label);
    o.s("-----\n");
    for line in b64.chunks(64) {
        o.raw(line);
        o.raw(b"\n");
    }
    o.s("-----END ");
    o.s(label);
    o.s("-----\n\"\"\"");
}

// ---------------------------------------------------------------- reading

/// A table being taken apart: each key read is removed, and `finish`
/// refuses what is left.
struct Item {
    t: Table,
    name: &'static str,
    index: usize,
    prefix: &'static str,
}

impl Item {
    fn err(&self, why: Why, key: &str) -> Refusal {
        let mut k = String::new();
        if !self.prefix.is_empty() {
            k.push_str(self.prefix);
            k.push('.');
        }
        k.extend(key.chars().take(64));
        at(self.name, self.index, &k, why)
    }

    fn val(&mut self, key: &str) -> Option<Val> {
        self.t.take(key)
    }

    fn str(&mut self, key: &str) -> Result<Option<Zeroizing<String>>, Refusal> {
        match self.val(key) {
            None => Ok(None),
            Some(Val::Str(s)) => Ok(Some(s)),
            Some(_) => Err(self.err(Why::WrongType, key)),
        }
    }

    fn need_str(&mut self, key: &str) -> Result<Zeroizing<String>, Refusal> {
        self.str(key)?.ok_or_else(|| self.err(Why::Missing, key))
    }

    fn int(&mut self, key: &str) -> Result<Option<i64>, Refusal> {
        match self.val(key) {
            None => Ok(None),
            Some(Val::Int(n)) => Ok(Some(n)),
            Some(_) => Err(self.err(Why::WrongType, key)),
        }
    }

    fn need_int(&mut self, key: &str) -> Result<i64, Refusal> {
        self.int(key)?.ok_or_else(|| self.err(Why::Missing, key))
    }

    fn small(&mut self, key: &str, max: u32) -> Result<Option<u32>, Refusal> {
        match self.int(key)? {
            None => Ok(None),
            Some(n) => u32::try_from(n)
                .ok()
                .filter(|&n| n <= max)
                .map(Some)
                .ok_or_else(|| self.err(Why::Range, key)),
        }
    }

    fn bool(&mut self, key: &str) -> Result<Option<bool>, Refusal> {
        match self.val(key) {
            None => Ok(None),
            Some(Val::Bool(b)) => Ok(Some(b)),
            Some(_) => Err(self.err(Why::WrongType, key)),
        }
    }

    fn array(&mut self, key: &str) -> Result<Option<Arr>, Refusal> {
        match self.val(key) {
            None => Ok(None),
            Some(Val::Array(a)) => Ok(Some(a)),
            Some(_) => Err(self.err(Why::WrongType, key)),
        }
    }

    fn strs(&mut self, key: &str) -> Result<Vec<Zeroizing<String>>, Refusal> {
        let Some(a) = self.array(key)? else {
            return Ok(Vec::new());
        };
        a.items
            .into_iter()
            .map(|v| match v {
                Val::Str(s) => Ok(s),
                _ => Err(self.err(Why::WrongType, key)),
            })
            .collect()
    }

    fn table(&mut self, key: &str, prefix: &'static str) -> Result<Option<Item>, Refusal> {
        match self.val(key) {
            None => Ok(None),
            Some(Val::Table(t)) => Ok(Some(Item {
                t,
                name: self.name,
                index: self.index,
                prefix,
            })),
            Some(_) => Err(self.err(Why::WrongType, key)),
        }
    }

    fn finish(self) -> Result<(), Refusal> {
        match self.t.entries.first() {
            Some((k, _)) => Err(self.err(Why::Unknown, k)),
            None => Ok(()),
        }
    }
}

fn unhex(s: &str, len: Option<usize>) -> Option<Zeroizing<Vec<u8>>> {
    let b = s.as_bytes();
    if !b.len().is_multiple_of(2) || len.is_some_and(|n| n * 2 != b.len()) {
        return None;
    }
    let nib = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(b.len() / 2));
    for p in b.chunks(2) {
        out.push(nib(p[0])? << 4 | nib(p[1])?);
    }
    Some(out)
}

/// A whole item: its table, and what it asks of its records.
fn finish(it: Item, rec: Record) -> Result<Record, Refusal> {
    let (name, index) = (it.name, it.index);
    it.finish()?;
    rec.check().map_err(|f| at(name, index, "", fault_why(f)))?;
    Ok(rec)
}

/// Costs a parse of the item's words, policy or descriptor: one item of a
/// file or QR vault as a record.
fn read_item(c: &Compute, k: u8, mut it: Item) -> Result<Record, Refusal> {
    let mut rec = Record::new(k);
    match k {
        kind::KEY => read_seed(c, &mut it, &mut rec)?,
        kind::WALLET => r_wallet(c, &mut it, &mut rec)?,
        kind::NOTE => rec.push(field::NOTE, it.need_str("text")?.as_bytes()),
        kind::SHEET => r_sheet(&mut it, &mut rec)?,
        kind::ENTRY => r_entry(&mut it, &mut rec)?,
        kind::GPG => return Err(it.err(Why::GpgNotRead, "")),
        kind::SECURE_BOOT => r_secure_boot(&mut it, &mut rec)?,
        kind::AMOUNTS => r_amounts(&mut it, &mut rec)?,
        kind::PLAN => r_plan(&mut it, &mut rec)?,
        kind::WALLET_SHARE => r_wallet_share(&mut it, &mut rec)?,
        kind::PASSPHRASE => r_passphrase(&mut it, &mut rec)?,
        kind::CODEX32_SHARE => r_codex32_share(&mut it, &mut rec)?,
        _ => return Err(it.err(Why::Unknown, "")),
    }
    finish(it, rec)
}

/// Costs a parse of the seed's words: the keys of a `seed` table as a key
/// record's fields.
fn read_seed(c: &Compute, it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    let words = it.str("words")?;
    let hex = it.str("hex")?;
    let list = it.str("list")?;
    match (words, hex) {
        (Some(_), Some(_)) => return Err(it.err(Why::Exclusive, "hex")),
        (None, None) => return Err(it.err(Why::Missing, "words")),
        (Some(w), None) => {
            let lang = match list {
                None => Language::English,
                Some(l) => LISTS
                    .iter()
                    .find(|(_, n)| *n == l.as_str())
                    .map(|(lang, _)| *lang)
                    .ok_or_else(|| it.err(Why::BadValue, "list"))?,
            };
            let m = Mnemonic::parse(c, lang, &w).map_err(|_| it.err(Why::BadWords, "words"))?;
            rec.push(field::KEY, &layouts::words_payload(&m));
        }
        (None, Some(h)) => {
            if list.is_some() {
                return Err(it.err(Why::Exclusive, "list"));
            }
            let seed = unhex(&h, None).ok_or_else(|| it.err(Why::BadHex, "hex"))?;
            let payload = layouts::seed_payload(&seed).ok_or_else(|| it.err(Why::BadHex, "hex"))?;
            rec.push(field::KEY, &payload);
        }
    }
    if let Some(l) = it.str("label")? {
        rec.push(field::KEY_LABEL, l.as_bytes());
    }
    let mut flags = 0u8;
    if it.bool("load")? == Some(false) {
        flags |= crate::records::NO_LOAD_AT_UNLOCK;
    }
    if it.bool("held")? == Some(true) {
        flags |= crate::records::BACKUP_PENDING;
    }
    if flags != 0 {
        rec.push(field::KEY_FLAGS, &[flags]);
    }
    if let Some(p) = it.str("passphrase")? {
        rec.push(field::KEY_PASSPHRASE, p.as_bytes());
    }
    if let Some(mut t) = it.table("codex32", "codex32")? {
        let facts = read_set(&mut t)?;
        let payload =
            codex32::set_payload(&facts).ok_or_else(|| it.err(Why::BadCodex32, "codex32"))?;
        t.finish()?;
        rec.push(field::KEY_CODEX32, &payload);
    }
    if let Some(mut t) = it.table("child", "child")? {
        let id = t.need_str("parent")?;
        let parent = unhex(&id, Some(4)).ok_or_else(|| t.err(Why::BadHex, "parent"))?;
        if t.need_int("application")? != 39 {
            return Err(t.err(Why::BadChild, "application"));
        }
        let language = t
            .small("language", 255)?
            .ok_or_else(|| t.err(Why::Missing, "language"))?;
        let words = t
            .small("words", 255)?
            .ok_or_else(|| t.err(Why::Missing, "words"))?;
        let index = t
            .small("index", u32::MAX)?
            .ok_or_else(|| t.err(Why::Missing, "index"))?;
        let c = ChildOf {
            parent: [parent[0], parent[1], parent[2], parent[3]],
            language: language as u8,
            words: words as u8,
            index,
        };
        let payload = child::payload(&c).ok_or_else(|| it.err(Why::BadChild, "child"))?;
        t.finish()?;
        rec.push(field::KEY_CHILD, &payload);
    }
    Ok(())
}

fn read_set(t: &mut Item) -> Result<SetFacts, Refusal> {
    let k = t.small("k", 9)?.ok_or_else(|| t.err(Why::Missing, "k"))?;
    let id = t.need_str("id")?;
    let identifier: [u8; 4] = id
        .as_bytes()
        .try_into()
        .map_err(|_| t.err(Why::BadCodex32, "id"))?;
    let lengths = t
        .array("lengths")?
        .ok_or_else(|| t.err(Why::Missing, "lengths"))?;
    let mut length = None;
    for v in lengths.items {
        let Val::Int(n) = v else {
            return Err(t.err(Why::WrongType, "lengths"));
        };
        let n = u8::try_from(n).map_err(|_| t.err(Why::Range, "lengths"))?;
        if length.is_some_and(|l| l != n) {
            return Err(t.err(Why::BadCodex32, "lengths"));
        }
        length = Some(n);
    }
    let length = length.ok_or_else(|| t.err(Why::BadCodex32, "lengths"))?;
    let unknown = t
        .small("unknown", 255)?
        .ok_or_else(|| t.err(Why::Missing, "unknown"))?;
    let mut strings = Vec::new();
    if let Some(shares) = t.array("shares")? {
        for v in shares.items {
            let Val::Table(tab) = v else {
                return Err(t.err(Why::WrongType, "shares"));
            };
            let mut s = Item {
                t: tab,
                name: t.name,
                index: t.index,
                prefix: "codex32.shares",
            };
            let index = s.need_str("index")?;
            let state = s.need_str("state")?;
            let ch: [u8; 1] = index
                .as_bytes()
                .try_into()
                .map_err(|_| s.err(Why::BadCodex32, "index"))?;
            let state = STATES
                .iter()
                .find(|(_, n)| *n == state.as_str())
                .map(|(st, _)| *st)
                .ok_or_else(|| s.err(Why::BadValue, "state"))?;
            s.finish()?;
            strings.push((ch[0], state));
        }
    }
    Ok(SetFacts {
        threshold: k as u8,
        identifier,
        length,
        unknown: unknown as u8,
        strings,
    })
}

fn read_guid(s: &str) -> Option<[u8; 16]> {
    let b = s.as_bytes();
    if b.len() != 36 || [8, 13, 18, 23].iter().any(|&i| b[i] != b'-') {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    let flat: String = lower.chars().filter(|&c| c != '-').collect();
    let raw = unhex(&flat, Some(16))?;
    let mut g = [0u8; 16];
    g.copy_from_slice(&raw);
    g[0..4].reverse();
    g[4..6].reverse();
    g[6..8].reverse();
    Some(g)
}

/// The DER inside a PEM block of exactly this label, and nothing else.
fn read_pem(pem: &str, label: &str) -> Option<Zeroizing<Vec<u8>>> {
    let mut lines = pem.trim().lines();
    let first = lines.next()?.trim();
    let begin = first.strip_prefix("-----BEGIN ")?.strip_suffix("-----")?;
    if begin != label {
        return None;
    }
    let mut body: Zeroizing<String> = Zeroizing::new(String::with_capacity(pem.len()));
    let mut ended = false;
    for l in lines {
        let l = l.trim();
        if ended {
            return None;
        }
        if let Some(end) = l.strip_prefix("-----END ") {
            if end.strip_suffix("-----")? != label {
                return None;
            }
            ended = true;
        } else {
            body.push_str(l);
        }
    }
    let der = Zeroizing::new(base64::decode(&body).ok()?);
    (ended && !der.is_empty()).then_some(der)
}

/// The seconds since 1970 of an RFC 3339 date-time with seconds, or `None`.
fn unix_of(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    if !s.is_ascii()
        || b.len() < 20
        || !matches!(b[10], b'T' | b't' | b' ')
        || b[4] != b'-'
        || b[7] != b'-'
    {
        return None;
    }
    if b[13] != b':' || b[16] != b':' {
        return None;
    }
    let n = |r: core::ops::Range<usize>| -> Option<i64> {
        let t = s.get(r)?;
        t.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| t.parse().ok())?
    };
    let (y, m, d) = (n(0..4)?, n(5..7)?, n(8..10)?);
    let (h, mi, sec) = (n(11..13)?, n(14..16)?, n(17..19)?);
    let mut rest = &s[19..];
    if let Some(frac) = rest.strip_prefix('.') {
        let digits = frac.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        rest = &frac[digits..];
    }
    let offset = match rest {
        "Z" | "z" => 0,
        o if o.len() == 6 && (o.starts_with('+') || o.starts_with('-')) && &o[3..4] == ":" => {
            let (oh, om) = (n_of(&o[1..3])?, n_of(&o[4..6])?);
            if oh > 23 || om > 59 {
                return None;
            }
            let v = oh * 3600 + om * 60;
            if o.starts_with('-') { -v } else { v }
        }
        _ => return None,
    };
    let dim = [
        31,
        if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            29
        } else {
            28
        },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&m) || d < 1 || d > dim[(m - 1) as usize] || h > 23 || mi > 59 || sec > 60
    {
        return None;
    }
    // Civil date to days since 1970-01-01.
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + h * 3600 + mi * 60 + sec.min(59) - offset)
        .ok()
        .filter(|&t| t <= MAX_TIME)
}

fn n_of(t: &str) -> Option<i64> {
    t.bytes()
        .all(|c| c.is_ascii_digit())
        .then(|| t.parse().ok())?
}

fn parse_tree(bytes: &[u8]) -> Result<Table, Refusal> {
    if bytes.len() > MAX_LEN {
        return Err(refuse(Why::TooLong));
    }
    let text = core::str::from_utf8(bytes).map_err(|_| refuse(Why::NotUtf8))?;
    tree::parse(text).map_err(|line| refuse(Why::Syntax(line)))
}

/// The tables of one kind of item in the document, each with its place.
fn tables_of(name: &'static str, v: Val) -> Result<Vec<Item>, Refusal> {
    let Val::Array(a) = v else {
        return Err(at(name, 0, "", Why::WrongType));
    };
    a.items
        .into_iter()
        .enumerate()
        .map(|(index, t)| match t {
            Val::Table(t) => Ok(Item {
                t,
                name,
                index,
                prefix: "",
            }),
            _ => Err(at(name, index, "", Why::WrongType)),
        })
        .collect()
}

/// Costs a parse of each wallet's policy or descriptor and of each seed's
/// words: reads a file vault's contents (§7, §7.1), up to [`MAX_LEN`]
/// bytes of any length, the records grouped as §7's table lists them.
pub fn read(c: &Compute, bytes: &[u8]) -> Result<Document, Refusal> {
    let mut root = parse_tree(bytes)?;
    if root.entries.is_empty() || root.entries[0].0 != "vault" {
        return Err(refuse(if root.entries.iter().any(|(k, _)| k == "vault") {
            Why::VaultNotFirst
        } else {
            Why::NoVault
        }));
    }
    let Val::Table(vt) = root.entries.remove(0).1 else {
        return Err(at("vault", 0, "", Why::WrongType));
    };
    let mut v = Item {
        t: vt,
        name: "vault",
        index: 0,
        prefix: "",
    };
    if v.need_int("format")? != 1 {
        return Err(v.err(Why::Format, "format"));
    }
    let id = v.need_str("id")?;
    let id = unhex(&id, Some(16)).ok_or_else(|| v.err(Why::BadHex, "id"))?;
    let mut doc_id = [0u8; 16];
    doc_id.copy_from_slice(&id);
    let mut records = Vec::new();
    if let Some(name) = v.str("name")? {
        let r = Record::new(kind::SLOT_LABEL).with(field::LABEL, name.as_bytes());
        r.check().map_err(|_| v.err(Why::TooLong, "name"))?;
        records.push(r);
    }
    let count = v.need_int("seals")?;
    let count = u64::try_from(count)
        .ok()
        .filter(|&n| n >= 1)
        .ok_or_else(|| v.err(Why::Seals, "seals"))?;
    let last = match v.val("sealed") {
        None => None,
        Some(Val::Date(d)) => Some(unix_of(&d).ok_or_else(|| v.err(Why::BadValue, "sealed"))?),
        Some(_) => return Err(v.err(Why::WrongType, "sealed")),
    };
    v.finish()?;

    let mut groups: Vec<Vec<Record>> = ORDER.iter().map(|_| Vec::new()).collect();
    let mut seen = [false; 12];
    for (key, val) in root.entries {
        let Some(slot) = ORDER.iter().position(|&(_, n)| n == key) else {
            let mut k = String::new();
            k.extend(key.chars().take(64));
            return Err(Refusal {
                why: Why::Unknown,
                item: "",
                index: 0,
                key: k,
            });
        };
        if core::mem::replace(&mut seen[slot], true) {
            return Err(at(ORDER[slot].1, 0, "", Why::Twice));
        }
        let (k, name) = ORDER[slot];
        for it in tables_of(name, val)? {
            groups[slot].push(read_item(c, k, it)?);
        }
    }
    records.extend(groups.into_iter().flatten());
    across(&records)?;
    Ok(Document {
        id: doc_id,
        seals: Seals { count, last },
        contents: Contents { records },
    })
}

/// A passphrase for one key twice, signed amounts for one transaction
/// twice, a share of one index and key twice.
fn across(records: &[Record]) -> Result<(), Refusal> {
    for (k, name, by) in [
        (kind::PASSPHRASE, "passphrase", field::PASSPHRASE_KEY),
        (kind::AMOUNTS, "signed", field::AMOUNTS_TX),
        (kind::CODEX32_SHARE, "share", field::CODEX32_KEY),
    ] {
        let mut seen: BTreeSet<(&[u8], u8)> = BTreeSet::new();
        for (i, r) in records.iter().filter(|r| r.kind == k).enumerate() {
            let id = r.field(by).unwrap_or(&[]);
            let extra = if k == kind::CODEX32_SHARE {
                r.field(field::CODEX32_STRING).map_or(0, |s| s[8])
            } else {
                0
            };
            if !seen.insert((id, extra)) {
                return Err(at(name, i, "", Why::Twice));
            }
        }
    }
    Ok(())
}

/// Says which form a plaintext is, by whether it has a `vault` table.
pub fn form(bytes: &[u8]) -> Result<Form, Refusal> {
    let root = parse_tree(bytes)?;
    Ok(if root.entries.iter().any(|(k, _)| k == "vault") {
        Form::File
    } else {
        Form::Qr
    })
}

// ------------------------------------------------------------------- QR

/// Writes a QR vault's contents: the one item `record` as top-level keys
/// (`words`; `codex32` and `for`; `passphrase` and `for`; `note`), padded
/// with line feeds to [`QR_LEN`]. The record is a key of 12 or 24 words
/// of the English list with nothing else, a codex32 share, a passphrase
/// of at most [`QR_PASSPHRASE_MAX`] bytes, or a note of at most
/// [`QR_NOTE_MAX`]; an empty note is the empty form, the padding alone.
pub fn write_qr(record: &Record) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    let mut o = Out::new(QR_LEN)?;
    record.check().map_err(|f| at("", 0, "", fault_why(f)))?;
    match record.kind {
        kind::KEY => {
            let payload = record.field(field::KEY).unwrap_or(&[]);
            let Some(Opened::Words(m)) = layouts::payload_of(layouts::KIND_WORDS, payload) else {
                return Err(at("", 0, "words", Why::NotQr));
            };
            if record.fields.len() != 1
                || m.language() != Language::English
                || !matches!(m.word_count(), 12 | 24)
            {
                return Err(at("", 0, "words", Why::NotQr));
            }
            o.line("words", |o| write_words(o, &m));
        }
        kind::CODEX32_SHARE => {
            o.line("codex32", |o| {
                o.text(text_of(record, field::CODEX32_STRING))
            });
            o.line("for", |o| qr_for(o, record.field(field::CODEX32_KEY)));
        }
        kind::PASSPHRASE => {
            o.line("passphrase", |o| o.text(text_of(record, field::PASSPHRASE)));
            o.line("for", |o| qr_for(o, record.field(field::PASSPHRASE_KEY)));
        }
        kind::NOTE => {
            if record.fields.len() != 1 {
                return Err(at("", 0, "note", Why::NotQr));
            }
            // An empty note is the empty form: the padding alone.
            let note = text_of(record, field::NOTE);
            if !note.is_empty() {
                o.line("note", |o| o.text(note));
            }
        }
        _ => return Err(refuse(Why::NotQr)),
    }
    if o.full {
        let key = match record.kind {
            kind::KEY => "words",
            kind::CODEX32_SHARE => "codex32",
            kind::PASSPHRASE => "passphrase",
            _ => "note",
        };
        return Err(at("", 0, key, Why::TooLong));
    }
    let mut bytes = o.bytes;
    bytes.resize(QR_LEN, b'\n');
    Ok(bytes)
}

fn qr_for(o: &mut Out, key: Option<&[u8]>) {
    o.raw(b"\"");
    o.hex(key.unwrap_or(&[]));
    o.raw(b"\"");
}

/// Costs a parse of the words: reads a QR vault's contents, which are
/// [`QR_LEN`] bytes holding one item as top-level keys and no `vault`
/// table.
pub fn read_qr(c: &Compute, bytes: &[u8]) -> Result<Record, Refusal> {
    if bytes.len() != QR_LEN {
        return Err(refuse(Why::NotQr));
    }
    let mut root = parse_tree(bytes)?;
    let has = |k: &str| root.entries.iter().any(|(n, _)| n == k);
    if root.entries.is_empty() {
        return Ok(Record::new(kind::NOTE).with(field::NOTE, b""));
    }
    let allowed: &[&str] = if has("words") {
        &["words"]
    } else if has("note") {
        &["note"]
    } else if has("codex32") {
        &["codex32", "for"]
    } else {
        &["passphrase", "for"]
    };
    if let Some((n, _)) = root
        .entries
        .iter()
        .find(|(n, _)| !allowed.contains(&n.as_str()))
    {
        let mut key = String::new();
        key.extend(n.chars().take(64));
        return Err(at("", 0, &key, Why::Unknown));
    }
    let (k, name) = if has("words") {
        (kind::KEY, "seed")
    } else if has("codex32") {
        (kind::CODEX32_SHARE, "share")
    } else if has("note") {
        for e in root.entries.iter_mut().filter(|e| e.0 == "note") {
            e.0 = "text".into();
        }
        (kind::NOTE, "note")
    } else if has("passphrase") {
        // The file vault's item names its text `text`.
        for e in root.entries.iter_mut().filter(|e| e.0 == "passphrase") {
            e.0 = "text".into();
        }
        (kind::PASSPHRASE, "passphrase")
    } else {
        return Err(refuse(Why::NotQr));
    };
    let record = read_item(
        c,
        k,
        Item {
            t: root,
            name,
            index: 0,
            prefix: "",
        },
    )?;
    if record.kind == kind::KEY {
        let words = record.field(field::KEY).map_or(0, |p| p[0]);
        if record.fields.len() != 1
            || !matches!(words, 12 | 24)
            || record.field(field::KEY).is_some_and(|p| p[1] != 0)
        {
            return Err(at("", 0, "words", Why::NotQr));
        }
    }
    // A form that is shorter written than `write_qr` writes it (`note="…"`
    // without spaces, a literal string with no escapes) can hold more than
    // the limits: what `write_qr` would refuse as too long is refused here.
    if matches!(record.kind, kind::NOTE | kind::PASSPHRASE)
        && write_qr(&record).is_err_and(|r| r.why == Why::TooLong)
    {
        let key = if record.kind == kind::NOTE {
            "note"
        } else {
            "passphrase"
        };
        return Err(at("", 0, key, Why::TooLong));
    }
    Ok(record)
}

fn w_key(o: &mut Out, name: &'static str, index: usize, r: &Record) -> Result<(), Refusal> {
    let bad = |key: &str, why: Why| at(name, index, key, why);
    let payload = r.field(field::KEY).unwrap_or(&[]);
    match layouts::payload_of(
        if payload.len() == 34 {
            layouts::KIND_WORDS
        } else {
            layouts::KIND_SEED
        },
        payload,
    ) {
        Some(Opened::Words(m)) => {
            o.line("words", |o| write_words(o, &m));
            if m.language() != Language::English {
                let list = LISTS.iter().find(|(l, _)| *l == m.language());
                o.line("list", |o| o.text(list.map_or("english", |(_, n)| n)));
            }
        }
        Some(Opened::Seed(seed)) => o.line("hex", |o| {
            o.raw(b"\"");
            o.hex(&seed);
            o.raw(b"\"");
        }),
        _ => return Err(bad("words", Why::BadValue)),
    }
    opt_text(o, r, field::KEY_LABEL, "label");
    let flags = r.field(field::KEY_FLAGS).map_or(0, |f| f[0]);
    if flags & crate::records::NO_LOAD_AT_UNLOCK != 0 {
        o.line("load", |o| o.s("false"));
    }
    if flags & crate::records::BACKUP_PENDING != 0 {
        o.line("held", |o| o.s("true"));
    }
    opt_text(o, r, field::KEY_PASSPHRASE, "passphrase");
    if let Some(set) = r.field(field::KEY_CODEX32) {
        let f = codex32::set_of(set).ok_or_else(|| bad("codex32", Why::BadCodex32))?;
        o.line("codex32", |o| write_set(o, &f));
    }
    if let Some(c) = r.field(field::KEY_CHILD) {
        let c = child::child_of(c).ok_or_else(|| bad("child", Why::BadChild))?;
        o.line("child", |o| {
            o.s("{parent = \"");
            o.hex(&c.parent);
            o.s("\", application = 39, language = ");
            o.num(u64::from(c.language));
            o.s(", words = ");
            o.num(u64::from(c.words));
            o.s(", index = ");
            o.num(u64::from(c.index));
            o.s("}");
        });
    }
    Ok(())
}

fn w_wallet(o: &mut Out, name: &'static str, index: usize, r: &Record) -> Result<(), Refusal> {
    let _ = (name, index);
    let text = text_of(r, field::WALLET);
    let key = if text.contains('@') {
        "policy"
    } else {
        "descriptor"
    };
    o.line(key, |o| o.text(text));
    opt_text(o, r, field::WALLET_NAME, "name");
    if r.field(field::WALLET_FLAGS).is_some_and(|f| f[0] & 1 != 0) {
        o.line("load", |o| o.s("false"));
    }
    Ok(())
}

fn w_sheet(o: &mut Out, name: &'static str, index: usize, r: &Record) -> Result<(), Refusal> {
    let bad = |key: &str, why: Why| at(name, index, key, why);
    let payload = r.field(field::SHEET).unwrap_or(&[]);
    let Some(Opened::Sheet(s)) = layouts::payload_of(layouts::KIND_SHEET, payload) else {
        return Err(bad("descriptor", Why::BadValue));
    };
    for (key, part) in [
        ("descriptor", &s.descriptor),
        ("name", &s.name),
        ("note", &s.note),
    ] {
        let text = core::str::from_utf8(part).unwrap_or("");
        o.line(key, |o| o.text(text));
    }
    Ok(())
}

fn w_entry(o: &mut Out, name: &'static str, index: usize, r: &Record) -> Result<(), Refusal> {
    let bad = |key: &str, why: Why| at(name, index, key, why);
    o.line("title", |o| o.text(text_of(r, field::TITLE)));
    opt_text(o, r, field::USERNAME, "username");
    opt_text(o, r, field::PASSWORD, "password");
    opt_text(o, r, field::URL, "url");
    opt_text(o, r, field::NOTES, "notes");
    if let Some(f) = r.text(field::TOTP) {
        let uri = if f.starts_with("otpauth://") {
            Some(f.into())
        } else {
            totp::normalise(f, text_of(r, field::TITLE), text_of(r, field::USERNAME))
        };
        match uri {
            Some(u) if totp::parse(&u).is_ok() => {
                let u = Zeroizing::new(u);
                o.line("otp", |o| o.text(&u));
            }
            _ => return Err(bad("otp", Why::BadOtp)),
        }
    }
    Ok(())
}

fn w_secure_boot(o: &mut Out, name: &'static str, index: usize, r: &Record) -> Result<(), Refusal> {
    let g = r.field(1).unwrap_or(&[0; 16]);
    o.line("owner", |o| {
        o.raw(b"\"");
        write_guid(o, g);
        o.raw(b"\"");
    });
    for (key, n, label) in PEMS {
        if let Some(der) = r.field(n) {
            if der.is_empty() {
                return Err(at(name, index, key, Why::BadPem));
            }
            o.line(key, |o| write_pem(o, label, der));
        }
    }
    Ok(())
}

fn w_amounts(o: &mut Out, name: &'static str, index: usize, r: &Record) -> Result<(), Refusal> {
    let _ = (name, index);
    o.line("tx", |o| {
        o.raw(b"\"");
        o.hex(r.field(field::AMOUNTS_TX).unwrap_or(&[]));
        o.raw(b"\"");
    });
    o.line("amounts", |o| {
        o.raw(b"[");
        let mut first = true;
        for f in r.fields.iter().filter(|f| f.number == field::AMOUNTS_INPUT) {
            if !core::mem::replace(&mut first, false) {
                o.s(", ");
            }
            let mut a = [0u8; 8];
            a.copy_from_slice(&f.bytes);
            o.num(u64::from_le_bytes(a));
        }
        o.raw(b"]");
    });
    Ok(())
}

fn w_plan(o: &mut Out, name: &'static str, index: usize, r: &Record) -> Result<(), Refusal> {
    let _ = (name, index);
    o.line("wallet", |o| o.text(text_of(r, field::PLAN_WALLET)));
    if let Some(a) = r.text(field::PLAN_ANSWERS) {
        let lines: Vec<&str> = a.split('\n').collect();
        o.line("answers", |o| o.strs(&lines));
    }
    for (key, n) in [
        ("places", field::PLAN_PLACE),
        ("holds", field::PLAN_HOLDS),
        ("marks", field::PLAN_NOTE),
    ] {
        if r.field(n).is_some() {
            let items: Vec<&str> = r
                .fields
                .iter()
                .filter(|f| f.number == n)
                .map(|f| core::str::from_utf8(&f.bytes).unwrap_or(""))
                .collect();
            o.line(key, |o| o.strs(&items));
        }
    }
    Ok(())
}

fn w_wallet_share(
    o: &mut Out,
    name: &'static str,
    index: usize,
    r: &Record,
) -> Result<(), Refusal> {
    let _ = (name, index);
    o.line("text", |o| o.text(text_of(r, field::SHARE)));
    opt_text(o, r, field::SHARE_WALLET_NAME, "name");
    for (key, n) in [("index", field::SHARE_NUMBER), ("of", field::SHARE_OF)] {
        if let Some(v) = r.field(n) {
            o.line(key, |o| o.num(u64::from(v[0])));
        }
    }
    opt_text(o, r, field::SHARE_WALLET, "wallet");
    Ok(())
}

fn w_passphrase(o: &mut Out, name: &'static str, index: usize, r: &Record) -> Result<(), Refusal> {
    let _ = (name, index);
    o.line("for", |o| {
        o.raw(b"\"");
        o.hex(r.field(field::PASSPHRASE_KEY).unwrap_or(&[]));
        o.raw(b"\"");
    });
    o.line("text", |o| o.text(text_of(r, field::PASSPHRASE)));
    Ok(())
}

fn w_codex32_share(
    o: &mut Out,
    name: &'static str,
    index: usize,
    r: &Record,
) -> Result<(), Refusal> {
    let _ = (name, index);
    o.line("codex32", |o| o.text(text_of(r, field::CODEX32_STRING)));
    o.line("for", |o| {
        o.raw(b"\"");
        o.hex(r.field(field::CODEX32_KEY).unwrap_or(&[]));
        o.raw(b"\"");
    });
    Ok(())
}

/// Costs a parse of the wallet's policy or descriptor: a `wallet` table as a
/// wallet record's fields.
fn r_wallet(c: &Compute, it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    let policy = it.str("policy")?;
    let descriptor = it.str("descriptor")?;
    let (key, text, ok) = match (policy, descriptor) {
        (Some(_), Some(_)) => return Err(it.err(Why::Exclusive, "descriptor")),
        (None, None) => return Err(it.err(Why::Missing, "policy")),
        (Some(p), None) => {
            let ok = faraday_bip::policy::WalletPolicy::parse(c, &p).is_ok();
            ("policy", p, ok)
        }
        (None, Some(d)) => {
            let ok = faraday_bip::descriptor::reads_as_descriptor(c, &d);
            ("descriptor", d, ok)
        }
    };
    if !ok {
        return Err(it.err(Why::BadWallet, key));
    }
    rec.push(field::WALLET, text.as_bytes());
    if let Some(n) = it.str("name")? {
        rec.push(field::WALLET_NAME, n.as_bytes());
    }
    if it.bool("load")? == Some(false) {
        rec.push(field::WALLET_FLAGS, &[crate::records::NO_LOAD_AT_UNLOCK]);
    }
    Ok(())
}

fn r_sheet(it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    let mut part =
        |key| -> Result<Zeroizing<String>, Refusal> { Ok(it.str(key)?.unwrap_or_default()) };
    let (d, n, t) = (part("descriptor")?, part("name")?, part("note")?);
    let payload = layouts::sheet_payload(d.as_bytes(), n.as_bytes(), t.as_bytes())
        .ok_or_else(|| it.err(Why::TooLong, "descriptor"))?;
    rec.push(field::SHEET, &payload);
    Ok(())
}

fn r_entry(it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    rec.push(field::TITLE, it.need_str("title")?.as_bytes());
    for (key, n) in [
        ("username", field::USERNAME),
        ("password", field::PASSWORD),
        ("url", field::URL),
        ("notes", field::NOTES),
    ] {
        if let Some(s) = it.str(key)? {
            rec.push(n, s.as_bytes());
        }
    }
    if let Some(otp) = it.str("otp")? {
        if !otp.starts_with("otpauth://") || totp::parse(&otp).is_err() {
            return Err(it.err(Why::BadOtp, "otp"));
        }
        rec.push(field::TOTP, otp.as_bytes());
    }
    Ok(())
}

fn r_secure_boot(it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    let owner = it.need_str("owner")?;
    let guid = read_guid(&owner).ok_or_else(|| it.err(Why::BadValue, "owner"))?;
    rec.push(1, &guid);
    for (key, n, label) in PEMS {
        if let Some(pem) = it.str(key)? {
            let der = read_pem(&pem, label).ok_or_else(|| it.err(Why::BadPem, key))?;
            rec.push(n, &der);
        }
    }
    rec.fields.sort_by_key(|f| f.number);
    Ok(())
}

fn r_amounts(it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    let tx = it.need_str("tx")?;
    let tx = unhex(&tx, Some(32)).ok_or_else(|| it.err(Why::BadHex, "tx"))?;
    rec.push(field::AMOUNTS_TX, &tx);
    let amounts = it
        .array("amounts")?
        .ok_or_else(|| it.err(Why::Missing, "amounts"))?;
    for v in amounts.items {
        match v {
            Val::Int(n) if n >= 0 => rec.push(field::AMOUNTS_INPUT, &(n as u64).to_le_bytes()),
            Val::Int(_) => return Err(it.err(Why::Range, "amounts")),
            _ => return Err(it.err(Why::WrongType, "amounts")),
        }
    }
    Ok(())
}

fn r_plan(it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    rec.push(field::PLAN_WALLET, it.need_str("wallet")?.as_bytes());
    let answers = it.strs("answers")?;
    if !answers.is_empty() {
        let mut joined: Zeroizing<String> = Zeroizing::new(String::new());
        for (i, a) in answers.iter().enumerate() {
            if a.contains('\n') {
                return Err(it.err(Why::BadValue, "answers"));
            }
            if i > 0 {
                joined.push('\n');
            }
            joined.push_str(a);
        }
        rec.push(field::PLAN_ANSWERS, joined.as_bytes());
    }
    for (key, n) in [
        ("places", field::PLAN_PLACE),
        ("holds", field::PLAN_HOLDS),
        ("marks", field::PLAN_NOTE),
    ] {
        for s in it.strs(key)? {
            rec.push(n, s.as_bytes());
        }
    }
    Ok(())
}

fn r_wallet_share(it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    rec.push(field::SHARE, it.need_str("text")?.as_bytes());
    if let Some(n) = it.str("name")? {
        rec.push(field::SHARE_WALLET_NAME, n.as_bytes());
    }
    for (key, n) in [("index", field::SHARE_NUMBER), ("of", field::SHARE_OF)] {
        if let Some(v) = it.small(key, 255)? {
            rec.push(n, &[v as u8]);
        }
    }
    if let Some(w) = it.str("wallet")? {
        rec.push(field::SHARE_WALLET, w.as_bytes());
    }
    Ok(())
}

fn r_passphrase(it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    let f = it.need_str("for")?;
    let f = unhex(&f, Some(4)).ok_or_else(|| it.err(Why::BadHex, "for"))?;
    rec.push(field::PASSPHRASE_KEY, &f);
    rec.push(field::PASSPHRASE, it.need_str("text")?.as_bytes());
    Ok(())
}

fn r_codex32_share(it: &mut Item, rec: &mut Record) -> Result<(), Refusal> {
    rec.push(field::CODEX32_STRING, it.need_str("codex32")?.as_bytes());
    let f = it.need_str("for")?;
    let f = unhex(&f, Some(4)).ok_or_else(|| it.err(Why::BadHex, "for"))?;
    rec.push(field::CODEX32_KEY, &f);
    if !rec
        .field(field::CODEX32_STRING)
        .is_some_and(codex32::is_share)
    {
        return Err(it.err(Why::BadCodex32, "codex32"));
    }
    Ok(())
}
