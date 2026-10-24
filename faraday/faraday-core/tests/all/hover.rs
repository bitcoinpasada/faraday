//! The pointer over something that can be pressed (`docs/MOTION.md`
//! §3.5): it shows so, shows nothing once the pointer leaves, and presses
//! nothing.

use faraday_core::ui::Preset;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn settings() -> Faraday {
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
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    app
}

fn pixels(app: &mut Faraday) -> Vec<u8> {
    while app.poll_command().is_some() {}
    app.frame().rgba.to_vec()
}

#[test]
fn a_button_under_the_pointer_shows_it_and_stops_when_the_pointer_leaves() {
    let mut app = settings();
    let theme = app.theme();
    let before = pixels(&mut app);
    let (x, y) = app.where_offered(Action::Theme(Preset::Light)).unwrap();
    app.event(Event::Hover { x, y });
    let over = pixels(&mut app);
    assert!(
        over != before,
        "the button under the pointer looks different"
    );
    app.event(Event::HoverEnd);
    assert!(
        pixels(&mut app) == before,
        "and as it was once the pointer leaves"
    );
    assert_eq!(app.theme(), theme, "hovering pressed nothing");
}

#[test]
fn a_sidebar_item_under_the_pointer_shows_it() {
    let mut app = settings();
    let before = pixels(&mut app);
    let (x, y) = app.where_offered(Action::Nav(Screen::Files)).unwrap();
    app.event(Event::Hover { x, y });
    assert!(pixels(&mut app) != before);
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn moving_over_nothing_pressable_draws_no_frame() {
    let mut app = settings();
    while app.poll_command().is_some() {}
    // Clear of the scrolled page's right edge, where its scrollbar is.
    app.event(Event::Hover { x: 1200, y: 790 });
    app.event(Event::Hover { x: 1195, y: 785 });
    assert!(
        std::iter::from_fn(|| app.poll_command()).next().is_none(),
        "no frame is asked for"
    );
}

/// Tab puts the keyboard on a control, which is ringed, Tab again
/// moves it on, and Enter presses the one it is on (`docs/DESIGN.md` §4.15).
#[test]
fn tab_rings_a_control_and_enter_presses_it() {
    use faraday_shell_api::Key;
    let mut app = settings();
    app.press(Action::Nav(Screen::Home));
    // Past the fade from Settings.
    let _ = pixels(&mut app);
    for now_ms in [1_000, 2_000] {
        app.event(Event::Tick { now_ms });
        let _ = pixels(&mut app);
    }
    let before = pixels(&mut app);
    app.event(Event::Key(Key::Tab));
    let ringed = pixels(&mut app);
    assert_ne!(before, ringed, "the control the keyboard is on is ringed");
    // The first is Home, the screen on: the next one Enter opens.
    app.event(Event::Key(Key::Tab));
    let _ = pixels(&mut app);
    app.event(Event::Key(Key::Enter));
    assert_ne!(app.screen, Screen::Home, "Enter pressed it");
}
