//! The test data a journey or an update test loads is made whole.

use faraday_testkit::{kits, musig_unsigned, spend, test_efi, test_vault, unsigned};

/// Invariant: Every test wallet has an unsigned spend, the Savings cosigner's copy carries a signature the unsigned one lacks, and the MuSig2 spend and the test vault are made.
/// Regression: A change to the domain crates leaves a test wallet or the test vault unbuildable, and every test that loads it fails far from the cause.
/// Owner: domain
#[test]
fn the_test_data_is_made() {
    let kits = kits();
    assert_eq!(kits.len(), 10);
    for kit in &kits {
        assert!(
            unsigned(kit).is_ok_and(|bytes| !bytes.is_empty()),
            "{}",
            kit.id
        );
    }
    let (unsigned, cosigned) = spend().expect("the Savings spend");
    assert_ne!(unsigned, cosigned);
    assert!(cosigned.len() > unsigned.len());
    assert!(musig_unsigned().is_ok());
    assert!(test_vault().is_ok_and(|file| !file.is_empty()));
    assert!(!test_efi().is_empty());
}
