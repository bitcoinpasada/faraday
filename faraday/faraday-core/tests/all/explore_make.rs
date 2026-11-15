//! Explore's **Make a wallet** at a loaded key's standard single-key
//! account: it makes the wallet Create would make (its first address
//! BIP-84's published one for "abandon … about"), a second press opens
//! that wallet and does not make another, and a multisig path offers
//! nothing.

use faraday_core::explore::XAction;
use faraday_core::{Action, Faraday, Screen, Sheet};
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

fn offered(app: &mut Faraday) -> bool {
    app.settle();
    let _ = app.frame();
    app.offers(Action::X(XAction::Make))
}

#[test]
fn a_bip84_path_makes_the_wallet_and_a_second_press_opens_it() {
    let c = faraday_core::testkit::compute();
    let mut app = exploring();
    assert!(offered(&mut app), "Make a wallet at BIP-84's path");
    let before = app.session.wallets.len();
    app.press(Action::X(XAction::Make));
    assert_eq!(app.session.wallets.len(), before + 1);
    assert_eq!(app.sheet, Some(Sheet::MadeWallet));
    let made = app.session.wallets.len() - 1;
    let w = &app.session.wallets[made];
    assert!(
        w.name.as_str().ends_with("m/84h/0h/0h"),
        "{}",
        w.name.as_str()
    );
    assert_eq!(
        String::from(&*app.session.address(&c, w, false, 0)),
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
    );
    let _ = app.frame();
    app.press(Action::X(XAction::MadeDone));
    assert_eq!(app.sheet, None);
    // The same account again opens the wallet.
    assert!(offered(&mut app));
    app.press(Action::X(XAction::Make));
    assert_eq!(app.session.wallets.len(), before + 1, "not made twice");
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.wallet_index(), made);
}

#[test]
fn a_multisig_path_offers_no_wallet() {
    let mut app = exploring();
    app.press(Action::XPreset(4));
    assert!(!offered(&mut app), "BIP-48 needs its cosigners");
}
