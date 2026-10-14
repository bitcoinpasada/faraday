//! What is attached that is of concern (`docs/STARTER.md` §3.1a), as each
//! system's shell reads it: Linux from a made-up `/sys` and `/proc` in a
//! temporary directory, Windows from made-up answers as its calls give
//! them (UTF-16 registry values, output technologies, device paths).
//! Never a real person's device dump.

use faraday_starter::api::{Concern, Kind};

/// A made-up EDID: the header, the maker's PNP ID and a monitor name.
fn edid(maker: &str, name: &str) -> Vec<u8> {
    let mut e = vec![0u8; 128];
    e[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    let b = maker.as_bytes();
    let v = (u16::from(b[0] - b'A' + 1) << 10)
        | (u16::from(b[1] - b'A' + 1) << 5)
        | u16::from(b[2] - b'A' + 1);
    e[8..10].copy_from_slice(&v.to_be_bytes());
    if !name.is_empty() {
        e[75] = 0xfc;
        let mut text = [0x20u8; 13];
        text[..name.len()].copy_from_slice(name.as_bytes());
        if name.len() < 13 {
            text[name.len()] = 0x0a;
        }
        e[77..90].copy_from_slice(&text);
    }
    e
}

fn kinds(c: &[Concern]) -> Vec<(Kind, &str)> {
    c.iter().map(|c| (c.kind, c.name.as_str())).collect()
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use faraday_starter_shell::linux::devices::{read, screens, usb, virtual_machine};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn tree(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("starter-devices-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("folder");
        dir
    }

    fn put(root: &Path, path: &str, bytes: &[u8]) {
        let p = root.join(path);
        fs::create_dir_all(p.parent().expect("parent")).expect("folders");
        fs::write(p, bytes).expect("file");
    }

    fn connector(root: &Path, name: &str, status: &str, edid: &[u8]) {
        put(
            root,
            &format!("sys/class/drm/{name}/status"),
            format!("{status}\n").as_bytes(),
        );
        put(root, &format!("sys/class/drm/{name}/edid"), edid);
    }

    fn device(root: &Path, port: &str, ids: (&str, &str), strings: (&str, &str), ifs: &[&str]) {
        let d = format!("sys/bus/usb/devices/{port}");
        put(
            root,
            &format!("{d}/idVendor"),
            format!("{}\n", ids.0).as_bytes(),
        );
        put(
            root,
            &format!("{d}/idProduct"),
            format!("{}\n", ids.1).as_bytes(),
        );
        if !strings.0.is_empty() {
            put(
                root,
                &format!("{d}/manufacturer"),
                format!("{}\n", strings.0).as_bytes(),
            );
        }
        if !strings.1.is_empty() {
            put(
                root,
                &format!("{d}/product"),
                format!("{}\n", strings.1).as_bytes(),
            );
        }
        for (n, class) in ifs.iter().enumerate() {
            let (c, s, p) = (&class[..2], &class[2..4], &class[4..6]);
            let i = format!("sys/bus/usb/devices/{port}:1.{n}");
            put(
                root,
                &format!("{i}/bInterfaceClass"),
                format!("{c}\n").as_bytes(),
            );
            put(
                root,
                &format!("{i}/bInterfaceSubClass"),
                format!("{s}\n").as_bytes(),
            );
            put(
                root,
                &format!("{i}/bInterfaceProtocol"),
                format!("{p}\n").as_bytes(),
            );
        }
    }

    #[test]
    fn connected_screens_are_read_by_their_edid() {
        let root = tree("screens");
        // The laptop's own panel, connected, with no name.
        connector(&root, "card1-eDP-1", "connected", &edid("ABC", ""));
        // A TV on HDMI, connected.
        connector(
            &root,
            "card1-HDMI-A-1",
            "connected",
            &edid("SAM", "EXAMPLE TV"),
        );
        // A TV's EDID left on a connector no longer connected.
        connector(
            &root,
            "card1-HDMI-A-2",
            "disconnected",
            &edid("SAM", "OTHER TV"),
        );
        // A connector with nothing at all.
        connector(&root, "card1-DP-1", "disconnected", &[]);
        // The card itself is not a connector.
        put(&root, "sys/class/drm/card1/dev", b"226:1\n");
        let found = screens(&root);
        assert_eq!(kinds(&found), vec![(Kind::Tv, "EXAMPLE TV")]);
        assert_eq!(found[0].id, "drm:card1-HDMI-A-1");
    }

    #[test]
    fn usb_devices_are_read_with_their_interfaces() {
        let root = tree("usb");
        // A root Starter, a webcam, a wired keyboard: none of concern.
        device(
            &root,
            "usb1",
            ("1d6b", "0002"),
            ("Linux Foundation", "xHCI Host Controller"),
            &["090000"],
        );
        device(
            &root,
            "1-1",
            ("1234", "0001"),
            ("Example", "HD Webcam"),
            &["0e0100", "0e0200"],
        );
        device(
            &root,
            "1-2",
            ("1234", "0002"),
            ("Example", "USB Keyboard"),
            &["030101"],
        );
        // An IP-KVM's gadget: keyboard, mouse and disk.
        device(
            &root,
            "1-3",
            ("1d6b", "0104"),
            ("", ""),
            &["030101", "030102", "080650"],
        );
        // A receiver.
        device(
            &root,
            "1-4.2",
            ("046d", "c52b"),
            ("Example", "USB Receiver"),
            &["030101", "030102"],
        );
        let found = usb(&root);
        assert_eq!(
            kinds(&found),
            vec![(Kind::Kvm, ""), (Kind::Receiver, "Example USB Receiver")]
        );
        assert_eq!(found[1].id, "usb:1-4.2:046d:c52b");
    }

    #[test]
    fn a_virtual_machine_by_its_firmware_or_its_cpu_flag() {
        let root = tree("vm-firmware");
        put(&root, "sys/class/dmi/id/sys_vendor", b"QEMU\n");
        put(
            &root,
            "sys/class/dmi/id/product_name",
            b"Standard PC (Q35 + ICH9, 2009)\n",
        );
        assert_eq!(virtual_machine(&root).map(|c| c.name), Some("QEMU".into()));

        let root = tree("vm-flag");
        put(&root, "sys/class/dmi/id/sys_vendor", b"Example Computers\n");
        put(&root, "sys/class/dmi/id/product_name", b"Example 14\n");
        put(
            &root,
            "proc/cpuinfo",
            b"processor\t: 0\nflags\t\t: fpu vme hypervisor lm\n",
        );
        assert_eq!(
            virtual_machine(&root).map(|c| c.name),
            Some("Example Computers Example 14".into())
        );

        let root = tree("vm-none");
        put(&root, "sys/class/dmi/id/sys_vendor", b"Example Computers\n");
        put(&root, "proc/cpuinfo", b"flags\t\t: fpu vme lm\n");
        assert_eq!(virtual_machine(&root), None);
    }

    #[test]
    fn everything_together_and_nothing_known_says_nothing() {
        let root = tree("all");
        put(&root, "sys/class/dmi/id/sys_vendor", b"innotek GmbH\n");
        put(&root, "sys/class/dmi/id/product_name", b"VirtualBox\n");
        connector(
            &root,
            "card0-HDMI-A-1",
            "connected",
            &edid("XYZ", "PiKVM V4"),
        );
        let found = read(&root, Some("localhost:10.0"));
        assert_eq!(
            kinds(&found),
            vec![
                (Kind::VirtualMachine, "VirtualBox"),
                (Kind::RemoteSession, ""),
                (Kind::Kvm, "PiKVM V4"),
            ]
        );
        assert!(read(&tree("empty"), Some(":0")).is_empty());
    }
}

mod windows {
    use super::*;
    use faraday_starter_shell::windows::devices::{
        HDMI, INTERNAL, MIRACAST, Output, RawUsb, concerns, edid_key, guest, output_kind,
        usb_devices, usb_ids, usb_interface,
    };
    use faraday_starter_shell::windows::registry::multi_sz;

    /// A `REG_MULTI_SZ` as the registry hands it back.
    fn multi(items: &[&str]) -> Vec<u8> {
        let mut units: Vec<u16> = Vec::new();
        for s in items {
            units.extend(s.encode_utf16());
            units.push(0);
        }
        units.push(0);
        units.iter().flat_map(|u| u.to_le_bytes()).collect()
    }

    fn raw(hw: &[&str], compat: &[&str], bus: &str, maker: &str) -> RawUsb {
        RawUsb {
            hardware_ids: multi_sz(&multi(hw)),
            compatible_ids: multi_sz(&multi(compat)),
            bus_description: bus.into(),
            description: "USB Input Device".into(),
            maker: maker.into(),
        }
    }

    #[test]
    fn output_technology_says_a_wireless_display_or_an_analogue_tv() {
        assert_eq!(output_kind(MIRACAST), Some(Kind::WirelessDisplay));
        assert_eq!(output_kind(2), Some(Kind::Tv));
        assert_eq!(output_kind(HDMI), None);
        assert_eq!(output_kind(INTERNAL), None);
        assert_eq!(output_kind(10), None);
    }

    #[test]
    fn the_monitors_edid_key_comes_from_its_device_path() {
        assert_eq!(
            edid_key(
                r"\\?\DISPLAY#SAM0F9C#5&1a2b3c4d&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}"
            )
            .as_deref(),
            Some(
                r"SYSTEM\CurrentControlSet\Enum\DISPLAY\SAM0F9C\5&1a2b3c4d&0&UID4352\Device Parameters"
            )
        );
        assert_eq!(edid_key(r"\\?\USB#VID_1234#x#{guid}"), None);
        assert_eq!(edid_key(r"\\?\DISPLAY#..\..#x#{guid}"), None);
        assert_eq!(edid_key("DISPLAY#SAM0F9C#x"), None);
    }

    #[test]
    fn hardware_and_compatible_ids_give_the_ids_and_the_interface() {
        assert_eq!(
            usb_ids(r"USB\VID_046D&PID_C52B&REV_1211"),
            Some((0x046d, 0xc52b))
        );
        assert_eq!(
            usb_ids(r"USB\VID_1D6B&PID_0104&MI_00"),
            Some((0x1d6b, 0x0104))
        );
        assert_eq!(usb_ids(r"USB\ROOT_HUB30"), None);
        assert_eq!(
            usb_interface(r"USB\Class_03&SubClass_01&Prot_02"),
            Some((3, 1, 2))
        );
        assert_eq!(usb_interface(r"USB\Class_03&SubClass_01"), None);
        assert_eq!(usb_interface(r"USB\COMPOSITE"), None);
    }

    #[test]
    fn a_composite_devices_interfaces_join_their_device() {
        let listed = [
            raw(
                &[r"USB\VID_1D6B&PID_0104&REV_0510", r"USB\VID_1D6B&PID_0104"],
                &[r"USB\COMPOSITE"],
                "",
                "(Standard USB Host Controller)",
            ),
            raw(
                &[r"USB\VID_1D6B&PID_0104&REV_0510&MI_00"],
                &[r"USB\Class_03&SubClass_01&Prot_01", r"USB\Class_03"],
                "",
                "",
            ),
            raw(
                &[r"USB\VID_1D6B&PID_0104&REV_0510&MI_02"],
                &[r"USB\Class_08&SubClass_06&Prot_50"],
                "",
                "",
            ),
        ];
        let found = usb_devices(&listed);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1.interfaces, vec![(3, 1, 1), (8, 6, 0x50)]);
        assert_eq!(found[0].1.manufacturer, "");
    }

    #[test]
    fn a_hypervisor_counts_unless_windows_is_its_root_partition() {
        assert!(guest(1 << 31, b"Microsoft Hv", 0));
        assert!(guest(1 << 31, b"KVMKVMKVM\0\0\0", 0));
        // Virtualization-based security: Windows is the root partition.
        assert!(!guest(1 << 31, b"Microsoft Hv", 1));
        assert!(!guest(0, b"\0\0\0\0\0\0\0\0\0\0\0\0", 0));
    }

    #[test]
    fn everything_windows_reports_is_read_in_order() {
        let outputs = [
            Output {
                technology: INTERNAL,
                name: String::new(),
                path: "p0".into(),
                edid: Some(edid("ABC", "")),
            },
            Output {
                technology: HDMI,
                name: "EXAMPLE TV".into(),
                path: "p1".into(),
                edid: Some(edid("XYZ", "EXAMPLE TV")),
            },
            Output {
                technology: HDMI,
                name: "EXAMPLE 27".into(),
                path: "p2".into(),
                edid: Some(edid("XYZ", "EXAMPLE 27")),
            },
            Output {
                technology: MIRACAST,
                name: "Example Cast".into(),
                path: "p3".into(),
                edid: None,
            },
        ];
        let usb = [
            raw(
                &[r"USB\VID_046D&PID_C548&REV_0400"],
                &[r"USB\Class_03&SubClass_01&Prot_01"],
                "USB Receiver",
                "Logitech",
            ),
            raw(
                &[r"USB\VID_1234&PID_0001&REV_0100"],
                &[r"USB\Class_0E&SubClass_01&Prot_00"],
                "HDMI Capture",
                "Example",
            ),
        ];
        let found = concerns(Some("Hyper-V".into()), true, &outputs, &usb);
        assert_eq!(
            kinds(&found),
            vec![
                (Kind::VirtualMachine, "Hyper-V"),
                (Kind::RemoteSession, ""),
                (Kind::Tv, "EXAMPLE TV"),
                (Kind::WirelessDisplay, "Example Cast"),
                (Kind::BoltReceiver, "Logitech USB Receiver"),
            ]
        );
        assert!(concerns(None, false, &[], &[]).is_empty());
    }
}
