//! BBQr (Coinkite's public-domain spec, `BBQr.md`): a file split across
//! QR codes, each with an eight-character header.
//!
//! ```text
//! B$ <encoding> <file type> <total, 2 base36> <index, 2 base36> <data>
//! ```
//!
//! Read: file types `P` (PSBT), `T` (transaction), `U` (text) and `J`
//! (JSON), in encodings `2` (base32), `H` (hex) and `Z` (raw DEFLATE, then
//! base32), as the Wallets tab and Faraday OS write them. Written: `2` only
//! (`docs/QR.md` §4), so nothing Faraday sends needs a decompressor.
//!
//! The limits are Faraday OS's, and every one fails closed: 4,096
//! characters a frame, 1,295 parts, 360 KiB put together or decompressed.
//! Parts of two transfers, a part arriving twice with different contents,
//! parts of uneven length and malformed encodings are all refused.

use std::collections::BTreeMap;

/// The longest frame read.
pub const MAX_FRAME: usize = 4096;
/// The most parts two base36 digits count.
pub const MAX_PARTS: usize = 1295;
/// The most bytes a transfer puts together or decompresses to.
pub const MAX_DATA: usize = 360 * 1024;

const B36: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const B32: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Whether `text` is offered as a BBQr part.
pub fn is_part(text: &str) -> bool {
    text.starts_with("B$")
}

fn b36(c: u8) -> Option<usize> {
    B36.iter().position(|&x| x == c)
}

fn b36_pair(n: usize) -> [u8; 2] {
    [B36[n / 36], B36[n % 36]]
}

/// RFC 4648 base32, upper case, no padding.
pub fn base32(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(5) * 8);
    for chunk in bytes.chunks(5) {
        let mut buf = [0u8; 5];
        buf[..chunk.len()].copy_from_slice(chunk);
        let n = u64::from_be_bytes([0, 0, 0, buf[0], buf[1], buf[2], buf[3], buf[4]]);
        let chars = (chunk.len() * 8).div_ceil(5);
        for i in 0..chars {
            out.push(char::from(B32[((n >> (35 - 5 * i)) & 31) as usize]));
        }
    }
    out
}

/// The bytes of unpadded base32; `None` when it is not exactly that.
fn unbase32(text: &str) -> Option<Vec<u8>> {
    let vals: Vec<u8> = text
        .bytes()
        .map(|c| B32.iter().position(|&x| x == c).map(|p| p as u8))
        .collect::<Option<_>>()?;
    // A length a whole number of bytes cannot leave is not base32.
    if matches!(vals.len() % 8, 1 | 3 | 6) {
        return None;
    }
    let mut out = Vec::with_capacity(vals.len() * 5 / 8);
    let (mut acc, mut bits) = (0u32, 0u32);
    for v in vals {
        acc = (acc << 5) | u32::from(v);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    // Leftover bits must be zero, or the encoding was not canonical.
    if acc != 0 {
        return None;
    }
    Some(out)
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2)
        || text
            .bytes()
            .any(|c| !matches!(c, b'0'..=b'9' | b'A'..=b'F'))
    {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

/// Why a part or a transfer was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not a BBQr part this reads.
    Malformed(&'static str),
    /// A file type Faraday does not read.
    Type(char),
    /// A part of another transfer than the one being put together.
    OtherTransfer,
    /// The same part twice, with different contents.
    Conflict,
    /// Over a limit.
    TooLarge,
}

impl Error {
    /// The sentence a screen shows.
    pub fn reason(&self) -> String {
        match self {
            Error::Malformed(why) => format!("Not a BBQr part: {why}"),
            Error::Type(t) => format!("A BBQr file of type {t}, which Faraday does not read"),
            Error::OtherTransfer => {
                "A part of another BBQr transfer. Start again to read that one".to_string()
            }
            Error::Conflict => "A BBQr part arrived twice with different contents".to_string(),
            Error::TooLarge => "The BBQr transfer is over 360 KiB".to_string(),
        }
    }
}

/// Where a transfer stands after a part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Got {
    /// Parts still missing, numbered from 1.
    Part {
        /// Parts read.
        have: usize,
        /// Parts in all.
        total: usize,
        /// The parts not read yet.
        missing: Vec<usize>,
    },
    /// The whole file.
    Done {
        /// `P`, `T`, `U` or `J`.
        file_type: char,
        /// Its bytes.
        data: Vec<u8>,
    },
}

/// A transfer being put together.
#[derive(Debug, Default)]
pub struct Collector {
    header: Option<(u8, u8, usize)>,
    parts: BTreeMap<usize, Vec<u8>>,
    bytes: usize,
}

impl Collector {
    /// Takes one part.
    pub fn add(&mut self, text: &str) -> Result<Got, Error> {
        let text = text.trim();
        if text.len() > MAX_FRAME {
            return Err(Error::TooLarge);
        }
        let b = text.as_bytes();
        if b.len() < 9 || !text.starts_with("B$") {
            return Err(Error::Malformed("too short"));
        }
        let (encoding, file_type) = (b[2], b[3]);
        if !matches!(encoding, b'2' | b'H' | b'Z') {
            return Err(Error::Malformed("an encoding BBQr does not have"));
        }
        if !matches!(file_type, b'P' | b'T' | b'U' | b'J') {
            return Err(Error::Type(char::from(file_type)));
        }
        let (Some(t1), Some(t0), Some(i1), Some(i0)) = (b36(b[4]), b36(b[5]), b36(b[6]), b36(b[7]))
        else {
            return Err(Error::Malformed("a part count that is not base36"));
        };
        let (total, index) = (t1 * 36 + t0, i1 * 36 + i0);
        if total == 0 || index >= total {
            return Err(Error::Malformed("a part number past the count"));
        }
        if let Some(h) = self.header
            && h != (encoding, file_type, total)
        {
            return Err(Error::OtherTransfer);
        }
        let payload = &text[8..];
        let part = match encoding {
            b'H' => unhex(payload),
            _ => unbase32(payload),
        }
        .filter(|p| !p.is_empty())
        .ok_or(Error::Malformed("its data is not in its encoding"))?;
        if let Some(seen) = self.parts.get(&index) {
            if *seen != part {
                return Err(Error::Conflict);
            }
        } else {
            if self.bytes + part.len() > MAX_DATA {
                return Err(Error::TooLarge);
            }
            self.bytes += part.len();
            self.parts.insert(index, part);
            self.header = Some((encoding, file_type, total));
        }
        if self.parts.len() < total {
            return Ok(Got::Part {
                have: self.parts.len(),
                total,
                missing: (0..total)
                    .filter(|i| !self.parts.contains_key(i))
                    .map(|i| i + 1)
                    .collect(),
            });
        }
        // Every part but the last is the same length, and the last is no
        // longer: anything else was not cut from one file.
        let lens: Vec<usize> = self.parts.values().map(Vec::len).collect();
        if lens.len() > 1
            && (lens[..lens.len() - 1].iter().any(|&l| l != lens[0])
                || lens[lens.len() - 1] > lens[0])
        {
            return Err(Error::Malformed("its parts are of uneven length"));
        }
        let joined: Vec<u8> = self.parts.values().flatten().copied().collect();
        let data = if encoding == b'Z' {
            miniz_oxide::inflate::decompress_to_vec_with_limit(&joined, MAX_DATA).map_err(|_| {
                Error::Malformed("its compressed data does not inflate within 360 KiB")
            })?
        } else {
            joined
        };
        Ok(Got::Done {
            file_type: char::from(file_type),
            data,
        })
    }
}

/// `data` as BBQr parts in encoding `2`, `part_bytes` of data a part (a
/// multiple of 5, so each part is whole base32 characters). `None` when it
/// would take more than 1,295 parts.
pub fn encode(file_type: char, data: &[u8], part_bytes: usize) -> Option<Vec<String>> {
    let part_bytes = (part_bytes / 5).max(1) * 5;
    let total = data.len().div_ceil(part_bytes).max(1);
    if total > MAX_PARTS || !matches!(file_type, 'P' | 'T' | 'U' | 'J') {
        return None;
    }
    let t = b36_pair(total);
    Some(
        (0..total)
            .map(|i| {
                let chunk = &data[i * part_bytes..data.len().min((i + 1) * part_bytes)];
                let n = b36_pair(i);
                format!(
                    "B$2{file_type}{}{}{}{}{}",
                    char::from(t[0]),
                    char::from(t[1]),
                    char::from(n[0]),
                    char::from(n[1]),
                    base32(chunk)
                )
            })
            .collect(),
    )
}
