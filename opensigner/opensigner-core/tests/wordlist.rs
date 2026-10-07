//! Tools › Word list: looking a BIP-39 word up by word, by number, by
//! binary and by hex, in the language's own keyboard, and walking the
//! list from any word.

mod common;

use common::{Harness, PANEL, PHONE};
use opensigner_core::wordlist::SearchBy;
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::bip39::Language;
use osk_bip::kana;
use osk_shell_api::Key;
use osk_ui::widgets::keyboard::{KeyInput, SMALL_KEY};

/// Home › Tools › Word list, through the language Choice, with the
/// wordlist at `lang` checked.
fn open_search(h: &mut Harness, lang: Language) {
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_WORD_LIST);
    assert_eq!(h.app.screen(), ScreenKind::WordList);
    let i = Language::ALL
        .iter()
        .position(|l| *l == lang)
        .expect("a listed wordlist");
    h.tap(ids::at(ids::WORDLIST_LANG_BASE, i));
    h.tap(ids::WORDLIST_LANG_CONTINUE);
}

/// Puts the check on one notation through the "Search by" row's Choice.
fn search_by(h: &mut Harness, by: SearchBy) {
    h.tap(ids::WORDLIST_BY);
    let i = SearchBy::ALL
        .iter()
        .position(|b| *b == by)
        .expect("a listed notation");
    h.tap(ids::at(ids::PICK_BASE, i));
    h.tap(ids::PICK_CONTINUE);
}

/// Opens the list at its first word, through the same Choice.
fn browse(h: &mut Harness) {
    h.tap(ids::WORDLIST_BY);
    h.tap(ids::at(ids::PICK_BASE, SearchBy::ALL.len()));
    h.tap(ids::PICK_CONTINUE);
}

/// Types `text` into the number, binary or hex field.
fn type_value(h: &mut Harness, text: &str) {
    for c in text.chars() {
        // The hex keyboard's caps are upper case; the field reads them
        // as the notation writes them.
        h.pad(
            ids::WORDLIST_KEYBOARD,
            KeyInput::Char(c.to_ascii_uppercase()),
        );
    }
}

fn texts(h: &Harness) -> Vec<String> {
    h.app.texts()
}

/// The first and the last word of the English list, each opened by every
/// notation, saying the same four things about itself.
#[test]
fn a_word_states_its_number_index_bits_and_hex() {
    let s = &strings::EN;
    for (word, number, index, binary, hex) in [
        ("abandon", "1", "0", "00000000000", "000"),
        ("zoo", "2048", "2047", "11111111111", "7ff"),
    ] {
        // By number.
        let mut h = Harness::new(PANEL);
        open_search(&mut h, Language::English);
        search_by(&mut h, SearchBy::Number);
        type_value(&mut h, number);
        h.key(Key::Enter);
        assert_eq!(h.app.screen(), ScreenKind::Word);
        let t = texts(&h);
        for expected in [word, number, index, binary, hex, "English"] {
            assert!(t.iter().any(|x| x == expected), "{expected} in {t:?}");
        }
        assert!(t.iter().any(|x| x == s.word_number_row));
        assert!(t.iter().any(|x| x == s.word_index_row));

        // By binary, and by hex, land on the same word.
        for (by, typed) in [(SearchBy::Binary, binary), (SearchBy::Hex, hex)] {
            let mut h = Harness::new(PANEL);
            open_search(&mut h, Language::English);
            search_by(&mut h, by);
            type_value(&mut h, typed);
            h.key(Key::Enter);
            assert_eq!(h.app.screen(), ScreenKind::Word, "{by:?}");
            assert!(texts(&h).iter().any(|x| x == word), "{by:?} found {word}");
        }
    }
}

/// The word search is the Load wizard's word entry: the candidates the
/// list has, and a tap on one opens it.
#[test]
fn typing_narrows_to_the_candidates_and_a_tap_opens_one() {
    let mut h = Harness::new(PHONE);
    open_search(&mut h, Language::English);
    h.type_text("zo");
    let offered: Vec<String> = Language::English
        .words()
        .iter()
        .filter(|w| w.starts_with("zo"))
        .map(|w| String::from(*w))
        .collect();
    let shown = h.app.candidates();
    assert_eq!(shown, offered, "the candidates the list has");
    // On a phone a candidate is taken by one tap on its cell.
    let cell = h
        .app
        .candidate_rect(ids::WORDLIST_CANDIDATES, 1)
        .expect("the second candidate's cell");
    let c = cell.center();
    h.tap_at(c.x as u16, c.y as u16);
    assert_eq!(h.app.screen(), ScreenKind::Word);
    assert!(
        texts(&h).iter().any(|t| t == &offered[1]),
        "the candidate that was tapped: {:?}",
        texts(&h)
    );
}

/// A number past the end of the list and a hex digit past `7ff` say why
/// and leave ✓ dead: the list has 2048 words and no more.
#[test]
fn a_value_outside_the_list_is_refused_with_its_reason() {
    let s = &strings::EN;
    for (by, typed, reason) in [
        (SearchBy::Number, "2049", s.wordlist_out_of_range),
        (SearchBy::Hex, "800", s.wordlist_hex_out_of_range),
    ] {
        let mut h = Harness::new(PANEL);
        open_search(&mut h, Language::English);
        search_by(&mut h, by);
        type_value(&mut h, typed);
        assert!(
            texts(&h).iter().any(|t| t == reason),
            "{by:?} says why: {:?}",
            texts(&h)
        );
        assert_eq!(
            h.app.key_rect(ids::WORDLIST_KEYBOARD, KeyInput::Done),
            None,
            "{by:?} leaves the check dead"
        );
        h.key(Key::Enter);
        assert_eq!(h.app.screen(), ScreenKind::WordList, "{by:?} opened a word");
    }
}

/// The Japanese list is looked up through its own kana keyboard, as the
/// Load wizard types it.
#[test]
fn the_japanese_list_is_searched_on_the_kana_keyboard() {
    let mut h = Harness::new(PHONE);
    open_search(&mut h, Language::Japanese);
    // あいこくしん is the first word of the published Japanese list.
    let word = Language::Japanese.word(0);
    for c in word.chars() {
        if let Some(big) = kana::large(c) {
            h.pad(ids::WORDLIST_KEYBOARD, KeyInput::Char(big));
            h.pad(ids::WORDLIST_KEYBOARD, KeyInput::Char(SMALL_KEY));
        } else if c == kana::DAKUTEN {
            h.pad(ids::WORDLIST_KEYBOARD, KeyInput::Char(kana::DAKUTEN_KEY));
        } else if c == kana::HANDAKUTEN {
            h.pad(ids::WORDLIST_KEYBOARD, KeyInput::Char(kana::HANDAKUTEN_KEY));
        } else {
            h.pad(ids::WORDLIST_KEYBOARD, KeyInput::Char(c));
        }
    }
    // §16.118: the word typed out in full is the one candidate left and
    // the tap on it is what opens it.
    assert_eq!(h.app.screen(), ScreenKind::WordList, "still the search");
    match h.app.candidate_rect(ids::WORDLIST_CANDIDATES, 0) {
        Some(cell) => {
            let c = cell.center();
            h.tap_at(c.x as u16, c.y as u16);
        }
        None => h.tap(ids::WORDLIST_CANDIDATES),
    }
    assert_eq!(h.app.screen(), ScreenKind::Word, "the word opened");
    let shown = texts(&h);
    let display = Language::Japanese.word_display(0);
    assert!(shown.iter().any(|t| t == display), "{display} in {shown:?}");
    assert!(shown.iter().any(|t| t == "1"), "the first word");
    assert!(shown.iter().any(|t| t == "Japanese"));
}

/// The pager walks the list from any word, and each end of it is dead.
#[test]
fn the_pager_walks_the_list_and_stops_at_its_ends() {
    let mut h = Harness::new(PANEL);
    open_search(&mut h, Language::English);
    browse(&mut h);
    assert!(texts(&h).iter().any(|t| t == "abandon"));
    h.tap(ids::WORD_PREV);
    assert!(
        texts(&h).iter().any(|t| t == "abandon"),
        "nothing is before word 1"
    );
    h.tap(ids::WORD_NEXT);
    assert!(texts(&h).iter().any(|t| t == "ability"));
    h.tap(ids::WORD_PREV);
    assert!(texts(&h).iter().any(|t| t == "abandon"));
    // Back from a word is the search, not a stack of every word read.
    h.tap(ids::WORD_NEXT);
    h.tap(ids::WORD_NEXT);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::WordList);
}
