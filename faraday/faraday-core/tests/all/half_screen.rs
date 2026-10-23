//! A window sharing the screen is laid out at the screen's own density,
//! whatever its size, and takes the layout its width in dp calls for: one
//! column under 1200 dp, the wide layout from there (`docs/LAYOUT.md` §7,
//! F101). Nothing is shrunk or enlarged to fit.

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
fn a_half_screen_window_is_drawn_at_the_compositors_scale() {
    // A half-screen window of a 3000 × 2000 panel at the compositor's
    // scale 2 is 750 dp wide: one column, two pixels a unit. A window of
    // the same shape at scale 1.5 is 1200 dp wide: the wide layout, one
    // and a half pixels a unit.
    let at_2 = window(1500, 2000, 320);
    let at_1_5 = window(1800, 2400, 240);
    assert!(at_2.is_compact());
    assert!(!at_1_5.is_compact());
    assert!((pixels_per_unit(&at_2, 1500) - 2.0).abs() < 1e-3);
    assert!((pixels_per_unit(&at_1_5, 1800) - 1.5).abs() < 1e-3);
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
    assert!((pixels_per_unit(&app, 2880) - 2.0).abs() < 1e-3);
}

#[test]
fn a_small_laptop_screen_keeps_the_wide_layout() {
    let app = window(1366, 768, 160);
    assert!(!app.is_compact());
    assert!((pixels_per_unit(&app, 1366) - 1.0).abs() < 1e-3);
}
