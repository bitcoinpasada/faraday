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

/// Explore on a 1366 × 768 window with two keys loaded, "abandon … about"
/// first and "zoo … wrong" second: the keys stand in a pane at the card's
/// left.
fn exploring_two_wide() -> Faraday {
    let mut app = wide_window();
    for phrase in [
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
    ] {
        app.press(Action::Entry(None));
        for c in phrase.chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.press(Action::EntryAdd);
        faraday_core::testkit::keep_published(&mut app);
    }
    app.press(Action::Explore);
    assert_eq!(app.screen, Screen::Explore);
    app
}

/// A started app on a 1366 × 768 window.
fn wide_window() -> Faraday {
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};
    let mut app = faraday_core::testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// Explore on the wide window with `n` keys loaded.
fn exploring_many(n: u8) -> Faraday {
    use faraday_bip::bip39::Language;
    let c = faraday_core::testkit::compute();
    let mut app = wide_window();
    for i in 0..n {
        let m = faraday_keys::words::from_entropy(Language::English, &[i + 1; 16]).unwrap();
        let words: Vec<&str> = m
            .indices()
            .iter()
            .map(|&w| Language::English.words()[usize::from(w)])
            .collect();
        app.session
            .keys
            .add_words(&c, &words.join(" "), &format!("Seed {i}"), None)
            .unwrap();
    }
    app.press(Action::Explore);
    assert_eq!(app.screen, Screen::Explore);
    app
}

/// A press and its release at the middle of what offers `a`.
fn touch(app: &mut Faraday, a: Action) {
    let (x, y) = app.where_offered(a).expect("offered");
    for phase in [
        faraday_shell_api::TouchPhase::Down,
        faraday_shell_api::TouchPhase::Up,
    ] {
        app.event(Event::Touch { x, y, phase });
    }
}

/// The left edge of the "Path" label: where the card begins.
fn card_left(app: &mut Faraday) -> i32 {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == "Path")
        .map(|(_, r)| r.0)
        .min()
        .expect("the path's label")
}

fn second_key(app: &Faraday) -> [u8; 4] {
    app.session.keys.all().nth(1).unwrap().fingerprint.0
}

#[test]
fn a_wide_window_lists_every_key_in_a_pane_and_a_press_picks_one() {
    let mut app = exploring_two_wide();
    let second = second_key(&app);
    let first = app.session.keys.all().next().unwrap().fingerprint.0;
    assert!(crate::common::offered(&mut app, Action::XKey(first)));
    let card = card_left(&mut app);
    for k in [first, second] {
        let (x, _, w, _) = app.hit_box(Action::XKey(k)).expect("a row");
        assert!(i32::from(x + w) < card, "the row ends left of the card");
    }
    touch(&mut app, Action::XKey(second));
    assert_eq!(app.explore().unwrap().key, Some(second));
}

#[test]
fn a_pane_of_many_keys_scrolls_to_its_last_row() {
    let mut app = exploring_many(20);
    let first = app.session.keys.all().next().unwrap().fingerprint.0;
    let last = app.session.keys.all().last().unwrap().fingerprint.0;
    let (w, h) = app.size();
    let _ = app.frame();
    let full = app.hit_box(Action::XKey(first)).expect("the first row").3;
    for _ in 0..400 {
        if app
            .hit_box(Action::XKey(last))
            .is_some_and(|(_, y, _, bh)| bh == full && f32::from(y + bh) <= h)
        {
            return;
        }
        app.event(Event::Scroll {
            x: (w * 0.5) as u16,
            y: (h / 2.0) as u16,
            dy: 120,
        });
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        let _ = app.frame();
    }
    panic!("the last key's row never came whole into the window");
}

#[test]
fn the_divider_stays_while_the_path_is_unfinished() {
    let c = faraday_core::testkit::compute();
    let mut app = exploring_two_wide();
    for _ in 0..20 {
        app.event(Event::Key(Key::Backspace));
    }
    for ch in "84h//0".chars() {
        app.event(Event::Key(Key::Char(ch)));
    }
    assert!(app.explore_reading(&c).is_none());
    let first = app.session.keys.all().next().unwrap().fingerprint.0;
    let card = card_left(&mut app);
    let (x, _, w, _) = app.hit_box(Action::XKey(first)).expect("a row");
    let (from, to) = (usize::from(x + w) + 1, usize::try_from(card).unwrap());
    let (width, _) = app.size();
    let rgba = app.frame().rgba.to_vec();
    let row = 330 * width as usize;
    let at = |col: usize| &rgba[(row + col) * 4..(row + col) * 4 + 4];
    assert!(
        (from..to).any(|col| at(col) != at(from)),
        "a divider between the pane and the card"
    );
}

#[test]
fn the_card_follows_the_key_picked_in_the_pane() {
    let c = faraday_core::testkit::compute();
    let mut app = exploring_two_wide();
    let before = app.explore_reading(&c).unwrap().address.clone();
    app.press(Action::XKey(second_key(&app)));
    let after = app.explore_reading(&c).unwrap().address.clone();
    assert_ne!(before, after);
    assert!(drawn(&mut app).contains(&after));
}

#[test]
fn another_key_picked_in_the_pane_hides_the_words_shown() {
    let mut app = exploring_two_wide();
    for e in [Eye::Secret, Eye::Wif, Eye::Xprv] {
        eye(&mut app, e);
    }
    assert_eq!(cells(&mut app, "abandon"), 11);
    app.press(Action::XKey(second_key(&app)));
    assert!(!app.explore_shown().unwrap().any());
    assert_eq!(cells(&mut app, "abandon"), 0);
    assert_eq!(cells(&mut app, "zoo"), 0);
}
