//! A seed pasted on the desktop app (F75): every scanner that loads a
//! seed takes its words, SeedQR digits or shares from the clipboard and
//! loads the key a scan or typing would; a scanner that takes no seed
//! still refuses one; and a shell with no clipboard shows no Paste there.

use std::str::FromStr;

use crate::common::type_text;
use faraday_bip::bitcoin::bip32::Xpriv;
use faraday_bip::bitcoin::secp256k1::Secp256k1;
use faraday_core::seeds::read::Choice;
use faraday_core::{Action, Faraday, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, Command, Event, FileKind, Key};

/// The BIP-39 reference vectors (passphrase "TREZOR"): the 12- and 24-word
/// all-zero entropy, with the master key each makes.
const VECTORS: [(&str, &str); 2] = [
    (
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        "xprv9s21ZrQH143K3h3fDYiay8mocZ3afhfULfb5GX8kCBdno77K4HiA15Tg23wpbeF1pLfs1c5SPmYHrEpTuuRhxMwvKDwqdKiGJS9XFKzUsAF",
    ),
    (
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        "xprv9s21ZrQH143K32qBagUJAMU2LsHg3ka7jqMcV98Y7gVeVyNStwYS3U7yVVoDZ4btbRNf4h6ibWpY22iRmXq35qgLs79f312g2kj5539ebPM",
    ),
];

/// The SeedQR specification's vectors: words and their standard digits.
const SEEDQRS: [(&str, &str); 2] = [
    (
        "attack pizza motion avocado network gather crop fresh patrol unusual wild holiday candy pony ranch winter theme error hybrid van cereal salon goddess expire",
        "011513251154012711900771041507421289190620080870026613431420201617920614089619290300152408010643",
    ),
    (
        "forum undo fragile fade shy sign arrest garment culture tube off merit",
        "073318950739065415961602009907670428187212261116",
    ),
];

#[expect(
    clippy::disallowed_methods,
    reason = "the test's own reading of the key, apart from the app's (docs/COMPUTE.md §7)"
)]
fn fingerprint_of(xprv: &str) -> [u8; 4] {
    let x = Xpriv::from_str(xprv).unwrap();
    x.fingerprint(&Secp256k1::new()).to_bytes()
}

/// The desktop app, started, with a clipboard.
fn desktop() -> Faraday {
    let mut app = testkit::started();
    app.clipboard = true;
    app
}

/// Paste pressed (or Ctrl+V), and the clipboard holding `text`.
fn paste(app: &mut Faraday, text: &str, ctrl_v: bool) {
    if ctrl_v {
        app.event(Event::Key(Key::Paste));
    } else {
        app.press(Action::ScanPaste);
    }
    let asked = std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, Command::RequestClipboard { .. }));
    assert!(asked, "Paste did not ask the shell for the clipboard");
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: text.to_string(),
    });
    let _ = app.frame();
}

fn note(app: &Faraday) -> String {
    app.scan().and_then(|s| s.note.clone()).unwrap_or_default()
}

/// Add a key's scanner, a seed pasted into it, and the seed loaded with
/// `passphrase` on the sheet that asks: the key's fingerprint.
fn pasted_seed(text: &str, passphrase: &str, ctrl_v: bool) -> [u8; 4] {
    let mut app = desktop();
    app.press(Action::ScanSeed);
    assert_eq!(
        app.sheet,
        Some(Sheet::Scan),
        "the seed scanner did not open"
    );
    assert!(app.scan_pastes(), "the seed scanner has no Paste");
    paste(&mut app, text, ctrl_v);
    assert_eq!(
        app.sheet,
        Some(Sheet::SeedRead),
        "the seed asks how it loads: {}",
        note(&app)
    );
    if passphrase.is_empty() {
        app.press(Action::SeedRead(Choice::Plain));
    } else {
        app.press(Action::SeedRead(Choice::AddPass));
        app.press(Action::SeedRead(Choice::Field(0)));
        type_text(&mut app, passphrase);
        app.press(Action::SeedRead(Choice::Field(1)));
        type_text(&mut app, passphrase);
        testkit::pause(&mut app);
        app.press(Action::SeedRead(Choice::WithPass));
    }
    assert_eq!(app.session.keys.len(), 1, "the paste loaded no key");
    app.session.keys[0].fingerprint.0
}

#[test]
fn bip39_vector_words_pasted_into_add_a_key_load_the_vectors_key() {
    for (k, (words, xprv)) in VECTORS.into_iter().enumerate() {
        // Ctrl+V for one, the Paste tile for the other.
        assert_eq!(
            pasted_seed(words, "TREZOR", k == 1),
            fingerprint_of(xprv),
            "{words}"
        );
    }
}

#[test]
fn words_pasted_in_capitals_across_lines_load_the_same_key() {
    let (words, xprv) = VECTORS[0];
    let messy = format!("  {}\n", words.to_uppercase().replacen(' ', "\n", 5));
    assert_eq!(pasted_seed(&messy, "TREZOR", false), fingerprint_of(xprv));
}

#[test]
fn seedqr_vector_digits_pasted_load_the_key_of_their_words() {
    for (words, digits) in SEEDQRS {
        assert_eq!(
            pasted_seed(digits, "", false),
            pasted_seed(words, "", false),
            "{words}"
        );
    }
}

#[test]
fn a_part_of_a_key_pasted_goes_into_its_form() {
    // BIP-93's first vector, into Add a key's codex32 form by its scanner.
    let mut app = desktop();
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    app.press(Action::ScanPart);
    assert!(app.scan_pastes(), "the part scanner has no Paste");
    paste(
        &mut app,
        "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw",
        false,
    );
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    assert_eq!(
        app.session.keys[0].fingerprint.0,
        fingerprint_of(
            "xprv9s21ZrQH143K3taPNekMd9oV5K6szJ8ND7vVh6fxicRUMDcChr3bFFzuxY8qP3xFFBL6DWc2uEYCfBFZ2nFWbAqKPhtCLRjgv78EZJDEfpL"
        )
    );
}

#[test]
fn a_wallets_missing_key_pasted_loads_at_once() {
    let mut app = desktop();
    app.storage(StorageEvent::Restored {
        inbox: vec![("savings.txt".to_string(), testkit::savings().into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::ScanKey(0));
    assert!(app.scan_pastes(), "the wallet's key scanner has no Paste");
    paste(&mut app, &testkit::test_words("zebra"), false);
    assert_eq!(app.session.keys.len(), 1, "{}", note(&app));
    let c = testkit::compute();
    let mut s = testkit::session();
    let zebra = s
        .keys
        .add_words(&c, &testkit::test_words("zebra"), "", None)
        .unwrap();
    assert_eq!(app.session.keys[0].fingerprint, zebra);
}

#[test]
fn a_seed_pasted_into_what_do_you_have_asks_how_it_loads() {
    use faraday_core::Screen;
    use faraday_core::family::{FamilyAction as F, page};
    let mut app = desktop();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Next(page::MAP)));
    app.press(Action::Family(F::Scan));
    assert_eq!(app.sheet, Some(Sheet::Scan), "the intake scanner is not on");
    paste(&mut app, &testkit::test_words("bacon"), false);
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "{}", note(&app));
    app.press(Action::SeedRead(Choice::Plain));
    assert_eq!(app.session.keys.len(), 1, "the paste loaded no key");
}

#[test]
fn a_private_key_pasted_into_a_seed_scanner_is_still_refused() {
    let mut app = desktop();
    app.press(Action::ScanSeed);
    paste(
        &mut app,
        "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn",
        false,
    );
    assert!(note(&app).contains("private key"), "{}", note(&app));
    assert!(app.session.keys.is_empty());
}

#[test]
fn seed_words_pasted_into_a_scanner_that_takes_no_seed_are_refused() {
    let mut app = desktop();
    app.press(Action::Scan);
    let (words, _) = VECTORS[1];
    paste(&mut app, words, false);
    assert!(
        note(&app).contains("seed's words") && note(&app).contains("No seed is taken here"),
        "{}",
        note(&app)
    );
    assert!(app.session.keys.is_empty(), "the words loaded a key");
    assert!(app.inbox().is_empty(), "the words were filed");
    assert_eq!(app.sheet, Some(Sheet::Scan), "the scanner closed");
}

#[test]
fn a_shell_with_no_clipboard_shows_no_paste_on_the_seed_scanner() {
    for clipboard in [false, true] {
        let mut app = testkit::started();
        app.clipboard = clipboard;
        app.press(Action::ScanSeed);
        assert_eq!(
            app.sheet,
            Some(Sheet::Scan),
            "the seed scanner did not open"
        );
        let _ = app.frame();
        let shown = app.drawn_texts().iter().any(|t| t == "Paste");
        assert_eq!(shown, clipboard, "Paste shown: {shown}");
        assert_eq!(app.scan_pastes(), clipboard);
        app.event(Event::Key(Key::Paste));
        let asked = std::iter::from_fn(|| app.poll_command())
            .any(|c| matches!(c, Command::RequestClipboard { .. }));
        assert_eq!(asked, clipboard, "Ctrl+V asked for the clipboard: {asked}");
    }
}
