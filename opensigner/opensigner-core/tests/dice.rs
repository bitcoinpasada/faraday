//! Tools › Dice passphrase: rolling a passphrase from an EFF list, what
//! it is worth, that the panel is masked until the eye, and that nothing
//! is left behind when the tool is left.

mod common;

use common::{Harness, PANEL, PHONE};
use opensigner_core::dice::WORD_COUNTS;
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::diceware::List;
use osk_shell_api::Command;
use osk_ui::widgets::keyboard::KeyInput;

/// Home › Tools › Dice passphrase, with `list` checked and `words`
/// words, standing on the pad.
fn open_rolls(h: &mut Harness, list: List, words: usize) {
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_DICE);
    assert_eq!(h.app.screen(), ScreenKind::DicePassphrase);
    let i = List::ALL
        .iter()
        .position(|l| *l == list)
        .expect("a listed list");
    h.tap(ids::at(ids::DICE_LIST_BASE, i));
    h.tap(ids::DICE_LIST_CONTINUE);
    let j = WORD_COUNTS
        .iter()
        .position(|n| *n == words)
        .expect("a listed length");
    h.tap(ids::at(ids::DICE_WORDS_BASE, j));
    h.tap(ids::DICE_WORDS_CONTINUE);
}

/// Throws `rolls`, a string of faces.
fn roll(h: &mut Harness, rolls: &str) {
    for c in rolls.chars() {
        h.pad(ids::DICE_PAD, KeyInput::Char(c));
    }
}

/// Rolls one word of `list`, `face` on every die.
fn one_word(list: List, face: char) -> String {
    core::iter::repeat_n(face, list.dice_per_word()).collect()
}

/// The words each list gives for its first and last roll.
#[test]
fn a_roll_names_the_word_its_list_names() {
    for (display, list, face, word) in [
        (PHONE, List::Large, '1', "abacus"),
        (PHONE, List::Large, '6', "zoom"),
        (PHONE, List::Short1, '1', "acid"),
        (PHONE, List::Short2, '1', "aardvark"),
    ] {
        let mut h = Harness::new(display);
        open_rolls(&mut h, list, 4);
        roll(&mut h, &one_word(list, face));
        // The panel is masked until the eye; the word is there behind it.
        h.tap(ids::SECRET_EYE);
        let shown = h.app.texts();
        assert!(
            shown.iter().any(|t| t.contains(word)),
            "{list:?} {face}: {word} in {shown:?}"
        );
    }
}

/// A length is offered with what it is worth, and the passphrase states
/// the same number when it is rolled.
#[test]
fn a_length_states_the_bits_it_is_worth() {
    let s = &strings::EN;
    for (list, words, bits) in [(List::Large, 6, "77"), (List::Short1, 8, "82")] {
        let mut h = Harness::new(PANEL);
        h.tap(ids::at(ids::HOME_TILE_BASE, 3));
        h.tap(ids::TOOLS_DICE);
        let i = List::ALL.iter().position(|l| *l == list).expect("a list");
        h.tap(ids::at(ids::DICE_LIST_BASE, i));
        h.tap(ids::DICE_LIST_CONTINUE);
        let row = opensigner_core::strings::fill(s.dice_words_bits, &[&format!("{words}"), bits]);
        let shown = h.app.texts();
        assert!(shown.contains(&row), "{row} in {shown:?}");

        // And the passphrase itself says it.
        let j = WORD_COUNTS
            .iter()
            .position(|n| *n == words)
            .expect("a length");
        h.tap(ids::at(ids::DICE_WORDS_BASE, j));
        h.tap(ids::DICE_WORDS_CONTINUE);
        for _ in 0..words {
            roll(&mut h, &one_word(list, '1'));
        }
        h.tap(ids::DICE_CONTINUE);
        let entropy = opensigner_core::strings::fill1(s.dice_bits, bits);
        let shown = h.app.texts();
        assert!(shown.contains(&entropy), "{entropy} in {shown:?}");
        assert!(shown.iter().any(|t| t == s.dice_entropy_row));
    }
}

/// The passphrase is a secret: masked on arrival, shown by the eye, and
/// the screen it is on stores nothing.
#[test]
fn the_passphrase_is_masked_until_the_eye_and_is_never_stored() {
    let mut h = Harness::new(PANEL);
    open_rolls(&mut h, List::Large, 4);
    for _ in 0..4 {
        roll(&mut h, &one_word(List::Large, '1'));
    }
    h.tap(ids::DICE_CONTINUE);
    let shows = |h: &Harness| h.app.texts().iter().any(|t| t.contains("abacus"));
    assert!(!shows(&h), "masked on arrival");
    h.tap(ids::SECRET_EYE);
    assert!(shows(&h), "the eye shows it");
    h.drain();
    h.tap(ids::DICE_DONE);
    assert_eq!(h.app.screen(), ScreenKind::Tools);
    assert!(
        !h.drain()
            .iter()
            .any(|c| matches!(c, Command::StoreSecret { .. })),
        "the tool keeps nothing"
    );
    // Nothing of the passphrase is left on Tools, and opening the tool
    // again starts from the list with no rolls.
    assert!(!shows(&h));
    h.tap(ids::TOOLS_DICE);
    assert!(!shows(&h));
    let s = &strings::EN;
    assert!(h.app.texts().iter().any(|t| t == s.dice_list_large));
}

/// Back with rolls entered asks before throwing them away.
#[test]
fn back_with_rolls_entered_asks_before_discarding() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_rolls(&mut h, List::Large, 4);
    roll(&mut h, "123");
    h.tap(ids::BACK);
    let shown = h.app.texts();
    assert!(
        shown.iter().any(|t| t == s.dice_discard_title),
        "the confirm: {shown:?}"
    );
    assert!(shown.iter().any(|t| t == "3"), "the rolls it would throw");
    // "Keep" returns to the pad with the rolls still there.
    h.tap(ids::DICE_KEEP);
    assert_eq!(h.app.screen(), ScreenKind::DicePassphrase);
    roll(&mut h, "4");
    h.tap(ids::BACK);
    h.tap(ids::DICE_DISCARD);
    assert_eq!(h.app.screen(), ScreenKind::Tools);
}

/// With nothing rolled there is nothing to discard: Back is one step.
#[test]
fn back_with_nothing_rolled_is_one_step() {
    let mut h = Harness::new(PANEL);
    open_rolls(&mut h, List::Large, 4);
    h.tap(ids::BACK);
    let s = &strings::EN;
    assert!(
        h.app.texts().iter().any(|t| t == s.dice_words_title),
        "the length step"
    );
}
