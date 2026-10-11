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

use alloc::vec::Vec;
use core::fmt;

use bitcoin::base58;
use bitcoin::bip32::{Xpriv, Xpub};
use bitcoin::secp256k1::{PublicKey, SecretKey};
use osk_crypto::Zeroize;

/// Serialized length of an extended key, without the checksum.
pub(crate) const LEN: usize = 78;

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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Base58 => f.write_str("not valid base58check"),
            Error::Length(n) => write!(f, "extended key is {n} bytes, expected 78"),
            Error::UnknownVersion(v) => {
                write!(
                    f,
                    "unknown version {:02x}{:02x}{:02x}{:02x}",
                    v[0], v[1], v[2], v[3]
                )
            }
            Error::ZeroDepth => f.write_str("depth 0 with a parent fingerprint or child number"),
            Error::Pad(b) => write!(f, "byte before the private key is {b:02x}, expected 00"),
            Error::Key(e) => write!(f, "invalid extended key: {e}"),
        }
    }
}

impl core::error::Error for Error {}

/// The 78 bytes of `s`, base58check-decoded and length-checked.
///
/// The caller owns the buffer and must erase it when it may hold private
/// material.
pub(crate) fn payload(s: &str) -> Result<Vec<u8>, Error> {
    let bytes = base58::decode_check(s).map_err(|_| Error::Base58)?;
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
    let mut bytes = payload(s)?;
    let result = xpriv_from_bytes(&bytes);
    bytes.as_mut_slice().zeroize();
    result
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
    SecretKey::from_slice(&bytes[46..])
        .map_err(|e| Error::Key(bitcoin::bip32::Error::Secp256k1(e)))?;
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
