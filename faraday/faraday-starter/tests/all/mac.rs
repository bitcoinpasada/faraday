//! Home, the Start page and Make a stick on a Mac (`docs/STARTER.md` §3.1's
//! "On a Mac", §3.2 step 0, §3.4; `docs/MAC.md` §6): what a person sees
//! and can do, on macOS and on Linux running on a Mac.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::api::{
    Bus, Computer, Disk, FaradayStick, Mode, Os, Payload, PayloadFile, PayloadKind, Source,
    StarterCommand, StarterEvent, Volume,
};
use faraday_starter::decide::Target;
use faraday_starter::{LearnId, Page, Starter, StarterAction};

const GB: u64 = 1_000_000_000;

fn display() -> Event {
    Event::Display(DisplayInfo {
        width: 1280,
        height: 1400,
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

fn file(name: &str, inner: &str, n: char) -> PayloadFile {
    PayloadFile {
        name: name.into(),
        sha256: n.to_string().repeat(64),
        inner: inner.into(),
        inner_sha256: n.to_ascii_uppercase().to_string().repeat(64),
        inner_size: 4 * GB,
        build_id: "0.2.0".into(),
    }
}

/// A payload with the PC stick image, and the Apple silicon one when
/// `apple` (a stand-in: no release carries it until `docs/MAC.md` A1).
fn payload(apple: bool) -> Payload {
    let mut files = vec![file(
        "faraday-stick-0.2.0-x86_64-uefi.img.xz",
        "faraday-stick-0.2.0-x86_64-uefi.img",
        'a',
    )];
    if apple {
        files.push(file(
            "faraday-stick-0.2.0-arm64-apple.img.xz",
            "faraday-stick-0.2.0-arm64-apple.img",
            'd',
        ));
    }
    Payload {
        version: "0.2.0".into(),
        commit: "4d0680b1a2b34d0680b1a2b34d0680b1a2b34d06".into(),
        files,
    }
}

fn blank(id: &str) -> Disk {
    Disk {
        id: id.into(),
        device: format!("/dev/{id}"),
        vendor: "Maker".into(),
        model: "Flash".into(),
        serial: format!("SN-{id}"),
        size: 16 * GB,
        bus: Bus::Usb,
        volumes: vec![Volume {
            label: None,
            files: false,
        }],
        in_use: None,
        faraday: None,
    }
}

fn stick() -> Disk {
    Disk {
        faraday: Some(FaradayStick {
            release: Some("6.6.84-faraday-0.2.0".into()),
            read: true,
        }),
        volumes: Vec::new(),
        ..blank("disk4")
    }
}

/// A Mac as the shell reports it: on macOS, `sysctl hw.model`; on Linux,
/// DMI's `product_name`. `arm` as `uname -m` (or Rosetta) says.
fn mac(os: Os, model: &str, arm: bool) -> Computer {
    Computer {
        os,
        maker: "Apple Inc.".into(),
        model: model.into(),
        family: String::new(),
        uefi: !arm,
        arm,
        secure_boot: if os == Os::MacOs { None } else { Some(false) },
        tpm_unlock: false,
    }
}

fn starter(c: Computer, disks: Vec<Disk>, apple_image: bool) -> Starter {
    let mut starter = Starter::new();
    starter.event(display());
    starter.starter_event(StarterEvent::Computer(c));
    starter.starter_event(StarterEvent::Payload(Some(payload(apple_image))));
    starter.starter_event(StarterEvent::Disks(disks));
    starter
}

fn texts(starter: &mut Starter) -> Vec<String> {
    starter.drawn_texts()
}

fn shows(starter: &mut Starter, s: &str) -> bool {
    texts(starter).iter().any(|t| t.contains(s))
}

fn reads(starter: &mut Starter, s: &str) -> bool {
    texts(starter).join(" ").contains(s)
}

fn position(starter: &mut Starter, s: &str) -> Option<usize> {
    texts(starter).iter().position(|t| t.contains(s))
}

/// Whether the card (above the checklist) carries `label`.
fn card_says(starter: &mut Starter, label: &str) -> bool {
    matches!(
        (position(starter, label), position(starter, "Checklist")),
        (Some(a), Some(b)) if a < b
    )
}

/// Whether the checklist (below its label) carries `label`.
fn listed(starter: &mut Starter, label: &str) -> bool {
    let t = texts(starter);
    let Some(from) = t.iter().position(|s| s.contains("Checklist")) else {
        return false;
    };
    t[from..].iter().any(|s| s.contains(label))
}

/// What the card's button opens, and whether a sheet came up.
fn card_opens(starter: &mut Starter) -> (Page, bool) {
    let _ = starter.drawn_texts();
    starter.press(StarterAction::Next);
    let opened = (starter.page, starter.sheet.is_some());
    starter.press(StarterAction::Cancel);
    starter.press(StarterAction::Home);
    opened
}

const SYSTEMS: [Os; 2] = [Os::MacOs, Os::Linux];

#[test]
fn an_intel_mac_is_named_and_offered_its_own_start() {
    for os in SYSTEMS {
        for (model, name, t2) in [
            (
                "MacBookPro11,1",
                "MacBook Pro (Retina, 13-inch, Late 2013)",
                false,
            ),
            ("MacBookPro15,1", "MacBook Pro (15-inch, 2018)", true),
        ] {
            let mut h = starter(mac(os, model, false), Vec::new(), false);
            assert!(reads(&mut h, name), "{model}");
            assert!(shows(&mut h, model), "{model}");
            assert_eq!(shows(&mut h, "Apple T2 Security Chip"), t2, "{model}");
            // No Secure Boot line, and no Secure Boot item.
            assert!(!shows(&mut h, "Secure Boot"), "{model} {os:?}");
            assert!(listed(&mut h, "Make a Faraday stick"));
            assert!(listed(&mut h, "Start this Mac from the stick"));
            assert!(!shows(&mut h, "Start this computer from the stick"));
            // No stick: the card makes one.
            assert!(card_says(&mut h, "Make a Faraday stick"));
            assert_eq!(card_opens(&mut h), (Page::Disks, false));

            // A stick attached: the card opens the Mac's steps, no sheet.
            h.starter_event(StarterEvent::Disks(vec![stick()]));
            assert!(card_says(&mut h, "Start this Mac from the stick"));
            assert_eq!(card_opens(&mut h), (Page::Start, false));
            assert!(!shows(&mut h, "Restart"), "{model} {os:?}");
            assert!(std::iter::from_fn(|| h.next_command()).next().is_none());
        }
    }
}

#[test]
fn a_mac_the_table_does_not_know_is_taken_as_an_intel_mac() {
    for os in SYSTEMS {
        let mut h = starter(mac(os, "MacBookPro99,1", false), vec![stick()], false);
        assert!(shows(&mut h, "Apple Inc. MacBookPro99,1"));
        assert!(!shows(&mut h, "Secure Boot"));
        assert!(listed(&mut h, "Start this Mac from the stick"));
        assert!(card_says(&mut h, "Start this Mac from the stick"));
        assert_eq!(card_opens(&mut h), (Page::Start, false));
    }
}

#[test]
fn a_mac_that_cannot_start_the_stick_says_so_and_keeps_make_a_stick() {
    for os in SYSTEMS {
        for (model, arm, line) in [
            (
                "MacBook2,1",
                false,
                "This Mac's firmware cannot start Faraday",
            ),
            (
                "MacPro2,1",
                false,
                "This Mac's firmware cannot start Faraday",
            ),
            ("MacBookAir10,1", true, "This Mac cannot start Faraday yet"),
            ("Mac14,2", true, "This Mac cannot start Faraday yet"),
            ("Mac15,12", true, "This Mac cannot start Faraday yet"),
            ("Mac16,1", true, "This Mac cannot start Faraday yet"),
            // An x86_64 Starter under Rosetta on an M1 reports ARM.
            ("MacBookPro17,1", true, "This Mac cannot start Faraday yet"),
        ] {
            let mut h = starter(mac(os, model, arm), vec![stick()], false);
            assert!(shows(&mut h, line), "{model} {os:?}");
            assert!(!shows(&mut h, "This computer cannot start"), "{model}");
            assert!(listed(&mut h, "Make a Faraday stick"));
            assert!(!listed(&mut h, "Start this Mac"), "{model}");
            assert!(!shows(&mut h, "Secure Boot"), "{model}");
            assert!(card_says(&mut h, "Make a Faraday stick"), "{model}");
            assert_eq!(card_opens(&mut h), (Page::Disks, false), "{model}");
            // The Start page says the same, and offers nothing.
            h.press(StarterAction::Start);
            assert!(shows(&mut h, line), "{model}");
            assert!(!shows(&mut h, "Option"), "{model}");
        }
    }
}

#[test]
fn every_mac_gets_the_mac_guide_whatever_it_runs() {
    for os in [Os::MacOs, Os::Linux, Os::Windows] {
        for model in [
            "MacBookPro11,1",
            "MacBookPro15,1",
            "MacBook2,1",
            "MacBookAir10,1",
            "Mac16,1",
            "MacBookPro99,1",
            "",
        ] {
            let c = mac(os, model, false);
            assert_eq!(LearnId::for_computer(&c), LearnId::MacIntel, "{model}");
            assert_eq!(
                LearnId::for_computer(&c).address(),
                "https://faradayoffline.app/start/mac-intel"
            );
        }
    }
    assert_eq!(
        LearnId::MacIntel.page().map(|p| p.title),
        Some("Starting an Intel Mac from the stick")
    );
}

#[test]
fn no_firmware_restart_is_offered_on_any_mac() {
    for os in [Os::MacOs, Os::Linux, Os::Windows] {
        for model in ["MacBookPro11,1", "MacBookPro15,1", "MacBookPro99,1"] {
            let mut c = mac(os, model, false);
            if os == Os::Windows {
                // Boot Camp on an older Mac starts Windows in legacy mode.
                c.uefi = false;
            }
            let mut h = starter(c, vec![stick()], false);
            assert!(!shows(&mut h, "Restart"), "{model} {os:?}");
            assert!(!shows(&mut h, "legacy BIOS"), "{model} {os:?}");
            h.press(StarterAction::Start);
            assert!(!shows(&mut h, "Restart"), "{model} {os:?}");
            assert!(!h.offers_action(StarterAction::Restart));
            h.press(StarterAction::Restart);
            assert!(h.sheet.is_none());
            h.press(StarterAction::RestartNow);
            let sent: Vec<StarterCommand> = std::iter::from_fn(|| h.next_command()).collect();
            assert!(sent.is_empty(), "{model} {os:?}: {sent:?}");
        }
    }
}

/// The Start page's words, in order.
fn start_page(model: &str) -> Vec<String> {
    let mut h = starter(mac(Os::MacOs, model, false), vec![stick()], false);
    h.press(StarterAction::Start);
    assert_eq!(h.page, Page::Start);
    let t = texts(&mut h);
    assert!(
        t.iter()
            .any(|s| s.contains("Starting an Intel Mac from the stick"))
    );
    assert!(
        t.iter()
            .any(|s| s.contains("https://faradayoffline.app/start/mac-intel"))
    );
    t
}

fn at(t: &[String], s: &str) -> usize {
    t.iter()
        .position(|x| x.contains(s))
        .unwrap_or_else(|| panic!("no {s}"))
}

#[test]
fn the_start_page_on_a_t2_mac_lowers_startup_security_and_puts_it_back() {
    let t = start_page("MacBookPro15,1");
    let joined = t.join(" ");
    let lower = at(&t, "Lower Startup Security");
    let start = at(&t, "Start from the stick");
    let back = at(&t, "Afterwards: Startup Security back to Full");
    assert!(lower < start && start < back);
    for s in [
        "Command-R",
        "No Security",
        "Allow booting from external or removable media",
        "Hold Option at power-on and choose EFI Boot",
        "firmware password",
        "Full Security",
        "Disallow booting from external or removable media",
    ] {
        assert!(joined.contains(s), "{s}");
    }
    assert!(!joined.contains("Only on a Mac with the T2 chip"));
    assert!(!joined.contains("Secure Boot"));
}

#[test]
fn the_start_page_on_a_mac_without_the_t2_chip_is_the_option_key() {
    let t = start_page("MacBookPro11,1");
    let joined = t.join(" ");
    assert!(joined.contains("Hold Option at power-on and choose EFI Boot"));
    assert!(joined.contains("firmware password"));
    for s in ["Startup Security", "Command-R", "Secure Boot"] {
        assert!(!joined.contains(s), "{s}");
    }
}

#[test]
fn the_start_page_on_an_unknown_mac_marks_the_t2_steps() {
    let t = start_page("MacBookPro99,1");
    let lower = at(&t, "Lower Startup Security");
    let only = at(&t, "Only on a Mac with the T2 chip");
    let start = at(&t, "Start from the stick");
    let back = at(&t, "Afterwards: Startup Security back to Full");
    assert!(lower < only && only < start && start < back);
}

#[test]
fn for_which_computer_is_asked_only_when_both_stick_images_are_carried() {
    // One stick image: straight to the disks.
    let mut h = starter(mac(Os::MacOs, "MacBookPro11,1", false), Vec::new(), false);
    h.press(StarterAction::Make(Target::Stick));
    assert_eq!(h.page, Page::Disks);
    assert!(!shows(&mut h, "For which computer?"));

    // Both: asked first, nothing chosen.
    let mut h = starter(mac(Os::MacOs, "MacBookPro11,1", false), Vec::new(), true);
    h.press(StarterAction::Make(Target::Stick));
    assert_eq!(h.page, Page::Which);
    assert!(shows(&mut h, "For which computer?"));
    assert_eq!(h.making.as_ref().and_then(|m| m.stick_for), None);
    // Back from the disks returns to the question.
    h.press(StarterAction::StickFor(faraday_starter::macs::StickFor::Pc));
    assert_eq!(h.page, Page::Disks);
    h.press(StarterAction::Back);
    assert_eq!(h.page, Page::Which);
}

/// The row the "This computer" tag sits under, on For which computer?
fn tagged(c: Computer) -> Option<&'static str> {
    let mut h = starter(c, Vec::new(), true);
    h.press(StarterAction::Make(Target::Stick));
    let t = texts(&mut h);
    let pc = at(&t, "A PC or an Intel Mac");
    let apple = at(&t, "A Mac with M1 or M2");
    match t.iter().position(|s| s == "This computer") {
        Some(i) if i == pc + 1 => Some("pc"),
        Some(i) if i == apple + 1 => Some("apple"),
        Some(i) => panic!("tag at {i}: {t:?}"),
        None => None,
    }
}

#[test]
fn for_which_computer_tags_this_computers_row() {
    assert_eq!(tagged(mac(Os::MacOs, "MacBookPro15,1", false)), Some("pc"));
    assert_eq!(
        tagged(mac(Os::MacOs, "MacBookAir10,1", true)),
        Some("apple")
    );
    assert_eq!(tagged(mac(Os::MacOs, "Mac14,2", true)), Some("apple"));
    assert_eq!(tagged(mac(Os::MacOs, "Mac16,1", true)), None);
    assert_eq!(tagged(mac(Os::MacOs, "MacBook2,1", false)), None);
    let pc = Computer {
        os: Os::Windows,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        uefi: true,
        secure_boot: Some(true),
        ..Computer::default()
    };
    assert_eq!(tagged(pc), Some("pc"));
}

#[test]
fn the_apple_silicon_choice_writes_the_apple_silicon_image() {
    assert_eq!(
        PayloadKind::of("faraday-stick-0.2.0-arm64-apple.img.xz"),
        Some(PayloadKind::AppleStick)
    );
    assert_eq!(
        PayloadKind::of("faraday-stick-0.2.0-x86_64-uefi.img.xz"),
        Some(PayloadKind::Stick)
    );
    for (k, name) in [
        (
            faraday_starter::macs::StickFor::AppleSilicon,
            "faraday-stick-0.2.0-arm64-apple.img.xz",
        ),
        (
            faraday_starter::macs::StickFor::Pc,
            "faraday-stick-0.2.0-x86_64-uefi.img.xz",
        ),
    ] {
        let mut h = starter(
            mac(Os::MacOs, "MacBookAir10,1", true),
            vec![blank("disk4")],
            true,
        );
        h.press(StarterAction::Make(Target::Stick));
        h.press(StarterAction::StickFor(k));
        let n = StarterAction::Disk(h.disk_number("disk4"));
        h.press(n);
        h.press(StarterAction::Mode(Mode::Whole));
        h.press(StarterAction::Continue);
        assert_eq!(h.page, Page::Confirm);
        let _ = h.drawn_texts();
        h.press(StarterAction::Write);
        let sent: Vec<StarterCommand> = std::iter::from_fn(|| h.next_command()).collect();
        assert!(
            matches!(
                sent.as_slice(),
                [StarterCommand::Write(o)] if matches!(&o.source, Source::Payload { name: n, .. } if n == name)
            ),
            "{sent:?}"
        );
    }
}
