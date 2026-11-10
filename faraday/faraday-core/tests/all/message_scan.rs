//! Sign a message takes its text from a scanner: a scanned plain text is
//! the message and is signed as any typed one is; a seed or any other
//! kind of code is named and refused, and no text is kept. Home's Scan
//! of a text offers Sign this message only while a single-key wallet is
//! loaded (`docs/SCANNING.md` §4.3).

use faraday_core::scan_read::Way;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, Event};

const SEED: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// The started app with the first test key and, if `wallet`, a
/// single-key wallet.
fn app(wallet: bool) -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    if wallet {
        let kit = testkit::kits()
            .into_iter()
            .find(|k| k.id == "spending")
            .unwrap();
        app.session
            .add_wallet(
                &c,
                kit.name,
                &kit.descriptor,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .expect("the wallet");
    }
    app
}

fn read(app: &mut Faraday, text: &str) {
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
}

fn note(app: &Faraday) -> String {
    app.scan().and_then(|s| s.note.clone()).unwrap_or_default()
}

#[test]
fn a_scanned_text_is_the_message_and_is_signed() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    read(&mut app, "Pay the bearer of this message");
    assert_eq!(app.sheet, None, "the scanner closes on a text");
    assert_eq!(
        testkit::message(&app).unwrap().text,
        "Pay the bearer of this message"
    );
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert_eq!(m.error, None);
    assert!(m.signed.is_some());
}

#[test]
fn a_scanned_seed_is_named_and_no_text_is_kept() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    read(&mut app, SEED);
    assert_eq!(app.sheet, Some(Sheet::Scan), "the scanner stays open");
    assert!(
        note(&app).contains("not used here"),
        "the seed is named: {}",
        note(&app)
    );
    assert_eq!(testkit::message(&app).unwrap().text, "");
}

#[test]
fn another_kind_of_code_is_named_and_refused_too() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    read(
        &mut app,
        "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn",
    );
    assert!(note(&app).contains("not used here"), "{}", note(&app));
    read(
        &mut app,
        "otpauth://totp/Scanned:me?secret=JBSWY3DPEHPK3PXP",
    );
    assert!(note(&app).contains("not used here"), "{}", note(&app));
    assert_eq!(testkit::message(&app).unwrap().text, "");
}

/// The read card's ways after a text read on Home's Scan; none where no
/// card shows.
fn ways_for_text(app: &mut Faraday) -> Vec<Way> {
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    read(app, "A note to sign");
    app.scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| c.ways.clone())
        .unwrap_or_default()
}

#[test]
fn homes_scan_of_a_text_offers_sign_this_message_only_with_a_wallet() {
    let mut without = app(false);
    assert!(!ways_for_text(&mut without).contains(&Way::SignMessage));
    let mut with = app(true);
    let ways = ways_for_text(&mut with);
    let at = ways
        .iter()
        .position(|w| *w == Way::SignMessage)
        .expect("Sign this message");
    with.press(Action::ReadGo(at as u8));
    assert_eq!(with.screen, Screen::Message);
    assert_eq!(testkit::message(&with).unwrap().text, "A note to sign");
}
