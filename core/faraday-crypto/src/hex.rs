//! Hexadecimal text, in one place (`docs/ESTABLISHED.md` §5.11).
//!
//! The hex that carried secrets in the copies this replaced (private keys,
//! entropy, BIP-85's hex, vault fingerprints) and the wallet data named in
//! `docs/ESTABLISHED.md` §5.11 go through here, over `base16ct`, which
//! works "without data-dependent branches or lookup tables".
//!
//! What stays outside: `faraday-psbt`'s transaction reader (bitcoin's
//! `FromHex`, no secret), `faraday-payload`'s direct `base16ct` calls,
//! paperkey's direct `upper` and `mixed` in `faraday-pgp`, the key origin's
//! `{fp:08x}` in `faraday-qr`'s registry, and display-only hex (the vault's
//! error text, the visit-taken line, the examples).
//! Output is lower case, with no prefix and no separators; input may be
//! either case (or a mix) and is refused if its length is odd, if it does
//! not fit the buffer, or if a character is not a hex digit.

use alloc::string::String;
use alloc::vec;

use zeroize::Zeroizing;

/// Write `bytes` as lower-case hex into `out` and return the text.
///
/// # Panics
///
/// If `out` is shorter than twice `bytes`; callers size the buffer from a
/// length they know.
pub fn encode_lower<'a>(bytes: &[u8], out: &'a mut [u8]) -> &'a str {
    base16ct::lower::encode_str(bytes, out).expect("the hex buffer is twice the bytes")
}

/// `bytes` as lower-case hex in a string that is wiped when dropped, for
/// key material. The buffer written through is the string's own, so no
/// other copy is left behind.
pub fn encode_secret(bytes: &[u8]) -> Zeroizing<String> {
    let mut buf = Zeroizing::new(vec![0u8; bytes.len() * 2]);
    base16ct::lower::encode(bytes, &mut buf).expect("the hex buffer is twice the bytes");
    // The digits are ASCII, so the conversion cannot fail; the bytes move
    // into the string without a copy.
    let text = String::from_utf8(core::mem::take(&mut *buf)).expect("hex digits are ASCII");
    Zeroizing::new(text)
}

/// Decode `text` (either case) into the start of `out` and return how many
/// bytes it holds. `None` if the length is odd or the bytes do not fit in
/// `out`, both checked before any digit is read, or if a character is not a
/// hex digit.
pub fn decode_into(text: &str, out: &mut [u8]) -> Option<usize> {
    base16ct::mixed::decode(text, out).ok().map(<[u8]>::len)
}

/// `bytes` as lower-case hex in a new string, for text that is not secret.
pub fn encode_string(bytes: &[u8]) -> String {
    base16ct::lower::encode_string(bytes)
}

/// The bytes `text` spells (either case), `None` if its length is odd or a
/// character is not a hex digit. Empty text gives no bytes.
pub fn decode_vec(text: &str) -> Option<alloc::vec::Vec<u8>> {
    base16ct::mixed::decode_vec(text).ok()
}

/// Exactly `N` bytes from exactly `2 * N` hex digits (either case), `None`
/// for any other length or a character that is not a hex digit.
pub fn decode_exact<const N: usize>(text: &str) -> Option<[u8; N]> {
    let mut out = [0u8; N];
    (decode_into(text, &mut out)? == N).then_some(out)
}
