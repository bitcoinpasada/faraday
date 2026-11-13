//! A stick attached while a tool's secret field is open
//! (`docs/SECURITY-MODEL.md`): the QR vault tool, the KEF tool, Convert and
//! the Core wallet tool take no typing in a passphrase or a secret and say
//! why; pulled, they work again.

use crate::common::type_text;
use faraday_bip::keys::Network;
use faraday_core::convert::{self, ConvertAction as C, ROWS};
use faraday_core::core_wallet::tool::CoreWalletAction as W;
use faraday_core::kef_tool::{self, KefToolAction as K};
use faraday_core::qr_tool::{self, QrToolAction as Q};
use faraday_core::wallet::Session;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};

const VECTOR_PASS: &str = "correct horse battery staple";
const LINE: &str = "Keys load only with no stick attached";
const EMPTY: &[u8] = include_bytes!("../../../../tools/vectors/vault/qr/qr-empty.pgp");
const ABOUT: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
abandon abandon about";

fn started() -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app
}

fn attach(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    if app.sheet == Some(Sheet::Lock) {
        app.press(Action::NotNow);
    }
    assert!(!app.sticks().is_empty());
}

fn pull(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(Vec::new()));
}

fn said(app: &Faraday) -> bool {
    app.toast_text().is_some_and(|t| t == LINE)
}

fn whole(app: &mut Faraday) -> bool {
    crate::common::texts(app)
        .iter()
        .any(|t| t == "The wallet is whole")
}

fn tile(app: &mut Faraday, name: &str) {
    app.press(Action::Nav(Screen::Catalog));
    let at = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == name)
        .unwrap();
    app.press(Action::Catalog(at as u8));
}

#[test]
fn the_qr_tool_takes_no_passphrase_while_a_stick_is_attached_and_opens_nothing() {
    let mut app = started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("code.pgp"), EMPTY.to_vec())]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::QrTool(Q::OpenFile(0)));
    assert_eq!(app.screen, Screen::QrTool);
    type_text(&mut app, VECTOR_PASS);
    attach(&mut app);
    app.press(Action::QrTool(Q::OpenCode));
    testkit::settle(&mut app);
    assert!(said(&app), "{:?}", app.toast_text());
    assert!(qr_tool::state(&app).unwrap().holds().is_none());
    pull(&mut app);
    app.press(Action::QrTool(Q::OpenCode));
    testkit::settle(&mut app);
    assert!(
        qr_tool::state(&app).unwrap().holds().is_none(),
        "the passphrase typed before was wiped"
    );
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Pass)));
    type_text(&mut app, VECTOR_PASS);
    app.press(Action::QrTool(Q::OpenCode));
    testkit::settle(&mut app);
    assert!(
        qr_tool::state(&app).unwrap().holds().is_some(),
        "typing works with the stick pulled"
    );
}

#[test]
fn the_qr_tools_note_takes_no_typing_while_a_stick_is_attached() {
    let mut app = started();
    tile(&mut app, "QR vault");
    app.press(Action::QrTool(Q::Pick(qr_tool::Pick::Note)));
    type_text(&mut app, "a");
    attach(&mut app);
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Note)));
    assert!(said(&app), "{:?}", app.toast_text());
    type_text(&mut app, "a note");
    assert_eq!(qr_tool::state(&app).unwrap().note_chars(), 0);
    pull(&mut app);
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Note)));
    type_text(&mut app, "a note");
    assert_eq!(qr_tool::state(&app).unwrap().note_chars(), 6);
}

#[test]
fn the_kef_tool_takes_no_typed_text_and_no_passphrase_while_a_stick_is_attached() {
    let mut app = started();
    tile(&mut app, "Krux code");
    assert_eq!(app.screen, Screen::KefTool);
    app.press(Action::KefTool(K::Pick(kef_tool::Pick::Type)));
    type_text(&mut app, "ab");
    assert_eq!(kef_tool::state(&app).map(|s| s.typed_len()), Some(2));
    attach(&mut app);
    app.press(Action::KefTool(K::Field));
    assert!(said(&app), "{:?}", app.toast_text());
    type_text(&mut app, "cd");
    assert_eq!(kef_tool::state(&app).map(|s| s.typed_len()), Some(0));
    pull(&mut app);
    app.press(Action::KefTool(K::Field));
    type_text(&mut app, "cd");
    assert_eq!(kef_tool::state(&app).map(|s| s.typed_len()), Some(2));
}

#[test]
fn convert_takes_no_copy_of_a_secret_while_a_stick_is_attached() {
    let mut app = started();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    tile(&mut app, "Convert a seed");
    let row = ROWS
        .iter()
        .position(|r| r.name == "Master seed, hex")
        .unwrap();
    app.press(Action::Convert(C::To(row as u8)));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(&mut app);
    attach(&mut app);
    app.press(Action::Convert(C::Check));
    assert!(said(&app), "{:?}", app.toast_text());
    type_text(&mut app, "5");
    assert_eq!(convert::typed(&app), None, "Check my copy did not open");
    pull(&mut app);
    app.press(Action::Convert(C::Check));
    type_text(&mut app, "5");
    assert_eq!(convert::typed(&app), Some("5"), "typing works pulled");
}

#[test]
fn convert_closes_and_wipes_a_copy_typed_before_a_stick_came() {
    let mut app = started();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    tile(&mut app, "Convert a seed");
    let row = ROWS
        .iter()
        .position(|r| r.name == "Master seed, hex")
        .unwrap();
    app.press(Action::Convert(C::To(row as u8)));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(&mut app);
    app.press(Action::Convert(C::Check));
    type_text(&mut app, "5");
    attach(&mut app);
    assert_eq!(convert::typed(&app), None, "Check my copy closed");
    pull(&mut app);
    app.press(Action::Convert(C::Check));
    assert_eq!(convert::typed(&app), Some(""), "the copy typed is wiped");
    assert_eq!(convert::found(&app), None, "nothing was compared");
}

#[test]
fn the_core_wallet_tool_takes_no_passphrase_while_a_stick_is_attached() {
    let mut app = started();
    app.session = Session::on(Network::Testnet);
    app.storage(StorageEvent::Restored {
        inbox: vec![(
            faraday_text::Text::of("wallet.dat"),
            std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tools/vectors/core-wallet/encrypted.dat"
            ))
            .unwrap()
            .into(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    tile(&mut app, "Read a Bitcoin Core wallet");
    assert_eq!(app.screen, Screen::CoreWallet);
    app.press(Action::CoreWallet(W::Focus));
    type_text(&mut app, "faraday");
    attach(&mut app);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    testkit::settle(&mut app);
    assert!(!whole(&mut app), "the wallet was not read");
    app.press(Action::CoreWallet(W::Read));
    testkit::settle(&mut app);
    assert!(!whole(&mut app), "nor on the press");
    pull(&mut app);
    app.press(Action::CoreWallet(W::Read));
    testkit::settle(&mut app);
    assert!(whole(&mut app), "read now, and with the passphrase gone");
    assert!(
        crate::common::texts(&mut app)
            .iter()
            .any(|t| t.contains("not checked, no passphrase given"))
    );
    attach(&mut app);
    app.press(Action::CoreWallet(W::Focus));
    assert!(said(&app), "{:?}", app.toast_text());
    assert!(
        !faraday_core::core_wallet::tool::state(&app)
            .unwrap()
            .typing()
    );
    pull(&mut app);
    app.press(Action::CoreWallet(W::Focus));
    assert!(
        faraday_core::core_wallet::tool::state(&app)
            .unwrap()
            .typing(),
        "focus works pulled"
    );
}

#[test]
fn the_qr_note_shows_no_focus_while_a_stick_is_attached() {
    let mut app = started();
    attach(&mut app);
    tile(&mut app, "QR vault");
    app.press(Action::QrTool(Q::Pick(qr_tool::Pick::Note)));
    assert_ne!(
        qr_tool::state(&app).unwrap().focus(),
        Some(qr_tool::Field::Note)
    );
}

#[test]
fn the_kef_tools_typed_text_shows_no_focus_while_a_stick_is_attached() {
    let mut app = started();
    attach(&mut app);
    tile(&mut app, "Krux code");
    app.press(Action::KefTool(K::Pick(kef_tool::Pick::Type)));
    assert!(!kef_tool::state(&app).unwrap().typing());
}
