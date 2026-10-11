//! Loading a key by the numbers of its words (`docs/PLANNING.md`
//! §16.125 rule 2): the numbers a backup written in numbers carries are
//! typed on the digit pad, the word each one names is tapped, and the
//! key that lands is the key the same words give.

mod common;

use common::{Harness, PHONE};
use opensigner_core::load::Step;
use opensigner_core::strings::EN;
use opensigner_core::{ScreenKind, ids};
use osk_bip::bip39::Language;
use osk_shell_api::Key;
use osk_ui::widgets::keyboard::KeyInput;

/// The BIP-39 test mnemonic, whose last word the checksum fixes.
const WORDS: [&str; 12] = [
    "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon",
    "abandon", "abandon", "abandon", "about",
];

/// The 1-based place of `word` on the English list, which is the number a
/// backup in numbers and the Backup › Numbers screen both print.
fn number(word: &str) -> u16 {
    Language::English
        .index_of(word)
        .unwrap_or_else(|| panic!("{word} is not an English BIP-39 word"))
        + 1
}

fn fingerprints(h: &Harness) -> Vec<String> {
    h.app
        .fingerprints()
        .iter()
        .map(|f| String::from_utf8(f.to_hex().to_vec()).expect("hex"))
        .collect()
}

/// Add › Load a key › "Word numbers" → 12 words → English.
fn open_numbers(h: &mut Harness) {
    h.open_load();
    assert_eq!(h.app.screen(), ScreenKind::Load);
    h.choose(ids::LOAD_SOURCE_NUMBERS, ids::LOAD_SOURCE_CONTINUE);
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    h.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::Words));
}

/// Types one word's number on the digit pad and taps the word it names.
fn type_number(h: &mut Harness, word: &str) {
    for c in number(word).to_string().chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    assert_eq!(
        h.app.candidates(),
        vec![String::from(word)],
        "the number names {word}"
    );
    h.tap_candidate(0, word);
}

/// A key typed as numbers is the key the same words typed give.
#[test]
fn a_key_typed_as_numbers_is_the_key_its_words_give() {
    let mut numbers = Harness::new(PHONE);
    open_numbers(&mut numbers);
    for w in WORDS {
        type_number(&mut numbers, w);
    }
    assert_eq!(numbers.app.load_step(), Some(Step::Checksum));
    numbers.finish_load(None);

    let mut typed = Harness::new(PHONE);
    typed.start_load(&WORDS);
    typed.finish_load(None);

    assert_eq!(fingerprints(&numbers), fingerprints(&typed));
    assert_eq!(fingerprints(&numbers).len(), 1);
}

/// A digit that cannot lead to a number on the list is not there to
/// press, and typing it changes nothing: `0` on an empty field, and any
/// digit after a number past 204.
#[test]
fn a_digit_that_cannot_name_a_word_does_nothing() {
    let mut h = Harness::new(PHONE);
    open_numbers(&mut h);
    assert!(
        h.app
            .key_rect(ids::LOAD_KEYBOARD, KeyInput::Char('0'))
            .is_none(),
        "0 names no word"
    );
    h.type_text("0");
    assert!(h.app.candidates().is_empty(), "nothing was typed");

    h.type_text("205");
    assert!(
        h.app
            .key_rect(ids::LOAD_KEYBOARD, KeyInput::Char('0'))
            .is_none(),
        "2050 is past the list"
    );
    h.type_text("0");
    assert_eq!(
        h.app.candidates(),
        vec![String::from(Language::English.word(204))],
        "the field still holds 205"
    );
}

/// Backspace on an empty number field steps back to the previous word,
/// with its own number in the field, as the typed-words entry does.
#[test]
fn backspace_on_an_empty_number_field_steps_back_a_word() {
    let mut h = Harness::new(PHONE);
    open_numbers(&mut h);
    type_number(&mut h, "abandon");
    type_number(&mut h, "ability");
    assert_eq!(h.app.load_words_accepted(), 2);

    h.key(Key::Backspace);
    assert_eq!(h.app.load_words_accepted(), 1, "the second word came back");
    assert_eq!(
        h.app.candidates(),
        vec![String::from("ability")],
        "its number is in the field"
    );
}

/// "Load from?" lists its rows most used first, with the two new sources
/// above the dimmed secure element (§16.125 rule 1).
#[test]
fn the_source_rows_are_listed_most_used_first() {
    let mut h = Harness::new(PHONE);
    h.open_load();
    let texts = h.app.texts();
    let order = [
        EN.load_source_type,
        EN.load_source_scan,
        EN.load_source_backup,
        EN.load_source_slip39,
        EN.load_source_xor,
        EN.load_source_codex32,
        EN.load_source_numbers,
        EN.load_source_hex,
        EN.load_source_element,
    ];
    let places: Vec<usize> = order
        .iter()
        .map(|row| {
            texts
                .iter()
                .position(|t| t == row)
                .unwrap_or_else(|| panic!("{row} is not on the screen: {texts:?}"))
        })
        .collect();
    let mut sorted = places.clone();
    sorted.sort_unstable();
    assert_eq!(places, sorted, "the rows are in order: {texts:?}");
}

/// Word numbers' pad has no ✓: a word is taken by a tap on its
/// candidate, so the pad draws the digits and the backspace and nothing
/// to confirm. The PIN pad the same flow ends on still has its ✓
/// (§16.132 rule 1).
#[test]
fn the_numbers_pad_has_no_check_and_the_pin_pad_does() {
    let mut h = Harness::new(PHONE);
    open_numbers(&mut h);
    let keys = h.app.keycaps(ids::LOAD_KEYBOARD);
    assert!(!keys.contains(&KeyInput::Done), "no ✓ on Word numbers' pad");
    assert!(
        keys.contains(&KeyInput::Backspace),
        "the backspace is there"
    );
    for d in '0'..='9' {
        assert!(keys.contains(&KeyInput::Char(d)), "{d} is on the pad");
    }

    for w in WORDS {
        type_number(&mut h, w);
    }
    h.tap(ids::LOAD_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::PassphraseOffer));
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.tap(ids::LOAD_HOLD);
    assert_eq!(h.app.load_step(), Some(Step::Pin));
    assert!(
        h.app
            .keycaps(ids::LOAD_PIN_KEYBOARD)
            .contains(&KeyInput::Done),
        "the PIN pad keeps its ✓"
    );
}
