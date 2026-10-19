//! Restart beside Power off on the PC stick and the Pi, with the BIOS key
//! where the Secure Boot reminder shows (`docs/FLOWS.md`, Restart; F42).

use faraday_core::{
    Action, Faraday, Medium, RESTART_CODE, RESTART_COMPUTER_CODE, SecureBootAtStart, Sheet,
    StorageEvent,
};
use osk_shell_api::{App, BootState, Command, DisplayInfo, Event, SecureHardware};

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
    assert!(t.iter().any(|s| s == "Power off"));
    assert!(t.iter().any(|s| s == "The computer powers off"));
}

#[test]
fn restart_with_files_waiting_asks_first_as_power_off_does() {
    let mut app = Faraday::new();
    with_outbox(&mut app);
    app.press(Action::PowerAsk);
    assert_eq!(app.sheet, Some(Sheet::Power));
    assert!(!exited(&mut app), "the sheet comes first");
    app.press(Action::Restart);
    assert!(exited(&mut app));
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
        now_ms: 21 * 60_000,
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
