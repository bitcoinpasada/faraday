//! The Spend tab when something is loaded already: it opens on what is
//! loaded and the next step for each wallet instead of the walk-through,
//! which stays one press away and is what someone with nothing loaded
//! still sees first.

use faraday_core::family::{FamilyAction as F, Open, page};
use faraday_core::testkit;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_files(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
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
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn savings() -> (String, Vec<u8>) {
    (
        "savings-wallet.txt".to_string(),
        kit("savings").descriptor.into_bytes(),
    )
}

fn savings_psbt() -> (String, Vec<u8>) {
    let (u, _) = testkit::spend_of(&kit("savings")).unwrap();
    ("savings-unsigned.psbt".to_string(), u.to_bytes())
}

#[test]
fn with_nothing_loaded_the_tab_is_the_walk_through() {
    let mut app = with_files(Vec::new());
    app.press(Action::Nav(Screen::Family));
    assert!(!app.family_overview());
}

#[test]
fn a_ready_wallet_goes_straight_to_its_next_page() {
    let mut app = with_files(vec![savings()]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Family));
    assert!(app.family_overview(), "the tab opens on what is loaded");
    let _ = app.frame();
    assert!(app.offers(Action::Family(F::SpendFrom(0))));
    app.press(Action::Family(F::SpendFrom(0)));
    assert!(!app.family_overview());
    assert_eq!(app.family_wallet(), Some(0));
    assert_eq!(
        app.family.open,
        Some(Open::Page(page::CHECK)),
        "the map, the stick and the wallet are behind it"
    );
}

#[test]
fn a_ready_wallet_with_its_transaction_in_files_goes_to_signing() {
    let mut app = with_files(vec![savings(), savings_psbt()]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::SpendFrom(0)));
    assert!(app.spend.is_some(), "the PSBT in Files is the one to sign");
    assert_eq!(app.screen, Screen::Family);
    assert_eq!(app.family.open, Some(Open::Spend));
    assert!(app.spend.as_ref().unwrap().steps.contains(&step::SIGN));
}

#[test]
fn a_wallet_short_of_keys_is_opened_rather_than_spent() {
    let mut app = with_files(vec![savings()]);
    add_key(&mut app, 0);
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Family));
    let _ = app.frame();
    assert!(!app.offers(Action::Family(F::SpendFrom(0))));
    assert!(app.offers(Action::OpenWallet(0)));
}

#[test]
fn a_seed_with_no_wallet_spends_as_a_single_key_wallet() {
    let mut app = with_files(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Nav(Screen::Family));
    assert!(app.family_overview());
    let _ = app.frame();
    assert!(app.offers(Action::Family(F::SeedWallet)));
    app.press(Action::Family(F::SeedWallet));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.family_wallet(), Some(0));
}

#[test]
fn the_walk_through_is_one_press_away() {
    let mut app = with_files(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Walkthrough));
    assert!(!app.family_overview());
    assert_eq!(app.family.open, Some(Open::Page(page::MAP)));
}
