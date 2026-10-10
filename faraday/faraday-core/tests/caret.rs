//! The caret in a field that takes typing blinks, on and off about every
//! half second, and shows at once on a key; a screen with no such field
//! asks for no frames to blink anything. A press in a field puts the
//! caret where it lands and typing goes in there; the arrow keys move
//! it; Backspace takes the character before it and Delete the one after
//! (`docs/NEW-WALLET.md` §13.2).

use faraday_core::{Action, Faraday, Screen, testkit};
use osk_shell_api::{App, Command, Event, Key, TouchPhase};

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

/// Wallets with the loaded wallet's new name being typed, `name` in it.
fn renaming(name: &str) -> Faraday {
    let mut app = testkit::started();
    let file = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![file],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Rename);
    let renamed = app.renaming.as_ref().unwrap().chars().count();
    for _ in 0..renamed {
        app.event(Event::Key(Key::Backspace));
    }
    for c in name.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let _ = app.frame();
    app
}

/// A press at the edge before character `i` of the name.
fn press_at(app: &mut Faraday, i: usize) {
    let (x, y) = app.where_typed(Action::Rename, i).unwrap();
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    let _ = app.frame();
}

#[test]
fn a_press_puts_the_caret_where_it_lands_and_typing_goes_in_there() {
    let mut app = renaming("Grocery");
    press_at(&mut app, 3);
    app.event(Event::Key(Key::Char('x')));
    app.event(Event::Key(Key::Char('y')));
    assert_eq!(app.renaming.as_deref(), Some("Groxycery"));
}

#[test]
fn backspace_takes_the_character_before_the_caret_and_delete_the_one_after() {
    let mut app = renaming("Grocery");
    press_at(&mut app, 3);
    app.event(Event::Key(Key::Backspace));
    assert_eq!(app.renaming.as_deref(), Some("Grcery"));
    app.event(Event::Key(Key::Delete));
    assert_eq!(app.renaming.as_deref(), Some("Grery"));
}

#[test]
fn the_arrow_keys_move_the_caret() {
    let mut app = renaming("Grocery");
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Char('-')));
    assert_eq!(app.renaming.as_deref(), Some("Groce-ry"));
    app.event(Event::Key(Key::Right));
    app.event(Event::Key(Key::Char('-')));
    assert_eq!(app.renaming.as_deref(), Some("Groce-r-y"));
}
