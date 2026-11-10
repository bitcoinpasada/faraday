//! The sample QR vault code (`docs/DEMO.md` §14.1): in Practice the QR
//! vault tool offers it, the public passphrase opens it, and the seed it
//! holds loads as a key; it is the same file the generator makes.

use crate::common::{PASSPHRASE, demo_app, strip_offers};
use faraday_core::qr_tool::{self, QrToolAction as Q};
use faraday_core::testkit::{self, compute};
use faraday_core::wallet::Session;
use faraday_core::{Action, Faraday, Screen, Sheet};
use faraday_keys::QrHolds;
use faraday_shell_api::App;

fn open_tool(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "QR vault")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    let _ = app.frame();
}

#[test]
fn the_sample_code_is_the_bacon_seed_under_the_public_passphrase_and_is_made_again_as_it_is() {
    let c = compute();
    let mut s = Session::on(faraday_bip::keys::Network::Testnet);
    let words = faraday_core::samples::repeated("bacon", 24);
    s.keys.add_words(&c, &words, "", None).unwrap();
    let exponent = *faraday_keys::QR_SEAL_EXPONENTS.start();
    let random = zeroize::Zeroizing::new([0x5a; faraday_keys::QR_RANDOM_LEN]);
    let made = s
        .keys
        .qr_seal_key_job(&c, 0, PASSPHRASE, exponent, random)
        .unwrap()
        .run(&c)
        .unwrap();
    assert_eq!(made.as_slice(), faraday_samples::SAMPLES.inputs.qr_vault);
}

#[test]
fn practice_offers_the_sample_code_and_the_public_passphrase_opens_it_to_the_seed_sheet() {
    let mut app = demo_app();
    open_tool(&mut app);
    assert!(strip_offers(&mut app, Action::QrTool(Q::UseSample)));
    app.press(Action::QrTool(Q::UseSample));
    let _ = app.frame();
    assert!(qr_tool::state(&app).unwrap().code_to_open().is_some());
    assert!(strip_offers(
        &mut app,
        Action::PassChip(faraday_core::demo::PassField::QrOpen)
    ));
    app.press(Action::PassChip(faraday_core::demo::PassField::QrOpen));
    app.press(Action::QrTool(Q::OpenCode));
    testkit::settle(&mut app);
    assert_eq!(
        qr_tool::state(&app).unwrap().holds(),
        Some(QrHolds::Seed { words: 24 })
    );
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "the seed sheet");
}

#[test]
fn use_a_sample_starts_the_open_card_fresh() {
    let mut app = demo_app();
    open_tool(&mut app);
    app.press(Action::QrTool(Q::Pick(qr_tool::Pick::Note)));
    assert!(qr_tool::state(&app).unwrap().pick().is_some());
    app.press(Action::QrTool(Q::UseSample));
    let _ = app.frame();
    let s = qr_tool::state(&app).unwrap();
    assert!(s.code_to_open().is_some());
    assert!(s.pick().is_none());
}

#[test]
fn use_faraday_in_make_fills_only_an_empty_field() {
    let mut app = demo_app();
    open_tool(&mut app);
    app.press(Action::QrTool(Q::Pick(qr_tool::Pick::Note)));
    for ch in "a note".chars() {
        app.event(faraday_shell_api::Event::Key(faraday_shell_api::Key::Char(
            ch,
        )));
    }
    app.press(Action::QrTool(Q::Next));
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Pass)));
    for ch in "own words".chars() {
        app.event(faraday_shell_api::Event::Key(faraday_shell_api::Key::Char(
            ch,
        )));
    }
    app.press(Action::PassChip(faraday_core::demo::PassField::QrMake(0)));
    assert!(
        !qr_tool::state(&app).unwrap().passphrase_ready(),
        "the typed passphrase was replaced"
    );
}
