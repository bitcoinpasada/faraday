//! The worker that runs the costly part of a press (`docs/WORKERS.md`):
//! Load ticked on Practice's samples, whose answer is applied on a later
//! pass.

use faraday_core::samples::Pick;
use faraday_core::testkit;
use faraday_core::{Action, DESKTOP_MODES, Faraday};
use faraday_shell_api::{App, Event};

/// Practice with the first sample key ticked on the Load samples sheet.
fn ticked() -> Faraday {
    let mut app = testkit::started();
    app.set_modes(DESKTOP_MODES);
    testkit::into_demo(&mut app);
    app.samples = Some(testkit::demo_samples());
    app.press(Action::Samples);
    app.press(Action::SampleTick(Pick::Key(0)));
    app
}

fn tick(app: &mut Faraday, now_ms: u64) {
    app.event(Event::Tick { now_ms });
}

#[test]
fn load_ticked_gives_the_same_key_on_the_worker_as_run_inline() {
    let mut inline = ticked();
    inline.press(Action::SamplesLoad);

    let mut app = ticked();
    testkit::press_on_worker(&mut app, Action::SamplesLoad);
    assert!(app.busy().is_some(), "the screen is busy while it runs");
    testkit::settle(&mut app);
    assert!(app.busy().is_none());
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys.len(), inline.session.keys.len());
    assert_eq!(app.toast_text(), inline.toast_text());
    assert_eq!(app.sheet, None, "the sheet closes with the answer");
}

#[test]
fn a_press_while_it_runs_does_nothing() {
    let mut app = ticked();
    testkit::press_on_worker(&mut app, Action::SamplesLoad);
    assert!(app.busy().is_some());
    app.press(Action::SamplesLoad);
    app.press(Action::SampleTick(Pick::Key(1)));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1, "loaded once, the one key");
}

#[test]
fn back_while_it_runs_leaves_nothing_loaded() {
    let mut app = ticked();
    testkit::press_on_worker(&mut app, Action::SamplesLoad);
    app.press(Action::Back);
    assert!(app.busy().is_none());
    testkit::settle(&mut app);
    tick(&mut app, 1000);
    assert_eq!(app.session.keys.len(), 0, "the answer was dropped");
}

#[test]
fn a_lock_while_it_runs_leaves_nothing_loaded() {
    let mut app = ticked();
    testkit::press_on_worker(&mut app, Action::SamplesLoad);
    app.press(Action::LockUnsealed);
    assert!(app.busy().is_none());
    testkit::settle(&mut app);
    tick(&mut app, 1000);
    assert_eq!(app.session.keys.len(), 0, "no key is held after the lock");
}
