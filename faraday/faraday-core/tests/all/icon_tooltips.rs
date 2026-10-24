//! Plain tooltips on icon-only controls (`docs/DESIGN.md` §4.12, Tooltip):
//! the pointer resting on one, or the keyboard on it, names it after a
//! short delay; a finger never does; Escape and leaving take it off.

use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

fn app() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    let now = app.now_ms() + 1000;
    app.event(Event::Tick { now_ms: now });
    let _ = app.frame();
    app
}

fn tick(app: &mut Faraday, ms: u64) {
    let now = app.now_ms() + ms;
    app.event(Event::Tick { now_ms: now });
}

/// How many times `label` is drawn: the sidebar and the page may draw the
/// same word, so a tooltip is a count that goes up by one.
fn count(app: &mut Faraday, label: &str) -> usize {
    app.drawn_texts().iter().filter(|t| *t == label).count()
}

/// The labels a plain tooltip may carry.
fn labels() -> [&'static str; 7] {
    [
        EN.tip_back,
        EN.tip_learn,
        EN.tip_more,
        EN.tip_lock,
        EN.tip_keyboard,
        EN.tip_power_off,
        EN.tip_remove,
    ]
}

fn all(app: &mut Faraday) -> usize {
    labels().iter().map(|l| count(app, l)).sum()
}

#[test]
fn resting_the_pointer_on_the_question_mark_names_it_and_leaving_hides_it() {
    let mut app = app();
    let (x, y) = app.where_offered(Action::Learn).expect("the ? is offered");
    let before = count(&mut app, EN.tip_learn);
    app.event(Event::Hover { x, y });
    tick(&mut app, 300);
    assert_eq!(
        count(&mut app, EN.tip_learn),
        before,
        "not before the delay"
    );
    tick(&mut app, 300);
    assert_eq!(count(&mut app, EN.tip_learn), before + 1, "after it");
    app.event(Event::Hover { x, y: y + 300 });
    assert_eq!(count(&mut app, EN.tip_learn), before, "the pointer gone");
    app.event(Event::Hover { x, y });
    tick(&mut app, 600);
    assert_eq!(count(&mut app, EN.tip_learn), before + 1);
    app.event(Event::HoverEnd);
    assert_eq!(count(&mut app, EN.tip_learn), before);
}

#[test]
fn the_keyboard_on_an_icon_button_names_it_and_escape_hides_it() {
    let mut app = app();
    let before = all(&mut app);
    let mut shown = false;
    for _ in 0..200 {
        app.event(Event::Key(Key::Tab));
        tick(&mut app, 600);
        if all(&mut app) > before {
            shown = true;
            break;
        }
    }
    assert!(shown, "Tab reaches an icon button and its label shows");
    assert_eq!(all(&mut app), before + 1);
    app.event(Event::Key(Key::Escape));
    assert_eq!(all(&mut app), before, "Escape takes it off");
    assert_eq!(app.screen, Screen::Create, "and does nothing else");
}

#[test]
fn a_finger_on_an_icon_button_shows_no_tooltip_and_the_lift_presses_it() {
    let mut app = app();
    let (x, y) = app.where_offered(Action::Learn).expect("the ? is offered");
    let before = all(&mut app);
    let page = app.drawn_texts();
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    tick(&mut app, 700);
    assert_eq!(all(&mut app), before, "held, still no tooltip");
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    tick(&mut app, 700);
    assert_ne!(app.drawn_texts(), page, "the ? did what it does");
}

#[test]
fn a_press_with_the_pointer_hides_the_tooltip() {
    let mut app = app();
    let (x, y) = app.where_offered(Action::Learn).expect("the ? is offered");
    let before = count(&mut app, EN.tip_learn);
    app.event(Event::Hover { x, y });
    tick(&mut app, 600);
    assert_eq!(count(&mut app, EN.tip_learn), before + 1);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    assert_eq!(
        count(&mut app, EN.tip_learn),
        before,
        "a press takes it off"
    );
}

#[test]
fn the_boards_cross_and_menu_are_named_when_the_pointer_rests_on_them() {
    use crate::backup_board::{on_board, three_on_an_empty_board};
    use faraday_core::board::{BoardPress as BP, Chip, Thing};
    let mut app = three_on_an_empty_board();
    app.press(Action::Board(BP::AddPlace));
    for (press, label) in [
        (BP::RemovePlace(1), EN.tip_remove),
        (BP::Menu(Chip::List(Thing::Seed(0))), EN.tip_more),
    ] {
        let (x, y) = on_board(&mut app, press);
        let before = count(&mut app, label);
        app.event(Event::Hover { x, y });
        assert_eq!(count(&mut app, label), before, "{press:?}: not at once");
        tick(&mut app, 600);
        assert_eq!(
            count(&mut app, label),
            before + 1,
            "{press:?}: after the delay"
        );
        app.event(Event::HoverEnd);
        assert_eq!(
            count(&mut app, label),
            before,
            "{press:?}: gone with the pointer"
        );
    }
}

#[test]
fn a_finger_held_on_the_boards_cross_shows_no_tooltip() {
    use crate::backup_board::{on_board, three_on_an_empty_board};
    use faraday_core::board::BoardPress as BP;
    let mut app = three_on_an_empty_board();
    app.press(Action::Board(BP::AddPlace));
    let (x, y) = on_board(&mut app, BP::RemovePlace(1));
    let before = count(&mut app, EN.tip_remove);
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    tick(&mut app, 900);
    assert_eq!(count(&mut app, EN.tip_remove), before);
}
