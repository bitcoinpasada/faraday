//! Standard base64 (RFC 4648 §4, `+/` alphabet, `=` padding), the text
//! encoding a PSBT and a BSMS signature are written in.
//!
//! `bitcoin`'s optional `base64` feature pulls the `base64` crate with its
//! `std` feature, which a `no_std` core crate cannot accept, so the codec
//! is in-house: about sixty lines, exercised against every BIP-174 vector.

use alloc::string::String;
use alloc::vec::Vec;

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes `bytes` with padding.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    each_char(bytes, |c| out.push(char::from(c)));
    out
}

/// Encodes `bytes` into `out` without padding, returning how many
/// characters were written. `out` must hold `bytes.len() * 4 / 3`
/// rounded up; a shorter one stops the encoding where it fills.
///
/// BIP-85's password applications encode a secret
/// ([`crate::bip85::child_password_base64`]) and cannot let it reach a
/// heap string, so they write into a buffer that zeroizes on drop.
pub fn encode_into(bytes: &[u8], out: &mut [u8]) -> usize {
    let mut n = 0;
    each_char(bytes, |c| {
        if c != b'=' && n < out.len() {
            out[n] = c;
            n += 1;
        }
    });
    n
}

/// Every character of the padded encoding of `bytes`, in order.
fn each_char(bytes: &[u8], mut push: impl FnMut(u8)) {
    for chunk in bytes.chunks(3) {
        let mut buf = [0u8; 3];
        buf[..chunk.len()].copy_from_slice(chunk);
        let n = (u32::from(buf[0]) << 16) | (u32::from(buf[1]) << 8) | u32::from(buf[2]);
        let sextets = [(n >> 18) & 63, (n >> 12) & 63, (n >> 6) & 63, n & 63];
        for (i, s) in sextets.iter().enumerate() {
            if i <= chunk.len() {
                push(ALPHABET[*s as usize]);
            } else {
                push(b'=');
            }
        }
    }
}

fn value(c: u8) -> Option<u32> {
    let v = match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    };
    Some(u32::from(v))
}

/// Decodes `text`. ASCII whitespace is ignored anywhere (files end with a
/// newline); any other character outside the alphabet, padding in the
/// wrong place, or a dangling single sextet is an error. Padding may be
/// omitted at the end.
pub fn decode(text: &str) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0u32;
    let mut padding = 0usize;
    for &c in text.as_bytes() {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            padding += 1;
            continue;
        }
        if padding > 0 {
            return Err(DecodeError);
        }
        let v = value(c).ok_or(DecodeError)?;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    // A dangling sextet (one character after a 3-byte boundary) encodes
    // nothing; the padding count must match the missing characters.
    let rem = (out.len() % 3, bits);
    let expected_padding = match rem {
        (0, 0) => 0,
        (1, 4) => 2,
        (2, 2) => 1,
        _ => return Err(DecodeError),
    };
    if padding != 0 && padding != expected_padding {
        return Err(DecodeError);
    }
    Ok(out)
}

/// The text was not valid base64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeError;

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid base64")
    }
}

impl core::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4648_vectors() {
        let cases: [(&[u8], &str); 7] = [
            (b"", ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (b"fooba", "Zm9vYmE="),
            (b"foobar", "Zm9vYmFy"),
        ];
        for (raw, text) in cases {
            assert_eq!(encode(raw), text);
            assert_eq!(decode(text).unwrap(), raw);
            assert_eq!(decode(text.trim_end_matches('=')).unwrap(), raw);
        }
    }

    #[test]
    fn whitespace_and_errors() {
        assert_eq!(decode("Zm9v\nYmFy\n").unwrap(), b"foobar");
        assert!(decode("Zm9v!").is_err());
        assert!(decode("Z").is_err());
        assert!(decode("Zg=").is_err());
        assert!(decode("Zg==Zg").is_err());
        assert!(decode("Zm8==").is_err());
    }

    #[test]
    fn all_byte_values_round_trip() {
        let bytes: Vec<u8> = (0..=255u8).collect();
        assert_eq!(decode(&encode(&bytes)).unwrap(), bytes);
    }
}
