//! Sign a message takes its text from a scanner: a scanned plain text is
//! the message and is signed as any typed one is; a seed or any other
//! kind of code is named and refused, and no text is kept. Home's Scan
//! of a text offers Sign this message only while a single-key wallet is
//! loaded (`docs/SCANNING.md` §4.3).

use faraday_core::scan_read::Way;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, Event};

const SEED: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// The started app with the first test key and, if `wallet`, a
/// single-key wallet.
fn app(wallet: bool) -> Faraday {
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
    if wallet {
        let kit = testkit::kits()
            .into_iter()
            .find(|k| k.id == "spending")
            .unwrap();
        app.session
            .add_wallet(
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

fn read(app: &mut Faraday, text: &str) {
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
}

fn read_bytes(app: &mut Faraday, bytes: &[u8]) {
    app.event(Event::Scanned {
        bytes: bytes.to_vec(),
    });
    let _ = app.frame();
}

fn stick_in(app: &mut Faraday, on: bool) {
    let sticks = if on {
        vec![faraday_core::StickInfo {
            make: Default::default(),
            serial: None,
            id: "a".into(),
            label: "STICK".into(),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        }]
    } else {
        Vec::new()
    };
    app.storage(faraday_core::StorageEvent::Sticks(sticks));
    let _ = app.frame();
}

fn note(app: &Faraday) -> String {
    app.scan()
        .and_then(|s| s.note.as_ref().map(|n| String::from(n.as_str())))
        .unwrap_or_default()
}

#[test]
fn a_scanned_text_is_the_message_and_is_signed() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    read(&mut app, "Pay the bearer of this message");
    assert_eq!(app.sheet, None, "the scanner closes on a text");
    assert_eq!(
        testkit::message(&app).unwrap().text,
        "Pay the bearer of this message"
    );
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert_eq!(m.error, None);
    assert!(m.signed.is_some());
}

#[test]
fn a_scanned_text_of_16_or_32_characters_is_the_message() {
    for text in ["Pay the bearer!!", "Pay the bearer of this message!!"] {
        let mut app = app(true);
        app.press(Action::SignMessage);
        app.press(Action::MScan);
        read(&mut app, text);
        assert_eq!(app.sheet, None, "{text}");
        assert_eq!(testkit::message(&app).unwrap().text, text);
    }
}

#[test]
fn a_private_key_of_16_or_32_characters_is_not_the_message() {
    for (text, kind) in [
        ("nsec1abcdefghijk", "Nostr secret key"),
        ("xprv9s21ZrQH143Kabcdefghijklmnop", "extended private key"),
    ] {
        let mut app = app(true);
        app.press(Action::SignMessage);
        app.press(Action::MScan);
        read(&mut app, text);
        assert_eq!(app.sheet, Some(Sheet::Scan), "{text}");
        assert!(note(&app).contains(kind), "{}", note(&app));
        assert_eq!(testkit::message(&app).unwrap().text, "");
    }
}

#[test]
fn a_scanned_seed_is_named_and_no_text_is_kept() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    read(&mut app, SEED);
    assert_eq!(app.sheet, Some(Sheet::Scan), "the scanner stays open");
    assert!(
        note(&app).contains("not used here"),
        "the seed is named: {}",
        note(&app)
    );
    assert_eq!(testkit::message(&app).unwrap().text, "");
}

#[test]
fn another_kind_of_code_is_named_and_refused_too() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    read(
        &mut app,
        "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn",
    );
    assert!(note(&app).contains("not used here"), "{}", note(&app));
    read(
        &mut app,
        "otpauth://totp/Scanned:me?secret=JBSWY3DPEHPK3PXP",
    );
    assert!(note(&app).contains("not used here"), "{}", note(&app));
    assert_eq!(testkit::message(&app).unwrap().text, "");
}

/// The read card's ways after a text read on Home's Scan; none where no
/// card shows.
fn ways_for_text(app: &mut Faraday) -> Vec<Way> {
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    read(app, "A note to sign");
    app.scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| c.ways.clone())
        .unwrap_or_default()
}

#[test]
fn homes_scan_of_a_text_offers_sign_this_message_only_with_a_wallet() {
    let mut without = app(false);
    assert!(!ways_for_text(&mut without).contains(&Way::SignMessage));
    let mut with = app(true);
    let ways = ways_for_text(&mut with);
    let at = ways
        .iter()
        .position(|w| *w == Way::SignMessage)
        .expect("Sign this message");
    with.press(Action::ReadGo(at as u8));
    assert_eq!(with.screen, Screen::Message);
    assert_eq!(testkit::message(&with).unwrap().text, "A note to sign");
}

#[test]
fn a_binary_code_is_refused_and_no_text_is_kept() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    read_bytes(&mut app, &[0x41, 0xFF, 0x42]);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(
        note(&app).contains("Nothing Faraday reads"),
        "{}",
        note(&app)
    );
    assert_eq!(testkit::message(&app).unwrap().text, "");
}

#[test]
fn a_binary_code_read_on_home_does_not_offer_sign_this_message() {
    let mut app = app(true);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    read_bytes(&mut app, &[0x41, 0xFF, 0x42]);
    let ways = app
        .scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| c.ways.clone())
        .unwrap_or_default();
    assert!(!ways.contains(&Way::SignMessage));
}

#[test]
fn a_psbt_and_a_url_are_named_and_refused() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    read(&mut app, "https://example.com/a/page");
    assert!(note(&app).contains("not used here"), "{}", note(&app));
    let (psbt, _) = testkit::spend().unwrap();
    read_bytes(&mut app, &psbt.to_bytes());
    assert!(note(&app).contains("not used here"), "{}", note(&app));
    assert_eq!(testkit::message(&app).unwrap().text, "");
    assert_eq!(app.sheet, Some(Sheet::Scan));
}

#[test]
fn a_text_in_parts_becomes_the_message() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    let text = "A message long enough to come in several parts, one code after another.";
    let parts = faraday_qr::bbqr::encode('U', text.as_bytes(), 20).expect("parts");
    assert!(parts.len() > 1);
    for p in &parts {
        assert_eq!(app.sheet, Some(Sheet::Scan));
        read(&mut app, p);
    }
    assert_eq!(app.sheet, None);
    assert_eq!(testkit::message(&app).unwrap().text, text);
}

#[test]
fn specter_parts_cut_after_a_space_sign_the_original_text() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    let text = "I own this address and agree";
    // Cut after "I own " and after "this address ".
    let parts = faraday_qr::specter::encode(text, 6);
    assert!(parts[0].ends_with("I own "), "{parts:?}");
    for p in &parts {
        assert_eq!(app.sheet, Some(Sheet::Scan));
        read(&mut app, p);
    }
    assert_eq!(app.sheet, None);
    assert_eq!(testkit::message(&app).unwrap().text, text);
}

#[test]
fn a_ur_bytes_message_that_is_no_text_is_refused_whole_or_in_parts() {
    let binary = [0x41u8, 0xFF, 0x42, 0x43, 0x80, 0x44, 0x45, 0x46, 0x47, 0x48];
    for parts in [vec![faraday_codec::ur::encode_bytes(&binary)], {
        let mut enc = faraday_codec::ur::Encoder::bytes(&binary, 4).expect("encoder");
        assert!(enc.fragment_count() > 1);
        (0..enc.fragment_count()).map(|_| enc.next_part()).collect()
    }] {
        let mut app = app(true);
        app.press(Action::SignMessage);
        app.press(Action::MScan);
        for p in &parts {
            read(&mut app, p);
        }
        assert_eq!(app.sheet, Some(Sheet::Scan));
        assert_eq!(
            note(&app),
            faraday_core::scan_read::Kind::Unreadable
                .not_here()
                .as_str()
        );
        assert_eq!(testkit::message(&app).unwrap().text, "");
    }
}

#[test]
fn a_new_scanned_text_clears_an_earlier_signature() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    read(&mut app, "First");
    app.press(Action::MSign);
    assert!(testkit::message(&app).unwrap().signed.is_some());
    app.press(Action::MScan);
    read(&mut app, "Second");
    let m = testkit::message(&app).unwrap();
    assert_eq!(m.text, "Second");
    assert!(m.signed.is_none());
}

#[test]
fn a_text_is_signed_exactly_as_scanned() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    read(&mut app, "Line\n");
    assert_eq!(testkit::message(&app).unwrap().text, "Line\n");
    app.press(Action::MSign);
    let scanned = testkit::message(&app).unwrap().signed.clone().unwrap();
    testkit::message_text(&mut app, "Line\n");
    app.press(Action::MSign);
    let typed = testkit::message(&app).unwrap().signed.clone().unwrap();
    assert_eq!(scanned.signature, typed.signature);
    testkit::message_text(&mut app, "Line");
    app.press(Action::MSign);
    let trimmed = testkit::message(&app).unwrap().signed.clone().unwrap();
    assert_ne!(scanned.signature, trimmed.signature);
}

#[test]
fn the_read_cards_sign_this_message_keeps_the_text_as_scanned() {
    let mut app = app(true);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    read(&mut app, "A note\n");
    let ways = app
        .scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| c.ways.clone())
        .unwrap_or_default();
    let at = ways.iter().position(|w| *w == Way::SignMessage).unwrap();
    app.press(Action::ReadGo(at as u8));
    assert_eq!(testkit::message(&app).unwrap().text, "A note\n");
}

#[test]
fn keys_do_not_reach_the_message_while_the_scanner_is_open() {
    let mut app = app(true);
    app.press(Action::SignMessage);
    app.press(Action::MType);
    app.press(Action::MScan);
    app.event(Event::Key(faraday_shell_api::Key::Char('x')));
    assert_eq!(testkit::message(&app).unwrap().text, "");
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    assert_eq!(app.sheet, None, "Escape is the scanner's Cancel");
}

#[test]
fn every_scan_that_opens_the_camera_waits_for_the_stick() {
    use faraday_core::backup_flow::steps::{FlowAction as F, RestorePress as R};
    use faraday_core::compare::{ComparePress, Slot};
    use faraday_core::decode::check::{self, SignerPress};
    let presses = [
        Action::MScan,
        Action::ReadAgain,
        Action::Compare(ComparePress::Scan(Slot::First)),
        Action::SignerCheck(SignerPress::Scan(check::Slot::Sent)),
        Action::Sweep(faraday_core::seeds::sweep::SweepAction::Scan),
        Action::BC32(faraday_core::codex32_backup::Press::ScanCopy),
        Action::KefTool(faraday_core::kef_tool::KefToolAction::Backup(
            faraday_core::kef_form::KefAct::ScanCopy,
        )),
        Action::Flow(F::Restore(R::Scan)),
        Action::Flow(F::Restore(R::ScanPaperkey)),
    ];
    for press in presses {
        let mut app = app(true);
        app.press(Action::SignMessage);
        stick_in(&mut app, true);
        app.press(press);
        assert_eq!(app.sheet, Some(Sheet::Pull), "{press:?}");
        assert!(app.scan().is_none(), "{press:?} opened the camera");
        stick_in(&mut app, false);
    }
}
