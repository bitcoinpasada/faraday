//! A wallet whose key waits for its backup is not offered in Sign a
//! message: Sign is refused and says why, and no file, QR code or sheet
//! with the address is made.

use faraday_core::catalog::Go;
use faraday_core::strings::EN;
use faraday_core::{Action, Code, Faraday, Screen, testkit};
use faraday_psbt::message::{Format, Signed};

/// A key of the test words with its single-key wallet.
fn app() -> Faraday {
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
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.session
        .add_wallet_confirmed(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the wallet");
    app
}

fn hold(app: &mut Faraday) {
    let fp = app.session.keys.last().unwrap().fingerprint;
    testkit::mark_made_here(app, fp);
}

fn back_up(app: &mut Faraday) {
    let fp = app.session.keys.last().unwrap().fingerprint;
    testkit::release_made(app, fp);
}

/// A signature of the wallet's first address, made beside the flow: what the
/// flow would hold had it signed.
fn signature(app: &Faraday) -> Signed {
    app.session
        .sign_message(&testkit::compute(), 0, 0, "hello", Format::Bip322)
        .expect("signs")
}

/// Sign a message open on the held wallet, chosen by hand.
fn held_flow() -> Faraday {
    let mut app = app();
    hold(&mut app);
    app.press(Action::SignMessage);
    app.press(Action::MWallet(0));
    app
}

/// The refusal said, and nothing made: no file and no code.
fn assert_refused(app: &Faraday) {
    let m = testkit::message(app).expect("open");
    assert_eq!(m.error.as_deref(), Some(EN.message_waits_backup));
    assert!(m.out_name.is_none());
    assert!(app.outbox.is_empty(), "no file");
    assert!(app.qr().is_none(), "no code");
}

#[test]
fn a_held_wallet_is_not_offered() {
    let mut app = app();
    assert_eq!(app.tile_need(Go::SignMessage), None);
    hold(&mut app);
    assert_eq!(
        app.tile_need(Go::SignMessage),
        Some(EN.message_waits_backup)
    );
    app.press(Action::SignMessage);
    assert_eq!(testkit::message(&app).unwrap().wallet, None);
    app.press(Action::SignMessageWith(0));
    assert_eq!(testkit::message(&app).unwrap().wallet, None);
}

#[test]
fn sign_is_refused() {
    let mut app = held_flow();
    app.press(Action::MSign);
    assert!(testkit::message(&app).unwrap().signed.is_none());
    assert_refused(&app);
}

#[test]
fn the_file_is_refused() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    app.press(Action::MOut);
    assert_refused(&app);
}

#[test]
fn the_code_is_refused() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    app.press(Action::MQr);
    assert_refused(&app);
}

#[test]
fn the_code_and_its_sheet_are_not_built() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    let why = app
        .code_view(&testkit::compute(), Code::Message)
        .err()
        .expect("no code");
    assert_eq!(why.as_str(), EN.message_waits_backup);
}

#[test]
fn check_it_is_refused() {
    let mut app = held_flow();
    let signed = signature(&app);
    testkit::message_put_signed(&mut app, signed);
    app.press(Action::CheckMessageMade);
    assert!(!testkit::message_made_kept(&app));
    assert_eq!(app.screen, Screen::Message);
    assert_refused(&app);
}

#[test]
fn a_key_held_after_signing_drops_what_was_made() {
    let mut app = app();
    app.press(Action::SignMessage);
    app.press(Action::MSign);
    app.press(Action::CheckMessageMade);
    assert!(testkit::message_made_kept(&app));
    hold(&mut app);
    assert!(!testkit::message_made_kept(&app));
    let m = testkit::message(&app).unwrap();
    assert!(m.signed.is_none());
    assert!(m.out_name.is_none() && m.out_sha.is_none());
    assert!(!m.done[faraday_core::mstep::SIGN as usize]);
}

#[test]
fn a_picked_address_of_a_held_key_opens_nothing() {
    let mut app = app();
    testkit::message_on_address(&mut app, "bc1qpicked");
    assert!(testkit::message(&app).unwrap().at.is_some());
    app.press(Action::MDone);
    hold(&mut app);
    testkit::message_on_address(&mut app, "bc1qpicked");
    assert!(testkit::message(&app).is_none());
}

#[test]
fn a_flow_left_half_way_signs_once_the_key_is_backed_up() {
    let mut app = held_flow();
    app.press(Action::MSign);
    assert!(testkit::message(&app).unwrap().signed.is_none());
    back_up(&mut app);
    app.press(Action::MType);
    assert_eq!(app.tile_need(Go::SignMessage), None);
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert!(m.signed.is_some());
    assert!(m.error.is_none());
    assert_eq!(app.outbox.len(), 1);
}
