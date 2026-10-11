//! A sheet closes on a press beside it, as its own way out does: the
//! camera, a QR code. A press on the sheet itself does not close it.
//! The network choice is a Settings row now, not a sheet: it changes the
//! network at once, with nothing to close.

use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use osk_shell_api::{App, Command, Event, TouchPhase};

fn press_at(app: &mut Faraday, x: u16, y: u16) -> Vec<Command> {
    let _ = app.frame();
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    std::iter::from_fn(|| app.poll_command()).collect()
}

#[test]
fn the_camera_closes_on_a_press_beside_it() {
    let mut app = testkit::started();
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    let sent = press_at(&mut app, 4, 4);
    assert_eq!(app.sheet, None);
    assert!(sent.contains(&Command::CameraOff), "and the camera is off");
    assert_eq!(app.screen, Screen::Home, "nothing under it was pressed");
}

#[test]
fn the_network_choice_in_settings_changes_the_network_with_no_sheet() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Network(osk_bip::keys::Network::Testnet));
    assert_eq!(app.session.network(), osk_bip::keys::Network::Testnet);
    assert_eq!(app.sheet, None);
}

#[test]
fn a_press_on_the_sheet_itself_leaves_it_open() {
    let mut app = testkit::started();
    app.press(Action::Scan);
    // The sheet is centred: its middle is on it.
    press_at(&mut app, 640, 400);
    assert_eq!(app.sheet, Some(Sheet::Scan));
}
