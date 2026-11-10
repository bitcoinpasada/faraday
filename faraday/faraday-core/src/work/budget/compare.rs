//! Compare does its costly work on the press that completes the second
//! card and nowhere else (`docs/COMPARE.md` §6.2): the press counts its
//! inspections, and drawing the Result card again, a pointer move and a
//! scroll work out nothing.

use crate::compare::{ComparePress, Slot};
use crate::work::{Work, since, snapshot};
use crate::{Action, Faraday, StorageEvent, testkit};
use faraday_shell_api::{App, Event};

fn vector(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../tools/vectors/psbt/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Files holds the Core pair; the flow is open on its First card with
/// nothing chosen yet.
fn open_unchosen() -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("original.psbt".into(), vector("bump-core-original.psbt")),
            (
                "replacement.psbt".into(),
                vector("bump-core-replacement.psbt"),
            ),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let tile = crate::catalog::TILES
        .iter()
        .position(|t| t.go == crate::catalog::Go::Compare)
        .expect("the tile") as u8;
    app.press(Action::Nav(crate::Screen::Catalog));
    app.press(Action::Catalog(tile));
    let _ = app.frame();
    app
}

#[test]
fn the_press_completing_the_second_card_counts_its_inspections_and_drawing_counts_none() {
    let mut app = open_unchosen();
    app.press(Action::Compare(ComparePress::Pick(Slot::First, 0)));
    let _ = app.frame();
    let before = snapshot();
    app.press(Action::Compare(ComparePress::Pick(Slot::Second, 1)));
    let press = since(&before);
    assert!(press.of(Work::PsbtInspect) > 0, "the pair is inspected");
    assert_eq!(press.of(Work::Sign), 0);
    assert!(app.compare().is_some_and(|s| s.shown.is_some()));

    // The Result drawn again, a pointer move and a scroll: nothing.
    let _ = app.frame();
    let before = snapshot();
    let _ = app.frame();
    app.event(Event::Hover { x: 40, y: 200 });
    let _ = app.frame();
    app.event(Event::Wheel {
        x: 40,
        y: 200,
        dy: 48,
    });
    let _ = app.frame();
    app.event(Event::Wheel {
        x: 40,
        y: 200,
        dy: -48,
    });
    let _ = app.frame();
    let drawn = since(&before);
    for k in crate::work::KINDS {
        if crate::work::PIXELS.contains(&k) {
            continue;
        }
        assert_eq!(drawn.of(k), 0, "{k:?} while drawing the Result");
    }
}
