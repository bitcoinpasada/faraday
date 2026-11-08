//! The Krux code sample (`docs/KEF.md` §15): in Practice the
//! seed scanner's Sample tile hands over, after a seed, a Krux encrypted
//! code of the first words sample, opened with the public
//! passphrase; outside Practice the tile is not there.

use crate::common::{PASSPHRASE, demo_app, key, strip_offers};
use faraday_core::seeds::read::Choice;
use faraday_core::{Action, Faraday, Sheet, testkit};
use faraday_shell_api::{App, Event, Key};

/// The seed scanner's Sample tile pressed for a seed, which is not loaded
/// (so nothing behind the sheet shows its fingerprint); the scanner opened
/// again offers the Krux code.
fn at_the_krux_code(app: &mut Faraday) {
    app.press(Action::ScanSeed);
    let _ = app.frame();
    assert_eq!(app.scan_sample_label(), "Load sample seed");
    app.press(Action::ScanSample);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::SeedRead(Choice::Skip));
    app.press(Action::ScanSeed);
    let _ = app.frame();
    assert_eq!(app.scan_sample_label(), "Load sample Krux code");
}

#[test]
fn practice_offers_the_sample_krux_code_and_the_public_passphrase_opens_it() {
    let mut app = demo_app();
    at_the_krux_code(&mut app);
    assert!(strip_offers(&mut app, Action::ScanSample));
    app.press(Action::ScanSample);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan), "the code's card");
    let ways = app
        .scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| c.ways.iter().map(|w| w.label()).collect::<Vec<_>>())
        .unwrap_or_default();
    let open = ways.iter().position(|w| w == "Open").expect("Open");
    app.press(Action::ReadGo(open as u8));
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    for c in PASSPHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Enter));
    for _ in 0..8 {
        testkit::settle(&mut app);
        if app.busy().is_none() {
            break;
        }
    }
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "the seed sheet");
    let hex: String = key("sample_guide")
        .plain
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let shown = app.drawn_texts().join(" ").to_lowercase();
    assert!(shown.contains(&hex), "{hex} not in {shown}");
}

#[test]
fn outside_practice_the_seed_scanner_offers_no_sample() {
    let mut app = testkit::started();
    app.press(Action::ScanSeed);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(!strip_offers(&mut app, Action::ScanSample));
}
