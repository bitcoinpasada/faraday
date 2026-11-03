//! The keys of a KDBX file (`docs/VAULT.md` §1.1) and the HMAC the
//! header and the block stream are authenticated with.

use alloc::vec::Vec;

use argon2::{Algorithm, Argon2, Block, Params, Version};
use faraday_crypto::work::{Compute, Work};
use faraday_crypto::{Secret, Zeroize};
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256, Sha512};
use zeroize::Zeroizing;

use crate::Cost;

use super::Error;
use super::header::{Kdf, cost_in_limits};

/// The block index the header's own HMAC is computed under.
pub(super) const HEADER_BLOCK: u64 = u64::MAX;

/// The two keys a file needs: the one the body is encrypted under and
/// the base every HMAC key is derived from.
pub(super) struct Keys {
    /// The body's cipher key.
    pub encryption: Secret<[u8; 32]>,
    /// `SHA-512(master_seed || T || 0x01)`.
    pub hmac_base: Secret<[u8; 64]>,
}

/// Costs an Argon2 run: `cost.memory_kib` of memory filled `cost.passes`
/// times. The keys of a file with this master seed, salt, passphrase
/// (its NFC form's bytes) and cost, the cost checked against §3's limits
/// before any memory is asked for.
pub(super) fn file_keys(
    c: &Compute,
    passphrase: &[u8],
    master_seed: &[u8],
    kdf: Kdf,
    cost: &Cost,
    salt: &[u8],
) -> Result<Keys, Error> {
    cost_in_limits(cost)?;
    // The composite key: the SHA-256 of the components' SHA-256s, and the
    // one component is the passphrase's.
    let first = Zeroizing::new(<[u8; 32]>::from(Sha256::digest(passphrase)));
    let mut composite = Secret::new(<[u8; 32]>::from(Sha256::digest(first.as_slice())));
    let stretched = argon_run(c, kdf, cost, composite.expose(), salt);
    composite.zeroize();
    let mut t = stretched?;

    let mut encryption = Sha256::new();
    encryption.update(master_seed);
    encryption.update(t.expose());
    let encryption = Secret::new(<[u8; 32]>::from(encryption.finalize()));
    let mut base = Sha512::new();
    base.update(master_seed);
    base.update(t.expose());
    base.update([1u8]);
    let hmac_base = Secret::new(<[u8; 64]>::from(base.finalize()));
    t.zeroize();
    Ok(Keys {
        encryption,
        hmac_base,
    })
}

/// Costs an Argon2 run, asked for so that a cost this machine cannot
/// meet is an error. Argon2(composite, salt) at `cost`, 32 bytes.
fn argon_run(
    c: &Compute,
    kdf: Kdf,
    cost: &Cost,
    composite: &[u8],
    salt: &[u8],
) -> Result<Secret<[u8; 32]>, Error> {
    c.count(Work::Kdf);
    let algorithm = match kdf {
        Kdf::Argon2id => Algorithm::Argon2id,
        Kdf::Argon2d => Algorithm::Argon2d,
    };
    let params =
        Params::new(cost.memory_kib, cost.passes, cost.lanes, Some(32)).map_err(|_| Error::Cost)?;
    let argon = Argon2::new(algorithm, Version::V0x13, params.clone());
    let blocks = params.block_count();
    let mut memory: Vec<Block> = Vec::new();
    memory
        .try_reserve_exact(blocks)
        .map_err(|_| Error::Memory(cost.memory_kib / 1024))?;
    memory.resize(blocks, Block::default());
    let mut derived = Secret::new([0u8; 32]);
    let hashed = argon
        .hash_password_into_with_memory(composite, salt, derived.expose_mut(), &mut memory)
        .is_ok();
    // What the memory holds is derived from the passphrase.
    memory.fill(Block::default());
    core::hint::black_box(&memory);
    if hashed {
        Ok(derived)
    } else {
        Err(Error::Cost)
    }
}

/// `SHA-512(i || base)`: block `i`'s HMAC key, and the header's when `i`
/// is [`HEADER_BLOCK`].
pub(super) fn block_key(base: &Secret<[u8; 64]>, i: u64) -> Secret<[u8; 64]> {
    let mut h = Sha512::new();
    h.update(i.to_le_bytes());
    h.update(base.expose());
    Secret::new(<[u8; 64]>::from(h.finalize()))
}

/// A running `HMAC-SHA-256` under `key`.
fn mac(key: &Secret<[u8; 64]>, parts: &[&[u8]]) -> Hmac<Sha256> {
    let mut mac =
        <Hmac<Sha256> as KeyInit>::new_from_slice(key.expose()).expect("HMAC takes any key length");
    for part in parts {
        mac.update(part);
    }
    mac
}

/// `HMAC-SHA-256(key, parts concatenated)`.
pub(super) fn hmac(key: &Secret<[u8; 64]>, parts: &[&[u8]]) -> [u8; 32] {
    mac(key, parts).finalize().into_bytes().into()
}

/// Whether `tag` is `HMAC-SHA-256(key, parts concatenated)`, compared in
/// constant time.
pub(super) fn hmac_ok(key: &Secret<[u8; 64]>, parts: &[&[u8]], tag: &[u8]) -> bool {
    mac(key, parts).verify_slice(tag).is_ok()
}
