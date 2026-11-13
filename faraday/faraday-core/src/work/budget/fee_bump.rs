//! The fee bump on Sign a transaction's review does its costly work when
//! the other transaction comes into view and nowhere else
//! (`docs/COMPARE.md` §4.3): drawing the Transaction step again, a
//! pointer move and a scroll work out nothing, and a press that changes
//! nothing about the spend or Files inspects no other file.

use crate::wallet::step;
use crate::work::{Work, since, snapshot};
use crate::{Action, Faraday, StorageEvent, testkit};
use faraday_shell_api::{App, Event};

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                       abandon abandon about";

fn vector(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../tools/vectors/psbt/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn files() -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "bump-core-replacement.psbt".into(),
            vector("bump-core-replacement.psbt"),
        ),
        (
            "bump-core-original.psbt".into(),
            vector("bump-core-original.psbt"),
        ),
    ]
}

fn restore(app: &mut Faraday) {
    app.storage(StorageEvent::Restored {
        inbox: crate::files_from(files()),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
}

/// The replacement open on the Transaction step, the original in Files.
fn reviewing() -> Faraday {
    let mut app = testkit::started();
    restore(&mut app);
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    app.press(Action::StartSpend(0));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    assert!(!app.spend().unwrap().others.is_empty());
    let _ = app.frame();
    app
}

#[test]
fn drawing_again_a_pointer_move_and_a_scroll_work_out_nothing() {
    let mut app = reviewing();
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
        assert_eq!(drawn.of(k), 0, "{k:?} while drawing the Transaction step");
    }
}

#[test]
fn a_press_that_changes_nothing_inspects_no_other_file() {
    let mut app = reviewing();
    let before = snapshot();
    // The step closed and opened again, and Files as it was.
    app.press(Action::Step(step::TRANSACTION));
    app.press(Action::Step(step::TRANSACTION));
    let _ = app.frame();
    let press = since(&before);
    assert_eq!(press.of(Work::PsbtInspect), 0);
    assert_eq!(press.of(Work::Sign), 0);
}

#[test]
fn the_file_coming_in_counts_the_one_inspection() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: crate::files_from(files().into_iter().take(1).collect()),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    app.press(Action::StartSpend(0));
    let before = snapshot();
    restore(&mut app);
    let press = since(&before);
    assert_eq!(press.of(Work::PsbtInspect), 1, "the original, once");
    let before = snapshot();
    restore(&mut app);
    assert_eq!(since(&before).of(Work::PsbtInspect), 0);
}

#[test]
fn a_touch_or_key_with_a_finished_transaction_in_files_does_no_costly_work() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: crate::files_from(vec![
            files().remove(0),
            (
                faraday_text::Text::of("bump-core-original.tx").to_string(),
                vector("bump-core-original.tx"),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    app.press(Action::StartSpend(0));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    assert!(!app.spend().unwrap().others.is_empty());
    let _ = app.frame();
    let before = snapshot();
    app.event(Event::Hover { x: 40, y: 200 });
    app.event(Event::Wheel {
        x: 40,
        y: 200,
        dy: 48,
    });
    let _ = app.frame();
    app.press(Action::Step(step::TRANSACTION));
    app.press(Action::Step(step::TRANSACTION));
    let _ = app.frame();
    let touched = since(&before);
    for k in crate::work::KINDS {
        if crate::work::PIXELS.contains(&k) {
            continue;
        }
        assert_eq!(
            touched.of(k),
            0,
            "{k:?} with a finished transaction in Files"
        );
    }
}
