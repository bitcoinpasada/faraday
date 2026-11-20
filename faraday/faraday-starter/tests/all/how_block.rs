//! Cryptography in the Starter (`docs/CRYPTO-LEARN.md` §8.4): the Guide
//! destination has a row that opens a sheet with the blocks of what the
//! Starter computes, the top app bar has no button, and Close, a
//! press beside the sheet or the back arrow put it away. Checked with the
//! real block of writing an image, which stands on What to write; the Learn destination
//! lists the page about what is written.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{
    Bus, Computer, Disk, FaradayStick, FoundImage, ImageFinding, Os, StarterEvent,
};
use faraday_starter::decide::Target;
use faraday_starter::{Page, StartGuide, Starter, StarterAction};
use faraday_strings::EN;

const GB: u64 = 1_000_000_000;

fn starter(width: u16, height: u16) -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
        width,
        height,
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
        camera: Default::default(),
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
            boot_sha256: None,
        },
    )]));
    s.starter_event(StarterEvent::Disks(vec![Disk {
        id: "/org/freedesktop/UDisks2/block_devices/sdb".into(),
        device: "/dev/sdb".into(),
        vendor: "SanDisk".into(),
        model: "Ultra Fit".into(),
        serial: "AAA111".into(),
        size: 16 * GB,
        bus: Bus::Usb,
        fixed: false,
        volumes: Vec::new(),
        in_use: None,
        faraday: Some(FaradayStick {
            release: Some("6.6.84-faraday-0.2.0".into()),
            read: true,
            boot_sha256: None,
        }),
    }]));
    s
}

/// What to write, for the stick, with the disk chosen.
fn to_what_to_write(s: &mut Starter) {
    s.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(s.disk_number("/org/freedesktop/UDisks2/block_devices/sdb"));
    s.press(n);
    assert_eq!(s.page, Page::Mode);
}

fn shows(s: &mut Starter, text: &str) -> bool {
    s.drawn_texts().iter().any(|t| t.contains(text))
}

#[test]
fn the_guide_offers_the_blocks_and_the_bar_does_not_at_every_width() {
    for (w, h) in [(480, 640), (1280, 800)] {
        let mut s = starter(w, h);
        to_what_to_write(&mut s);
        let _ = s.drawn_texts();
        assert!(
            !s.offers_action(StarterAction::How),
            "{w}x{h}: no button in the bar"
        );
        s.press(StarterAction::GuideDestination);
        assert_eq!(s.page, Page::Guide);
        let _ = s.drawn_texts();
        assert!(
            s.offers_action(StarterAction::How),
            "{w}x{h}: the Guide has the row"
        );
        assert!(
            !shows(&mut s, "Write an image to a stick"),
            "{w}x{h}: the page first"
        );
        s.press(StarterAction::How);
        assert!(shows(&mut s, EN.starter_how_title), "{w}x{h}: the sheet");
        for text in [
            "Write an image to a stick",
            EN.how_field_action,
            EN.how_field_standards,
            EN.starter_how_close,
        ] {
            assert!(shows(&mut s, text), "{w}x{h}: {text}");
        }
        s.press(StarterAction::Cancel);
        assert!(
            !shows(&mut s, "Write an image to a stick"),
            "{w}x{h}: Close"
        );
        assert_eq!(s.page, Page::Guide, "{w}x{h}: still on the Guide");
    }
}

#[test]
fn the_sheet_is_put_away_by_the_back_arrow_and_by_the_row_again() {
    let mut s = starter(1280, 800);
    s.press(StarterAction::GuideDestination);
    s.press(StarterAction::How);
    assert!(shows(&mut s, "Write an image to a stick"));
    s.press(StarterAction::Back);
    assert!(!shows(&mut s, "Write an image to a stick"));
    assert_eq!(s.page, Page::Guide, "the back arrow closed the sheet only");
    s.press(StarterAction::How);
    s.press(StarterAction::How);
    assert!(!shows(&mut s, "Write an image to a stick"));
}

#[test]
fn the_guide_destination_lists_replacing_faraday() {
    let s = starter(1280, 800);
    assert!(s.start_guides().contains(&StartGuide::KeepData));
}
