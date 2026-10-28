//! The window class a display gets (`docs/LAYOUT.md` §2): read from its
//! size at its own density, so the same panel or window is the same
//! class wherever it is shown.

use faraday_core::{Action, Faraday, testkit};
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

/// That 720 dp window takes the one-column layout, at the screen's two
/// pixels a unit (`docs/LAYOUT.md` §7, F101).
#[test]
fn a_scale_two_window_of_1440_by_900_pixels_lays_out_one_column() {
    let app = shown_on(1440, 900, 320);
    assert!(app.is_compact());
    let (w, h) = app.size();
    assert!(
        (w - 720.0).abs() < 0.5 && (h - 450.0).abs() < 0.5,
        "{w} {h}"
    );
}

/// A 1920 × 1080 monitor at 160 dpi is laid out at 1920 × 1080 dp, one
/// pixel a unit, with the sidebar: more room, not the 1280 × 800 page
/// made larger. Its text is as tall in pixels as on a 1280 × 800 window.
#[test]
fn a_full_monitor_lays_out_wide_at_its_own_size() {
    let mut app = shown_on(1920, 1080, 160);
    assert!(!app.is_compact());
    assert_eq!(app.size(), (1920.0, 1080.0));
    let tallest = |app: &mut Faraday| {
        app.drawn_text_boxes()
            .into_iter()
            .map(|(_, b)| b.3)
            .max()
            .expect("text drawn")
    };
    let big = tallest(&mut app);
    let small = tallest(&mut shown_on(1280, 800, 160));
    assert_eq!(big, small);
}

/// The Pi's 2.8-inch panel is one column at its 286 dpi: 268 × 358 dp.
#[test]
fn the_pi_panel_lays_out_one_column_at_its_density() {
    let app = shown_on(480, 640, 286);
    assert!(app.is_compact());
    let (w, h) = app.size();
    assert!(
        (w - 268.5).abs() < 0.5 && (h - 358.0).abs() < 0.5,
        "{w} {h}"
    );
}

/// The person's Display scale still enlarges: at 150 % a 1920 × 1080
/// window draws one and a half pixels a unit, and its 1280 dp take the
/// wide layout; at 200 % its 960 dp take one column.
#[test]
fn the_display_scale_setting_enlarges() {
    let mut app = shown_on(1920, 1080, 160);
    app.press(Action::Scale(150));
    assert!(!app.is_compact());
    assert!((app.size().0 - 1280.0).abs() < 0.5);
    app.press(Action::Scale(200));
    assert!(app.is_compact());
    assert_eq!(app.window().class(), WindowClass::Expanded);
    assert!((app.size().0 - 960.0).abs() < 0.5);
}

/// Larger text lays the desktop app's 1280 × 800 window out as a smaller
/// one would be (`docs/LAYOUT.md` §2): at 1.4 times the text it counts as
/// 914 × 571 dp, an expanded and low window, and takes the navigation
/// rail and one-column pages in place of the expanded rail; Standard
/// gives the expanded rail back.
#[test]
fn larger_text_at_1280_lays_out_a_class_down_with_the_rail() {
    use faraday_core::nav::Nav;
    use faraday_core::ui::TextSize;
    let mut app = shown_on(1280, 800, 160);
    assert_eq!(app.nav(), Nav::ExpandedRail);
    app.press(Action::TextSize(TextSize::Larger));
    assert_eq!(app.nav(), Nav::Rail);
    assert!(app.is_compact());
    app.press(Action::TextSize(TextSize::Standard));
    assert_eq!(app.nav(), Nav::ExpandedRail);
}
