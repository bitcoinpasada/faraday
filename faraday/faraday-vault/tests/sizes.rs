//! `faraday-codec` tells a vault by its length, for the programs that
//! list files without opening them: every length Faraday writes counts, up
//! to the ceiling (F206).

use faraday_codec::vault_file;
use faraday_vault::file::{CEILING, STEP, file_len};

#[test]
fn every_length_a_vault_can_have_is_told_as_a_vault_up_to_the_ceiling() {
    for plain in [STEP, 2 * STEP, 1 << 20, 4 << 20, 100 << 20, CEILING] {
        assert!(vault_file::is_vault_len(file_len(plain) as u64), "{plain}");
    }
    assert_eq!(file_len(CEILING), 134_222_685);
}

#[test]
fn a_file_longer_than_the_ceiling_and_an_eighth_is_not_told_as_a_vault() {
    // The stored body may be an eighth over the ceiling, as a compressing
    // program may leave it; the header, its hash and its HMAC come before.
    let most = 313 + CEILING + CEILING / 8;
    assert!(vault_file::is_vault_len(most as u64));
    assert!(!vault_file::is_vault_len(most as u64 + 1));
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

#[test]
fn a_file_faraday_makes_is_one_step_long_and_the_codec_tells_it() {
    use faraday_vault::Cost;
    let c = compute();
    // Argon2 at the least a reader accepts.
    let cost = Cost {
        memory_kib: 8 * 1024,
        passes: 1,
        lanes: 1,
    };
    let bytes = faraday_vault::file::create(
        &c,
        [0x5a; 16],
        "Savings",
        cost,
        "correct horse",
        &std::array::from_fn::<u8, { faraday_vault::kdbx::RANDOM_LEN }, _>(|i| i as u8),
        Some(1_700_000_000),
    )
    .expect("creates");
    assert_eq!(bytes.len(), file_len(STEP));
    assert!(vault_file::is_vault_len(bytes.len() as u64));
    assert_eq!(vault_file::kdbx_major(&bytes), Some(4));
}
