//! About's Nakamoto Institute: in the desktop's online app its name is a
//! link the shell opens in the browser; the stick and the Pi, offline,
//! show the same words as text and open nothing.

use faraday_bip::keys::Network;
use faraday_core::wallet::Session;
use faraday_core::{Action, Faraday, Screen, StorageCommand, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// Settings on a screen tall enough for About, online or not; online,
/// on mainnet, where the online app has no warning over it.
fn settings(online: bool) -> Faraday {
    let mut app = testkit::started();
    app.online = online;
    app.session = Session::on(Network::Mainnet);
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
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
    let _ = app.frame();
    while app.poll_storage().is_some() {}
    app
}

#[test]
fn the_desktop_opens_the_institute_from_about() {
    let mut app = settings(true);
    assert!(app.offers(Action::OpenInstitute));
    app.press(Action::OpenInstitute);
    assert!(std::iter::from_fn(|| app.poll_storage()).any(|c| c == StorageCommand::OpenInstitute));
}

#[test]
fn the_stick_shows_the_institute_as_text_and_opens_nothing() {
    let mut app = settings(false);
    assert!(!app.offers(Action::OpenInstitute));
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.contains("nakamotoinstitute.org"))
    );
    app.press(Action::OpenInstitute);
    assert!(!std::iter::from_fn(|| app.poll_storage()).any(|c| c == StorageCommand::OpenInstitute));
}
