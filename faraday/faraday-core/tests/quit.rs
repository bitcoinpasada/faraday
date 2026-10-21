//! The desktop app quits and the stick powers off: the Settings button
//! and the Power sheet say which, and the desktop has no power-off timer.

use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

use common::{has, offered};

mod common;

fn settings(online: bool, width: u16, height: u16) -> Faraday {
    let mut app = testkit::started();
    app.online = online;
    app.shell_sticks = online;
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
    app.press(Action::Nav(Screen::Settings));
    app
}

#[test]
fn the_desktop_settings_offer_quit_and_no_power_off_timer() {
    for (w, h) in [(1366, 2400), (480, 640)] {
        let mut app = settings(true, w, h);
        assert!(has(&mut app, "Quit"), "{w}x{h}");
        assert!(!has(&mut app, "Power off"), "{w}x{h}");
        assert!(!has(&mut app, "Power off after"), "{w}x{h}");
        assert!(has(&mut app, "Lock after"), "{w}x{h}");
        assert!(!offered(&mut app, Action::IdleOff(5)), "{w}x{h}");
    }
}

#[test]
fn the_stick_settings_keep_power_off_and_its_timer() {
    for (w, h) in [(1366, 2400), (480, 640)] {
        let mut app = settings(false, w, h);
        assert!(has(&mut app, "Power off"), "{w}x{h}");
        assert!(has(&mut app, "Power off after"), "{w}x{h}");
        assert!(!has(&mut app, "Quit"), "{w}x{h}");
    }
}

#[test]
fn the_desktop_power_sheet_says_faraday_closes() {
    let mut app = settings(true, 1366, 2400);
    // Something still waiting to be written, so the button asks first.
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: vec![("vault.ofv".into(), vec![1, 2, 3])],
        kept: Vec::new(),
    });
    app.press(Action::PowerAsk);
    assert_eq!(app.sheet, Some(Sheet::Power));
    assert!(has(&mut app, "Faraday closes"));
    assert!(has(&mut app, "Quit"));
    assert!(!has(&mut app, "Power off"));
}
