//! A new input device is believed only once a person at the screen says
//! so (`docs/history/PLAN.md` §4.6): a keyboard by typing the code shown, a pointer by
//! input already believed; ignored devices, and keyboards that keep
//! guessing the code, stay out until unplugged, and after three such
//! keyboards new ones get no code until Faraday starts again. The code is
//! drawn from the session's seed: a keyboard that arrives before the
//! system gave any randomness gets no code, cannot be let in, and the
//! sheet says so; it gets its code once the randomness arrives.

use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};

/// An app whose shell answered its first request for randomness.
fn seeded() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Entropy(EntropyBytes::new([7u8; 32])));
    app
}

/// The code the first waiting device must type.
fn code_of(app: &Faraday) -> String {
    app.inputs[0].code.clone().expect("a code")
}

fn new_device(app: &mut Faraday, id: u32, keyboard: bool, pointer: bool) {
    app.storage(StorageEvent::NewInput {
        id,
        name: "USB Keyboard".into(),
        keyboard,
        pointer,
    });
}

fn type_on(app: &mut Faraday, id: u32, text: &str) {
    for ch in text.chars() {
        app.storage(StorageEvent::InputTyped { id, ch });
    }
}

#[test]
fn a_new_keyboard_is_believed_once_it_types_the_code() {
    let mut app = seeded();
    new_device(&mut app, 4, true, false);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    let code = code_of(&app);
    assert_eq!(code.len(), 6);
    // Anything else typed is not the code.
    type_on(&mut app, 4, "zzzzzz");
    assert_eq!(app.poll_input_decision(), None);
    type_on(&mut app, 4, &code);
    assert_eq!(app.poll_input_decision(), Some((4, true)));
    assert!(app.inputs.is_empty());
    assert_eq!(app.sheet, None);
}

#[test]
fn a_keyboard_that_keeps_guessing_is_kept_out_until_it_is_unplugged() {
    let mut app = seeded();
    new_device(&mut app, 5, true, false);
    let code = code_of(&app);
    // Three codes' worth of wrong letters, as a device trying codes one
    // after another types them.
    type_on(&mut app, 5, &"z".repeat(18));
    assert_eq!(app.poll_input_decision(), Some((5, false)));
    assert_eq!(app.ignored_inputs.len(), 1);
    assert_eq!(app.sheet, None);
    // The right code no longer gets it in.
    type_on(&mut app, 5, &code);
    assert_eq!(app.poll_input_decision(), None);
    // Plugged in again, it is asked for a code again.
    app.storage(StorageEvent::InputGone { id: 5 });
    new_device(&mut app, 6, true, false);
    let code = code_of(&app);
    type_on(&mut app, 6, &code);
    assert_eq!(app.poll_input_decision(), Some((6, true)));
}

#[test]
fn after_three_keyboards_are_kept_out_new_keyboards_get_no_code() {
    let mut app = seeded();
    // A device that unplugs and plugs itself back in, guessing each time.
    for id in 1..=3 {
        new_device(&mut app, id, true, false);
        type_on(&mut app, id, &"z".repeat(18));
        assert_eq!(app.poll_input_decision(), Some((id, false)));
        app.storage(StorageEvent::InputGone { id });
    }
    new_device(&mut app, 4, true, false);
    assert_eq!(app.poll_input_decision(), Some((4, false)));
    assert!(app.inputs.is_empty());
    assert_eq!(app.sheet, None);
    assert_eq!(app.ignored_inputs.len(), 1);
    // A new pointer is still asked about.
    new_device(&mut app, 5, false, true);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputUse(5));
    assert_eq!(app.poll_input_decision(), Some((5, true)));
    // Faraday started again asks new keyboards for a code again.
    let mut app = seeded();
    new_device(&mut app, 6, true, false);
    let code = code_of(&app);
    type_on(&mut app, 6, &code);
    assert_eq!(app.poll_input_decision(), Some((6, true)));
}

#[test]
fn keyboards_a_person_ignores_do_not_count_against_new_ones() {
    let mut app = seeded();
    for id in 1..=3 {
        new_device(&mut app, id, true, false);
        app.press(Action::InputIgnore(id));
        assert_eq!(app.poll_input_decision(), Some((id, false)));
    }
    new_device(&mut app, 4, true, false);
    let code = code_of(&app);
    type_on(&mut app, 4, &code);
    assert_eq!(app.poll_input_decision(), Some((4, true)));
}

#[test]
fn a_person_who_mistypes_the_code_twice_can_type_it_again() {
    let mut app = seeded();
    new_device(&mut app, 3, true, false);
    let code = code_of(&app);
    type_on(&mut app, 3, "zzzzzzzzzzzz");
    assert_eq!(app.poll_input_decision(), None);
    type_on(&mut app, 3, &code);
    assert_eq!(app.poll_input_decision(), Some((3, true)));
}

#[test]
fn a_new_pointer_is_believed_when_a_person_says_so() {
    let mut app = seeded();
    new_device(&mut app, 7, false, true);
    app.press(Action::InputUse(7));
    assert_eq!(app.poll_input_decision(), Some((7, true)));
}

#[test]
fn an_ignored_device_stays_out_until_it_is_unplugged() {
    let mut app = seeded();
    new_device(&mut app, 2, true, true);
    app.press(Action::InputIgnore(2));
    assert_eq!(app.poll_input_decision(), Some((2, false)));
    assert_eq!(app.ignored_inputs.len(), 1);
    // Typing on it now does nothing.
    type_on(&mut app, 2, "acdefh");
    assert_eq!(app.poll_input_decision(), None);
    app.storage(StorageEvent::InputGone { id: 2 });
    assert!(app.ignored_inputs.is_empty());
}

#[test]
fn closing_another_sheet_brings_a_waiting_device_back() {
    let mut app = seeded();
    new_device(&mut app, 1, true, false);
    app.press(Action::PowerAsk);
    app.press(Action::Cancel);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
}

#[test]
fn a_keyboard_before_the_systems_randomness_gets_no_code_until_it_arrives() {
    let mut app = testkit::started();
    new_device(&mut app, 8, true, false);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert_eq!(app.inputs[0].code, None);
    // Nothing it types lets it in or counts against it.
    type_on(&mut app, 8, &"acdefh".repeat(4));
    assert_eq!(app.poll_input_decision(), None);
    assert_eq!(app.inputs.len(), 1);
    // Past the first moments with no randomness, the sheet says so.
    app.event(Event::Tick { now_ms: 1 });
    app.event(Event::Tick { now_ms: 5_000 });
    assert!(app.drawn_texts().iter().any(|t| t == EN.new_input_no_code));
    // The randomness arrives: the keyboard gets its code, and typing it
    // lets it in.
    app.event(Event::Entropy(EntropyBytes::new([9u8; 32])));
    let code = code_of(&app);
    assert_eq!(code.len(), 6);
    assert!(!app.drawn_texts().iter().any(|t| t == EN.new_input_no_code));
    type_on(&mut app, 8, &code);
    assert_eq!(app.poll_input_decision(), Some((8, true)));
}
