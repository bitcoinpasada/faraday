//! The online app (the desktop) says it is not air-gapped before it goes
//! to mainnet: the network waits for "I understand", and Cancel leaves it
//! on testnet. Reaching mainnet another way brings the same warning up
//! with no way past it but the acknowledgement. The device never shows it.

use faraday_core::wallet::Session;
use faraday_core::{Action, Faraday, Sheet, StorageEvent, testkit};
use osk_bip::keys::Network;
use osk_shell_api::App;

/// BIP-84's account 0 for the "abandon … about" seed: a mainnet wallet.
const MAINNET_WALLET: &str = "wpkh([73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*)";

/// The desktop's app: online, starting on testnet.
fn desktop() -> Faraday {
    let mut app = testkit::started();
    app.online = true;
    app.session = Session::on(Network::Testnet);
    app
}

#[test]
fn mainnet_waits_for_i_understand_on_the_desktop() {
    let mut app = desktop();
    app.press(Action::Network(Network::Mainnet));
    assert_eq!(app.sheet, Some(Sheet::NotAirgapped));
    assert_eq!(app.session.network(), Network::Testnet);
    let _ = app.frame();
    assert!(app.offers(Action::AirgapUnderstood));
    assert!(app.offers(Action::Cancel));
    app.press(Action::AirgapUnderstood);
    assert_eq!(app.session.network(), Network::Mainnet);
    assert_eq!(app.sheet, None);
    // Once a session: back to testnet and to mainnet again asks nothing.
    app.press(Action::Network(Network::Testnet));
    app.press(Action::Network(Network::Mainnet));
    assert_eq!(app.session.network(), Network::Mainnet);
    assert_eq!(app.sheet, None);
}

#[test]
fn cancel_leaves_the_desktop_on_testnet() {
    let mut app = desktop();
    app.press(Action::Network(Network::Mainnet));
    app.press(Action::Cancel);
    assert_eq!(app.session.network(), Network::Testnet);
    assert_eq!(app.sheet, None);
    let _ = app.frame();
    assert_eq!(app.sheet, None, "and the warning does not come back");
}

#[test]
fn a_mainnet_wallet_loaded_on_the_desktop_brings_the_warning_up() {
    let mut app = desktop();
    app.storage(StorageEvent::Restored {
        inbox: vec![(
            "main-wallet.txt".to_string(),
            MAINNET_WALLET.as_bytes().to_vec(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    assert_eq!(app.session.network(), Network::Mainnet);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NotAirgapped));
    let _ = app.frame();
    assert!(app.offers(Action::AirgapUnderstood));
    assert!(!app.offers(Action::Cancel), "only the acknowledgement");
    app.press(Action::AirgapUnderstood);
    let _ = app.frame();
    assert_eq!(app.sheet, None);
}

#[test]
fn the_device_goes_to_mainnet_without_the_warning() {
    let mut app = testkit::started();
    app.session = Session::on(Network::Testnet);
    app.press(Action::Network(Network::Mainnet));
    let _ = app.frame();
    assert_eq!(app.session.network(), Network::Mainnet);
    assert_eq!(app.sheet, None);
}
