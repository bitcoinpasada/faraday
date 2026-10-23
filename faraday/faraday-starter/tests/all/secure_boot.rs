//! Secure Boot back on is the step (`docs/STARTER.md` §6, F33): the
//! Starter shows, keeps and asks for no BitLocker recovery key, and
//! Windows' Use a device is offered only while Windows reports Secure
//! Boot off. Another encryption product still closes the firmware steps
//! until its rescue is confirmed (§6.4).

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{
    Bus, Computer, Disk, FaradayStick, InstallItem, Installed, Os, StarterCommand, StarterEvent,
};
use faraday_starter::decide::Item;
use faraday_starter::{Page, RestartTo, Sheet, Starter, StarterAction};

fn display() -> Event {
    Event::Display(DisplayInfo {
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
    })
}

/// A Windows computer whose Secure Boot reads `secure_boot`: on, off, or
/// not read (the registry value missing).
fn windows(secure_boot: Option<bool>) -> Computer {
    Computer {
        os: Os::Windows,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot,
        tpm_unlock: false,
    }
}

fn stick() -> Disk {
    Disk {
        id: "1".into(),
        device: "Disk 1".into(),
        vendor: "Maker".into(),
        model: "Stick".into(),
        serial: "SN-1".into(),
        size: 32_000_000_000,
        bus: Bus::Usb,
        volumes: Vec::new(),
        in_use: None,
        faraday: Some(FaradayStick {
            release: Some("6.6.84-faraday-0.2.0".into()),
            read: true,
        }),
    }
}

fn starter(secure_boot: Option<bool>) -> Starter {
    let mut starter = Starter::new();
    starter.event(display());
    starter.starter_event(StarterEvent::Computer(windows(secure_boot)));
    starter.starter_event(StarterEvent::Disks(vec![stick()]));
    starter
}

fn text(starter: &mut Starter) -> String {
    starter.drawn_texts().join(" ")
}

fn commands(starter: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| starter.next_command()).collect()
}

/// Every page of the Starter a Windows computer reaches, as the words on
/// it.
fn every_page(starter: &mut Starter) -> Vec<String> {
    let mut pages = vec![text(starter)];
    for action in [
        StarterAction::Start,
        StarterAction::Install,
        StarterAction::About,
    ] {
        starter.press(StarterAction::Home);
        starter.press(action);
        pages.push(text(starter));
    }
    starter.starter_event(StarterEvent::Installed(Some(Installed {
        version: "0.2.0".into(),
        items: vec![(
            InstallItem::App,
            r"%LOCALAPPDATA%\Programs\Faraday Starter\Faraday Starter.exe".into(),
        )],
    })));
    starter.press(StarterAction::Home);
    starter.press(StarterAction::Installed);
    pages.push(text(starter));
    starter.press(StarterAction::Uninstall);
    pages.push(text(starter));
    starter.press(StarterAction::Cancel);
    starter.press(StarterAction::Home);
    pages
}

#[test]
fn without_bitlocker_read_on_nothing_in_the_starter_asks_for_shows_or_holds_a_recovery_key() {
    // BitLocker's steps and the key on the sheet are offered only with
    // BitLocker read on the system drive (F49, `tests/bitlocker.rs`);
    // here Windows has answered nothing about it.
    for secure_boot in [Some(true), Some(false), None] {
        let mut s = starter(secure_boot);
        for page in every_page(&mut s) {
            let page = page.to_lowercase();
            for word in ["recovery key", "resume", "suspend", "back up", "key shown"] {
                assert!(!page.contains(word), "{secure_boot:?}: {word}: {page}");
            }
        }
        // Nothing was sent to the computer but the Start page's reading
        // of BitLocker, which changes nothing.
        assert!(
            commands(&mut s)
                .iter()
                .all(|c| *c == StarterCommand::ReadBitLocker)
        );
    }
}

#[test]
fn use_a_device_is_offered_only_while_windows_reports_secure_boot_off() {
    // Off: the card restarts from the stick, through Use a device.
    let mut s = starter(Some(false));
    assert!(text(&mut s).contains("Restart from the stick"));
    s.press(StarterAction::Next);
    assert_eq!(s.sheet, Some(Sheet::Restart(RestartTo::Stick)));
    s.press(StarterAction::RestartNow);
    assert!(matches!(
        commands(&mut s).as_slice(),
        [StarterCommand::RestartFromStick { .. }]
    ));
    s.press(StarterAction::Start);
    assert!(text(&mut s).contains("Use a device"));
    // The Start page's own button opens the same sheet.
    assert!(text(&mut s).contains("Restart to choose a device"));
    s.press(StarterAction::Cancel);
    s.press(StarterAction::RestartStick);
    assert_eq!(s.sheet, Some(Sheet::Restart(RestartTo::Stick)));

    // On, or not read: the BIOS only, and Use a device is not mentioned.
    for secure_boot in [Some(true), None] {
        let mut s = starter(secure_boot);
        let home = text(&mut s);
        assert!(home.contains("Restart into the BIOS"), "{secure_boot:?}");
        assert!(!home.contains("Restart from the stick"), "{secure_boot:?}");
        s.press(StarterAction::Next);
        assert_eq!(s.sheet, Some(Sheet::Restart(RestartTo::Firmware)));
        s.press(StarterAction::Cancel);
        s.press(StarterAction::Start);
        assert!(!text(&mut s).contains("Use a device"), "{secure_boot:?}");
        assert!(!text(&mut s).contains("Restart to choose a device"));
        s.press(StarterAction::RestartStick);
        assert_eq!(s.sheet, None);
    }

    // Reported on after being off: the offer goes.
    let mut s = starter(Some(false));
    s.starter_event(StarterEvent::Computer(windows(Some(true))));
    assert!(!text(&mut s).contains("Restart from the stick"));
}

#[test]
fn turn_secure_boot_back_on_is_listed_while_it_is_off_and_ends_the_start_page() {
    let mut s = starter(Some(false));
    assert!(text(&mut s).contains("Turn Secure Boot back on"));
    // It opens the Start page at its last step.
    s.press(StarterAction::Item(Item::SecureBootOn));
    assert_eq!(s.page, Page::Start);
    // The page is longer than the screen with a stick attached: its end.
    let _ = s.frame();
    for _ in 0..20 {
        s.event(Event::Wheel {
            x: 10,
            y: 10,
            dy: 2000,
        });
        let _ = s.frame();
    }
    assert!(text(&mut s).contains("Turn Secure Boot back on before starting Windows"));

    let mut s = starter(Some(true));
    assert!(!text(&mut s).contains("Turn Secure Boot back on"));
    s.press(StarterAction::Start);
    assert!(text(&mut s).contains("Turn Secure Boot back on before starting Windows"));
}

#[test]
fn another_encryption_product_closes_the_firmware_until_its_rescue_is_confirmed() {
    let mut s = starter(Some(true));
    s.starter_event(StarterEvent::OtherEncryption(Some("VeraCrypt".into())));
    let home = text(&mut s);
    assert!(home.contains("An encrypted drive Faraday does not manage: VeraCrypt"));
    assert!(home.contains("I have its rescue disk or password"));
    assert!(!home.contains("Restart into the BIOS"));
    // Pressed, the card confirms and sends nothing.
    s.press(StarterAction::Next);
    assert!(commands(&mut s).is_empty());
    assert!(text(&mut s).contains("Restart into the BIOS"));
    s.press(StarterAction::Next);
    assert_eq!(s.sheet, Some(Sheet::Restart(RestartTo::Firmware)));
}

#[test]
fn the_start_page_closes_its_restart_for_another_product_and_confirms_there() {
    let mut s = starter(Some(true));
    s.starter_event(StarterEvent::OtherEncryption(Some("VeraCrypt".into())));
    s.press(StarterAction::Start);
    let _ = text(&mut s);
    s.press(StarterAction::Restart);
    assert_eq!(s.sheet, None);
    assert!(text(&mut s).contains("I have its rescue disk or password"));
    s.press(StarterAction::Rescue);
    s.press(StarterAction::Restart);
    assert_eq!(s.sheet, Some(Sheet::Restart(RestartTo::Firmware)));
}

#[test]
fn use_a_device_is_not_offered_on_a_windows_started_in_legacy_bios_mode() {
    let mut c = windows(Some(false));
    c.uefi = false;
    let mut s = Starter::new();
    s.event(display());
    s.starter_event(StarterEvent::Computer(c));
    s.starter_event(StarterEvent::Disks(vec![stick()]));
    assert!(!text(&mut s).contains("Restart from the stick"));
    s.press(StarterAction::Start);
    assert!(!text(&mut s).contains("Use a device"));
}
