//! The committed wallet fixtures are what the example still builds.
//!
//! Its own file, because the example shares the PSBT builder with
//! `examples/warnings.rs` and a module is loaded once per test crate.

#[path = "../examples/wallet.rs"]
mod wallet;

use std::path::PathBuf;

/// The registered-wallet fixtures are byte for byte what
/// `examples/wallet.rs` writes today, so the policy a person scans and
/// the transaction it verifies stay one wallet.
#[test]
fn the_committed_wallet_fixtures_are_the_ones_the_example_builds() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/psbt");
    for (name, body) in [
        (
            wallet::POLICY_NAME,
            format!("{}\n", wallet::policy().to_text()),
        ),
        (wallet::PSBT_NAME, wallet::psbt().to_base64()),
        (
            wallet::LIANA_POLICY_NAME,
            format!("{}\n", wallet::liana_policy().to_text()),
        ),
        (wallet::LIANA_PSBT_NAME, wallet::liana_psbt().to_base64()),
        (
            wallet::TREE_POLICY_NAME,
            format!("{}\n", wallet::tree_policy().to_text()),
        ),
        (
            wallet::LIANA_RECOVERY_PSBT_NAME,
            wallet::liana_recovery_psbt().to_base64(),
        ),
        (wallet::TREE_PSBT_NAME, wallet::tree_psbt().to_base64()),
        (
            wallet::TREE_KEYPATH_PSBT_NAME,
            wallet::tree_keypath_psbt().to_base64(),
        ),
    ] {
        let path = dir.join(name);
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e}; run `just psbt-fixtures`", path.display()));
        assert_eq!(
            committed,
            body,
            "{} is not what the example builds; run `just psbt-fixtures`",
            path.display()
        );
    }
}
