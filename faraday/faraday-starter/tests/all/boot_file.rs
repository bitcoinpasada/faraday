//! What to write's Boot file (`docs/STARTER.md` §4.5, F133): the
//! release's own by default; "One I signed" lists the `.efi` files in
//! Downloads, has the one tapped checked against the release, shows who
//! signed it, and goes on only once it is taken; a refused file says why
//! by name; Confirm shows the choice and the write carries it.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{
    BootFile, BootSigner, Bus, Computer, Disk, FaradayStick, FoundImage, ImageFinding, Mode, Os,
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

/// The release's stick and Pi images, as the finder found them in
/// Downloads.
fn images() -> Vec<ImageFinding> {
    let file = |name: &str, xz: char, img: char, size: u64| {
        ImageFinding::Found(FoundImage {
            path: format!("{DOWNLOADS}/{name}"),
            name: name.into(),
            xz_sha256: xz.to_string().repeat(64),
            img_sha256: img.to_string().repeat(64),
            inner_size: size,
            build_id: "0.3.0+89abcdef0123".into(),
            board: String::new(),
            panel: String::new(),
            starts: Vec::new(),
            boot_sha256: None,
        })
    };
    vec![
        file(
            "faraday-stick-0.3.0-x86_64-uefi.img.xz",
            'a',
            'b',
            202_395_648,
        ),
        file("faraday-pi-0.3.0-pi3.img.xz", 'd', 'e', 205_520_896),
    ]
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
            boot_sha256: None,
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
    s.starter_event(StarterEvent::ImagesFound(images()));
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
    Source::Found {
        path: file("faraday-stick-0.3.0-x86_64-uefi.img.xz"),
        xz_sha256: "a".repeat(64),
        img_sha256: "b".repeat(64),
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
    // The check after keep data names the boot file as the difference.
    s.starter_event(StarterEvent::Written(Ok(())));
    assert!(shows(
        &mut s,
        "The boot partition matches the release, except the boot file you signed, as expected"
    ));
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
        [
            // The finder looks at it with the images in Downloads.
            StarterCommand::FindImages {
                dropped: vec![file("from-the-stick.efi")]
            },
            StarterCommand::CheckBootFile {
                path: file("from-the-stick.efi"),
                source: release_source(),
            }
        ]
    );
}

/// A card or stick is written from the release image the finder found,
/// the first of its kind.
#[test]
fn a_found_image_is_what_a_pi_card_is_written_from() {
    use faraday_starter::decide::Target;
    let found = FoundImage {
        path: file("faraday-pi-0.3.0.img.xz"),
        name: "faraday-pi-0.3.0.img.xz".into(),
        xz_sha256: "aa".repeat(32),
        img_sha256: "bb".repeat(32),
        inner_size: 4096,
        build_id: "0.3.0".into(),
        board: "pi3".into(),
        panel: "any".into(),
        starts: vec!["Pi 3B".into()],
        boot_sha256: None,
    };
    let mut s = starter();
    s.starter_event(StarterEvent::ImagesFound(vec![ImageFinding::Found(
        found.clone(),
    )]));
    let image = s.image(Target::Pi).expect("the found file's");
    assert_eq!(
        image.source,
        Source::Found {
            path: found.path.clone(),
            xz_sha256: found.xz_sha256.clone(),
            img_sha256: found.img_sha256.clone(),
        }
    );
    assert_eq!((image.size, image.sha256), (4096, found.img_sha256));
    assert!(!image.dev);
    // It is a Pi image: no stick is offered from it.
    assert!(s.image(Target::Stick).is_none());
}

#[test]
fn what_the_finder_answers_is_kept_and_the_matching_images_listed() {
    let mut s = starter();
    let found = FoundImage {
        path: file("faraday-pi-0.3.0.img.xz"),
        name: "faraday-pi-0.3.0.img.xz".into(),
        xz_sha256: "aa".repeat(32),
        img_sha256: "bb".repeat(32),
        inner_size: 4096,
        build_id: "0.3.0".into(),
        board: "pi3".into(),
        panel: "any".into(),
        starts: vec!["Pi 3B".into()],
        boot_sha256: None,
    };
    let older = ImageFinding::OtherRelease {
        path: file("faraday-pi-0.2.1.img.xz"),
        name: "faraday-pi-0.2.1.img.xz".into(),
        needs: "0.3.0".into(),
    };
    s.starter_event(StarterEvent::ImagesFound(vec![
        older.clone(),
        ImageFinding::Found(found.clone()),
    ]));
    assert_eq!(s.image_findings().len(), 2);
    assert_eq!(s.found_images().collect::<Vec<_>>(), [&found]);
    // A later answer replaces it.
    s.starter_event(StarterEvent::ImagesFound(vec![older]));
    assert_eq!(s.found_images().count(), 0);
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

/// Make a Pi card lists the images found, never one image twice, one can
/// be chosen, and the file to download is named for hardware with none.
#[test]
fn make_a_pi_card_offers_the_found_images_once_and_names_what_is_missing() {
    use faraday_starter::decide::Target;
    let mut s = starter();
    let first_pi = "faraday-pi-0.3.0-pi3.img.xz";
    let found = |name: &str, img: &str| FoundImage {
        path: file(name),
        name: name.into(),
        xz_sha256: "aa".repeat(32),
        img_sha256: img.into(),
        inner_size: 4096,
        build_id: "0.3.0".into(),
        board: "pi3".into(),
        panel: "any".into(),
        starts: vec!["Pi 3B".into()],
        boot_sha256: None,
    };
    s.starter_event(StarterEvent::ImagesWanted(vec![
        first_pi.into(),
        "faraday-stick-0.9.0-x86_64-uefi.img.xz".into(),
    ]));
    let mut all = images();
    all.extend([
        // The same image found again: not offered twice.
        ImageFinding::Found(found("faraday-pi-copy.img.xz", &"e".repeat(64))),
        ImageFinding::Found(found("faraday-pi-0.3.0.img.xz", &"bb".repeat(32))),
    ]);
    s.starter_event(StarterEvent::ImagesFound(all));
    s.press(StarterAction::Make(Target::Pi));
    let heads: Vec<_> = s
        .image_choices(Target::Pi)
        .into_iter()
        .map(|c| c.head)
        .collect();
    assert_eq!(heads, [first_pi, "faraday-pi-0.3.0.img.xz"]);
    assert_eq!(
        s.images_to_download(Target::Stick),
        ["faraday-stick-0.9.0-x86_64-uefi.img.xz"]
    );
    assert!(s.images_to_download(Target::Pi).is_empty());
    s.press(StarterAction::Image(1));
    assert!(matches!(
        s.image(Target::Pi).expect("chosen").source,
        Source::Found { .. }
    ));
    s.press(StarterAction::Image(0));
    assert!(matches!(
        s.image(Target::Pi).expect("chosen").source,
        Source::Found { path, .. } if path.ends_with(first_pi)
    ));
}
