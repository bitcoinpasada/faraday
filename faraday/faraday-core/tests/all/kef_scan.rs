//! Scanning a Krux encrypted code (`docs/KEF.md` §6, §7, §8, §15): the card
//! it gets, the passphrase it asks, and where what comes out goes. Krux's
//! own codes (`tools/vectors/kef/`) are read, and codes Faraday seals
//! carry the text and the bytes.

use crate::common::{has, has_part, offered, texts, type_text};
use faraday_codec::kef::{Form, encode, wrap};
use faraday_core::seeds::kef::KefAct;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_keys::kef::unwrap;
use faraday_shell_api::{App, EntropyBytes, Event, Key};
use zeroize::Zeroizing;

fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../tools/vectors/kef/vectors.json")).unwrap()
}

/// A vector's envelope bytes and passphrase.
fn vector(name: &str) -> (Vec<u8>, String) {
    let v = &vectors()[name];
    let env = v["envelope"].as_str().unwrap();
    let bytes = (0..env.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&env[i..i + 2], 16).unwrap())
        .collect();
    (bytes, v["passphrase"].as_str().unwrap().to_string())
}

fn scanner() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app
}

fn read(app: &mut Faraday, code: &[u8]) {
    app.event(Event::Scanned {
        bytes: code.to_vec(),
    });
    let _ = app.frame();
}

fn ways(app: &Faraday) -> Vec<String> {
    app.scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| c.ways.iter().map(|w| w.label()).collect())
        .unwrap_or_default()
}

fn facts(app: &Faraday) -> Vec<String> {
    app.scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| c.facts.clone())
        .unwrap_or_default()
}

/// Presses the card's way named `label`.
fn go(app: &mut Faraday, label: &str) {
    let i = ways(app)
        .iter()
        .position(|w| w == label)
        .unwrap_or_else(|| panic!("no way {label}: {:?}", ways(app)));
    app.press(Action::ReadGo(i as u8));
}

fn open_with(app: &mut Faraday, pass: &str) {
    type_text(app, pass);
    app.event(Event::Key(Key::Enter));
    for _ in 0..6 {
        testkit::settle(app);
        if app.busy().is_none() {
            break;
        }
    }
    let _ = app.frame();
}

fn fresh(app: &mut Faraday) {
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
}

/// A code Faraday seals around `plain`, as text.
fn sealed(app: &mut Faraday, plain: &[u8]) -> String {
    encode(Form::Base43, &sealed_raw(app, plain)).unwrap()
}

fn sealed_raw(app: &mut Faraday, plain: &[u8]) -> Vec<u8> {
    fresh(app);
    app.kef_seal(
        Zeroizing::new(b"note key".to_vec()),
        Zeroizing::new(plain.to_vec()),
        20,
        faraday_core::worker::kef::KefId::Random(8),
        10_000,
        "Working",
    )
    .expect("fresh bytes");
    testkit::settle(app);
    app.kef_take_sealed().unwrap().unwrap()
}

fn rewrapped(name: &str, count: u32) -> Vec<u8> {
    let (bytes, _) = vector(name);
    let e = unwrap(&bytes).unwrap();
    wrap(&e.id, e.version, count, &e.payload).unwrap()
}

#[test]
fn a_krux_code_read_shows_its_version_id_and_count_and_asks_nothing_yet() {
    let (code, _) = vector("gcm");
    let mut app = scanner();
    let asked = testkit::jobs_asked(&app);
    read(&mut app, &code);
    let f = facts(&app).join(" | ");
    assert!(f.contains("Version 20"), "{f}");
    assert!(f.contains("Id test ID"), "{f}");
    assert!(f.contains("100000 rounds"), "{f}");
    assert_eq!(ways(&app), ["Not encrypted", "Open"]);
    assert_eq!(testkit::jobs_asked(&app), asked);
    assert!(app.session.keys.is_empty());
}

#[test]
fn a_krux_seed_code_opens_to_the_seed_sheet_with_its_id() {
    let mut app = scanner();
    for name in ["ctr", "gcm"] {
        let (code, pass) = vector(name);
        read(&mut app, &code);
        go(&mut app, "Open");
        assert_eq!(app.sheet, Some(Sheet::KefRead));
        open_with(&mut app, &pass);
        let shown = texts(&mut app);
        assert_eq!(app.sheet, Some(Sheet::SeedRead), "{name} {shown:?}");
        assert!(has_part(&mut app, "Code id test ID"), "{name}");
        app.press(Action::Cancel);
        app.press(Action::Scan);
    }
}

#[test]
fn a_wrong_passphrase_gives_one_message_and_asks_again() {
    let (code, _) = vector("gcm");
    let mut app = scanner();
    read(&mut app, &code);
    go(&mut app, "Open");
    open_with(&mut app, "not the key");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    assert!(has(
        &mut app,
        "Could not open. Wrong passphrase or damaged code."
    ));
    assert!(app.session.keys.is_empty());
}

#[test]
fn text_that_names_nothing_is_a_note_and_can_be_a_passphrase() {
    let mut app = scanner();
    let code = sealed(&mut app, b"correct horse battery");
    read(&mut app, code.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    assert!(has(&mut app, "Decrypted text"));
    assert!(!has_part(&mut app, "correct horse"));
    app.press(Action::KefRead(KefAct::Show));
    assert!(has_part(&mut app, "correct horse battery"));
    app.press(Action::KefRead(KefAct::UseAsPassphrase));
    assert_eq!(app.sheet, Some(Sheet::Scan));
}

#[test]
fn bytes_that_are_no_text_are_shown_as_hex_and_go_no_further() {
    let mut app = scanner();
    let code = sealed(&mut app, &[0xff, 0xfe, 0x01, 0x02, 0x03]);
    read(&mut app, code.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    assert!(has(&mut app, "Decrypted bytes"));
    app.press(Action::KefRead(KefAct::Show));
    assert!(has_part(&mut app, "fffe010203"));
    assert!(!offered(&mut app, Action::KefRead(KefAct::UseAsPassphrase)));
    assert!(app.session.keys.is_empty());
}

#[test]
fn this_is_not_encrypted_sends_the_code_on_as_an_unknown_one() {
    let mut app = scanner();
    let text = sealed(&mut app, b"x");
    read(&mut app, text.as_bytes());
    go(&mut app, "Not encrypted");
    let kind = app.scan().and_then(|s| s.card.as_ref()).map(|c| c.kind);
    assert!(
        kind.is_some_and(|k| !k.label().contains("encrypted")),
        "{kind:?}"
    );
    assert!(!ways(&app).contains(&"Open".to_string()));
}

#[test]
fn a_count_above_what_krux_writes_asks_once_more_before_the_sheet() {
    let code = rewrapped("ctr", 600_000);
    let mut app = scanner();
    read(&mut app, &code);
    let f = facts(&app).join(" | ");
    assert!(f.contains("More rounds than Krux writes"), "{f}");
    go(&mut app, "Open");
    assert_eq!(app.sheet, Some(Sheet::Scan), "the first press only asks");
    assert!(app.scan().and_then(|s| s.card.as_ref()).is_some());
    go(&mut app, "Open");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
}

#[test]
fn cancel_while_the_code_opens_stops_the_work_and_asks_the_passphrase_again() {
    let code = rewrapped("ctr", 20_000_000);
    let mut app = scanner();
    read(&mut app, &code);
    go(&mut app, "Open");
    go(&mut app, "Open");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    type_text(&mut app, "key");
    app.event(Event::Key(Key::Enter));
    assert!(app.busy().is_some(), "the work did not start");
    app.press(Action::Cancel);
    assert!(app.busy().is_none());
    testkit::settle_stale(&mut app);
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    assert!(offered(&mut app, Action::KefRead(KefAct::Open)));
    assert!(app.session.keys.is_empty());
}

#[test]
fn a_32_byte_code_that_is_also_a_seed_offers_to_open_as_an_encrypted_code() {
    // The raw envelope of Krux's 32-byte AES-CTR code is not 16 or 32 bytes,
    // so the code is made 32 bytes of Faraday's own: a KEF envelope can be
    // exactly that size, and then it is also a CompactSeedQR.
    let mut code = None;
    for len in 1..64 {
        let payload = vec![0xab; len];
        if let Ok(b) = wrap(b"", 20, 10_000, &payload)
            && b.len() == 32
        {
            code = Some(b);
            break;
        }
    }
    let code = code.expect("a 32-byte envelope");
    // By Home's Scan.
    let mut app = scanner();
    read(&mut app, &code);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert!(has(&mut app, "Open as an encrypted code"));
    // By Scan a SeedQR: the seed sheet carries the way.
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::ScanSeed);
    read(&mut app, &code);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert!(
        has(&mut app, "Open as an encrypted code"),
        "{:?}",
        texts(&mut app)
    );
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Kef));
    assert_eq!(app.sheet, Some(Sheet::KefRead));
}

#[test]
fn a_code_inside_a_code_is_offered_as_one_and_can_be_said_not_to_be() {
    let mut app = scanner();
    let inner = sealed_raw(&mut app, b"inner words");
    let outer = sealed(&mut app, &inner);
    read(&mut app, outer.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    // The contents parse as a code: offered, not asked at once.
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    assert!(offered(&mut app, Action::KefRead(KefAct::NotEncrypted)));
    assert!(offered(&mut app, Action::KefRead(KefAct::Open)));
    assert!(!has(&mut app, "Passphrase"));
    app.press(Action::KefRead(KefAct::NotEncrypted));
    // Past the offer the contents are bytes that are no text.
    assert!(
        has(&mut app, "Decrypted bytes") || has(&mut app, "Decrypted text"),
        "{:?}",
        texts(&mut app)
    );
    // The other way: Open asks the inner code's passphrase.
    let mut app = scanner();
    let inner = sealed_raw(&mut app, b"inner words");
    let outer = sealed(&mut app, &inner);
    read(&mut app, outer.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    app.press(Action::KefRead(KefAct::Open));
    assert!(has(&mut app, "Passphrase"));
    open_with(&mut app, "note key");
    assert!(has(&mut app, "Decrypted text"));
}
