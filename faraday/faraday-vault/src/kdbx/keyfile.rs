//! A KeePass key file and a YubiKey's secret, read into the bytes the
//! composite key takes (`docs/VAULT.md` §3.2, F208).
//!
//! A key file is read in KeePass's order: XML version 1 or 2, a file of
//! exactly 32 bytes, 64 hexadecimal characters, else the file's SHA-256.
//! KeePass, KeePassXC and pykeepass agree on that order and differ at its
//! edges; where they differ this reader refuses, so a file means one thing
//! here or nothing:
//!
//! - an XML key that is not 32 bytes (KeePass uses it as it is, KeePassXC
//!   fits it to 32): refused;
//! - a version 2 file with no `Hash` (KeePass takes it, KeePassXC and
//!   pykeepass do not), or with one that fails: refused;
//! - a `KeyFile` document with no key, or no version, or another one (KeePass
//!   refuses it, the other two hash the file): refused;
//! - an empty file (KeePass hashes it, KeePassXC refuses it): refused;
//! - a second `Version`, `Data` or `Hash`, which the three read differently:
//!   refused.
//!
//! - a `KeyFile` document with a DOCTYPE, anything after its root but white
//!   space and a well formed comment or processing instruction (an element,
//!   text, a reference, CDATA, a bad comment or declaration, a byte that is
//!   not valid UTF-8 cut short at the file's end, or a character XML does
//!   not allow), a processing instruction standing alone whose target one
//!   reader takes and the other refuses (a target that is not an XML name,
//!   has no ASCII punctuation but `_`, `-` and `.` and does not start with a
//!   digit, `-` or `.`: a control character or DEL, U+00B7, U+00D7, U+00F7,
//!   `a` then U+00F7, U+0300, U+2000), markup inside the
//!   `Version` or `Data`, a declared version of `1.` and digits other than
//!   `1.0` (libxml2 reads it, Qt does not), a declared encoding other than
//!   UTF-8 in a `KeyFile` document with a byte at or above 0x80 (the
//!   encodings both readers take are taken for ASCII content only; in such
//!   a file a processing instruction target with such a byte is refused too,
//!   unless its ASCII characters already make both readers refuse it), a
//!   version KeePassXC does not read (it takes exactly `1.0`, `1.00` and
//!   `2.0`), nesting
//!   deeper than the vault's reader allows, or a UTF-16 byte order mark
//!   (KeePassXC reads those as XML, a strict reader does not): refused.
//!
//! A document that is not well formed (an unquoted or repeated attribute on
//! any element, a malformed XML declaration, `--` inside a comment, a
//! processing instruction whose target is not an XML name (refused after the
//! root), a reference that is not XML's five or a character, a byte that is
//! not valid UTF-8 wherever it is or a character XML does not allow before or
//! inside the root, a second byte order mark) or has another
//! root is not a `KeyFile` document and is hashed as any other file is,
//! except a `KeyFile` root under a declared encoding other than UTF-8 in a
//! file with a byte at or above 0x80, which is refused.

use alloc::vec::Vec;

use base64ct::{Base64, Encoding};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::xml::{MAX_DEPTH, valid_ref};

/// Why a key file or a YubiKey secret was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFileError {
    /// The file is empty.
    Empty,
    /// The file is a KDBX database, which is not a key file.
    Database,
    /// A `KeyFile` document that holds no key Faraday takes as stated.
    Damaged,
}

/// A secret that is not 40 hexadecimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretError {
    /// How many hexadecimal digits were typed.
    pub digits: usize,
    /// Whether anything but a digit, a space or a colon was typed.
    pub stray: bool,
}

/// The first four bytes of every KDBX file.
const KDBX_SIGNATURE: [u8; 4] = [0x03, 0xD9, 0xA2, 0x9A];

/// Costs a SHA-256 of the file. The 32-byte key of the key file `bytes`
/// (§3.2). The caller wipes `bytes`.
pub fn read(bytes: &[u8]) -> Result<Zeroizing<[u8; 32]>, KeyFileError> {
    if bytes.is_empty() {
        return Err(KeyFileError::Empty);
    }
    if bytes.starts_with(&KDBX_SIGNATURE) {
        return Err(KeyFileError::Database);
    }
    if let Some(key) = xml(bytes)? {
        return Ok(key);
    }
    if let Ok(key) = <[u8; 32]>::try_from(bytes) {
        return Ok(Zeroizing::new(key));
    }
    if bytes.len() == 64
        && let Some(key) = hex_key(bytes)
    {
        return Ok(key);
    }
    Ok(Zeroizing::new(Sha256::digest(bytes).into()))
}

/// The key file `key` is kept in: KeePass's XML key file version 2.0, byte
/// for byte as KeePassXC 2.7.10 writes it (a `.keyx`), so that the
/// KeePass family opens a vault with it. The caller draws `key` and wipes
/// the result.
pub fn write(key: &[u8; 32]) -> Zeroizing<Vec<u8>> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let hash = Sha256::digest(key);
    let mut out = Zeroizing::new(Vec::with_capacity(320));
    out.extend_from_slice(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<KeyFile>\n    <Meta>\n        \
          <Version>2.0</Version>\n    </Meta>\n    <Key>\n        <Data Hash=\"",
    );
    for b in hash.iter().take(4) {
        out.push(HEX[usize::from(b >> 4)]);
        out.push(HEX[usize::from(b & 15)]);
    }
    out.extend_from_slice(b"\">");
    for (i, b) in key.iter().enumerate() {
        match i {
            0 => {}
            _ if i % 16 == 0 => out.push(b'\n'),
            _ if i % 4 == 0 => out.push(b' '),
            _ => {}
        }
        if i % 16 == 0 {
            if i == 0 {
                out.push(b'\n');
            }
            out.extend_from_slice(b"            ");
        }
        out.push(HEX[usize::from(b >> 4)]);
        out.push(HEX[usize::from(b & 15)]);
    }
    out.extend_from_slice(b"\n        </Data>\n    </Key>\n</KeyFile>\n");
    out
}

/// The 20-byte secret `text` spells: 40 hexadecimal digits in either case,
/// with spaces and colons between them ignored.
pub fn secret(text: &str) -> Result<Zeroizing<[u8; 20]>, SecretError> {
    let mut out = Zeroizing::new([0u8; 20]);
    let mut digits = 0usize;
    let mut stray = false;
    for b in text.bytes() {
        match b {
            b' ' | b':' => {}
            _ => match nibble(b) {
                Some(n) => {
                    if let Some(slot) = out.get_mut(digits / 2) {
                        *slot = (*slot << 4) | n;
                    }
                    digits += 1;
                }
                None => stray = true,
            },
        }
    }
    if digits == 40 && !stray {
        Ok(out)
    } else {
        Err(SecretError { digits, stray })
    }
}

fn nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// The 32 bytes that `digits` (64 of them) spell, or none if any is not a
/// hexadecimal digit.
fn hex_key(digits: &[u8]) -> Option<Zeroizing<[u8; 32]>> {
    let mut out = Zeroizing::new([0u8; 32]);
    for (slot, pair) in out.iter_mut().zip(digits.chunks_exact(2)) {
        *slot = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(out)
}

fn is_ws(b: &u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

/// What a `KeyFile` document states.
#[derive(Default)]
struct Found {
    version: Option<Vec<u8>>,
    data: Option<Zeroizing<Vec<u8>>>,
    hash: Option<Vec<u8>>,
    /// Something was stated twice.
    twice: bool,
}

/// The key of a `KeyFile` XML document: none if `bytes` is not one (not
/// well formed, another root, a DOCTYPE), refused if it is one and does
/// not state a key as §3.2 requires.
fn xml(bytes: &[u8]) -> Result<Option<Zeroizing<[u8; 32]>>, KeyFileError> {
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        // KeePassXC reads UTF-16 as XML, a strict reader does not: a
        // `KeyFile` document in it is refused, any other file is hashed.
        let le = bytes[0] == 0xFF;
        let units = bytes[2..].chunks_exact(2).map(|p| {
            if le {
                u16::from_le_bytes([p[0], p[1]])
            } else {
                u16::from_be_bytes([p[0], p[1]])
            }
        });
        let mut text = Zeroizing::new(Vec::new());
        for c in core::char::decode_utf16(units) {
            let Ok(c) = c else { return Ok(None) };
            let mut buf = [0u8; 4];
            text.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
        return match scan(&text)? {
            Some(_) => Err(KeyFileError::Damaged),
            None => Ok(None),
        };
    }
    match scan(bytes)? {
        Some(found) => key_of(found).map(Some),
        None => Ok(None),
    }
}

/// The element whose text is wanted: the depth it sits at, whether it is the
/// Version, and the text so far.
type Wanted = (usize, bool, Zeroizing<Vec<u8>>);

/// The text a started element asks for, if it is the Version or the Data:
/// the depth it sits at, whether it is the Version, and the text so far.
fn start_text(
    e: &BytesStart<'_>,
    path: &[Vec<u8>],
    found: &mut Found,
) -> Result<Option<Wanted>, KeyFileError> {
    let Some(is_version) = wanted(path, e.name().as_ref()) else {
        return Ok(None);
    };
    if !is_version {
        take_hash(e, found)?;
    }
    Ok(Some((path.len(), is_version, Zeroizing::new(Vec::new()))))
}

/// An element with no text that sits inside the `KeyFile`: an empty Version
/// or Data states nothing.
fn empty_element(
    e: &BytesStart<'_>,
    path: &[Vec<u8>],
    found: &mut Found,
) -> Result<(), KeyFileError> {
    match wanted(path, e.name().as_ref()) {
        Some(true) => set_version(found, Vec::new()),
        Some(false) => {
            take_hash(e, found)?;
            set_data(found, Zeroizing::new(Vec::new()));
        }
        None => {}
    }
    Ok(())
}

/// The text of the Version or Data element that has closed, if `text` is
/// one at `depth`.
fn close_text(found: &mut Found, text: &mut Option<Wanted>, depth: usize) {
    if text.as_ref().is_some_and(|(d, _, _)| *d == depth)
        && let Some((_, is_version, buf)) = text.take()
    {
        if is_version {
            set_version(found, buf.to_vec());
        } else {
            set_data(found, buf);
        }
    }
}

/// What a declaration, comment or processing instruction makes of the
/// document, as the two readers (KeePassXC's and pykeepass's) read it.
#[derive(PartialEq)]
enum Markup {
    /// Both read it as XML.
    Fine,
    /// Both hash the file.
    Hash,
    /// One reads the file as XML, the other hashes it.
    Differ,
}

/// Whether a declaration, comment or processing instruction is well formed
/// as a strict reader takes it: the declaration first, a comment with no
/// `--` in it and not ending in `-`, a processing instruction with a target
/// that is not `xml` in any case.
fn markup(event: &Event<'_>, at_start: bool, utf8: bool) -> Markup {
    match event {
        Event::Decl(d) if at_start => declaration(d),
        Event::Decl(_) => Markup::Hash,
        Event::Comment(c) if c.windows(2).any(|w| w == b"--") || c.ends_with(b"-") => Markup::Hash,
        Event::PI(p) if p.target().eq_ignore_ascii_case(b"xml") => Markup::Hash,
        // Under another encoding each reader reads a byte at or above 0x80
        // as a different character, so one takes the target and one refuses.
        // A target both readers refuse for its ASCII characters is hashed.
        Event::PI(p) if !utf8 && p.target().iter().any(|&b| b >= 0x80) => {
            if ascii_refused(p.target()) {
                Markup::Hash
            } else {
                Markup::Differ
            }
        }
        Event::PI(p) if !is_name(p.target()) => bad_target(p.target(), p.content().is_empty()),
        _ => Markup::Fine,
    }
}

/// Whether `c` may start an XML name (XML 1.0, fifth edition).
fn name_start(c: char) -> bool {
    matches!(c,
        'A'..='Z' | '_' | 'a'..='z' | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}'
        | '\u{F8}'..='\u{2FF}' | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}' | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}' | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}')
}

/// Whether `c` may continue an XML name.
fn name_char(c: char) -> bool {
    name_start(c)
        || matches!(c, '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}'
            | '\u{203F}'..='\u{2040}')
}

/// Whether `target` is an XML name: a processing instruction's target must
/// be, and a strict reader refuses the document when it is not.
fn is_name(target: &[u8]) -> bool {
    let Ok(text) = core::str::from_utf8(target) else {
        return false;
    };
    let mut chars = text.chars();
    chars.next().is_some_and(name_start) && chars.all(name_char)
}

/// Whether a target is refused by both readers whatever its bytes at or
/// above 0x80 are read as: it is empty, starts with a digit, `-` or `.`, or
/// has ASCII punctuation but `_`, `-` and `.`.
fn ascii_refused(target: &[u8]) -> bool {
    target.is_empty()
        || target[0].is_ascii_digit()
        || matches!(target[0], b'-' | b'.')
        || target
            .iter()
            .any(|b| b.is_ascii_punctuation() && !matches!(b, b'_' | b'-' | b'.'))
}

/// What a processing instruction target that is not an XML name makes of
/// the document. libxml2 refuses it. Qt refuses one with an ASCII
/// punctuation mark or one that starts with a digit, `-` or `.`, and takes
/// any other when the target stands alone (`<?p\u{1}?>`, `<?\u{D7}?>`).
fn bad_target(target: &[u8], alone: bool) -> Markup {
    if !alone || ascii_refused(target) {
        Markup::Hash
    } else {
        Markup::Differ
    }
}

/// The encodings a declaration may name: both readers take the document as
/// XML with these. UTF-16 and UTF-32 and a name neither knows are hashed by
/// both, as is a name with a space in it.
const ENCODINGS: [&[u8]; 9] = [
    b"utf-8",
    b"utf8",
    b"us-ascii",
    b"ascii",
    b"iso-8859-1",
    b"iso-8859-15",
    b"latin1",
    b"windows-1252",
    b"shift_jis",
];

/// What a declaration's value makes of the document: the version is `1.0`
/// (another `1.` and digits is read by libxml2 and not by Qt), standalone
/// `yes` or `no`, and the encoding one both readers know.
fn declared(name: &[u8], value: &[u8]) -> Markup {
    match name {
        b"version" if value == b"1.0" => Markup::Fine,
        b"version" if value.starts_with(b"1.") && value[2..].iter().all(u8::is_ascii_digit) => {
            Markup::Differ
        }
        b"standalone" if matches!(value, b"yes" | b"no") => Markup::Fine,
        b"encoding" if ENCODINGS.iter().any(|e| e.eq_ignore_ascii_case(value)) => Markup::Fine,
        _ => Markup::Hash,
    }
}

/// What the XML declaration `d` makes of the document: hashed unless it is
/// well formed as a strict reader takes it (`version`, then `encoding` and
/// `standalone` if present, each in quotes) with values both readers accept.
fn declaration(d: &[u8]) -> Markup {
    let Some(mut rest) = d.strip_prefix(b"xml") else {
        return Markup::Hash;
    };
    let mut verdict = Markup::Fine;
    let mut at = 0;
    loop {
        let trimmed = rest.trim_ascii_start();
        if trimmed.is_empty() {
            return if at >= 1 { verdict } else { Markup::Hash };
        }
        if trimmed.len() == rest.len() {
            return Markup::Hash;
        }
        let Some(eq) = trimmed.iter().position(|&b| b == b'=') else {
            return Markup::Hash;
        };
        let name = trimmed[..eq].trim_ascii_end();
        let want: &[&[u8]] = match at {
            0 => &[b"version"],
            1 => &[b"encoding", b"standalone"],
            2 => &[b"standalone"],
            _ => &[],
        };
        let Some(this) = want.iter().position(|w| *w == name) else {
            return Markup::Hash;
        };
        let value = trimmed[eq + 1..].trim_ascii_start();
        let Some(&quote) = value.first().filter(|q| matches!(q, b'"' | b'\'')) else {
            return Markup::Hash;
        };
        let Some(end) = value[1..].iter().position(|&b| b == quote) else {
            return Markup::Hash;
        };
        match declared(name, &value[1..=end]) {
            Markup::Hash => return Markup::Hash,
            Markup::Differ => verdict = Markup::Differ,
            Markup::Fine => {}
        }
        rest = &value[end + 2..];
        // The next may not go back over `encoding` once `standalone` came.
        at = if want.len() == 2 && this == 1 {
            3
        } else {
            at + 1
        };
    }
}

/// The bytes of `src` from offset `from` up to `to`.
fn read_since(src: &[u8], from: u64, to: u64) -> &[u8] {
    usize::try_from(from)
        .ok()
        .zip(usize::try_from(to).ok())
        .and_then(|(from, to)| src.get(from..to))
        .unwrap_or_default()
}

/// Whether `event` is a processing instruction with a target and nothing
/// else: the one place where Qt takes what libxml2 refuses.
fn bare_pi(event: &Event<'_>) -> bool {
    matches!(event, Event::PI(p) if p.content().is_empty())
}

/// The state after the document element opens: whether it is also closed
/// (it is empty), and the path.
fn open_root(empty: bool) -> (bool, Vec<Vec<u8>>) {
    (
        empty,
        if empty {
            Vec::new()
        } else {
            Vec::from([b"KeyFile".to_vec()])
        },
    )
}

/// Whether `e` is the document element, which comes once and is the `KeyFile`.
fn is_root(e: &BytesStart<'_>, seen_root: bool, closed: bool) -> bool {
    !(seen_root || closed) && e.name().as_ref() == b"KeyFile"
}

/// Markup that is not a key file: hashed, but refused if it comes after the
/// document element, which KeePassXC does not read and pykeepass does.
fn not_a_key(closed: bool) -> Result<Option<Found>, KeyFileError> {
    if closed {
        Err(KeyFileError::Damaged)
    } else {
        Ok(None)
    }
}

/// Whether `chunk` holds something XML does not allow: a control character
/// other than tab, line feed and carriage return, and, when the document is
/// UTF-8, a byte that is not valid UTF-8 and U+FFFE and U+FFFF.
fn not_allowed(chunk: &[u8], utf8: bool) -> bool {
    if chunk
        .iter()
        .any(|&b| b < b' ' && !matches!(b, b'\t' | b'\n' | b'\r'))
    {
        return true;
    }
    utf8 && core::str::from_utf8(chunk).map_or(true, |t| t.contains(['\u{FFFE}', '\u{FFFF}']))
}

/// Whether the declaration at the start of `src` names an encoding that is
/// not UTF-8: the other names both readers take are read as that encoding
/// only when the file is ASCII.
fn declares_other_encoding(src: &[u8]) -> bool {
    let Some(d) = src.strip_prefix(b"<?xml") else {
        return false;
    };
    let d = d.split(|&b| b == b'?').next().unwrap_or(d);
    let Some(at) = d.windows(8).position(|w| w == b"encoding") else {
        return false;
    };
    let value = d[at + 8..].trim_ascii_start();
    let Some(value) = value.strip_prefix(b"=") else {
        return false;
    };
    let value = value.trim_ascii_start();
    let Some(&quote) = value.first() else {
        return false;
    };
    let name = value[1..].split(|&b| b == quote).next().unwrap_or(&[]);
    !(name.eq_ignore_ascii_case(b"utf-8") || name.eq_ignore_ascii_case(b"utf8"))
}

/// What the `KeyFile` document `bytes` states, none if it is not one.
///
/// Bytes that are not valid UTF-8 make both readers hash the file. In a file
/// whose declaration names an encoding other than UTF-8 a byte at or above
/// 0x80 is read as another character by each reader, so such a `KeyFile`
/// document is refused, as is a file that reached the `KeyFile` root, since
/// a reader of that encoding may end a section where this one does not; any
/// other file is hashed. A character XML does not
/// allow is dealt with where it is found, by `scan_document`.
fn scan(bytes: &[u8]) -> Result<Option<Found>, KeyFileError> {
    let src = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    // A second byte order mark: both readers hash the file.
    if src.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Ok(None);
    }
    let other = declares_other_encoding(src);
    if !other && core::str::from_utf8(src).is_err_and(|e| e.error_len().is_some()) {
        return Ok(None);
    }
    let mut seen_root = false;
    let found = scan_document(src, !other, &mut seen_root)?;
    // Once the root is seen the document may be a `KeyFile` one: under
    // another encoding what follows is read differently by each reader, so
    // no hashing from here on.
    if other && (found.is_some() || seen_root) && src.iter().any(|&b| b >= 0x80) {
        return Err(KeyFileError::Damaged);
    }
    Ok(found)
}

/// What the `KeyFile` document `src` states, none if it is not one. Something
/// XML does not allow in what was read makes both readers hash the file; after
/// the root KeePassXC does not read that far and pykeepass does, so that is
/// refused.
fn scan_document(
    src: &[u8],
    utf8: bool,
    root_seen: &mut bool,
) -> Result<Option<Found>, KeyFileError> {
    let mut reader = Reader::from_reader(src);
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut seen_root = false;
    let mut closed = false;
    let mut found = Found::default();
    // The element whose text is wanted, as `Wanted` says.
    let mut text: Option<Wanted> = None;
    let mut stray_ref = false;
    // Something KeePassXC reads as XML and a strict reader does not.
    let mut differ = false;
    // Whether an event has been read: a declaration is only the first.
    let mut started = false;
    loop {
        let from = reader.buffer_position();
        let event = match reader.read_event() {
            Ok(event) => event,
            Err(_) => return not_a_key(closed),
        };
        if not_allowed(read_since(src, from, reader.buffer_position()), utf8) && !bare_pi(&event) {
            return not_a_key(closed);
        }
        let at_start = !core::mem::replace(&mut started, true);
        match event {
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) => {
                let made = markup(&event, at_start, utf8);
                if made == Markup::Hash {
                    return not_a_key(closed);
                }
                differ |= made == Markup::Differ;
            }
            Event::DocType(_) => differ = true,
            Event::Eof => break,
            Event::Start(ref e) | Event::Empty(ref e) if path.is_empty() => {
                // The document element: the only one, and the KeyFile.
                if !is_root(e, seen_root, closed) || !attributes_ok(e) {
                    return not_a_key(closed);
                }
                seen_root = true;
                *root_seen = true;
                (closed, path) = open_root(matches!(event, Event::Empty(_)));
            }
            Event::Start(e) => {
                if path.len() >= MAX_DEPTH {
                    return Err(KeyFileError::Damaged);
                }
                if !attributes_ok(&e) {
                    return Ok(None);
                }
                if text.is_some() {
                    return Err(KeyFileError::Damaged);
                }
                text = start_text(&e, &path, &mut found)?;
                path.push(e.name().as_ref().to_vec());
            }
            Event::Empty(e) => {
                if !attributes_ok(&e) {
                    return Ok(None);
                }
                if text.is_some() {
                    return Err(KeyFileError::Damaged);
                }
                empty_element(&e, &path, &mut found)?;
            }
            Event::Text(t) => {
                if let Some((_, _, buf)) = text.as_mut() {
                    buf.extend_from_slice(&t);
                } else if path.is_empty() && !t.iter().all(is_ws) {
                    return not_a_key(closed);
                }
            }
            Event::CData(t) => {
                if let Some((_, _, buf)) = text.as_mut() {
                    buf.extend_from_slice(&t);
                } else if path.is_empty() {
                    if !seen_root {
                        return Ok(None);
                    }
                    differ = true;
                }
            }
            Event::GeneralRef(r) => {
                if path.is_empty() {
                    return not_a_key(closed);
                }
                if !valid_ref(&r) {
                    return Ok(None);
                }
                if text.is_some() {
                    stray_ref = true;
                }
            }
            Event::End(_) => {
                if path.pop().is_none() {
                    return not_a_key(closed);
                }
                close_text(&mut found, &mut text, path.len());
                closed = path.is_empty();
            }
        }
    }
    if !seen_root || !path.is_empty() {
        return Ok(None);
    }
    finish(found, differ, stray_ref).map(Some)
}

/// What a closed document states: damaged if something in it is read
/// differently by KeePassXC, and read twice if a reference sat in text that
/// is wanted.
fn finish(mut found: Found, differ: bool, stray_ref: bool) -> Result<Found, KeyFileError> {
    if differ {
        return Err(KeyFileError::Damaged);
    }
    if stray_ref {
        found.twice = true;
    }
    Ok(found)
}

/// The key a `KeyFile` document states, as §3.2 requires it.
fn key_of(found: Found) -> Result<Zeroizing<[u8; 32]>, KeyFileError> {
    if found.twice {
        return Err(KeyFileError::Damaged);
    }
    let version = found.version.ok_or(KeyFileError::Damaged)?;
    let data = found.data.ok_or(KeyFileError::Damaged)?;
    let digits: Zeroizing<Vec<u8>> =
        Zeroizing::new(data.iter().copied().filter(|b| !is_ws(b)).collect());
    // Exactly what KeePassXC reads: it opens with neither ` 2.0 ` nor `2.00`.
    match version.as_slice() {
        b"1.0" | b"1.00" => {
            let mut key = Zeroizing::new([0u8; 32]);
            let mut buf = Zeroizing::new([0u8; 48]);
            let text = core::str::from_utf8(&digits).map_err(|_| KeyFileError::Damaged)?;
            let decoded =
                Base64::decode(text, buf.as_mut_slice()).map_err(|_| KeyFileError::Damaged)?;
            if decoded.len() != 32 {
                return Err(KeyFileError::Damaged);
            }
            key.copy_from_slice(decoded);
            Ok(key)
        }
        b"2.0" => {
            if digits.len() != 64 {
                return Err(KeyFileError::Damaged);
            }
            let key = hex_key(&digits).ok_or(KeyFileError::Damaged)?;
            let hash = found.hash.ok_or(KeyFileError::Damaged)?;
            let hash: Vec<u8> = hash.into_iter().filter(|b| !is_ws(b)).collect();
            let want = Sha256::digest(key.as_slice());
            let stated = hex_key_n(&hash).ok_or(KeyFileError::Damaged)?;
            if stated[..] != want[..4] {
                return Err(KeyFileError::Damaged);
            }
            Ok(key)
        }
        _ => Err(KeyFileError::Damaged),
    }
}

/// Whether the element named `name` under `path` is the Version (`Some(true)`)
/// or the Data (`Some(false)`) of a key file.
fn wanted(path: &[Vec<u8>], name: &[u8]) -> Option<bool> {
    match (path, name) {
        ([root, meta], b"Version") if root == b"KeyFile" && meta == b"Meta" => Some(true),
        ([root, key], b"Data") if root == b"KeyFile" && key == b"Key" => Some(false),
        _ => None,
    }
}

/// The 4 bytes that 8 hexadecimal digits spell.
fn hex_key_n(digits: &[u8]) -> Option<[u8; 4]> {
    if digits.len() != 8 {
        return None;
    }
    let mut out = [0u8; 4];
    for (slot, pair) in out.iter_mut().zip(digits.chunks_exact(2)) {
        *slot = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(out)
}

/// Whether every attribute of `e` is well formed: quoted, none twice.
fn attributes_ok(e: &BytesStart<'_>) -> bool {
    e.attributes().all(|a| a.is_ok())
}

/// The `Hash` attribute of a `Data` element, kept; a second one is noted.
fn take_hash(e: &BytesStart<'_>, found: &mut Found) -> Result<(), KeyFileError> {
    for attr in e.attributes() {
        let Ok(attr) = attr else {
            return Err(KeyFileError::Damaged);
        };
        if attr.key.as_ref() == b"Hash" {
            if found.hash.is_some() {
                found.twice = true;
            }
            found.hash = Some(attr.value.to_vec());
        }
    }
    Ok(())
}

fn set_version(found: &mut Found, v: Vec<u8>) {
    if found.version.is_some() {
        found.twice = true;
    }
    found.version = Some(v);
}

fn set_data(found: &mut Found, v: Zeroizing<Vec<u8>>) {
    if found.data.is_some() {
        found.twice = true;
    }
    found.data = Some(v);
}
