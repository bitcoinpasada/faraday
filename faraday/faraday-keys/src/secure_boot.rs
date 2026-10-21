//! Secure Boot keys, their enrolment files and signed images (faraday-sb).

use faraday_sb::Policy;
use faraday_sb::rng::Stream;

pub use faraday_sb::{Error, Keys, Pair};

/// The owner's three keys from a record's parts, when the owner GUID is
/// 16 bytes.
pub fn keys_of(owner: &[u8], pk: Pair, kek: Pair, db: Pair) -> Option<Keys> {
    Some(Keys {
        owner: owner.try_into().ok()?,
        pk,
        kek,
        db,
    })
}

/// Makes the three keys from `seed`, named for `owner`, valid from `now`.
/// Costs three RSA key generations.
pub fn make(seed: &[u8; 32], owner: &str, now: u64) -> Result<Keys, Error> {
    faraday_sb::make(seed, owner, now)
}

/// The enrolment files, by name, for `keys` under `policy`, the signing
/// blinded from `blind_seed` (fresh bytes of the system's). Costs RSA
/// signatures.
pub fn enrolment(
    blind_seed: &[u8; 32],
    keys: &Keys,
    policy: Policy,
    now: u64,
) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let mut blind = Stream::new(blind_seed, 0);
    faraday_sb::enrolment(&mut blind, keys, policy, now)
}

/// The image `pe` signed by the db key, the signing blinded from
/// `blind_seed`. Costs an RSA signature.
pub fn sign_image(blind_seed: &[u8; 32], pe: &[u8], keys: &Keys) -> Result<Vec<u8>, Error> {
    let mut blind = Stream::new(blind_seed, 0);
    faraday_sb::pe::sign(&mut blind, pe, &keys.db)
}
