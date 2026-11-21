//! Add a key's two forms of one line of text: an extended private key and a
//! master seed in hex, against BIP-32's test vector 1 (`docs/KEY-FORMS.md`
//! §6).

use faraday_core::{Action, Faraday};
use faraday_forms::Form;
use faraday_shell_api::{App, Event, Key};

const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
const CHILD: &str = "xprv9uHRZZhk6KAJC1avXpDAp4MDc3sQKNxDiPvvkX8Br5ngLNv1TxvUxt4cV1rGL5hj6KCesnDYUhd7oWgT11eZG7XnxHrnYeSvkzY7d2bhkJ7";
/// `xprv` written as a test network's `tprv`, checksum made again: the
/// session the tests run on is a test network's.
fn tprv(xprv: &str) -> String {
    let read = faraday_codec::encodings::read(xprv).unwrap();
    let mut payload = vec![0x35, 0x83, 0x94];
    payload.extend_from_slice(&read.bytes[3..]);
    faraday_codec::encodings::base58check(0x04, &payload)
}

const SEED: &str = "000102030405060708090a0b0c0d0e0f";
const FINGERPRINT: [u8; 4] = [0x34, 0x42, 0x19, 0x3e];

fn adding(form: Form) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(form.code()));
    app
}

fn type_and_enter(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Enter));
}

fn error(app: &Faraday) -> String {
    faraday_core::testkit::entry(app)
        .error
        .as_ref()
        .map(|t| t.as_str().to_string())
        .unwrap_or_default()
}

#[test]
fn the_master_xprv_and_its_seed_load_the_same_key() {
    let mut a = adding(Form::Xprv);
    type_and_enter(&mut a, &tprv(XPRV));
    assert_eq!(a.session.keys.len(), 1, "{}", error(&a));
    assert_eq!(a.session.keys[0].fingerprint.0, FINGERPRINT);

    let mut b = adding(Form::HexSeed);
    type_and_enter(&mut b, SEED);
    assert_eq!(b.session.keys.len(), 1, "{}", error(&b));
    assert_eq!(b.session.keys[0].fingerprint.0, FINGERPRINT);
}

#[test]
fn an_xprv_below_depth_zero_says_it_is_not_a_master_key_and_loads_nothing() {
    let mut app = adding(Form::Xprv);
    type_and_enter(&mut app, &tprv(CHILD));
    assert!(app.session.keys.is_empty());
    let e = error(&app);
    assert!(e.contains("Not a master key"), "{e}");
    assert!(e.contains("depth 1") || e.contains("depth 1,"), "{e}");
    assert!(e.contains("3442193e"), "{e}");
}

#[test]
fn the_other_networks_version_is_refused_by_name() {
    let mut app = adding(Form::Xprv);
    type_and_enter(&mut app, XPRV);
    assert!(app.session.keys.is_empty());
    assert_eq!(error(&app), "The key is xprv: this session reads tprv");
}

#[test]
fn a_hex_seed_of_a_wrong_length_or_with_odd_digits_is_refused_with_its_length() {
    let mut app = adding(Form::HexSeed);
    type_and_enter(&mut app, "0001020304050607");
    assert!(app.session.keys.is_empty());
    assert!(error(&app).contains("8 bytes"), "{}", error(&app));

    let mut app = adding(Form::HexSeed);
    type_and_enter(&mut app, &format!("{SEED}0"));
    assert!(app.session.keys.is_empty());
    assert!(error(&app).contains("33"), "{}", error(&app));
}

#[test]
fn a_hex_seed_in_capitals_with_spaces_loads() {
    let mut app = adding(Form::HexSeed);
    type_and_enter(&mut app, "00010203 04050607 08090A0B 0C0D0E0F");
    assert_eq!(app.session.keys.len(), 1, "{}", error(&app));
    assert_eq!(app.session.keys[0].fingerprint.0, FINGERPRINT);
}

fn read(app: &mut Faraday, code: &str) {
    app.event(Event::Scanned {
        bytes: code.as_bytes().to_vec(),
    });
    let _ = app.frame();
}

#[test]
fn add_a_keys_scan_on_the_xprv_form_loads_a_master_key_and_refuses_a_child_by_name() {
    let mut app = adding(Form::Xprv);
    app.press(Action::ScanPart);
    read(&mut app, &tprv(CHILD));
    let said = app
        .scan()
        .and_then(|s| s.note.as_ref().map(|t| t.as_str().to_string()))
        .unwrap_or_default();
    assert!(said.contains("master key"), "{said}");
    read(&mut app, &tprv(XPRV));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].fingerprint.0, FINGERPRINT);
    assert!(app.scan().is_none(), "the camera closes once it is loaded");
}

#[test]
fn a_master_xprv_read_at_homes_scan_loads_its_key() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Home));
    app.press(Action::Scan);
    read(&mut app, &tprv(XPRV));
    assert_eq!(app.session.keys.len(), 1, "{}", error(&app));
    assert_eq!(app.session.keys[0].fingerprint.0, FINGERPRINT);
}

#[test]
fn a_child_xprv_read_at_homes_scan_is_refused_by_name_on_the_form() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Home));
    app.press(Action::Scan);
    read(&mut app, &tprv(CHILD));
    assert!(app.session.keys.is_empty());
    assert!(error(&app).contains("master key"), "{}", error(&app));
}

const HASH: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

#[test]
fn a_hex_code_read_at_homes_scan_is_filed_as_text_and_loads_no_key() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Home));
    app.press(Action::Scan);
    read(&mut app, HASH);
    assert!(app.session.keys.is_empty());
    assert_eq!(app.inbox().len(), 1);
    assert_eq!(app.inbox()[0].kind, faraday_core::wallet::FileKind::Text);
}

#[test]
fn a_hex_code_kept_from_a_read_card_is_filed_as_text() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::ScanSeed);
    read(&mut app, HASH);
    let ways: Vec<String> = app
        .scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| {
            c.ways
                .iter()
                .map(|w| w.label().as_str().to_string())
                .collect()
        })
        .unwrap_or_default();
    let keep = ways
        .iter()
        .position(|w| w == "Keep in Files")
        .unwrap_or_else(|| panic!("no Keep in Files: {ways:?}"));
    app.press(Action::ReadGo(keep as u8));
    assert_eq!(app.inbox().len(), 1);
    assert_eq!(app.inbox()[0].kind, faraday_core::wallet::FileKind::Text);
    assert!(app.session.keys.is_empty());
}

#[test]
fn a_hex_code_read_at_homes_scan_with_a_stick_in_is_filed_as_text() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Home));
    app.press(Action::Scan);
    app.storage(faraday_core::StorageEvent::Sticks(vec![
        faraday_core::StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("a"),
            label: faraday_text::Text::of("STICK"),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        },
    ]));
    read(&mut app, HASH);
    assert!(app.session.keys.is_empty());
    assert_eq!(app.inbox().len(), 1);
    assert_eq!(app.inbox()[0].kind, faraday_core::wallet::FileKind::Text);
}

#[test]
fn a_master_xprv_read_at_homes_scan_with_a_stick_in_says_to_remove_it() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Home));
    app.press(Action::Scan);
    app.storage(faraday_core::StorageEvent::Sticks(vec![
        faraday_core::StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("a"),
            label: faraday_text::Text::of("STICK"),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        },
    ]));
    read(&mut app, &tprv(XPRV));
    assert!(app.session.keys.is_empty());
    let said = app
        .scan()
        .and_then(|s| s.note.as_ref().map(|t| t.as_str().to_string()))
        .unwrap_or_default();
    assert!(
        said.starts_with("Keys load only with no"),
        "{said:?} {:?} {:?}",
        app.scan().is_some(),
        app.sheet
    );
}
