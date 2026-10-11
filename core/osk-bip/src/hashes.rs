//! The three hashes Bitcoin names bytes by: SHA-256, SHA-256d and
//! HASH160, which is what the Hashes calculator states.

use bitcoin::hashes::{Hash, ripemd160, sha256};

/// The three hashes of some bytes, and how many bytes they were.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hashes {
    /// How long the input was.
    pub len: usize,
    /// SHA-256.
    pub sha256: [u8; 32],
    /// SHA-256 of SHA-256.
    pub sha256d: [u8; 32],
    /// RIPEMD-160 of SHA-256, which Bitcoin calls HASH160.
    pub hash160: [u8; 20],
}

/// The three hashes of `bytes`.
pub fn hashes(bytes: &[u8]) -> Hashes {
    let once = osk_crypto::sha256(bytes);
    Hashes {
        len: bytes.len(),
        sha256: once,
        sha256d: osk_crypto::sha256(&once),
        hash160: ripemd160::Hash::hash(sha256::Hash::hash(bytes).as_byte_array()).to_byte_array(),
    }
}
