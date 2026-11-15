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
