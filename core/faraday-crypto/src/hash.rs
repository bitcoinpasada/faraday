use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256, Sha512};

use crate::work::{Compute, Work};

/// SHA-256 of `data`: for small, fixed inputs (a key, a checksum's
/// bytes). A whole file is hashed with [`sha256_whole`].
pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

/// Costs a SHA-256 pass over every byte: the SHA-256 of a whole file or
/// transaction, `bytes`, counted as [`Work::FileHash`].
pub fn sha256_whole(c: &Compute, bytes: &[u8]) -> [u8; 32] {
    c.count(Work::FileHash);
    Sha256::digest(bytes).into()
}

/// HMAC-SHA-512 of `msg` under `key`. Any key length is accepted, as
/// RFC 2104 specifies.
pub fn hmac_sha512(key: &[u8], msg: &[u8]) -> [u8; 64] {
    let mut mac = Hmac::<Sha512>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

/// Costs a PBKDF2: `rounds` HMACs per 64 bytes of `out`. PBKDF2 with
/// HMAC-SHA-512 as the PRF (RFC 8018 §5.2). Fills `out`, which may be any length; BIP-39 uses 64
/// bytes and 2048 rounds.
pub fn pbkdf2_hmac_sha512(c: &Compute, password: &[u8], salt: &[u8], rounds: u32, out: &mut [u8]) {
    c.count(Work::Kdf);
    pbkdf2::pbkdf2_hmac::<Sha512>(password, salt, rounds, out);
}

/// HMAC-SHA-256 of `msg` under `key`. Any key length is accepted, as
/// RFC 2104 specifies.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

/// The most bytes [`hkdf_sha256_expand`] makes: 255 blocks of 32, the
/// bound RFC 5869 §2.3 puts on `L`.
pub const HKDF_SHA256_MAX: usize = 255 * 32;

/// HKDF-Expand with SHA-256 (RFC 5869 §2.3), an HMAC-SHA-256 per 32
/// bytes of `out`: fills `out` with T(1) ‖ T(2) ‖ …, where T(0) is empty
/// and T(i) = HMAC-SHA-256(`prk`, T(i−1) ‖ `info` ‖ i). `prk` is a
/// pseudorandom key of at least 32 bytes: HKDF-Extract's output, or
/// bytes already uniformly random, as §3.3 allows. `None`, with `out`
/// untouched, when `out` is longer than [`HKDF_SHA256_MAX`].
pub fn hkdf_sha256_expand(prk: &[u8], info: &[u8], out: &mut [u8]) -> Option<()> {
    if out.len() > HKDF_SHA256_MAX {
        return None;
    }
    let mut t = zeroize::Zeroizing::new([0u8; 32]);
    let mut previous = 0;
    for (i, chunk) in out.chunks_mut(32).enumerate() {
        let mut mac = Hmac::<Sha256>::new_from_slice(prk).expect("HMAC accepts any key length");
        mac.update(&t[..previous]);
        mac.update(info);
        // At most 255 blocks, checked above.
        mac.update(&[(i + 1) as u8]);
        t.copy_from_slice(&mac.finalize().into_bytes());
        previous = 32;
        chunk.copy_from_slice(&t[..chunk.len()]);
    }
    Some(())
}

/// Costs a PBKDF2: `rounds` HMACs per 32 bytes of `out`. PBKDF2 with
/// HMAC-SHA-256 as the PRF (RFC 8018 §5.2). Fills `out`, which may be any length; SLIP-39's
/// round function uses it with half the master secret's length.
pub fn pbkdf2_hmac_sha256(c: &Compute, password: &[u8], salt: &[u8], rounds: u32, out: &mut [u8]) {
    c.count(Work::Kdf);
    pbkdf2_sha256(password, salt, rounds, out);
}

/// [`pbkdf2_hmac_sha256`] uncounted, for scrypt's own two single-round
/// passes: scrypt counts itself once.
pub(crate) fn pbkdf2_sha256(password: &[u8], salt: &[u8], rounds: u32, out: &mut [u8]) {
    pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, rounds, out);
}
