//! Files on a desktop and beside the rail: From the stick and For the
//! stick scroll each on its own, each under the pointer, with its own
//! scrollbar (`docs/LAYOUT.md` §17).

use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// Files with twenty files each way, on a 1366 by 768 desktop.
fn files() -> Faraday {
    files_on(1366, 768, 160)
}

/// Files with twenty files each way, on a `width` × `height` display at
/// `dpi`.
fn files_on(width: u16, height: u16, dpi: u16) -> Faraday {
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
    wheel(&mut app, 800, 600);
    assert!(app.files_out_offset() > 0.0, "For the stick did not move");
    assert_eq!(app.list_offset, 0.0, "From the stick moved with it");
}

#[test]
fn from_the_stick_scrolls_alone() {
    let mut app = files();
    wheel(&mut app, 500, 600);
    assert!(app.list_offset > 0.0, "From the stick did not move");
    assert_eq!(app.files_out_offset(), 0.0, "For the stick moved with it");
}

/// The stick's 7-inch panel in landscape, 964 × 565 dp beside the rail:
/// the two lists stand side by side.
fn seven_inch() -> Faraday {
    let app = files_on(1024, 600, 170);
    assert_eq!(app.nav(), faraday_core::nav::Nav::Rail);
    app
}

/// Where the text `want` is drawn, its first box (x, y, w, h), pixels.
fn drawn(app: &mut Faraday, want: &str) -> Option<(i32, i32, i32, i32)> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == want)
        .map(|(_, b)| b)
}

#[test]
fn beside_the_rail_for_the_stick_scrolls_alone() {
    let mut app = seven_inch();
    let first = drawn(&mut app, "in-0.txt").expect("From the stick's first file");
    wheel(&mut app, 800, 500);
    assert!(app.files_out_offset() > 0.0, "For the stick did not move");
    assert_eq!(app.list_offset, 0.0, "the page moved with it");
    assert_eq!(
        drawn(&mut app, "in-0.txt"),
        Some(first),
        "From the stick moved with it"
    );
    assert!(
        drawn(&mut app, "out-0.txt").is_none(),
        "For the stick's first file is scrolled out of its pane"
    );
}

#[test]
fn beside_the_rail_from_the_stick_scrolls_alone() {
    let mut app = seven_inch();
    wheel(&mut app, 200, 500);
    assert!(app.list_offset > 0.0, "From the stick did not move");
    assert_eq!(app.files_out_offset(), 0.0, "For the stick moved with it");
    let (_, y, ..) = drawn(&mut app, "out-0.txt").expect("For the stick's first file");
    assert!(
        y > 0,
        "For the stick's first file stands in its pane, at {y}"
    );
}
