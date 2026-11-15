//! Base64 decoding, which wipes what it holds: the text may spell a secret.

use alloc::vec::Vec;
use base64ct::{Base64, Encoding};
use zeroize::Zeroizing;

use super::DecodeError;

/// Decodes `text`. ASCII whitespace is ignored anywhere (files end with a
/// newline). Any other character outside the alphabet, padding in the
/// wrong place or missing, a dangling single sextet, or non-zero bits
/// left over in the last sextet is an error: an encoding has one
/// spelling.
///
/// The text may spell a secret, and decoding is reversible: the white-space
/// free copy and the decoded bytes are held in buffers that zeroize on
/// drop, a failed decode included.
pub fn decode(text: &str) -> Result<Zeroizing<Vec<u8>>, DecodeError> {
    let squeezed: Zeroizing<Vec<u8>>;
    let input: &str = if text.bytes().any(|b| b.is_ascii_whitespace()) {
        squeezed = Zeroizing::new(text.bytes().filter(|b| !b.is_ascii_whitespace()).collect());
        core::str::from_utf8(&squeezed).map_err(|_| DecodeError)?
    } else {
        text
    };
    // Four characters decode to at most three bytes.
    let mut out = Zeroizing::new(alloc::vec![0u8; input.len() / 4 * 3 + 3]);
    let n = Base64::decode(input, &mut out)
        .map_err(|_| DecodeError)?
        .len();
    out.truncate(n);
    Ok(out)
}
