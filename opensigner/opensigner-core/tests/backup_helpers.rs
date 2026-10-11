//! The three helpers under a key's Backup menu (`docs/PLANNING.md` §8.2
//! items 5a, 6 and 14): the SeedQR transcription grid, the numbers a
//! steel plate takes, and the Seed XOR split.

mod common;

use common::{ABANDON, DESKTOP, Harness, PANEL, PHONE, TINY};
use opensigner_core::backup::BackupStep;
use opensigner_core::create::Step;
use opensigner_core::ids;
use opensigner_core::strings;
use osk_bip::bip39::{Language, Mnemonic};
use osk_shell_api::{Command, EntropyBytes, Event, FileKind, Key};

/// Fifty plausible rolls, and fifty more that are not the same fifty.
const ROLLS_A: &str = "32461151351521144121541512665155412152342515356215";
const ROLLS_B: &str = "51263414253611524316253142653142536142531426351425";

/// Backup → "Split with Seed XOR" → `parts` parts → source row
/// `source`, and Continue, which opens the first random part's entry.
fn start_split(h: &mut Harness, parts: usize, source: usize) {
    open_backup_menu(h);
    h.tap(ids::BACKUP_XOR);
    h.tap(ids::at(ids::XOR_COUNT_BASE, parts - 2));
    h.tap(ids::XOR_COUNT_CONTINUE);
    assert_eq!(h.app.backup_step(), Some(BackupStep::XorSource));
    h.tap(ids::at(ids::XOR_SOURCE_BASE, source));
    h.tap(ids::XOR_SOURCE_CONTINUE);
}

/// The words of the part on screen, read while the panel is held.
fn shown_part(h: &mut Harness) -> Vec<String> {
    let point = h.press(ids::CREATE_REVEAL);
    let words = panel_words(h);
    h.release(point);
    words
}

/// The words the revealed panel is showing, in order: the rows are
/// "` 1. abandon`", numbered from one.
fn panel_words(h: &Harness) -> Vec<String> {
    h.app
        .texts()
        .into_iter()
        .filter_map(|t| {
            let (number, rest) = t.split_once(". ")?;
            number.trim().parse::<usize>().ok()?;
            Some(rest.trim().to_string())
        })
        .collect()
}

fn open_backup_menu(h: &mut Harness) {
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
}

/// The three rows are on the Backup menu of a key that holds its words,
/// and each opens its own screen.
#[test]
fn the_backup_menu_offers_the_grid_the_steel_numbers_and_the_split() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    open_backup_menu(&mut h);
    let texts = h.app.texts();
    for label in [s.backup_grid, s.backup_steel, s.xor_split] {
        assert!(texts.iter().any(|t| t == label), "{label}: {texts:?}");
    }
    // The key holds its words, so the rows are live: each has a hit
    // target of its own.
    for row in [ids::BACKUP_GRID, ids::BACKUP_STEEL, ids::BACKUP_XOR] {
        assert!(h.app.rect_of(row).is_some(), "{row:?} is a live row");
    }
}

/// "Draw a SeedQR" asks which code first, and the grid it opens pages by
/// quadrant on the 268 dp panel and fits one screen on a phone.
#[test]
fn the_grid_pages_by_quadrant_on_the_small_panel_and_not_on_a_phone() {
    let s = &strings::EN;
    let quadrants = [
        s.grid_top_left,
        s.grid_top_right,
        s.grid_bottom_left,
        s.grid_bottom_right,
    ];
    let mut h = Harness::new(PANEL);
    open_backup_menu(&mut h);
    h.tap(ids::BACKUP_GRID);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.backup_grid_title),
        "the code is chosen first: {texts:?}"
    );
    h.tap(ids::BACKUP_GRID_CONTINUE);
    for (i, name) in quadrants.iter().enumerate() {
        let texts = h.app.texts();
        assert!(
            texts.iter().any(|t| t == name),
            "page {i} is {name}: {texts:?}"
        );
        h.tap(ids::WORDS_NEXT);
    }
    // Four pages and no more: the last one stays put.
    let texts = h.app.texts();
    assert!(texts.iter().any(|t| t == s.grid_bottom_right));

    let mut h = Harness::new(PHONE);
    open_backup_menu(&mut h);
    h.tap(ids::BACKUP_GRID);
    h.tap(ids::BACKUP_GRID_CONTINUE);
    assert!(
        h.app.rect_of(ids::WORDS_NEXT).is_none(),
        "a phone shows the whole grid"
    );
    let texts = h.app.texts();
    for name in quadrants {
        assert!(!texts.iter().any(|t| t == name), "{name}: {texts:?}");
    }
}

/// "Numbers for steel" shows each word as a numbered plate and a letter
/// punch take it.
#[test]
fn the_steel_rows_carry_the_wordlist_number_and_the_first_four_letters() {
    let mut h = Harness::new(PHONE);
    open_backup_menu(&mut h);
    h.tap(ids::BACKUP_STEEL);
    h.tap(ids::SECRET_EYE);
    let rows = panel_words(&h);
    assert_eq!(rows.len(), ABANDON.len(), "one row per word: {rows:?}");
    assert_eq!(rows[0], "0001 \u{00b7} ABAN \u{00b7} abandon");
    assert_eq!(rows[11], "0004 \u{00b7} ABOU \u{00b7} about");
}

/// A steel row is the widest row the panel ever draws, and the smallest
/// panel is where it has the least room: every row is still shown whole
/// on each of the four sizes.
#[test]
fn the_steel_rows_are_shown_whole_on_every_size() {
    for display in [TINY, PANEL, PHONE, DESKTOP] {
        let mut h = Harness::new(display);
        open_backup_menu(&mut h);
        h.tap(ids::BACKUP_STEEL);
        h.tap(ids::SECRET_EYE);
        let rows = panel_words(&h);
        assert!(!rows.is_empty(), "the page has rows: {rows:?}");
        assert!(
            h.app.cut_texts().is_empty(),
            "cut off at {}x{}: {:?}",
            display.width,
            display.height,
            h.app.cut_texts(),
        );
        let panel = h.app.rect_of(ids::CREATE_REVEAL).expect("the panel");
        let pad = (osk_ui::tokens::SECRET_PAD * f32::from(display.dpi) / 160.0 + 0.5) as i32;
        for (text, rect) in h.app.text_rects() {
            if !text.contains('\u{00b7}') {
                continue;
            }
            assert!(
                rect.x >= panel.x + pad && rect.right() <= panel.right() - pad,
                "{text:?} at {rect:?} runs past the panel {panel:?} at {}x{}",
                display.width,
                display.height,
            );
        }
    }
}

/// "Print template" writes a blank table of the key's word count: the
/// rows a person fills in, and none of the words.
#[test]
fn the_steel_template_has_a_row_per_word_and_no_word_in_it() {
    let s = &strings::EN;
    let mut h = Harness::new(DESKTOP);
    open_backup_menu(&mut h);
    h.tap(ids::BACKUP_STEEL);
    h.seen.clear();
    h.tap(ids::BACKUP_STEEL_TEMPLATE);
    let written = h
        .seen
        .clone()
        .into_iter()
        .find_map(|c| match c {
            Command::WriteFile {
                kind: FileKind::Any,
                name_hint,
                bytes,
            } => Some((name_hint, bytes)),
            _ => None,
        })
        .expect("a file was offered");
    assert_eq!(written.0, s.steel_template_file);
    let text = String::from_utf8(written.1).expect("text");
    let rows: Vec<&str> = text.lines().skip(1).collect();
    assert_eq!(rows.len(), ABANDON.len(), "{text}");
    assert!(rows[0].starts_with(" 1."), "{text}");
    assert!(rows[11].starts_with("12."), "{text}");
    for word in ABANDON {
        assert!(!text.contains(word), "the template carries no word: {text}");
    }
}

/// "Split with Seed XOR" makes the parts from entropy the person enters,
/// names each one's fingerprint, and a part loaded as a key has the
/// fingerprint that was named.
#[test]
fn a_split_names_each_parts_fingerprint_and_a_part_loads_as_that_key() {
    let s = &strings::EN;
    let mut h = Harness::new(PHONE);
    start_split(&mut h, 3, 0);
    // One entry per random part, in part order.
    for rolls in [ROLLS_A, ROLLS_B] {
        assert_eq!(h.app.create_step(), Some(Step::Entropy));
        h.type_text(rolls);
        h.key(Key::Enter);
        assert_eq!(h.app.create_step(), Some(Step::Sanity));
        h.tap(ids::CREATE_CONTINUE);
    }
    assert_eq!(h.app.backup_step(), Some(BackupStep::XorPart));
    let mut parts: Vec<Vec<String>> = Vec::new();
    for i in 0..3 {
        let words = shown_part(&mut h);
        assert_eq!(words.len(), ABANDON.len(), "part {i}: {words:?}");
        parts.push(words);
        h.tap(ids::CREATE_CONTINUE);
    }
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.xor_split_result),
        "the split's own result: {texts:?}"
    );
    // The result names where the random parts came from.
    let at = texts
        .iter()
        .position(|t| t == s.xor_source_row)
        .unwrap_or_else(|| panic!("{}: {texts:?}", s.xor_source_row));
    assert_eq!(texts[at + 1], s.create_source_dice);
    let named: Vec<String> = (1..=3)
        .map(|i| {
            let label = strings::fill1(s.xor_part, &format!("{i}"));
            let at = texts
                .iter()
                .position(|t| *t == label)
                .unwrap_or_else(|| panic!("{label}: {texts:?}"));
            texts[at + 1].clone()
        })
        .collect();
    assert_eq!(
        named
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3,
        "three fingerprints: {named:?}"
    );
    // The parts are not the key, and no two of them are alike.
    for part in &parts {
        assert_ne!(part.as_slice(), &ABANDON.map(String::from)[..]);
    }

    // Loading the first part gives the fingerprint the result named for
    // it, which is how a person checks a part later.
    let words: Vec<&str> = parts[0].iter().map(String::as_str).collect();
    let mut h = Harness::new(PHONE);
    h.start_load(&words);
    h.finish_load(None);
    let loaded: Vec<String> = h
        .app
        .fingerprints()
        .iter()
        .map(|f| String::from_utf8(f.to_hex().to_vec()).expect("hex"))
        .collect();
    assert_eq!(loaded, vec![named[0].clone()]);
}

/// Hex entropy and two parts: what was typed is part 1 itself, so the
/// XOR can be done by hand and checked against the device; and the two
/// parts combine back into the key they came from.
#[test]
fn typed_hex_is_the_first_part_and_the_two_parts_combine_to_the_key() {
    const HEX: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f0";
    let mut h = Harness::new(PHONE);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let key: Vec<String> = h
        .app
        .fingerprints()
        .iter()
        .map(|f| String::from_utf8(f.to_hex().to_vec()).expect("hex"))
        .collect();
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_XOR);
    h.tap(ids::at(ids::XOR_COUNT_BASE, 0));
    h.tap(ids::XOR_COUNT_CONTINUE);
    h.tap(ids::at(ids::XOR_SOURCE_BASE, 2));
    h.tap(ids::XOR_SOURCE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    h.type_text(HEX);
    h.key(Key::Enter);
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.backup_step(), Some(BackupStep::XorPart));

    let typed: Vec<u8> = (0..HEX.len() / 2)
        .map(|i| u8::from_str_radix(&HEX[2 * i..2 * i + 2], 16).expect("hex"))
        .collect();
    let expected: Vec<String> = Mnemonic::from_entropy(Language::English, &typed)
        .expect("a mnemonic")
        .indices()
        .iter()
        .map(|&i| Language::English.word(i).to_string())
        .collect();
    let first = shown_part(&mut h);
    assert_eq!(first, expected, "part 1 is the hex that was typed");
    h.tap(ids::CREATE_CONTINUE);
    let second = shown_part(&mut h);
    h.tap(ids::CREATE_CONTINUE);

    // The two parts, loaded as Seed XOR parts, add the key back.
    let mut h = Harness::new(PHONE);
    h.open_load();
    h.choose(ids::LOAD_SOURCE_XOR, ids::LOAD_SOURCE_CONTINUE);
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    for (i, part) in [first, second].iter().enumerate() {
        h.tap(ids::SCAN_TYPE);
        for w in part {
            h.type_text(w);
            h.key(Key::Enter);
        }
        if i == 0 {
            h.tap(ids::XOR_ADD_ANOTHER);
        } else {
            h.tap(ids::XOR_COMBINE);
        }
    }
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    if h.app.screen() == opensigner_core::ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let combined: Vec<String> = h
        .app
        .fingerprints()
        .iter()
        .map(|f| String::from_utf8(f.to_hex().to_vec()).expect("hex"))
        .collect();
    assert_eq!(combined, key);
}

/// A 15-word key splits into parts of fifteen words: the hex typed for
/// the random part is forty digits, part 1 is that hex, and the two
/// parts combine back to the key.
#[test]
fn a_fifteen_word_key_splits_into_fifteen_word_parts() {
    const FIFTEEN: [&str; 15] = [
        "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon",
        "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "address",
    ];
    const HEX: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f00a1b2c3d";
    let mut h = Harness::new(PHONE);
    h.open_load();
    // Row 2 of "How many words?" is 15.
    h.load_choices(2);
    for w in FIFTEEN {
        h.type_text(w);
        h.key(Key::Enter);
    }
    h.tap(ids::LOAD_CONTINUE);
    h.finish_passphrase(None);
    let key: Vec<String> = h
        .app
        .fingerprints()
        .iter()
        .map(|f| String::from_utf8(f.to_hex().to_vec()).expect("hex"))
        .collect();

    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_XOR);
    h.tap(ids::at(ids::XOR_COUNT_BASE, 0));
    h.tap(ids::XOR_COUNT_CONTINUE);
    h.tap(ids::at(ids::XOR_SOURCE_BASE, 2));
    h.tap(ids::XOR_SOURCE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    h.type_text(HEX);
    h.type_text("f");
    assert_eq!(
        h.app.create_entries(),
        Some(40),
        "a 15-word part is forty hex digits"
    );
    h.key(Key::Enter);
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.backup_step(), Some(BackupStep::XorPart));

    let typed: Vec<u8> = (0..HEX.len() / 2)
        .map(|i| u8::from_str_radix(&HEX[2 * i..2 * i + 2], 16).expect("hex"))
        .collect();
    assert_eq!(typed.len(), 20);
    let expected: Vec<String> = Mnemonic::from_entropy(Language::English, &typed)
        .expect("a mnemonic")
        .indices()
        .iter()
        .map(|&i| Language::English.word(i).to_string())
        .collect();
    let first = shown_part(&mut h);
    assert_eq!(first, expected, "part 1 is the hex that was typed");
    h.tap(ids::CREATE_CONTINUE);
    let second = shown_part(&mut h);
    assert_eq!(second.len(), 15);
    h.tap(ids::CREATE_CONTINUE);

    let mut h = Harness::new(PHONE);
    h.open_load();
    h.choose(ids::LOAD_SOURCE_XOR, ids::LOAD_SOURCE_CONTINUE);
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 2),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    for (i, part) in [first, second].iter().enumerate() {
        h.tap(ids::SCAN_TYPE);
        for w in part {
            h.type_text(w);
            h.key(Key::Enter);
        }
        if i == 0 {
            h.tap(ids::XOR_ADD_ANOTHER);
        } else {
            h.tap(ids::XOR_COMBINE);
        }
    }
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    if h.app.screen() == opensigner_core::ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let combined: Vec<String> = h
        .app
        .fingerprints()
        .iter()
        .map(|f| String::from_utf8(f.to_hex().to_vec()).expect("hex"))
        .collect();
    assert_eq!(combined, key);
}

/// "This device" asks the shell once per random part, and the result
/// names the generator and what it rests on.
#[test]
fn the_device_source_is_asked_once_per_part_and_the_result_names_it() {
    let s = &strings::EN;
    let mut h = Harness::new(PHONE);
    start_split(&mut h, 3, 6);
    for _ in 0..2 {
        assert_eq!(h.app.create_step(), Some(Step::Device));
        h.seen.clear();
        h.tap(ids::CREATE_CONTINUE);
    }
    assert_eq!(h.app.backup_step(), Some(BackupStep::XorPart));
    for _ in 0..3 {
        h.tap(ids::CREATE_CONTINUE);
    }
    let texts = h.app.texts();
    let at = texts
        .iter()
        .position(|t| t == s.xor_source_row)
        .unwrap_or_else(|| panic!("{}: {texts:?}", s.xor_source_row));
    assert_eq!(
        texts[at + 1],
        format!("{} \u{00b7} {}", s.create_source_device, s.create_device_os)
    );
    assert!(
        texts.iter().any(|t| t == s.create_trust_device),
        "the result does not say what the parts rest on: {texts:?}"
    );

    // A shell that has not answered yet leaves the way on dead.
    let mut h = Harness::new(PHONE);
    open_backup_menu(&mut h);
    h.answer_entropy = false;
    h.tap(ids::BACKUP_XOR);
    h.tap(ids::at(ids::XOR_COUNT_BASE, 1));
    h.tap(ids::XOR_COUNT_CONTINUE);
    h.tap(ids::at(ids::XOR_SOURCE_BASE, 6));
    h.tap(ids::XOR_SOURCE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Device));
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(
        h.app.create_step(),
        Some(Step::Device),
        "the way on moved before the shell answered"
    );
    h.send(Event::Entropy(EntropyBytes::new([0x11; 32])));
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Device), "the second part");
}

/// The chevron out of the first part's entry lands on the source
/// choice, and coming back in starts that entry again.
#[test]
fn back_from_the_first_parts_entry_returns_to_the_source_choice() {
    let mut h = Harness::new(PHONE);
    start_split(&mut h, 2, 0);
    h.type_text(&ROLLS_A[..10]);
    assert_eq!(h.app.create_entries(), Some(10));
    h.tap(ids::BACK);
    assert_eq!(h.app.backup_step(), Some(BackupStep::XorSource));
    h.tap(ids::XOR_SOURCE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    assert_eq!(h.app.create_entries(), Some(0));
}
