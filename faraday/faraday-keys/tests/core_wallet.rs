//! The private keys of Bitcoin Core's own descriptor wallets
//! (`tools/vectors/core-wallet/`): each key passes, a wrong passphrase is
//! its own outcome, and a changed byte fails the key.

use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_keys::core_wallet::{KeyVerdict, OpenError, check_encrypted_keys, check_private_key};
use zeroize::Zeroizing;

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

/// The records of a wallet whose key starts with the record type `name`:
/// the public key its key names and its value, and the `mkey` value.
type Pairs = Vec<(Vec<u8>, Vec<u8>)>;

fn records(wallet: &str, name: &str) -> (Pairs, Vec<u8>) {
    let path = format!(
        "{}/../../tools/vectors/core-wallet/{wallet}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("dump")).expect("json");
    let mut keys = Vec::new();
    let mut mkey = Vec::new();
    for r in json["records"].as_array().expect("records") {
        let key = unhex(r[0].as_str().expect("hex"));
        let value = unhex(r[1].as_str().expect("hex"));
        let (len, rest) = (usize::from(key[0]), &key[1..]);
        let (kind, tail) = rest.split_at(len);
        if kind == b"mkey" {
            mkey = value;
        } else if kind == name.as_bytes() {
            // 32 bytes of descriptor id, then the public key as a vector.
            assert!(matches!(tail[32], 33 | 65));
            keys.push((tail[33..].to_vec(), value));
        }
    }
    (keys, mkey)
}

#[test]
fn core_wallet_the_keys_of_a_plain_wallet_pass() {
    let c = compute();
    let (keys, _) = records("plain", "walletdescriptorkey");
    assert!(!keys.is_empty());
    for (pubkey, value) in keys {
        assert_eq!(
            check_private_key(&c, &pubkey, Zeroizing::new(value)),
            KeyVerdict::Pass
        );
    }
}

#[test]
fn core_wallet_a_changed_byte_fails_a_plain_wallets_key() {
    let c = compute();
    let (keys, _) = records("plain", "walletdescriptorkey");
    let (pubkey, value) = keys[0].clone();
    // In the private key, in the stored hash and in the public key named.
    for at in [20, value.len() - 5] {
        let mut bad = value.clone();
        bad[at] ^= 1;
        assert_ne!(
            check_private_key(&c, &pubkey, Zeroizing::new(bad)),
            KeyVerdict::Pass
        );
    }
    let mut other = pubkey.clone();
    other[10] ^= 1;
    assert_ne!(
        check_private_key(&c, &other, Zeroizing::new(value)),
        KeyVerdict::Pass
    );
}

#[test]
fn core_wallet_an_encrypted_wallets_keys_open_under_its_passphrase() {
    let c = compute();
    let (keys, mkey) = records("encrypted", "walletdescriptorckey");
    let keys: Vec<_> = keys
        .into_iter()
        .map(|(p, v)| (p, Zeroizing::new(v)))
        .collect();
    let count = keys.len();
    assert!(count > 0);
    let right = SecretText::of("faraday");
    let verdicts = check_encrypted_keys(&c, &mkey, &right, keys.clone()).expect("opens");
    assert_eq!(verdicts, vec![KeyVerdict::Pass; count]);

    let wrong = SecretText::of("faraday ");
    assert_eq!(
        check_encrypted_keys(&c, &mkey, &wrong, keys),
        Err(OpenError::WrongPassphrase)
    );
}

#[test]
fn core_wallet_a_changed_byte_in_an_encrypted_key_fails_that_key() {
    let c = compute();
    let (keys, mkey) = records("encrypted", "walletdescriptorckey");
    let mut keys: Vec<_> = keys
        .into_iter()
        .map(|(p, v)| (p, Zeroizing::new(v)))
        .collect();
    keys[0].1[5] ^= 1;
    let right = SecretText::of("faraday");
    let verdicts = check_encrypted_keys(&c, &mkey, &right, keys).expect("opens");
    assert_ne!(verdicts[0], KeyVerdict::Pass);
    assert!(verdicts[1..].iter().all(|v| *v == KeyVerdict::Pass));
}

#[test]
fn core_wallet_an_uncompressed_key_passes_and_a_changed_byte_fails_it() {
    let c = compute();
    let (keys, _) = records("uncompressed", "walletdescriptorkey");
    let (uncompressed, compressed): (Vec<_>, Vec<_>) =
        keys.into_iter().partition(|(pubkey, _)| pubkey.len() == 65);
    assert_eq!(uncompressed.len(), 1);
    assert!(!compressed.is_empty());
    let (pubkey, value) = uncompressed[0].clone();
    // The value is the 279-byte DER (a 3-byte length), then the hash.
    assert_eq!(value.len(), 3 + 279 + 32);
    assert_eq!(
        check_private_key(&c, &pubkey, Zeroizing::new(value.clone())),
        KeyVerdict::Pass
    );
    for at in [20, 100, value.len() - 5] {
        let mut bad = value.clone();
        bad[at] ^= 1;
        assert_ne!(
            check_private_key(&c, &pubkey, Zeroizing::new(bad)),
            KeyVerdict::Pass,
            "{at}"
        );
    }
    // The compressed form of the same record is not the shape.
    assert_eq!(
        check_private_key(&c, &pubkey[..33], Zeroizing::new(value)),
        KeyVerdict::Malformed
    );
}

#[test]
fn core_wallet_a_private_key_with_a_right_hash_and_another_public_key_fails() {
    let c = compute();
    let (keys, _) = records("plain", "walletdescriptorkey");
    let (pubkey, value) = keys[0].clone();
    // [214][DER][hash]: change the private key (at 1 + 8), then store
    // the hash of the public key and that DER, as Core would for it.
    let mut bad = value.clone();
    bad[1 + 8 + 5] ^= 1;
    let hash = faraday_crypto::sha256(&faraday_crypto::sha256(
        &[&pubkey[..], &bad[1..215]].concat(),
    ));
    bad[215..].copy_from_slice(&hash);
    assert_eq!(
        check_private_key(&c, &pubkey, Zeroizing::new(bad)),
        KeyVerdict::PublicDiffers
    );
}
