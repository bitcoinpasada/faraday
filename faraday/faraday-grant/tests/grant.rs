//! What the grant helper hands out and keeps back, over a stand-in `/sys`
//! and `/dev`, with the changes of ownership recorded instead of made.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use faraday_grant::{Decision, Grant, Owner};

/// Who owns each node, as the helper left it.
#[derive(Default)]
struct Owners {
    by_node: std::collections::BTreeMap<String, &'static str>,
    /// Interfaces a driver was asked for.
    probed: Vec<String>,
    /// Who each input or video node was given to.
    assigned: std::collections::BTreeMap<String, (u32, u32)>,
}

impl Owner for &mut Owners {
    fn take(&mut self, node: &Path) -> std::io::Result<()> {
        self.by_node.insert(name(node), "grant");
        Ok(())
    }
    fn give(&mut self, node: &Path) -> std::io::Result<()> {
        self.by_node.insert(name(node), "disk");
        Ok(())
    }
    fn back(&mut self, node: &Path) -> std::io::Result<()> {
        self.by_node.insert(name(node), "root");
        Ok(())
    }
    fn assign(&mut self, node: &Path, uid: u32, gid: u32, _mode: u32) -> std::io::Result<()> {
        self.assigned.insert(name(node), (uid, gid));
        Ok(())
    }
    fn owner_of(&mut self, node: &Path) -> std::io::Result<(u32, u32)> {
        Ok(self.assigned.get(&name(node)).copied().unwrap_or((0, 0)))
    }
    fn probe(&mut self, _drivers_probe: &Path, interface: &str) -> std::io::Result<()> {
        self.probed.push(interface.to_string());
        Ok(())
    }
    fn authorize(&mut self, attr: &Path) -> std::io::Result<()> {
        fs::write(attr, "1\n")?;
        let owner = attr.parent().unwrap();
        self.by_node.insert(name(owner), "authorized");
        Ok(())
    }
}

fn name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

/// A boot sector with this FAT16 label.
fn sector(label: &str) -> Vec<u8> {
    let mut b = vec![0u8; 512];
    b[22] = 32;
    b[38] = 0x29;
    let mut l = [b' '; 11];
    l[..label.len()].copy_from_slice(label.as_bytes());
    b[43..54].copy_from_slice(&l);
    b[510] = 0x55;
    b[511] = 0xAA;
    b
}

struct Machine {
    root: PathBuf,
}

impl Machine {
    fn new(name: &str) -> Machine {
        let root =
            std::env::temp_dir().join(format!("faraday-grant-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sys/class/block")).unwrap();
        fs::create_dir_all(root.join("dev")).unwrap();
        fs::create_dir_all(root.join("run")).unwrap();
        Machine { root }
    }

    fn clean(&self, on: bool) {
        let m = self.root.join("run/clean");
        if on {
            fs::write(m, b"clean\n").unwrap();
        } else {
            let _ = fs::remove_file(m);
        }
    }

    /// A disk on `bus` (a path under devices/) with partitions of
    /// (GPT name, boot sector), linked from class/block as the kernel
    /// does, by a relative link.
    fn disk(&self, bus: &str, disk: &str, parts: &[(&str, Vec<u8>)]) {
        let d = self.root.join("sys/devices").join(bus).join(disk);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("diskseq"), "4\n").unwrap();
        for (k, (partname, head)) in parts.iter().enumerate() {
            // The kernel puts a `p` between a disk name ending in a digit
            // and the partition number.
            let p = if disk.ends_with(|c: char| c.is_ascii_digit()) {
                "p"
            } else {
                ""
            };
            let node = format!("{disk}{p}{}", k + 1);
            let part = d.join(&node);
            fs::create_dir_all(&part).unwrap();
            fs::write(part.join("partition"), "1\n").unwrap();
            fs::write(part.join("uevent"), format!("PARTNAME={partname}\n")).unwrap();
            let rel = PathBuf::from("../../devices")
                .join(bus)
                .join(disk)
                .join(&node);
            symlink(rel, self.root.join("sys/class/block").join(&node)).unwrap();
            fs::write(self.root.join("dev").join(&node), head).unwrap();
        }
    }

    fn grant<'a>(&self, owners: &'a mut Owners) -> Grant<&'a mut Owners> {
        Grant::new(
            &self.root.join("sys"),
            &self.root.join("dev"),
            &self.root.join("run/clean"),
            owners,
        )
    }
}

#[test]
fn a_stick_is_handed_out_while_clean_and_the_boot_partition_never() {
    let m = Machine::new("hand");
    // The boot stick, its boot partition named in GPT; an ordinary stick;
    // the laptop's own disk, not on USB; and the Pi's own card, whose
    // first partition is its boot partition.
    m.disk(
        "pci0/usb1/1-1/host0/target0/0:0:0:0/block",
        "sda",
        &[
            ("OSKBOOT", sector("OSKBOOT")),
            ("OSKDATA", sector("OSKDATA")),
        ],
    );
    m.disk("pci0/usb2/2-1/block", "sdc", &[("", sector("TESTSTICK"))]);
    m.disk(
        "pci0/0000:00:1f.2/ata1/block",
        "sdd",
        &[("", sector("WINDOWS"))],
    );
    m.disk(
        "soc/mmc0/block",
        "mmcblk0",
        &[("", sector("OSKBOOT")), ("", sector("OSKDATA"))],
    );
    m.clean(true);
    let mut owners = Owners::default();
    let mut g = m.grant(&mut owners);
    g.tick();
    let decided: Vec<(String, Decision)> = g
        .decided
        .iter()
        .map(|(id, (_, d))| (id.clone(), *d))
        .collect();
    assert_eq!(
        decided,
        vec![
            ("mmcblk0p1#4".into(), Decision::Boot),
            ("mmcblk0p2#4".into(), Decision::Handed),
            ("sda1#4".into(), Decision::Boot),
            ("sda2#4".into(), Decision::Handed),
            ("sdc1#4".into(), Decision::Handed),
        ]
    );
    drop(g);
    // Neither boot partition was touched, nor the laptop's own disk.
    assert_eq!(owners.by_node.get("sda1"), None);
    assert_eq!(owners.by_node.get("mmcblk0p1"), None);
    assert_eq!(owners.by_node.get("sda2"), Some(&"disk"));
    assert_eq!(owners.by_node.get("sdc1"), Some(&"disk"));
    assert_eq!(owners.by_node.get("sdd1"), None);
}

#[test]
fn nothing_is_handed_out_until_clean_and_everything_comes_back_when_it_is_not() {
    let m = Machine::new("clean");
    m.disk("pci0/usb1/1-1/block", "sdb", &[("", sector("TESTSTICK"))]);
    let mut owners = Owners::default();
    {
        let mut g = m.grant(&mut owners);
        m.clean(false);
        g.tick();
        assert_eq!(g.decided["sdb1#4"].1, Decision::Waiting);
        m.clean(true);
        g.tick();
        assert_eq!(g.decided["sdb1#4"].1, Decision::Handed);
        // The app loads a key: the marker goes, and the stick with it.
        m.clean(false);
        g.tick();
        assert_eq!(g.decided["sdb1#4"].1, Decision::Waiting);
    }
    assert_eq!(owners.by_node.get("sdb1"), Some(&"root"));
    // A fresh, clean process: handed out again.
    let mut g = m.grant(&mut owners);
    m.clean(true);
    g.tick();
    assert_eq!(g.decided["sdb1#4"].1, Decision::Handed);
}

impl Machine {
    /// A USB device on `port` of bus 1 with interfaces of these classes,
    /// not authorised by the kernel: what `usbcore.authorized_default=2`
    /// leaves an external device as.
    fn usb(&self, port: &str, devnum: u32, classes: &[u8]) {
        let d = self.root.join("sys/devices/pci0/usb1").join(port);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("authorized"), "0\n").unwrap();
        fs::write(d.join("devnum"), format!("{devnum}\n")).unwrap();
        let bus = self.root.join("sys/bus/usb/devices");
        fs::create_dir_all(&bus).unwrap();
        let _ = fs::remove_file(bus.join(port));
        symlink(&d, bus.join(port)).unwrap();
        for (k, c) in classes.iter().enumerate() {
            let name = format!("{port}:1.{k}");
            let i = d.join(&name);
            fs::create_dir_all(&i).unwrap();
            fs::write(i.join("bInterfaceClass"), format!("{c:02x}\n")).unwrap();
            fs::write(i.join("authorized"), "0\n").unwrap();
            let _ = fs::remove_file(bus.join(&name));
            symlink(&i, bus.join(&name)).unwrap();
        }
    }

    fn authorized(&self, port: &str, iface: &str) -> bool {
        let p = self.root.join("sys/devices/pci0/usb1").join(port);
        let p = if iface.is_empty() {
            p
        } else {
            p.join(format!("{port}:1.{iface}"))
        };
        fs::read_to_string(p.join("authorized")).unwrap().trim() == "1"
    }
}

#[test]
fn a_stick_that_says_it_is_a_keyboard_types_nothing() {
    let m = Machine::new("usb");
    // A stick with a storage interface and a HID one; a keyboard; a
    // webcam with video and audio; a network adapter.
    m.usb("1-1", 2, &[0x08, 0x03]);
    m.usb("1-2", 3, &[0x03]);
    m.usb("1-3", 4, &[0x0e, 0x0e, 0x01]);
    m.usb("1-4", 5, &[0x02, 0x0a]);
    let mut owners = Owners::default();
    let mut g = m.grant(&mut owners);
    g.tick();
    // Every device is authorised; what it may do is its interfaces.
    for port in ["1-1", "1-2", "1-3", "1-4"] {
        assert!(m.authorized(port, ""), "{port}");
    }
    assert!(m.authorized("1-1", "0"), "the stick's storage");
    assert!(!m.authorized("1-1", "1"), "the stick's keyboard");
    assert!(m.authorized("1-2", "0"), "a keyboard");
    assert!(
        m.authorized("1-3", "0") && m.authorized("1-3", "1"),
        "a webcam"
    );
    assert!(!m.authorized("1-3", "2"), "a webcam's microphone");
    assert!(
        !m.authorized("1-4", "0") && !m.authorized("1-4", "1"),
        "a network adapter"
    );
}

#[test]
fn a_keyboard_or_camera_plugged_in_later_can_be_read_by_the_app() {
    let m = Machine::new("nodes");
    fs::create_dir_all(m.root.join("dev/input")).unwrap();
    for n in [
        "dev/input/event5",
        "dev/input/mice",
        "dev/video0",
        "dev/video-not",
    ] {
        fs::write(m.root.join(n), b"").unwrap();
    }
    let mut owners = Owners::default();
    let mut g = m.grant(&mut owners);
    g.tick();
    drop(g);
    assert_eq!(owners.assigned.get("event5"), Some(&(200, 200)));
    assert_eq!(owners.assigned.get("video0"), Some(&(0, 200)));
    assert_eq!(owners.assigned.get("mice"), None);
    assert_eq!(owners.assigned.get("video-not"), None);
}
