//! The BitLocker step as a person meets it (`docs/STARTER.md` §6, §11, F17):
//! by default the recovery key in hand, which opens the firmware steps
//! and changes nothing on the computer; on the extra-safe path, the key
//! shown, kept and confirmed with it hidden before anything is
//! suspended, and the firmware steps closed until both parts are done;
//! the cases the guard cannot pass; Resume, and Uninstall while
//! suspended. The shell's answers are made up here as the elevated copy
//! gives them.

use faraday_qr::bitlocker::{KeyText, RecoveryKey};
use faraday_starter::api::{
    BitLocker, BitLockerStep, Bus, Computer, Disk, FaradayStick, InstallItem, Installed, KeyShown,
    Marker, Os, Protection, Reason, StarterCommand, StarterEvent, Volume,
};
use faraday_starter::{Page, Starter, StarterAction};
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

fn with(secure_boot: bool, b: BitLocker) -> Starter {
    let mut starter = Starter::new();
    starter.event(display(1280, 800, 160));
    starter.starter_event(StarterEvent::Computer(windows(secure_boot)));
    starter.starter_event(StarterEvent::BitLocker(Some(b)));
    starter.starter_event(StarterEvent::Disks(Vec::new()));
    starter
}

fn texts(starter: &mut Starter) -> Vec<String> {
    starter.drawn_texts()
}

/// Whether `s` is on screen, a line wrapped over several read as one.
fn shows(starter: &mut Starter, s: &str) -> bool {
    texts(starter).join(" ").contains(s)
}

fn offered(starter: &mut Starter, a: StarterAction) -> bool {
    let _ = starter.drawn_texts();
    starter.offers_action(a)
}

fn commands(starter: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| starter.next_command()).collect()
}

/// Whether any digit group of the key is on screen.
fn key_on_screen(starter: &mut Starter) -> bool {
    let t = texts(starter);
    GROUPS.iter().any(|g| t.iter().any(|s| s.contains(g)))
}

/// Whether a firmware step can be taken: Home's card or the Start
/// page's restart.
fn firmware_open(starter: &mut Starter) -> bool {
    let page = starter.page;
    starter.press(StarterAction::Start);
    let open = offered(starter, StarterAction::Restart);
    starter.press(StarterAction::Restart);
    starter.press(StarterAction::RestartNow);
    let sent = commands(starter)
        .iter()
        .any(|c| matches!(c, StarterCommand::RestartIntoFirmware));
    starter.press(StarterAction::Home);
    if page == Page::BitLocker {
        starter.press(StarterAction::BitLocker);
    }
    assert_eq!(open, sent);
    open
}

/// On the extra-safe path, shows the key: the elevation prompt's step,
/// answered.
fn show(starter: &mut Starter, entropy: [u8; 2]) {
    starter.press(StarterAction::BitLocker);
    starter.press(StarterAction::ExtraSafe);
    starter.press(StarterAction::ShowKey);
    assert_eq!(
        commands(starter),
        vec![StarterCommand::BitLocker(BitLockerStep::ShowKey)]
    );
    starter.starter_event(StarterEvent::KeyShown(Ok(key())));
    // The Starter asked for random bytes to choose the groups from.
    let mut asked = false;
    while let Some(c) = starter.poll_command() {
        asked |= c == osk_shell_api::Command::RequestEntropy;
    }
    assert!(asked);
    let mut bytes = [0u8; 32];
    bytes[..2].copy_from_slice(&entropy);
    starter.event(Event::Entropy(EntropyBytes::new(bytes)));
}

fn type_digits(starter: &mut Starter, s: &str) {
    for c in s.chars() {
        starter.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn the_recovery_key_in_hand_opens_the_firmware_steps_and_changes_nothing() {
    let mut starter = with(true, protected());
    assert!(!firmware_open(&mut starter));
    starter.press(StarterAction::BitLocker);
    assert!(shows(&mut starter, "Drive C: is encrypted with BitLocker"));
    assert!(shows(&mut starter, "Also kept in"));
    assert!(shows(
        &mut starter,
        "Microsoft account: aka.ms/myrecoverykey"
    ));
    assert!(offered(&mut starter, StarterAction::ShowKey));
    assert!(offered(&mut starter, StarterAction::ExtraSafe));
    assert!(!offered(&mut starter, StarterAction::Suspend));
    // Pressed without showing the key first: no elevation prompt, no
    // suspension, nothing sent to the shell at all.
    starter.press(StarterAction::KeyInHand);
    assert!(commands(&mut starter).is_empty());
    assert!(shows(&mut starter, "Recovery key kept"));
    assert!(!offered(&mut starter, StarterAction::KeyInHand));
    assert!(firmware_open(&mut starter));
    // Home's checklist says so.
    starter.press(StarterAction::Home);
    assert!(shows(&mut starter, "Recovery key kept"));
}

#[test]
fn the_key_is_shown_on_the_default_path_without_typing_back() {
    let mut starter = with(true, protected());
    starter.press(StarterAction::BitLocker);
    starter.press(StarterAction::ShowKey);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::BitLocker(BitLockerStep::ShowKey)]
    );
    starter.starter_event(StarterEvent::KeyShown(Ok(key())));
    assert!(key_on_screen(&mut starter));
    assert!(shows(&mut starter, "Or take a photo of this screen."));
    assert!(offered(&mut starter, StarterAction::KeyInHand));
    // No groups asked for, nothing hidden, nothing to suspend.
    assert!(!offered(&mut starter, StarterAction::KeptIt));
    starter.press(StarterAction::KeptIt);
    assert!(key_on_screen(&mut starter));
    assert!(!shows(&mut starter, "Type groups"));
    starter.press(StarterAction::Suspend);
    assert!(commands(&mut starter).is_empty());
    // In hand: the key leaves the screen and the firmware steps open.
    starter.press(StarterAction::KeyInHand);
    assert!(!key_on_screen(&mut starter));
    assert!(commands(&mut starter).is_empty());
    assert!(firmware_open(&mut starter));
}

#[test]
fn extra_safe_leads_to_the_suspension_path() {
    let mut starter = with(true, protected());
    starter.press(StarterAction::BitLocker);
    starter.press(StarterAction::ExtraSafe);
    assert!(!offered(&mut starter, StarterAction::KeyInHand));
    starter.press(StarterAction::KeyInHand);
    assert!(!firmware_open(&mut starter));
    show(&mut starter, [3, 2]);
    assert!(offered(&mut starter, StarterAction::KeptIt));
    starter.press(StarterAction::KeptIt);
    assert!(shows(&mut starter, "Type groups 4 and 7 of the key"));
    // Left, the page opens on the default step again.
    starter.press(StarterAction::Home);
    starter.press(StarterAction::BitLocker);
    assert!(offered(&mut starter, StarterAction::KeyInHand));
}

#[test]
fn the_key_is_confirmed_with_it_hidden_before_bitlocker_is_suspended() {
    let mut starter = with(true, protected());
    starter.press(StarterAction::BitLocker);
    starter.press(StarterAction::ExtraSafe);
    assert!(shows(&mut starter, "Drive C: is encrypted with BitLocker"));
    assert!(offered(&mut starter, StarterAction::ShowKey));
    assert!(!offered(&mut starter, StarterAction::Suspend));

    show(&mut starter, [3, 2]);
    // The key, its QR code's line and its ID.
    assert!(key_on_screen(&mut starter));
    assert!(shows(&mut starter, "Or take a photo of this screen."));
    assert!(shows(&mut starter, "6A0E1F2C"));
    assert!(offered(&mut starter, StarterAction::KeptIt));
    assert!(!offered(&mut starter, StarterAction::Suspend));

    // Kept: the key is hidden, and groups 4 and 7 are asked for.
    starter.press(StarterAction::KeptIt);
    assert!(!key_on_screen(&mut starter));
    assert!(shows(&mut starter, "Type groups 4 and 7 of the key"));
    type_digits(&mut starter, GROUPS[3]);
    type_digits(&mut starter, GROUPS[6]);
    // Only what was typed is on screen, never the rest of the key.
    assert!(!texts(&mut starter).iter().any(|t| t.contains(GROUPS[0])));
    starter.press(StarterAction::Suspend);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::BitLocker(BitLockerStep::Suspend)]
    );
    assert!(shows(&mut starter, "Suspending"));
}

#[test]
fn a_wrong_group_is_refused_and_nothing_is_suspended() {
    let mut starter = with(true, protected());
    show(&mut starter, [0, 0]);
    starter.press(StarterAction::KeptIt);
    // Groups 1 and 2.
    assert!(shows(&mut starter, "Type groups 1 and 2 of the key"));
    type_digits(&mut starter, GROUPS[0]);
    type_digits(&mut starter, "492018");
    starter.event(Event::Key(Key::Enter));
    assert!(commands(&mut starter).is_empty());
    assert!(shows(&mut starter, "Group 2 does not match the key."));
    // Another group of the key is not the one asked for.
    starter.event(Event::Key(Key::Backspace));
    type_digits(&mut starter, "9");
    starter.event(Event::Key(Key::Tab));
    for _ in 0..6 {
        starter.event(Event::Key(Key::Backspace));
    }
    type_digits(&mut starter, GROUPS[2]);
    starter.press(StarterAction::Suspend);
    assert!(commands(&mut starter).is_empty());
    assert!(shows(&mut starter, "Group 1 does not match the key."));
    starter.press(StarterAction::Group(0));
    for _ in 0..6 {
        starter.event(Event::Key(Key::Backspace));
    }
    type_digits(&mut starter, GROUPS[0]);
    starter.press(StarterAction::Suspend);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::BitLocker(BitLockerStep::Suspend)]
    );
}

#[test]
fn suspend_is_never_sent_before_the_key_is_confirmed() {
    let mut starter = with(true, protected());
    starter.press(StarterAction::BitLocker);
    starter.press(StarterAction::ExtraSafe);
    // No key yet.
    starter.press(StarterAction::Suspend);
    assert!(commands(&mut starter).is_empty());
    // The key shown, not yet kept: typing does nothing, Suspend nothing.
    show(&mut starter, [5, 1]);
    type_digits(&mut starter, KEY);
    starter.event(Event::Key(Key::Enter));
    starter.press(StarterAction::Suspend);
    assert!(commands(&mut starter).is_empty());
    // Kept, nothing typed.
    starter.press(StarterAction::KeptIt);
    starter.press(StarterAction::Suspend);
    assert!(commands(&mut starter).is_empty());
    // Shown again: the groups typed are gone, and so is Suspend.
    starter.press(StarterAction::ShowAgain);
    assert!(key_on_screen(&mut starter));
    starter.press(StarterAction::Suspend);
    assert!(commands(&mut starter).is_empty());
    // Leaving the page forgets the key: it is shown again from the
    // elevation prompt.
    starter.press(StarterAction::Home);
    starter.press(StarterAction::BitLocker);
    assert!(!key_on_screen(&mut starter));
    assert!(offered(&mut starter, StarterAction::ShowKey));
}

#[test]
fn on_the_extra_safe_path_no_firmware_step_before_both_parts_are_done() {
    let mut starter = with(true, protected());
    assert!(!firmware_open(&mut starter));
    show(&mut starter, [1, 1]);
    assert!(!firmware_open(&mut starter));
    starter.press(StarterAction::BitLocker);
    show(&mut starter, [1, 1]);
    starter.press(StarterAction::KeptIt);
    let asked = [1, 3];
    type_digits(&mut starter, GROUPS[asked[0]]);
    type_digits(&mut starter, GROUPS[asked[1]]);
    starter.press(StarterAction::Suspend);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::BitLocker(BitLockerStep::Suspend)]
    );
    // Sent, not yet answered.
    assert!(!firmware_open(&mut starter));
    // Suspended, the marker written: the firmware steps open, and the
    // next step is still Start in this session.
    starter.press(StarterAction::BitLocker);
    starter.starter_event(StarterEvent::BitLocker(Some(suspended(true))));
    starter.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Ok(()),
    });
    assert!(shows(
        &mut starter,
        "Windows starts without the key until BitLocker is resumed. Resume it here once Secure Boot is back on."
    ));
    assert!(shows(&mut starter, "Open Faraday again to do that."));
    assert!(!key_on_screen(&mut starter));
    assert!(firmware_open(&mut starter));
}

#[test]
fn a_refused_suspension_shows_the_managed_line_and_keeps_the_firmware_closed() {
    let mut starter = with(true, protected());
    show(&mut starter, [2, 0]);
    starter.press(StarterAction::KeptIt);
    type_digits(&mut starter, GROUPS[2]);
    type_digits(&mut starter, GROUPS[3]);
    starter.press(StarterAction::Suspend);
    let _ = commands(&mut starter);
    starter.starter_event(StarterEvent::BitLocker(Some(protected())));
    starter.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Err(Reason::Managed(0x8031_0050)),
    });
    assert!(shows(
        &mut starter,
        "BitLocker was not suspended (0x80310050). This computer's BitLocker is managed: its administrator has to suspend it."
    ));
    assert!(!firmware_open(&mut starter));
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
        let mut starter = with(true, protected());
        starter.press(StarterAction::BitLocker);
        starter.press(StarterAction::ShowKey);
        assert!(shows(&mut starter, "Waiting for administrator permission"));
        starter.starter_event(StarterEvent::KeyShown(Err(why)));
        assert!(shows(&mut starter, says));
        assert!(offered(&mut starter, StarterAction::ShowKey));
        assert!(!firmware_open(&mut starter));
    }
}

#[test]
fn save_is_offered_only_with_a_removable_disk_that_is_not_a_faraday_stick() {
    let mut starter = with(true, protected());
    show(&mut starter, [0, 1]);
    assert!(!shows(&mut starter, "Save to"));
    // A Faraday stick, a disk the computer uses, a disk inside it, an
    // empty one: none.
    let mut stick = usb("1", true);
    stick.faraday = Some(FaradayStick { release: None });
    let mut used = usb("2", true);
    used.in_use = Some(faraday_starter::api::InUse::System);
    let mut inside = usb("3", true);
    inside.bus = Bus::Other;
    let blank = usb("4", false);
    starter.starter_event(StarterEvent::Disks(vec![stick, used, inside, blank]));
    assert!(!shows(&mut starter, "Save to"));
    // A USB disk with a file system: offered, and Save names it.
    starter.starter_event(StarterEvent::Disks(vec![usb("5", true)]));
    assert!(shows(&mut starter, "Save to Kingston DataTraveler"));
    let n = StarterAction::SaveKey(starter.disk_number("5"));
    starter.press(n);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::SaveKey {
            disk: "5".into(),
            key: key()
        }]
    );
    starter.starter_event(StarterEvent::KeySaved(Ok(
        r"E:\BitLocker recovery key 6A0E1F2C.txt".into(),
    )));
    assert!(shows(
        &mut starter,
        r"Saved to E:\BitLocker recovery key 6A0E1F2C.txt"
    ));
    // Hidden, there is nothing to save.
    starter.press(StarterAction::KeptIt);
    starter.press(n);
    assert!(commands(&mut starter).is_empty());
}

#[test]
fn resume_is_offered_only_with_the_marker_and_secure_boot_as_it_recorded() {
    // Secure Boot on, as recorded: Resume on the page, sent once.
    let mut starter = with(true, suspended(true));
    starter.press(StarterAction::BitLocker);
    assert!(offered(&mut starter, StarterAction::Resume));
    starter.press(StarterAction::Resume);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::BitLocker(BitLockerStep::Resume)]
    );
    assert!(shows(&mut starter, "Resuming"));
    starter.starter_event(StarterEvent::BitLocker(Some(protected())));
    starter.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Resume,
        result: Ok(()),
    });
    assert!(shows(&mut starter, "BitLocker resumed"));
    // Resumed, the drive is protected again: the firmware steps need the
    // guard again.
    assert!(!firmware_open(&mut starter));

    // Secure Boot off, recorded on: not offered, and the line says why.
    let mut starter = with(false, suspended(true));
    starter.press(StarterAction::BitLocker);
    assert!(!offered(&mut starter, StarterAction::Resume));
    assert!(shows(&mut starter, "Needs Secure Boot on"));
    starter.press(StarterAction::Resume);
    assert!(commands(&mut starter).is_empty());

    // No marker: nothing of the Starter's to resume.
    let mut starter = with(true, protected());
    starter.press(StarterAction::BitLocker);
    assert!(!offered(&mut starter, StarterAction::Resume));
}

#[test]
fn a_suspension_undone_later_offers_the_step_again() {
    // Protected again, the marker still there.
    let mut b = suspended(true);
    b.protection = Protection::On;
    let mut starter = with(true, b);
    assert!(shows(&mut starter, "Protected"));
    starter.press(StarterAction::BitLocker);
    assert!(shows(
        &mut starter,
        "Protected again after Faraday suspended it"
    ));
    assert!(offered(&mut starter, StarterAction::ShowKey));
    assert!(!offered(&mut starter, StarterAction::Resume));
    assert!(!firmware_open(&mut starter));
}

#[test]
fn a_drive_suspended_by_someone_else_still_needs_the_guard() {
    let mut b = protected();
    b.protection = Protection::Suspended;
    let mut starter = with(true, b);
    assert!(!firmware_open(&mut starter));
    // Home's checklist says what the drive is, not "Protected".
    assert!(shows(&mut starter, "Suspended"));
    assert!(!shows(&mut starter, "Protected"));
    starter.press(StarterAction::BitLocker);
    assert!(offered(&mut starter, StarterAction::ShowKey));
}

#[test]
fn another_encryption_product_closes_the_firmware_until_its_rescue_is_confirmed() {
    let mut starter = with(
        true,
        BitLocker {
            drive: "C:".into(),
            protection: Protection::None,
            marker: None,
        },
    );
    assert!(firmware_open(&mut starter));
    starter.starter_event(StarterEvent::OtherEncryption(Some("VeraCrypt".into())));
    assert!(!firmware_open(&mut starter));
    assert!(shows(&mut starter, "Not managed by Faraday"));
    starter.press(StarterAction::BitLocker);
    assert!(shows(
        &mut starter,
        "An encrypted drive Faraday does not manage: VeraCrypt"
    ));
    starter.press(StarterAction::Rescue);
    assert!(firmware_open(&mut starter));
}

fn installed() -> Option<Installed> {
    Some(Installed {
        version: "0.2.0".into(),
        items: vec![
            (
                InstallItem::App,
                r"%LOCALAPPDATA%\Programs\Faraday Starter\Faraday Starter.exe".into(),
            ),
            (
                InstallItem::Reminder,
                r"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run\Faraday Starter"
                    .into(),
            ),
        ],
    })
}

#[test]
fn uninstall_while_suspended_offers_resume_with_secure_boot_on() {
    let mut starter = with(true, suspended(true));
    starter.starter_event(StarterEvent::Installed(installed()));
    starter.press(StarterAction::Installed);
    starter.press(StarterAction::Uninstall);
    assert!(shows(
        &mut starter,
        "BitLocker stays suspended until it is resumed."
    ));
    assert!(offered(&mut starter, StarterAction::Resume));
    starter.press(StarterAction::Resume);
    assert_eq!(
        commands(&mut starter),
        vec![StarterCommand::BitLocker(BitLockerStep::Resume)]
    );
    assert_eq!(starter.page, Page::BitLocker);
    assert!(starter.sheet.is_none());
}

#[test]
fn uninstall_while_suspended_with_secure_boot_off_says_it_stays_suspended() {
    let mut starter = with(false, suspended(true));
    starter.starter_event(StarterEvent::Installed(installed()));
    starter.press(StarterAction::Installed);
    starter.press(StarterAction::Uninstall);
    assert!(shows(
        &mut starter,
        "BitLocker stays suspended until it is resumed: Control Panel, BitLocker Drive Encryption, Resume protection."
    ));
    assert!(!offered(&mut starter, StarterAction::Resume));
    starter.press(StarterAction::UninstallNow);
    assert_eq!(commands(&mut starter), vec![StarterCommand::Uninstall]);
}

#[test]
fn an_installed_starter_does_not_say_open_it_again() {
    let mut starter = with(true, suspended(true));
    starter.starter_event(StarterEvent::Installed(installed()));
    starter.press(StarterAction::BitLocker);
    assert!(shows(
        &mut starter,
        "Windows starts without the key until BitLocker is resumed."
    ));
    assert!(!shows(&mut starter, "Open Faraday again to do that."));
}

#[test]
fn the_guard_fits_a_small_panel() {
    let mut starter = Starter::new();
    starter.event(display(480, 640, 286));
    starter.starter_event(StarterEvent::Computer(windows(true)));
    starter.starter_event(StarterEvent::BitLocker(Some(protected())));
    show(&mut starter, [6, 6]);
    assert!(key_on_screen(&mut starter));
    // I have kept it is under the code: scrolled to.
    let _ = starter.frame();
    for _ in 0..20 {
        starter.event(Event::Wheel {
            x: 10,
            y: 10,
            dy: 2000,
        });
        let _ = starter.frame();
    }
    assert!(offered(&mut starter, StarterAction::KeptIt));
    starter.press(StarterAction::KeptIt);
    assert!(!key_on_screen(&mut starter));
}

/// The key step answered `why` with the drive protected as the shell
/// reads it (2 on a shell reading is not trusted: not known).
fn key_step_answers(starter: &mut Starter, why: Reason) {
    starter.press(StarterAction::BitLocker);
    starter.press(StarterAction::ShowKey);
    let _ = commands(starter);
    starter.starter_event(StarterEvent::KeyShown(Err(why)));
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
    let mut starter = with(true, unknown());
    assert!(!firmware_open(&mut starter));
    starter.press(StarterAction::BitLocker);
    assert!(offered(&mut starter, StarterAction::ShowKey));
}

#[test]
fn the_guard_reading_not_encrypted_opens_the_firmware_steps_for_the_session() {
    let mut starter = with(true, unknown());
    assert!(!firmware_open(&mut starter));
    key_step_answers(&mut starter, Reason::NotEncrypted);
    assert!(shows(&mut starter, "Drive C: not encrypted"));
    assert!(firmware_open(&mut starter));
    // The shell reads the drive as not known again: still open.
    starter.starter_event(StarterEvent::BitLocker(Some(unknown())));
    assert!(firmware_open(&mut starter));
}

#[test]
fn the_guard_reading_not_encrypted_opens_them_when_no_reading_was_made() {
    let mut starter = Starter::new();
    starter.event(display(1280, 800, 160));
    starter.starter_event(StarterEvent::Computer(windows(true)));
    starter.starter_event(StarterEvent::BitLocker(None));
    starter.starter_event(StarterEvent::Disks(Vec::new()));
    assert!(!firmware_open(&mut starter));
    key_step_answers(&mut starter, Reason::NotEncrypted);
    assert!(firmware_open(&mut starter));
}

#[test]
fn a_drive_windows_does_not_list_keeps_the_firmware_closed_and_says_so() {
    let mut starter = with(true, unknown());
    key_step_answers(&mut starter, Reason::NotRead);
    assert!(shows(
        &mut starter,
        "BitLocker on the system drive could not be read. The drive is treated as protected."
    ));
    assert!(!firmware_open(&mut starter));
    // Any other failure keeps it closed too.
    key_step_answers(
        &mut starter,
        Reason::BitLocker {
            call: "GetKeyProtectors".into(),
            code: 0x8031_0008,
        },
    );
    assert!(!firmware_open(&mut starter));
}

fn suspended_now() -> BitLocker {
    let mut b = suspended(true);
    if let Some(m) = &mut b.marker {
        m.this_boot = true;
    }
    b
}

/// Whether "Resume BitLocker" is on screen or offered on `page`.
fn resume_on(starter: &mut Starter, page: StarterAction) -> bool {
    starter.press(StarterAction::Home);
    if page != StarterAction::Home {
        starter.press(page);
    }
    let shown = offered(starter, StarterAction::Resume);
    starter.press(StarterAction::Home);
    shown
}

#[test]
fn right_after_suspending_resume_is_offered_nowhere() {
    let mut starter = with(true, protected());
    show(&mut starter, [1, 1]);
    starter.press(StarterAction::KeptIt);
    type_digits(&mut starter, GROUPS[1]);
    type_digits(&mut starter, GROUPS[3]);
    starter.press(StarterAction::Suspend);
    let _ = commands(&mut starter);
    starter.starter_event(StarterEvent::BitLocker(Some(suspended_now())));
    starter.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Ok(()),
    });
    starter.press(StarterAction::BitLocker);
    assert!(!offered(&mut starter, StarterAction::Resume));
    assert!(!resume_on(&mut starter, StarterAction::Home));
    assert!(!resume_on(&mut starter, StarterAction::Start));
    // The Starter is opened again in the same boot: still not offered.
    let mut starter = with(true, suspended_now());
    starter.press(StarterAction::BitLocker);
    assert!(!offered(&mut starter, StarterAction::Resume));
    assert!(shows(&mut starter, "Needs Windows restarted first"));
    assert!(!resume_on(&mut starter, StarterAction::Home));
    // Windows started again, Secure Boot as recorded: offered.
    let mut starter = with(true, suspended(true));
    starter.press(StarterAction::BitLocker);
    assert!(offered(&mut starter, StarterAction::Resume));
}

/// A group's field with "2056" typed into it, the first of the two
/// asked for (`show(&mut Starter, [0, 0])` asks for groups 1 and 2).
fn field_typed() -> Starter {
    let mut starter = with(true, protected());
    show(&mut starter, [0, 0]);
    starter.press(StarterAction::KeptIt);
    type_digits(&mut starter, "2056");
    starter
}

#[test]
fn ctrl_a_then_typing_replaces_the_field() {
    let mut starter = field_typed();
    starter.event(Event::Key(Key::SelectAll));
    type_digits(&mut starter, "9");
    assert_eq!(starter.guard.typed(0), "9");
}

#[test]
fn ctrl_a_then_backspace_clears_the_field() {
    let mut starter = field_typed();
    starter.event(Event::Key(Key::SelectAll));
    starter.event(Event::Key(Key::Backspace));
    assert_eq!(starter.guard.typed(0), "");
}

#[test]
fn ctrl_a_selects_only_the_focused_field() {
    let mut starter = field_typed();
    // Filling the field moves on to the second one, asked for too.
    type_digits(&mut starter, "68");
    type_digits(&mut starter, "4920");
    starter.event(Event::Key(Key::SelectAll));
    type_digits(&mut starter, "9");
    assert_eq!(starter.guard.typed(1), "9");
    assert_eq!(
        starter.guard.typed(0),
        "205668",
        "selecting the second field leaves the first as it was"
    );
}

#[test]
fn home_then_typing_puts_the_text_first() {
    let mut starter = field_typed();
    starter.event(Event::Key(Key::Home));
    type_digits(&mut starter, "9");
    assert_eq!(starter.guard.typed(0), "92056");
}

#[test]
fn end_after_home_types_at_the_end() {
    let mut starter = field_typed();
    starter.event(Event::Key(Key::Home));
    starter.event(Event::Key(Key::End));
    type_digits(&mut starter, "9");
    assert_eq!(starter.guard.typed(0), "20569");
}

#[test]
fn home_and_end_stay_within_the_field() {
    let mut starter = field_typed();
    starter.event(Event::Key(Key::Home));
    assert_eq!(starter.guard.focus, 0);
    starter.event(Event::Key(Key::End));
    assert_eq!(starter.guard.focus, 0);
    // Backspace at the start of a field that still has text in it does
    // nothing: it never jumps to the other field, only an empty field
    // does that.
    starter.event(Event::Key(Key::Home));
    starter.event(Event::Key(Key::Backspace));
    assert_eq!(starter.guard.focus, 0);
    assert_eq!(starter.guard.typed(0), "2056");
}
