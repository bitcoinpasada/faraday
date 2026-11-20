//! Scanning a Krux encrypted code (`docs/KEF.md` §6, §7, §8, §15): the card
//! it gets, the passphrase it asks, and where what comes out goes. Krux's
//! own codes (`tools/vectors/kef/`) are read, and codes Faraday seals
//! carry the text and the bytes.

use crate::common::{has, has_part, offered, texts, type_text};
use crate::vault_seal::{device as vault_device, open_and_change};
use faraday_codec::kef::{Form, encode, wrap};
use faraday_core::seeds::kef::KefAct;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, testkit};
use faraday_keys::kef::unwrap;
use faraday_shell_api::{App, Command, EntropyBytes, Event, Key};
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
        .map(|c| {
            c.ways
                .iter()
                .map(|w| String::from(w.label().as_str()))
                .collect()
        })
        .unwrap_or_default()
}

fn facts(app: &Faraday) -> Vec<String> {
    app.scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| c.facts.iter().map(|f| String::from(f.as_str())).collect())
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
    app.kef_take_sealed().unwrap().unwrap().to_vec()
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
    let before = testkit::jobs_asked(&app);
    read(&mut app, &code);
    // Only the short timed run went out: no open has started.
    assert_eq!(testkit::jobs_asked(&app), before + 1);
    assert!(app.busy().is_none());
    let f = facts(&app).join(" | ");
    assert!(f.contains("Version 20"), "{f}");
    assert!(f.contains("Id test ID"), "{f}");
    assert!(f.contains("100000 rounds"), "{f}");
    assert_eq!(ways(&app), ["Not encrypted", "Open"]);
    assert!(app.kef_opened().is_none());
    assert!(app.session.keys.is_empty());
}

#[test]
fn the_read_card_shows_the_count_first_and_the_time_when_it_is_worked_out() {
    let (code, _) = vector("gcm");
    let mut app = scanner();
    read(&mut app, &code);
    let first = facts(&app).join(" | ");
    assert!(first.contains("100000 rounds"), "{first}");
    assert!(!first.contains("under a second"), "{first}");
    testkit::settle_time(&mut app);
    let later = facts(&app).join(" | ");
    assert!(later.contains("100000 rounds"), "{later}");
    assert!(later.contains("under a second"), "{later}");
}

#[test]
fn pressing_open_does_not_wait_for_the_time_nor_is_refused_by_it() {
    let (code, pass) = vector("gcm");
    let mut app = scanner();
    testkit::hold_worker();
    read(&mut app, &code);
    go(&mut app, "Open");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    type_text(&mut app, &pass);
    app.event(Event::Key(Key::Enter));
    assert!(app.busy().is_some(), "the open was refused");
    testkit::release_worker();
    for _ in 0..6 {
        testkit::settle(&mut app);
        if app.busy().is_none() {
            break;
        }
    }
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    // The timed run was stopped when the open began, so no time was
    // worked out: reading the code again sends a new timed run.
    testkit::settle_time(&mut app);
    app.press(Action::Cancel);
    app.press(Action::Scan);
    let before = testkit::jobs_asked(&app);
    read(&mut app, &code);
    assert_eq!(testkit::jobs_asked(&app), before + 1, "a rate was kept");
}

#[test]
fn the_sheet_gains_the_time_when_it_comes_after_open() {
    let (code, _) = vector("gcm");
    let mut app = scanner();
    testkit::hold_worker();
    read(&mut app, &code);
    go(&mut app, "Open");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    assert!(has_part(&mut app, "100000 rounds"));
    assert!(!has_part(&mut app, "under a second"));
    testkit::release_worker();
    testkit::settle_time(&mut app);
    assert!(has_part(&mut app, "under a second"));
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
fn sixteen_bytes_of_text_are_a_note_not_a_seed() {
    let mut app = scanner();
    let code = sealed(&mut app, b"sixteen byte txt");
    read(&mut app, code.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    assert!(has(&mut app, "Decrypted text"));
}

/// Opens a code sealed around `plain` to the sheet that shows it.
fn opened_note(app: &mut Faraday, plain: &[u8]) {
    let code = sealed(app, plain);
    read(app, code.as_bytes());
    go(app, "Open");
    open_with(app, "note key");
}

#[test]
fn sixteen_bytes_of_text_offer_to_be_read_as_a_seed() {
    let mut app = scanner();
    opened_note(&mut app, b"sixteen byte txt");
    assert!(has(&mut app, "Decrypted text"));
    assert!(offered(&mut app, Action::KefRead(KefAct::ReadAsSeed)));
    assert!(has(&mut app, "Read as a seed"));
    app.press(Action::KefRead(KefAct::ReadAsSeed));
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    // The seed is the one those bytes are, not any seed.
    let m = faraday_keys::words::from_compact_seedqr(
        b"sixteen byte txt",
        faraday_bip::bip39::Language::English,
    )
    .unwrap();
    let mut shown = String::new();
    for &i in m.indices() {
        shown.push_str(faraday_bip::bip39::Language::English.words()[usize::from(i)]);
        shown.push(' ');
    }
    let mut want = scanner();
    read(&mut want, shown.trim().as_bytes());
    // The same seed: the title names how it was read, the rest is equal.
    assert_eq!(texts(&mut app)[1..], texts(&mut want)[1..]);
}

#[test]
fn sixteen_bytes_with_a_control_character_go_to_the_seed_sheet() {
    let mut app = scanner();
    opened_note(&mut app, b"sixteen byte\x01txt");
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
}

#[test]
fn a_note_of_another_length_does_not_offer_to_be_a_seed() {
    let mut app = scanner();
    opened_note(&mut app, b"twenty bytes of txt!");
    assert!(has(&mut app, "Decrypted text"));
    assert!(!offered(&mut app, Action::KefRead(KefAct::ReadAsSeed)));
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

const WORDS: &[u8] = b"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// What the seed sheet shows for a seed scanned in a fresh scanner.
fn seed_sheet_texts(app: &mut Faraday) -> Vec<String> {
    read(app, WORDS);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    texts(app)
}

fn stick() -> StorageEvent {
    StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("stick"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }])
}

#[test]
fn a_note_used_as_a_passphrase_is_forgotten_when_the_scanner_is_cancelled() {
    let mut plain = scanner();
    let expected = seed_sheet_texts(&mut plain);
    let mut app = scanner();
    let code = sealed(&mut app, b"correct horse battery");
    read(&mut app, code.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    app.press(Action::KefRead(KefAct::UseAsPassphrase));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app.press(Action::Cancel);
    app.press(Action::Scan);
    assert_eq!(seed_sheet_texts(&mut app), expected);
}

/// The sheet asking a passphrase for a Krux code, nothing typed.
fn asking(app: &mut Faraday) {
    let (code, _) = vector("ctr");
    read(app, &code);
    go(app, "Open");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
}

/// The sheet while the key of a slow code is worked out.
fn working(app: &mut Faraday) {
    let code = rewrapped("ctr", 20_000_000);
    read(app, &code);
    go(app, "Open");
    go(app, "Open");
    type_text(app, "key");
    app.event(Event::Key(Key::Enter));
    assert!(app.busy().is_some(), "the work did not start");
}

/// The sheet is gone, no job is left and nothing it held is kept.
fn closed(app: &mut Faraday) {
    assert_ne!(app.sheet, Some(Sheet::KefRead));
    assert!(app.busy().is_none());
    assert!(app.kef_opened().is_none());
    assert!(app.session.keys.is_empty());
}

#[test]
fn a_stick_plugged_in_while_the_passphrase_is_asked_closes_the_sheet() {
    let mut app = scanner();
    asking(&mut app);
    app.storage(stick());
    closed(&mut app);
    // The read card still waits in the scanner and counts as held, so the
    // stick is not handed out: Lock is asked, as after a typed passphrase.
    assert_eq!(app.sheet, Some(Sheet::Lock));
}

#[test]
fn a_stick_plugged_in_after_a_passphrase_was_typed_locks_and_closes_the_sheet() {
    let mut app = scanner();
    asking(&mut app);
    type_text(&mut app, "key");
    app.storage(stick());
    closed(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Lock));
}

#[test]
fn storage_only_the_kernel_shows_closes_the_passphrase_sheet() {
    let mut app = scanner();
    asking(&mut app);
    app.storage(StorageEvent::Attached(true));
    closed(&mut app);
}

#[test]
fn storage_only_the_kernel_shows_after_a_passphrase_was_typed_locks() {
    let mut app = scanner();
    asking(&mut app);
    type_text(&mut app, "key");
    app.storage(StorageEvent::Attached(true));
    closed(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Lock));
}

#[test]
fn a_stick_plugged_in_while_the_code_opens_stops_the_work_and_closes_the_sheet() {
    let mut app = scanner();
    working(&mut app);
    app.storage(stick());
    testkit::settle_stale(&mut app);
    closed(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Lock));
}

#[test]
fn a_passphrase_typed_and_cancelled_still_locks_when_a_stick_comes() {
    let mut app = scanner();
    asking(&mut app);
    type_text(&mut app, "key");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    app.storage(stick());
    assert_eq!(app.sheet, Some(Sheet::Lock));
}

#[test]
fn a_note_shown_and_closed_still_locks_when_a_stick_comes() {
    let mut app = scanner();
    let code = sealed(&mut app, b"correct horse battery");
    read(&mut app, code.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    app.storage(stick());
    assert_eq!(app.sheet, Some(Sheet::Lock));
}

#[test]
fn open_waits_on_the_card_while_a_stick_is_plugged_in() {
    let mut app = scanner();
    let (code, _) = vector("ctr");
    read(&mut app, &code);
    app.storage(stick());
    go(&mut app, "Open");
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.scan().is_some_and(|s| s.card.is_some()));
}

#[test]
fn scan_again_with_a_card_held_and_a_stick_in_asks_for_lock_and_turns_the_camera_on_when_the_stick_is_pulled()
 {
    let mut app = scanner();
    let (code, _) = vector("ctr");
    read(&mut app, &code);
    app.storage(stick());
    while app.poll_command().is_some() {}
    app.press(Action::ReadAgain);
    assert!(!std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::CameraOn));
    // The read card is a held secret, so the press waits under the Lock
    // sheet, not Pull.
    assert_eq!(app.sheet, Some(Sheet::Lock));
    assert!(app.scan().is_some_and(|s| s.card.is_some()));
    // The stick coming out closes the Lock ask and carries the press on:
    // the camera turns on and the card is gone, as with no stick in.
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert!(std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::CameraOn));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.scan().is_some_and(|s| s.card.is_none()));
    assert!(!app.clean());
}

#[test]
fn lock_pressed_on_the_scan_again_ask_locks() {
    let mut app = scanner();
    let (code, _) = vector("ctr");
    read(&mut app, &code);
    app.storage(stick());
    app.press(Action::ReadAgain);
    assert_eq!(app.sheet, Some(Sheet::Lock));
    while app.poll_command().is_some() {}
    app.press(Action::Lock);
    // Locking wipes the card and does not turn the camera on.
    assert!(!std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::CameraOn));
    assert!(app.scan().is_none_or(|s| s.card.is_none()));
}

/// A note carried as a passphrase, the seed scanner up.
fn carried_note(app: &mut Faraday) {
    let code = sealed(app, b"correct horse battery");
    read(app, code.as_bytes());
    go(app, "Open");
    open_with(app, "note key");
    app.press(Action::KefRead(KefAct::UseAsPassphrase));
    assert_eq!(app.sheet, Some(Sheet::Scan));
}

#[test]
fn a_note_carried_is_filled_in_when_an_encrypted_seed_code_is_opened_next() {
    let mut app = scanner();
    carried_note(&mut app);
    let (code, pass) = vector("ctr");
    read(&mut app, &code);
    go(&mut app, "Open");
    open_with(&mut app, &pass);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    let s = app.seed_read().expect("the seed sheet");
    assert!(s.adding);
    assert_eq!(s.pass.as_str(), "correct horse battery");
}

#[test]
fn a_note_carried_is_not_filled_in_after_a_codex32_string_or_a_slip39_share_was_read() {
    let mut plain = scanner();
    let expected = seed_sheet_texts(&mut plain);
    for other in [
        "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw",
        "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed",
    ] {
        let mut app = scanner();
        carried_note(&mut app);
        read(&mut app, other.as_bytes());
        assert_ne!(app.sheet, Some(Sheet::Scan), "{other}");
        app.press(Action::Scan);
        assert_eq!(seed_sheet_texts(&mut app), expected, "{other}");
    }
}

#[test]
fn a_note_carried_is_forgotten_when_a_stick_comes_and_goes_over_the_krux_sheet() {
    let mut plain = scanner();
    let expected = seed_sheet_texts(&mut plain);
    let mut app = scanner();
    carried_note(&mut app);
    let (code, _) = vector("ctr");
    read(&mut app, &code);
    go(&mut app, "Open");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    app.storage(stick());
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Scan);
    assert_eq!(seed_sheet_texts(&mut app), expected);
}

#[test]
fn a_note_carried_is_forgotten_when_the_next_krux_code_holds_something_public() {
    let mut plain = scanner();
    let expected = seed_sheet_texts(&mut plain);
    let mut app = scanner();
    carried_note(&mut app);
    let code = sealed(&mut app, b"bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq");
    read(&mut app, code.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    assert_ne!(app.sheet, Some(Sheet::KefRead));
    assert_ne!(app.sheet, Some(Sheet::SeedRead));
    if app.sheet == Some(Sheet::Scan) {
        app.press(Action::ReadAgain);
    } else {
        app.press(Action::Scan);
    }
    assert_eq!(seed_sheet_texts(&mut app), expected);
}

#[test]
fn a_note_carried_is_filled_in_when_the_next_krux_code_holds_a_seed_as_words() {
    let mut app = scanner();
    carried_note(&mut app);
    let code = sealed(&mut app, WORDS);
    read(&mut app, code.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    let s = app.seed_read().expect("the seed sheet");
    assert_eq!(s.pass.as_str(), "correct horse battery");
}

#[test]
fn a_note_carried_is_filled_in_when_a_seeds_card_adds_its_key() {
    let mut app = scanner();
    carried_note(&mut app);
    let words = String::from_utf8_lossy(WORDS).replace(' ', "\n");
    read(&mut app, words.as_bytes());
    assert!(app.scan().is_some_and(|s| s.card.is_some()));
    go(&mut app, "Add this key");
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    let s = app.seed_read().expect("the seed sheet");
    assert_eq!(s.pass.as_str(), "correct horse battery");
}

#[test]
fn a_passphrase_carried_from_a_note_is_not_filled_in_after_another_code_was_read() {
    let mut plain = scanner();
    let expected = seed_sheet_texts(&mut plain);
    let mut app = scanner();
    let code = sealed(&mut app, b"correct horse battery");
    read(&mut app, code.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    app.press(Action::KefRead(KefAct::UseAsPassphrase));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    // A code that is no seed, and its way over.
    read(&mut app, b"some words that are no seed at all");
    let way = ways(&app).into_iter().next().expect("a way over");
    go(&mut app, &way);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    assert_eq!(seed_sheet_texts(&mut app), expected);
}

#[test]
fn codes_inside_codes_stop_being_offered_at_the_depth_limit() {
    let mut app = scanner();
    // Raw layers, so the size stays small; the innermost code is text.
    let text = sealed(&mut app, b"the note at the bottom");
    let mut code = sealed_raw(&mut app, text.as_bytes());
    for _ in 0..7 {
        code = sealed_raw(&mut app, &code);
    }
    read(&mut app, &code);
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    let mut opened = 1;
    while app.sheet == Some(Sheet::KefRead) && offered(&mut app, Action::KefRead(KefAct::Open)) {
        app.press(Action::KefRead(KefAct::Open));
        open_with(&mut app, "note key");
        opened += 1;
        assert!(opened <= 8, "opened past the depth limit");
    }
    assert_eq!(opened, 8);
    assert_ne!(app.sheet, Some(Sheet::KefRead));
    assert!(!ways(&app).iter().any(|w| w == "Open"), "{:?}", ways(&app));
}

#[test]
fn a_count_above_what_krux_writes_asks_again_on_open_as_an_encrypted_code() {
    let mut app = scanner();
    let mut code = None;
    for len in 1..64 {
        if let Ok(b) = wrap(b"", 20, 600_000, &vec![0xab; len])
            && b.len() == 32
        {
            code = Some(b);
            break;
        }
    }
    read(&mut app, &code.expect("a 32-byte envelope"));
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Kef));
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    type_text(&mut app, "key");
    app.event(Event::Key(Key::Enter));
    assert!(
        app.busy().is_none(),
        "the work started without the question"
    );
    assert!(has(&mut app, "Open anyway"), "{:?}", texts(&mut app));
    app.event(Event::Key(Key::Enter));
    assert!(
        app.busy().is_some(),
        "the second press did not start the work"
    );
}

#[test]
fn text_that_is_a_code_after_not_encrypted_does_not_open_another_krux_card() {
    let mut app = scanner();
    let inner = sealed(&mut app, b"inner words");
    let outer = sealed(&mut app, inner.as_bytes());
    read(&mut app, outer.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    app.press(Action::KefRead(KefAct::NotEncrypted));
    assert_ne!(app.sheet, Some(Sheet::KefRead));
    assert!(!ways(&app).iter().any(|w| w == "Open"), "{:?}", ways(&app));
}

#[test]
fn a_code_inside_a_code_that_is_seed_bytes_offers_to_open_as_a_code() {
    // The inner raw code is 32 bytes: a seed and an envelope at once.
    let mut app = scanner();
    let mut inner = None;
    for len in 1..64 {
        if let Ok(b) = wrap(b"", 20, 10_000, &vec![0xab; len])
            && b.len() == 32
        {
            inner = Some(b);
            break;
        }
    }
    let outer = sealed(&mut app, &inner.expect("a 32-byte envelope"));
    read(&mut app, outer.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    let shown = texts(&mut app);
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "{shown:?}");
    assert!(has(&mut app, "Open as an encrypted code"));
}

#[test]
fn cancel_on_a_lock_sheet_over_a_working_sheet_drops_the_answer() {
    let mut app = vault_device(crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7));
    open_and_change(&mut app);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    working(&mut app);
    app.press(Action::Lock);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.press(Action::Cancel);
    testkit::settle_stale(&mut app);
    assert!(app.busy().is_none());
    assert!(app.kef_opened().is_none());
    assert_eq!(app.vaults().open.len(), 1);
}

#[test]
fn a_seal_running_is_not_stopped_by_a_cancel_while_a_krux_sheet_is_left_over() {
    let mut app = vault_device(crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7));
    open_and_change(&mut app);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    asking(&mut app);
    fresh(&mut app);
    app.kef_seal(
        Zeroizing::new(b"note key".to_vec()),
        Zeroizing::new(b"plain".to_vec()),
        20,
        faraday_core::worker::kef::KefId::Random(8),
        200_000,
        "Working",
    )
    .expect("fresh bytes");
    app.press(Action::LockAsk);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.press(Action::Cancel);
    // The seal goes on: its work is not the Krux sheet's to stop.
    assert!(
        app.busy()
            .is_some_and(|b| b.kind == faraday_core::worker::Kind::KefSeal)
    );
    testkit::settle(&mut app);
    assert!(app.busy().is_none());
}

/// A Krux code sealed around 32 bytes of seed with `id`, read back in a
/// fresh scanner that holds the key of `loaded`'s words; what its seed sheet shows.
fn seed_code_sheet(entropy: u8, id: &[u8], loaded: &str) -> Vec<String> {
    let mut app = scanner();
    app.session
        .keys
        .add_words_with(&testkit::compute(), loaded, "", "Held", None)
        .unwrap();
    fresh(&mut app);
    app.kef_seal(
        Zeroizing::new(b"key".to_vec()),
        Zeroizing::new(vec![entropy; 32]),
        20,
        faraday_core::worker::kef::KefId::Public(id.to_vec()),
        10_000,
        "Working",
    )
    .expect("fresh bytes");
    testkit::settle(&mut app);
    let code = app.kef_take_sealed().unwrap().unwrap().to_vec();
    read(&mut app, &code);
    go(&mut app, "Open");
    open_with(&mut app, "key");
    let shown = texts(&mut app);
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "{shown:?}");
    shown
}

#[test]
fn a_code_whose_id_is_its_words_fingerprint_says_the_id_matches_the_key() {
    let words_of = |b: u8| {
        let m = faraday_keys::words::from_compact_seedqr(
            &[b; 32],
            faraday_bip::bip39::Language::English,
        )
        .unwrap();
        let list = faraday_bip::bip39::Language::English.words();
        m.indices()
            .iter()
            .map(|&i| list[usize::from(i)])
            .collect::<Vec<_>>()
            .join(" ")
    };
    let (mine, other) = (words_of(0xa1), words_of(0xb2));
    let mut held = testkit::started();
    held.session
        .keys
        .add_words_with(&testkit::compute(), &mine, "", "Held", None)
        .unwrap();
    let fp = held.session.keys.last().unwrap().fingerprint;
    let id = String::from_utf8(fp.to_hex().to_vec()).unwrap();
    let shown = seed_code_sheet(0xa1, id.as_bytes(), &mine);
    assert!(
        shown
            .iter()
            .any(|t| *t == format!("Code id {id} matches this key")),
        "{shown:?}"
    );
    // The same id on a code of another key's words does not match.
    let shown = seed_code_sheet(0xb2, id.as_bytes(), &other);
    assert!(
        shown
            .iter()
            .any(|t| *t == format!("Code id {id} differs from this key")),
        "{shown:?}"
    );
    assert!(!shown.iter().any(|t| t.contains("matches this key")));
}

/// The Krux tool, opened from its tile.
fn krux_tool() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Krux code")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::KefTool);
    app
}

/// Add a key, on its Krux code form.
fn krux_form() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    let at = faraday_core::forms::Form::ALL
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Krux)
        .unwrap();
    app.press(Action::EntryForm(at as u8));
    app
}

#[test]
fn the_tool_scans_a_krux_code_and_loads_its_key() {
    let (code, pass) = vector("gcm");
    let mut app = krux_tool();
    assert!(has(&mut app, "Scan a Krux code"));
    assert!(has(&mut app, "From Files"));
    app.press(Action::ScanKef);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    read(&mut app, &code);
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    open_with(&mut app, &pass);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert!(has_part(&mut app, "Code id test ID"));
}

#[test]
fn the_tool_scan_with_a_wrong_passphrase_loads_no_key() {
    let (code, _) = vector("gcm");
    let mut app = krux_tool();
    app.press(Action::ScanKef);
    read(&mut app, &code);
    open_with(&mut app, "not the key");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    assert!(app.session.keys.is_empty());
}

#[test]
fn the_krux_scanner_says_a_code_that_is_none_and_stays_on() {
    let mut app = krux_tool();
    app.press(Action::ScanKef);
    read(&mut app, b"hello there");
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(has(&mut app, "That is not a Krux code"));
}

#[test]
fn the_tool_takes_a_krux_code_from_files() {
    let (code, pass) = vector("gcm");
    let mut app = krux_tool();
    testkit::inbox_file(&mut app, "krux.kef", code);
    assert!(has(&mut app, "From Files · krux.kef"));
    app.press(Action::KefFile(0));
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    open_with(&mut app, &pass);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert!(has_part(&mut app, "Code id test ID"));
}

#[test]
fn a_file_that_is_no_krux_code_is_named() {
    let mut app = krux_tool();
    testkit::inbox_file(&mut app, "note.txt", b"just a note".to_vec());
    app.press(Action::KefFile(0));
    assert_eq!(app.sheet, None);
    assert!(has(&mut app, "That file is not a Krux code"));
}

#[test]
fn add_a_key_reads_a_krux_code_by_scan() {
    let (code, pass) = vector("ctr");
    let mut app = krux_form();
    assert!(has(&mut app, "Scan a Krux code"));
    app.press(Action::ScanKef);
    read(&mut app, &code);
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    open_with(&mut app, "not the key");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    assert!(app.session.keys.is_empty());
    for _ in 0..11 {
        app.event(Event::Key(Key::Backspace));
    }
    open_with(&mut app, &pass);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
}

#[test]
fn add_a_key_reads_a_krux_code_from_files() {
    let (code, pass) = vector("gcm");
    let mut app = krux_form();
    assert!(has(&mut app, "From Files"));
    testkit::inbox_file(&mut app, "backup.kef", code);
    assert!(has(&mut app, "From Files · backup.kef"));
    app.press(Action::KefFile(0));
    open_with(&mut app, &pass);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert!(has_part(&mut app, "Code id test ID"));
}

#[test]
fn a_krux_code_file_on_a_stick_shows_in_files_and_opens_there() {
    let (code, pass) = vector("gcm");
    let mut app = testkit::started();
    testkit::inbox_file(&mut app, "krux-v20-74657374.kef", code);
    app.press(Action::Nav(Screen::Files));
    let shown = texts(&mut app);
    assert!(
        shown.iter().any(|t| t.contains("Krux encrypted code")),
        "{shown:?}"
    );
    assert!(offered(&mut app, Action::KefFile(0)));
    app.press(Action::KefFile(0));
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    open_with(&mut app, "not the key");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    assert!(app.session.keys.is_empty());
    for _ in 0..11 {
        app.event(Event::Key(Key::Backspace));
    }
    open_with(&mut app, &pass);
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert!(has_part(&mut app, "Code id test ID"));
}

#[test]
fn checks_krux_row_opens_add_a_key_on_its_krux_form() {
    use faraday_core::check::{Backup, Press, Row};
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Check));
    app.press(Action::Check(Press::Row(Row::Backup)));
    app.press(Action::Check(Press::Form(Backup::Krux)));
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(testkit::entry(&app).form, faraday_core::forms::Form::Krux);
    assert!(has(&mut app, "Scan a Krux code"));
}

#[test]
fn new_keys_start_card_offers_a_krux_code_that_opens_its_form() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::KeyGen(Some(0)));
    assert!(has(&mut app, "Krux code"));
    app.press(Action::KStart(faraday_core::keygen::Start::Krux.index()));
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(testkit::entry(&app).form, faraday_core::forms::Form::Krux);
    assert!(has(&mut app, "Scan a Krux code"));
    // The code read there fills the slot it was opened for, as Scan a
    // SeedQR on the same card does.
    let code = sealed_raw(&mut app, &[0xa1; 32]);
    app.press(Action::ScanKef);
    read(&mut app, &code);
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
    assert_eq!(app.screen, Screen::Create);
    assert!(matches!(
        app.create().unwrap().slots[0],
        faraday_core::create::Source::Here(_)
    ));
}

/// Create's multisig kind, on its keys, at Scan Key 2's scanner.
fn create_key_2_scanner() -> Faraday {
    use faraday_core::create::NewKind;
    use faraday_core::cstep;
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CStep(cstep::KIND));
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    crate::common::ckind(&mut app, multi);
    app.press(Action::CNext(cstep::QUORUM));
    app.press(Action::ScanFor(faraday_core::Ask::Xpub(2)));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app
}

fn key_2_filled(app: &Faraday) -> bool {
    matches!(
        app.create().unwrap().slots.get(1),
        Some(faraday_core::create::Source::Here(_))
    )
}

/// The key typed on Add a key was loaded.
fn typed_key_loaded(app: &Faraday) -> bool {
    !app.session.keys.is_empty()
}

/// Create's multisig kind with Key 2 on New key's Krux code form.
fn key_2_krux_form() -> Faraday {
    use faraday_core::create::NewKind;
    use faraday_core::cstep;
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CStep(cstep::KIND));
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    crate::common::ckind(&mut app, multi);
    app.press(Action::CNext(cstep::QUORUM));
    app.press(Action::KeyGen(Some(1)));
    app.press(Action::KStart(faraday_core::keygen::Start::Krux.index()));
    assert_eq!(app.screen, Screen::Entry);
    app
}

/// Reads the code at the Krux form's scanner and loads its key.
fn load_from_the_krux_form(app: &mut Faraday, code: &[u8]) {
    app.press(Action::ScanKef);
    read(app, code);
    open_with(app, "note key");
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
}

#[test]
fn a_krux_code_cancelled_on_the_seed_sheet_leaves_no_slot_waiting() {
    let mut app = create_key_2_scanner();
    let code = sealed_raw(&mut app, &[0xa1; 32]);
    read(&mut app, &code);
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::Cancel);
    assert_eq!(app.screen, Screen::Create);
    // A key added by typing its words is not Key 2's.
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert!(typed_key_loaded(&app));
    assert!(!key_2_filled(&app));
}

#[test]
fn a_krux_code_cancelled_on_the_scanner_keeps_key_2_waiting() {
    let mut app = key_2_krux_form();
    let code = sealed_raw(&mut app, &[0xa1; 32]);
    app.press(Action::ScanKef);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app.press(Action::Cancel);
    assert_eq!(app.screen, Screen::Entry);
    load_from_the_krux_form(&mut app, &code);
    assert!(key_2_filled(&app));
}

#[test]
fn a_krux_code_cancelled_on_the_seed_sheet_keeps_key_2_waiting_for_a_second_read() {
    let mut app = key_2_krux_form();
    let code = sealed_raw(&mut app, &[0xa1; 32]);
    app.press(Action::ScanKef);
    read(&mut app, &code);
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::Cancel);
    assert_eq!(app.screen, Screen::Entry);
    load_from_the_krux_form(&mut app, &code);
    assert!(key_2_filled(&app));
}

#[test]
fn a_krux_code_of_words_read_at_key_2_fills_key_2() {
    let mut app = create_key_2_scanner();
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    fresh(&mut app);
    app.kef_seal(
        Zeroizing::new(b"note key".to_vec()),
        Zeroizing::new(words.as_bytes().to_vec()),
        20,
        faraday_core::worker::kef::KefId::Public(b"my words".to_vec()),
        10_000,
        "Working",
    )
    .expect("fresh bytes");
    testkit::settle(&mut app);
    let code = encode(Form::Base43, &app.kef_take_sealed().unwrap().unwrap()).unwrap();
    read(&mut app, code.as_bytes());
    go(&mut app, "Open");
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert!(has_part(&mut app, "Code id my words"));
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
    assert!(key_2_filled(&app));
}

#[test]
fn cancel_on_a_krux_codes_passphrase_leaves_the_flow_where_it_was() {
    let mut app = create_key_2_scanner();
    let code = sealed_raw(&mut app, &[0xa1; 32]);
    read(&mut app, &code);
    go(&mut app, "Open");
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    app.press(Action::Cancel);
    assert_eq!(app.screen, Screen::Create);
    assert!(app.session.keys.is_empty());
    assert!(!key_2_filled(&app));
}

#[test]
fn back_out_of_a_krux_form_opened_for_a_slot_leaves_no_slot_waiting() {
    let mut app = key_2_krux_form();
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert!(typed_key_loaded(&app));
    assert!(!key_2_filled(&app));
}

#[test]
fn home_from_a_krux_form_opened_for_a_slot_leaves_no_slot_waiting() {
    let mut app = key_2_krux_form();
    app.press(Action::Nav(Screen::Home));
    assert_eq!(app.screen, Screen::Home);
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert!(typed_key_loaded(&app));
    assert!(!key_2_filled(&app));
}

/// My seed makes my wallet on its Seed card, with the Krux code row opened.
fn check_krux() -> Faraday {
    use faraday_core::seed_check::{FORMS, SeedAction as S};
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "My seed makes my wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    let at = FORMS
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Krux)
        .unwrap();
    app.press(Action::SeedCheck(S::Form(at as u8)));
    app
}

/// The fingerprint the check holds for the words typed, no passphrase.
fn typed_fingerprint() -> faraday_bip::keys::Fingerprint {
    use faraday_core::seed_check::SeedAction as S;
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "My seed makes my wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::SeedCheck(S::Type));
    testkit::type_words(&mut app, std::str::from_utf8(WORDS).unwrap());
    app.press(Action::SeedCheck(S::Words));
    app.press(Action::SeedCheck(S::NoPass));
    app.session.keys.check_facts().unwrap().fingerprint
}

#[test]
fn a_krux_code_scanned_on_the_seed_card_gives_the_fingerprint_of_the_words_typed() {
    use faraday_core::seed_check::SeedAction as S;
    let mut app = check_krux();
    assert!(has(&mut app, "Scan a Krux code"));
    app.press(Action::ScanKef);
    let code = sealed_raw(&mut app, WORDS);
    read(&mut app, &code);
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::SeedCheck);
    assert!(has(&mut app, "No passphrase"));
    app.press(Action::SeedCheck(S::NoPass));
    assert_eq!(
        app.session.keys.check_facts().unwrap().fingerprint,
        typed_fingerprint()
    );
}

#[test]
fn a_krux_code_from_files_on_the_seed_card_gives_the_fingerprint_of_the_words_typed() {
    use faraday_core::seed_check::SeedAction as S;
    let mut app = check_krux();
    let code = sealed_raw(&mut app, WORDS);
    testkit::inbox_file(&mut app, "krux.kef", code);
    assert!(has(&mut app, "From Files · krux.kef"));
    app.press(Action::KefFile(0));
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    open_with(&mut app, "note key");
    assert_eq!(app.screen, Screen::SeedCheck);
    app.press(Action::SeedCheck(S::NoPass));
    assert_eq!(
        app.session.keys.check_facts().unwrap().fingerprint,
        typed_fingerprint()
    );
}

#[test]
fn cancel_on_the_passphrase_leaves_the_seed_card_with_nothing_taken() {
    let mut app = check_krux();
    app.press(Action::ScanKef);
    let code = sealed_raw(&mut app, WORDS);
    read(&mut app, &code);
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::SeedCheck);
    assert!(has(&mut app, "Scan a Krux code"));
    assert!(app.session.keys.check_facts().is_none());
    assert!(!app.session.keys.holds_secret());
    assert!(testkit::holds_nothing(&app));
}

#[test]
fn lock_with_the_passphrase_sheet_open_from_the_seed_card_leaves_nothing() {
    let mut app = check_krux();
    app.press(Action::ScanKef);
    let code = sealed_raw(&mut app, WORDS);
    read(&mut app, &code);
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    type_text(&mut app, "note key");
    app.press(Action::Lock);
    let _ = app.frame();
    assert!(testkit::holds_nothing(&app));
    assert!(app.session.keys.check_facts().is_none());
    assert!(!app.session.keys.holds_secret());
}

#[test]
fn only_a_kef_file_is_named_a_krux_code_in_files() {
    let (code, _) = vector("gcm");
    let mut app = testkit::started();
    testkit::inbox_file(&mut app, "krux-v20-74657374.KEF", code.clone());
    testkit::inbox_file(&mut app, "renamed.bin", code);
    app.press(Action::Nav(Screen::Files));
    assert!(offered(&mut app, Action::KefFile(0)));
    assert!(!offered(&mut app, Action::KefFile(1)));
}

#[test]
fn a_refused_psbt_is_not_a_krux_code_in_files() {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/vectors/bip375/bip375_test_vectors.json"
    ))
    .unwrap();
    let text = v["invalid"][15]["psbt"].as_str().unwrap();
    let bytes = faraday_bip::base64::decode(text).unwrap().to_vec();
    let mut app = testkit::started();
    testkit::inbox_file(&mut app, "invalid.psbt", bytes);
    app.press(Action::Nav(Screen::Files));
    let shown = texts(&mut app);
    assert!(
        !shown.iter().any(|t| t.contains("Krux encrypted code")),
        "{shown:?}"
    );
    assert!(!offered(&mut app, Action::KefFile(0)), "{shown:?}");
}

#[test]
fn a_kef_file_is_dropped_at_lock() {
    let (code, _) = vector("gcm");
    let mut app = testkit::started();
    testkit::inbox_file(&mut app, "krux-v20-74657374.kef", code);
    while app.poll_storage().is_some() {}
    app.press(Action::Lock);
    let mut left = None;
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::SaveBoxes { inbox, .. } = c {
            left = Some(testkit::plain_files(inbox));
        }
    }
    assert!(left.unwrap().is_empty());
}

/// A code sealing `plain`, opened on the Seed card, gives the fingerprint of the words typed.
fn seed_card_opens_as_the_words_typed(plain: &[u8]) {
    use faraday_core::seed_check::SeedAction as S;
    let mut app = check_krux();
    app.press(Action::ScanKef);
    let code = sealed_raw(&mut app, plain);
    read(&mut app, &code);
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::SeedCheck);
    app.press(Action::SeedCheck(S::NoPass));
    assert_eq!(
        app.session.keys.check_facts().unwrap().fingerprint,
        typed_fingerprint()
    );
}

#[test]
fn a_krux_code_of_compact_seedqr_bytes_on_the_seed_card_gives_the_words_typed() {
    // Sixteen zero bytes are the twelve words of WORDS.
    seed_card_opens_as_the_words_typed(&[0u8; 16]);
}

#[test]
fn a_krux_code_of_standard_seedqr_digits_on_the_seed_card_gives_the_words_typed() {
    // Eleven times abandon (0000) and about (0003).
    seed_card_opens_as_the_words_typed(format!("{}0003", "0000".repeat(11)).as_bytes());
}

#[test]
fn a_multisig_checks_seed_card_lists_the_krux_row_and_a_code_read_there_reaches_the_check() {
    use faraday_core::multisig_check::MsAction as M;
    use faraday_core::seed_check::{FORMS, SeedAction as S};
    let descriptor = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .unwrap()
        .descriptor;
    let mut app = testkit::started();
    app.press(Action::MultisigCheck(M::Open));
    app.press(Action::MultisigCheck(M::Scan));
    app.event(Event::Scanned {
        bytes: descriptor.into_bytes(),
    });
    let _ = app.frame();
    app.press(Action::MultisigCheck(M::CheckSeed));
    let at = FORMS
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Krux)
        .unwrap();
    app.press(Action::SeedCheck(S::Form(at as u8)));
    assert!(has(&mut app, "Scan a Krux code"));
    app.press(Action::ScanKef);
    let code = sealed_raw(&mut app, WORDS);
    read(&mut app, &code);
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    open_with(&mut app, "note key");
    assert_eq!(app.sheet, None);
    assert!(has(&mut app, "No passphrase"));
    app.press(Action::SeedCheck(S::NoPass));
    testkit::settle(&mut app);
    assert_eq!(app.screen, Screen::MultisigCheck);
    assert!(has_part(&mut app, "Not a key of this wallet"));
    assert!(app.session.keys.all().next().is_none());
}
