//! paperkey's two forms read back (`docs/GPG-PAPERKEY.md` §2): the base16
//! lines it prints and a person types, each ending in a CRC-24 of its
//! bytes, with a last line holding the CRC-24 of the whole; the raw form
//! with the whole's CRC-24 after it; and the entries the data holds, a
//! fingerprint and a secret part for each key.
//!
//! What paperkey 1.6 reads (`parse.c`): comment lines start `#`, a line's
//! number before its colon must be the next line's, the last six digits
//! of a line are its CRC, and a line with a CRC and no bytes is the
//! whole's. Faraday reads a typed line more freely, as `docs/GPG-PAPERKEY.md`
//! §4.5 has it: spaces anywhere, either case, the number and its colon
//! left out.
//!
//! Every function here takes a paperkey's secret bytes; nothing is kept,
//! and the bytes a line reads to are in memory that is wiped.
//! `tools/lint-secrets.sh` rule 4(c) keeps the callers in faraday-keys.

use alloc::vec::Vec;

use zeroize::Zeroizing;

use crate::certificate::Refusal;
use crate::crc24;

/// The bytes a line holds at paperkey's default width of 78 columns:
/// `(78 − 11) / 3`.
pub const LINE_BYTES: usize = 22;

/// The longest secret part a key of Faraday's has: the string-to-key
/// usage octet, a 256-bit MPI and its two-octet checksum.
pub const SECRET_MAX: usize = 1 + 2 + 32 + 2;

/// Why a line does not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineError {
    /// A character other than 0–9, A–F in either case, a space, or a
    /// colon after the line's number.
    Chars,
    /// The number before the colon is `typed`, not the line's.
    Number {
        /// The number typed.
        typed: u32,
    },
    /// The digits are not pairs and then six for the CRC, or the CRC does
    /// not match the line's bytes.
    Crc,
}

/// One line read: its bytes, wiped when it drops, and the CRC it ended
/// with. A line with no bytes is the whole data's CRC, and its CRC is not
/// checked here.
pub struct Line {
    /// The line's bytes.
    pub bytes: Zeroizing<Vec<u8>>,
    /// The six hex digits it ends with.
    pub crc: u32,
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Line `n` of a paperkey's base16 form, as printed or typed: an optional
/// number and colon (checked against `n`), then bytes in hex and the
/// line's CRC-24 in six hex digits, spaces anywhere. The CRC is checked
/// against the bytes; a line of six digits alone is the whole data's CRC,
/// checked by the caller.
pub fn line(n: u32, text: &str) -> Result<Line, LineError> {
    let mut rest = text;
    if let Some(colon) = text.find(':') {
        let head = text[..colon].trim_matches(|c| c == ' ' || c == '\t');
        if head.is_empty() || !head.bytes().all(|b| b.is_ascii_digit()) {
            return Err(LineError::Chars);
        }
        let typed: u32 = head.parse().map_err(|_| LineError::Chars)?;
        if typed != n {
            return Err(LineError::Number { typed });
        }
        rest = &text[colon + 1..];
    }
    let mut digits = 0usize;
    for b in rest.bytes() {
        match b {
            b' ' | b'\t' => {}
            _ if hex_value(b).is_some() => digits += 1,
            _ => return Err(LineError::Chars),
        }
    }
    if digits < 6 || !digits.is_multiple_of(2) {
        return Err(LineError::Crc);
    }
    let count = (digits - 6) / 2;
    // Sized once, so that no copy is left behind by growing.
    let mut bytes = Zeroizing::new(Vec::with_capacity(count));
    let mut crc = 0u32;
    let mut high: Option<u8> = None;
    for (seen, v) in rest.bytes().filter_map(hex_value).enumerate() {
        if seen < 2 * count {
            match high.take() {
                Some(h) => bytes.push(h << 4 | v),
                None => high = Some(v),
            }
        } else {
            crc = crc << 4 | u32::from(v);
        }
    }
    if count > 0 && crc24(&bytes) != crc {
        return Err(LineError::Crc);
    }
    Ok(Line { bytes, crc })
}

/// Why a whole paperkey does not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    /// Line `n` does not read.
    Line(u32, LineError),
    /// Line `n` has no number and colon before its bytes.
    NoNumber(u32),
    /// Something follows the whole data's CRC, on line `n`.
    After(u32),
    /// The whole data's CRC does not match ("CRC of secret does not
    /// match").
    Crc,
    /// The whole data's CRC is not there ("CRC of secret is missing").
    CrcMissing,
}

/// Costs a CRC-24 over each line. A paperkey's base16 form as paperkey
/// prints it, comment lines included: the data, its lines and the whole
/// checked by their CRCs.
pub fn read_text(text: &str) -> Result<Zeroizing<Vec<u8>>, ReadError> {
    // No more bytes than half the digits: sized once.
    let mut data = Zeroizing::new(Vec::with_capacity(text.len() / 2));
    let mut n = 0u32;
    let mut done = false;
    for raw in text.split('\n') {
        let l = raw.strip_suffix('\r').unwrap_or(raw);
        let t = l.trim_start();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        n += 1;
        if done {
            return Err(ReadError::After(n));
        }
        if !t.contains(':') {
            return Err(ReadError::NoNumber(n));
        }
        let read = line(n, l).map_err(|e| ReadError::Line(n, e))?;
        if read.bytes.is_empty() {
            if read.crc != crc24(&data) {
                return Err(ReadError::Crc);
            }
            done = true;
        } else {
            data.extend_from_slice(&read.bytes);
        }
    }
    if !done {
        return Err(ReadError::CrcMissing);
    }
    Ok(data)
}

/// Costs a CRC-24 over the data. paperkey's raw form (`--output-type
/// raw`): the data, then its CRC-24 in three bytes, high byte first.
pub fn read_raw(bytes: &[u8]) -> Result<Zeroizing<Vec<u8>>, ReadError> {
    if bytes.len() < 4 {
        return Err(ReadError::CrcMissing);
    }
    let (data, tail) = bytes.split_at(bytes.len() - 3);
    let crc = u32::from(tail[0]) << 16 | u32::from(tail[1]) << 8 | u32::from(tail[2]);
    if crc24(data) != crc {
        return Err(ReadError::Crc);
    }
    let mut out = Zeroizing::new(Vec::with_capacity(data.len()));
    out.extend_from_slice(data);
    Ok(out)
}

/// One key's entry in a paperkey's data.
pub struct Entry<'a> {
    /// The key's fingerprint.
    pub fingerprint: [u8; 20],
    /// The secret part of its secret key packet: from the string-to-key
    /// usage octet to the packet's end.
    pub secret: &'a [u8],
}

/// Every entry of a paperkey's data, in order, the primary key's first.
/// A format version other than 0, a key version other than 4 and data
/// that stops inside an entry are refused.
pub fn entries(data: &[u8]) -> Result<Vec<Entry<'_>>, Refusal> {
    let (&format, mut rest) = data.split_first().ok_or(Refusal::Unreadable)?;
    if format != 0 {
        return Err(Refusal::FormatVersion(format));
    }
    let mut out = Vec::new();
    while let Some((&version, after)) = rest.split_first() {
        if version != 4 {
            return Err(Refusal::Version(version));
        }
        if after.len() < 22 {
            return Err(Refusal::Unreadable);
        }
        let mut fingerprint = [0u8; 20];
        fingerprint.copy_from_slice(&after[..20]);
        let len = usize::from(u16::from_be_bytes([after[20], after[21]]));
        let secret = after.get(22..22 + len).ok_or(Refusal::Unreadable)?;
        out.push(Entry {
            fingerprint,
            secret,
        });
        rest = &after[22 + len..];
    }
    Ok(out)
}

/// What the start of a paperkey's data says, once checked against the
/// keys it must be for ([`check`]): public, all of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    /// The fingerprints read, in order.
    pub fingerprints: Vec<[u8; 20]>,
    /// The data's whole length, once every entry's length is read.
    pub total: Option<usize>,
}

/// The start of a paperkey's data, `prefix`, checked against `keys`, the
/// fingerprints of the certificate's primary key and subkey in that
/// order, as far as it goes (`docs/GPG-PAPERKEY.md` §4.3, steps 6 to 8):
/// the format version 0; each entry's key version 4 and fingerprint the
/// key's; each secret part's string-to-key usage 0 and its length one a
/// key of Faraday's has; no more entries than keys. `ended` says the data
/// is all there: fewer entries than keys are then refused too. A check
/// that cannot be made yet waits for more bytes.
pub fn check(prefix: &[u8], keys: &[[u8; 20]], ended: bool) -> Result<Progress, Refusal> {
    let mut p = Progress::default();
    let Some((&format, _)) = prefix.split_first() else {
        return if ended {
            Err(Refusal::Unreadable)
        } else {
            Ok(p)
        };
    };
    if format != 0 {
        return Err(Refusal::FormatVersion(format));
    }
    let mut at = 1usize;
    while at < prefix.len() {
        let k = p.fingerprints.len();
        let Some(want) = keys.get(k) else {
            return Err(Refusal::Count(k + 1));
        };
        if prefix[at] != 4 {
            return Err(Refusal::Version(prefix[at]));
        }
        let Some(fp) = prefix.get(at + 1..at + 21) else {
            break;
        };
        let mut fingerprint = [0u8; 20];
        fingerprint.copy_from_slice(fp);
        if &fingerprint != want {
            return Err(if k == 0 {
                Refusal::OtherKey(fingerprint)
            } else {
                Refusal::OtherSubkey(fingerprint)
            });
        }
        p.fingerprints.push(fingerprint);
        let Some(len) = prefix.get(at + 21..at + 23) else {
            break;
        };
        let len = usize::from(u16::from_be_bytes([len[0], len[1]]));
        match prefix.get(at + 23) {
            Some(0) => {}
            Some(_) => return Err(Refusal::Protected),
            // The usage octet decides between a protected key and one
            // that does not read: wait for it.
            None => break,
        }
        if len > SECRET_MAX {
            return Err(Refusal::Unreadable);
        }
        at += 23 + len;
        if p.fingerprints.len() == keys.len() {
            p.total = Some(at);
            if prefix.len() > at {
                return Err(Refusal::Count(keys.len() + 1));
            }
        }
    }
    if ended && p.total != Some(prefix.len()) {
        return Err(if at == prefix.len() {
            Refusal::Count(p.fingerprints.len())
        } else {
            Refusal::Unreadable
        });
    }
    Ok(p)
}
