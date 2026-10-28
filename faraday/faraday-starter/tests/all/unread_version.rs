//! A Faraday stick whose version Windows did not let the Starter read
//! (option e): Home and Which stick say "version not read",
//! never "0.1.0 or earlier"; New stick needs no version; keep data asks
//! the elevated copy to read it once, then applies Keep data's
//! rules; a declined prompt says so and nothing is written.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{
    Bus, Computer, Disk, FaradayStick, Mode, Os, Payload, PayloadFile, Reason, StarterCommand,
    StarterEvent,
};
use faraday_starter::decide::Target;
use faraday_starter::{Page, Starter, StarterAction};
use faraday_strings::EN;

const GB: u64 = 1_000_000_000;

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

fn payload() -> Payload {
    Payload {
        version: "0.2.0".into(),
        commit: "4d0680b1a2b34d0680b1a2b34d0680b1a2b34d06".into(),
        files: vec![PayloadFile {
            name: "faraday-stick-0.2.0-x86_64-uefi-strict.img.xz".into(),
            sha256: "a".repeat(64),
            inner: "faraday-stick-0.2.0-x86_64-uefi-strict.img".into(),
            inner_sha256: "A".repeat(64),
            inner_size: 4 * GB,
            build_id: "0.2.0".into(),
        }],
    }
}

/// A Faraday stick as Windows lists it without elevation.
fn unread_stick() -> Disk {
    Disk {
        id: r"\\.\PhysicalDrive2".into(),
        device: "Disk 2".into(),
        vendor: "SanDisk".into(),
        model: "Ultra Fit".into(),
        serial: "AAA111".into(),
        size: 16 * GB,
        bus: Bus::Usb,
        volumes: Vec::new(),
        in_use: None,
        faraday: Some(FaradayStick {
            release: None,
            read: false,
        }),
    }
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

fn starter() -> Starter {
    let mut s = Starter::new();
    s.event(display());
    s.starter_event(StarterEvent::Computer(windows()));
    s.starter_event(StarterEvent::Payload(Some(payload())));
    s.starter_event(StarterEvent::Disks(vec![unread_stick()]));
    s
}

fn shows(s: &mut Starter, text: &str) -> bool {
    s.drawn_texts().iter().any(|t| t.contains(text))
}

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

/// To What to write? for the stick.
fn to_mode(s: &mut Starter) {
    s.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(s.disk_number(r"\\.\PhysicalDrive2"));
    s.press(n);
    assert_eq!(s.page, Page::Mode);
}

/// The elevated copy's answer.
fn read(s: &mut Starter, result: Result<Option<String>, Reason>) {
    s.starter_event(StarterEvent::VersionRead {
        disk: r"\\.\PhysicalDrive2".into(),
        serial: "AAA111".into(),
        size: 16 * GB,
        result,
    });
}

#[test]
fn a_version_not_read_says_so_and_never_0_1_0_or_earlier() {
    let mut s = starter();
    assert!(shows(&mut s, "Faraday stick attached · version not read"));
    assert!(!shows(&mut s, EN.build_0_1_0_or_earlier));
    s.press(StarterAction::Make(Target::Stick));
    assert!(shows(&mut s, "Faraday stick · version not read"));
    assert!(!shows(&mut s, EN.build_0_1_0_or_earlier));

    // A stick read and found to hold no version is one made before
    // Faraday put it there.
    let mut s = starter();
    to_mode(&mut s);
    s.press(StarterAction::Mode(Mode::KeepData));
    read(&mut s, Ok(None));
    s.press(StarterAction::Home);
    assert!(shows(&mut s, "Faraday stick attached · 0.1.0 or earlier"));
}

#[test]
fn a_new_stick_asks_no_version() {
    let mut s = starter();
    to_mode(&mut s);
    s.press(StarterAction::Mode(Mode::Whole));
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Confirm);
    s.press(StarterAction::Write);
    let c = commands(&mut s);
    assert!(
        matches!(c.as_slice(), [StarterCommand::Write(o)] if o.mode == Mode::Whole),
        "{c:?}"
    );
}

#[test]
fn keep_data_asks_for_the_elevated_read_once_then_refuses_the_same_build() {
    let mut s = starter();
    to_mode(&mut s);
    s.press(StarterAction::Mode(Mode::KeepData));
    assert_eq!(
        commands(&mut s),
        [StarterCommand::ReadVersion {
            disk: r"\\.\PhysicalDrive2".into(),
            serial: "AAA111".into(),
            size: 16 * GB,
        }]
    );
    assert!(shows(&mut s, EN.starter_mode_reading));
    // Nothing goes on until it is read, and it is asked once.
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Mode);
    s.press(StarterAction::Mode(Mode::KeepData));
    assert!(commands(&mut s).is_empty());

    read(&mut s, Ok(Some("6.6.84-faraday-0.2.0".into())));
    assert!(shows(&mut s, "Already this build"));
    assert!(!s.offers_action(StarterAction::Mode(Mode::KeepData)));
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Mode);
    assert!(commands(&mut s).is_empty());
    // The whole stick can still be written, and Home shows the version.
    assert!(s.offers_action(StarterAction::Mode(Mode::Whole)));
    s.press(StarterAction::Home);
    assert!(shows(&mut s, "Faraday stick attached · 0.2.0"));
    // The shell's next listing, still unread, keeps what was read.
    s.starter_event(StarterEvent::Disks(vec![unread_stick()]));
    assert!(shows(&mut s, "Faraday stick attached · 0.2.0"));
}

#[test]
fn keep_data_after_the_elevated_read_warns_about_an_older_faraday_and_writes_it() {
    let mut s = starter();
    to_mode(&mut s);
    s.press(StarterAction::Mode(Mode::KeepData));
    assert_eq!(commands(&mut s).len(), 1);
    read(&mut s, Ok(Some("6.6.84-faraday-0.3.0".into())));
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Confirm);
    assert!(shows(&mut s, "This stick holds a newer Faraday: 0.3.0"));
    s.press(StarterAction::Write);
    let c = commands(&mut s);
    assert!(
        matches!(c.as_slice(), [StarterCommand::Write(o)] if o.mode == Mode::KeepData),
        "{c:?}"
    );
    // Once written, what was read is no longer on the stick.
    s.starter_event(StarterEvent::Written(Ok(())));
    s.press(StarterAction::Home);
    assert!(shows(&mut s, "Faraday stick attached · version not read"));
}

#[test]
fn a_declined_elevation_says_so_and_nothing_is_written() {
    let mut s = starter();
    to_mode(&mut s);
    s.press(StarterAction::Mode(Mode::KeepData));
    assert_eq!(commands(&mut s).len(), 1);
    read(&mut s, Err(Reason::Declined));
    assert!(shows(&mut s, EN.starter_why_declined));
    assert!(!shows(&mut s, EN.starter_mode_reading));
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Mode);
    s.press(StarterAction::Write);
    assert!(commands(&mut s).is_empty());
    // Keep data asks again when ticked again.
    s.press(StarterAction::Mode(Mode::KeepData));
    assert!(matches!(
        commands(&mut s).as_slice(),
        [StarterCommand::ReadVersion { .. }]
    ));
}
