//! The TOML inside a vault (`docs/VAULT.md` §2.1, §7): the QR vault's
//! contents and the three TOML values a file vault keeps in KDBX custom
//! data (F190 points 3 and 5), each read and written as the Spec names
//! them and refused as it says; a file vault's own document is KDBX XML
//! and is not read here.
//!
//! - [`write_qr`] and [`read_qr`] are the QR vault's one item as the keys
//!   of a TOML document, padded to [`QR_LEN`]: a seed's `words`, a
//!   codex32 share's `codex32` and `for`, a passphrase's `passphrase` and
//!   `for`, a `note`, or nothing (§2.1).
//! - [`write_plan`] and [`read_plan`] are a backup plan, the value of a
//!   `faraday.plan.{n}` key: a TOML document of the `plan` keys
//!   (`wallet`, `answers`, `places`, `holds`, `marks`), as a plan
//!   record (§7).
//! - [`write_codex32_facts`] and [`read_codex32_facts`] are a seed's
//!   `faraday.codex32`, and [`write_child_facts`] and [`read_child_facts`]
//!   its `faraday.child`, each one TOML inline table of plain facts (§7).
//!
//! A syntax refusal names the line ([`Why::Syntax`]); every other
//! refusal names the key ([`Refusal`]). Anything not named in §7 is
//! refused ([`Why::Unknown`]).
//!
//! Every string is kept in a buffer that is wiped when it drops, and so is
//! every key of the document; what `toml_parser` keeps is wiped when this
//! reader is done with it, and what the parser of words keeps is not
//! (`docs/deps/toml_parser.md`).
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

mod tree;

use alloc::vec::Vec;

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use zeroize::Zeroizing;

use crate::child::ChildOf;
use crate::codex32::{self, SetFacts, State};
use crate::layouts::{self, Opened};
use crate::records::{Record, field, kind};
use tree::{Arr, Table, Val};

/// The most a TOML value is, in bytes: a backup plan is the longest.
pub const MAX_VALUE: usize = 1024 * 1024;

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

/// How many characters, each one a Unicode scalar value (what Backspace
/// takes), must come off the end of `note` for its TOML text to fit
/// [`QR_NOTE_MAX`]: 0 for a note that fits. The text is counted as
/// [`write_qr`] writes it, with the same escapes: a quote, a backslash or
/// a line break takes two bytes, a character outside ASCII its UTF-8
/// length.
pub fn qr_note_over(note: &str) -> usize {
    let mut o = Out {
        bytes: Zeroizing::new(Vec::with_capacity(note.len().saturating_mul(6))),
        limit: usize::MAX,
        full: false,
    };
    // The text's length after each character, from the start.
    let mut ends: Vec<usize> = Vec::with_capacity(note.len());
    for ch in note.chars() {
        o.escaped(ch);
        ends.push(o.bytes.len());
    }
    // The characters that stay are the most whose text fits.
    ends.len() - ends.partition_point(|&n| n <= QR_NOTE_MAX)
}

/// Why a value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// Not TOML v1.0.0, at this line (from 1).
    Syntax(usize),
    /// Not UTF-8.
    NotUtf8,
    /// Longer than the value or the QR vault allows.
    TooLong,
    /// A key §7 does not name.
    Unknown,
    /// A value of the wrong type.
    WrongType,
    /// A required key is missing.
    Missing,
    /// Two keys of which one is allowed.
    Exclusive,
    /// Words that are not a valid BIP-39 phrase of the English list.
    BadWords,
    /// Hex that is not lower case, or not of its length.
    BadHex,
    /// A codex32 set's facts, or a codex32 string, that are not a set or
    /// share this layout holds.
    BadCodex32,
    /// BIP-85 facts that are not child facts this layout holds.
    BadChild,
    /// A state or another value that is not one.
    BadValue,
    /// A number outside its range.
    Range,
    /// An item a QR vault does not hold, or a QR vault of the wrong
    /// length.
    NotQr,
}

/// A refusal: what, and at which key (empty when it is the whole value;
/// a syntax refusal carries its line in [`Why::Syntax`]).
#[derive(Debug, PartialEq, Eq)]
pub struct Refusal {
    /// What was wrong.
    pub why: Why,
    /// The key, as the value or the Spec names it; a key of a table
    /// inside is written `table.key`.
    pub key: KeyName,
}

#[expect(
    clippy::disallowed_methods,
    reason = "a public value: its fields hold no secret"
)]
impl Clone for Refusal {
    fn clone(&self) -> Self {
        Self {
            why: self.why,
            key: self.key.clone(),
        }
    }
}

/// The name of the key a [`Refusal`] is at: a prefix, a dot where there is
/// a prefix, and the first 64 characters of the key, kept as UTF-8 bytes in
/// a buffer that is wiped when it drops.
pub struct KeyName(Zeroizing<Vec<u8>>);

#[expect(
    clippy::disallowed_methods,
    reason = "a public value: its fields hold no secret"
)]
impl Clone for KeyName {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl KeyName {
    /// `prefix`, a dot where there is a prefix, and the first 64
    /// characters of `key`; the room for all of it is reserved first, so
    /// the buffer never grows.
    fn of(prefix: &str, key: &str) -> KeyName {
        let room = prefix.len() + 1 + key.len().min(64 * 4);
        let mut name = Zeroizing::new(Vec::with_capacity(room));
        if !prefix.is_empty() {
            name.extend_from_slice(prefix.as_bytes());
            name.push(b'.');
        }
        for ch in key.chars().take(64) {
            let mut b = [0u8; 4];
            name.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
        }
        KeyName(name)
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        // Every byte was copied from a `&str` or a `char`, so this is
        // always UTF-8.
        core::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl PartialEq for KeyName {
    fn eq(&self, other: &KeyName) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for KeyName {}

impl From<&str> for KeyName {
    fn from(key: &str) -> KeyName {
        KeyName::of("", key)
    }
}

impl core::fmt::Debug for KeyName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}

fn refuse(why: Why) -> Refusal {
    Refusal {
        why,
        key: KeyName::from(""),
    }
}

fn at(key: &str, why: Why) -> Refusal {
    Refusal {
        why,
        key: KeyName::from(key),
    }
}

const STATES: [(State, &str); 5] = [
    (State::Made, "made"),
    (State::Placed, "placed"),
    (State::Lost, "lost"),
    (State::Exposed, "exposed"),
    (State::Destroyed, "destroyed"),
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

    /// `bytes` as lower-case hex, written straight into the document's
    /// reserved buffer, so no other copy is made.
    fn hex(&mut self, bytes: &[u8]) {
        if self.bytes.len() + 2 * bytes.len() > self.limit {
            self.full = true;
            return;
        }
        for &b in bytes {
            self.bytes.push(nibble(b >> 4));
            self.bytes.push(nibble(b & 15));
        }
    }

    /// A basic string: `"` and `\` escaped, control characters as escapes
    /// and everything else as written.
    fn text(&mut self, s: &str) {
        self.raw(b"\"");
        for ch in s.chars() {
            self.escaped(ch);
        }
        self.raw(b"\"");
    }

    /// One character of a basic string, as [`Out::text`] writes it.
    fn escaped(&mut self, ch: char) {
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

    /// The bytes written, or [`Why::TooLong`] at `key` where they did not
    /// fit.
    fn done(self, key: &str) -> Result<Zeroizing<Vec<u8>>, Refusal> {
        if self.full {
            Err(at(key, Why::TooLong))
        } else {
            Ok(self.bytes)
        }
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

/// The lower-case hex digit of `n` (0 to 15), with no branch and no table
/// indexed by it: 39 is added for the digits past 9, as the high bit of
/// `9 - n` says.
fn nibble(n: u8) -> u8 {
    b'0' + n + 39 * (9u8.wrapping_sub(n) >> 7)
}

fn text_of(r: &Record, n: u8) -> &str {
    r.text(n).unwrap_or("")
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

// ---------------------------------------------------------------- reading

/// A table being taken apart: each key read is removed, and `finish`
/// refuses what is left.
struct Item {
    t: Table,
    prefix: &'static str,
}

impl Item {
    fn err(&self, why: Why, key: &str) -> Refusal {
        Refusal {
            why,
            key: KeyName::of(self.prefix, key),
        }
    }

    fn val(&mut self, key: &str) -> Option<Val> {
        self.t.take(key)
    }

    fn str(&mut self, key: &str) -> Result<Option<SecretText>, Refusal> {
        match self.val(key) {
            None => Ok(None),
            Some(Val::Str(s)) => Ok(Some(s)),
            Some(_) => Err(self.err(Why::WrongType, key)),
        }
    }

    fn need_str(&mut self, key: &str) -> Result<SecretText, Refusal> {
        self.str(key)?.ok_or_else(|| self.err(Why::Missing, key))
    }

    fn need_int(&mut self, key: &str) -> Result<i64, Refusal> {
        match self.val(key) {
            None => Err(self.err(Why::Missing, key)),
            Some(Val::Int(n)) => Ok(n),
            Some(_) => Err(self.err(Why::WrongType, key)),
        }
    }

    fn small(&mut self, key: &str, max: u32) -> Result<Option<u32>, Refusal> {
        match self.val(key) {
            None => Ok(None),
            Some(Val::Int(n)) => u32::try_from(n)
                .ok()
                .filter(|&n| n <= max)
                .map(Some)
                .ok_or_else(|| self.err(Why::Range, key)),
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

    fn strs(&mut self, key: &str) -> Result<Vec<SecretText>, Refusal> {
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

    fn finish(self) -> Result<(), Refusal> {
        match self.t.entries.first() {
            Some((k, _)) => Err(self.err(Why::Unknown, k.text())),
            None => Ok(()),
        }
    }
}

fn unhex(s: &str, len: Option<usize>) -> Option<Zeroizing<Vec<u8>>> {
    // Lower case only: the vault writes no other.
    if s.bytes().any(|c| c.is_ascii_uppercase()) || len.is_some_and(|n| n * 2 != s.len()) {
        return None;
    }
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(alloc::vec![0u8; s.len() / 2]);
    (faraday_crypto::hex::decode_into(s, &mut out)? == out.len()).then_some(out)
}

fn parse_tree(bytes: &[u8], max: usize) -> Result<Table, Refusal> {
    if bytes.len() > max {
        return Err(refuse(Why::TooLong));
    }
    let text = core::str::from_utf8(bytes).map_err(|_| refuse(Why::NotUtf8))?;
    tree::parse(text).map_err(|line| refuse(Why::Syntax(line)))
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
    record.check().map_err(|_| refuse(Why::BadValue))?;
    match record.kind {
        kind::KEY => {
            let payload = record.field(field::KEY).unwrap_or(&[]);
            let Some(Opened::Words(m)) = layouts::payload_of(layouts::KIND_WORDS, payload) else {
                return Err(at("words", Why::NotQr));
            };
            if record.fields.len() != 1
                || m.language() != Language::English
                || !matches!(m.word_count(), 12 | 24)
            {
                return Err(at("words", Why::NotQr));
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
                return Err(at("note", Why::NotQr));
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
        return Err(at(key, Why::TooLong));
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
/// [`QR_LEN`] bytes holding one item as top-level keys (§2.1): `words`
/// of 12 or 24 English words, `codex32` and `for`, `passphrase` and
/// `for`, `note`, or nothing (an empty note).
pub fn read_qr(c: &Compute, bytes: &[u8]) -> Result<Record, Refusal> {
    if bytes.len() != QR_LEN {
        return Err(refuse(Why::NotQr));
    }
    let root = parse_tree(bytes, QR_LEN)?;
    let has = |k: &str| root.entries.iter().any(|(n, _)| n.is(k));
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
        .find(|(n, _)| !allowed.contains(&n.text()))
    {
        return Err(at(n.text(), Why::Unknown));
    }
    let key = if has("words") {
        "words"
    } else if has("codex32") {
        "codex32"
    } else if has("note") {
        "note"
    } else if has("passphrase") {
        "passphrase"
    } else {
        return Err(refuse(Why::NotQr));
    };
    let mut it = Item {
        t: root,
        prefix: "",
    };
    let mut rec;
    match key {
        "words" => {
            let w = it.need_str("words")?;
            let m = Mnemonic::parse(c, Language::English, &w)
                .map_err(|_| it.err(Why::BadWords, "words"))?;
            if !matches!(m.word_count(), 12 | 24) {
                return Err(it.err(Why::NotQr, "words"));
            }
            rec = Record::new(kind::KEY);
            rec.push(field::KEY, &layouts::words_payload(&m));
        }
        "codex32" => {
            rec = Record::new(kind::CODEX32_SHARE);
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
        }
        "note" => {
            rec = Record::new(kind::NOTE);
            rec.push(field::NOTE, it.need_str("note")?.as_bytes());
        }
        _ => {
            rec = Record::new(kind::PASSPHRASE);
            let f = it.need_str("for")?;
            let f = unhex(&f, Some(4)).ok_or_else(|| it.err(Why::BadHex, "for"))?;
            rec.push(field::PASSPHRASE_KEY, &f);
            rec.push(field::PASSPHRASE, it.need_str("passphrase")?.as_bytes());
        }
    }
    it.finish()?;
    rec.check().map_err(|_| refuse(Why::BadValue))?;
    // A form that is shorter written than `write_qr` writes it (`note="…"`
    // without spaces, a literal string with no escapes) can hold more than
    // the limits: what `write_qr` would refuse as too long is refused here.
    if matches!(rec.kind, kind::NOTE | kind::PASSPHRASE)
        && write_qr(&rec).is_err_and(|r| r.why == Why::TooLong)
    {
        return Err(at(key, Why::TooLong));
    }
    Ok(rec)
}

// ------------------------------------------------------------ custom data

/// A backup plan record as a TOML document of its keys (§7,
/// `faraday.plan.{n}`): `wallet`, then `answers`, `places`, `holds` and
/// `marks` where it has them.
pub fn write_plan(record: &Record) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    if record.kind != kind::PLAN {
        return Err(refuse(Why::Unknown));
    }
    record.check().map_err(|_| refuse(Why::BadValue))?;
    let mut o = Out::new(MAX_VALUE)?;
    o.line("wallet", |o| o.text(text_of(record, field::PLAN_WALLET)));
    if let Some(a) = record.text(field::PLAN_ANSWERS) {
        let lines: Vec<&str> = a.split('\n').collect();
        o.line("answers", |o| o.strs(&lines));
    }
    for (key, n) in [
        ("places", field::PLAN_PLACE),
        ("holds", field::PLAN_HOLDS),
        ("marks", field::PLAN_NOTE),
    ] {
        if record.field(n).is_some() {
            let items: Vec<&str> = record
                .fields
                .iter()
                .filter(|f| f.number == n)
                .map(|f| core::str::from_utf8(&f.bytes).unwrap_or(""))
                .collect();
            o.line(key, |o| o.strs(&items));
        }
    }
    o.done("plan")
}

/// Reads a `faraday.plan.{n}` value (§7): a TOML document of the `plan`
/// keys, as a plan record. No other key is read.
pub fn read_plan(bytes: &[u8]) -> Result<Record, Refusal> {
    let mut it = Item {
        t: parse_tree(bytes, MAX_VALUE)?,
        prefix: "",
    };
    let mut rec = Record::new(kind::PLAN);
    rec.push(field::PLAN_WALLET, it.need_str("wallet")?.as_bytes());
    let answers = it.strs("answers")?;
    if !answers.is_empty() {
        let mut joined = SecretText::new();
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
    it.finish()?;
    rec.check().map_err(|_| refuse(Why::BadValue))?;
    // A plan that `write_plan` would refuse as too long (backslashes in
    // literal strings, line breaks, tabs or many short items take more
    // written than read) is refused here, so no vault opens that cannot be
    // sealed again.
    if write_plan(&rec).is_err_and(|r| r.why == Why::TooLong) {
        return Err(at("plan", Why::TooLong));
    }
    Ok(rec)
}

/// A seed's codex32 facts as one TOML inline table, the value of its
/// `faraday.codex32` (§7): `k`, `id`, `lengths`, `unknown`, `shares`.
pub fn write_codex32_facts(f: &SetFacts) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    if !f.is_sound() {
        return Err(refuse(Why::BadCodex32));
    }
    let mut o = Out::new(4096)?;
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
    o.done("codex32")
}

/// Reads a `faraday.codex32` value (§7): the inline table of a set's
/// facts. `lengths` is read when every entry is equal.
pub fn read_codex32_facts(bytes: &[u8]) -> Result<SetFacts, Refusal> {
    let mut v = value_table(bytes, 4096)?;
    let k = v.small("k", 9)?.ok_or_else(|| v.err(Why::Missing, "k"))?;
    let id = v.need_str("id")?;
    let identifier: [u8; 4] = id
        .as_bytes()
        .try_into()
        .map_err(|_| v.err(Why::BadCodex32, "id"))?;
    let lengths = v
        .array("lengths")?
        .ok_or_else(|| v.err(Why::Missing, "lengths"))?;
    let mut length = None;
    for e in lengths.items {
        let Val::Int(n) = e else {
            return Err(v.err(Why::WrongType, "lengths"));
        };
        let n = u8::try_from(n).map_err(|_| v.err(Why::Range, "lengths"))?;
        if length.is_some_and(|l| l != n) {
            return Err(v.err(Why::BadCodex32, "lengths"));
        }
        length = Some(n);
    }
    let length = length.ok_or_else(|| v.err(Why::BadCodex32, "lengths"))?;
    let unknown = v
        .small("unknown", 255)?
        .ok_or_else(|| v.err(Why::Missing, "unknown"))?;
    let mut strings = Vec::new();
    if let Some(shares) = v.array("shares")? {
        for e in shares.items {
            let Val::Table(tab) = e else {
                return Err(v.err(Why::WrongType, "shares"));
            };
            let mut s = Item {
                t: tab,
                prefix: "shares",
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
    v.finish()?;
    let facts = SetFacts {
        threshold: k as u8,
        identifier,
        length,
        unknown: unknown as u8,
        strings,
    };
    if !facts.is_sound() {
        return Err(refuse(Why::BadCodex32));
    }
    Ok(facts)
}

/// A seed's BIP-85 facts as one TOML inline table, the value of its
/// `faraday.child` (§7): `parent`, `application` 39, `language`,
/// `words`, `index`.
pub fn write_child_facts(c: &ChildOf) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    if !c.is_sound() {
        return Err(refuse(Why::BadChild));
    }
    let mut o = Out::new(256)?;
    o.s("{parent = \"");
    o.hex(&c.parent);
    o.s("\", application = 39, language = ");
    o.num(u64::from(c.language));
    o.s(", words = ");
    o.num(u64::from(c.words));
    o.s(", index = ");
    o.num(u64::from(c.index));
    o.s("}");
    o.done("child")
}

/// Reads a `faraday.child` value (§7): the inline table of a BIP-85
/// origin.
pub fn read_child_facts(bytes: &[u8]) -> Result<ChildOf, Refusal> {
    let mut t = value_table(bytes, 256)?;
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
    t.finish()?;
    let c = ChildOf {
        parent: [parent[0], parent[1], parent[2], parent[3]],
        language: language as u8,
        words: words as u8,
        index,
    };
    if !c.is_sound() {
        return Err(refuse(Why::BadChild));
    }
    Ok(c)
}

/// The inline table a value is: read as the one key `v` of a document, so
/// that anything after the table is a second key and is refused.
fn value_table(bytes: &[u8], max: usize) -> Result<Item, Refusal> {
    if bytes.len() > max {
        return Err(refuse(Why::TooLong));
    }
    let text = core::str::from_utf8(bytes).map_err(|_| refuse(Why::NotUtf8))?;
    let mut doc = SecretText::new();
    doc.push_str("v = ");
    doc.push_str(text);
    let mut root = tree::parse(&doc).map_err(|line| refuse(Why::Syntax(line)))?;
    let v = root.take("v");
    if !root.entries.is_empty() {
        return Err(refuse(Why::Unknown));
    }
    match v {
        Some(Val::Table(t)) => Ok(Item { t, prefix: "" }),
        _ => Err(refuse(Why::WrongType)),
    }
}
