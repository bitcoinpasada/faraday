//! On Windows, a drive whose BitLocker state the Starter has not read is
//! treated as protected (`docs/STARTER.md` §6.2): the firmware steps stay
//! closed until the BitLocker step is done, so nobody is led to turn
//! Secure Boot off over a drive that may be sealed to it.

use faraday_starter::api::{BitLocker, Computer, Os, Protection, StarterCommand, StarterEvent};
use faraday_starter::{Starter, StarterAction};
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

fn starter(bitlocker: Option<BitLocker>) -> Starter {
    let mut starter = Starter::new();
    starter.event(display(1280, 800, 160));
    starter.starter_event(StarterEvent::Computer(windows()));
    starter.starter_event(StarterEvent::BitLocker(bitlocker));
    starter
}

/// Whether the frame now on screen offers a restart: Home's card, or
/// the Start page's own button.
fn restart_offered(starter: &mut Starter) -> bool {
    let _ = starter.drawn_texts();
    starter.offers_action(StarterAction::Restart)
}

/// Every restart the Starter has asked the shell for.
fn restarts_asked(starter: &mut Starter) -> usize {
    std::iter::from_fn(|| starter.next_command())
        .filter(|c| {
            matches!(
                c,
                StarterCommand::RestartIntoFirmware | StarterCommand::RestartFromStick { .. }
            )
        })
        .count()
}

#[test]
fn windows_with_no_bitlocker_state_read_offers_no_firmware_restart() {
    let mut starter = starter(None);
    let home = starter.drawn_texts();
    assert!(
        !home
            .iter()
            .any(|t| t.starts_with("Restart into") || t.starts_with("Restart from")),
        "{home:?}"
    );
    assert!(!restart_offered(&mut starter));
    starter.press(StarterAction::Start);
    assert!(!restart_offered(&mut starter));
    starter.press(StarterAction::Restart);
    starter.press(StarterAction::RestartNow);
    assert_eq!(restarts_asked(&mut starter), 0);
}

#[test]
fn windows_with_a_drive_read_as_not_encrypted_offers_it() {
    let mut starter = starter(Some(BitLocker {
        drive: "C:".into(),
        protection: Protection::None,
        marker: None,
    }));
    starter.press(StarterAction::Start);
    assert!(restart_offered(&mut starter));
}
