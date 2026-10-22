//! What the test modules share.

use faraday_bip::bitcoin::bip32::Xpriv;
use faraday_bip::bitcoin::secp256k1::Secp256k1;
use faraday_bip::bitcoin::{Network, NetworkKind};
use faraday_core::samples::SampleKey;
use faraday_samples::SAMPLES;

pub const PASSWORD: &str = "password";

/// The sample whose parts are exactly `name`'s, the `n`th of that name.
pub fn key(name: &str) -> &'static SampleKey {
    SAMPLES
        .keys
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("no sample named {name}"))
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A vector file of `tools/vectors`, read when the test runs.
pub fn vector(path: &str) -> String {
    let at = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors")
        .join(path);
    std::fs::read_to_string(&at).unwrap_or_else(|e| panic!("{}: {e}", at.display()))
}

pub fn json(path: &str) -> serde_json::Value {
    serde_json::from_str(&vector(path)).expect("a json vector file")
}

/// The fingerprint of the BIP-32 master key of `seed`, by the `bitcoin`
/// crate alone: the first four bytes of the key's identifier.
#[expect(
    clippy::disallowed_methods,
    reason = "another implementation's master key, to check Faraday's against (docs/COMPUTE.md §7)"
)]
pub fn fingerprint_of_seed(seed: &[u8]) -> [u8; 4] {
    let secp = Secp256k1::new();
    let master = Xpriv::new_master(NetworkKind::Main, seed).expect("a master key");
    master.fingerprint(&secp).to_bytes()
}

/// The master key of `seed`, written as an `xprv`, by the `bitcoin` crate.
#[expect(
    clippy::disallowed_methods,
    reason = "another implementation's master key, to check Faraday's against (docs/COMPUTE.md §7)"
)]
pub fn xprv_of_seed(seed: &[u8]) -> String {
    Xpriv::new_master(Network::Bitcoin, seed)
        .expect("a master key")
        .to_string()
}

/// The fingerprint BIP-39 `words` have with `passphrase`, by the `bip39`
/// crate and the `bitcoin` crate, with none of Faraday's code. `words` are
/// in `lang`, in the form (published or NFKD) the crate reads.
pub fn reference_fingerprint(lang: bip39::Language, words: &str, passphrase: &str) -> [u8; 4] {
    let m = bip39::Mnemonic::parse_in_normalized(lang, words).expect("valid words");
    fingerprint_of_seed(&m.to_seed_normalized(passphrase))
}

/// The entropy a `bip39` mnemonic encodes.
pub fn entropy_of(m: &bip39::Mnemonic) -> Vec<u8> {
    let (bytes, len) = m.to_entropy_array();
    bytes[..len].to_vec()
}
