//! Secure Boot keys, their enrolment files and signed images (faraday-sb).

use faraday_sb::Policy;
use faraday_sb::rng::Stream;
use faraday_vault::Record;
use faraday_vault::records::kind;

use crate::vault::{MadeKey, sb_field as f};

pub use faraday_sb::Error;
pub(crate) use faraday_sb::{Keys, Pair};

/// An owner's keys made here ([`create`]): their record, the owner GUID and
/// the enrolment files.
pub struct Made {
    /// The record every vault of the plan takes: the owner GUID and the
    /// PK's, KEK's and db's keys and certificates.
    pub key: MadeKey,
    /// The owner GUID.
    pub owner: [u8; 16],
    /// The enrolment files, by name, which are public.
    pub enrolment: Vec<(String, Vec<u8>)>,
}

/// Costs three RSA key generations and the enrolment's RSA signatures. An
/// owner's three keys from `seed`, named for `owner`, valid from `now`,
/// and their enrolment files under `policy`, the signing blinded from
/// `blind_seed` (both fresh bytes of the system's).
pub fn create(
    seed: &[u8; 32],
    blind_seed: &[u8; 32],
    owner: &str,
    policy: Policy,
    now: u64,
) -> Result<Made, Error> {
    let keys = faraday_sb::make(seed, owner, now)?;
    let enrolment = enrolment(blind_seed, &keys, policy, now)?;
    let record = Record::new(kind::SECURE_BOOT)
        .with(f::OWNER, &keys.owner)
        .with(f::PK_KEY, &keys.pk.key)
        .with(f::PK_CERT, &keys.pk.cert)
        .with(f::KEK_KEY, &keys.kek.key)
        .with(f::KEK_CERT, &keys.kek.cert)
        .with(f::DB_KEY, &keys.db.key)
        .with(f::DB_CERT, &keys.db.cert);
    Ok(Made {
        key: MadeKey::of(record),
        owner: keys.owner,
        enrolment,
    })
}

/// The owner's three keys from a record's parts, when the owner GUID is
/// 16 bytes.
pub(crate) fn keys_of(owner: &[u8], pk: Pair, kek: Pair, db: Pair) -> Option<Keys> {
    Some(Keys {
        owner: owner.try_into().ok()?,
        pk,
        kek,
        db,
    })
}

/// The enrolment files, by name, for `keys` under `policy`, the signing
/// blinded from `blind_seed` (fresh bytes of the system's). Costs RSA
/// signatures.
pub(crate) fn enrolment(
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
pub(crate) fn sign_image(blind_seed: &[u8; 32], pe: &[u8], keys: &Keys) -> Result<Vec<u8>, Error> {
    let mut blind = Stream::new(blind_seed, 0);
    faraday_sb::pe::sign(&mut blind, pe, &keys.db)
}
