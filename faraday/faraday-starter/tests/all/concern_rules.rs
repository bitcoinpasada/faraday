//! The rules that read what is attached as something that can capture
//! what Faraday shows or what is typed (`docs/STARTER.md` §3.1a), on
//! made-up EDIDs, USB identities and firmware strings: never a real
//! person's device dump.

use faraday_starter::api::Kind;
use faraday_starter::concern::{
    Usb, edid_maker, edid_name, screen, usb, usb_name, virtual_machine,
};

/// A made-up EDID: the header, the maker's PNP ID, a monitor name
/// descriptor when `name` is given, and a CEA-861 extension with an
/// HDMI vendor block when `hdmi`, and the video codes `vics`.
fn edid(maker: &str, name: Option<&str>, hdmi: bool, vics: &[u8]) -> Vec<u8> {
    let mut e = vec![0u8; 128];
    e[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    let b = maker.as_bytes();
    let v = (u16::from(b[0] - b'A' + 1) << 10)
        | (u16::from(b[1] - b'A' + 1) << 5)
        | u16::from(b[2] - b'A' + 1);
    e[8..10].copy_from_slice(&v.to_be_bytes());
    if let Some(n) = name {
        let d = &mut e[72..90];
        d[3] = 0xfc;
        let mut text = [0x20u8; 13];
        let nb = n.as_bytes();
        text[..nb.len()].copy_from_slice(nb);
        if nb.len() < 13 {
            text[nb.len()] = 0x0a;
        }
        d[5..].copy_from_slice(&text);
    }
    if !hdmi && vics.is_empty() {
        return e;
    }
    e[126] = 1;
    let mut x = vec![0u8; 128];
    x[0] = 0x02;
    x[1] = 0x03;
    let mut blocks = Vec::new();
    if !vics.is_empty() {
        blocks.push((2u8 << 5) | vics.len() as u8);
        blocks.extend_from_slice(vics);
    }
    if hdmi {
        // The OUI, then a physical address 1.0.0.0.
        blocks.extend_from_slice(&[(3 << 5) | 5, 0x03, 0x0c, 0x00, 0x10, 0x00]);
    }
    x[2] = 4 + blocks.len() as u8;
    x[4..4 + blocks.len()].copy_from_slice(&blocks);
    e.extend_from_slice(&x);
    e
}

#[test]
fn an_edid_says_its_maker_and_name() {
    let e = edid("SAM", Some("EXAMPLE 65"), false, &[]);
    assert_eq!(edid_maker(&e).as_deref(), Some("SAM"));
    assert_eq!(edid_name(&e).as_deref(), Some("EXAMPLE 65"));
    assert_eq!(edid_name(&edid("ABC", None, false, &[])), None);
    assert_eq!(edid_name(b"not an edid at all"), None);
}

#[test]
fn a_screen_whose_name_names_a_tv_is_a_tv() {
    for name in ["EXAMPLE TV", "LGTV", "SMART TV 55", "TV-1000"] {
        assert_eq!(
            screen(&edid("XYZ", Some(name), false, &[])),
            Some(Kind::Tv),
            "{name}"
        );
    }
}

#[test]
fn a_tv_makers_screen_with_hdmi_and_broadcast_timings_is_a_tv() {
    // 1080i60 (5) and 1080i50 (20) beside 1080p60 (16, native).
    let tv = edid("SAM", Some("EXAMPLE"), true, &[0x90, 5, 20, 4]);
    assert_eq!(screen(&tv), Some(Kind::Tv));
    // The same maker's monitor: HDMI, progressive timings only.
    let monitor = edid("SAM", Some("EXAMPLE 27"), true, &[16, 4, 31]);
    assert_eq!(screen(&monitor), None);
    // Broadcast timings without the HDMI block.
    assert_eq!(screen(&edid("SAM", None, false, &[5, 20])), None);
    // Another maker's screen with both.
    assert_eq!(
        screen(&edid("XYZ", Some("PROJECTOR"), true, &[5, 20])),
        None
    );
}

#[test]
fn a_laptop_panel_and_a_monitor_are_not_of_concern() {
    assert_eq!(screen(&edid("ABC", None, false, &[])), None);
    assert_eq!(screen(&edid("DEF", Some("EXAMPLE 24"), false, &[])), None);
    assert_eq!(screen(&[]), None);
    assert_eq!(screen(&[0u8; 256]), None);
}

#[test]
fn the_edid_a_kvms_capture_side_reports_names_it() {
    for name in ["PiKVM V4", "EXAMPLEKVM", "TinyPilot"] {
        assert_eq!(
            screen(&edid("XYZ", Some(name), true, &[16])),
            Some(Kind::Kvm),
            "{name}"
        );
    }
}

fn device(vendor: u16, product: u16, maker: &str, name: &str, ifs: &[(u8, u8, u8)]) -> Usb {
    Usb {
        vendor,
        product,
        manufacturer: maker.into(),
        name: name.into(),
        interfaces: ifs.to_vec(),
    }
}

const KEYBOARD: (u8, u8, u8) = (0x03, 0x01, 0x01);
const MOUSE: (u8, u8, u8) = (0x03, 0x01, 0x02);
const DISK: (u8, u8, u8) = (0x08, 0x06, 0x50);

#[test]
fn a_linux_gadget_keyboard_with_a_disk_is_a_kvm() {
    let ipkvm = device(
        0x1d6b,
        0x0104,
        "Example",
        "Composite Gadget",
        &[KEYBOARD, MOUSE, DISK],
    );
    assert_eq!(usb(&ipkvm), Some(Kind::Kvm));
    // A Linux root Starter carries the same vendor, and only a Starter.
    let starter = device(
        0x1d6b,
        0x0002,
        "Linux Foundation",
        "2.0 root starter",
        &[(9, 0, 0)],
    );
    assert_eq!(usb(&starter), None);
    // A gadget keyboard alone is a keyboard.
    assert_eq!(usb(&device(0x1d6b, 0x0104, "", "", &[KEYBOARD])), None);
}

#[test]
fn strings_naming_a_kvm_are_a_kvm() {
    for (maker, name) in [("Example", "USB KVM Switch"), ("TinyPilot", "Keyboard")] {
        assert_eq!(
            usb(&device(0x1234, 0x5678, maker, name, &[KEYBOARD])),
            Some(Kind::Kvm)
        );
    }
}

#[test]
fn a_known_receiver_or_one_named_so_is_a_receiver() {
    for product in [0xc52b, 0xc534] {
        assert_eq!(
            usb(&device(0x046d, product, "", "", &[])),
            Some(Kind::Receiver)
        );
    }
    // Logitech's Bolt receiver, told apart by its ID.
    assert_eq!(
        usb(&device(0x046d, 0xc548, "", "", &[])),
        Some(Kind::BoltReceiver)
    );
    for name in ["2.4G Receiver", "Wireless Mouse", "Nano Dongle"] {
        assert_eq!(
            usb(&device(0x1234, 0x0001, "Example", name, &[KEYBOARD, MOUSE])),
            Some(Kind::Receiver),
            "{name}"
        );
    }
    // A wired keyboard; a headset with "wireless" and no keyboard or
    // mouse interface.
    assert_eq!(
        usb(&device(
            0x1234,
            0x0002,
            "Example",
            "USB Keyboard",
            &[KEYBOARD]
        )),
        None
    );
    assert_eq!(
        usb(&device(
            0x1234,
            0x0003,
            "Example",
            "Wireless Headset",
            &[(1, 1, 0), (3, 0, 0)]
        )),
        None
    );
}

#[test]
fn radios_networks_cameras_and_capture_cards_are_not_warned_about() {
    for d in [
        device(
            0x1234,
            0x0010,
            "Example",
            "Bluetooth Radio",
            &[(0xe0, 1, 1)],
        ),
        device(
            0x1234,
            0x0011,
            "Example",
            "802.11ac WLAN Adapter",
            &[(0xff, 0xff, 0xff)],
        ),
        device(
            0x1234,
            0x0012,
            "Example",
            "USB Ethernet",
            &[(0x02, 0x06, 0), (0x0a, 0, 0)],
        ),
        device(
            0x1234,
            0x0013,
            "Example",
            "HD Webcam",
            &[(0x0e, 1, 0), (0x0e, 2, 0)],
        ),
        device(
            0x1234,
            0x0014,
            "Example",
            "USB3 HDMI Capture",
            &[(0x0e, 1, 0), (0x0e, 2, 0)],
        ),
    ] {
        assert_eq!(usb(&d), None, "{}", d.name);
    }
}

#[test]
fn a_usb_device_is_named_by_its_maker_and_product() {
    assert_eq!(
        usb_name(&device(1, 1, "Example", "Receiver", &[])),
        "Example Receiver"
    );
    assert_eq!(
        usb_name(&device(1, 1, "Example", "Example Receiver", &[])),
        "Example Receiver"
    );
    assert_eq!(usb_name(&device(1, 1, "", "Receiver", &[])), "Receiver");
}

#[test]
fn a_virtual_machine_is_known_by_its_firmware_strings_or_the_cpu() {
    let cases = [
        ("QEMU", "Standard PC (Q35 + ICH9, 2009)", "QEMU"),
        ("VMware, Inc.", "VMware Virtual Platform", "VMware"),
        ("innotek GmbH", "VirtualBox", "VirtualBox"),
        ("Microsoft Corporation", "Virtual Machine", "Hyper-V"),
        ("Xen", "HVM domU", "Xen"),
        (
            "Parallels Software International Inc.",
            "Parallels Virtual Platform",
            "Parallels",
        ),
    ];
    for (maker, model, name) in cases {
        assert_eq!(
            virtual_machine(maker, model, false).as_deref(),
            Some(name),
            "{maker}"
        );
    }
    // Microsoft's own computers are real.
    assert_eq!(
        virtual_machine("Microsoft Corporation", "Surface Example", false),
        None
    );
    assert_eq!(
        virtual_machine("Example Computers", "Example 14", false),
        None
    );
    // The CPU alone says so: named by maker and model.
    assert_eq!(
        virtual_machine("Example Computers", "Example 14", true).as_deref(),
        Some("Example Computers Example 14")
    );
}
