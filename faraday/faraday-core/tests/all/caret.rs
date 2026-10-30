//! The caret in a field that takes typing blinks, on and off about every
//! half second, and shows at once on a key; a screen with no such field
//! asks for no frames to blink anything. A press in a field puts the
//! caret where it lands and typing goes in there; the arrow keys move
//! it; Backspace takes the character before it and Delete the one after
//! (`docs/history/NEW-WALLET.md` §13.2).

use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, Command, Event, Key, TouchPhase};

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
    let renamed = app.renaming().unwrap().chars().count();
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
    assert_eq!(app.renaming().map(String::as_str), Some("Groxycery"));
}

#[test]
fn backspace_takes_the_character_before_the_caret_and_delete_the_one_after() {
    let mut app = renaming("Grocery");
    press_at(&mut app, 3);
    app.event(Event::Key(Key::Backspace));
    assert_eq!(app.renaming().map(String::as_str), Some("Grcery"));
    app.event(Event::Key(Key::Delete));
    assert_eq!(app.renaming().map(String::as_str), Some("Grery"));
}

#[test]
fn the_arrow_keys_move_the_caret() {
    let mut app = renaming("Grocery");
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Char('-')));
    assert_eq!(app.renaming().map(String::as_str), Some("Groce-ry"));
    app.event(Event::Key(Key::Right));
    app.event(Event::Key(Key::Char('-')));
    assert_eq!(app.renaming().map(String::as_str), Some("Groce-r-y"));
}

/// Tools' Find a tool takes typing with no press to focus it; the arrow
/// keys and a press there move the caret in it as in any field
/// (`docs/TEXT-CURSOR.md` §4, C2).
#[test]
fn find_a_tool_takes_typing_where_the_caret_is() {
    let mut app = tools(1_000);
    for c in "slp".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Char('x')));
    assert_eq!(testkit::catalog_find(&app), "asxlp");
    let _ = app.frame();
    let here = Action::TypeHere(faraday_core::edit::Here::Find);
    let (x, y) = app.where_typed(here, 1).expect("the find drawn");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    app.event(Event::Key(Key::Delete));
    app.event(Event::Key(Key::Char('i')));
    assert_eq!(testkit::catalog_find(&app), "aixlp");
}

/// Sign a message with `text` typed as its message.
fn message(text: &str) -> Faraday {
    let mut app = testkit::started();
    app.press(Action::SignMessage);
    app.press(Action::MStep(faraday_core::mstep::TEXT));
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let _ = app.frame();
    app
}

fn message_text(app: &Faraday) -> String {
    testkit::message(app)
        .map(|m| m.text.clone())
        .unwrap_or_default()
}

/// A message of several lines takes an edit at its caret: the arrow
/// keys move it, a press on the second line puts it there, and Up takes
/// it to the line above (`docs/TEXT-CURSOR.md` §4, C3).
#[test]
fn a_message_of_several_lines_takes_typing_where_the_caret_is() {
    let mut app = message("Pay to the order of");
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Char('-')));
    assert_eq!(message_text(&app), "Pay to the order -of");
    let words = "the quick brown fox jumps over the lazy dog ".repeat(8);
    let mut app = message(&words);
    let here = Action::TypeHere(faraday_core::edit::Here::Message);
    let (_, first) = app.where_typed(here, 0).expect("the message drawn");
    let at = 300;
    let (x, y) = app.where_typed(here, at).expect("the message drawn");
    assert!(y > first, "the message wraps");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    app.event(Event::Key(Key::Char('X')));
    assert_eq!(message_text(&app).find('X'), Some(at));
    app.event(Event::Key(Key::Backspace));
    app.event(Event::Key(Key::Up));
    app.event(Event::Key(Key::Char('Y')));
    let text = message_text(&app);
    let y_at = text.find('Y').expect("typed");
    assert!(y_at < at, "on a line above: {y_at}");
    assert_eq!(text.replace('Y', ""), words);
}

/// Add a key's codex32 string takes an edit at its caret: the arrow
/// keys, Home among them, and a press move it, and what is typed goes in there
/// (`docs/TEXT-CURSOR.md` §4, C3).
#[test]
fn a_codex32_string_takes_typing_where_the_caret_is() {
    use faraday_core::forms::Form;
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    let codex32 = Form::ALL.iter().position(|f| *f == Form::Codex32).unwrap();
    app.press(Action::EntryForm(u8::try_from(codex32).unwrap()));
    for c in "ms10tstsx".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    for _ in 0..4 {
        app.event(Event::Key(Key::Left));
    }
    app.event(Event::Key(Key::Char('e')));
    assert_eq!(&*testkit::entry(&app).typed, "ms10testsx");
    app.event(Event::Key(Key::Home));
    for _ in 0..9 {
        app.event(Event::Key(Key::Right));
    }
    app.event(Event::Key(Key::Char('s')));
    assert_eq!(&*testkit::entry(&app).typed, "ms10testssx");
    // A press between two characters puts the caret there.
    let _ = app.frame();
    let here = Action::TypeHere(faraday_core::edit::Here::Entry);
    let (x, y) = app.where_typed(here, 9).expect("the string drawn");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    app.event(Event::Key(Key::Backspace));
    assert_eq!(&*testkit::entry(&app).typed, "ms10testsx");
}

/// Add a key's words take an edit at the caret: the arrow keys move it
/// within a word and on past its edge into the next cell, a press on a
/// cell puts it there, and what is typed goes in there as it is, no word
/// completed part way through (`docs/TEXT-CURSOR.md`).
#[test]
fn the_words_grid_moves_its_caret_through_the_cells() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in "abandon ability ".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let t0 = testkit::entry(&app).typed.to_string();
    let second = t0.find("abi").expect("the second word");
    app.event(Event::Key(Key::Home));
    for _ in 0..3 {
        app.event(Event::Key(Key::Right));
    }
    app.event(Event::Key(Key::Char('x')));
    let want = format!("{}x{}", &t0[..3], &t0[3..]);
    assert_eq!(&*testkit::entry(&app).typed, want);
    // On past the first word's end, through the space, into the second.
    for _ in 0..(second + 2 - 4) {
        app.event(Event::Key(Key::Right));
    }
    app.event(Event::Key(Key::Char('q')));
    let at = second + 1 + 1;
    let want = format!("{}q{}", &want[..at], &want[at..]);
    assert_eq!(&*testkit::entry(&app).typed, want);
    // A press in the first cell, between its first two letters.
    let _ = app.frame();
    let here = Action::TypeHere(faraday_core::edit::Here::Words);
    let (x, y) = app.where_typed(here, 1).expect("the cells drawn");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    app.event(Event::Key(Key::Delete));
    let want = format!("{}{}", &want[..1], &want[2..]);
    assert_eq!(&*testkit::entry(&app).typed, want);
}

/// The aezeed's passphrase on the Lightning screen, `typed` into it.
fn aezeed_passphrase(typed: &str) -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    app.press(Action::LPassphrase);
    for c in typed.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app
}

/// A masked field takes an edit at its caret as any field does, and
/// what it shows is a dot a character wherever the caret is: two
/// passphrases of the same length, with the caret at the same place,
/// draw the same.
#[test]
fn a_masked_field_moves_its_caret_and_shows_nothing_of_what_is_typed() {
    let mut app = aezeed_passphrase("abc");
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Char('x')));
    assert_eq!(&*app.lightning().unwrap().passphrase, "axbc");
    let mut other = aezeed_passphrase("qqqq");
    app.event(Event::Key(Key::Left));
    for _ in 0..3 {
        other.event(Event::Key(Key::Left));
    }
    assert!(pixels(&mut app) == pixels(&mut other));
    let (x, y) = app
        .where_typed(Action::LPassphrase, 3)
        .expect("the dots drawn");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    app.event(Event::Key(Key::Backspace));
    assert_eq!(&*app.lightning().unwrap().passphrase, "axc");
}
