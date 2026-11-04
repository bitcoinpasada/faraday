//! `faraday-codec` tells a vault by its length, for the programs that
//! list files without opening them: every length Faraday writes counts.

use faraday_codec::vault_file;

#[test]
fn every_size_gives_a_file_the_codec_tells_as_a_vault() {
    for s in faraday_vault::file::SIZES {
        assert!(vault_file::is_vault_len(
            faraday_vault::file::file_len(s) as u64
        ));
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

#[test]
fn a_file_faraday_makes_is_as_long_as_its_size_says_and_the_codec_tells_it() {
    use faraday_vault::Cost;
    let c = compute();
    // Argon2 at the least a reader accepts.
    let cost = Cost {
        memory_kib: 8 * 1024,
        passes: 1,
        lanes: 1,
    };
    for size in faraday_vault::file::SIZES {
        let bytes = faraday_vault::file::create(
            &c,
            [0x5a; 16],
            "Savings",
            cost,
            size,
            "correct horse",
            &std::array::from_fn::<u8, { faraday_vault::kdbx::RANDOM_LEN }, _>(|i| i as u8),
            Some(1_700_000_000),
        )
        .expect("creates");
        assert_eq!(bytes.len(), faraday_vault::file::file_len(size));
        assert!(vault_file::is_vault_len(bytes.len() as u64));
        assert_eq!(vault_file::kdbx_major(&bytes), Some(4));
    }
}
