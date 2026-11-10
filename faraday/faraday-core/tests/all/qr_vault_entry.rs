//! A QR vault's code (`docs/VAULT.md` §1.2) reaches the QR vault tool
//! from Home's Scan, Add a key's Scan a SeedQR and Files, holding the code
//! for the Open card; a refused one is none of that.

use faraday_core::keygen::Start;
use faraday_core::qr_tool::QrToolAction;
use faraday_core::scan_read::Kind;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, qr_tool, testkit};
use faraday_shell_api::{App, Event};

const NOTE: &[u8] = include_bytes!("../../../../tools/vectors/vault/qr/qr-note.pgp");
const SEED: &[u8] = include_bytes!("../../../../tools/vectors/vault/qr/qr-seed-12.pgp");
const REFUSED: &[u8] = include_bytes!("../../../../tools/vectors/vault/qr/refused-trailing.pgp");

fn read(app: &mut Faraday, code: &[u8]) {
    app.event(Event::Scanned {
        bytes: code.to_vec(),
    });
    let _ = app.frame();
}

fn card_kind(app: &Faraday) -> Option<Kind> {
    app.scan()?.card.as_ref().map(|c| c.kind)
}

/// The card's only way is pressed.
fn press_way(app: &mut Faraday) {
    let ways = app.scan().and_then(|s| s.card.as_ref()).unwrap().ways.len();
    assert_eq!(ways, 1);
    app.press(Action::ReadGo(0));
}

fn holds(app: &Faraday, code: &[u8]) {
    assert_eq!(app.screen, Screen::QrTool);
    let held = qr_tool::state(app).and_then(|s| s.code_to_open());
    assert_eq!(held, Some(code));
}

#[test]
fn home_scan_names_a_qr_vault_and_opens_the_tool_with_it() {
    for code in [NOTE, SEED] {
        let mut app = testkit::started();
        app.press(Action::Nav(Screen::Home));
        app.press(Action::Scan);
        assert_eq!(app.sheet, Some(Sheet::Scan));
        read(&mut app, code);
        assert_eq!(card_kind(&app), Some(Kind::QrVault));
        press_way(&mut app);
        holds(&app, code);
        assert!(app.inbox().is_empty(), "the scanned code was filed");
    }
}

#[test]
fn scan_a_seedqr_names_a_qr_vault_and_opens_the_tool_with_it() {
    for code in [NOTE, SEED] {
        let mut app = testkit::started();
        app.press(Action::KeyGen(None));
        let _ = app.frame();
        app.press(Action::KStart(Start::Scan.index()));
        assert_eq!(app.sheet, Some(Sheet::Scan));
        read(&mut app, code);
        assert_eq!(card_kind(&app), Some(Kind::QrVault));
        press_way(&mut app);
        holds(&app, code);
        assert!(app.inbox().is_empty(), "the scanned code was filed");
    }
}

#[test]
fn scan_a_seedqr_with_a_stick_plugged_in_still_opens_the_tool() {
    for code in [NOTE, SEED] {
        let mut app = testkit::started();
        app.press(Action::KeyGen(None));
        let _ = app.frame();
        app.press(Action::KStart(Start::Scan.index()));
        assert_eq!(app.sheet, Some(Sheet::Scan));
        // Plugged in while the camera is on.
        app.storage(StorageEvent::Sticks(vec![StickInfo {
            make: Default::default(),
            serial: None,
            id: "stick".into(),
            label: "STICK".into(),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        }]));
        read(&mut app, code);
        assert_eq!(card_kind(&app), Some(Kind::QrVault), "keys do not load");
        press_way(&mut app);
        holds(&app, code);
    }
}

#[test]
fn a_qr_vault_file_in_files_opens_the_tool_and_stays_listed() {
    for (name, code) in [("note.pgp", NOTE), ("seed.pgp", SEED)] {
        let mut app = testkit::started();
        app.storage(StorageEvent::Restored {
            inbox: vec![(name.to_string(), code.to_vec())],
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.press(Action::Nav(Screen::Files));
        let _ = app.frame();
        assert!(app.offers(Action::QrTool(QrToolAction::OpenFile(0))));
        app.press(Action::QrTool(QrToolAction::OpenFile(0)));
        holds(&app, code);
        assert_eq!(app.inbox().iter().filter(|i| i.name == name).count(), 1);
    }
}

#[test]
fn a_refused_code_is_no_qr_vault() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    read(&mut app, REFUSED);
    assert_ne!(card_kind(&app), Some(Kind::QrVault));
}

const GNUPG: &[u8] = include_bytes!("../../../fuzz/corpus/pgp_message/refused-skesk4-seipd1.pgp");

#[test]
fn a_qr_vault_file_in_the_outbox_reads_as_sealed() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: vec![("note.pgp".to_string(), NOTE.to_vec())],
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Written sealed"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Public"), "{texts:?}");
}

#[test]
fn a_refused_pgp_file_in_files_does_not_offer_the_qr_vault() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![("gnupg.pgp".to_string(), GNUPG.to_vec())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(!app.offers(Action::QrTool(QrToolAction::OpenFile(0))));
    app.press(Action::QrTool(QrToolAction::OpenFile(0)));
    assert_ne!(app.screen, Screen::QrTool);
}

#[test]
fn the_codex32_and_slip39_scanner_names_a_qr_vault() {
    for code in [NOTE, SEED] {
        let mut app = testkit::started();
        app.press(Action::Entry(None));
        app.press(Action::EntryForm(2));
        app.press(Action::ScanPart);
        assert_eq!(
            app.scan().map(|s| s.purpose),
            Some(faraday_core::ScanPurpose::KeyPart)
        );
        read(&mut app, code);
        assert_eq!(card_kind(&app), Some(Kind::QrVault));
    }
}
