//! Home's Scan tile: open to anyone, for a person coming from OpenSigner
//! who expects to start by scanning, and refused while a stick is
//! attached, as any way to load a key already is.

use faraday_core::{Action, Faraday, StickInfo, StorageEvent};
use osk_shell_api::{App, DisplayInfo};

fn shown() -> Faraday {
    let mut app = Faraday::new();
    app.event(osk_shell_api::Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: osk_shell_api::SecureHardware::None,
        boot: osk_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

#[test]
fn home_offers_scan_with_no_stick_attached() {
    let app = shown();
    assert!(app.offers(Action::Scan));
}

#[test]
fn home_s_scan_tile_is_refused_with_a_stick_attached() {
    let mut app = shown();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "a".into(),
        label: "STICK".into(),
        boot: false,
        files: Vec::new(),
    }]));
    let _ = app.frame();
    assert!(!app.offers(Action::Scan));
}
