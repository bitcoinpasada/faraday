use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256, Sha512};

/// SHA-256 of `data`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

/// HMAC-SHA-512 of `msg` under `key`. Any key length is accepted, as
/// RFC 2104 specifies.
pub fn hmac_sha512(key: &[u8], msg: &[u8]) -> [u8; 64] {
    let mut mac = Hmac::<Sha512>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

/// PBKDF2 with HMAC-SHA-512 as the PRF (RFC 8018 §5.2). Fills `out`, which
/// may be any length; BIP-39 uses 64 bytes and 2048 rounds.
pub fn pbkdf2_hmac_sha512(password: &[u8], salt: &[u8], rounds: u32, out: &mut [u8]) {
    pbkdf2::pbkdf2_hmac::<Sha512>(password, salt, rounds, out);
}

/// HMAC-SHA-256 of `msg` under `key`. Any key length is accepted, as
/// RFC 2104 specifies.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

/// PBKDF2 with HMAC-SHA-256 as the PRF (RFC 8018 §5.2). Fills `out`, which
/// may be any length; SLIP-39's round function uses it with half the
/// master secret's length.
pub fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], rounds: u32, out: &mut [u8]) {
    pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, rounds, out);
}
