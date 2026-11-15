//! The keys of a KDBX file (`docs/VAULT.md` §1.1) and the HMAC the
//! header and the block stream are authenticated with.

use alloc::boxed::Box;
use argon2::Algorithm;
use faraday_crypto::work::Compute;
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

/// What a file needs besides its passphrase (`docs/VAULT.md` §3.2, F208):
/// a key file's key. It is secret bytes and wipes as it drops; it has no
/// reveal outside this crate, and no second copy is kept; each seal makes
/// a short-lived copy ([`Components::changed`]) that wipes as it drops.
#[derive(Default)]
pub struct Components {
    /// A key file's 32-byte key ([`super::keyfile::read`]).
    key_file: Option<Zeroizing<[u8; 32]>>,
}

impl Components {
    /// The passphrase alone.
    pub fn none() -> Self {
        Self::default()
    }

    /// A key file's key alone.
    pub fn with_key_file(key: Zeroizing<[u8; 32]>) -> Self {
        Self {
            key_file: Some(key),
        }
    }

    /// Whether a key file's key is held.
    pub fn has_key_file(&self) -> bool {
        self.key_file.is_some()
    }

    /// What these become at a seal that changes them: `key_file` is
    /// `Some(None)` to take the key file out, `Some(Some(key))` to add or
    /// replace it and `None` to keep it.
    #[expect(
        clippy::disallowed_methods,
        reason = "the components are copies in wiping memory (`Zeroizing`)"
    )]
    pub fn changed(&self, key_file: Option<Option<&[u8; 32]>>) -> Components {
        Components {
            key_file: match key_file {
                None => self.key_file.clone(),
                Some(k) => k.map(|k| Zeroizing::new(*k)),
            },
        }
    }
}

/// The composite key (§3.2), the one place it is made: the SHA-256 of
/// `SHA-256(passphrase)`, then the key file's key if there is one.
pub(super) fn composite_key(passphrase: &[u8], components: &Components) -> Secret<[u8; 32]> {
    let mut all = Sha256::new();
    let mut first = Zeroizing::new([0u8; 32]);
    let mut inner = Sha256::new();
    inner.update(passphrase);
    inner.finalize_into((&mut *first).into());
    all.update(first.as_slice());
    if let Some(key) = &components.key_file {
        all.update(key.as_slice());
    }
    let mut out = Secret::new([0u8; 32]);
    all.finalize_into(out.expose_mut().into());
    out
}

/// Costs an Argon2 run: `cost.memory_kib` of memory filled `cost.passes`
/// times. The keys of a file with this master seed, salt, passphrase
/// (its NFC form's bytes), components and cost, the cost checked against
/// §3's limits before any memory is asked for.
pub(super) fn file_keys(
    c: &Compute,
    passphrase: &[u8],
    components: &Components,
    master_seed: &[u8],
    kdf: Kdf,
    cost: &Cost,
    salt: &[u8],
) -> Result<Box<Keys>, Error> {
    cost_in_limits(cost)?;
    let mut composite = composite_key(passphrase, components);
    let stretched = argon_run(c, kdf, cost, composite.expose(), salt);
    composite.zeroize();
    let mut t = stretched?;

    // The keys are made in place in their heap holder and only the pointer
    // moves out: no by-value move of the key bytes leaves a copy on the stack.
    let mut keys = Box::new(Keys {
        encryption: Secret::new([0u8; 32]),
        hmac_base: Secret::new([0u8; 64]),
    });
    let mut encryption = Sha256::new();
    encryption.update(master_seed);
    encryption.update(t.expose());
    encryption.finalize_into(keys.encryption.expose_mut().into());
    let mut base = Sha512::new();
    base.update(master_seed);
    base.update(t.expose());
    base.update([1u8]);
    base.finalize_into(keys.hmac_base.expose_mut().into());
    t.zeroize();
    Ok(keys)
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
    let algorithm = match kdf {
        Kdf::Argon2id => Algorithm::Argon2id,
        Kdf::Argon2d => Algorithm::Argon2d,
    };
    let cost_in = faraday_crypto::Argon2Cost {
        memory_kib: cost.memory_kib,
        passes: cost.passes,
        lanes: cost.lanes,
    };
    let mut derived = Secret::new([0u8; 32]);
    faraday_crypto::argon2(c, algorithm, composite, salt, cost_in, &mut derived).map_err(|e| {
        match e {
            faraday_crypto::Argon2Error::Memory => Error::Memory(cost.memory_kib / 1024),
            faraday_crypto::Argon2Error::Params => Error::Cost,
        }
    })?;
    Ok(derived)
}

/// `SHA-512(i || base)`: block `i`'s HMAC key, and the header's when `i`
/// is [`HEADER_BLOCK`].
pub(super) fn block_key(base: &Secret<[u8; 64]>, i: u64) -> Secret<[u8; 64]> {
    let mut h = Sha512::new();
    h.update(i.to_le_bytes());
    h.update(base.expose());
    let mut out = Secret::new([0u8; 64]);
    h.finalize_into(out.expose_mut().into());
    out
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
