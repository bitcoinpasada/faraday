//! How Faraday does it in the Starter (`docs/CRYPTO-LEARN.md` §8.4): the
//! pages that write what was chosen carry the button in the top app bar,
//! it opens a sheet with the blocks of the page's actions, and Close, a
//! press beside the sheet or the back arrow put it away. Checked with the
//! sample block, which stands on What to write; the Learn destination
//! lists the page about what is written.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{
    Bus, Computer, Disk, FaradayStick, Os, Payload, PayloadFile, StarterEvent,
};
use faraday_starter::decide::Target;
use faraday_starter::{LearnId, Page, Starter, StarterAction};
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
    }));
    s.starter_event(StarterEvent::Payload(Some(Payload {
        version: "0.3.0".into(),
        commit: "89abcdef0123456789abcdef0123456789abcdef".into(),
        files: vec![PayloadFile {
            name: "faraday-stick-0.3.0-x86_64-uefi.img.xz".into(),
            sha256: "a".repeat(64),
            inner: "faraday-stick-0.3.0-x86_64-uefi.img".into(),
            inner_sha256: "b".repeat(64),
            inner_size: 202_395_648,
            build_id: "0.3.0+89abcdef0123".into(),
        }],
    })));
    s.starter_event(StarterEvent::Disks(vec![Disk {
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
fn what_to_write_offers_the_button_and_it_opens_the_blocks_at_every_width() {
    for (w, h) in [(480, 640), (1280, 800)] {
        let mut s = starter(w, h);
        let _ = s.drawn_texts();
        assert!(!s.offers_action(StarterAction::How), "{w}x{h}: not on Home");
        to_what_to_write(&mut s);
        let _ = s.drawn_texts();
        assert!(
            s.offers_action(StarterAction::How),
            "{w}x{h}: the button is in the bar"
        );
        assert!(!shows(&mut s, "Sample action"), "{w}x{h}: the page first");
        s.press(StarterAction::How);
        assert!(shows(&mut s, EN.starter_how_title), "{w}x{h}: the sheet");
        for text in [
            "Sample action",
            EN.how_field_action,
            EN.how_field_crosscheck,
            EN.starter_how_close,
        ] {
            assert!(shows(&mut s, text), "{w}x{h}: {text}");
        }
        s.press(StarterAction::Cancel);
        assert!(!shows(&mut s, "Sample action"), "{w}x{h}: Close");
        assert_eq!(s.page, Page::Mode, "{w}x{h}: still on the page");
    }
}

#[test]
fn the_sheet_is_put_away_by_the_back_arrow_and_by_the_button_again() {
    let mut s = starter(1280, 800);
    to_what_to_write(&mut s);
    s.press(StarterAction::How);
    assert!(shows(&mut s, "Sample action"));
    s.press(StarterAction::Back);
    assert!(!shows(&mut s, "Sample action"));
    assert_eq!(s.page, Page::Mode, "the back arrow closed the sheet only");
    s.press(StarterAction::How);
    s.press(StarterAction::How);
    assert!(!shows(&mut s, "Sample action"));
}

#[test]
fn the_learn_destination_lists_what_is_written_after_replacing_faraday() {
    let s = starter(1280, 800);
    let pages = s.learn_pages();
    let keep = LearnId::KeepData.page().expect("the page about replacing");
    let checking = LearnId::CheckingWritten
        .page()
        .expect("the page about what is checked");
    let at = |p: &faraday_learn::Page| pages.iter().position(|q| std::ptr::eq(*q, p));
    assert_eq!(at(checking), at(keep).map(|n| n + 1));
}
