//! A window sharing the screen with another keeps its text at the size
//! the screen's density asks for: it takes the one-column layout rather
//! than the wide layout shrunk to fit.

use faraday_core::Faraday;
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn window(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// Pixels per design unit.
fn pixels_per_unit(app: &Faraday, width: u16) -> f32 {
    f32::from(width) / app.size().0
}

#[test]
fn half_of_a_scaled_laptop_screen_draws_one_column_at_the_screens_density() {
    // A 2880 × 1800 panel at the compositor's scale 2, split in two.
    let app = window(1440, 1800, 320);
    assert!(app.is_compact());
    assert!((pixels_per_unit(&app, 1440) - 2.0).abs() < 1e-3);
}

#[test]
fn the_whole_screen_keeps_the_wide_layout() {
    let app = window(2880, 1800, 320);
    assert!(!app.is_compact());
    assert!(pixels_per_unit(&app, 2880) >= 2.0);
}

#[test]
fn a_small_laptop_screen_keeps_the_wide_layout() {
    let app = window(1366, 768, 160);
    assert!(!app.is_compact());
}
