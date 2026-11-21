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
    assert!(error(&app).contains("tprv"), "{}", error(&app));
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
