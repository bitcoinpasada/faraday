//! Vanity addresses: turning a loaded key's account number or its
//! passphrase until the first receive address begins with the characters
//! typed, and opening the wallet that address belongs to.

use faraday_core::testkit;
use faraday_core::vanity::{VanityAction as V, vstep};
use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, Event, Key};

fn with_key() -> Faraday {
    let mut app = Faraday::new();
    app.press(Action::Network(testkit::NET));
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::Vanity(V::Open));
    assert_eq!(app.screen, Screen::Vanity);
    app
}

/// Types `chars` after the fixed part and searches until a find.
fn search(app: &mut Faraday, chars: &str) -> String {
    app.press(Action::Vanity(V::Next(vstep::DIAL)));
    app.press(Action::Vanity(V::Next(vstep::SCRIPT)));
    for c in chars.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vanity(V::Next(vstep::PREFIX)));
    app.press(Action::Vanity(V::Start));
    for t in 1..5000u64 {
        app.event(Event::Tick { now_ms: t * 100 });
        let v = app.vanity.as_ref().unwrap();
        if let Some(f) = &v.grind.find {
            return f.address.as_str().to_string();
        }
        assert!(v.running, "the search stopped with no find: {:?}", v.error);
    }
    panic!("no find");
}

fn opened_address(app: &mut Faraday) -> String {
    app.press(Action::Vanity(V::Use));
    assert_eq!(app.screen, Screen::Wallets);
    let w = &app.session.wallets[app.wallet];
    app.session.address(w, false, 0)
}

#[test]
fn the_account_dial_finds_an_address_and_opens_its_wallet() {
    let mut app = with_key();
    let found = search(&mut app, "q");
    assert!(found.starts_with("tb1qq"), "{found}");
    let account = app
        .vanity
        .as_ref()
        .unwrap()
        .grind
        .find
        .as_ref()
        .unwrap()
        .account;
    assert!(account > 0, "account 0 already matched: a weak test");
    assert_eq!(opened_address(&mut app), found);
    assert_eq!(
        app.session.keys.len(),
        1,
        "the account dial loads no new key"
    );
}

#[test]
fn the_passphrase_dial_loads_the_words_with_the_passphrase_found() {
    let mut app = with_key();
    app.press(Action::Vanity(V::Dial(
        opensigner_core::vanity::Dial::ALL
            .iter()
            .position(|d| *d == opensigner_core::vanity::Dial::Passphrase)
            .unwrap() as u8,
    )));
    let found = search(&mut app, "q");
    assert!(found.starts_with("tb1qq"), "{found}");
    assert_eq!(opened_address(&mut app), found);
    assert_eq!(
        app.session.keys.len(),
        2,
        "the key with the passphrase did not load"
    );
    assert!(app.session.keys[1].passphrase.is_some());
}

#[test]
fn a_character_that_cannot_start_the_address_is_refused() {
    let mut app = with_key();
    app.press(Action::Vanity(V::Next(vstep::DIAL)));
    app.press(Action::Vanity(V::Next(vstep::SCRIPT)));
    // Bech32 has no b, i, o or 1.
    app.event(Event::Key(Key::Char('b')));
    let v = app.vanity.as_ref().unwrap();
    assert!(v.error.is_some());
    assert_eq!(v.grind.prefix.as_str(), "tb1q");
}
