//! The desktop window moves by its top strip: the app says which points
//! of the strip are empty room, where a press and a drag move the window
//! (`docs/DESIGN.md` §5, the top strip).

use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

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
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

#[test]
fn empty_strip_room_moves_the_window_and_a_control_does_not() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080)] {
        let app = on(w, h);
        let (w, h) = (i32::from(w), i32::from(h));
        // Empty room in the strip, in the middle and at the top of the
        // sidebar.
        assert!(app.is_strip_room(w / 2, 6), "{w}x{h}: middle of the strip");
        assert!(
            app.is_strip_room(8, 6),
            "{w}x{h}: above the sidebar's logo row"
        );
        // The ? at the top right is a control, and the page below the
        // strip is not strip.
        let (qx, qy) = app.where_offered(Action::Learn).expect("? offered");
        assert!(
            !app.is_strip_room(i32::from(qx), i32::from(qy)),
            "{w}x{h}: ?"
        );
        assert!(!app.is_strip_room(w / 2, h / 2), "{w}x{h}: the page");
        assert!(!app.is_strip_room(-1, 6) && !app.is_strip_room(w / 2, -1));
    }
}
