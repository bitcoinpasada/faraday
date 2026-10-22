//! Fingerprints and identity: which key some words, a seed or a record
//! make, worked out from the secret and returned as public values.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::bip32::{DerivationPath, Xpub};
use faraday_bip::keys::{Fingerprint, MasterKey, Network};
use faraday_bip::slip39;
use faraday_crypto::work::Compute;

/// What a master key is made from.
pub enum Material<'a> {
    /// BIP-39 words, with the passphrase given beside them.
    Words(&'a Mnemonic),
    /// SLIP-39 shares, with the passphrase given beside them.
    Slip39(&'a [slip39::Share]),
    /// A master seed, as it is (16 to 64 bytes).
    Seed(&'a [u8]),
}

/// Costs a PBKDF2 (words) or a share recovery (SLIP-39) and a master
/// key. The master key `material` makes with `passphrase` on `network`;
/// `None` for a seed of no valid length or shares that do not recover.
pub(crate) fn master_of(
    c: &Compute,
    material: Material<'_>,
    passphrase: &str,
    network: Network,
) -> Option<MasterKey> {
    match material {
        Material::Words(m) => {
            let seed = m.to_seed(c, passphrase.as_bytes()).ok()?;
            Some(MasterKey::from_seed(c, &seed, network))
        }
        Material::Slip39(shares) => {
            let secret = slip39::recover(c, shares, passphrase.as_bytes()).ok()?;
            let bytes = faraday_crypto::SeedBytes::new(secret.expose().as_bytes())?;
            Some(MasterKey::from_seed_bytes(
                c,
                &faraday_crypto::Secret::new(bytes),
                network,
            ))
        }
        Material::Seed(s) => {
            let bytes = faraday_crypto::SeedBytes::new(s)?;
            Some(MasterKey::from_seed_bytes(
                c,
                &faraday_crypto::Secret::new(bytes),
                network,
            ))
        }
    }
}

/// Costs what `master_of` costs. The fingerprint `material` has with
/// `passphrase`; the same on every network.
pub fn fingerprint(
    c: &Compute,
    material: Material<'_>,
    passphrase: &str,
    network: Network,
) -> Option<Fingerprint> {
    master_of(c, material, passphrase, network).map(|m| m.fingerprint())
}

/// Costs a PBKDF2 and an elliptic-curve multiplication. The fingerprint
/// BIP-39 `words` make with `passphrase`, read in English first and then
/// the other lists in BIP-39's order; the text of the first list's refusal
/// when no list reads them.
pub fn fingerprint_of_words(
    c: &Compute,
    words: &str,
    passphrase: &str,
) -> Result<Fingerprint, String> {
    let normal = zeroize::Zeroizing::new(words.split_whitespace().collect::<Vec<_>>().join(" "));
    let mut first = None;
    for lang in Language::ALL {
        match Mnemonic::parse(c, lang, &normal) {
            Ok(m) => return fingerprint_of_mnemonic(c, &m, passphrase),
            Err(e) => {
                first.get_or_insert(e);
            }
        }
    }
    Err(first.map(|e| e.to_string()).unwrap_or_default())
}

/// Costs a PBKDF2 and an elliptic-curve multiplication. The fingerprint a
/// mnemonic makes with `passphrase`; the text of the refusal when BIP-39
/// refuses the passphrase.
pub fn fingerprint_of_mnemonic(
    c: &Compute,
    m: &Mnemonic,
    passphrase: &str,
) -> Result<Fingerprint, String> {
    let seed = m
        .to_seed(c, passphrase.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(MasterKey::from_seed(c, &seed, Network::Mainnet).fingerprint())
}

/// Costs an HMAC-SHA512 and an elliptic-curve multiplication. The
/// fingerprint a master seed gives; `None` for a seed of no valid length.
pub fn fingerprint_of_seed(c: &Compute, seed: &[u8]) -> Option<Fingerprint> {
    fingerprint(c, Material::Seed(seed), "", Network::Mainnet)
}

/// Costs an elliptic-curve multiplication and a hash. What tells one
/// master key from another in all 32 bytes, the passphrase applied: the
/// SHA-256 of its public key and chain code, the same on every network.
/// Public (a hash of public data).
pub fn master_id(c: &Compute, m: &MasterKey) -> [u8; 32] {
    let x = m.xpub(c);
    let mut bytes = x.public_key.serialize().to_vec();
    bytes.extend_from_slice(x.chain_code.as_bytes());
    faraday_crypto::sha256_whole(c, &bytes)
}

/// Costs what `master_of` and `master_id` cost. The fingerprint and
/// [`master_id`] of what `material` makes with `passphrase`.
pub fn identity(
    c: &Compute,
    material: Material<'_>,
    passphrase: &str,
    network: Network,
) -> Option<(Fingerprint, [u8; 32])> {
    let master = master_of(c, material, passphrase, network)?;
    Some((master.fingerprint(), master_id(c, &master)))
}

/// Costs what `master_of` costs, and a key derivation per step of `path`.
/// The extended public key at `path` below what `material` makes with
/// `passphrase`.
pub fn xpub_at(
    c: &Compute,
    material: Material<'_>,
    passphrase: &str,
    network: Network,
    path: &DerivationPath,
) -> Option<Xpub> {
    let master = master_of(c, material, passphrase, network)?;
    Some(master.derive(c, path).to_xpub(c))
}

/// Costs a PBKDF2 and a key derivation for each step of the path. The
/// public key text `[fingerprint/path]xpub` that BIP-39 `words` (read in
/// English) make with `passphrase`, at `spec`: the key a cosigner hands
/// over, from words that stay with the caller. The text of the refusal when
/// the words are no key.
pub fn key_text_of_words(
    c: &Compute,
    words: &str,
    passphrase: &str,
    network: Network,
    spec: &crate::texts::Spec,
) -> Result<String, String> {
    let normal = zeroize::Zeroizing::new(words.split_whitespace().collect::<Vec<_>>().join(" "));
    let m = Mnemonic::parse(c, Language::English, &normal).map_err(|e| e.to_string())?;
    let master = master_of(c, Material::Words(&m), passphrase, network)
        .ok_or_else(|| "no key".to_string())?;
    crate::texts::key_text(c, &master, spec)
}

/// A hash of secret material under a tag: what a memo of its fingerprint
/// is kept by, so that the secret itself is kept nowhere. Costs a hash of
/// the bytes.
pub fn secret_id(tag: &[u8], secret: &[u8]) -> [u8; 32] {
    use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
    let mut e = sha256::Hash::engine();
    e.input(tag);
    e.input(secret);
    sha256::Hash::from_engine(e).to_byte_array()
}

/// Costs a PBKDF2. The 64-byte BIP-39 seed `m` makes with `passphrase`:
/// what a codex32 backup of the key is made from.
pub(crate) fn seed_of(
    c: &Compute,
    m: &Mnemonic,
    passphrase: &str,
) -> Option<faraday_crypto::Secret<[u8; 64]>> {
    m.to_seed(c, passphrase.as_bytes()).ok()
}

/// The fingerprint as eight hex digits.
pub fn fp_text(fp: Fingerprint) -> String {
    String::from_utf8_lossy(&fp.to_hex()).into_owned()
}

/// Costs a base58 checksum. Whether `text` is a WIF private key: asked to
/// refuse it where a secret may not go, and kept nowhere.
pub fn is_wif(text: &str) -> bool {
    faraday_bip::bitcoin::PrivateKey::from_wif(text).is_ok()
}
