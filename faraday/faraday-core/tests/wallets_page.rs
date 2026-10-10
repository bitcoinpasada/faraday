//! The Wallets page with wallets loaded (`docs/SIMPLIFY.md` §1.3): the
//! start page is the list and Add a key; every other job (Sign a
//! transaction, Back up, Show wallet QR, Sign a message) is reached
//! from the wallet card.

use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const WORDS: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                      abandon abandon about";

fn shown() -> Faraday {
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
    app
}

/// A single-key wallet, its one key loaded here.
fn with_single_key_wallet() -> Faraday {
    let mut app = shown();
    app.session.add_words(WORDS, "Seed", None).unwrap();
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&app.session.keys[0].master)
        .unwrap();
    let d = format!("wpkh({text}/<0;1>/*)");
    app.session.add_wallet("Wallet", &d, "test").unwrap();
    app.press(Action::Nav(Screen::Start));
    let _ = app.frame();
    app
}

/// A watch-only single-key wallet: the key is not held here, so it
/// cannot sign a message.
fn with_watch_only_wallet() -> Faraday {
    let mut app = shown();
    let mut elsewhere = faraday_core::wallet::Session::default();
    elsewhere.add_words(WORDS, "Seed", None).unwrap();
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&elsewhere.keys[0].master)
        .unwrap();
    let d = format!("wpkh({text}/<0;1>/*)");
    app.session.add_wallet("Watch-only", &d, "test").unwrap();
    app.press(Action::Nav(Screen::Start));
    let _ = app.frame();
    app
}

#[test]
fn wallets_start_page_offers_the_row_and_add_a_key() {
    let app = with_single_key_wallet();
    assert!(app.offers(Action::OpenWallet(0)), "the wallet row");
    assert!(app.offers(Action::Entry(None)), "Add a key");
}

#[test]
fn the_wallet_card_offers_its_jobs() {
    let mut app = with_single_key_wallet();
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Wallets);
    assert!(app.offers(Action::Backup(0)), "Back up");
    assert!(app.offers(Action::QrWallet(0)), "Show wallet QR");
}

#[test]
fn another_wallets_psbt_in_files_leaves_the_card_on_spend_from_this_wallet() {
    // The test kit's PSBT spends from a test-network wallet, not this one
    // (`docs/NEW-WALLET.md` §11.2: only its own PSBT turns the button).
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = with_single_key_wallet();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![("spend.psbt".to_string(), psbt)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Wallets);
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Spend from this wallet"),
        "{texts:?}"
    );
    assert!(app.offers(Action::Family(
        faraday_core::family::FamilyAction::SpendFrom(0)
    )));
}

#[test]
fn the_card_offers_sign_a_message_for_a_single_key_wallet_with_its_key_here() {
    let mut app = with_single_key_wallet();
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert!(app.offers(Action::SignMessage));
}

#[test]
fn the_card_does_not_offer_sign_a_message_without_the_key_here() {
    let mut app = with_watch_only_wallet();
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert!(!app.offers(Action::SignMessage));
}
