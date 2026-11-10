//! The sample wallets (`docs/MODES.md` §6): in Practice the Read a Bitcoin
//! Core wallet tile opens with the plain sample read whole, its keys
//! checked, and both sample files in Files; Use faraday opens the encrypted one.

use crate::common::demo_app;
use faraday_core::{Action, Screen};
use faraday_shell_api::App;

#[test]
fn practice_opens_the_tile_with_the_plain_sample_read_and_both_files_in_files() {
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Read a Bitcoin Core wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::CoreWallet);
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "The wallet is whole"));
    assert!(texts.iter().any(|t| t == "8 checked"));
    assert!(texts.iter().any(|t| t == "sample-plain-wallet.dat"));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "sample-encrypted-wallet.dat"));
}

#[test]
fn use_faraday_fills_the_passphrase_of_the_encrypted_sample() {
    use faraday_core::core_wallet::tool::CoreWalletAction as W;
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Read a Bitcoin Core wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::CoreWallet(W::File(1)));
    let _ = app.frame();
    app.press(Action::PassChip(faraday_core::demo::PassField::CoreWallet));
    app.press(Action::CoreWallet(W::Read));
    app.event(faraday_shell_api::Event::Tick { now_ms: 1 });
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "8 checked"), "{texts:?}");
}
