//! Decode a Lightning invoice: the tile opens the tool, a scanned
//! invoice opens it decoded, what BOLT #11 says an invoice holds shows,
//! a refused invoice shows its line, and nothing is kept.

use crate::common::type_text;
use faraday_core::catalog::TILES;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// BOLT #11's examples, as the vectors hold them.
const VECTORS: &str = include_str!("../../../../tools/vectors/lightning-bolt11/invoice-test.json");

fn invoice(starts: &str, about: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .find(|i| {
            i["invoice"].as_str().unwrap().starts_with(starts)
                && i["description"].as_str().unwrap().contains(about)
        })
        .and_then(|i| i["invoice"].as_str())
        .unwrap()
        .to_string()
}

fn coffee() -> String {
    invoice("lnbc2500u", "cup of coffee")
}

fn opened() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::Catalog));
    let tile = TILES
        .iter()
        .position(|t| t.name == "Decode a Lightning invoice")
        .expect("no tile");
    app.press(Action::Catalog(tile as u8));
    app
}

fn value(app: &Faraday, label: &str) -> Option<String> {
    let rows = app.invoice()?.read()?.as_ref().ok()?;
    rows.iter()
        .find(|r| r.label == label)
        .map(|r| r.value.clone())
}

#[test]
fn the_tile_opens_the_tool_empty() {
    let app = opened();
    assert_eq!(app.screen, Screen::Invoice);
    assert!(app.invoice().unwrap().text().is_empty());
    assert!(app.invoice().unwrap().read().is_none());
}

#[test]
fn a_typed_invoice_shows_its_amount_description_and_payee() {
    let mut app = opened();
    type_text(&mut app, &coffee());
    app.press(Action::Invoice(faraday_core::invoice::InvoicePress::Decode));
    assert_eq!(value(&app, "Amount").as_deref(), Some("250000 sat"));
    assert!(value(&app, "Description").unwrap().contains("coffee"));
    assert_eq!(value(&app, "Payee key").unwrap().len(), 66);
    assert!(!value(&app, "Network").unwrap().is_empty());
}

#[test]
fn an_invoice_with_no_amount_says_any_amount() {
    let mut app = opened();
    type_text(&mut app, &invoice("lnbc1p", "donation of any amount"));
    app.event(Event::Key(Key::Enter));
    assert_eq!(value(&app, "Amount").as_deref(), Some("Any amount"));
}

#[test]
fn a_refused_invoice_shows_one_line_and_no_rows() {
    let mut app = opened();
    type_text(
        &mut app,
        &invoice("lnbc25m1pvjluezpp5", "invalid unknown feature"),
    );
    app.press(Action::Invoice(faraday_core::invoice::InvoicePress::Decode));
    let line = app
        .invoice()
        .unwrap()
        .read()
        .unwrap()
        .as_ref()
        .expect_err("decoded")
        .clone();
    assert!(line.starts_with("Refused"), "{line}");
    assert!(app.drawn_texts().contains(&line));
}

#[test]
fn a_scanned_invoice_offers_decode_it_and_opens_decoded() {
    let mut app = Faraday::new();
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: coffee().into_bytes(),
    });
    let way = app
        .scan()
        .and_then(|s| s.card.as_ref())
        .map(|c| c.ways[0].label());
    assert_eq!(way.as_deref(), Some("Decode it"));
    // Home's Scan filed the text as it files every public kind; opening
    // the tool adds nothing.
    let filed = app.inbox().len();
    app.press(Action::ReadGo(0));
    assert_eq!(app.screen, Screen::Invoice);
    assert!(value(&app, "Description").unwrap().contains("coffee"));
    assert_eq!(app.inbox().len(), filed, "the tool filed the invoice");
}

#[test]
fn leaving_the_screen_drops_the_text() {
    let mut app = opened();
    type_text(&mut app, &coffee());
    app.press(Action::Invoice(faraday_core::invoice::InvoicePress::Decode));
    app.press(Action::Nav(Screen::Catalog));
    assert!(app.invoice().is_none());
    assert!(app.inbox().is_empty());
}
