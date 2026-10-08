//! Home: Add a key leads the Start tiles, and Scan floats at the foot.
//! Keys load only with no stick attached: Add a key pressed with one in
//! asks for it to be pulled, and opens once it is. Scanning loads no key,
//! so Scan stays open with a stick in.

use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent};
use osk_shell_api::{App, Command, DisplayInfo, Event, TouchPhase};

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

fn stick(app: &mut Faraday, on: bool) {
    let sticks = if on {
        vec![StickInfo {
            id: "a".into(),
            label: "STICK".into(),
            boot: false,
            files: Vec::new(),
        }]
    } else {
        Vec::new()
    };
    app.storage(StorageEvent::Sticks(sticks));
    let _ = app.frame();
}

/// Presses `action` where the last frame drew it.
fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

/// Home with a stick attached: the stick visit opens on its arrival, and
/// Home is one press away.
fn home_with_stick() -> Faraday {
    let mut app = shown();
    stick(&mut app, true);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

#[test]
fn home_offers_scan_with_no_stick_attached() {
    let app = shown();
    assert!(app.offers(Action::Scan));
}

#[test]
fn home_offers_scan_with_a_stick_attached() {
    let app = home_with_stick();
    assert!(app.offers(Action::Scan));
}

#[test]
fn scan_on_home_starts_the_camera() {
    let mut app = shown();
    while app.poll_command().is_some() {}
    tap(&mut app, Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::CameraOn));
}

#[test]
fn add_a_key_on_home_opens_add_a_key() {
    let mut app = shown();
    tap(&mut app, Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(app.sheet, None);
}

#[test]
fn add_a_key_with_a_stick_in_asks_for_it_and_opens_once_it_is_pulled() {
    let mut app = home_with_stick();
    tap(&mut app, Action::Entry(None));
    assert_eq!(app.sheet, Some(Sheet::Pull));
    assert_eq!(app.screen, Screen::Home);
    stick(&mut app, false);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn cancel_leaves_home_and_a_later_pull_opens_nothing() {
    let mut app = home_with_stick();
    tap(&mut app, Action::Entry(None));
    tap(&mut app, Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Home);
    stick(&mut app, false);
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn the_wallets_tab_offers_add_a_key() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Start));
    tap(&mut app, Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn make_a_key_with_a_stick_in_waits_for_the_pull() {
    let mut app = home_with_stick();
    app.press(Action::KeyGen(None));
    assert_eq!(app.sheet, Some(Sheet::Pull));
    stick(&mut app, false);
    assert_eq!(app.screen, Screen::KeyGen);
}
