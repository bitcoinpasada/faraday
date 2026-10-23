//! The window class a display gets (`docs/LAYOUT.md` §2): read from its
//! size at its own density, so the same panel or window is the same
//! class wherever it is shown.

use faraday_core::{Faraday, testkit};
use faraday_draw::geom::WindowClass;
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn shown_on(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = testkit::started();
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
    app
}

/// The Pi's 2.8-inch panel is compact and short; a 7-inch panel is
/// expanded and low; the desktop window's default is large; a full
/// 1920 × 1080 monitor is extra-large; a half-screen window is medium.
#[test]
fn each_screen_gets_its_window_class() {
    let pi = shown_on(480, 640, 286).window();
    assert_eq!(pi.class(), WindowClass::Compact);
    assert!(pi.short());
    let seven_inch = shown_on(800, 480, 133).window();
    assert_eq!(seven_inch.class(), WindowClass::Expanded);
    assert!(seven_inch.low() && !seven_inch.short());
    let desktop = shown_on(1280, 800, 160).window();
    assert_eq!(desktop.class(), WindowClass::Large);
    assert!(!desktop.low());
    assert_eq!(
        shown_on(1920, 1080, 160).window().class(),
        WindowClass::ExtraLarge
    );
    assert_eq!(
        shown_on(760, 1000, 160).window().class(),
        WindowClass::Medium
    );
}

/// A scale-2 screen counts its pixels at half: a 1440 × 900 pixel
/// window there is a 720 dp, medium window.
#[test]
fn a_scale_two_window_is_classed_by_its_dp_not_its_pixels() {
    let w = shown_on(1440, 900, 320).window();
    assert_eq!(w.class(), WindowClass::Medium);
}
