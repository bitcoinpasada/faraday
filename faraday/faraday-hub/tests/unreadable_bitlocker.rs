//! On Windows, a drive whose BitLocker state the hub app has not read is
//! treated as protected (`docs/HUB.md` §6.2): the firmware steps stay
//! closed until the BitLocker step is done, so nobody is led to turn
//! Secure Boot off over a drive that may be sealed to it.

use faraday_hub::api::{BitLocker, Computer, HubCommand, HubEvent, Os, Protection};
use faraday_hub::{Hub, HubAction};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn display(width: u16, height: u16, dpi: u16) -> Event {
    Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

fn windows() -> Computer {
    Computer {
        os: Os::Windows,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: Some(true),
        tpm_unlock: false,
    }
}

fn hub(bitlocker: Option<BitLocker>) -> Hub {
    let mut hub = Hub::new();
    hub.event(display(1280, 800, 160));
    hub.hub_event(HubEvent::Computer(windows()));
    hub.hub_event(HubEvent::BitLocker(bitlocker));
    hub
}

/// Whether the frame now on screen offers a restart: Home's card, or
/// the Start page's own button.
fn restart_offered(hub: &mut Hub) -> bool {
    let _ = hub.drawn_texts();
    hub.offers_action(HubAction::Restart)
}

/// Every restart the hub has asked the shell for.
fn restarts_asked(hub: &mut Hub) -> usize {
    std::iter::from_fn(|| hub.poll_hub())
        .filter(|c| {
            matches!(
                c,
                HubCommand::RestartIntoFirmware | HubCommand::RestartFromStick { .. }
            )
        })
        .count()
}

#[test]
fn windows_with_no_bitlocker_state_read_offers_no_firmware_restart() {
    let mut hub = hub(None);
    let home = hub.drawn_texts();
    assert!(
        !home
            .iter()
            .any(|t| t.starts_with("Restart into") || t.starts_with("Restart from")),
        "{home:?}"
    );
    assert!(!restart_offered(&mut hub));
    hub.press(HubAction::Start);
    assert!(!restart_offered(&mut hub));
    hub.press(HubAction::Restart);
    hub.press(HubAction::RestartNow);
    assert_eq!(restarts_asked(&mut hub), 0);
}

#[test]
fn windows_with_a_drive_read_as_not_encrypted_offers_it() {
    let mut hub = hub(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::None,
        marker: None,
    }));
    hub.press(HubAction::Start);
    assert!(restart_offered(&mut hub));
}
