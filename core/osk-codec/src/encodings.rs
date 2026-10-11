//! The text encodings Bitcoin writes bytes in: Base58, Base58Check,
//! Bech32 and Bech32m, and plain hex.
//!
//! [`read`] says which one a string is and what it decodes to; the
//! encoders beside it write the same bytes in the other spellings. The
//! primitives are `bitcoin`'s `base58` and the `bech32` crate it
//! re-exports, so the checksum rules are the ones every other wallet
//! applies.

use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::base58;
use bitcoin::bech32::primitives::decode::UncheckedHrpstring;
use bitcoin::bech32::{Bech32, Bech32m, Hrp};

/// How a string spells its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// Base58 with a four-byte double-SHA-256 check.
    Base58Check,
    /// Base58 with nothing appended.
    Base58,
    /// BIP-173 bech32.
    Bech32,
    /// BIP-350 bech32m.
    Bech32m,
    /// Hex digits, two per byte.
    Hex,
}

/// What [`read`] made of a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// Which encoding the string is.
    pub encoding: Encoding,
    /// The bytes it carries, without the version byte or the checksum.
    pub bytes: Vec<u8>,
    /// The first byte of a Base58Check payload, which is its version.
    pub version: Option<u8>,
    /// The human-readable part of a bech32 or bech32m string.
    pub hrp: Option<String>,
    /// Whether the checksum holds, for the encodings that carry one.
    pub checksum: Option<bool>,
}

/// Reads `text` as one of the encodings, or `None` when it is none of
/// them.
///
/// The order is the one that cannot mistake a string for something
/// else: a bech32 string carries its separator and its own charset, a
/// Base58Check string decodes with its checksum, and hex is only hex
/// when every character is a hex digit and there is an even number of
/// them.
pub fn read(text: &str) -> Option<Reading> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(r) = read_bech32(text) {
        return Some(r);
    }
    if let Ok(payload) = base58::decode_check(text) {
        return Some(Reading {
            encoding: Encoding::Base58Check,
            version: payload.first().copied(),
            bytes: payload.get(1..).unwrap_or_default().to_vec(),
            hrp: None,
            checksum: Some(true),
        });
    }
    if let Ok(bytes) = base58::decode(text) {
        return Some(Reading {
            encoding: Encoding::Base58,
            bytes,
            version: None,
            hrp: None,
            checksum: None,
        });
    }
    if let Some(bytes) = from_hex(text) {
        return Some(Reading {
            encoding: Encoding::Hex,
            bytes,
            version: None,
            hrp: None,
            checksum: None,
        });
    }
    None
}

/// A bech32 or bech32m string, by which checksum holds over it.
fn read_bech32(text: &str) -> Option<Reading> {
    let unchecked = UncheckedHrpstring::new(text).ok()?;
    let encoding = if unchecked.has_valid_checksum::<Bech32>() {
        Encoding::Bech32
    } else if unchecked.has_valid_checksum::<Bech32m>() {
        Encoding::Bech32m
    } else {
        return None;
    };
    let checked = match encoding {
        Encoding::Bech32 => unchecked.remove_checksum::<Bech32>(),
        _ => unchecked.remove_checksum::<Bech32m>(),
    };
    Some(Reading {
        encoding,
        bytes: checked.byte_iter().collect(),
        version: None,
        hrp: Some(checked.hrp().to_lowercase()),
        checksum: Some(true),
    })
}

/// The bytes of an even-length string of hex digits.
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    if text.is_empty()
        || !text.len().is_multiple_of(2)
        || !text.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let digit = |b: u8| -> u8 {
        match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            _ => b - b'A' + 10,
        }
    };
    let bytes = text.as_bytes();
    Some(
        bytes
            .chunks(2)
            .map(|pair| digit(pair[0]) << 4 | digit(pair[1]))
            .collect(),
    )
}

/// `payload` under `version`, in Base58Check.
pub fn base58check(version: u8, payload: &[u8]) -> String {
    let mut bytes = Vec::with_capacity(payload.len() + 1);
    bytes.push(version);
    bytes.extend_from_slice(payload);
    base58::encode_check(&bytes)
}

/// `bytes` in Base58, with nothing appended.
pub fn base58(bytes: &[u8]) -> String {
    base58::encode(bytes)
}

/// `bytes` under `hrp`, in bech32 (BIP-173).
pub fn bech32(hrp: &str, bytes: &[u8]) -> Option<String> {
    let hrp = Hrp::parse(hrp).ok()?;
    bitcoin::bech32::encode::<Bech32>(hrp, bytes).ok()
}

/// `bytes` under `hrp`, in bech32m (BIP-350).
pub fn bech32m(hrp: &str, bytes: &[u8]) -> Option<String> {
    let hrp = Hrp::parse(hrp).ok()?;
    bitcoin::bech32::encode::<Bech32m>(hrp, bytes).ok()
}

/// How text names bytes, where it could be either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadAs {
    /// Hex when every character is a hex digit and there is an even
    /// number of them, UTF-8 text otherwise.
    Auto,
    /// The characters' own bytes, whatever they spell.
    Text,
    /// Hex digits, two per byte.
    Hex,
}

impl ReadAs {
    /// The three, in the order a Choice lists them.
    pub const ALL: [ReadAs; 3] = [ReadAs::Auto, ReadAs::Text, ReadAs::Hex];
}

/// The bytes `text` names under `read_as`: its hex when that says hex
/// (or says nothing and the text is hex), its own UTF-8 bytes otherwise.
/// Text that is not hex, read as hex, is no bytes.
pub fn read_input(text: &str, read_as: ReadAs) -> Vec<u8> {
    match read_as {
        ReadAs::Text => text.as_bytes().to_vec(),
        ReadAs::Hex => from_hex(text).unwrap_or_default(),
        ReadAs::Auto => from_hex(text).unwrap_or_else(|| text.as_bytes().to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base58check_round_trips_through_bitcoins_own_encoder() {
        let payload = [0x11u8; 20];
        let text = base58check(0x00, &payload);
        assert_eq!(
            base58::encode_check(&[&[0u8][..], &payload[..]].concat()),
            text
        );
        let read = read(&text).expect("an encoding");
        assert_eq!(read.encoding, Encoding::Base58Check);
        assert_eq!(read.version, Some(0));
        assert_eq!(read.bytes, payload);
    }

    #[test]
    fn bech32_and_bech32m_are_told_apart() {
        let data = [0x00u8, 0x01, 0x02, 0x03, 0x04];
        let b = bech32("bc", &data).expect("bech32");
        let m = bech32m("bc", &data).expect("bech32m");
        assert_eq!(read(&b).expect("read").encoding, Encoding::Bech32);
        assert_eq!(read(&m).expect("read").encoding, Encoding::Bech32m);
        assert_eq!(read(&b).expect("read").bytes, data);
    }

    #[test]
    fn hex_needs_an_even_number_of_hex_digits() {
        assert_eq!(from_hex("00ff"), Some(alloc::vec![0x00, 0xff]));
        assert_eq!(from_hex("0f0"), None);
        assert_eq!(from_hex("zz"), None);
    }
}
