//! The Krux code tool in Tools (`docs/KEF.md` §15.2): the tile opens it, a
//! typed text or a loaded wallet's descriptor makes a code that opens
//! back to it, and leaving wipes what was typed.

use faraday_core::kef_form::{KefAct, Stage};
use faraday_core::kef_tool::{self, KefToolAction as K, Pick};
use faraday_core::{Action, Faraday, OskPress, Screen, testkit};
use faraday_keys::kef::unwrap;
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

const WORDS: &str = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";

fn started() -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app
}

fn open(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Krux code")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn form_stage(app: &Faraday) -> Stage {
    kef_tool::state(app).unwrap().form().unwrap().stage()
}

/// Types the passphrase and the quickest rounds, and makes the code.
fn make(app: &mut Faraday) {
    app.press(Action::KefTool(K::Form(KefAct::Options)));
    app.press(Action::KefTool(K::Form(KefAct::Rounds)));
    for _ in 0..8 {
        app.event(Event::Key(Key::Backspace));
    }
    type_in(app, "10000");
    app.press(Action::KefTool(K::Form(KefAct::Back)));
    app.press(Action::KefTool(K::Form(KefAct::Field)));
    type_in(app, "a passphrase");
    app.press(Action::KefTool(K::Form(KefAct::Make)));
    testkit::settle(app);
    assert_eq!(form_stage(app), Stage::Made);
}

fn check(app: &mut Faraday) {
    app.press(Action::KefTool(K::Form(KefAct::Check)));
    type_in(app, "a passphrase");
    app.press(Action::KefTool(K::Form(KefAct::Compare)));
    testkit::settle(app);
}

#[test]
fn the_tile_opens_the_tool() {
    let mut app = started();
    open(&mut app);
    assert_eq!(app.screen, Screen::KefTool);
    assert!(kef_tool::state(&app).is_some());
}

#[test]
fn a_typed_text_makes_a_code_that_opens_back_to_it() {
    let mut app = started();
    open(&mut app);
    app.press(Action::KefTool(K::Pick(Pick::Type)));
    type_in(&mut app, "a note for later");
    app.press(Action::KefTool(K::Next));
    make(&mut app);
    assert!(app.outbox.iter().any(|i| i.name.ends_with(".png")));
    check(&mut app);
    assert!(kef_tool::state(&app).unwrap().form().unwrap().checked());
}

#[test]
fn a_loaded_wallets_descriptor_makes_a_code_with_its_fingerprint_as_id() {
    let mut app = started();
    let c = testkit::compute();
    app.session.keys.add_words(&c, WORDS, "Test", None).unwrap();
    use faraday_core::testkit::KindText;
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    app.session
        .add_wallet(
            &c,
            "Spending",
            &format!("wpkh({text}/<0;1>/*)"),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let fp = app.session.keys[0].fingerprint.0;
    open(&mut app);
    app.press(Action::KefTool(K::Pick(Pick::Wallet)));
    app.press(Action::KefTool(K::Wallet(0)));
    app.press(Action::KefTool(K::Form(KefAct::Options)));
    app.press(Action::KefTool(K::Form(KefAct::Id(
        faraday_core::kef_form::IdKind::Fingerprint,
    ))));
    app.press(Action::KefTool(K::Form(KefAct::Back)));
    make(&mut app);
    let form = kef_tool::state(&app).unwrap().form().unwrap();
    let envelope = unwrap(&form.made().unwrap().bytes).unwrap();
    assert_eq!(envelope.id, fp);
    check(&mut app);
    assert!(kef_tool::state(&app).unwrap().form().unwrap().checked());
}

#[test]
fn leaving_wipes_what_was_typed() {
    let mut app = started();
    open(&mut app);
    app.press(Action::KefTool(K::Pick(Pick::Type)));
    type_in(&mut app, "a note for later");
    app.press(Action::Nav(Screen::Home));
    assert!(kef_tool::state(&app).is_none());
    open(&mut app);
    assert_eq!(kef_tool::state(&app).unwrap().typed_len(), 0);
}

#[test]
fn a_text_typed_here_leaves_the_session_not_clean() {
    let mut app = started();
    assert!(app.clean());
    open(&mut app);
    app.press(Action::KefTool(K::Pick(Pick::Type)));
    type_in(&mut app, "a note for later");
    // What the shell does after every event.
    while app.poll_command().is_some() {}
    app.press(Action::Nav(Screen::Home));
    assert!(!app.clean());
}

fn small(app: &mut Faraday) {
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

fn osk_type(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.press(Action::Osk(OskPress::Char(c)));
    }
}

#[test]
fn on_a_small_panel_the_keyboard_types_the_text_and_the_passphrases() {
    let mut app = started();
    small(&mut app);
    open(&mut app);
    app.press(Action::KefTool(K::Pick(Pick::Type)));
    let _ = app.frame();
    assert!(
        app.keyboard_top().is_some(),
        "the keyboard is up for the text"
    );
    osk_type(&mut app, "a note");
    assert_eq!(kef_tool::state(&app).unwrap().typed_len(), 6);
    app.press(Action::KefTool(K::Next));
    app.press(Action::KefTool(K::Form(KefAct::Field)));
    let _ = app.frame();
    assert!(
        app.keyboard_top().is_some(),
        "the keyboard is up for the passphrase"
    );
    osk_type(&mut app, "secret");
    let form = kef_tool::state(&app).unwrap().form().unwrap();
    assert_eq!(form.passphrase(), "secret");
    for _ in 0..6 {
        app.event(Event::Key(Key::Backspace));
    }
    make(&mut app);
    app.press(Action::KefTool(K::Form(KefAct::Check)));
    app.press(Action::KefTool(K::Form(KefAct::CheckField)));
    let _ = app.frame();
    assert!(
        app.keyboard_top().is_some(),
        "the keyboard is up for Check my code"
    );
    osk_type(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Form(KefAct::Compare)));
    testkit::settle(&mut app);
    assert!(kef_tool::state(&app).unwrap().form().unwrap().checked());
}
