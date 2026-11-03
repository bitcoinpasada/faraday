//! Secure Boot's keys are the same bytes whatever else `x509::generate`
//! learns to make: an owner's PK, KEK and db from a fixed seed hash to
//! the values recorded here (those of `ChaCha20Rng`'s streams).

use faraday_sb::make;
use sha2::{Digest, Sha256};

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn an_owners_secure_boot_keys_are_the_bytes_they_always_were() {
    let k = make(&[0x42; 32], "Golden", 1_791_000_000).unwrap();
    let got: Vec<String> = [&k.pk, &k.kek, &k.db]
        .iter()
        .flat_map(|p| [hex(&p.key), hex(&p.cert)])
        .collect();
    // Each key's and certificate's SHA-256, PK first, taken from the
    // build with `ChaCha20Rng` as the generator.
    assert_eq!(
        got,
        [
            "f6b2293b639f9d64c593ae6a08cdda9d6d411b464ce7bb3f1dc9e4f3a2dc96d8",
            "bc897af56df79fed9b6062eda24cbab738acce1e50256802e32556138d8f8861",
            "1c2dfe7f7e9cebee7c7bfbc2de83c53f4f17e9177482547924a53517772651e6",
            "8fb666b5bfb2ea59690835a82423c324e433e805c53422240b40c216b26b33e5",
            "e105b0fa3ac2d58ef348a5d3951b7d051842da7c12a3964b82206915f2acd31e",
            "cb051f5d487219797f4aaaebd15192b1c0c154e709334bfab3cc932b913efc26",
        ]
    );
}
