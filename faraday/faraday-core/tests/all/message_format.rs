//! Sign a message's format follows the wallet: it starts on a format the
//! wallet signs in, and a wallet chosen that cannot sign in the format
//! chosen moves it to one it can (`faraday_psbt::message::Format::for_script`).

use faraday_core::{Action, testkit};
use faraday_psbt::message::Format;

/// The started app with the first test key and the kits `ids` loaded, in
/// that order.
fn with_wallets(ids: &[&str]) -> faraday_core::Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    for id in ids {
        let kit = testkit::kits().into_iter().find(|k| k.id == *id).unwrap();
        app.session
            .add_wallet_confirmed(
                &c,
                kit.name,
                &kit.descriptor,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .expect("the wallet");
    }
    app
}

fn wallet(app: &faraday_core::Faraday, name: &str) -> usize {
    app.session
        .wallets
        .iter()
        .position(|w| w.name == name)
        .unwrap()
}

fn bip322(app: &faraday_core::Faraday) -> bool {
    testkit::message(app).unwrap().bip322
}

#[test]
fn a_nested_wallet_starts_on_bip137_and_signs_in_it() {
    let mut app = with_wallets(&["nested"]);
    app.press(Action::SignMessage);
    assert!(!bip322(&app), "BIP-322 is greyed for a nested wallet");
    testkit::message_text(&mut app, "Faraday signs this");
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert_eq!(m.error, None);
    assert_eq!(m.signed.as_ref().expect("signed").format, Format::Bip137);
}

#[test]
fn a_wallet_chosen_moves_the_format_off_one_greyed_for_it() {
    let mut app = with_wallets(&["spending", "taproot", "legacy"]);
    app.press(Action::SignMessage);
    assert!(bip322(&app), "a native segwit wallet starts on BIP-322");
    app.press(Action::MWallet(wallet(&app, "Legacy")));
    assert!(!bip322(&app), "BIP-322 is greyed for a legacy wallet");
    app.press(Action::MWallet(wallet(&app, "Spending")));
    assert!(!bip322(&app), "BIP-137, still allowed, stays chosen");
    app.press(Action::MWallet(wallet(&app, "Taproot")));
    assert!(bip322(&app), "BIP-137 is greyed for a taproot wallet");
}

/// The address a message is signed with, signing it from the start the
/// way `open` does.
fn signed_address(app: &mut faraday_core::Faraday, open: Action) -> String {
    app.press(open);
    testkit::message_text(app, "Faraday signs this");
    app.press(Action::MSign);
    let m = testkit::message(app).unwrap();
    assert_eq!(m.error, None);
    m.signed.as_ref().expect("signed").address.to_string()
}

#[test]
fn sign_a_message_from_the_second_wallets_card_signs_with_the_second_wallet() {
    let mut app = with_wallets(&["spending", "taproot"]);
    let second = wallet(&app, "Taproot");
    let first = signed_address(&mut app, Action::SignMessage);
    let from_card = signed_address(&mut app, Action::SignMessageWith(second));
    assert_ne!(first, from_card, "the first wallet is not the one signing");
    signed_address(&mut app, Action::SignMessage);
    app.press(Action::MWallet(second));
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert_eq!(from_card, m.signed.as_ref().unwrap().address.to_string());
}
