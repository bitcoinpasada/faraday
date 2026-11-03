//! Standard base64 (RFC 4648 §4, `+/` alphabet, `=` padding), the text
//! encoding a PSBT and a BSMS signature are written in: the `base64ct`
//! crate's, which is constant time with respect to the data.

use alloc::string::String;
use alloc::vec::Vec;
use base64ct::{Base64, Base64Unpadded, Encoding};

/// Encodes `bytes` with padding.
pub fn encode(bytes: &[u8]) -> String {
    Base64::encode_string(bytes)
}

/// Encodes `bytes` into `out` without padding, returning how many
/// characters were written. `out` must hold `bytes.len() * 4 / 3`
/// rounded up; a shorter one is left unwritten and 0 is returned.
///
/// BIP-85's password applications encode a secret
/// ([`crate::bip85::child_password_base64`]) and cannot let it reach a
/// heap string, so they write into a buffer that zeroizes on drop.
pub fn encode_into(bytes: &[u8], out: &mut [u8]) -> usize {
    Base64Unpadded::encode(bytes, out).map_or(0, |text| text.len())
}

/// Decodes `text`. ASCII whitespace is ignored anywhere (files end with a
/// newline). Any other character outside the alphabet, padding in the
/// wrong place or missing, a dangling single sextet, or non-zero bits
/// left over in the last sextet is an error: an encoding has one
/// spelling.
pub fn decode(text: &str) -> Result<Vec<u8>, DecodeError> {
    let squeezed: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    Base64::decode_vec(core::str::from_utf8(&squeezed).map_err(|_| DecodeError)?)
        .map_err(|_| DecodeError)
}

/// The text was not valid base64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeError;

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
            if text.ends_with('=') {
                assert!(decode(text.trim_end_matches('=')).is_err(), "{text}");
            }
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
    fn padding_bits_that_are_not_zero_are_refused() {
        // "Zh==" and "Zm9=" decode to the bytes of "Zg==" and "Zm8=" but
        // carry non-zero bits in the last sextet: not canonical.
        for text in ["Zh==", "Zh", "Zm9=", "Zm9", "Zg==", "Zm8="] {
            let canonical = text == "Zg==" || text == "Zm8=";
            assert_eq!(decode(text).is_ok(), canonical, "{text}");
        }
        // An encoding with a second spelling, or without its padding, is not taken.
        assert!(decode("Zm8").is_err());
    }

    #[test]
    fn unpadded_encoding_fills_the_buffer_it_is_given() {
        let mut out = [0u8; 86];
        assert_eq!(encode_into(&[0xff; 64], &mut out), 86);
        assert_eq!(&out[..4], b"////");
        let mut short = [0u8; 3];
        assert_eq!(encode_into(b"foo", &mut short), 0);
    }

    #[test]
    fn all_byte_values_round_trip() {
        let bytes: Vec<u8> = (0..=255u8).collect();
        assert_eq!(decode(&encode(&bytes)).unwrap(), bytes);
    }
}
