//! A window sharing the screen lays out the same on any screen of
//! compositor scale 1.5 or more: a half-screen window takes the wide
//! layout fitted to it, whatever the scale. A window too narrow for the
//! wide layout at three quarters of the screen's density (counted up to
//! scale 1.5) takes the one-column layout at that density rather than
//! the wide layout shrunk further (F78).

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
fn a_half_screen_window_lays_out_the_same_at_scale_2_and_at_scale_1_5() {
    // A half-screen window of a 3000 × 2000 panel at the compositor's
    // scale 2, and a window of the same shape at scale 1.5.
    let at_2 = window(1500, 2000, 320);
    let at_1_5 = window(1800, 2400, 240);
    assert!(!at_2.is_compact());
    assert!(!at_1_5.is_compact());
    let (a, b) = (at_2.size(), at_1_5.size());
    assert!(
        (a.0 - b.0).abs() < 0.5 && (a.1 - b.1).abs() < 0.5,
        "{a:?} {b:?}"
    );
}

#[test]
fn a_third_of_a_scaled_laptop_screen_draws_one_column_at_the_screens_density() {
    // A 3000 × 2000 panel at the compositor's scale 2, split in three.
    let app = window(1000, 2000, 320);
    assert!(app.is_compact());
    assert!((pixels_per_unit(&app, 1000) - 2.0).abs() < 1e-3);
}

#[test]
fn a_narrow_window_on_a_scale_1_5_screen_draws_one_column() {
    let app = window(1200, 1400, 240);
    assert!(app.is_compact());
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
