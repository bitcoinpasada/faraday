//! A wide window's page area (`docs/LAYOUT.md` §14, §15): what stands at
//! the right (the ?, Scan) stands 24 dp from the window's edge, and a
//! reading page sits centred in the room beside the navigation.

use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const MARGIN: f32 = 24.0;
const SIDEBAR: f32 = 240.0;

fn shown(width: u16, height: u16) -> Faraday {
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
    app
}

#[test]
fn the_question_mark_and_scan_stand_24_dp_from_the_windows_edge() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (2560, 1440)] {
        let mut app = shown(w, h);
        app.press(Action::Nav(Screen::Home));
        let _ = app.frame();
        // The ? is a 28 dp circle in a 40 dp target.
        let q = app.hit_box(Action::Learn).expect("the ? on Home");
        let right = f32::from(q.0) + f32::from(q.2) / 2.0 + 14.0;
        assert!(
            (right - (f32::from(w) - MARGIN)).abs() <= 1.0,
            "{w}: the ? ends at {right}"
        );
        let scan = app.hit_box(Action::Scan).expect("Scan on Home");
        assert!(
            (f32::from(scan.0 + scan.2) - (f32::from(w) - MARGIN)).abs() <= 1.0,
            "{w}: Scan {scan:?}"
        );
        assert!(
            (f32::from(scan.1 + scan.3) - (f32::from(h) - MARGIN)).abs() <= 1.0,
            "{w}: Scan {scan:?}"
        );
    }
}

#[test]
fn set_the_time_sits_centred_in_the_room_beside_the_navigation_on_a_big_window() {
    for (w, h) in [(1920u16, 1080u16), (2560, 1440)] {
        let mut app = shown(w, h);
        app.press(Action::Nav(Screen::Settings));
        app.press(Action::OpenSetTime);
        let _ = app.frame();
        let title = app
            .drawn_text_boxes()
            .into_iter()
            .filter(|(t, _)| t == "Set the time")
            .map(|b| b.1)
            .min_by_key(|r| r.1)
            .expect("the page's title");
        // Its column runs 520 dp from the title's left edge.
        let middle = f32::from(i16::try_from(title.0).expect("on screen")) + 520.0 / 2.0;
        let room = SIDEBAR + (f32::from(w) - SIDEBAR) / 2.0;
        assert!(
            (middle - room).abs() <= 2.0,
            "{w}: the column's middle {middle}, the room's {room}"
        );
    }
}
