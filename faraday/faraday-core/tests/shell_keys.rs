//! The shell's own keys, pinned in small text at the top of the window:
//! the desktop sets them (F2 to F5); an app no shell gave keys to, the
//! device's or an embedding page's, draws no such line.

use faraday_core::Faraday;
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const KEYS: &str = "F2 boot stick · F3 stick 2";

fn shown(keys: Option<&str>) -> Faraday {
    let mut app = Faraday::new();
    app.shell_keys = keys.map(str::to_string);
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
    let _ = app.frame();
    app
}

#[test]
fn keys_given_by_the_shell_are_pinned_at_the_top() {
    let mut app = shown(Some(KEYS));
    assert!(app.drawn_texts().iter().any(|t| t == KEYS));
}

#[test]
fn an_app_given_no_keys_draws_none() {
    let mut app = shown(None);
    assert!(!app.drawn_texts().iter().any(|t| t.starts_with("F2")));
}

/// Over a sheet too: the desktop opens on the boot stick's sheet, whose
/// way on is a key.
#[test]
fn the_keys_stay_over_a_sheet() {
    let mut app = shown(Some(KEYS));
    app.press(faraday_core::Action::Learn);
    assert!(app.sheet.is_some());
    assert!(app.drawn_texts().iter().any(|t| t == KEYS));
}
