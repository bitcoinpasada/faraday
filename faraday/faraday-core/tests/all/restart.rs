//! Restart beside Power off on the PC stick and the Pi, with the BIOS key
//! where the Secure Boot reminder shows (`docs/FLOWS.md`, Restart; F42).

use faraday_core::vault::HOLD_MS;
use faraday_core::{
    Action, Faraday, Medium, RESTART_CODE, RESTART_COMPUTER_CODE, SecureBootAtStart, Sheet,
    StorageEvent,
};
use faraday_shell_api::{App, BootState, Command, DisplayInfo, Event, SecureHardware, TouchPhase};

fn shown(app: &mut Faraday) -> Vec<String> {
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
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
    app.drawn_texts()
}

fn exited(app: &mut Faraday) -> bool {
    let mut exit = false;
    while let Some(c) = app.poll_command() {
        exit |= c == Command::Exit;
    }
    exit
}

fn with_outbox(app: &mut Faraday) {
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: vec![("vault.ofv".into(), vec![1, 2, 3])],
        kept: Vec::new(),
    });
}

#[test]
fn the_stick_and_the_pi_offer_restart_and_power_off() {
    for medium in [Medium::Stick, Medium::SdCard] {
        let mut app = Faraday::new();
        app.medium = medium;
        app.press(Action::PowerAsk);
        assert_eq!(app.sheet, Some(Sheet::Power), "{medium:?}");
        let t = shown(&mut app);
        for label in ["Power", "Cancel", "Restart", "Power off"] {
            assert!(t.iter().any(|s| s == label), "{medium:?} {label}: {t:?}");
        }
        assert!(
            t.iter()
                .any(|s| s == "The computer powers off, or restarts")
        );
    }
}

#[test]
fn faraday_desktop_offers_power_off_only() {
    let mut app = Faraday::new();
    app.shell_sticks = true;
    with_outbox(&mut app);
    app.press(Action::PowerAsk);
    let t = shown(&mut app);
    assert!(!t.iter().any(|s| s == "Restart"), "{t:?}");
    assert!(
        t.iter().any(|s| s == "Hold to power off"),
        "files wait: {t:?}"
    );
    assert!(t.iter().any(|s| s == "The computer powers off"));
}

/// Touches `action` where the last frame drew it at `t0` ms and lets go
/// at `t1`, a tick between.
fn held(app: &mut Faraday, action: Action, t0: u64, t1: u64) {
    let (x, y) = app.where_offered(action).expect("on screen");
    app.event(Event::Tick { now_ms: t0 });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    app.event(Event::Tick { now_ms: t1 });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
}

/// Files waiting for the stick: Restart and Power off each take a hold
/// (decision 5); a press let go too soon does nothing.
#[test]
fn with_files_waiting_restart_and_power_off_take_a_hold() {
    let mut app = Faraday::new();
    with_outbox(&mut app);
    app.press(Action::PowerAsk);
    assert_eq!(app.sheet, Some(Sheet::Power));
    assert!(!exited(&mut app), "the sheet comes first");
    let t = shown(&mut app);
    for label in ["Hold to restart", "Hold to power off"] {
        assert!(t.iter().any(|s| s == label), "{label}: {t:?}");
    }
    assert!(!app.offers(Action::PowerOff) && !app.offers(Action::Restart));
    held(&mut app, Action::HoldPowerOff, 1_000, 1_200);
    let _ = app.frame();
    assert!(!exited(&mut app), "let go too soon");
    held(&mut app, Action::HoldRestart, 5_000, 5_000 + HOLD_MS);
    assert!(exited(&mut app));
    assert!(app.computer_restart_requested());
}

/// With nothing waiting, Power off is a press.
#[test]
fn with_nothing_waiting_power_off_is_a_press() {
    let mut app = Faraday::new();
    app.medium = Medium::Stick;
    app.press(Action::PowerAsk);
    let _ = shown(&mut app);
    assert!(app.offers(Action::PowerOff) && app.offers(Action::Restart));
    assert!(!app.offers(Action::HoldPowerOff));
}

#[test]
fn restart_ends_the_app_with_restarts_status_and_power_off_with_its_own() {
    let mut app = Faraday::new();
    app.press(Action::PowerAsk);
    app.press(Action::Restart);
    assert!(exited(&mut app));
    assert!(app.computer_restart_requested());
    assert!(!app.restart_requested(), "75 stays the lock's");
    assert_ne!(RESTART_COMPUTER_CODE, RESTART_CODE);

    let mut app = Faraday::new();
    app.press(Action::PowerAsk);
    app.press(Action::PowerOff);
    assert!(exited(&mut app));
    assert!(!app.computer_restart_requested());
}

#[test]
fn the_idle_power_off_never_restarts() {
    let mut app = Faraday::new();
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick {
        now_ms: 61 * 60_000,
    });
    assert!(exited(&mut app));
    assert!(!app.computer_restart_requested());
}

fn sheet_texts(maker: &str, secure_boot: SecureBootAtStart) -> Vec<String> {
    let mut app = Faraday::new();
    app.medium = Medium::Stick;
    app.maker = maker.into();
    app.secure_boot = secure_boot;
    app.press(Action::PowerAsk);
    shown(&mut app)
}

#[test]
fn a_dell_started_with_secure_boot_off_shows_its_bios_key() {
    let t = sheet_texts("Dell Inc.", SecureBootAtStart::Off);
    assert!(t.iter().any(|s| s == "Usual BIOS key"), "{t:?}");
    assert!(t.iter().any(|s| s == "F2"), "{t:?}");
}

#[test]
fn a_computer_started_with_secure_boot_on_shows_no_row() {
    let t = sheet_texts("Dell Inc.", SecureBootAtStart::On);
    assert!(!t.iter().any(|s| s == "Usual BIOS key"), "{t:?}");
}

#[test]
fn an_unknown_maker_shows_no_row_and_the_guides_code() {
    let t = sheet_texts("Example Computers", SecureBootAtStart::Off);
    assert!(!t.iter().any(|s| s == "Usual BIOS key"), "{t:?}");
    assert!(
        t.iter().any(|s| s.contains("faradayoffline.app/start/")),
        "{t:?}"
    );
}
