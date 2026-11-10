//! A QR vault's code (`docs/VAULT.md` §1.2) reaches the QR vault tool
//! from Home's Scan, Add a key's Scan a SeedQR and Files, holding the code
//! for the Open card; a refused one is none of that.

use faraday_core::keygen::Start;
use faraday_core::scan_read::Kind;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, qr_tool, testkit};
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
        assert!(app.offers(Action::OpenQrVault(0)));
        app.press(Action::OpenQrVault(0));
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
