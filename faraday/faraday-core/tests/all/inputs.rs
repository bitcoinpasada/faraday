//! A new input device is believed only once a person at the screen says
//! so (`docs/history/PLAN.md` §4.6): a keyboard by typing the code shown, a pointer by
//! input already believed; ignored devices, and keyboards that keep
//! guessing the code, stay out until unplugged, and after three such
//! keyboards new ones get no code until Faraday starts again. The code is
//! drawn from the session's seed: a keyboard that arrives before the
//! system gave any randomness gets no code, cannot be let in, and the
//! sheet says so; it gets its code once the randomness arrives. A new
//! USB touchscreen is believed once it taps the target drawn at a place
//! the randomness gives; three taps beside it keep it out until it is
//! unplugged, as a keyboard's wrong letters do (F153). Several targets in
//! a row, a pointer with no other input approving itself the same way,
//! and every press beside a target counted for the session, whatever
//! becomes of the device, are F202, which also has a held device ask only
//! when it is used: a notice at plug-in, its sheet at its first act.

use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Sheet, StorageEvent, inputs, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

/// An app whose shell answered its first request for randomness.
fn seeded() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Entropy(EntropyBytes::new([7u8; 32])));
    app
}

/// The code the first waiting device must type.
fn code_of(app: &Faraday) -> String {
    app.new_inputs()[0].code.clone().expect("a code")
}

fn new_device(app: &mut Faraday, id: u32, keyboard: bool, pointer: bool) {
    app.storage(StorageEvent::NewInput {
        id,
        name: "USB Keyboard".into(),
        keyboard,
        pointer,
        touch: false,
    });
    app.storage(StorageEvent::InputUsed { id });
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
    assert!(app.new_inputs().is_empty());
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
    assert_eq!(app.ignored_inputs().len(), 1);
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
    assert!(app.new_inputs().is_empty());
    assert_eq!(app.sheet, None);
    assert_eq!(app.ignored_inputs().len(), 1);
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
    assert_eq!(app.ignored_inputs().len(), 1);
    // Typing on it now does nothing.
    type_on(&mut app, 2, "acdefh");
    assert_eq!(app.poll_input_decision(), None);
    app.storage(StorageEvent::InputGone { id: 2 });
    assert!(app.ignored_inputs().is_empty());
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
    assert_eq!(app.new_inputs()[0].code, None);
    // Nothing it types lets it in or counts against it.
    type_on(&mut app, 8, &"acdefh".repeat(4));
    assert_eq!(app.poll_input_decision(), None);
    assert_eq!(app.new_inputs().len(), 1);
    // A minute with no randomness, the sheet says so.
    app.event(Event::Tick { now_ms: 1 });
    app.event(Event::Tick { now_ms: 61_000 });
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

/// An app on a 1280 by 800 screen whose shell answered its first request
/// for randomness with `seed`.
fn on_screen(seed: u8) -> Faraday {
    on_size(seed, (1280, 800), 160)
}

/// The same on a `width` by `height` screen at `dpi`.
fn on_size(seed: u8, (width, height): (u16, u16), dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::ReduceMotion(true));
    app.event(Event::Entropy(EntropyBytes::new([seed; 32])));
    app
}

/// A USB touchscreen plugged in, and the frame that shows its target.
fn new_touchscreen(app: &mut Faraday, id: u32) {
    app.storage(StorageEvent::NewInput {
        id,
        name: "USB Touchscreen".into(),
        keyboard: false,
        pointer: false,
        touch: true,
    });
    app.storage(StorageEvent::InputUsed { id });
    let _ = app.frame();
}

/// The held-back touchscreen's finger down at (x, y), and the next frame.
fn tap_held(app: &mut Faraday, id: u32, (x, y): (u16, u16)) {
    app.storage(StorageEvent::InputTapped { id, x, y });
    let _ = app.frame();
}

/// The target number `step` of the run as it shows, with how many there
/// are: the sheet offers it as `InputTarget(id, step, of)`.
fn shown(app: &Faraday, id: u32, step: u8) -> Option<((u16, u16), u8)> {
    (1..=16).find_map(|of| {
        app.where_offered(Action::InputTarget(id, step, of))
            .map(|at| (at, of))
    })
}

/// Where the first target is drawn.
fn target(app: &Faraday, id: u32) -> (u16, u16) {
    shown(app, id, 0).expect("the target is on screen").0
}

/// A place level with the target showing, at the other side of the screen.
fn beside(app: &Faraday, id: u32) -> (u16, u16) {
    let step = app.new_inputs()[0].reached as u8;
    let (x, y) = shown(app, id, step).expect("the target is on screen").0;
    (if x > 640 { 40 } else { 1240 }, y)
}

/// Presses each target as it shows, until the device is in; how many it
/// took.
fn get_in(app: &mut Faraday, id: u32) -> usize {
    for n in 1..=16 {
        let step = app
            .new_inputs()
            .iter()
            .find(|d| d.id == id)
            .expect("held")
            .reached as u8;
        let (at, _) = shown(app, id, step).expect("a target shows");
        tap_held(app, id, at);
        if app.new_inputs().iter().all(|d| d.id != id) {
            return n;
        }
    }
    panic!("never let in");
}

/// A USB pointer plugged in with nothing trusted, and the frame that
/// shows its target.
fn new_pointer(app: &mut Faraday, id: u32) {
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: false,
    });
    app.storage(StorageEvent::NewInput {
        id,
        name: "USB Mouse".into(),
        keyboard: false,
        pointer: true,
        touch: false,
    });
    app.storage(StorageEvent::InputUsed { id });
    let _ = app.frame();
}

/// A press on the sheet's Ignore.
fn press_ignore(app: &mut Faraday, id: u32) {
    let at = app
        .where_offered(Action::InputIgnore(id))
        .expect("Ignore is on screen");
    tap_held(app, id, at);
}

#[test]
fn a_new_touchscreen_is_believed_once_it_taps_the_target() {
    let mut app = on_screen(7);
    new_touchscreen(&mut app, 4);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == EN.new_input_tap_target)
    );
    // A tap beside it lets nothing in and presses nothing.
    let at = beside(&app, 4);
    tap_held(&mut app, 4, at);
    assert_eq!(app.poll_input_decision(), None);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    get_in(&mut app, 4);
    assert_eq!(app.poll_input_decision(), Some((4, true)));
    assert!(app.new_inputs().is_empty());
    assert_eq!(app.sheet, None);
}

#[test]
fn a_tap_beside_the_target_counts_and_the_target_stays_where_it_was() {
    let mut app = on_screen(7);
    new_touchscreen(&mut app, 4);
    let (_, of) = shown(&app, 4, 0).expect("a target");
    let before = app.hit_box(Action::InputTarget(4, 0, of));
    let at = beside(&app, 4);
    tap_held(&mut app, 4, at);
    assert!(app.drawn_texts().iter().any(|t| t == "Tries left: 2"));
    assert_eq!(app.hit_box(Action::InputTarget(4, 0, of)), before);
}

#[test]
fn a_touchscreen_that_keeps_tapping_beside_the_target_is_kept_out_until_unplugged() {
    let mut app = on_screen(7);
    new_touchscreen(&mut app, 5);
    let at = beside(&app, 5);
    let on = target(&app, 5);
    for _ in 0..3 {
        tap_held(&mut app, 5, at);
    }
    assert_eq!(app.poll_input_decision(), Some((5, false)));
    assert_eq!(app.ignored_inputs().len(), 1);
    assert_eq!(app.sheet, None);
    assert_eq!(
        app.toast_text(),
        Some("USB Touchscreen tapped beside the target. Unplug it to try again.")
    );
    // Where the target was no longer lets it in.
    tap_held(&mut app, 5, on);
    assert_eq!(app.poll_input_decision(), None);
    // Plugged in again, it is asked again.
    app.storage(StorageEvent::InputGone { id: 5 });
    new_touchscreen(&mut app, 6);
    get_in(&mut app, 6);
    assert_eq!(app.poll_input_decision(), Some((6, true)));
}

#[test]
fn after_three_devices_are_kept_out_new_touchscreens_are_ignored() {
    let mut app = on_screen(7);
    for id in 1..=3 {
        new_touchscreen(&mut app, id);
        let at = beside(&app, id);
        for _ in 0..3 {
            tap_held(&mut app, id, at);
        }
        assert_eq!(app.poll_input_decision(), Some((id, false)));
        app.storage(StorageEvent::InputGone { id });
    }
    new_touchscreen(&mut app, 4);
    assert_eq!(app.poll_input_decision(), Some((4, false)));
    assert!(app.new_inputs().is_empty());
    // And new keyboards with them.
    new_device(&mut app, 5, true, false);
    assert_eq!(app.poll_input_decision(), Some((5, false)));
}

#[test]
fn a_new_touchscreen_can_ignore_itself() {
    let mut app = on_screen(7);
    new_touchscreen(&mut app, 2);
    press_ignore(&mut app, 2);
    assert_eq!(app.poll_input_decision(), Some((2, false)));
    assert_eq!(app.ignored_inputs().len(), 1);
}

#[test]
fn the_targets_place_comes_from_the_systems_randomness() {
    let mut a = on_screen(7);
    let mut b = on_screen(9);
    new_touchscreen(&mut a, 1);
    new_touchscreen(&mut b, 1);
    assert_ne!(target(&a, 1), target(&b, 1));
}

#[test]
fn taps_while_another_sheet_is_up_count_for_nothing() {
    let mut app = on_screen(7);
    app.press(Action::PowerAsk);
    new_touchscreen(&mut app, 3);
    assert_eq!(app.sheet, Some(Sheet::Power));
    for _ in 0..5 {
        tap_held(&mut app, 3, (640, 400));
    }
    assert_eq!(app.poll_input_decision(), None);
    app.press(Action::Cancel);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    get_in(&mut app, 3);
    assert_eq!(app.poll_input_decision(), Some((3, true)));
}

#[test]
fn a_touchscreen_before_the_systems_randomness_has_no_target_until_it_arrives() {
    let mut app = testkit::started();
    new_touchscreen(&mut app, 8);
    assert_eq!(shown(&app, 8, 0), None);
    // Nothing it taps lets it in or counts against it.
    for _ in 0..5 {
        tap_held(&mut app, 8, (100, 300));
    }
    assert_eq!(app.poll_input_decision(), None);
    app.event(Event::Tick { now_ms: 1 });
    app.event(Event::Tick { now_ms: 61_000 });
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == EN.new_input_no_target)
    );
    app.event(Event::Entropy(EntropyBytes::new([9u8; 32])));
    let _ = app.frame();
    get_in(&mut app, 8);
    assert_eq!(app.poll_input_decision(), Some((8, true)));
}

/// A device that is kept out after its press count is `presses` beside a
/// target, over rounds of two presses beside then `leave`: how many
/// presses it took for the door to close.
fn presses_until_closed(
    app: &mut Faraday,
    plug: fn(&mut Faraday, u32),
    leave: fn(&mut Faraday, u32),
) -> usize {
    let mut presses = 0;
    for id in 1..=10 {
        plug(app, id);
        for _ in 0..2 {
            let at = beside(app, id);
            tap_held(app, id, at);
            presses += 1;
            if app.poll_input_decision() == Some((id, false)) {
                return presses;
            }
        }
        leave(app, id);
        let _ = app.poll_input_decision();
    }
    panic!("the door never closed");
}

fn unplug(app: &mut Faraday, id: u32) {
    app.storage(StorageEvent::InputGone { id });
}

#[test]
fn one_press_does_not_let_a_device_in_and_each_target_in_turn_does() {
    for pointer in [true, false] {
        let mut app = on_screen(7);
        if pointer {
            new_pointer(&mut app, 1);
        } else {
            new_touchscreen(&mut app, 1);
        }
        let at = target(&app, 1);
        tap_held(&mut app, 1, at);
        assert_eq!(app.poll_input_decision(), None);
        assert_eq!(app.new_inputs().len(), 1);
        // Back to the first, the way a person starts again.
        app.storage(StorageEvent::InputGone { id: 1 });
        if pointer {
            new_pointer(&mut app, 2);
        } else {
            new_touchscreen(&mut app, 2);
        }
        assert!(get_in(&mut app, 2) > 1);
        assert_eq!(app.poll_input_decision(), Some((2, true)));
    }
}

#[test]
fn a_miss_on_a_later_target_goes_back_to_the_first_at_the_place_it_had() {
    let mut app = on_screen(7);
    new_pointer(&mut app, 1);
    let first = target(&app, 1);
    tap_held(&mut app, 1, first);
    assert_eq!(app.new_inputs()[0].reached, 1);
    let at = beside(&app, 1);
    tap_held(&mut app, 1, at);
    assert_eq!(app.new_inputs()[0].reached, 0);
    assert_eq!(target(&app, 1), first);
}

#[test]
fn pressing_one_place_twice_before_a_frame_is_no_hit_and_no_wrong_try() {
    let mut app = on_screen(7);
    new_pointer(&mut app, 1);
    let (first, of) = shown(&app, 1, 0).expect("target");
    assert!(of > 1);
    app.storage(StorageEvent::InputTapped {
        id: 1,
        x: first.0,
        y: first.1,
    });
    // The frame the second press would be on has not been drawn.
    app.storage(StorageEvent::InputTapped {
        id: 1,
        x: first.0,
        y: first.1,
    });
    let _ = app.frame();
    assert_eq!(app.new_inputs()[0].reached, 1);
    assert_eq!(app.new_inputs()[0].missed, 0);
    assert!(
        !app.drawn_texts()
            .iter()
            .any(|t| t.starts_with("Tries left"))
    );
    let (at, _) = shown(&app, 1, 1).expect("the second target");
    tap_held(&mut app, 1, at);
    assert_eq!(app.new_inputs().first().map_or(2, |d| d.reached), 2);
}

#[test]
fn the_sheet_says_which_target_of_how_many() {
    let mut app = on_screen(7);
    new_pointer(&mut app, 1);
    let (at, of) = shown(&app, 1, 0).expect("target");
    let says = |app: &mut Faraday, n: usize| {
        let want = format!("Target {n} of {of}");
        app.drawn_texts().contains(&want)
    };
    assert!(says(&mut app, 1));
    tap_held(&mut app, 1, at);
    assert!(says(&mut app, 2));
}

#[test]
fn with_believed_input_one_press_on_a_touchscreens_target_lets_it_in() {
    let mut app = on_screen(7);
    new_device(&mut app, 1, false, true);
    app.press(Action::InputUse(1));
    assert_eq!(app.poll_input_decision(), Some((1, true)));
    new_touchscreen(&mut app, 2);
    let (_, of) = shown(&app, 2, 0).expect("target");
    app.press(Action::InputTarget(2, 0, of));
    assert_eq!(app.poll_input_decision(), Some((2, true)));
}

#[test]
fn a_screen_too_small_to_hide_a_target_shows_none_and_ignore_works() {
    let mut app = on_size(7, (160, 200), 160);
    new_touchscreen(&mut app, 1);
    assert_eq!(shown(&app, 1, 0), None);
    for _ in 0..9 {
        tap_held(&mut app, 1, (80, 100));
    }
    assert_eq!(app.poll_input_decision(), None);
    assert_eq!(app.new_inputs()[0].missed, 0);
    // The door is still open for a real keyboard.
    new_device(&mut app, 2, true, false);
    assert!(app.new_inputs().iter().any(|d| d.id == 2));
    press_ignore(&mut app, 1);
    assert_eq!(app.poll_input_decision(), Some((1, false)));
}

#[test]
fn a_smaller_screen_takes_more_presses() {
    let mut big = on_size(7, (1920, 1080), 160);
    let mut small = on_size(7, (480, 640), 286);
    new_pointer(&mut big, 1);
    new_pointer(&mut small, 1);
    assert!(get_in(&mut small, 1) > get_in(&mut big, 1));
}

#[test]
fn a_new_pointer_with_nothing_trusted_is_let_in_by_its_targets() {
    let mut app = on_screen(7);
    new_pointer(&mut app, 1);
    assert!(app.input_aims(1));
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == EN.new_input_press_target)
    );
    get_in(&mut app, 1);
    assert_eq!(app.poll_input_decision(), Some((1, true)));
    assert_eq!(app.toast_text(), Some("Pointer in use: USB Mouse"));
}

#[test]
fn with_trusted_input_a_new_pointer_gets_the_dialog_and_presses_do_nothing() {
    let mut app = on_screen(7);
    new_device(&mut app, 1, false, true);
    let _ = app.frame();
    assert!(!app.input_aims(1));
    assert_eq!(shown(&app, 1, 0), None);
    for _ in 0..5 {
        tap_held(&mut app, 1, (640, 400));
    }
    assert_eq!(app.poll_input_decision(), None);
    assert_eq!(app.new_inputs().len(), 1);
    app.press(Action::InputUse(1));
    assert_eq!(app.poll_input_decision(), Some((1, true)));
}

#[test]
fn a_pointer_that_presses_beside_three_times_is_kept_out_until_unplugged() {
    let mut app = on_screen(7);
    new_pointer(&mut app, 1);
    for _ in 0..3 {
        let at = beside(&app, 1);
        tap_held(&mut app, 1, at);
    }
    assert_eq!(app.poll_input_decision(), Some((1, false)));
    assert_eq!(
        app.toast_text(),
        Some("USB Mouse pressed beside the target. Unplug it to try again.")
    );
    unplug(&mut app, 1);
    new_pointer(&mut app, 2);
    assert!(shown(&app, 2, 0).is_some());
}

#[test]
fn presses_beside_are_counted_for_the_session_not_the_device() {
    for kind in 0..3 {
        let mut app = on_screen(7);
        let plug = if kind == 1 {
            new_touchscreen
        } else {
            new_pointer
        };
        let leave = if kind == 2 { leave_by_ignore } else { unplug };
        let presses = presses_until_closed(&mut app, plug, leave);
        assert_eq!(presses, 9);
        // A new one is turned away, with nothing to press.
        plug(&mut app, 20);
        assert!(app.new_inputs().is_empty());
        assert_eq!(shown(&app, 20, 0), None);
        assert_eq!(app.poll_input_decision(), Some((20, false)));
    }
}

fn leave_by_ignore(app: &mut Faraday, id: u32) {
    press_ignore(app, id);
}

#[test]
fn six_pointers_held_at_once_get_nine_wrong_presses_and_the_rest_are_kept_out() {
    let mut app = on_screen(7);
    for id in 1..=6 {
        new_pointer(&mut app, id);
    }
    let mut presses = 0;
    while let Some(first) = app.new_inputs().first().map(|d| d.id) {
        let at = beside(&app, first);
        tap_held(&mut app, first, at);
        presses += 1;
        assert!(presses <= 9);
    }
    assert_eq!(presses, 9);
    let mut out = Vec::new();
    while let Some((id, believed)) = app.poll_input_decision() {
        assert!(!believed);
        out.push(id);
    }
    out.sort_unstable();
    assert_eq!(out, [1, 2, 3, 4, 5, 6]);
}

#[test]
fn ignoring_a_pointer_counts_nothing() {
    let mut app = on_screen(7);
    for id in 1..=8 {
        new_pointer(&mut app, id);
        press_ignore(&mut app, id);
        assert_eq!(app.poll_input_decision(), Some((id, false)));
        unplug(&mut app, id);
    }
    new_pointer(&mut app, 9);
    assert!(shown(&app, 9, 0).is_some());
}

#[test]
fn past_the_limit_a_new_pointer_is_ignored_when_alone_and_asked_when_not() {
    let mut app = on_screen(7);
    presses_until_closed(&mut app, new_pointer, unplug);
    assert_eq!(app.toast_text(), Some(EN.inputs_new_devices_ignored));
    new_pointer(&mut app, 30);
    assert_eq!(app.poll_input_decision(), Some((30, false)));
    assert!(app.new_inputs().is_empty());

    // With trusted input, touchscreens close the door, and a pointer is
    // still asked about.
    let mut app = on_screen(7);
    presses_until_closed(&mut app, new_touchscreen, unplug);
    new_device(&mut app, 30, false, true);
    assert_eq!(app.new_inputs().len(), 1);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    // The trusted device is unplugged while the pointer is held for its
    // dialog: its hits believe nothing, and it is turned away.
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: false,
    });
    assert!(app.new_inputs().is_empty());
    assert_eq!(app.poll_input_decision(), Some((30, false)));
    assert_eq!(app.sheet, None);
    for _ in 0..16 {
        app.storage(StorageEvent::InputTapped {
            id: 30,
            x: 640,
            y: 400,
        });
    }
    assert_eq!(app.poll_input_decision(), None);
}

#[test]
fn a_keyboard_with_a_pointer_gets_no_target_and_its_code_lets_it_in() {
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: false,
    });
    new_device(&mut app, 1, true, true);
    let _ = app.frame();
    assert!(!app.input_aims(1));
    assert_eq!(shown(&app, 1, 0), None);
    let code = code_of(&app);
    type_on(&mut app, 1, &code);
    assert_eq!(app.poll_input_decision(), Some((1, true)));
    assert_eq!(
        app.toast_text(),
        Some("Keyboard and pointer in use: USB Keyboard")
    );
}

#[test]
fn the_toast_shows_over_the_next_devices_sheet() {
    let mut app = on_screen(7);
    new_pointer(&mut app, 1);
    new_device(&mut app, 2, true, false);
    get_in(&mut app, 1);
    let _ = app.frame();
    assert_eq!(app.new_inputs()[0].id, 2);
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Pointer in use: USB Mouse")
    );
}

#[test]
fn under_another_sheet_a_pointer_does_not_aim_and_presses_count_nothing() {
    let mut app = on_screen(7);
    app.press(Action::PowerAsk);
    new_pointer(&mut app, 1);
    assert_eq!(app.sheet, Some(Sheet::Power));
    assert!(!app.input_aims(1));
    for _ in 0..5 {
        tap_held(&mut app, 1, (640, 400));
    }
    assert_eq!(app.poll_input_decision(), None);
    app.press(Action::Cancel);
    let _ = app.frame();
    assert!(app.input_aims(1));
    assert!(shown(&app, 1, 0).is_some());
}

#[test]
fn a_touchscreen_never_aims() {
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: false,
    });
    new_touchscreen(&mut app, 1);
    assert!(!app.input_aims(1));
}

#[test]
fn a_pointer_before_the_randomness_has_no_target_and_can_still_be_ignored() {
    let mut app = testkit::started();
    new_pointer(&mut app, 1);
    assert!(app.input_aims(1));
    assert_eq!(shown(&app, 1, 0), None);
    for _ in 0..5 {
        tap_held(&mut app, 1, (100, 300));
    }
    assert_eq!(app.poll_input_decision(), None);
    assert_eq!(app.new_inputs()[0].missed, 0);
    press_ignore(&mut app, 1);
    assert_eq!(app.poll_input_decision(), Some((1, false)));
}

/// A device plugged in and not used: the kinds are (keyboard, pointer,
/// touch).
fn announce(
    app: &mut Faraday,
    id: u32,
    name: &str,
    (keyboard, pointer, touch): (bool, bool, bool),
) {
    app.storage(StorageEvent::NewInput {
        id,
        name: name.into(),
        keyboard,
        pointer,
        touch,
    });
    let _ = app.frame();
}

#[test]
fn a_new_device_shows_a_notice_with_its_name_and_no_sheet() {
    for (id, kinds) in [
        (1, (true, false, false)),
        (2, (false, true, false)),
        (3, (false, false, true)),
    ] {
        let mut app = on_screen(7);
        app.storage(StorageEvent::InputTrust {
            pointer: false,
            keyboard: false,
        });
        announce(&mut app, id, "USB Thing", kinds);
        assert_eq!(app.sheet, None);
        assert!(
            app.drawn_texts()
                .iter()
                .any(|t| t == "New device: USB Thing")
        );
        // The screen under it still takes presses.
        app.press(Action::PowerAsk);
        assert_eq!(app.sheet, Some(Sheet::Power));
    }
}

#[test]
fn a_keyboards_first_letter_brings_up_the_code_and_counts() {
    let mut app = seeded();
    announce(&mut app, 4, "USB Keyboard", (true, false, false));
    let code = code_of(&app);
    let (first, rest) = code.split_at(1);
    type_on(&mut app, 4, first);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    type_on(&mut app, 4, rest);
    assert_eq!(app.poll_input_decision(), Some((4, true)));
    assert_eq!(app.sheet, None);
}

#[test]
fn a_pointers_first_motion_and_a_touchscreens_first_touch_bring_up_targets_and_count_for_nothing() {
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: false,
    });
    announce(&mut app, 2, "USB Mouse", (false, true, false));
    assert_eq!(app.sheet, None);
    app.storage(StorageEvent::InputUsed { id: 2 });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert!(shown(&app, 2, 0).is_some());
    assert_eq!(app.new_inputs()[0].missed, 0);
    press_ignore(&mut app, 2);

    let mut app = on_screen(7);
    announce(&mut app, 3, "USB Touchscreen", (false, false, true));
    // A touch where a target would be, before anything was drawn.
    app.storage(StorageEvent::InputTapped {
        id: 3,
        x: 640,
        y: 400,
    });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert!(shown(&app, 3, 0).is_some());
    assert_eq!(app.new_inputs()[0].missed, 0);
    assert_eq!(app.new_inputs()[0].reached, 0);
}

#[test]
fn a_receivers_parts_ask_only_for_the_one_used() {
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: false,
    });
    for id in 1..=5 {
        announce(&mut app, id, "USB Receiver", (false, true, false));
    }
    announce(&mut app, 6, "USB Receiver", (true, false, false));
    assert_eq!(app.sheet, None);
    assert_eq!(app.poll_input_decision(), None);
    // The mouse part is used, last of the six to be plugged in or not.
    app.storage(StorageEvent::InputUsed { id: 3 });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert!(shown(&app, 3, 0).is_some());
    get_in(&mut app, 3);
    assert_eq!(app.poll_input_decision(), Some((3, true)));
    assert_eq!(app.sheet, None);
    assert_eq!(app.new_inputs().len(), 5);
    assert_eq!(app.poll_input_decision(), None);
}

#[test]
fn with_trusted_input_a_pointer_gets_its_dialog_when_it_moves() {
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: true,
        keyboard: false,
    });
    announce(&mut app, 2, "USB Mouse", (false, true, false));
    assert_eq!(app.sheet, None);
    app.storage(StorageEvent::InputUsed { id: 2 });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert!(app.where_offered(Action::InputUse(2)).is_some());
}

/// A trusted keyboard and nothing trusted that points, then a USB mouse
/// that has moved: its sheet is up.
fn mouse_beside_a_keyboard(trusted_keyboard: bool) -> Faraday {
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: trusted_keyboard,
    });
    announce(&mut app, 2, "USB Mouse", (false, true, false));
    app.storage(StorageEvent::InputUsed { id: 2 });
    let _ = app.frame();
    app
}

#[test]
fn with_only_a_trusted_keyboard_a_mouse_gets_its_targets_and_arrow() {
    let mut app = mouse_beside_a_keyboard(true);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert!(app.input_aims(2));
    assert!(app.where_offered(Action::InputUse(2)).is_none());
    get_in(&mut app, 2);
    assert_eq!(app.poll_input_decision(), Some((2, true)));
}

/// The keyboard stays quiet with the sheet up for `ms`.
fn wait(app: &mut Faraday, ms: u64) {
    let now = app.now_ms() + ms;
    app.event(Event::Tick { now_ms: now });
}

#[test]
fn enter_on_a_trusted_keyboard_lets_the_shown_mouse_in_and_the_sheet_says_so() {
    let mut app = mouse_beside_a_keyboard(true);
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == EN.new_input_enter_also)
    );
    wait(&mut app, inputs::ENTER_QUIET_MS);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert_eq!(app.poll_input_decision(), Some((2, true)));
    assert_eq!(app.toast_text(), Some("Pointer in use: USB Mouse"));
    assert_eq!(app.sheet, None);
}

#[test]
fn enter_as_a_mouse_sheet_comes_up_lets_nothing_in() {
    // A mouse that moves as a person presses Enter on what they are typing.
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: true,
    });
    announce(&mut app, 2, "USB Mouse", (false, true, false));
    app.storage(StorageEvent::InputUsed { id: 2 });
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    let _ = app.frame();
    assert_eq!(app.poll_input_decision(), None);
    assert_eq!(app.new_inputs().len(), 1);

    // Drawn, but not up long enough to have been read.
    wait(&mut app, inputs::ENTER_QUIET_MS - 1);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert_eq!(app.poll_input_decision(), None);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
}

#[test]
fn enter_at_the_end_of_typing_lets_nothing_in_however_long_the_sheet_was_up() {
    let mut app = mouse_beside_a_keyboard(true);
    for _ in 0..5 {
        wait(&mut app, inputs::ENTER_QUIET_MS / 2);
        app.event(Event::Key(faraday_shell_api::Key::Char('a')));
    }
    wait(&mut app, inputs::ENTER_QUIET_MS / 2);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert_eq!(app.poll_input_decision(), None);

    // The person stops and reads, and means it.
    wait(&mut app, inputs::ENTER_QUIET_MS);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert_eq!(app.poll_input_decision(), Some((2, true)));
}

#[test]
fn enter_on_ignore_keeps_the_mouse_out() {
    let mut app = mouse_beside_a_keyboard(true);
    // Tab goes to the first control drawn, Ignore.
    app.event(Event::Key(faraday_shell_api::Key::Tab));
    let _ = app.frame();
    wait(&mut app, inputs::ENTER_QUIET_MS);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    let _ = app.frame();
    assert_eq!(app.poll_input_decision(), Some((2, false)));
}

#[test]
fn without_a_trusted_keyboard_the_sheet_has_no_enter_line_and_enter_does_nothing() {
    let mut app = mouse_beside_a_keyboard(false);
    assert!(app.input_aims(2));
    assert!(
        !app.drawn_texts()
            .iter()
            .any(|t| t == EN.new_input_enter_also)
    );
    wait(&mut app, inputs::ENTER_QUIET_MS);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert_eq!(app.poll_input_decision(), None);
    assert_eq!(app.new_inputs().len(), 1);
}

#[test]
fn with_a_trusted_pointer_and_keyboard_the_mouse_gets_the_dialog_and_no_aim() {
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: true,
        keyboard: true,
    });
    announce(&mut app, 2, "USB Mouse", (false, true, false));
    app.storage(StorageEvent::InputUsed { id: 2 });
    let _ = app.frame();
    assert!(!app.input_aims(2));
    assert!(app.where_offered(Action::InputUse(2)).is_some());
    assert!(
        !app.drawn_texts()
            .iter()
            .any(|t| t == EN.new_input_enter_also)
    );
}

#[test]
fn wrong_tries_still_count_with_a_trusted_keyboard() {
    let mut app = mouse_beside_a_keyboard(true);
    for n in 1..=3 {
        let at = beside(&app, 2);
        tap_held(&mut app, 2, at);
        if n < 3 {
            assert_eq!(app.new_inputs().len(), 1);
        }
    }
    assert!(app.new_inputs().is_empty());
    assert_eq!(app.poll_input_decision(), Some((2, false)));
}

#[test]
fn a_keyboard_believed_before_the_mouse_moved_does_not_stop_its_targets() {
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: false,
    });
    announce(&mut app, 1, "USB Keyboard", (true, false, false));
    let code = code_of(&app);
    type_on(&mut app, 1, &code);
    assert_eq!(app.poll_input_decision(), Some((1, true)));
    announce(&mut app, 2, "USB Mouse", (false, true, false));
    app.storage(StorageEvent::InputUsed { id: 2 });
    let _ = app.frame();
    assert!(app.input_aims(2));
    wait(&mut app, inputs::ENTER_QUIET_MS);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert_eq!(app.poll_input_decision(), Some((2, true)));
}

#[test]
fn two_devices_used_in_turn_take_turns_and_the_second_keeps_counting() {
    let mut app = seeded();
    announce(&mut app, 1, "USB Keyboard", (true, false, false));
    announce(&mut app, 2, "Other Keyboard", (true, false, false));
    let second = app.new_inputs()[1].code.clone().unwrap();
    type_on(&mut app, 2, &second[..1]);
    type_on(&mut app, 1, "z");
    assert_eq!(app.new_inputs()[0].id, 2);
    // The one that acted first shows; the other is behind it.
    type_on(&mut app, 1, "zzzz");
    assert_eq!(app.new_inputs()[1].letters, 5);
    app.press(Action::InputIgnore(2));
    assert_eq!(app.shown_input().map(|d| d.id), Some(1));
    assert_eq!(app.sheet, Some(Sheet::NewInput));
}

#[test]
fn a_device_used_under_another_sheet_waits_for_it() {
    let mut app = seeded();
    announce(&mut app, 1, "USB Keyboard", (true, false, false));
    app.press(Action::PowerAsk);
    type_on(&mut app, 1, "z");
    assert_eq!(app.sheet, Some(Sheet::Power));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
}

/// A pointer plugged in and used, and no frame drawn yet.
fn used_undrawn(app: &mut Faraday, id: u32) {
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: false,
    });
    app.storage(StorageEvent::NewInput {
        id,
        name: "USB Mouse".into(),
        keyboard: false,
        pointer: true,
        touch: false,
    });
    app.storage(StorageEvent::InputUsed { id });
}

#[test]
fn a_double_click_on_a_sheet_not_yet_drawn_counts_nothing() {
    let mut app = on_screen(7);
    used_undrawn(&mut app, 1);
    for _ in 0..4 {
        app.storage(StorageEvent::InputTapped {
            id: 1,
            x: 640,
            y: 400,
        });
    }
    let _ = app.frame();
    assert_eq!(app.new_inputs()[0].missed, 0);
    assert!(
        !app.drawn_texts()
            .iter()
            .any(|t| t.starts_with("Tries left"))
    );
}

#[test]
fn a_press_just_after_the_seed_or_another_devices_sheet_counts_nothing() {
    // Before the seed: nothing drawn; the seed arrives and the next press
    // lands before the frame.
    let mut app = testkit::started();
    new_pointer(&mut app, 1);
    app.event(Event::Entropy(EntropyBytes::new([9u8; 32])));
    app.storage(StorageEvent::InputTapped {
        id: 1,
        x: 640,
        y: 400,
    });
    assert_eq!(app.new_inputs()[0].missed, 0);
    // Device A's sheet closes and B's press comes before B's is drawn.
    let mut app = on_screen(7);
    new_pointer(&mut app, 1);
    used_undrawn(&mut app, 2);
    let at = app.where_offered(Action::InputIgnore(1)).expect("Ignore");
    app.storage(StorageEvent::InputTapped {
        id: 1,
        x: at.0,
        y: at.1,
    });
    app.storage(StorageEvent::InputTapped {
        id: 2,
        x: 640,
        y: 400,
    });
    assert_eq!(app.new_inputs()[0].missed, 0);
}

// What was plugged in at start: one notice lists it (F202).

fn boot_usb(app: &mut Faraday, names: &[&str]) {
    app.storage(StorageEvent::BootUsb {
        names: names.iter().map(|n| n.to_string()).collect(),
    });
}

/// Everything the frame on screen says, as one line of text.
fn said(app: &mut Faraday) -> String {
    use faraday_draw::canvas::InkKind;
    let _ = app.frame();
    app.drawn_ink()
        .iter()
        .filter_map(|i| match &i.kind {
            InkKind::Text { text, .. } => Some(text.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn the_start_notice_lists_what_was_plugged_in_and_names_a_nameless_device() {
    let mut app = on_screen(7);
    boot_usb(&mut app, &["USB Receiver", "", "Backup Disk"]);
    let said = said(&mut app);
    assert!(
        said.contains("Plugged in: USB Receiver, USB device, Backup Disk. Only your keyboard and mouse should be plugged in. Unplug anything else."),
        "{said}"
    );
}

#[test]
fn nothing_plugged_in_at_start_shows_no_notice() {
    let mut app = on_screen(7);
    boot_usb(&mut app, &[]);
    assert!(!said(&mut app).contains("Plugged in"));
}

/// The boot notice's × as drawn: where a press lands on it.
fn notice_x(app: &mut Faraday) -> (u16, u16) {
    let _ = app.frame();
    app.where_offered(Action::BootNoticeClose)
        .expect("the notice's × is on screen")
}

fn tap(app: &mut Faraday, (x, y): (u16, u16)) {
    app.event(Event::Touch {
        x,
        y,
        phase: faraday_shell_api::TouchPhase::Down,
    });
    app.event(Event::Touch {
        x,
        y,
        phase: faraday_shell_api::TouchPhase::Up,
    });
}

#[test]
fn the_start_notice_goes_after_ten_seconds_whatever_else_is_pressed_or_toasted() {
    use faraday_core::Screen;
    let mut app = on_screen(7);
    app.event(Event::Tick { now_ms: 1000 });
    boot_usb(&mut app, &["USB Receiver"]);
    assert!(said(&mut app).contains("Plugged in"));
    // Another key, and a press elsewhere, do not touch it; a press reaches
    // the page under it.
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    app.press(Action::Nav(Screen::Settings));
    assert_eq!(app.screen, Screen::Settings);
    app.storage(StorageEvent::NewInput {
        id: 9,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: false,
        touch: false,
    });
    let both = said(&mut app);
    assert!(both.contains("Plugged in"), "{both}");
    assert!(both.contains("New device"), "the other toast shows too");
    app.event(Event::Tick { now_ms: 6000 });
    assert!(said(&mut app).contains("Plugged in"));
    app.event(Event::Tick { now_ms: 10_900 });
    assert!(
        said(&mut app).contains("Plugged in"),
        "10 s from the first frame"
    );
    app.event(Event::Tick { now_ms: 11_100 });
    assert!(!said(&mut app).contains("Plugged in"));
}

#[test]
fn pressing_the_start_notice_x_hides_it_for_good() {
    let mut app = on_screen(7);
    app.event(Event::Tick { now_ms: 1000 });
    boot_usb(&mut app, &["USB Receiver"]);
    let at = notice_x(&mut app);
    tap(&mut app, at);
    assert!(!said(&mut app).contains("Plugged in"));
    app.event(Event::Tick { now_ms: 3000 });
    assert!(!said(&mut app).contains("Plugged in"));
}

#[test]
fn escape_hides_the_start_notice_and_does_nothing_else() {
    use faraday_core::Screen;
    let mut app = on_screen(7);
    app.event(Event::Tick { now_ms: 1000 });
    app.press(Action::Nav(Screen::Settings));
    // No notice: Escape is as it was.
    let before = app.screen;
    boot_usb(&mut app, &["USB Receiver"]);
    let _ = app.frame();
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    assert!(!said(&mut app).contains("Plugged in"));
    assert_eq!(app.screen, before, "the screen did not go back");
}

#[test]
fn a_press_on_the_x_does_not_reach_what_is_under_it() {
    use faraday_core::Screen;
    let mut app = on_size(7, (480, 640), 160);
    app.event(Event::Tick { now_ms: 1000 });
    app.press(Action::Nav(Screen::Files));
    boot_usb(&mut app, &["USB Receiver"]);
    let at = notice_x(&mut app);
    let screen = app.screen;
    // The bottom bar's buttons stand under the notice's end.
    let (x, y) = (i32::from(at.0), i32::from(at.1));
    let under = [
        Screen::Home,
        Screen::WalletsStart,
        Screen::Files,
        Screen::Vaults,
        Screen::Catalog,
    ]
    .into_iter()
    .any(|s| {
        app.hit_box(Action::Nav(s)).is_some_and(|(bx, by, bw, bh)| {
            let (bx, by, bw, bh) = (i32::from(bx), i32::from(by), i32::from(bw), i32::from(bh));
            (bx..bx + bw).contains(&x) && (by..by + bh).contains(&y)
        })
    });
    assert!(under, "a navigation button is under the ×");
    tap(&mut app, at);
    assert_eq!(app.screen, screen);
    assert!(!said(&mut app).contains("Plugged in"));
}

#[test]
fn a_press_before_the_x_is_drawn_counts_nothing() {
    let mut app = on_screen(7);
    app.event(Event::Tick { now_ms: 1000 });
    boot_usb(&mut app, &["USB Receiver"]);
    app.press(Action::BootNoticeClose);
    assert!(said(&mut app).contains("Plugged in"));
}

#[test]
fn a_held_back_device_cannot_dismiss_the_start_notice() {
    let mut app = on_screen(7);
    app.event(Event::Tick { now_ms: 1000 });
    boot_usb(&mut app, &["USB Receiver"]);
    let at = notice_x(&mut app);
    app.storage(StorageEvent::NewInput {
        id: 5,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: true,
        touch: false,
    });
    app.storage(StorageEvent::InputUsed { id: 5 });
    // Its Escape, and a press of its own on the ×.
    app.storage(StorageEvent::InputTyped {
        id: 5,
        ch: '\u{1b}',
    });
    tap_held(&mut app, 5, at);
    assert!(said(&mut app).contains("Plugged in"));
}

/// Add a key with "a" typed, a trusted keyboard, and a mouse's sheet over it.
fn sheet_over_a_form() -> Faraday {
    use faraday_shell_api::Key;
    let mut app = on_screen(7);
    app.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: true,
    });
    app.press(Action::Entry(None));
    app.event(Event::Key(Key::Char('a')));
    assert_eq!(&*testkit::entry(&app).typed, "a");
    announce(&mut app, 2, "USB Mouse", (false, true, false));
    app.storage(StorageEvent::InputUsed { id: 2 });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app
}

#[test]
fn keys_typed_while_a_devices_sheet_is_up_do_not_reach_the_form_under_it() {
    use faraday_shell_api::Key;
    let mut app = sheet_over_a_form();
    let screen = app.screen;
    for key in [Key::Char('b'), Key::Backspace, Key::Escape] {
        app.event(Event::Key(key));
        assert_eq!(&*testkit::entry(&app).typed, "a");
        assert_eq!(app.screen, screen);
        assert_eq!(app.sheet, Some(Sheet::NewInput));
    }
}

#[test]
fn over_a_form_the_keyboard_can_tab_to_ignore_and_press_it() {
    use faraday_shell_api::Key;
    let mut app = sheet_over_a_form();
    app.event(Event::Key(Key::Tab));
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.poll_input_decision(), Some((2, false)));
    assert_eq!(&*testkit::entry(&app).typed, "a");
}

const MIN: u64 = 60_000;

fn tick_to(app: &mut Faraday, ms: u64) {
    app.event(Event::Tick { now_ms: ms });
}

/// An app with a wallet loaded and its randomness given, on a screen.
fn session_with_wallet(seed: u8) -> Faraday {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = on_screen(seed);
    app.storage(StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app
}

/// The app that follows an idle lock, with the lock notice up and no
/// trusted keyboard or pointer.
fn after_lock() -> Faraday {
    let mut app = session_with_wallet(7);
    tick_to(&mut app, 0);
    tick_to(&mut app, 30 * MIN);
    assert!(app.restart_requested());
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    let mut next = on_screen(7);
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    next.storage(StorageEvent::InputTrust {
        pointer: false,
        keyboard: false,
    });
    let _ = next.frame();
    assert_eq!(next.sheet, Some(Sheet::Locked));
    next
}

fn used_pointer(app: &mut Faraday, id: u32) {
    announce(app, id, "USB Mouse", (false, true, false));
    app.storage(StorageEvent::InputUsed { id });
    let _ = app.frame();
}

#[test]
fn a_pointer_used_under_the_lock_notice_gets_its_sheet_and_presses_beside_do_nothing() {
    let mut app = after_lock();
    let continue_at = app.where_offered(Action::Cancel);
    used_pointer(&mut app, 3);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    let aside = beside(&app, 3);
    tap_held(&mut app, 3, aside);
    if let Some(at) = continue_at {
        tap_held(&mut app, 3, at);
    }
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert_eq!(app.poll_input_decision(), None);
    assert!(!app.holds_secret());
}

#[test]
fn a_pointer_let_in_under_the_lock_notice_brings_the_notice_back() {
    let mut app = after_lock();
    used_pointer(&mut app, 3);
    get_in(&mut app, 3);
    assert_eq!(app.poll_input_decision(), Some((3, true)));
    assert_eq!(app.sheet, Some(Sheet::Locked));
}

#[test]
fn a_keyboard_used_under_the_lock_notice_shows_its_code_and_its_enter_does_nothing() {
    let mut app = after_lock();
    announce(&mut app, 4, "USB Keyboard", (true, false, false));
    app.storage(StorageEvent::InputTyped { id: 4, ch: 'q' });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert!(app.new_inputs()[0].code.is_some());
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    assert_eq!(app.poll_input_decision(), None);
    let code = code_of(&app);
    type_on(&mut app, 4, &code);
    assert_eq!(app.poll_input_decision(), Some((4, true)));
    assert_eq!(app.sheet, Some(Sheet::Locked));
}

#[test]
fn a_device_used_during_the_idle_warning_leaves_it_up_and_the_lock_comes_on_time() {
    let mut app = session_with_wallet(7);
    tick_to(&mut app, 0);
    tick_to(&mut app, 26 * MIN);
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    used_pointer(&mut app, 3);
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    tick_to(&mut app, 29 * MIN);
    assert!(!app.restart_requested());
    tick_to(&mut app, 30 * MIN);
    assert!(app.restart_requested());
}
