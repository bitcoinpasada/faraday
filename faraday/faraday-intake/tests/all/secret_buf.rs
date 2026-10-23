//! Text typed that may be a secret.

use crate::common::bip39_english;
use faraday_intake::secret_buf::{SecretText, hex};

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
fn backspace_takes_off_the_last_character_and_a_replacement_keeps_only_the_new_text() {
    let mut t = SecretText::of("zoo wrong");
    assert_eq!(t.pop(), Some('g'));
    assert_eq!(t, "zoo wron");
    t.set("legal");
    assert_eq!(t, "legal");
    t.clear();
    assert!(t.is_empty());
    assert_eq!(t.pop(), None);
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
