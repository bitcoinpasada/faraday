//! Check a signer's Sample tile (`docs/MODES.md` §5.3): in Practice both
//! scanners hand over a sample pair, the Sent and then a Returned that is
//! clean, and a second press of the Returned tile hands over the same
//! with a proprietary pair added.

use crate::common::{demo_app, strip_offers};
use faraday_core::catalog::{Go, TILES};
use faraday_core::decode::check::{SignerPress, Slot};
use faraday_core::strings::EN;
use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::App;

fn open(app: &mut Faraday) {
    let tile = TILES
        .iter()
        .position(|t| t.go == Go::SignerCheck)
        .expect("the tile is in Tools") as u8;
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile));
    let _ = app.frame();
}

fn sample_from(app: &mut Faraday, slot: Slot) {
    app.press(Action::SignerCheck(SignerPress::Scan(slot)));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(strip_offers(app, Action::ScanSample), "{slot:?}");
    app.press(Action::ScanSample);
    let _ = app.frame();
}

fn shows(app: &mut Faraday, part: &str) -> bool {
    app.drawn_texts().iter().any(|t| t.contains(part))
}

#[test]
fn the_sample_pair_reads_clean_then_a_second_returned_has_a_proprietary_finding() {
    let mut app = demo_app();
    open(&mut app);
    sample_from(&mut app, Slot::Sent);
    sample_from(&mut app, Slot::Returned);
    assert!(shows(&mut app, EN.signer_only_signatures));
    assert!(!shows(&mut app, "proprietary field added"));

    app.press(Action::SignerCheck(SignerPress::Card(1)));
    sample_from(&mut app, Slot::Returned);
    assert!(!shows(&mut app, EN.signer_only_signatures));
    let found: Vec<String> = app
        .drawn_texts()
        .plain()
        .into_iter()
        .filter(|t| t.contains("proprietary field added"))
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("Danger"), "{found:?}");

    app.press(Action::SignerCheck(SignerPress::Card(1)));
    sample_from(&mut app, Slot::Returned);
    assert!(shows(&mut app, EN.signer_only_signatures));
    assert!(!shows(&mut app, "proprietary field added"));
}

#[test]
fn outside_practice_the_scanners_offer_no_sample() {
    let mut app = testkit::started();
    open(&mut app);
    for slot in [Slot::Sent, Slot::Returned] {
        app.press(Action::SignerCheck(SignerPress::Scan(slot)));
        let _ = app.frame();
        assert_eq!(app.sheet, Some(Sheet::Scan));
        assert!(!strip_offers(&mut app, Action::ScanSample), "{slot:?}");
        app.press(Action::Back);
    }
}
