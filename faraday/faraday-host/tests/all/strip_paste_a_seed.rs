//! The strip's side of the paste_a_seed tests: what Faraday Demo's host offers there.

use crate::strips::strip_offers;
use faraday_core::wallet::Session;
use faraday_core::{Action, DESKTOP_MODES, Faraday, testkit};
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

/// The desktop app, started, with a clipboard.
fn desktop() -> Faraday {
    let mut app = testkit::started();
    app.clipboard = true;
    app
}

/// Faraday Demo on Testnet, its clipboard read by Paste, Add a key open.
fn demo_on_testnet() -> Faraday {
    let mut app = desktop();
    app.online = true;
    app.session = Session::on(testkit::NET);
    app.set_modes(DESKTOP_MODES);
    app.press(Action::Entry(None));
    let _ = app.frame();
    app
}

/// Paste words pressed (or Ctrl+V) on the words form, and the clipboard
/// holding `text`. Returns whether the shell was asked for it.
fn paste_words(app: &mut Faraday, text: &str, ctrl_v: bool) -> bool {
    if ctrl_v {
        app.event(Event::Key(Key::Paste));
    } else {
        app.press(Action::PasteWords);
    }
    let asked = std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, Command::RequestClipboard { .. }));
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: text.to_string(),
    });
    let _ = app.frame();
    asked
}

fn words_in(app: &Faraday) -> String {
    testkit::entry(app).typed.trim().to_string()
}

/// The key Add key loads from what the words form holds.
fn added(app: &mut Faraday) -> [u8; 4] {
    app.press(Action::EntryAdd);
    assert_eq!(
        app.session.keys.len(),
        1,
        "no key loaded: {:?}",
        testkit::entry(app).error
    );
    app.session.keys[0].fingerprint.0
}

#[test]
fn the_device_takes_no_words_pasted_on_any_network() {
    for testnet in [false, true] {
        let mut app = testkit::started();
        if testnet {
            app.session = Session::on(testkit::NET);
        }
        app.press(Action::Entry(None));
        let _ = app.frame();
        assert!(!app.entry_pastes());
        let (words, _) = VECTORS[0];
        for ctrl_v in [false, true] {
            assert!(!paste_words(&mut app, words, ctrl_v), "the shell was asked");
        }
        assert_eq!(words_in(&app), "", "words went in");
        assert!(!strip_offers(&mut app, Action::PasteWords));
    }
}

#[test]
fn paste_words_is_in_practices_strip_and_a_sample_pasted_loads() {
    let mut app = desktop();
    app.online = true;
    testkit::into_demo(&mut app);
    app.set_modes(DESKTOP_MODES);
    app.press(Action::Entry(None));
    let _ = app.frame();
    assert!(strip_offers(&mut app, Action::PasteWords));
    let zebra = testkit::test_words("zebra");
    assert!(paste_words(&mut app, &zebra, false));
    assert_eq!(words_in(&app), *zebra);
    added(&mut app);
}

#[test]
fn on_testnet_the_strip_offers_paste_words_and_it_and_ctrl_v_paste() {
    let mut app = demo_on_testnet();
    assert!(strip_offers(&mut app, Action::PasteWords));
    let (words, _) = VECTORS[0];
    assert!(paste_words(&mut app, words, false));
    assert_eq!(words_in(&app), words);
    let mut app = demo_on_testnet();
    assert!(paste_words(&mut app, words, true));
    assert_eq!(words_in(&app), words);
}
