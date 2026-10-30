//! What the grant helper hands out and keeps back, over a stand-in `/sys`
//! and `/dev`, with the changes of ownership recorded instead of made.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use faraday_boot::medium::Record;
use faraday_grant::{Decision, Finder, GIVE_UP, Grant, Owner, SETTLE};

/// Who owns each node, as the helper left it.
#[derive(Default)]
struct Owners {
    by_node: std::collections::BTreeMap<String, &'static str>,
    /// Interfaces a driver was asked for.
    probed: Vec<String>,
    /// Webcams whose camera came up, by device.
    cameras: Vec<String>,
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
    /// uvcvideo as the kernel runs it: bound to a webcam's first video
    /// interface, it claims the device's other video interfaces, and a
    /// claim on one not yet authorised fails and leaves no camera.
    fn probe(&mut self, drivers_probe: &Path, interface: &str) -> std::io::Result<()> {
        self.probed.push(interface.to_string());
        let devices = drivers_probe.parent().unwrap().join("devices");
        let read =
            |n: &str, f: &str| fs::read_to_string(devices.join(n).join(f)).unwrap_or_default();
        let video = |n: &str| read(n, "bInterfaceClass").trim() == "0e";
        let (device, number) = interface.split_once(':').unwrap();
        if number != "1.0" || !video(interface) {
            return Ok(());
        }
        let claimed = fs::read_dir(&devices)?
            .flatten()
            .map(|e| name(&e.path()))
            .filter(|n| n.starts_with(&format!("{device}:")) && video(n))
            .all(|n| read(&n, "authorized").trim() == "1");
        if claimed {
            fs::create_dir_all(devices.join(interface).join("driver"))?;
            self.cameras.push(device.to_string());
        }
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
        let m = Machine { root };
        // The finder has looked and found no stick of this build, unless
        // a test says otherwise.
        m.record("none\n");
        m
    }

    /// The medium's record, as the finder or the image wrote it.
    fn record(&self, text: &str) {
        fs::write(self.root.join("run/exchange"), text).unwrap();
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
        let spans: Vec<(u64, u64)> = (0..parts.len() as u64)
            .map(|k| (2048 + k * 4096, 4096))
            .collect();
        self.disk_laid(bus, disk, "4", parts, &spans);
    }

    /// A disk as [`Machine::disk`] makes one, with its sequence number and
    /// each partition's start and size in sectors.
    fn disk_laid(
        &self,
        bus: &str,
        disk: &str,
        seq: &str,
        parts: &[(&str, Vec<u8>)],
        spans: &[(u64, u64)],
    ) {
        let d = self.root.join("sys/devices").join(bus).join(disk);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("diskseq"), format!("{seq}\n")).unwrap();
        symlink(
            PathBuf::from("../../devices").join(bus).join(disk),
            self.root.join("sys/class/block").join(disk),
        )
        .unwrap();
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
            fs::write(part.join("partition"), format!("{}\n", k + 1)).unwrap();
            fs::write(part.join("start"), format!("{}\n", spans[k].0)).unwrap();
            fs::write(part.join("size"), format!("{}\n", spans[k].1)).unwrap();
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
            &self.root.join("run/exchange"),
            owners,
        )
    }

    /// A stick written from a Faraday image of build `release` (or an
    /// ordinary stick with `None`): a Faraday stick's layout, the names an
    /// ordinary stick has, and the release string in its boot partition.
    fn stick(&self, port: &str, disk: &str, seq: &str, release: Option<&str>) {
        let mut boot = sector("EFI");
        boot.resize(3 << 20, 0x80);
        if let Some(r) = release {
            let at = 1_500_000;
            let text = format!("\0{r} (user@host) #1 SMP\0");
            boot[at..at + text.len()].copy_from_slice(text.as_bytes());
        }
        self.disk_laid(
            &format!("pci0/usb1/{port}/host0/block"),
            disk,
            seq,
            &[
                ("EFI system partition", boot),
                ("Basic data partition", sector("NO NAME")),
            ],
            &[(2048, 131_072), (133_120, 262_144)],
        );
    }

    fn finder(&self, release: &str) -> Finder {
        Finder::new(&self.root.join("sys"), &self.root.join("dev"), release)
    }
}

impl Drop for Machine {
    /// The tree is a test's own; leaving it fills the temp directory.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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
            ("FARADAYBOOT", sector("FARADAYBOOT")),
            ("FARADAYDATA", sector("FARADAYDATA")),
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
        &[("", sector("FARADAYBOOT")), ("", sector("FARADAYDATA"))],
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

#[test]
fn a_faraday_boot_partition_is_never_handed_to_any_process() {
    let m = Machine::new("boot");
    // The stick Faraday started from, another Faraday stick, and the Pi's
    // own card.
    let faraday = |bus: &str, disk: &str| {
        m.disk(
            bus,
            disk,
            &[
                ("FARADAYBOOT", sector("FARADAYBOOT")),
                ("FARADAYDATA", sector("FARADAYDATA")),
            ],
        )
    };
    faraday("pci0/usb1/1-1/block", "sda");
    faraday("pci0/usb1/1-2/block", "sdb");
    m.disk(
        "soc/mmc0/block",
        "mmcblk0",
        &[("", sector("FARADAYBOOT")), ("", sector("FARADAYDATA"))],
    );
    // A file an earlier Faraday published beside the clean marker while
    // it upgraded a stick changes nothing.
    fs::write(m.root.join("run/upgrade"), b"upgrade\n").unwrap();
    let state = |g: &Grant<&mut Owners>, id: &str| g.decided[id].1;
    let mut owners = Owners::default();
    {
        let mut g = m.grant(&mut owners);
        for clean in [false, true, false, true] {
            m.clean(clean);
            g.tick();
            for boot in ["sda1#4", "sdb1#4", "mmcblk0p1#4"] {
                assert_eq!(state(&g, boot), Decision::Boot, "{boot}, clean {clean}");
            }
            let data = if clean {
                Decision::Handed
            } else {
                Decision::Waiting
            };
            assert_eq!(state(&g, "sda2#4"), data);
            assert_eq!(state(&g, "sdb2#4"), data);
        }
    }
    // No boot partition's node was ever taken, given or changed.
    for node in ["sda1", "sdb1", "mmcblk0p1"] {
        assert_eq!(owners.by_node.get(node), None, "{node}");
    }
    assert_eq!(owners.by_node.get("sdb2"), Some(&"disk"));
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
            self.interface(port, k, *c);
        }
    }

    /// Interface `k` of the device on `port`, of class `class`, not
    /// authorised: a root hub after `rcS` authorises none by itself.
    fn interface(&self, port: &str, k: usize, class: u8) {
        let d = self.root.join("sys/devices/pci0/usb1").join(port);
        let bus = self.root.join("sys/bus/usb/devices");
        let name = format!("{port}:1.{k}");
        let i = d.join(&name);
        fs::create_dir_all(&i).unwrap();
        fs::write(i.join("bInterfaceClass"), format!("{class:02x}\n")).unwrap();
        fs::write(i.join("authorized"), "0\n").unwrap();
        let _ = fs::remove_file(bus.join(&name));
        symlink(&i, bus.join(&name)).unwrap();
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
fn a_webcam_comes_up_as_a_camera() {
    let m = Machine::new("webcam");
    // A webcam with its control and streaming interfaces and a
    // microphone, and one with two cameras on it (colour and infrared).
    m.usb("1-3", 4, &[0x0e, 0x0e, 0x01]);
    m.usb("1-5", 6, &[0x0e, 0x0e, 0x0e, 0x0e]);
    let mut owners = Owners::default();
    let mut g = m.grant(&mut owners);
    g.tick();
    g.tick();
    drop(g);
    assert_eq!(owners.cameras, ["1-3", "1-5"]);
}

#[test]
fn a_webcam_whose_interfaces_appear_a_tick_apart_comes_up_as_a_camera() {
    let m = Machine::new("webcam-late");
    m.usb("1-3", 4, &[0x0e]);
    let mut owners = Owners::default();
    let mut g = m.grant(&mut owners);
    g.tick();
    m.interface("1-3", 1, 0x0e);
    g.tick();
    g.tick();
    drop(g);
    assert_eq!(owners.cameras, ["1-3"]);
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

const RELEASE: &str = "6.18.56-faraday-0.1.9+4d0680b1a2b3.test";

/// The finder's answer once the disks have settled.
fn settled(f: &mut Finder) -> Record {
    assert_eq!(f.look(std::time::Duration::ZERO), None);
    f.look(SETTLE).expect("decided once settled")
}

#[test]
fn the_pc_finds_the_one_stick_of_its_own_build() {
    let m = Machine::new("find-one");
    // The stick it started from, a stick of another build, and an
    // ordinary stick with the same layout.
    m.stick("1-1", "sda", "3", Some("6.18.56-faraday-0.1.8"));
    m.stick("1-2", "sdb", "5", Some(RELEASE));
    m.stick("1-3", "sdc", "6", None);
    let mut f = m.finder(RELEASE);
    assert_eq!(
        settled(&mut f),
        Record::Found {
            partition: "sdb2".into(),
            diskseq: Some("5".into())
        }
    );
}

#[test]
fn two_sticks_of_its_build_are_neither_and_none_is_said() {
    let m = Machine::new("find-two");
    m.stick("1-1", "sda", "3", Some(RELEASE));
    m.stick("1-2", "sdb", "5", Some(RELEASE));
    assert_eq!(settled(&mut m.finder(RELEASE)), Record::Two);

    let m = Machine::new("find-none");
    m.stick(
        "1-1",
        "sda",
        "3",
        Some("6.18.56-faraday-0.1.9+4d0680b1a2b3.dev"),
    );
    // A disk of the build but another layout is not a Faraday stick.
    m.disk(
        "pci0/usb1/1-4/block",
        "sdd",
        &[("EFI system partition", {
            let mut b = sector("EFI");
            b.extend_from_slice(format!("\0{RELEASE}\0").as_bytes());
            b
        })],
    );
    let mut f = m.finder(RELEASE);
    assert_eq!(f.look(SETTLE), None);
    assert_eq!(f.look(GIVE_UP), Some(Record::NotFound));
}

#[test]
fn a_second_stick_put_in_while_settling_is_waited_for() {
    let m = Machine::new("find-late");
    m.stick("1-1", "sda", "3", Some(RELEASE));
    let mut f = m.finder(RELEASE);
    assert_eq!(f.look(std::time::Duration::ZERO), None);
    m.stick("1-2", "sdb", "5", Some(RELEASE));
    assert_eq!(f.look(SETTLE), None);
    assert_eq!(f.look(SETTLE * 2), Some(Record::Two));
}

#[test]
fn every_boot_partition_is_kept_back_and_nothing_goes_before_the_medium_is_known() {
    let m = Machine::new("keep");
    // The stick it started from, whose first partition someone renamed;
    // another Faraday stick; an ordinary stick with an EFI system
    // partition; and a stick made before F174.
    m.stick("1-1", "sda", "4", Some(RELEASE));
    m.stick("1-2", "sdb", "4", Some("6.18.56-faraday-0.1.8"));
    m.disk(
        "pci0/usb1/1-3/block",
        "sdc",
        &[
            ("EFI system partition", sector("EFI")),
            ("Basic data partition", sector("PHOTOS")),
        ],
    );
    m.disk(
        "pci0/usb1/1-4/block",
        "sde",
        &[
            ("FARADAYBOOT", sector("FARADAYBOOT")),
            ("FARADAYDATA", sector("FARADAYDATA")),
        ],
    );
    fs::write(
        m.root
            .join("sys/devices/pci0/usb1/1-1/host0/block/sda/sda1/uevent"),
        "PARTNAME=\n",
    )
    .unwrap();
    m.clean(true);
    let mut owners = Owners::default();
    {
        // Not looked for yet: nothing at all is handed out.
        m.record("layout\n");
        let mut g = m.grant(&mut owners);
        g.tick();
        for (id, (_, d)) in &g.decided {
            assert_ne!(*d, Decision::Handed, "{id}");
        }
        m.record("/dev/sda2 4\n");
        g.tick();
        let state = |id: &str| g.decided[id].1;
        for boot in ["sda1#4", "sdb1#4", "sdc1#4", "sde1#4"] {
            assert_eq!(state(boot), Decision::Boot, "{boot}");
        }
        for data in ["sda2#4", "sdb2#4", "sdc2#4", "sde2#4"] {
            assert_eq!(state(data), Decision::Handed, "{data}");
        }
    }
    // None was ever the helper's or the disk process's: root's throughout.
    for node in ["sda1", "sdb1", "sdc1", "sde1"] {
        let owner = owners.by_node.get(node).copied();
        assert!(matches!(owner, None | Some("root")), "{node}: {owner:?}");
    }
}
