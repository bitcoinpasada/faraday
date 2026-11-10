//! The keys of a KDBX file (`docs/VAULT.md` §1.1) and the HMAC the
//! header and the block stream are authenticated with.

use argon2::Algorithm;
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, Zeroize};
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
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

/// What a file needs besides its passphrase (`docs/VAULT.md` §3.2, F208).
/// Both are secret bytes and wipe as they drop; neither has a reveal
/// outside this crate.
#[derive(Default, Clone)]
pub struct Components {
    /// A key file's 32-byte key ([`super::keyfile::read`]).
    key_file: Option<Zeroizing<[u8; 32]>>,
    /// A YubiKey's 20-byte HMAC-SHA-1 secret ([`super::keyfile::secret`]).
    yubikey: Option<Zeroizing<[u8; 20]>>,
}

impl Components {
    /// The passphrase alone.
    pub fn none() -> Self {
        Self::default()
    }

    /// A key file's key and, if there is one, a YubiKey's secret.
    pub fn new(
        key_file: Option<Zeroizing<[u8; 32]>>,
        yubikey: Option<Zeroizing<[u8; 20]>>,
    ) -> Self {
        Self { key_file, yubikey }
    }

    /// A key file's key alone.
    pub fn with_key_file(key: Zeroizing<[u8; 32]>) -> Self {
        Self::new(Some(key), None)
    }

    /// A YubiKey's secret alone.
    pub fn with_yubikey(secret: Zeroizing<[u8; 20]>) -> Self {
        Self::new(None, Some(secret))
    }

    /// Whether a key file's key is held.
    pub fn has_key_file(&self) -> bool {
        self.key_file.is_some()
    }

    /// Whether a YubiKey's secret is held.
    pub fn has_yubikey(&self) -> bool {
        self.yubikey.is_some()
    }

    /// What these become at a seal that changes them: `key_file` is
    /// `Some(None)` to take the key file out, `Some(Some(key))` to add or
    /// replace it and `None` to keep it; `drop_yubikey` takes the secret
    /// out. A secret is never added (F208 point 5).
    pub fn changed(&self, key_file: Option<Option<&[u8; 32]>>, drop_yubikey: bool) -> Components {
        Components {
            key_file: match key_file {
                None => self.key_file.clone(),
                Some(k) => k.map(|k| Zeroizing::new(*k)),
            },
            yubikey: if drop_yubikey {
                None
            } else {
                self.yubikey.clone()
            },
        }
    }

    /// What `salt` is changed to at a seal: a salt ending `0x20` becomes
    /// one ending `0x21` when a YubiKey secret is held, so no seal rests on
    /// the key's padding rule (F208 point 6). The bytes are the file's own
    /// otherwise.
    pub(super) fn fit_salt(&self, salt: &mut [u8]) {
        if self.yubikey.is_some()
            && let Some(last) = salt.last_mut()
            && *last == 0x20
        {
            *last = 0x21;
        }
    }
}

/// `HMAC-SHA-1(secret, message)` where the message is the challenge `salt`
/// as a key programmed for variable-length input takes it: the salt padded
/// to 64 bytes with `64 - len` bytes of that value, less its last byte and
/// every equal byte before it (§3.2).
fn yubikey_response(secret: &[u8; 20], salt: &[u8]) -> Zeroizing<[u8; 20]> {
    let mut padded = Zeroizing::new([0u8; 64]);
    let len = salt.len().min(64);
    padded[..len].copy_from_slice(&salt[..len]);
    padded[len..].fill((64 - len) as u8);
    let last = padded[63];
    let mut n = 64;
    while n > 0 && padded[n - 1] == last {
        n -= 1;
    }
    let mut mac =
        <Hmac<Sha1> as KeyInit>::new_from_slice(secret).expect("HMAC takes any key length");
    mac.update(&padded[..n]);
    Zeroizing::new(mac.finalize().into_bytes().into())
}

/// The composite key (§3.2), the one place it is made: the SHA-256 of
/// `SHA-256(passphrase)`, then the key file's key if there is one, then
/// `SHA-256(response)` if there is a secret, `salt` being the challenge.
pub(super) fn composite_key(
    passphrase: &[u8],
    components: &Components,
    salt: &[u8],
) -> Secret<[u8; 32]> {
    let mut all = Sha256::new();
    let first = Zeroizing::new(<[u8; 32]>::from(Sha256::digest(passphrase)));
    all.update(first.as_slice());
    if let Some(key) = &components.key_file {
        all.update(key.as_slice());
    }
    if let Some(secret) = &components.yubikey {
        let response = yubikey_response(secret, salt);
        let hashed = Zeroizing::new(<[u8; 32]>::from(Sha256::digest(response.as_slice())));
        all.update(hashed.as_slice());
    }
    Secret::new(<[u8; 32]>::from(all.finalize()))
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
) -> Result<Keys, Error> {
    cost_in_limits(cost)?;
    let mut composite = composite_key(passphrase, components, salt);
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
