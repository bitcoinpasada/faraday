//! Which disks a person is offered to write, from udisks2's answers as
//! recorded (`tests/udisks/`), and which Faraday a stick holds, read
//! from its boot partition alone.

#![cfg(target_os = "linux")]

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

use faraday_starter::api::{Bus, Disk, InUse, StarterEvent};
use faraday_starter::decide::{Refusal, Target, offers};
use faraday_starter_shell::linux::udisks::Objects;
use faraday_starter_shell::linux::{self, Open, Udisks, Versions};

/// The stick image's size: 193 MiB and the backup GPT's 20 KiB (F80).
const IMAGE: u64 = 202_395_648;

fn recorded(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/udisks")
            .join(name),
    )
    .expect("a recorded answer")
}

fn objects(text: &str) -> Objects {
    Objects::parse(text).expect("a well-formed recording")
}

/// A udisks2 that answers from recordings, one after another, and
/// remembers what it was asked to mount, unmount and open.
#[derive(Default)]
struct Recorded {
    answers: Vec<Objects>,
    asked: usize,
    mounts: Vec<(String, PathBuf)>,
    calls: Vec<String>,
}

impl Udisks for Recorded {
    fn objects(&mut self) -> Result<Objects, String> {
        let i = self.asked.min(self.answers.len() - 1);
        self.asked += 1;
        Ok(self.answers[i].clone())
    }

    fn wait(&mut self, _timeout: Duration) -> Result<bool, String> {
        std::thread::sleep(Duration::from_millis(5));
        Ok(true)
    }

    fn mount_ro(&mut self, object: &str) -> Result<PathBuf, String> {
        self.calls.push(format!("mount {object}"));
        self.mounts
            .iter()
            .find(|(o, _)| o == object)
            .map(|(_, p)| p.clone())
            .ok_or_else(|| String::from("no such filesystem"))
    }

    fn unmount(&mut self, object: &str) -> Result<(), String> {
        self.calls.push(format!("unmount {object}"));
        Ok(())
    }

    fn open(&mut self, object: &str, _how: Open) -> Result<File, String> {
        self.calls.push(format!("open {object}"));
        Err(String::from("not here"))
    }

    fn rescan(&mut self, object: &str) -> Result<(), String> {
        self.calls.push(format!("rescan {object}"));
        Ok(())
    }
}

fn listed(text: &str) -> Vec<Disk> {
    let mut bus = Recorded::default();
    linux::list(&mut bus, &objects(text), &mut Versions::default())
}

fn names(list: &[faraday_starter::decide::Offer<'_>]) -> Vec<String> {
    list.iter().map(|o| o.disk.device.clone()).collect()
}

#[test]
fn a_usb_stick_is_offered_and_the_disks_the_computer_uses_are_refused_with_why() {
    let disks = listed(&recorded("attached.txt"));
    let list = offers(&disks, Target::Stick, Some(IMAGE));
    // USB disks only, the smallest first; the computer's own NVMe disk
    // and SD reader are not listed for a stick.
    assert_eq!(
        names(&list),
        ["/dev/sdf", "/dev/sde", "/dev/sdb", "/dev/sdc", "/dev/sdd"]
    );
    let refusal = |dev: &str| {
        list.iter()
            .find(|o| o.disk.device == dev)
            .and_then(|o| o.refused.clone())
    };
    assert_eq!(refusal("/dev/sdb"), None);
    assert_eq!(refusal("/dev/sdf"), Some(Refusal::TooSmall(IMAGE)));
    assert_eq!(refusal("/dev/sde"), Some(Refusal::InUse(InUse::Swap)));
    assert_eq!(
        refusal("/dev/sdc"),
        Some(Refusal::InUse(InUse::Mounted("/".into())))
    );
    assert_eq!(
        refusal("/dev/sdd"),
        Some(Refusal::InUse(InUse::Mounted("/home".into())))
    );
    // What the list says of the free stick.
    let stick = disks
        .iter()
        .find(|d| d.device == "/dev/sdb")
        .expect("listed");
    assert_eq!(
        (stick.vendor.as_str(), stick.model.as_str()),
        ("Acme", "Stick")
    );
    assert_eq!(stick.serial, "AAA111");
    assert_eq!(stick.size, 16_008_609_792);
    assert_eq!(stick.bus, Bus::Usb);
    assert_eq!(stick.volumes.len(), 1);
    assert_eq!(stick.volumes[0].label.as_deref(), Some("HOLIDAY"));
    assert!(stick.volumes[0].files);
    assert!(stick.faraday.is_none());
}

#[test]
fn a_pi_card_is_offered_the_computers_own_sd_reader_too() {
    let disks = listed(&recorded("attached.txt"));
    let list = offers(&disks, Target::Pi, Some(IMAGE));
    let sd = list
        .iter()
        .find(|o| o.disk.device == "/dev/mmcblk0")
        .expect("the SD reader is listed for a card");
    assert_eq!(sd.disk.bus, Bus::SdReader);
    assert_eq!(sd.refused, None);
    assert!(!names(&list).contains(&"/dev/nvme0n1".to_string()));
}

/// A Faraday stick's recording, with its partitions mounted where given.
fn faraday_stick(boot: Option<&Path>, data: Option<&Path>) -> String {
    let at = |p: Option<&Path>| p.map(|p| p.display().to_string()).unwrap_or_default();
    recorded("faraday-stick.txt")
        .replace("BOOT_MOUNT", &at(boot))
        .replace("DATA_MOUNT", &at(data))
}

#[test]
fn a_faraday_stick_is_known_by_its_layout_whatever_its_names_and_another_layout_is_not() {
    let known = |text: &str| listed(text)[0].faraday.is_some();
    let stick = faraday_stick(None, None);
    assert!(known(&stick));
    // A stick made before, with Faraday's old names and labels, is the
    // same layout (docs/STICK-IDENTITY.md §5.4).
    let old = stick
        .replace("Name s EFI system partition", "Name s FARADAYBOOT")
        .replace("IdLabel s EFI", "IdLabel s FARADAYBOOT")
        .replace("Name s Basic data partition", "Name s FARADAYDATA");
    assert!(known(&old));
    // Another stick with the same two kinds of partition at other
    // places, other kinds at the same places, or the same in an MBR.
    for other in [
        stick.replace("Offset t 68157440", "Offset t 69206016"),
        stick.replace("Size t 67108864", "Size t 50331648"),
        stick.replace(
            "Type s ebd0a0a2-b9e5-4433-87c0-68b6b72699c7",
            "Type s 0fc63daf-8483-4772-8e79-3d69d8477de4",
        ),
        stick.replace("Type s gpt", "Type s dos"),
    ] {
        assert!(!known(&other), "{other}");
    }
}

/// A folder standing for a mounted partition, holding the PC stick's
/// kernel with `release` in it.
fn partition(name: &str, release: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("starter-disks-{name}"));
    let boot = dir.join("EFI/BOOT");
    std::fs::create_dir_all(&boot).expect("a folder");
    let mut kernel = vec![0x4d_u8; 3 << 20];
    let at = (2 << 20) - 10;
    kernel[at] = 0;
    kernel[at + 1..at + 1 + release.len()].copy_from_slice(release.as_bytes());
    kernel[at + 1 + release.len()] = 0;
    std::fs::write(boot.join("BOOTX64.EFI"), kernel).expect("written");
    dir
}

const BOOT_RELEASE: &str = "6.6.84-faraday-0.2.0+4d0680b1a2b3.test";
const DATA_RELEASE: &str = "6.6.84-faraday-9.9.9+000000000000.test";
const BOOT_OBJECT: &str = "/org/freedesktop/UDisks2/block_devices/sdg1";
const DATA_OBJECT: &str = "/org/freedesktop/UDisks2/block_devices/sdg2";

#[test]
fn a_faraday_sticks_version_is_read_where_the_desktop_mounted_its_boot_partition() {
    let boot = partition("mounted-boot", BOOT_RELEASE);
    // The data partition holds a kernel too: were it read, its version
    // would show.
    let data = partition("mounted-data", DATA_RELEASE);
    let mut bus = Recorded::default();
    let disks = linux::list(
        &mut bus,
        &objects(&faraday_stick(Some(&boot), Some(&data))),
        &mut Versions::default(),
    );
    let stick = &disks[0];
    assert_eq!(
        stick.faraday.as_ref().and_then(|f| f.release.as_deref()),
        Some(BOOT_RELEASE)
    );
    // Nothing mounted, unmounted or opened.
    assert_eq!(bus.calls, Vec::<String>::new());
    // A Faraday stick comes first in the list.
    let mut all = listed(&recorded("attached.txt"));
    all.extend(disks);
    let list = offers(&all, Target::Stick, Some(IMAGE));
    assert_eq!(list[0].disk.device, "/dev/sdg");
}

#[test]
fn an_unmounted_stick_has_its_boot_partition_alone_mounted_read_only_for_the_moment_it_takes() {
    let boot = partition("unmounted-boot", BOOT_RELEASE);
    let data = partition("unmounted-data", DATA_RELEASE);
    let mut bus = Recorded {
        mounts: vec![
            (BOOT_OBJECT.to_string(), boot),
            (DATA_OBJECT.to_string(), data),
        ],
        ..Recorded::default()
    };
    let mut versions = Versions::default();
    let recording = objects(&faraday_stick(None, None));
    let disks = linux::list(&mut bus, &recording, &mut versions);
    assert_eq!(
        disks[0].faraday.as_ref().and_then(|f| f.release.as_deref()),
        Some(BOOT_RELEASE)
    );
    assert_eq!(
        bus.calls,
        [
            format!("mount {BOOT_OBJECT}"),
            format!("unmount {BOOT_OBJECT}")
        ]
    );
    // Read once while the stick stays attached.
    linux::list(&mut bus, &recording, &mut versions);
    assert_eq!(bus.calls.len(), 2);
    assert!(!bus.calls.iter().any(|c| c.contains(DATA_OBJECT)));
}

#[test]
fn a_stick_plugged_in_and_pulled_out_changes_the_list() {
    let without = recorded("attached.txt");
    let with = format!("{without}\n{}", faraday_stick(None, None));
    let mut bus = Recorded {
        answers: vec![objects(&without), objects(&with), objects(&without)],
        ..Recorded::default()
    };
    // The stick's version cannot be read here: it is listed without one.
    bus.mounts.clear();
    let (tx, rx) = channel();
    let (forget, forgotten) = channel::<String>();
    let follower = std::thread::spawn(move || linux::follow(bus, &tx, &forgotten));
    let mut seen = Vec::new();
    for _ in 0..3 {
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(StarterEvent::Disks(d)) => {
                seen.push(d.iter().map(|d| d.device.clone()).collect::<Vec<_>>())
            }
            other => panic!("expected the disks, got {other:?}"),
        }
    }
    drop(forget);
    drop(rx);
    follower.join().expect("the follower ends");
    let has = |list: &Vec<String>| list.contains(&"/dev/sdg".to_string());
    assert!(!has(&seen[0]));
    assert!(has(&seen[1]));
    assert!(!has(&seen[2]));
}

#[test]
fn a_computer_without_udisks2_is_told_it_cannot_write_from_here() {
    struct Absent;
    impl Udisks for Absent {
        fn objects(&mut self) -> Result<Objects, String> {
            Err(String::from("org.freedesktop.DBus.Error.ServiceUnknown"))
        }
        fn wait(&mut self, _: Duration) -> Result<bool, String> {
            Ok(false)
        }
        fn mount_ro(&mut self, _: &str) -> Result<PathBuf, String> {
            unreachable!()
        }
        fn unmount(&mut self, _: &str) -> Result<(), String> {
            unreachable!()
        }
        fn open(&mut self, _: &str, _: Open) -> Result<File, String> {
            unreachable!()
        }
        fn rescan(&mut self, _: &str) -> Result<(), String> {
            unreachable!()
        }
    }
    let (tx, rx) = channel();
    let (_forget, forgotten) = channel::<String>();
    linux::follow(Absent, &tx, &forgotten);
    assert_eq!(rx.try_recv().ok(), Some(StarterEvent::NoWriter));
}

#[test]
fn a_disk_unlocked_by_the_tpm_is_found_in_crypttab() {
    assert!(linux::tpm_unlock(
        "# <name> <device> <key> <options>\nroot UUID=1234 none tpm2-device=auto,discard\n"
    ));
    assert!(!linux::tpm_unlock(
        "root UUID=1234 none luks,discard\n# x y z tpm2-device=auto\n"
    ));
}

#[test]
fn a_disk_the_computer_uses_through_fstab_lvm_or_raid_or_cannot_be_checked_is_refused_and_a_free_one_is_offered()
 {
    let disks = listed(&recorded("in-use.txt"));
    let list = offers(&disks, Target::Stick, Some(IMAGE));
    let refusal = |dev: &str| {
        list.iter()
            .find(|o| o.disk.device == dev)
            .unwrap_or_else(|| panic!("{dev} is listed"))
            .refused
            .clone()
    };
    // Mounted at /mnt/backup: the computer's own place, not the desktop's.
    assert_eq!(
        refusal("/dev/sdh"),
        Some(Refusal::InUse(InUse::Mounted("/mnt/backup".into())))
    );
    // A logical volume on a partition's physical volume.
    assert_eq!(refusal("/dev/sdi"), Some(Refusal::InUse(InUse::Held)));
    // A RAID member, with its array on the whole disk.
    assert_eq!(refusal("/dev/sdj"), Some(Refusal::InUse(InUse::Held)));
    // Nothing says what is built on it.
    assert_eq!(refusal("/dev/sdk"), Some(Refusal::InUse(InUse::Unchecked)));
    // Automounted by the desktop, nothing built on it.
    assert_eq!(refusal("/dev/sdl"), None);
}

#[test]
fn an_opened_encrypted_volume_holds_its_disk_whether_or_not_it_is_mounted() {
    let text = recorded("attached.txt");
    // The cleartext mounted nowhere: still held by the opened volume.
    let unmounted = text.replace("    MountPoints aay /home\n", "    MountPoints aay\n");
    let all = listed(&unmounted);
    let vault = all.iter().find(|d| d.device == "/dev/sdd").expect("listed");
    assert_eq!(vault.in_use, Some(InUse::Held));
}

#[test]
fn holders_are_read_from_the_kernel_and_a_block_it_does_not_list_is_not_checked() {
    use faraday_starter_shell::linux::bus::record_holders;
    let sys = Path::new(env!("CARGO_TARGET_TMPDIR")).join("starter-disks-sys-block");
    let _ = std::fs::remove_dir_all(&sys);
    // sdh and sdh1 are in sysfs, sdh1 held by dm-0; the rest are not.
    for dir in ["sdh/holders", "sdh1/holders/dm-0"] {
        std::fs::create_dir_all(sys.join(dir)).expect("a folder");
    }
    let mut recording = objects(&recorded("in-use.txt"));
    // What the recording stated is replaced by what the kernel says.
    for ifaces in recording.0.values_mut() {
        ifaces.remove("Holders");
    }
    record_holders(&mut recording, &sys);
    let block = |dev: &str| format!("/org/freedesktop/UDisks2/block_devices/{dev}");
    assert_eq!(
        recording.in_use(&block("sdh")),
        Some(InUse::Mounted("/mnt/backup".into()))
    );
    assert_eq!(recording.in_use(&block("sdi")), Some(InUse::Unchecked));
    // Without the mount, sdh1's dm-0 holds the disk.
    recording
        .0
        .get_mut(&block("sdh1"))
        .expect("there")
        .remove("Filesystem");
    assert_eq!(recording.in_use(&block("sdh")), Some(InUse::Held));
}
