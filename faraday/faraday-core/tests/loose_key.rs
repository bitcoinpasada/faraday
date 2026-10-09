//! A key added with no wallet loaded: Wallets offers a wallet from it.
//! Make a wallet from this key opens Restore's seeds card with the key in
//! and the shape at one key, Native SegWit, account 0; Make the wallet
//! gives the wallet whose first receive address is BIP-84's published one
//! for that seed. Add another key opens the same card at two keys. Back
//! returns to Wallets with the key still loaded.

use faraday_core::seeds::SeedsAction as S;
use faraday_core::wallet::{Session, fp_text, key_line, needed};
use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// BIP-84's test mnemonic.
const ABANDON: &str = "abandon abandon abandon abandon abandon abandon \
                       abandon abandon abandon abandon abandon about";
/// BIP-84's first receive address for it, m/84'/0'/0'/0/0, on mainnet.
const BIP84_FIRST: &str = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";

fn shown(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

fn desktop() -> Faraday {
    shown(1366, 768, 160)
}

/// Home, Add a key, the words typed, Add key. Returns the key's
/// fingerprint.
fn add_key(app: &mut Faraday, words: &str) -> [u8; 4] {
    let before: Vec<[u8; 4]> = app
        .session
        .keys
        .iter()
        .map(|k| k.master.fingerprint().0)
        .collect();
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    for c in words.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    let _ = app.frame();
    app.session
        .keys
        .iter()
        .map(|k| k.master.fingerprint().0)
        .find(|f| !before.contains(f))
        .expect("no key added")
}

#[test]
fn a_key_with_no_wallet_makes_the_bip84_wallet() {
    let mut app = desktop();
    let fp = add_key(&mut app, ABANDON);
    assert_eq!(app.screen, Screen::Wallets);
    assert!(app.session.wallets.is_empty());
    let from_key = Action::KeyWallet(fp, 1);
    assert!(
        app.offers(from_key),
        "Wallets offers no wallet from the key"
    );

    app.press(from_key);
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::Seeds(S::Make));

    assert_eq!(app.session.wallets.len(), 1);
    let w = &app.session.wallets[0];
    assert_eq!(Session::shape(w), "Single key · native SegWit");
    let here = app
        .session
        .slots(w)
        .iter()
        .filter(|s| s.held_by.is_some())
        .count();
    assert!(here >= needed(w), "the wallet cannot sign here");
    assert_eq!(app.session.address(w, false, 0), BIP84_FIRST);
}

#[test]
fn add_another_key_opens_the_shape_at_two_keys() {
    let mut app = desktop();
    let fp = add_key(&mut app, ABANDON);
    app.press(Action::KeyWallet(fp, 2));
    assert_eq!(app.screen, Screen::Restore);
    let s = app
        .restore
        .as_ref()
        .and_then(|r| r.seeds.as_ref())
        .expect("no seeds card");
    assert_eq!(s.keys, vec![fp]);
    assert!(s.shaping);
    assert_eq!((s.m, s.n), (2, 2));
    // One box for the second key's xpub.
    assert_eq!(s.cosigners.len(), 1);
}

#[test]
fn back_from_the_seeds_card_returns_to_wallets_with_the_key() {
    let mut app = desktop();
    let fp = add_key(&mut app, ABANDON);
    app.press(Action::KeyWallet(fp, 1));
    let _ = app.frame();
    let back = Action::Nav(Screen::Wallets);
    assert!(app.offers(back), "no Back to Wallets");
    app.press(back);
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Wallets);
    assert!(app.session.wallets.is_empty());
    assert!(app.offers(Action::KeyWallet(fp, 1)));
}

#[test]
fn a_key_with_no_label_is_named_by_its_fingerprint_once() {
    let mut app = desktop();
    let fp = add_key(&mut app, ABANDON);
    let f = osk_bip::keys::Fingerprint(fp);
    let line = key_line(f, app.session.key_label(f).unwrap_or(""));
    assert_eq!(line, fp_text(f));
}

#[test]
fn with_a_wallet_loaded_the_loose_keys_row_offers_a_wallet() {
    let mut app = desktop();
    let first = add_key(&mut app, ABANDON);
    app.press(Action::KeyWallet(first, 1));
    app.press(Action::Seeds(S::Make));
    app.press(Action::Nav(Screen::Wallets));
    let second = add_key(&mut app, &faraday_core::testkit::test_words("zebra"));
    assert_eq!(app.screen, Screen::Wallets);
    assert!(app.offers(Action::KeyWallet(second, 1)));
    assert!(!app.offers(Action::KeyWallet(first, 1)));
}

#[test]
fn several_loose_keys_offer_the_one_picked() {
    let mut app = desktop();
    let a = add_key(&mut app, ABANDON);
    let b = add_key(&mut app, &faraday_core::testkit::test_words("zebra"));
    assert!(app.offers(Action::KeyWallet(a, 1)));
    app.press(Action::PickKey(b));
    let _ = app.frame();
    assert!(app.offers(Action::KeyWallet(b, 1)));
    assert!(!app.offers(Action::KeyWallet(a, 1)));
}

#[test]
fn the_small_panel_offers_a_wallet_from_the_key() {
    let mut app = shown(480, 640, 286);
    assert!(app.is_compact());
    let fp = add_key(&mut app, ABANDON);
    assert_eq!(app.screen, Screen::Wallets);
    let mut seen = false;
    for _ in 0..20 {
        let _ = app.frame();
        if app
            .where_offered(Action::KeyWallet(fp, 1))
            .is_some_and(|(_, y)| y < 640)
        {
            seen = true;
            break;
        }
        app.event(Event::Scroll {
            x: 240,
            y: 320,
            dy: 60,
        });
    }
    assert!(seen, "the small panel offers no wallet from the key");
}

#[test]
fn the_small_panels_wallet_list_offers_a_wallet_from_a_loose_key() {
    let mut app = shown(480, 640, 286);
    let first = add_key(&mut app, ABANDON);
    app.press(Action::KeyWallet(first, 1));
    app.press(Action::Seeds(S::Make));
    let second = add_key(&mut app, &faraday_core::testkit::test_words("zebra"));
    app.press(Action::Nav(Screen::Start));
    let mut seen = false;
    for _ in 0..20 {
        let _ = app.frame();
        if app
            .where_offered(Action::KeyWallet(second, 1))
            .is_some_and(|(_, y)| y < 640)
        {
            seen = true;
            break;
        }
        app.event(Event::Scroll {
            x: 240,
            y: 320,
            dy: 60,
        });
    }
    assert!(seen, "the wallet list offers no wallet from the loose key");
    assert!(!app.offers(Action::KeyWallet(first, 1)));
}
