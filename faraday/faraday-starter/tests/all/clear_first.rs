//! What to write's Clear the whole stick first (`docs/STICK-IDENTITY.md`
//! §4.2): offered for a new stick, unticked, with the minutes it adds;
//! not offered with keep data; ticked, Confirm says so, the write carries
//! it and Writing shows the clearing pass.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{
    Bus, Computer, Disk, FaradayStick, FoundImage, ImageFinding, Mode, Os, StarterCommand,
    StarterEvent, WriteStage,
};
use faraday_starter::decide::Target;
use faraday_starter::{Page, Starter, StarterAction};

const GB: u64 = 1_000_000_000;
const SDB: &str = "/org/freedesktop/UDisks2/block_devices/sdb";

fn starter() -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
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
    s.starter_event(StarterEvent::Computer(Computer {
        os: Os::Linux,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: Some(true),
        tpm_unlock: false,
    }));
    s.starter_event(StarterEvent::ImagesFound(vec![ImageFinding::Found(
        FoundImage {
            path: "/Downloads/faraday-stick-0.3.0-x86_64-uefi.img.xz".into(),
            name: "faraday-stick-0.3.0-x86_64-uefi.img.xz".into(),
            xz_sha256: "a".repeat(64),
            img_sha256: "b".repeat(64),
            inner_size: 202_395_648,
            build_id: "0.3.0+89abcdef0123".into(),
            board: String::new(),
            panel: String::new(),
            starts: Vec::new(),
        },
    )]));
    s.starter_event(StarterEvent::Disks(vec![Disk {
        id: SDB.into(),
        device: "/dev/sdb".into(),
        vendor: "SanDisk".into(),
        model: "Ultra Fit".into(),
        serial: "AAA111".into(),
        size: 32 * GB,
        bus: Bus::Usb,
        volumes: Vec::new(),
        in_use: None,
        faraday: Some(FaradayStick {
            release: Some("6.6.84-faraday-0.2.0".into()),
            read: true,
        }),
    }]));
    s
}

fn shows(s: &mut Starter, text: &str) -> bool {
    s.drawn_texts().iter().any(|t| t.contains(text))
}

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

fn to_mode(s: &mut Starter, mode: Mode) {
    s.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(s.disk_number(SDB));
    s.press(n);
    s.press(StarterAction::Mode(mode));
    assert_eq!(s.page, Page::Mode);
    commands(s);
}

/// Continue, then Write, as a person does; the write asked of the shell.
fn written(s: &mut Starter) -> bool {
    s.press(StarterAction::Continue);
    s.press(StarterAction::Write);
    match commands(s).as_slice() {
        [StarterCommand::Write(o)] => o.clear_first,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_new_stick_offers_it_unticked_with_the_minutes_it_adds() {
    let mut s = starter();
    to_mode(&mut s, Mode::Whole);
    assert!(shows(&mut s, "Clear the whole stick first"));
    assert!(shows(&mut s, "Adds about 67 min"));
    assert!(!written(&mut s));
}

#[test]
fn keep_data_does_not_offer_it() {
    let mut s = starter();
    to_mode(&mut s, Mode::KeepData);
    assert!(!shows(&mut s, "Clear the whole stick first"));
    assert!(!s.offers_action(StarterAction::ClearFirst));
}

#[test]
fn ticked_confirm_says_so_and_the_write_carries_it() {
    let mut s = starter();
    to_mode(&mut s, Mode::Whole);
    s.press(StarterAction::ClearFirst);
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Confirm);
    assert!(shows(&mut s, "Clears the whole stick, about 67 min"));
    s.press(StarterAction::Write);
    match commands(&mut s).as_slice() {
        [StarterCommand::Write(o)] => assert!(o.clear_first),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_tick_left_on_is_not_carried_into_keep_data() {
    let mut s = starter();
    to_mode(&mut s, Mode::Whole);
    s.press(StarterAction::ClearFirst);
    s.press(StarterAction::Mode(Mode::KeepData));
    assert!(!written(&mut s));
}

#[test]
fn writing_shows_the_clearing_pass_with_the_time_left() {
    let mut s = starter();
    to_mode(&mut s, Mode::Whole);
    s.press(StarterAction::ClearFirst);
    assert!(written(&mut s));
    assert_eq!(s.page, Page::Writing);
    assert!(shows(&mut s, "Clearing"));
    assert!(shows(&mut s, "About 67 min left"));
    s.starter_event(StarterEvent::WriteProgress {
        stage: WriteStage::Clearing,
        done: 16 * GB,
        total: 32 * GB,
    });
    assert!(shows(&mut s, "About 34 min left"));
    s.starter_event(StarterEvent::WriteProgress {
        stage: WriteStage::Writing,
        done: 1_000_000,
        total: 200_000_000,
    });
    assert!(shows(&mut s, "Written"));
    assert!(!shows(&mut s, "min left"));
}
