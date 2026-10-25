//! Dialogs (`docs/DESIGN.md` §4.13, Dialog): the one Material 3 dialog
//! every question over a page is drawn with. Its actions are pressed by
//! a finger or the pointer on every layout, and the keyboard reaches
//! them: Tab goes to the first, Enter presses it.

use faraday_core::{Action, Faraday, Sheet};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

fn on(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::ReduceMotion(true));
    app
}

/// The power dialog asked for, a frame drawn.
fn power(width: u16, height: u16) -> Faraday {
    let mut app = on(width, height);
    app.press(Action::PowerAsk);
    assert_eq!(app.sheet, Some(Sheet::Power));
    let _ = app.frame();
    app
}

/// A finger down and up where `action` is drawn.
fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

#[test]
fn a_dialogs_cancel_takes_a_press_on_every_layout() {
    for (w, h) in [(1280, 800), (1920, 1080), (480, 640)] {
        let mut app = power(w, h);
        assert!(
            app.offers(Action::PowerOff),
            "{w}x{h}: Power off is offered"
        );
        tap(&mut app, Action::Cancel);
        assert_eq!(app.sheet, None, "{w}x{h}: Cancel closed it");
    }
}

#[test]
fn the_keyboard_reaches_a_dialogs_first_action_and_enter_presses_it() {
    for (w, h) in [(1280, 800), (480, 640)] {
        let mut app = power(w, h);
        app.event(Event::Key(Key::Tab));
        let _ = app.frame();
        assert_eq!(
            app.sheet,
            Some(Sheet::Power),
            "{w}x{h}: Tab presses nothing"
        );
        app.event(Event::Key(Key::Enter));
        let _ = app.frame();
        assert_eq!(app.sheet, None, "{w}x{h}: Enter on Cancel closed it");
    }
}

#[test]
fn nothing_under_a_dialog_takes_a_press() {
    let mut app = power(1280, 800);
    let _ = app.frame();
    assert!(
        !app.offers(Action::PowerAsk),
        "the page's power button is out of reach"
    );
}
