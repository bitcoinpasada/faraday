//! Text typed that may be a secret.

use crate::common::bip39_english;
use faraday_crypto::hex::encode_secret as hex;
use faraday_intake::secret_buf::SecretText;

#[test]
fn a_phrase_typed_a_character_at_a_time_is_kept_whole_however_long() {
    // Longer than the room it starts with.
    let phrase = "abandon ".repeat(200);
    let mut typed = SecretText::new();
    for c in phrase.chars() {
        typed.push(c);
    }
    assert_eq!(typed, phrase.as_str());
}

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
