//! A new input device is believed only once a person at the screen says
//! so (`PLAN.md` §4.6): a keyboard by typing the code shown, a pointer by
//! input already believed; ignored devices stay out until unplugged.

use faraday_core::{Action, Faraday, Sheet, StorageEvent};

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
    let mut app = Faraday::new();
    new_device(&mut app, 4, true, false);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    let code = app.inputs[0].code.clone();
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
fn a_new_pointer_is_believed_when_a_person_says_so() {
    let mut app = Faraday::new();
    new_device(&mut app, 7, false, true);
    app.press(Action::InputUse(7));
    assert_eq!(app.poll_input_decision(), Some((7, true)));
}

#[test]
fn an_ignored_device_stays_out_until_it_is_unplugged() {
    let mut app = Faraday::new();
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
    let mut app = Faraday::new();
    new_device(&mut app, 1, true, false);
    app.press(Action::PowerAsk);
    app.press(Action::Cancel);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
}
