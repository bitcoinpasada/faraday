//! The caret in a field that takes typing blinks, on and off about every
//! half second, and shows at once on a key; a screen with no such field
//! asks for no frames to blink anything.

use faraday_core::{Action, Faraday, Screen, testkit};
use osk_shell_api::{App, Command, Event, Key};

fn tick(app: &mut Faraday, now_ms: u64) -> bool {
    app.event(Event::Tick { now_ms });
    std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::Draw)
}

fn pixels(app: &mut Faraday) -> Vec<u8> {
    app.frame().rgba.to_vec()
}

/// Tools, whose find box takes typing from the start, drawn at `now`.
fn tools(now: u64) -> Faraday {
    let mut app = testkit::started();
    tick(&mut app, now);
    app.press(Action::Nav(Screen::Catalog));
    app.event(Event::Key(Key::Char('a')));
    let _ = app.frame();
    while app.poll_command().is_some() {}
    app
}

#[test]
fn the_caret_blinks() {
    let mut app = tools(1_000);
    let on = pixels(&mut app);
    assert!(!tick(&mut app, 1_200), "no frame while it stays");
    assert!(tick(&mut app, 1_540), "a frame when it goes");
    let off = pixels(&mut app);
    assert!(on != off);
    assert!(tick(&mut app, 2_070), "and when it comes back");
    assert!(pixels(&mut app) == on);
}

#[test]
fn a_key_shows_the_caret_at_once() {
    let mut app = tools(1_000);
    tick(&mut app, 1_540);
    let _ = app.frame();
    app.event(Event::Key(Key::Char('b')));
    app.event(Event::Key(Key::Backspace));
    let after_key = pixels(&mut app);
    let fresh = pixels(&mut tools(1_540));
    assert!(after_key == fresh, "on, as just after typing");
}

#[test]
fn nothing_blinks_where_nothing_is_typed() {
    let mut app = testkit::started();
    tick(&mut app, 1_000);
    let _ = app.frame();
    while app.poll_command().is_some() {}
    for now in (1_016..3_000).step_by(16) {
        assert!(!tick(&mut app, now));
    }
}
