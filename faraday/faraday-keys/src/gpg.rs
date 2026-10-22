//! OpenPGP keys: a key made from seeds, and what is made from it — the
//! certificate, detached signatures, revocations and the paperkey text.
//! The packets are faraday-pgp's.

use faraday_pgp::{Armor, Hash, Reason, armor};
use faraday_vault::Record;
use faraday_vault::records::kind;
use zeroize::Zeroizing;

use crate::vault::{GpgFacts, MadeKey, gpg_field as f};

/// An Ed25519 certification key and its signing subkey, from seeds; the
/// seeds are wiped when it drops.
pub(crate) use faraday_pgp::Key;

/// A key made here ([`create`]): its record, and what is public of it.
pub struct Made {
    /// The record every vault of the plan takes: the seeds, the dates, the
    /// user ID, the expiry, the certificate and the revocation certificate
    /// kept with it, as GnuPG 2.1 and later keep one.
    pub key: MadeKey,
    /// Its fingerprints, dates and user IDs.
    pub facts: GpgFacts,
    /// The public certificate (packets), signed when it was made.
    pub certificate: Vec<u8>,
}

/// Costs Ed25519 signatures. A new key from the fresh seeds `primary` and
/// `subkey`, made at `now` with one user ID and `expiry` (seconds after
/// creation, 0 for none), with its certificate and the revocation
/// certificate kept with it.
pub fn create(
    primary: &Zeroizing<[u8; 32]>,
    subkey: &Zeroizing<[u8; 32]>,
    now: u32,
    user_id: String,
    expiry: u32,
) -> Made {
    let key = make(primary, subkey, now, user_id, expiry);
    let certificate = certificate(&key, now);
    let rev = Zeroizing::new(revocation(&key, now, Reason::Unspecified, ""));
    let mut r = Record::new(kind::GPG)
        .with(f::PRIMARY, &key.primary_seed)
        .with(f::CREATED, &key.created.to_le_bytes())
        .with(f::SUBKEY, &key.subkey_seed)
        .with(f::SUBKEY_CREATED, &key.subkey_created.to_le_bytes());
    for uid in &key.user_ids {
        r.push(f::USER_ID, uid.as_bytes());
    }
    r.push(f::EXPIRY, &key.expiry.to_le_bytes());
    r.push(f::CERTIFICATE, &certificate);
    r.push(f::REVOCATION, &rev);
    Made {
        key: MadeKey::of(r),
        facts: facts_of(&key),
        certificate,
    }
}

/// What is public of `key`.
pub(crate) fn facts_of(key: &Key) -> GpgFacts {
    GpgFacts {
        fingerprint: key.fingerprint(),
        subkey_fingerprint: key.subkey_fingerprint(),
        created: key.created,
        subkey_created: key.subkey_created,
        user_ids: key.user_ids.clone(),
        expiry: key.expiry,
    }
}

/// The key whose seeds are `primary` and `subkey`, made at `created` and
/// `subkey_created`, with `user_ids` and `expiry` (seconds after
/// creation, 0 for none); `None` unless both seeds are 32 bytes.
pub(crate) fn key_of(
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
pub(crate) fn make(
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
pub(crate) fn certificate(key: &Key, at: u32) -> Vec<u8> {
    key.certificate(at)
}

/// A revocation certificate (packets) for `key`. Costs an Ed25519
/// signature.
pub(crate) fn revocation(key: &Key, at: u32, reason: Reason, text: &str) -> Vec<u8> {
    key.revocation(at, reason, text)
}

/// A revocation certificate as the file GnuPG keeps it in.
pub(crate) fn revocation_file(key: &Key, revocation: &[u8]) -> String {
    key.revocation_file(revocation)
}

/// A detached signature (packets) over `data` by the signing subkey, at
/// `at`, with SHA-512. Costs an Ed25519 signature.
pub(crate) fn sign(key: &Key, data: &[u8], at: u32) -> Vec<u8> {
    key.sign(data, at, Hash::Sha512)
}

/// The signature over `data`, armoured.
pub(crate) fn sign_armored(key: &Key, data: &[u8], at: u32) -> String {
    armor(Armor::Signature, &sign(key, data, at))
}

/// The paperkey text of the key: secret, made only to be shown.
pub(crate) fn paperkey(key: &Key) -> Zeroizing<String> {
    key.paperkey()
}
