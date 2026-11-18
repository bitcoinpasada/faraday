//! Explore shows the public side of a loaded key at a path: the
//! addresses BIP-84 and BIP-86 publish for "abandon … about" (their
//! Testnet counterparts, worked out with embit 0.8.0: Mainnet refuses the sample
//! seed), the script
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
    faraday_core::testkit::keep_published(&mut app);
    app.press(Action::Explore);
    assert_eq!(app.screen, Screen::Explore);
    app
}

#[test]
fn bip84_and_bip86s_first_addresses() {
    let c = faraday_core::testkit::compute();
    let mut app = exploring();
    let r = app.explore_reading(&c).unwrap();
    assert_eq!(r.address, "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl");
    app.press(Action::XPreset(1));
    let r = app.explore_reading(&c).unwrap();
    assert_eq!(
        r.address,
        "tb1p8wpt9v4frpf3tkn0srd97pksgsxc5hs52lafxwru9kgeephvs7rqlqt9zj"
    );
    // The next index along.
    app.press(Action::XPreset(0));
    app.press(Action::XIndex(1));
    let r = app.explore_reading(&c).unwrap();
    assert_eq!(r.address, "tb1qd7spv5q28348xl4myc8zmh983w5jx32cjhkn97");
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

/// The Testnet private key (WIF) of "abandon … about"'s first receive
/// address, m/84h/1h/0h/0/0, on Testnet because Mainnet refuses the sample
/// seed. Worked out with embit 0.8.0, which reproduces BIP-84's published
/// Mainnet vectors for this seed.
const FIRST_WIF: &str = "cTGhosGriPpuGA586jemcuH9pE9spwUmneMBmYYzrQEbY92DJrbo";

#[test]
fn nothing_secret_is_shown_until_an_eye_is_pressed() {
    let mut app = exploring();
    assert_eq!(cells(&mut app, "abandon"), 0);
    let all = drawn(&mut app);
    assert!(!all.contains("tprv"));
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
        all.matches("tprv").count(),
        2,
        "the root's and the account's"
    );
    assert!(crate::common::has(&mut app, "m/84h/1h/0h"));
    eye(&mut app, Eye::Xprv);
    assert!(!drawn(&mut app).contains("tprv"));
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
    assert!(!all.contains("tprv"));
    assert!(!all.contains(FIRST_WIF));
}

/// Explore's address, public key and extended key at the path each open
/// their code, as the screen prints them (BIP-84's first address and its
/// public key for "abandon … about"), with no picture (`docs/QR.md` §7).
#[test]
fn the_address_public_key_and_extended_key_open_their_codes() {
    use faraday_core::Code;
    let c = faraday_core::testkit::compute();
    let mut app = exploring();
    let r = app.explore_reading(&c).unwrap();
    for (code, want) in [
        (
            Code::ExploreAddress,
            "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl",
        ),
        (
            Code::ExplorePublicKey,
            "02e7ab2537b5d49e970309aae06e9e49f36ce1c9febbd44ec8e0d1cca0b4f9c319",
        ),
        (Code::ExploreXpub, r.xpub.as_str()),
    ] {
        crate::common::opens_its_code(&mut app, code, want);
    }
}

const GIVES_ACCOUNT: &str = "Withtheaccount'sextendedpublickey,thiskeygivesevery";

#[test]
fn a_private_key_at_an_unhardened_path_says_it_gives_the_account() {
    let mut app = exploring();
    assert!(!drawn(&mut app).contains(GIVES_ACCOUNT));
    eye(&mut app, Eye::Wif);
    assert!(drawn(&mut app).contains(FIRST_WIF));
    assert!(drawn(&mut app).contains(GIVES_ACCOUNT));
}

#[test]
fn a_private_key_on_the_addresses_page_says_it_gives_the_account() {
    let mut app = exploring();
    app.press(Action::X(XAction::Page(Page::Addresses)));
    assert!(!drawn(&mut app).contains(GIVES_ACCOUNT));
    eye(&mut app, Eye::Row(false, 0));
    assert!(drawn(&mut app).contains(FIRST_WIF));
    assert!(drawn(&mut app).contains(GIVES_ACCOUNT));
}

#[test]
fn a_private_key_at_an_all_hardened_path_says_nothing_of_the_account() {
    let mut app = exploring();
    for _ in 0..20 {
        app.event(Event::Key(Key::Backspace));
    }
    for c in "84h/0h/0h/0h/0h".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    eye(&mut app, Eye::Wif);
    assert!(app.explore_shown().is_some_and(|s| s.any()));
    assert!(!drawn(&mut app).contains(GIVES_ACCOUNT));
}

#[test]
fn explore_with_no_key_adds_one_on_the_add_a_key_page_and_returns() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Explore);
    assert_eq!(app.screen, Screen::Explore);
    let _ = app.frame();
    assert!(app.offers(Action::Entry(None)), "Add a key is offered");
    assert!(!app.offers(Action::KeyGen(None)));
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    for c in "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.screen, Screen::Explore, "back in Explore");
}
