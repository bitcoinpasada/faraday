//! Secure Boot's keys are the same bytes whatever else `x509::generate`
//! learns to make: an owner's PK, KEK and db from a fixed seed hash to
//! the values recorded here (those of `ChaCha20Rng`'s streams).

use faraday_sb::{Policy, enrolment, make, pe};
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

#[test]
fn the_signed_updates_and_a_signed_image_are_the_bytes_they_always_were() {
    const NOW: u64 = 1_791_000_000;
    let k = make(&[0x42; 32], "Golden", NOW).unwrap();
    let blind = || faraday_sb::rng::stream(&[0x43; 32], 0);
    // Each .auth file's SHA-256, then the image: the build before
    // `x509-cert` and `der` wrote them.
    let mut got = Vec::new();
    for policy in [Policy::Windows, Policy::OwnKeysOnly] {
        for (name, bytes) in enrolment(&mut blind(), &k, policy, NOW).unwrap() {
            if name.ends_with(".auth") {
                got.push(format!("{policy:?} {name} {}", hex(&bytes)));
            }
        }
    }
    let tiny = include_bytes!("vectors/tiny.efi");
    got.push(format!(
        "signed {}",
        hex(&pe::sign(&mut blind(), tiny, &k.db).unwrap())
    ));
    assert_eq!(
        got,
        [
            "Windows PK.auth 494940e9f8a38f23cc1594b48e2f9700f16413c9aa9810593e7a46ddd4a3d4ed",
            "Windows KEK.auth 4cce333ffbc07d1914d57821edde92c0595c91b7b62cd2847ce531429b928e9c",
            "Windows db.auth b25a27a09b1e7b52add3d89fa969bc92440e7c0ffef277e6cd3500d465c95594",
            "OwnKeysOnly PK.auth 494940e9f8a38f23cc1594b48e2f9700f16413c9aa9810593e7a46ddd4a3d4ed",
            "OwnKeysOnly KEK.auth 7c4c4aa27db00d7ccd4e40ff1f590a50d5045bd0de257943c1668208baa36d46",
            "OwnKeysOnly db.auth eab6322cb69847bbc9c3e5527de51b2464c8ec82f6039c95e0c18ed4481703e1",
            "signed 6fc97119c21d17ab23371a81eb43d47b6e9433217c8f9e3dbf574626c8441341",
        ]
    );
}
