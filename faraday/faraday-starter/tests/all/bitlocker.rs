//! BitLocker on the Start page (`docs/STARTER.md` §3.4, §6, F49): on
//! Windows with BitLocker on the system drive, the page offers, never
//! requires, to suspend it for the next three times Windows starts, to
//! resume it, and to put its recovery key on the printed sheet when the
//! person ticks it. With BitLocker off or not on Windows, none of it
//! shows; not read shows "Not read". The key is never drawn and never a
//! QR code.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};
use faraday_starter::api::{
    BitLocker, BitLockerStep, Bus, Computer, Disk, FaradayStick, Os, Protection, Reason,
    RecoveryKey, StarterCommand, StarterEvent,
};
use faraday_starter::notes::Notes;
use faraday_starter::{Page, Starter, StarterAction};

/// A token for the PDFs these tests make, counting nothing. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: the Starter's tests, which draw nothing"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

const ID: &str = "{6A0E1F2C-3B4D-4E5F-8A9B-0C1D2E3F4A5B}";
const KEY: &str = "205568-492019-108251-336446-650375-210331-584155-047520";
const SUSPEND: &str = "Temporarily suspend BitLocker";
const RESUME: &str = "Resume BitLocker";
const TICK: &str = "Include the BitLocker recovery key";
const ROW: &str = "BitLocker, drive C:";
const SUSPENDED_3: &str = "Suspended · turns back on by itself after Windows starts 3 times";

fn dell(os: Os) -> Computer {
    Computer {
        os,
        maker: "Dell Inc.".into(),
        model: "Latitude 5440".into(),
        family: "Latitude".into(),
        uefi: true,
        arm: false,
        secure_boot: Some(true),
        tpm_unlock: false,
    }
}

fn drive(protection: Protection, starts_left: Option<u32>) -> Option<BitLocker> {
    Some(BitLocker {
        drive: "C:".into(),
        protection,
        starts_left,
    })
}

/// A Faraday stick Windows lists, for the restart from the stick.
fn faraday_stick() -> Disk {
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
        fixed: false,
        faraday: Some(FaradayStick {
            release: Some("6.6.84-faraday-0.2.0".into()),
            read: true,
            boot_sha256: None,
        }),
    }
}

fn key() -> RecoveryKey {
    RecoveryKey::parse("C:", ID, KEY).expect("a key")
}

/// The commands the Starter asked for since the last call.
fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

/// The Home page of a computer running `os`, no stick attached, nothing
/// asked of the shell yet.
fn home(os: Os) -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 4000,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    s.starter_event(StarterEvent::Computer(dell(os)));
    s.starter_event(StarterEvent::Disks(Vec::new()));
    let _ = commands(&mut s);
    s
}

/// On the Start page of a computer running `os`, with what Windows reads
/// of BitLocker answered.
fn start_page(os: Os, read: Option<BitLocker>) -> Starter {
    let mut s = home(os);
    s.press(StarterAction::Start);
    assert_eq!(s.page, Page::Start);
    let asked = commands(&mut s);
    assert_eq!(
        asked.contains(&StarterCommand::ReadBitLocker),
        os == Os::Windows,
        "{asked:?}"
    );
    s.starter_event(StarterEvent::BitLocker(read));
    s
}

fn texts(s: &mut Starter) -> Vec<String> {
    s.drawn_texts()
}

fn shows(s: &mut Starter, text: &str) -> bool {
    texts(s).iter().any(|t| t.contains(text))
}

fn position(s: &mut Starter, text: &str) -> Option<usize> {
    texts(s).iter().position(|t| t.contains(text))
}

/// The value drawn right after a row's label.
fn value_after(s: &mut Starter, label: &str) -> String {
    let t = texts(s);
    let i = t
        .iter()
        .position(|x| x == label)
        .unwrap_or_else(|| panic!("no row {label}: {t:?}"));
    t[i + 1].clone()
}

/// Whether the PDF's page draws `text` (its streams are not compressed;
/// each word is enough to find, a wrapped line being split at spaces).
fn pdf_has(pdf: &[u8], text: &str) -> bool {
    text.split(' ')
        .filter(|w| !w.is_empty())
        .all(|w| pdf.windows(w.len()).any(|x| x == w.as_bytes()))
}

/// Print, and the sheet the shell is handed: its PDF, and whether it is
/// marked as carrying the key.
fn printed(s: &mut Starter) -> (Vec<u8>, bool) {
    let _ = commands(s);
    s.press(StarterAction::PrintNotes);
    match commands(s).as_slice() {
        [StarterCommand::PrintNotes { pdf, with_key, .. }] => (pdf.to_vec(), *with_key),
        other => panic!("one PrintNotes expected: {other:?}"),
    }
}

#[test]
fn with_bitlocker_on_the_start_page_offers_suspend_and_the_key_on_the_sheet() {
    let mut s = start_page(Os::Windows, drive(Protection::On, None));
    assert_eq!(value_after(&mut s, ROW), "On");
    assert!(shows(&mut s, SUSPEND));
    assert!(shows(&mut s, "For the next 3 times Windows starts"));
    assert!(shows(&mut s, TICK));
    assert!(!shows(&mut s, RESUME));
    // Between Write down and the restart: offered before it, never a
    // step of its own.
    let notes = position(&mut s, "Write down").expect("Write down");
    let row = position(&mut s, ROW).expect("the row");
    let restart = position(&mut s, "Restart into the BIOS").expect("the restart");
    assert!(notes < row && row < restart);
    // Nothing waits on it: the restart is offered as without BitLocker.
    let _ = commands(&mut s);
    s.press(StarterAction::Restart);
    assert!(s.sheet.is_some());
}

#[test]
fn suspend_asks_once_then_says_it_turns_back_on_by_itself_and_offers_resume() {
    let mut s = start_page(Os::Windows, drive(Protection::On, None));
    s.press(StarterAction::SuspendBitLocker);
    assert_eq!(
        commands(&mut s),
        [StarterCommand::BitLocker(BitLockerStep::Suspend)]
    );
    assert!(shows(&mut s, "Waiting for administrator permission"));
    // A second press while the prompt is open asks nothing more.
    s.press(StarterAction::SuspendBitLocker);
    assert!(commands(&mut s).is_empty());
    s.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Ok(()),
    });
    s.starter_event(StarterEvent::BitLocker(drive(
        Protection::Suspended,
        Some(3),
    )));
    assert_eq!(value_after(&mut s, ROW), SUSPENDED_3);
    assert!(shows(&mut s, RESUME));
    assert!(!shows(&mut s, SUSPEND));
    assert!(!shows(&mut s, "Waiting for administrator permission"));

    // Shown again: Windows read without elevation has no count, and the
    // one read when it was suspended stands.
    s.press(StarterAction::Home);
    s.press(StarterAction::Start);
    assert!(commands(&mut s).contains(&StarterCommand::ReadBitLocker));
    s.starter_event(StarterEvent::BitLocker(drive(Protection::Suspended, None)));
    assert_eq!(value_after(&mut s, ROW), SUSPENDED_3);
}

#[test]
fn a_suspension_read_without_its_count_says_suspended_and_offers_resume() {
    let mut s = start_page(Os::Windows, drive(Protection::Suspended, None));
    assert_eq!(value_after(&mut s, ROW), "Suspended");
    assert!(shows(&mut s, RESUME));
    let mut s = start_page(Os::Windows, drive(Protection::Suspended, Some(1)));
    assert_eq!(
        value_after(&mut s, ROW),
        "Suspended · turns back on by itself after Windows starts once"
    );
    let mut s = start_page(Os::Windows, drive(Protection::Suspended, Some(0)));
    assert_eq!(value_after(&mut s, ROW), "Suspended until resumed");
}

#[test]
fn resume_returns_to_on_and_suspend_is_offered_again() {
    let mut s = start_page(Os::Windows, drive(Protection::Suspended, Some(3)));
    let _ = commands(&mut s);
    s.press(StarterAction::ResumeBitLocker);
    assert_eq!(
        commands(&mut s),
        [StarterCommand::BitLocker(BitLockerStep::Resume)]
    );
    s.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Resume,
        result: Ok(()),
    });
    s.starter_event(StarterEvent::BitLocker(drive(Protection::On, None)));
    assert_eq!(value_after(&mut s, ROW), "On");
    assert!(shows(&mut s, SUSPEND));
    assert!(!shows(&mut s, RESUME));
    // Resume pressed with protection on does nothing.
    s.press(StarterAction::ResumeBitLocker);
    assert!(commands(&mut s).is_empty());
}

#[test]
fn a_refused_step_says_why_and_the_drive_stays_as_read() {
    let mut s = start_page(Os::Windows, drive(Protection::On, None));
    s.press(StarterAction::SuspendBitLocker);
    s.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Err(Reason::Managed(0x8031_003A)),
    });
    s.starter_event(StarterEvent::BitLocker(drive(Protection::On, None)));
    assert!(shows(&mut s, "BitLocker was not suspended (0x8031003A)"));
    assert_eq!(value_after(&mut s, ROW), "On");
    s.starter_event(StarterEvent::BitLockerDone {
        step: BitLockerStep::Suspend,
        result: Err(Reason::NoAdmin),
    });
    assert!(shows(
        &mut s,
        "An administrator of this computer has to do this step."
    ));
}

#[test]
fn bitlocker_steps_and_key_show_only_when_read_on_on_windows() {
    for (os, read) in [
        (Os::Windows, drive(Protection::None, None)),
        (Os::Windows, drive(Protection::Unknown, None)),
        (Os::Windows, None),
        (Os::Linux, None),
    ] {
        let mut s = start_page(os, read.clone());
        let unread = os == Os::Windows
            && read
                .as_ref()
                .is_none_or(|b| b.protection == Protection::Unknown);
        assert_eq!(shows(&mut s, "Not read"), unread, "{os:?} {read:?}");
        assert!(
            unread || !shows(&mut s, "BitLocker, drive"),
            "{os:?} {read:?}"
        );
        for t in [SUSPEND, RESUME, TICK] {
            assert!(!shows(&mut s, t), "{os:?} {read:?}: {t}");
        }
        let _ = commands(&mut s);
        s.press(StarterAction::SuspendBitLocker);
        s.press(StarterAction::IncludeKey);
        assert!(commands(&mut s).is_empty());
        let (pdf, with_key) = printed(&mut s);
        assert!(!with_key);
        assert!(!pdf_has(&pdf, "recovery"));
    }
}

#[test]
fn the_sheet_carries_the_key_only_when_ticked() {
    let c = compute();
    let mut s = start_page(Os::Windows, drive(Protection::On, None));
    assert!(!s.include_key);
    // Unticked, as it starts: the sheet at once, without the key.
    let (pdf, with_key) = printed(&mut s);
    assert!(!with_key);
    assert!(!pdf_has(&pdf, "BitLocker"));
    assert!(KEY.split('-').all(|g| !pdf_has(&pdf, g)));

    // Ticked: the key is read through the elevated copy first.
    s.press(StarterAction::IncludeKey);
    assert!(s.include_key);
    let _ = commands(&mut s);
    s.press(StarterAction::PrintNotes);
    assert_eq!(commands(&mut s), [StarterCommand::ReadRecoveryKey]);
    assert!(shows(&mut s, "Waiting for administrator permission"));
    s.starter_event(StarterEvent::RecoveryKey(Ok(vec![key()])));
    let sheet = commands(&mut s);
    let [StarterCommand::PrintNotes { pdf, with_key, .. }] = sheet.as_slice() else {
        panic!("one PrintNotes expected: {sheet:?}");
    };
    assert!(*with_key);
    assert!(pdf_has(pdf, "BitLocker recovery key, drive C:"));
    // The eight groups, four to a line.
    assert!(pdf_has(pdf, "205568-492019-108251-336446-"));
    assert!(pdf_has(pdf, "650375-210331-584155-047520"));
    // The rest of the sheet as without it.
    let notes = Notes::for_computer(&dell(Os::Windows)).expect("notes");
    assert_eq!(*pdf, notes.pdf_with_key(&c, &[key()]));
    assert!(pdf_has(pdf, "Boot menu key"));
    // The key is never on the page itself.
    let all = texts(&mut s).join(" ");
    assert!(KEY.split('-').all(|g| !all.contains(g)), "{all}");

    // Unticked again: no key.
    s.press(StarterAction::IncludeKey);
    let (pdf, with_key) = printed(&mut s);
    assert!(!with_key && !pdf_has(&pdf, "205568"));
}

#[test]
fn a_key_that_cannot_be_read_says_why_and_prints_nothing() {
    let mut s = start_page(Os::Windows, drive(Protection::On, None));
    s.press(StarterAction::IncludeKey);
    s.press(StarterAction::PrintNotes);
    let _ = commands(&mut s);
    s.starter_event(StarterEvent::RecoveryKey(Err(Reason::NoRecoveryKey)));
    assert!(commands(&mut s).is_empty());
    assert!(shows(&mut s, "The drive has no BitLocker recovery key."));
}

#[test]
fn the_key_s_sheet_carries_the_key_protector_s_id() {
    let c = compute();
    let pdf = Notes::for_computer(&dell(Os::Windows))
        .expect("notes")
        .pdf_with_key(&c, &[key()]);
    let id = b"6A0E1F2C-3B4D-4E5F-8A9B-0C1D2E3F4A5B";
    assert!(pdf.windows(id.len()).any(|w| w == id));
    let without = Notes::for_computer(&dell(Os::Windows))
        .expect("notes")
        .pdf(&c);
    assert!(!without.windows(id.len()).any(|w| w == id));
}

#[test]
fn the_key_s_sheet_names_nothing_else_no_path_no_computer_and_holds_no_qr_code() {
    let c = compute();
    let pdf = Notes::for_computer(&dell(Os::Windows))
        .expect("notes")
        .pdf_with_key(&c, &[key()]);
    for word in [
        b"/Info".as_slice(),
        b"/Producer",
        b"/Author",
        b"/Title",
        b"/CreationDate",
        b"/home/",
        b"\\Users\\",
        b"AppData",
        b"Latitude",
        b"Dell Inc.",
        b"DESKTOP-",
        b"aka.ms",
        // A protector's ID is printed without braces.
        b"{",
    ] {
        assert!(!pdf.windows(word.len()).any(|w| w == word), "{word:?}");
    }
    // A QR code is drawn as filled squares (`re f`); the sheet has none.
    assert!(!pdf.windows(4).any(|w| w == b"re f"));
}

#[test]
fn a_recovery_key_is_eight_groups_of_six_digits_and_debug_shows_none_of_it() {
    assert!(RecoveryKey::parse("C:", ID, KEY).is_some());
    assert!(RecoveryKey::parse("C", ID, KEY).is_none());
    assert!(RecoveryKey::parse("C:", ID, "205568-492019").is_none());
    assert!(RecoveryKey::parse("C:", ID, &KEY.replace('-', " ")).is_none());
    assert!(RecoveryKey::parse("C:", ID, &KEY.replacen('2', "x", 1)).is_none());
    assert!(RecoveryKey::parse("C:", "6A0E1F2C-3B4D-4E5F-8A9B", KEY).is_none());
    assert!(RecoveryKey::parse("C:", "", KEY).is_none());
    let shown = format!("{:?}", key());
    assert!(!shown.contains("205568"), "{shown}");
}

#[test]
fn a_drive_with_two_recovery_passwords_has_both_on_the_sheet_each_with_its_id() {
    const KEY2: &str = "111111-222222-333333-444444-555555-666666-777777-888888";
    let second = RecoveryKey::parse("C:", "{0F1E2D3C-4B5A-4968-8776-655443322110}", KEY2)
        .expect("a second key");
    let pdf = Notes::for_computer(&dell(Os::Windows))
        .expect("notes")
        .pdf_with_key(&compute(), &[key(), second]);
    for text in [
        "205568-492019-108251-336446-",
        "650375-210331-584155-047520",
        "111111-222222-333333-444444-",
        "555555-666666-777777-888888",
        "6A0E1F2C-3B4D-4E5F-8A9B-0C1D2E3F4A5B",
        "0F1E2D3C-4B5A-4968-8776-655443322110",
    ] {
        assert!(pdf_has(&pdf, text), "{text}");
    }
    // Each key is followed by its own ID: key 1, ID 1, key 2, ID 2.
    let at = |text: &str| {
        pdf.windows(text.len())
            .position(|w| w == text.as_bytes())
            .unwrap_or_else(|| panic!("{text} not on the sheet"))
    };
    let order = [
        at("205568-492019"),
        at("6A0E1F2C-3B4D-4E5F-8A9B-0C1D2E3F4A5B"),
        at("111111-222222"),
        at("0F1E2D3C-4B5A-4968-8776-655443322110"),
    ];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
}

#[test]
fn a_key_read_that_is_empty_says_no_recovery_key_and_prints_nothing() {
    let mut s = start_page(Os::Windows, drive(Protection::On, None));
    s.press(StarterAction::IncludeKey);
    s.press(StarterAction::PrintNotes);
    let _ = commands(&mut s);
    s.starter_event(StarterEvent::RecoveryKey(Ok(Vec::new())));
    assert!(commands(&mut s).is_empty());
    assert!(shows(&mut s, "The drive has no BitLocker recovery key."));
}

/// The restart sheet on a Windows computer whose Secure Boot is off,
/// with BitLocker as read.
fn restart_sheet(read: Option<BitLocker>) -> Starter {
    let mut s = start_page(Os::Windows, read);
    let mut c = dell(Os::Windows);
    c.secure_boot = Some(false);
    s.starter_event(StarterEvent::Computer(c));
    let _ = commands(&mut s);
    s.press(StarterAction::Restart);
    assert!(s.sheet.is_some(), "the restart sheet is up");
    let _ = commands(&mut s);
    s
}

const RESTART_WARNING: &str = "recovery key if it starts with Secure Boot off";
const RESTART_HOLD: &str = "Hold to restart without the recovery key";
const RESTART_UNREAD: &str = "Hold to restart, BitLocker not read";

/// A tap at the restart button, Down and Up, as a finger does.
fn tap(s: &mut Starter) {
    let _ = s.drawn_texts();
    let (x, y) = s.where_offered(StarterAction::RestartNow).expect("offered");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        s.event(Event::Touch { x, y, phase });
    }
}

/// The restart is a hold: a tap does nothing, held it sends `sent`.
fn assert_held(s: &mut Starter, label: &str, sent: StarterCommand) {
    assert!(shows(s, label), "{label}");
    let at = position(s, label).expect("the hold");
    // The sheet's own warning, not the Start page's under it: between the
    // sheet's first line and the hold.
    let top =
        position(s, "This restarts the computer now. Save your work first.").expect("the sheet");
    let warned = texts(s).iter().rposition(|t| t.contains(RESTART_WARNING));
    assert!(
        warned.is_some_and(|w| top < w && w < at),
        "the warning between the sheet's first line and the hold"
    );
    tap(s);
    assert!(s.sheet.is_some());
    assert!(commands(s).is_empty());
    crate::starter::hold(s, StarterAction::RestartNow);
    assert_eq!(s.sheet, None);
    assert_eq!(commands(s), [sent]);
}

/// The restart is a press: one tap restarts.
fn assert_pressed(s: &mut Starter) {
    for l in [RESTART_HOLD, RESTART_UNREAD] {
        assert!(!shows(s, l), "{l}");
    }
    tap(s);
    assert_eq!(s.sheet, None);
    assert_eq!(commands(s), [StarterCommand::RestartIntoFirmware]);
}

#[test]
fn restart_is_a_hold_while_bitlocker_is_on_with_no_key_printed() {
    let mut s = restart_sheet(drive(Protection::On, None));
    assert_held(&mut s, RESTART_HOLD, StarterCommand::RestartIntoFirmware);
}

#[test]
fn restart_is_a_hold_while_bitlocker_is_not_read() {
    for read in [None, drive(Protection::Unknown, None)] {
        let mut s = restart_sheet(read.clone());
        assert_held(&mut s, RESTART_UNREAD, StarterCommand::RestartIntoFirmware);
    }
}

#[test]
fn restart_to_a_device_is_held_the_same() {
    let mut s = start_page(Os::Windows, drive(Protection::On, None));
    let mut c = dell(Os::Windows);
    c.secure_boot = Some(false);
    s.starter_event(StarterEvent::Computer(c));
    s.starter_event(StarterEvent::Disks(vec![faraday_stick()]));
    let _ = commands(&mut s);
    s.press(StarterAction::RestartStick);
    assert!(s.sheet.is_some());
    assert_held(
        &mut s,
        RESTART_HOLD,
        StarterCommand::RestartFromStick {
            device: "Disk 1".into(),
        },
    );
}

#[test]
fn restart_from_home_reads_bitlocker_first_and_is_a_press_when_it_is_off() {
    let mut s = home(Os::Windows);
    // With a Faraday stick attached, Home's Next is the restart.
    s.starter_event(StarterEvent::Disks(vec![faraday_stick()]));
    let _ = commands(&mut s);
    assert_eq!(s.page, Page::Home);
    let _ = s.drawn_texts();
    s.press(StarterAction::Next);
    assert!(s.sheet.is_some(), "the restart sheet is up");
    // Not read yet: the hold, and the reading is asked for.
    assert_eq!(commands(&mut s), [StarterCommand::ReadBitLocker]);
    assert!(shows(&mut s, RESTART_UNREAD));
    // Windows answers there is no BitLocker: one tap restarts.
    s.starter_event(StarterEvent::BitLocker(drive(Protection::None, None)));
    assert_pressed(&mut s);
}

#[test]
fn restart_is_a_press_once_the_key_sheet_opened() {
    let mut s = start_page(Os::Windows, drive(Protection::On, None));
    s.press(StarterAction::IncludeKey);
    s.press(StarterAction::PrintNotes);
    s.starter_event(StarterEvent::RecoveryKey(Ok(vec![key()])));
    let mut c = dell(Os::Windows);
    c.secure_boot = Some(false);
    s.starter_event(StarterEvent::Computer(c));
    // Made but not yet opened, or failed to open: still a hold.
    s.starter_event(StarterEvent::NotesOpened(Err(Reason::NoWriter)));
    s.press(StarterAction::Restart);
    assert!(shows(&mut s, RESTART_HOLD));
    s.press(StarterAction::Cancel);
    s.press(StarterAction::PrintNotes);
    s.starter_event(StarterEvent::RecoveryKey(Ok(vec![key()])));
    s.starter_event(StarterEvent::NotesOpened(Ok(())));
    let _ = commands(&mut s);
    s.press(StarterAction::Restart);
    assert_pressed(&mut s);
}

#[test]
fn restart_is_a_press_while_bitlocker_is_suspended() {
    let mut s = restart_sheet(drive(Protection::Suspended, Some(3)));
    assert_pressed(&mut s);
}

#[test]
fn the_hold_comes_back_after_resume() {
    let mut s = start_page(Os::Windows, drive(Protection::Suspended, Some(3)));
    s.press(StarterAction::ResumeBitLocker);
    s.starter_event(StarterEvent::BitLocker(drive(Protection::On, None)));
    let mut c = dell(Os::Windows);
    c.secure_boot = Some(false);
    s.starter_event(StarterEvent::Computer(c));
    let _ = commands(&mut s);
    s.press(StarterAction::Restart);
    let _ = commands(&mut s);
    assert_held(&mut s, RESTART_HOLD, StarterCommand::RestartIntoFirmware);
}
