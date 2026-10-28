//! What to write's Boot file (`docs/STARTER.md` §4.5, F133): the
//! release's own by default; "One I signed" lists the `.efi` files in
//! Downloads, has the one tapped checked against the release, shows who
//! signed it, and goes on only once it is taken; a refused file says why
//! by name; Confirm shows the choice and the write carries it.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{
    BootFile, BootSigner, Bus, Computer, Disk, FaradayStick, Mode, Os, Payload, PayloadFile,
    Reason, Source, StarterCommand, StarterEvent,
};
use faraday_starter::decide::Target;
use faraday_starter::{Page, Starter, StarterAction};
use faraday_strings::EN;

const GB: u64 = 1_000_000_000;
const DOWNLOADS: &str = "/media/stick/Downloads";
const FINGERPRINT: &str = "5a1c0e7b2d9f4c3a8b6e1d0f7a2c5e9b3d8f1a6c4e0b7d2f9a5c3e8b1d6f0a4c";

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
        version: "0.3.0".into(),
        commit: "89abcdef0123456789abcdef0123456789abcdef".into(),
        files: vec![
            PayloadFile {
                name: "faraday-stick-0.3.0-x86_64-uefi-strict.img.xz".into(),
                sha256: "a".repeat(64),
                inner: "faraday-stick-0.3.0-x86_64-uefi-strict.img".into(),
                inner_sha256: "b".repeat(64),
                inner_size: 202_395_648,
                build_id: "0.3.0+89abcdef0123".into(),
            },
            PayloadFile {
                name: "faraday-pi-0.3.0-pi3.img.xz".into(),
                sha256: "d".repeat(64),
                inner: "faraday-pi-0.3.0-pi3.img".into(),
                inner_sha256: "e".repeat(64),
                inner_size: 205_520_896,
                build_id: "0.3.0+89abcdef0123".into(),
            },
        ],
    }
}

/// A Faraday stick holding an older release.
fn stick() -> Disk {
    Disk {
        id: "/org/freedesktop/UDisks2/block_devices/sdb".into(),
        device: "/dev/sdb".into(),
        vendor: "SanDisk".into(),
        model: "Ultra Fit".into(),
        serial: "AAA111".into(),
        size: 16 * GB,
        bus: Bus::Usb,
        volumes: Vec::new(),
        in_use: None,
        faraday: Some(FaradayStick {
            release: Some("6.6.84-faraday-0.2.0".into()),
            read: true,
        }),
    }
}

fn starter() -> Starter {
    let mut s = Starter::new();
    s.event(display());
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
    s.starter_event(StarterEvent::Payload(Some(payload())));
    s.starter_event(StarterEvent::Disks(vec![stick()]));
    s
}

fn shows(s: &mut Starter, text: &str) -> bool {
    s.drawn_texts().iter().any(|t| t.contains(text))
}

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

fn release_source() -> Source {
    Source::Payload {
        name: "faraday-stick-0.3.0-x86_64-uefi-strict.img.xz".into(),
        inner_sha256: "b".repeat(64),
    }
}

fn file(name: &str) -> String {
    format!("{DOWNLOADS}/{name}")
}

/// What to write for the stick, keep data ticked.
fn to_mode(s: &mut Starter) {
    s.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(s.disk_number("/org/freedesktop/UDisks2/block_devices/sdb"));
    s.press(n);
    s.press(StarterAction::Mode(Mode::KeepData));
    assert_eq!(s.page, Page::Mode);
    commands(s);
}

/// "One I signed" ticked, and the shell's listing of Downloads.
fn signed_ticked(s: &mut Starter) {
    s.press(StarterAction::BootChoice(true));
    assert_eq!(commands(s), [StarterCommand::ListBootFiles]);
    s.starter_event(StarterEvent::BootFiles {
        folder: DOWNLOADS.into(),
        files: vec![file("BOOTX64-signed.EFI"), file("BOOTX64.EFI")],
    });
}

fn checked(path: &str, result: Result<(BootFile, BootSigner), Reason>) -> StarterEvent {
    StarterEvent::BootFileChecked {
        path: path.into(),
        result,
    }
}

fn taken(path: &str) -> Result<(BootFile, BootSigner), Reason> {
    Ok((
        BootFile {
            path: path.into(),
            sha256: "c".repeat(64),
        },
        BootSigner {
            subject: "Test db".into(),
            fingerprint: FINGERPRINT.into(),
        },
    ))
}

#[test]
fn the_releases_boot_file_is_written_unless_one_signed_is_chosen() {
    let mut s = starter();
    to_mode(&mut s);
    assert!(shows(&mut s, EN.starter_boot_file));
    assert!(shows(&mut s, EN.starter_boot_release));
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Confirm);
    assert!(shows(&mut s, EN.starter_boot_release));
    s.press(StarterAction::Write);
    let c = commands(&mut s);
    assert!(
        matches!(c.as_slice(), [StarterCommand::Write(o)] if o.boot_file.is_none()),
        "{c:?}"
    );
}

#[test]
fn a_signed_boot_file_is_checked_shows_its_signer_and_goes_into_the_write() {
    let mut s = starter();
    to_mode(&mut s);
    signed_ticked(&mut s);
    assert!(shows(&mut s, "BOOTX64-signed.EFI"));
    // Nothing goes on before a file is taken.
    assert!(!s.offers_action(StarterAction::Continue));
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Mode);
    s.press(StarterAction::BootFile(0));
    assert_eq!(
        commands(&mut s),
        [StarterCommand::CheckBootFile {
            path: file("BOOTX64-signed.EFI"),
            source: release_source(),
        }]
    );
    assert!(shows(&mut s, EN.starter_boot_checking));
    s.starter_event(checked(
        &file("BOOTX64-signed.EFI"),
        taken(&file("BOOTX64-signed.EFI")),
    ));
    assert!(shows(&mut s, EN.starter_boot_cert_name));
    assert!(shows(&mut s, "Test db"));
    assert!(shows(&mut s, "5a1c 0e7b"));
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Confirm);
    assert!(shows(&mut s, EN.starter_boot_cert_name));
    assert!(shows(&mut s, "Test db"));
    s.press(StarterAction::Write);
    let c = commands(&mut s);
    let want = BootFile {
        path: file("BOOTX64-signed.EFI"),
        sha256: "c".repeat(64),
    };
    assert!(
        matches!(c.as_slice(), [StarterCommand::Write(o)]
            if o.mode == Mode::KeepData && o.boot_file.as_ref() == Some(&want)),
        "{c:?}"
    );
}

#[test]
fn a_refused_boot_file_says_why_by_name_and_nothing_goes_on() {
    for (why, words) in [
        (Reason::BootFileOther, EN.starter_why_boot_file_other),
        (Reason::BootFileUnsigned, EN.starter_why_boot_file_unsigned),
        (Reason::BootFileBroken, EN.starter_why_boot_file_broken),
        (Reason::BootFileLayout, EN.starter_why_boot_file_layout),
    ] {
        let mut s = starter();
        to_mode(&mut s);
        signed_ticked(&mut s);
        s.press(StarterAction::BootFile(1));
        assert_eq!(commands(&mut s).len(), 1);
        s.starter_event(checked(&file("BOOTX64.EFI"), Err(why)));
        assert!(shows(&mut s, words), "{words}");
        assert!(!shows(&mut s, EN.starter_boot_cert_name));
        s.press(StarterAction::Continue);
        assert_eq!(s.page, Page::Mode);
        // Back to the release's own: it goes on, and writes no other.
        s.press(StarterAction::BootChoice(false));
        s.press(StarterAction::Continue);
        assert_eq!(s.page, Page::Confirm);
        s.press(StarterAction::Write);
        let c = commands(&mut s);
        assert!(
            matches!(c.as_slice(), [StarterCommand::Write(o)] if o.boot_file.is_none()),
            "{c:?}"
        );
    }
}

#[test]
fn downloads_without_an_efi_file_says_so() {
    let mut s = starter();
    to_mode(&mut s);
    s.press(StarterAction::BootChoice(true));
    commands(&mut s);
    s.starter_event(StarterEvent::BootFiles {
        folder: DOWNLOADS.into(),
        files: Vec::new(),
    });
    assert!(shows(&mut s, &format!("No .efi file in {DOWNLOADS}")));
    // A file dropped on the window is checked as one tapped.
    s.file_dropped(file("from-the-stick.efi"));
    assert_eq!(
        commands(&mut s),
        [StarterCommand::CheckBootFile {
            path: file("from-the-stick.efi"),
            source: release_source(),
        }]
    );
}

#[test]
fn a_pi_card_has_no_boot_file_choice() {
    let mut s = starter();
    s.press(StarterAction::Make(Target::Pi));
    let n = StarterAction::Disk(s.disk_number("/org/freedesktop/UDisks2/block_devices/sdb"));
    s.press(n);
    assert!(!shows(&mut s, EN.starter_boot_file));
    assert!(!s.offers_action(StarterAction::BootChoice(true)));
}
