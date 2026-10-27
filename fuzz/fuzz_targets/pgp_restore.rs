//! `faraday_pgp::Certificate::read` and `faraday_pgp::restore`
//! (`docs/GPG-PAPERKEY.md` §4.3): a public key file read as a certificate
//! to restore from, and a restore over arbitrary entries. Whatever reads
//! is of Faraday's shape; a restore that succeeds gives the certificate's
//! own key, whose certificate it makes again; and a Faraday key's own
//! certificate, with its entries changed, restores only to that key.

#![no_main]

use faraday_pgp::{Armor, Certificate, Key, armor, restore};
use libfuzzer_sys::fuzz_target;

/// A restore that succeeds is the key the certificate is for.
fn restores_only_its_key(cert: &Certificate, data: &[u8]) {
    if let Ok(r) = restore(cert, data) {
        assert_eq!(r.key.fingerprint(), cert.fingerprint());
        assert_eq!(r.key.subkey_fingerprint(), cert.subkey_fingerprint());
        assert_eq!(r.certificate, r.key.certificate(r.certified_at));
    }
}

fuzz_target!(|data: &[u8]| {
    // The bytes as a file, and the bytes again as entries against it.
    if let Ok(cert) = Certificate::read(data) {
        assert!(!cert.user_ids().is_empty());
        restores_only_its_key(&cert, data);
    }
    // A Faraday key's certificate, and the bytes as its paperkey's data;
    // then the key's own data with the bytes laid over it.
    let mut key = Key {
        primary_seed: [0x11; 32],
        created: 1_780_000_000,
        subkey_seed: [0x22; 32],
        subkey_created: 1_780_000_000,
        user_ids: vec!["Fuzz <fuzz@example.com>".to_string()],
        expiry: 0,
    };
    if let Some(&b) = data.first() {
        key.primary_seed[0] = b;
    }
    let cert = Certificate::read(armor(Armor::PublicKey, &key.certificate(key.created)).as_bytes())
        .expect("a certificate of Faraday's shape");
    restores_only_its_key(&cert, data);
    let own = faraday_pgp::paperkey::read_text(&key.paperkey()).expect("its own lines");
    let mut changed = own.to_vec();
    for (i, b) in data.iter().skip(1).enumerate() {
        let at = i % changed.len();
        changed[at] ^= b;
    }
    restores_only_its_key(&cert, &changed);
    if changed[..] != own[..] {
        // Any other secret is refused or is this key's seed again.
        if let Ok(r) = restore(&cert, &changed) {
            assert_eq!(r.key.primary_seed, key.primary_seed);
            assert_eq!(r.key.subkey_seed, key.subkey_seed);
        }
    }
});
