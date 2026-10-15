//! Secure Boot's keys are the same bytes whatever else `x509::generate`
//! learns to make: an owner's PK, KEK and db from a fixed seed hash to
//! the values recorded here.

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
    // build before `generate` took a size and a profile.
    assert_eq!(
        got,
        [
            "5fea8de1d74f536b49e9d0514236f017befc4367f32c5319c3014be6b2f40a36",
            "063b93838b24f2e1424c23b6f91d3c31832a48414ee08cf83d1af34f15da7d22",
            "debf4e7253b044a1559b95bd60e2b6c7ec4009e61fdccfc2f3db5cf3fcfb5c73",
            "e588086b8fe4c0d07673b3538f4732c472e51e409f2843661e04c08bff50fdc4",
            "aec17a2d81d0cad2b2063ce1d7ce15be7d86f510ff61e4ea058e5e86702407c2",
            "3b73fcf09c559db92886d69a4a72521f6dfd6a2616393dac7590738018bc75d1",
        ]
    );
}
