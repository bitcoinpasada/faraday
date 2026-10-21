//! OpenPGP keys: a key made from seeds, and what is made from it — the
//! certificate, detached signatures, revocations and the paperkey text.
//! The packets are faraday-pgp's.

use faraday_pgp::{Armor, Hash, Reason, armor};
use zeroize::Zeroizing;

/// An Ed25519 certification key and its signing subkey, from seeds; the
/// seeds are wiped when it drops.
pub use faraday_pgp::Key;

/// The key whose seeds are `primary` and `subkey`, made at `created` and
/// `subkey_created`, with `user_ids` and `expiry` (seconds after
/// creation, 0 for none); `None` unless both seeds are 32 bytes.
pub fn key_of(
    primary: &[u8],
    created: u32,
    subkey: &[u8],
    subkey_created: u32,
    user_ids: Vec<String>,
    expiry: u32,
) -> Option<Key> {
    if primary.len() != 32 || subkey.len() != 32 {
        return None;
    }
    let mut key = Key {
        primary_seed: [0; 32],
        created,
        subkey_seed: [0; 32],
        subkey_created,
        user_ids,
        expiry,
    };
    key.primary_seed.copy_from_slice(primary);
    key.subkey_seed.copy_from_slice(subkey);
    Some(key)
}

/// A new key from fresh seeds, made at `now` with one user ID.
pub fn make(
    primary: &Zeroizing<[u8; 32]>,
    subkey: &Zeroizing<[u8; 32]>,
    now: u32,
    user_id: String,
    expiry: u32,
) -> Key {
    Key {
        primary_seed: **primary,
        created: now,
        subkey_seed: **subkey,
        subkey_created: now,
        user_ids: vec![user_id],
        expiry,
    }
}

/// The public certificate (packets) of `key`, signed at `at`. Costs
/// Ed25519 signatures.
pub fn certificate(key: &Key, at: u32) -> Vec<u8> {
    key.certificate(at)
}

/// A revocation certificate (packets) for `key`. Costs an Ed25519
/// signature.
pub fn revocation(key: &Key, at: u32, reason: Reason, text: &str) -> Vec<u8> {
    key.revocation(at, reason, text)
}

/// A revocation certificate as the file GnuPG keeps it in.
pub fn revocation_file(key: &Key, revocation: &[u8]) -> String {
    key.revocation_file(revocation)
}

/// A detached signature (packets) over `data` by the signing subkey, at
/// `at`, with SHA-512. Costs an Ed25519 signature.
pub fn sign(key: &Key, data: &[u8], at: u32) -> Vec<u8> {
    key.sign(data, at, Hash::Sha512)
}

/// The signature over `data`, armoured.
pub fn sign_armored(key: &Key, data: &[u8], at: u32) -> String {
    armor(Armor::Signature, &sign(key, data, at))
}

/// The paperkey text of the key: secret, made only to be shown.
pub fn paperkey(key: &Key) -> Zeroizing<String> {
    key.paperkey()
}
