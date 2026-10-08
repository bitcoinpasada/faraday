//! The start-up self-test (`osk_selftest`): it runs when the display
//! first arrives, before anything can be pressed, and no key is accepted
//! before it has passed. A failed check blocks the app on a screen that
//! names it, with Exit the only thing that does anything. Settings runs
//! it again.

use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, BootState, Command, DisplayInfo, Event, Key, SecureHardware};

#[test]
fn the_self_test_passes_at_start_and_settings_runs_it_again() {
    let mut app = faraday_core::testkit::started();
    assert_eq!(app.selftest(), Some(Ok(osk_selftest::CHECKS.len())));
    assert!(app.may_load_keys());
    // A display tall enough for all of Settings.
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
    assert!(app.offers(Action::SelfTestRun));
    app.press(Action::SelfTestRun);
    assert_eq!(app.selftest(), Some(Ok(osk_selftest::CHECKS.len())));
}

#[test]
fn no_key_is_accepted_before_the_self_test_has_run() {
    let mut app = Faraday::new();
    assert_eq!(app.selftest(), None);
    assert!(!app.may_load_keys());
    app.press(Action::Entry(None));
    assert_ne!(app.screen, Screen::Entry);
}

#[test]
fn a_failed_check_blocks_everything_but_exit() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::Settings));
    app.run_selftest_with(&[osk_selftest::Check {
        name: "Injected failure",
        run: || false,
    }]);
    assert_eq!(app.selftest(), Some(Err("Injected failure")));
    let _ = app.frame();
    assert!(app.offers(Action::PowerOff), "Exit is on screen");
    assert!(!app.offers(Action::Nav(Screen::Wallets)), "nothing else is");
    // No press and no key does anything.
    app.press(Action::Entry(None));
    app.press(Action::Nav(Screen::Wallets));
    app.press(Action::SelfTestRun);
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.screen, Screen::Settings);
    assert!(!app.may_load_keys());
    assert_eq!(app.selftest(), Some(Err("Injected failure")));
    while app.poll_command().is_some() {}
    // Exit exits.
    app.press(Action::PowerOff);
    let mut exited = false;
    while let Some(c) = app.poll_command() {
        exited |= c == Command::Exit;
    }
    assert!(exited);
}
