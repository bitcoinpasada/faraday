//! Check an address and Decode's change check, opened with no wallet
//! loaded, lead into loading one and come back to the check with it in.

use faraday_core::catalog::TILES;
use faraday_core::testkit::{self, Kit};
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::App;

fn kit() -> Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap()
}

fn with_files(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

#[test]
fn check_an_address_with_no_wallet_opens_its_page_saying_to_load_one() {
    let mut app = with_files(vec![("wallet.txt", kit().descriptor.into_bytes())]);
    app.press(Action::Nav(Screen::Catalog));
    let i = TILES
        .iter()
        .position(|t| t.name == "Check an address")
        .unwrap();
    assert!(app.session.wallets.is_empty());
    app.press(Action::Catalog(i as u8));
    assert_eq!(app.screen, Screen::CheckAddress, "the page opens");
    assert!(app.address_check_state().unwrap().load_first());
    app.press(Action::Nav(Screen::Catalog));
    assert_eq!(app.screen, Screen::Catalog, "back where it was opened");
    assert!(app.address_check_state().is_none(), "nothing is kept");
}

#[test]
fn decode_with_no_wallet_loads_one_and_names_its_change() {
    let k = kit();
    let tx = testkit::unsigned(&k).unwrap().inner().unsigned_tx.clone();
    let hex = faraday_bip::bitcoin::consensus::encode::serialize_hex(&tx);
    let mut app = with_files(vec![
        ("tx.txt", hex.into_bytes()),
        ("wallet.txt", k.descriptor.into_bytes()),
    ]);
    app.press(Action::DecodeInbox(0));
    assert_eq!(app.screen, Screen::Decode);
    assert!(
        app.decode()
            .unwrap()
            .decoded
            .outputs
            .iter()
            .all(|o| o.ours.is_none())
    );
    let _ = app.frame();
    assert!(app.offers(Action::RestoreFor(faraday_core::restore::Check::Decode)));
    app.press(Action::RestoreFor(faraday_core::restore::Check::Decode));
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::RUse(1));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.screen, Screen::Decode, "back on the transaction");
    assert!(
        app.decode()
            .unwrap()
            .decoded
            .outputs
            .iter()
            .any(|o| o.ours.is_some())
    );
}

/// Decode opened with the wallet's file waiting and no wallet loaded, a
/// typed transaction of the spending kit read.
fn typed_transaction() -> (Faraday, usize) {
    let k = kit();
    let tx = testkit::unsigned(&k).unwrap().inner().unsigned_tx.clone();
    let hex = faraday_bip::bitcoin::consensus::encode::serialize_hex(&tx);
    let mut app = with_files(vec![("wallet.txt", k.descriptor.into_bytes())]);
    app.press(Action::Nav(Screen::Catalog));
    let tile = TILES
        .iter()
        .position(|t| t.name == faraday_core::strings::EN.catalog_decode_a_transaction)
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::Decode);
    app.press(Action::DecodeTyped(faraday_core::decode::TypedPress::Open));
    for c in hex.chars() {
        app.event(faraday_shell_api::Event::Key(faraday_shell_api::Key::Char(
            c,
        )));
    }
    app.press(Action::DecodeTyped(faraday_core::decode::TypedPress::Next));
    assert!(app.decode().is_some());
    (app, tile)
}

#[test]
fn a_typed_transaction_is_on_show_again_after_loading_a_wallet() {
    let (mut app, _) = typed_transaction();
    let _ = app.frame();
    app.press(Action::RestoreFor(faraday_core::restore::Check::Decode));
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::RUse(0));
    assert_eq!(app.screen, Screen::Decode, "back on the transaction");
    assert!(
        app.decode()
            .expect("the transaction is on show")
            .decoded
            .outputs
            .iter()
            .any(|o| o.ours.is_some())
    );
}

#[test]
fn a_typed_transaction_left_through_restore_for_elsewhere_is_dropped() {
    let (mut app, tile) = typed_transaction();
    let _ = app.frame();
    app.press(Action::RestoreFor(faraday_core::restore::Check::Decode));
    app.press(Action::Nav(Screen::Catalog));
    assert!(app.decode().is_none(), "dropped on leaving");
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::Decode);
    assert!(app.decode().is_none());
}

fn stick() -> faraday_core::StickInfo {
    faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }
}

#[test]
fn a_typed_transaction_survives_a_stick_put_in_while_loading_a_wallet() {
    let (mut app, _) = typed_transaction();
    let _ = app.frame();
    app.press(Action::RestoreFor(faraday_core::restore::Check::Decode));
    app.storage(StorageEvent::Sticks(vec![stick()]));
    assert_eq!(app.screen, Screen::Files);
    assert!(app.decode().is_some(), "kept while Files is up");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::RUse(0));
    assert_eq!(app.screen, Screen::Decode);
    assert!(app.decode().is_some(), "back on the transaction");
}

#[test]
fn a_typed_transaction_is_dropped_when_files_from_restore_is_left_for_elsewhere() {
    let (mut app, _) = typed_transaction();
    let _ = app.frame();
    app.press(Action::RestoreFor(faraday_core::restore::Check::Decode));
    app.storage(StorageEvent::Sticks(vec![stick()]));
    assert_eq!(app.screen, Screen::Files);
    app.press(Action::Nav(Screen::Catalog));
    assert!(app.decode().is_none());
}

#[test]
fn a_typed_transaction_is_dropped_at_once_when_restore_is_left_for_elsewhere() {
    let (mut app, tile) = typed_transaction();
    let _ = app.frame();
    app.press(Action::RestoreFor(faraday_core::restore::Check::Decode));
    app.press(Action::Nav(Screen::Catalog));
    assert!(app.decode().is_none(), "dropped on leaving");
    app.press(Action::Catalog(tile as u8));
    let _ = app.frame();
    assert!(!app.offers(Action::DecodeTyped(faraday_core::decode::TypedPress::Next)));
}

#[test]
fn a_typed_transaction_is_dropped_on_leaving_decode_after_restore_went_back() {
    let (mut app, tile) = typed_transaction();
    let _ = app.frame();
    app.press(Action::RestoreFor(faraday_core::restore::Check::Decode));
    app.press(Action::Nav(Screen::Decode));
    assert_eq!(app.screen, Screen::Decode);
    app.press(Action::Nav(Screen::Catalog));
    assert!(app.decode().is_none());
    app.press(Action::Catalog(tile as u8));
    let _ = app.frame();
    assert!(!app.offers(Action::DecodeTyped(faraday_core::decode::TypedPress::Next)));
}
