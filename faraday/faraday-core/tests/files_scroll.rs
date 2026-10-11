//! Files on a desktop: From the stick and For the stick scroll each on
//! its own, each under the pointer, with its own scrollbar.

use faraday_core::{Action, Faraday, Screen, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// Files with twenty files each way, on a 1366 by 768 desktop.
fn files() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let many = |stem: &str| -> Vec<(String, Vec<u8>)> {
        (0..20)
            .map(|i| (format!("{stem}-{i}.txt"), b"a note".to_vec()))
            .collect()
    };
    app.storage(StorageEvent::Restored {
        inbox: many("in"),
        outbox: many("out"),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app
}

/// Turns the wheel at (x, y) and lets the glide land.
fn wheel(app: &mut Faraday, x: u16, y: u16) {
    app.event(Event::Wheel { x, y, dy: 3000 });
    app.settle();
    let _ = app.frame();
}

#[test]
fn for_the_stick_scrolls_alone() {
    let mut app = files();
    wheel(&mut app, 1100, 600);
    assert!(app.files_out_offset > 0.0, "For the stick did not move");
    assert_eq!(app.list_offset, 0.0, "From the stick moved with it");
}

#[test]
fn from_the_stick_scrolls_alone() {
    let mut app = files();
    wheel(&mut app, 500, 600);
    assert!(app.list_offset > 0.0, "From the stick did not move");
    assert_eq!(app.files_out_offset, 0.0, "For the stick moved with it");
}
