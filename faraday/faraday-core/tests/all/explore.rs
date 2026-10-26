//! Explore shows the public side of a loaded key at a path: the
//! addresses BIP-84 and BIP-86 publish for "abandon … about", the script
//! a path's purpose names, and a path that is not one named as such; and
//! behind its eyes the key's words, its extended private keys and an
//! address's private key (BIP-84's), each hidden until its eye is pressed
//! and all hidden again once Explore is left.

use faraday_core::explore::{Eye, Page, XAction};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, Event, Key};

fn exploring() -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    for c in "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::Explore);
    assert_eq!(app.screen, Screen::Explore);
    app
}

#[test]
fn bip84_and_bip86s_first_addresses() {
    let c = faraday_core::testkit::compute();
    let mut app = exploring();
    let r = app.explore_reading(&c).unwrap();
    assert_eq!(r.address, "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
    app.press(Action::XPreset(1));
    let r = app.explore_reading(&c).unwrap();
    assert_eq!(
        r.address,
        "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
    );
    // The next index along.
    app.press(Action::XPreset(0));
    app.press(Action::XIndex(1));
    let r = app.explore_reading(&c).unwrap();
    assert_eq!(r.address, "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g");
}

#[test]
fn a_path_typed_wrongly_is_named() {
    let c = faraday_core::testkit::compute();
    let mut app = exploring();
    for _ in 0..20 {
        app.event(Event::Key(Key::Backspace));
    }
    for c in "84h//0".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert!(app.explore_path().is_err());
    assert!(app.explore_reading(&c).is_none());
}

/// What the screen draws now, every text run together with no spaces, so
/// a value grouped or wrapped reads whole.
fn drawn(app: &mut Faraday) -> String {
    crate::common::texts(app).concat().replace(' ', "")
}

/// How many cells of the words grid say `word`.
fn cells(app: &mut Faraday, word: &str) -> usize {
    crate::common::texts(app)
        .iter()
        .filter(|t| t.as_str() == word)
        .count()
}

fn eye(app: &mut Faraday, e: Eye) {
    app.press(Action::X(XAction::Eye(e)));
}

/// BIP-84's published private key of "abandon … about"'s first receive
/// address, m/84h/0h/0h/0/0 (faraday-keys' tests derive it and check it
/// pays to the address BIP-84 publishes).
const FIRST_WIF: &str = "KyZpNDKnfs94vbrwhJneDi77V6jF64PWPF8x5cdJb8ifgg2DUc9d";

#[test]
fn nothing_secret_is_shown_until_an_eye_is_pressed() {
    let mut app = exploring();
    assert_eq!(cells(&mut app, "abandon"), 0);
    let all = drawn(&mut app);
    assert!(!all.contains("xprv"));
    assert!(!all.contains(FIRST_WIF));
    // Every eye is offered, closed.
    for e in [Eye::Secret, Eye::Wif, Eye::Xprv] {
        assert!(crate::common::offered(&mut app, Action::X(XAction::Eye(e))));
    }
    assert!(!app.explore_shown().unwrap().any());
}

#[test]
fn the_eye_shows_the_words_and_hides_them() {
    let mut app = exploring();
    eye(&mut app, Eye::Secret);
    assert_eq!(cells(&mut app, "abandon"), 11);
    assert_eq!(cells(&mut app, "about"), 1);
    // The key has no passphrase: none is shown.
    assert!(!crate::common::has(&mut app, "Passphrase"));
    eye(&mut app, Eye::Secret);
    assert_eq!(cells(&mut app, "abandon"), 0);
    assert_eq!(cells(&mut app, "about"), 0);
}

#[test]
fn an_addresss_private_key_shown_is_bip84s() {
    let mut app = exploring();
    eye(&mut app, Eye::Wif);
    assert!(drawn(&mut app).contains(FIRST_WIF));
    // The next index along is another address: the key is hidden again.
    app.press(Action::XIndex(1));
    assert!(!drawn(&mut app).contains(FIRST_WIF));
    app.press(Action::XIndex(-1));
    assert!(!drawn(&mut app).contains(FIRST_WIF));
    // On the Addresses page, Receive 0's own eye shows the same key.
    app.press(Action::X(XAction::Page(Page::Addresses)));
    assert!(!drawn(&mut app).contains(FIRST_WIF));
    eye(&mut app, Eye::Row(false, 0));
    assert!(drawn(&mut app).contains(FIRST_WIF));
    eye(&mut app, Eye::Row(false, 0));
    assert!(!drawn(&mut app).contains(FIRST_WIF));
}

#[test]
fn the_extended_private_keys_show_behind_their_eye() {
    let mut app = exploring();
    eye(&mut app, Eye::Xprv);
    let all = drawn(&mut app);
    assert_eq!(
        all.matches("xprv").count(),
        2,
        "the root's and the account's"
    );
    assert!(crate::common::has(&mut app, "m/84h/0h/0h"));
    eye(&mut app, Eye::Xprv);
    assert!(!drawn(&mut app).contains("xprv"));
}

#[test]
fn everything_is_hidden_after_leaving_and_coming_back() {
    let mut app = exploring();
    for e in [Eye::Secret, Eye::Wif, Eye::Xprv] {
        eye(&mut app, e);
    }
    assert!(app.explore_shown().unwrap().any());
    app.event(Event::Key(Key::Escape));
    assert_ne!(app.screen, Screen::Explore);
    app.press(Action::Explore);
    assert_eq!(app.screen, Screen::Explore);
    assert!(!app.explore_shown().unwrap().any());
    assert_eq!(cells(&mut app, "abandon"), 0);
    let all = drawn(&mut app);
    assert!(!all.contains("xprv"));
    assert!(!all.contains(FIRST_WIF));
}
