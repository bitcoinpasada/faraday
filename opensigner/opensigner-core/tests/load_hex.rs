//! Loading a key from its entropy in hex (`docs/PLANNING.md` §16.125
//! rule 3): the digits are typed on the hex keyboard, the words follow
//! from them and the language, and the key is the key those words give.

mod common;

use common::{Harness, PHONE};
use opensigner_core::load::Step;
use opensigner_core::{ScreenKind, ids};
use osk_ui::widgets::keyboard::KeyInput;

/// 16 zero bytes, whose words are the BIP-39 test mnemonic.
const ZEROS: &str = "00000000000000000000000000000000";

const WORDS: [&str; 12] = [
    "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon",
    "abandon", "abandon", "abandon", "about",
];

fn fingerprints(h: &Harness) -> Vec<String> {
    h.app
        .fingerprints()
        .iter()
        .map(|f| String::from_utf8(f.to_hex().to_vec()).expect("hex"))
        .collect()
}

/// Add › Load a key › "Hex entropy" → 12 words → English.
fn open_hex(h: &mut Harness) {
    h.open_load();
    assert_eq!(h.app.screen(), ScreenKind::Load);
    h.choose(ids::LOAD_SOURCE_HEX, ids::LOAD_SOURCE_CONTINUE);
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    h.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::Words));
}

fn type_digits(h: &mut Harness, digits: &str) {
    for c in digits.chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
}

/// A key loaded from its entropy is the key the same words typed give.
#[test]
fn a_key_loaded_from_hex_entropy_is_the_key_its_words_give() {
    let mut hex = Harness::new(PHONE);
    open_hex(&mut hex);
    type_digits(&mut hex, ZEROS);
    hex.pad(ids::LOAD_KEYBOARD, KeyInput::Done);
    assert_eq!(hex.app.load_step(), Some(Step::Checksum));
    hex.finish_load(None);

    let mut typed = Harness::new(PHONE);
    typed.start_load(&WORDS);
    typed.finish_load(None);

    assert_eq!(fingerprints(&hex), fingerprints(&typed));
    assert_eq!(fingerprints(&hex).len(), 1);
}

/// Short of the digits the word count asks for, the entry does not go
/// on: ✓ is not there to press and a physical Enter takes nothing.
#[test]
fn the_entry_waits_for_every_digit_of_the_entropy() {
    let mut h = Harness::new(PHONE);
    open_hex(&mut h);
    type_digits(&mut h, &ZEROS[..ZEROS.len() - 1]);
    assert!(
        h.app.key_rect(ids::LOAD_KEYBOARD, KeyInput::Done).is_none(),
        "31 digits are not 16 bytes"
    );
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.load_step(), Some(Step::Words));

    type_digits(&mut h, "0");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.load_step(), Some(Step::Checksum));
}
