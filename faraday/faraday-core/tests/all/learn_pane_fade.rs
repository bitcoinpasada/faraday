//! Learn's supporting pane on an extra-large window fades in and out as
//! a change of screen does (`docs/MOTION.md` §3.5): opening, closing,
//! the close at another screen when unpinned, a change of its tab; one
//! fade, not two, when a pinned pane follows a change of screen; none
//! with reduce motion.

use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// A started app on a 1920 x 1080 window, on Settings, drawn, with the
/// fades counted from here.
fn wide() -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: 1920,
        height: 1080,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    let _ = app.take_fade_counts();
    app
}

/// Draws what the last press changed and says how many changes faded
/// in and how many were drawn at once for reduce motion.
fn counts(app: &mut Faraday) -> (u32, u32) {
    let _ = app.frame();
    let c = app.take_fade_counts();
    (c.started, c.reduced)
}

#[test]
fn opening_the_pane_and_closing_it_each_fade() {
    let mut app = wide();
    app.press(Action::Learn);
    assert_eq!(counts(&mut app), (1, 0), "opening fades");
    assert!(app.offers(Action::LearnClose), "the pane is open");
    app.press(Action::LearnClose);
    assert_eq!(counts(&mut app), (1, 0), "closing fades");
    assert!(!app.offers(Action::LearnClose));
}

#[test]
fn a_change_of_the_panes_tab_fades() {
    let mut app = wide();
    app.press(Action::Learn);
    let _ = counts(&mut app);
    app.press(Action::LearnPage(1));
    assert_eq!(counts(&mut app), (1, 0));
}

#[test]
fn an_unpinned_pane_put_away_at_another_screen_fades_once_with_the_screen() {
    let mut app = wide();
    app.press(Action::Learn);
    let _ = counts(&mut app);
    app.press(Action::Nav(Screen::Home));
    assert_eq!(counts(&mut app), (1, 0));
    assert!(!app.offers(Action::LearnClose));
}

#[test]
fn a_pinned_pane_following_to_another_screen_fades_once() {
    let mut app = wide();
    app.press(Action::Learn);
    app.press(Action::LearnPin);
    let _ = counts(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    assert_eq!(counts(&mut app), (1, 0), "one fade for the screen and pane");
    assert!(app.offers(Action::LearnClose), "the pane followed");
}

#[test]
fn with_reduce_motion_the_pane_is_there_and_gone_at_once() {
    let mut app = wide();
    app.press(Action::ReduceMotion(true));
    let _ = counts(&mut app);
    app.press(Action::Learn);
    assert_eq!(counts(&mut app), (0, 1));
    app.press(Action::LearnClose);
    assert_eq!(counts(&mut app), (0, 1));
}
