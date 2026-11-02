//! Text typed that may be a secret.

use crate::common::bip39_english;
use faraday_intake::secret_buf::{SecretText, hex};

#[test]
fn a_secret_never_prints_what_it_holds() {
    let t = SecretText::of("abandon abandon about");
    let shown = format!("{t:?}");
    assert!(!shown.contains("abandon"), "{shown}");
}

#[test]
fn a_private_key_written_as_hex_matches_the_published_entropy() {
    for [entropy_hex, _, _, _] in bip39_english() {
        let raw = crate::common::unhex(&entropy_hex);
        assert_eq!(hex(&raw).as_str(), entropy_hex);
    }
}
