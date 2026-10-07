//! Silent payments from a loaded key: the address is BIP-352's for the
//! key, a label makes another, the record in the Outbox reads back as
//! the same wallet, and the scan key leaves only through the secret sheet.

use faraday_core::{Action, Faraday, Screen, Sheet};
use osk_bip::silent::Receiver;
use osk_bip::silent_wallet::SilentWallet;
use osk_shell_api::{App, Event, Key};

fn with_key() -> Faraday {
    let mut app = Faraday::new();
    app.press(Action::Entry(None));
    for c in faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app
}

#[test]
fn the_address_is_bip352s_for_the_key_and_a_label_makes_another() {
    let mut app = with_key();
    app.press(Action::Silent);
    assert_eq!(app.screen, Screen::Silent);
    let secp = osk_bip::bitcoin::secp256k1::Secp256k1::new();
    let r = Receiver::derive(&app.session.keys[0].master, 0);
    let plain = r.address(&secp).unwrap().as_str().to_string();
    assert!(plain.starts_with("sp1"), "{plain}");
    assert_eq!(app.silent_address().as_deref(), Some(plain.as_str()));
    app.press(Action::SLabel(1));
    let one = r.labelled_address(&secp, 1).unwrap().as_str().to_string();
    assert_eq!(app.silent_address().as_deref(), Some(one.as_str()));
    assert_ne!(one, plain);
    // The record lists the label handed out and reads back.
    app.press(Action::SRecord);
    let text = String::from_utf8(app.outbox[0].bytes.clone()).unwrap();
    let w = SilentWallet::parse(&text).expect("the record does not read back");
    assert_eq!(w.labels, 1);
    assert_eq!(w.address(), plain);
}

#[test]
fn the_scan_key_leaves_only_through_the_secret_sheet() {
    let mut app = with_key();
    app.press(Action::Silent);
    app.press(Action::SNext);
    app.press(Action::SScanVault);
    assert!(app.outbox.is_empty(), "no vault is open");
    app.press(Action::SScanOut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty());
}
