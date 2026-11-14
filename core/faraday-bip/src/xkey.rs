//! Strict decoding of an extended key written as text.
//!
//! `bitcoin` 0.32 accepts seven of the sixteen serialisations BIP-32 test
//! vector 5 says a parser must reject, for two reasons: it does not check
//! the pad byte before a private key, and neither half checks that a
//! depth-0 key has a zero parent fingerprint and child number. A person
//! who imports such a key here and nowhere else would get addresses no
//! other wallet reproduces, so every extended key that reaches us as
//! text is decoded by the rules below instead.
//!
//! The primitives are `bitcoin`'s: base58check, `SecretKey`,
//! `PublicKey`, `Xpriv::decode` and `Xpub::decode`. The rules are ours,
//! and every one of them is applied before the byte string is handed to
//! `bitcoin` at all:
//!
//! 1. base58check, so a bad checksum is a rejection;
//! 2. exactly 78 bytes;
//! 3. version bytes that are the ones the caller asked for;
//! 4. at depth 0, a zero parent fingerprint and a zero child number;
//! 5. for a private key, a zero pad byte at offset 45 and a scalar in
//!    `1..n`;
//! 6. for a public key, a point on the curve with a 0x02 or 0x03 prefix.
//!
//! Private material passes through the decoded buffer, which is erased
//! before this module returns.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use alloc::vec;
use alloc::vec::Vec;

use bitcoin::bip32::{Xpriv, Xpub};
use bitcoin::hashes::{Hash, sha256d as hash256};
use bitcoin::secp256k1::{PublicKey, SecretKey};
use faraday_crypto::Zeroizing;

/// Serialized length of an extended key, without the checksum.
pub(crate) const LEN: usize = 78;

/// The most base58 characters a key can have: 82 bytes (the payload and its
/// four-byte check) are at most 112 digits (82 * log58(256) = 111.98), and a
/// leading zero byte is one digit, never more. The decoder is quadratic, so
/// longer text is refused before it runs.
pub(crate) const MAX_TEXT: usize = 112;

/// `xprv`, mainnet private.
pub const XPRV: [u8; 4] = [0x04, 0x88, 0xad, 0xe4];
/// `tprv`, test-network private.
pub const TPRV: [u8; 4] = [0x04, 0x35, 0x83, 0x94];
/// `xpub`, mainnet public.
pub const XPUB: [u8; 4] = [0x04, 0x88, 0xb2, 0x1e];
/// `tpub`, test-network public.
pub const TPUB: [u8; 4] = [0x04, 0x35, 0x87, 0xcf];

/// Why an extended key written as text was rejected.
///
/// The same vocabulary reports a BIP-32 key and a SLIP-132 one; both go
/// through the rules above.
// `allow`, not `expect`: a derive copies `allow` onto what it writes, not `expect`.
#[allow(clippy::disallowed_methods)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not valid base58, or the checksum does not hold.
    Base58,
    /// Decoded to something other than 78 bytes.
    Length(usize),
    /// The version bytes are not one this decoder was asked for.
    UnknownVersion([u8; 4]),
    /// The key says depth 0 but carries a parent fingerprint or a child
    /// number, so it claims to be a master key and a child at once.
    ZeroDepth,
    /// The byte before a private key, which BIP-32 fixes at zero so the
    /// key is 33 bytes like a public one, is something else.
    Pad(u8),
    /// The key material itself is invalid: a private key of zero or at or
    /// above the curve order, or a public key that is not a point on the
    /// curve or carries a prefix other than 0x02 or 0x03.
    Key(bitcoin::bip32::Error),
}

/// The bytes of `s` written in base58check, without the four checksum
/// bytes, or `None` where the text has a character outside base58's
/// alphabet, decodes to fewer than four bytes or fails its checksum. The
/// same as `base58::decode_check`, but every buffer is sized once from the
/// text's length and wiped when it goes: that decoder frees its own
/// unwiped, and the text may be a private key.
pub(crate) fn decode_check(s: &str) -> Option<Zeroizing<Vec<u8>>> {
    const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    // log256(58) is under 0.733, so this many bytes hold any value of the
    // digits after the leading ones.
    let room = s.len() * 733 / 1000 + 1;
    // The number so far, least significant byte first.
    let mut number = Zeroizing::new(vec![0u8; room]);
    let mut used = 0usize;
    for ch in s.bytes() {
        let mut carry = ALPHABET.iter().position(|&b| b == ch)?;
        for byte in &mut number[..used] {
            carry += usize::from(*byte) * 58;
            *byte = (carry & 0xFF) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            *number.get_mut(used)? = (carry & 0xFF) as u8;
            used += 1;
            carry >>= 8;
        }
    }
    let zeros = s.bytes().take_while(|&b| b == b'1').count();
    let total = zeros + used;
    if total < 4 {
        return None;
    }
    let mut bytes = Zeroizing::new(vec![0u8; total]);
    for (to, from) in bytes[zeros..].iter_mut().zip(number[..used].iter().rev()) {
        *to = *from;
    }
    let (body, check) = bytes.split_at(total - 4);
    if hash256::Hash::hash(body).to_byte_array()[..4] != *check {
        return None;
    }
    bytes.truncate(total - 4);
    Some(bytes)
}

/// The 78 bytes of `s`, base58check-decoded and length-checked, in a
/// buffer that wipes itself.
pub(crate) fn payload(s: &str) -> Result<Zeroizing<Vec<u8>>, Error> {
    if s.len() > MAX_TEXT {
        return Err(Error::Base58);
    }
    let bytes = decode_check(s).ok_or(Error::Base58)?;
    if bytes.len() != LEN {
        return Err(Error::Length(bytes.len()));
    }
    Ok(bytes)
}

/// The version bytes of a decoded payload.
pub(crate) fn version(bytes: &[u8]) -> [u8; 4] {
    bytes[..4].try_into().expect("four bytes")
}

/// Rule 4: a key at depth 0 is a master key, so it has no parent and no
/// index.
pub(crate) fn check_depth(bytes: &[u8]) -> Result<(), Error> {
    if bytes[4] == 0 && bytes[5..13].iter().any(|b| *b != 0) {
        return Err(Error::ZeroDepth);
    }
    Ok(())
}

/// Decodes an `xprv` or `tprv`.
pub fn decode_xpriv(s: &str) -> Result<Xpriv, Error> {
    let bytes = payload(s)?;
    xpriv_from_bytes(&bytes)
}

/// Decodes an `xpub` or `tpub`. SLIP-132 prefixes are
/// [`crate::slip132::decode_xpub`], which applies the same rules.
pub fn decode_xpub(s: &str) -> Result<Xpub, Error> {
    let bytes = payload(s)?;
    let v = version(&bytes);
    if v != XPUB && v != TPUB {
        return Err(Error::UnknownVersion(v));
    }
    xpub_from_bytes(&bytes)
}

/// Rules 3 to 5 on a 78-byte private payload, then `bitcoin`'s decoder.
fn xpriv_from_bytes(bytes: &[u8]) -> Result<Xpriv, Error> {
    let v = version(bytes);
    if v != XPRV && v != TPRV {
        return Err(Error::UnknownVersion(v));
    }
    check_depth(bytes)?;
    if bytes[45] != 0 {
        return Err(Error::Pad(bytes[45]));
    }
    let mut key = SecretKey::from_slice(&bytes[46..])
        .map_err(|e| Error::Key(bitcoin::bip32::Error::Secp256k1(e)))?;
    key.non_secure_erase();
    Xpriv::decode(bytes).map_err(Error::Key)
}

/// Rules 4 and 6 on a 78-byte public payload with BIP-32 version bytes,
/// then `bitcoin`'s decoder.
pub(crate) fn xpub_from_bytes(bytes: &[u8]) -> Result<Xpub, Error> {
    check_depth(bytes)?;
    PublicKey::from_slice(&bytes[45..])
        .map_err(|e| Error::Key(bitcoin::bip32::Error::Secp256k1(e)))?;
    Xpub::decode(bytes).map_err(Error::Key)
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec::Vec;

    use bitcoin::base58;

    use super::{Error, decode_check, payload};

    // BIP-32 test vector 1, chain m.
    const XPUB: &str = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";
    const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";

    fn same_as_bitcoin(text: &str) {
        let ours = decode_check(text);
        let theirs = base58::decode_check(text).ok();
        assert_eq!(
            ours.as_deref().map(Vec::as_slice),
            theirs.as_deref(),
            "{text}"
        );
    }

    #[test]
    fn the_published_keys_decode_as_the_bitcoin_crate_does() {
        for text in [XPUB, XPRV] {
            assert_eq!(
                payload(text).unwrap().as_slice(),
                base58::decode_check(text).unwrap()
            );
        }
    }

    #[test]
    fn a_bad_checksum_a_bad_character_and_a_wrong_length_are_refused() {
        let mut bad = String::from(XPRV);
        bad.pop();
        bad.push('j');
        assert_eq!(payload(&bad).err(), Some(Error::Base58));
        let mut sum = String::from(XPUB);
        sum.pop();
        sum.push('s');
        assert_eq!(payload(&sum).err(), Some(Error::Base58));
        for text in ["", "1", "111", "0OIl", "xpub 661", "é"] {
            assert_eq!(decode_check(text), None, "{text}");
            same_as_bitcoin(text);
        }
        let short = base58::encode_check(&[7u8; 20]);
        assert_eq!(payload(&short).err(), Some(Error::Length(20)));
    }

    #[test]
    fn random_texts_decode_or_fail_as_the_bitcoin_crate_does() {
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for round in 0..3000usize {
            let len = round % 90;
            let mut data: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            // Leading zero bytes are leading ones in the text.
            let zeros = (next() % 4) as usize;
            for b in data.iter_mut().take(zeros) {
                *b = 0;
            }
            let text = base58::encode_check(&data);
            same_as_bitcoin(&text);
            assert_eq!(
                decode_check(&text).as_deref().map(Vec::as_slice),
                Some(&data[..])
            );
            // The same text with one character changed.
            let mut chars: Vec<char> = text.chars().collect();
            let at = (next() as usize) % chars.len();
            let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz0Ol";
            let pick = alphabet
                .chars()
                .nth((next() as usize) % alphabet.len())
                .unwrap();
            chars[at] = pick;
            same_as_bitcoin(&chars.iter().collect::<String>());
            // Arbitrary text from the alphabet.
            let free: String = (0..(next() % 120) as usize)
                .map(|_| alphabet.chars().nth((next() as usize) % 58).unwrap())
                .collect();
            same_as_bitcoin(&free);
        }
    }
}
