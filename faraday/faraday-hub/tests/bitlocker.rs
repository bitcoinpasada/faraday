//! The BitLocker guard as a person meets it (`docs/HUB.md` §6, §11): the
//! key shown, kept and confirmed with it hidden before anything is
//! suspended; the firmware steps closed until both parts are done; the
//! cases the guard cannot pass; Resume, and Uninstall while suspended.
//! The shell's answers are made up here as the elevated copy gives them.

use faraday_hub::api::{
    BitLocker, BitLockerStep, Bus, Computer, Disk, FaradayStick, HubCommand, HubEvent, InstallItem,
    Installed, KeyShown, Marker, Os, Protection, Reason, Volume,
};
use faraday_hub::{Hub, HubAction, Page};
use faraday_qr::bitlocker::{KeyText, RecoveryKey};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

const KEY: &str = "205568-492019-108251-336446-650375-210331-584155-047520";
const GROUPS: [&str; 8] = [
    "205568", "492019", "108251", "336446", "650375", "210331", "584155", "047520",
];

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

fn windows(secure_boot: bool) -> Computer {
    Computer {
        os: Os::Windows,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: Some(secure_boot),
        tpm_unlock: false,
    }
}

fn protected() -> BitLocker {
    BitLocker {
        drive: "C:".into(),
        protection: Protection::On,
        marker: None,
    }
}

fn suspended(was_on: bool) -> BitLocker {
    BitLocker {
        drive: "C:".into(),
        protection: Protection::Suspended,
        marker: Some(Marker {
            volumes: vec!["C:".into()],
            secure_boot_was_on: was_on,
            this_boot: false,
        }),
    }
}

fn key() -> KeyShown {
    KeyShown {
        text: KeyText {
            computer: "DESKTOP-4K2Q".into(),
            drive: "C:".into(),
            key: RecoveryKey::parse(KEY).expect("a key"),
        },
        id: "{6A0E1F2C-3B4D-4E5F-8A9B-0C1D2E3F4A5B}".into(),
    }
}

fn usb(id: &str, files: bool) -> Disk {
    Disk {
        id: id.into(),
        device: format!("Disk {id}"),
        vendor: "Kingston".into(),
        model: "DataTraveler".into(),
        serial: format!("SN{id}"),
        size: 64_000_000_000,
        bus: Bus::Usb,
        volumes: vec![Volume {
            label: Some("PHOTOS".into()),
            files,
        }],
        in_use: None,
        faraday: None,
    }
}

fn with(secure_boot: bool, b: BitLocker) -> Hub {
    let mut hub = Hub::new();
    hub.event(display(1280, 800, 160));
    hub.hub_event(HubEvent::Computer(windows(secure_boot)));
    hub.hub_event(HubEvent::BitLocker(Some(b)));
    hub.hub_event(HubEvent::Disks(Vec::new()));
    hub
}

fn texts(hub: &mut Hub) -> Vec<String> {
    hub.drawn_texts()
}

/// Whether `s` is on screen, a line wrapped over several read as one.
fn shows(hub: &mut Hub, s: &str) -> bool {
    texts(hub).join(" ").contains(s)
}

fn offered(hub: &mut Hub, a: HubAction) -> bool {
    let _ = hub.drawn_texts();
    hub.offers_action(a)
}

fn commands(hub: &mut Hub) -> Vec<HubCommand> {
    std::iter::from_fn(|| hub.poll_hub()).collect()
}

/// Whether any digit group of the key is on screen.
fn key_on_screen(hub: &mut Hub) -> bool {
    let t = texts(hub);
    GROUPS.iter().any(|g| t.iter().any(|s| s.contains(g)))
}

/// Whether a firmware step can be taken: Home's card or the Start
/// page's restart.
fn firmware_open(hub: &mut Hub) -> bool {
    let page = hub.page;
    hub.press(HubAction::Start);
    let open = offered(hub, HubAction::Restart);
    hub.press(HubAction::Restart);
    hub.press(HubAction::RestartNow);
    let sent = commands(hub)
        .iter()
        .any(|c| matches!(c, HubCommand::RestartIntoFirmware));
    hub.press(HubAction::Home);
    if page == Page::BitLocker {
        hub.press(HubAction::BitLocker);
    }
    assert_eq!(open, sent);
    open
}

/// Shows the key: the elevation prompt's step, answered.
fn show(hub: &mut Hub, entropy: [u8; 2]) {
    hub.press(HubAction::BitLocker);
    hub.press(HubAction::ShowKey);
    assert_eq!(
        commands(hub),
        vec![HubCommand::BitLocker(BitLockerStep::ShowKey)]
    );
    hub.hub_event(HubEvent::KeyShown(Ok(key())));
    // The hub asked for random bytes to choose the groups from.
    let mut asked = false;
    while let Some(c) = hub.poll_command() {
        asked |= c == osk_shell_api::Command::RequestEntropy;
    }
    assert!(asked);
    let mut bytes = [0u8; 32];
    bytes[..2].copy_from_slice(&entropy);
    hub.event(Event::Entropy(EntropyBytes::new(bytes)));
}

fn type_digits(hub: &mut Hub, s: &str) {
    for c in s.chars() {
        hub.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn the_key_is_confirmed_with_it_hidden_before_bitlocker_is_suspended() {
    let mut hub = with(true, protected());
    hub.press(HubAction::BitLocker);
    assert!(shows(&mut hub, "Drive C: is encrypted with BitLocker"));
    assert!(offered(&mut hub, HubAction::ShowKey));
    assert!(!offered(&mut hub, HubAction::Suspend));

    show(&mut hub, [3, 2]);
    // The key, its QR code's line and its ID.
    assert!(key_on_screen(&mut hub));
    assert!(shows(&mut hub, "Or take a photo of this screen."));
    assert!(shows(&mut hub, "6A0E1F2C"));
    assert!(offered(&mut hub, HubAction::KeptIt));
    assert!(!offered(&mut hub, HubAction::Suspend));

    // Kept: the key is hidden, and groups 4 and 7 are asked for.
    hub.press(HubAction::KeptIt);
    assert!(!key_on_screen(&mut hub));
    assert!(shows(&mut hub, "Type groups 4 and 7 of the key"));
    type_digits(&mut hub, GROUPS[3]);
    type_digits(&mut hub, GROUPS[6]);
    // Only what was typed is on screen, never the rest of the key.
    assert!(!texts(&mut hub).iter().any(|t| t.contains(GROUPS[0])));
    hub.press(HubAction::Suspend);
    assert_eq!(
        commands(&mut hub),
        vec![HubCommand::BitLocker(BitLockerStep::Suspend)]
    );
    assert!(shows(&mut hub, "Suspending"));
}

#[test]
fn a_wrong_group_is_refused_and_nothing_is_suspended() {
    let mut hub = with(true, protected());
    show(&mut hub, [0, 0]);
    hub.press(HubAction::KeptIt);
    // Groups 1 and 2.
    assert!(shows(&mut hub, "Type groups 1 and 2 of the key"));
    type_digits(&mut hub, GROUPS[0]);
    type_digits(&mut hub, "492018");
    hub.event(Event::Key(Key::Enter));
    assert!(commands(&mut hub).is_empty());
    assert!(shows(&mut hub, "Group 2 does not match the key."));
    // Another group of the key is not the one asked for.
    hub.event(Event::Key(Key::Backspace));
    type_digits(&mut hub, "9");
    hub.event(Event::Key(Key::Tab));
    for _ in 0..6 {
        hub.event(Event::Key(Key::Backspace));
    }
    type_digits(&mut hub, GROUPS[2]);
    hub.press(HubAction::Suspend);
    assert!(commands(&mut hub).is_empty());
    assert!(shows(&mut hub, "Group 1 does not match the key."));
    hub.press(HubAction::Group(0));
    for _ in 0..6 {
        hub.event(Event::Key(Key::Backspace));
    }
    type_digits(&mut hub, GROUPS[0]);
    hub.press(HubAction::Suspend);
    assert_eq!(
        commands(&mut hub),
        vec![HubCommand::BitLocker(BitLockerStep::Suspend)]
    );
}

#[test]
fn suspend_is_never_sent_before_the_key_is_confirmed() {
    let mut hub = with(true, protected());
    hub.press(HubAction::BitLocker);
    // No key yet.
    hub.press(HubAction::Suspend);
    assert!(commands(&mut hub).is_empty());
    // The key shown, not yet kept: typing does nothing, Suspend nothing.
    show(&mut hub, [5, 1]);
    type_digits(&mut hub, KEY);
    hub.event(Event::Key(Key::Enter));
    hub.press(HubAction::Suspend);
    assert!(commands(&mut hub).is_empty());
    // Kept, nothing typed.
    hub.press(HubAction::KeptIt);
    hub.press(HubAction::Suspend);
    assert!(commands(&mut hub).is_empty());
    // Shown again: the groups typed are gone, and so is Suspend.
    hub.press(HubAction::ShowAgain);
    assert!(key_on_screen(&mut hub));
    hub.press(HubAction::Suspend);
    assert!(commands(&mut hub).is_empty());
    // Leaving the page forgets the key: it is shown again from the
    // elevation prompt.
    hub.press(HubAction::Home);
    hub.press(HubAction::BitLocker);
    assert!(!key_on_screen(&mut hub));
    assert!(offered(&mut hub, HubAction::ShowKey));
}

#[test]
fn no_firmware_step_before_both_parts_are_done() {
    let mut hub = with(true, protected());
    assert!(!firmware_open(&mut hub));
    show(&mut hub, [1, 1]);
    assert!(!firmware_open(&mut hub));
    hub.press(HubAction::BitLocker);
    show(&mut hub, [1, 1]);
    hub.press(HubAction::KeptIt);
    let asked = [1, 3];
    type_digits(&mut hub, GROUPS[asked[0]]);
    type_digits(&mut hub, GROUPS[asked[1]]);
    hub.press(HubAction::Suspend);
    assert_eq!(
        commands(&mut hub),
        vec![HubCommand::BitLocker(BitLockerStep::Suspend)]
    );
    // Sent, not yet answered.
    assert!(!firmware_open(&mut hub));
    // Suspended, the marker written: the firmware steps open, and the
    // next step is still Start in this session.
    hub.press(HubAction::BitLocker);
    hub.hub_event(HubEvent::BitLocker(Some(suspended(true))));
    hub.hub_event(HubEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Ok(()),
    });
    assert!(shows(
        &mut hub,
        "Windows starts without the key until BitLocker is resumed. Resume it here once Secure Boot is back on."
    ));
    assert!(shows(&mut hub, "Open Faraday again to do that."));
    assert!(!key_on_screen(&mut hub));
    assert!(firmware_open(&mut hub));
}

#[test]
fn a_refused_suspension_shows_the_managed_line_and_keeps_the_firmware_closed() {
    let mut hub = with(true, protected());
    show(&mut hub, [2, 0]);
    hub.press(HubAction::KeptIt);
    type_digits(&mut hub, GROUPS[2]);
    type_digits(&mut hub, GROUPS[3]);
    hub.press(HubAction::Suspend);
    let _ = commands(&mut hub);
    hub.hub_event(HubEvent::BitLocker(Some(protected())));
    hub.hub_event(HubEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Err(Reason::Managed(0x8031_0050)),
    });
    assert!(shows(
        &mut hub,
        "BitLocker was not suspended (0x80310050). This computer's BitLocker is managed: its administrator has to suspend it."
    ));
    assert!(!firmware_open(&mut hub));
}

#[test]
fn a_declined_prompt_says_why_and_offers_the_step_again() {
    for (why, says) in [
        (
            Reason::Declined,
            "Administrator permission was declined. Nothing was changed.",
        ),
        (
            Reason::NoAdmin,
            "An administrator of this computer has to do this step.",
        ),
    ] {
        let mut hub = with(true, protected());
        hub.press(HubAction::BitLocker);
        hub.press(HubAction::ShowKey);
        assert!(shows(&mut hub, "Waiting for administrator permission"));
        hub.hub_event(HubEvent::KeyShown(Err(why)));
        assert!(shows(&mut hub, says));
        assert!(offered(&mut hub, HubAction::ShowKey));
        assert!(!firmware_open(&mut hub));
    }
}

#[test]
fn save_is_offered_only_with_a_removable_disk_that_is_not_a_faraday_stick() {
    let mut hub = with(true, protected());
    show(&mut hub, [0, 1]);
    assert!(!shows(&mut hub, "Save to"));
    // A Faraday stick, a disk the computer uses, a disk inside it, an
    // empty one: none.
    let mut stick = usb("1", true);
    stick.faraday = Some(FaradayStick { release: None });
    let mut used = usb("2", true);
    used.in_use = Some(faraday_hub::api::InUse::System);
    let mut inside = usb("3", true);
    inside.bus = Bus::Other;
    let blank = usb("4", false);
    hub.hub_event(HubEvent::Disks(vec![stick, used, inside, blank]));
    assert!(!shows(&mut hub, "Save to"));
    // A USB disk with a file system: offered, and Save names it.
    hub.hub_event(HubEvent::Disks(vec![usb("5", true)]));
    assert!(shows(&mut hub, "Save to Kingston DataTraveler"));
    let n = HubAction::SaveKey(hub.disk_number("5"));
    hub.press(n);
    assert_eq!(
        commands(&mut hub),
        vec![HubCommand::SaveKey {
            disk: "5".into(),
            key: key()
        }]
    );
    hub.hub_event(HubEvent::KeySaved(Ok(
        r"E:\BitLocker recovery key 6A0E1F2C.txt".into(),
    )));
    assert!(shows(
        &mut hub,
        r"Saved to E:\BitLocker recovery key 6A0E1F2C.txt"
    ));
    // Hidden, there is nothing to save.
    hub.press(HubAction::KeptIt);
    hub.press(n);
    assert!(commands(&mut hub).is_empty());
}

#[test]
fn resume_is_offered_only_with_the_marker_and_secure_boot_as_it_recorded() {
    // Secure Boot on, as recorded: Resume on the page, sent once.
    let mut hub = with(true, suspended(true));
    hub.press(HubAction::BitLocker);
    assert!(offered(&mut hub, HubAction::Resume));
    hub.press(HubAction::Resume);
    assert_eq!(
        commands(&mut hub),
        vec![HubCommand::BitLocker(BitLockerStep::Resume)]
    );
    assert!(shows(&mut hub, "Resuming"));
    hub.hub_event(HubEvent::BitLocker(Some(protected())));
    hub.hub_event(HubEvent::BitLockerDone {
        step: BitLockerStep::Resume,
        result: Ok(()),
    });
    assert!(shows(&mut hub, "BitLocker resumed"));
    // Resumed, the drive is protected again: the firmware steps need the
    // guard again.
    assert!(!firmware_open(&mut hub));

    // Secure Boot off, recorded on: not offered, and the line says why.
    let mut hub = with(false, suspended(true));
    hub.press(HubAction::BitLocker);
    assert!(!offered(&mut hub, HubAction::Resume));
    assert!(shows(&mut hub, "Needs Secure Boot on"));
    hub.press(HubAction::Resume);
    assert!(commands(&mut hub).is_empty());

    // No marker: nothing of the hub app's to resume.
    let mut hub = with(true, protected());
    hub.press(HubAction::BitLocker);
    assert!(!offered(&mut hub, HubAction::Resume));
}

#[test]
fn a_suspension_undone_later_offers_the_step_again() {
    // Protected again, the marker still there.
    let mut b = suspended(true);
    b.protection = Protection::On;
    let mut hub = with(true, b);
    assert!(shows(&mut hub, "Protected"));
    hub.press(HubAction::BitLocker);
    assert!(shows(
        &mut hub,
        "Protected again after Faraday suspended it"
    ));
    assert!(offered(&mut hub, HubAction::ShowKey));
    assert!(!offered(&mut hub, HubAction::Resume));
    assert!(!firmware_open(&mut hub));
}

#[test]
fn a_drive_suspended_by_someone_else_still_needs_the_guard() {
    let mut b = protected();
    b.protection = Protection::Suspended;
    let mut hub = with(true, b);
    assert!(!firmware_open(&mut hub));
    hub.press(HubAction::BitLocker);
    assert!(offered(&mut hub, HubAction::ShowKey));
}

#[test]
fn another_encryption_product_closes_the_firmware_until_its_rescue_is_confirmed() {
    let mut hub = with(
        true,
        BitLocker {
            drive: "C:".into(),
            protection: Protection::None,
            marker: None,
        },
    );
    assert!(firmware_open(&mut hub));
    hub.hub_event(HubEvent::OtherEncryption(Some("VeraCrypt".into())));
    assert!(!firmware_open(&mut hub));
    assert!(shows(&mut hub, "Not managed by Faraday"));
    hub.press(HubAction::BitLocker);
    assert!(shows(
        &mut hub,
        "An encrypted drive Faraday does not manage: VeraCrypt"
    ));
    hub.press(HubAction::Rescue);
    assert!(firmware_open(&mut hub));
}

fn installed() -> Option<Installed> {
    Some(Installed {
        version: "0.2.0".into(),
        items: vec![
            (
                InstallItem::App,
                r"%LOCALAPPDATA%\Programs\Faraday\Faraday.exe".into(),
            ),
            (
                InstallItem::Reminder,
                r"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run\Faraday".into(),
            ),
        ],
    })
}

#[test]
fn uninstall_while_suspended_offers_resume_with_secure_boot_on() {
    let mut hub = with(true, suspended(true));
    hub.hub_event(HubEvent::Installed(installed()));
    hub.press(HubAction::Installed);
    hub.press(HubAction::Uninstall);
    assert!(shows(
        &mut hub,
        "BitLocker stays suspended until it is resumed."
    ));
    assert!(offered(&mut hub, HubAction::Resume));
    hub.press(HubAction::Resume);
    assert_eq!(
        commands(&mut hub),
        vec![HubCommand::BitLocker(BitLockerStep::Resume)]
    );
    assert_eq!(hub.page, Page::BitLocker);
    assert!(hub.sheet.is_none());
}

#[test]
fn uninstall_while_suspended_with_secure_boot_off_says_it_stays_suspended() {
    let mut hub = with(false, suspended(true));
    hub.hub_event(HubEvent::Installed(installed()));
    hub.press(HubAction::Installed);
    hub.press(HubAction::Uninstall);
    assert!(shows(
        &mut hub,
        "BitLocker stays suspended until it is resumed: Control Panel, BitLocker Drive Encryption, Resume protection."
    ));
    assert!(!offered(&mut hub, HubAction::Resume));
    hub.press(HubAction::UninstallNow);
    assert_eq!(commands(&mut hub), vec![HubCommand::Uninstall]);
}

#[test]
fn an_installed_hub_app_does_not_say_open_it_again() {
    let mut hub = with(true, suspended(true));
    hub.hub_event(HubEvent::Installed(installed()));
    hub.press(HubAction::BitLocker);
    assert!(shows(
        &mut hub,
        "Windows starts without the key until BitLocker is resumed."
    ));
    assert!(!shows(&mut hub, "Open Faraday again to do that."));
}

#[test]
fn the_guard_fits_a_small_panel() {
    let mut hub = Hub::new();
    hub.event(display(480, 640, 286));
    hub.hub_event(HubEvent::Computer(windows(true)));
    hub.hub_event(HubEvent::BitLocker(Some(protected())));
    show(&mut hub, [6, 6]);
    assert!(key_on_screen(&mut hub));
    // I have kept it is under the code: scrolled to.
    let _ = hub.frame();
    for _ in 0..20 {
        hub.event(Event::Wheel {
            x: 10,
            y: 10,
            dy: 2000,
        });
        let _ = hub.frame();
    }
    assert!(offered(&mut hub, HubAction::KeptIt));
    hub.press(HubAction::KeptIt);
    assert!(!key_on_screen(&mut hub));
}

/// The key step answered `why` with the drive protected as the shell
/// reads it (2 on a shell reading is not trusted: not known).
fn key_step_answers(hub: &mut Hub, why: Reason) {
    hub.press(HubAction::BitLocker);
    hub.press(HubAction::ShowKey);
    let _ = commands(hub);
    hub.hub_event(HubEvent::KeyShown(Err(why)));
}

fn unknown() -> BitLocker {
    BitLocker {
        drive: "C:".into(),
        protection: Protection::Unknown,
        marker: None,
    }
}

#[test]
fn a_drive_the_shell_could_not_read_keeps_the_guard_required() {
    let mut hub = with(true, unknown());
    assert!(!firmware_open(&mut hub));
    hub.press(HubAction::BitLocker);
    assert!(offered(&mut hub, HubAction::ShowKey));
}

#[test]
fn the_guard_reading_not_encrypted_opens_the_firmware_steps_for_the_session() {
    let mut hub = with(true, unknown());
    assert!(!firmware_open(&mut hub));
    key_step_answers(&mut hub, Reason::NotEncrypted);
    assert!(shows(&mut hub, "Drive C: not encrypted"));
    assert!(firmware_open(&mut hub));
    // The shell reads the drive as not known again: still open.
    hub.hub_event(HubEvent::BitLocker(Some(unknown())));
    assert!(firmware_open(&mut hub));
}

#[test]
fn the_guard_reading_not_encrypted_opens_them_when_no_reading_was_made() {
    let mut hub = Hub::new();
    hub.event(display(1280, 800, 160));
    hub.hub_event(HubEvent::Computer(windows(true)));
    hub.hub_event(HubEvent::BitLocker(None));
    hub.hub_event(HubEvent::Disks(Vec::new()));
    assert!(!firmware_open(&mut hub));
    key_step_answers(&mut hub, Reason::NotEncrypted);
    assert!(firmware_open(&mut hub));
}

#[test]
fn a_drive_windows_does_not_list_keeps_the_firmware_closed_and_says_so() {
    let mut hub = with(true, unknown());
    key_step_answers(&mut hub, Reason::NotRead);
    assert!(shows(
        &mut hub,
        "BitLocker on the system drive could not be read. The drive is treated as protected."
    ));
    assert!(!firmware_open(&mut hub));
    // Any other failure keeps it closed too.
    key_step_answers(
        &mut hub,
        Reason::BitLocker {
            call: "GetKeyProtectors".into(),
            code: 0x8031_0008,
        },
    );
    assert!(!firmware_open(&mut hub));
}

fn suspended_now() -> BitLocker {
    let mut b = suspended(true);
    if let Some(m) = &mut b.marker {
        m.this_boot = true;
    }
    b
}

/// Whether "Resume BitLocker" is on screen or offered on `page`.
fn resume_on(hub: &mut Hub, page: HubAction) -> bool {
    hub.press(HubAction::Home);
    if page != HubAction::Home {
        hub.press(page);
    }
    let shown = offered(hub, HubAction::Resume);
    hub.press(HubAction::Home);
    shown
}

#[test]
fn right_after_suspending_resume_is_offered_nowhere() {
    let mut hub = with(true, protected());
    show(&mut hub, [1, 1]);
    hub.press(HubAction::KeptIt);
    type_digits(&mut hub, GROUPS[1]);
    type_digits(&mut hub, GROUPS[3]);
    hub.press(HubAction::Suspend);
    let _ = commands(&mut hub);
    hub.hub_event(HubEvent::BitLocker(Some(suspended_now())));
    hub.hub_event(HubEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Ok(()),
    });
    hub.press(HubAction::BitLocker);
    assert!(!offered(&mut hub, HubAction::Resume));
    assert!(!resume_on(&mut hub, HubAction::Home));
    assert!(!resume_on(&mut hub, HubAction::Start));
    // The hub app is opened again in the same boot: still not offered.
    let mut hub = with(true, suspended_now());
    hub.press(HubAction::BitLocker);
    assert!(!offered(&mut hub, HubAction::Resume));
    assert!(shows(&mut hub, "Needs Windows restarted first"));
    assert!(!resume_on(&mut hub, HubAction::Home));
    // Windows started again, Secure Boot as recorded: offered.
    let mut hub = with(true, suspended(true));
    hub.press(HubAction::BitLocker);
    assert!(offered(&mut hub, HubAction::Resume));
}
