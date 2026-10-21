//! The disk process over a stand-in `/sys` and `/dev`: partitions are
//! image files, and a partition is handed out when its node can be
//! opened read-write. What it lists, what it refuses, and the same
//! answers over the real pipes from the real binary.

use std::fs;
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use faraday_disk::Disks;
use faraday_files::proto::{self, Problem, Request, Response};

/// A token for the reads these tests ask, counting nothing: the disk
/// process draws nothing. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-disk's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

/// A 4 MiB FAT16 volume with this label, as `mkfs.fat` would make one:
/// 512-byte sectors and clusters, two FATs, a 512-entry root.
fn fat16(label: &str) -> Vec<u8> {
    let total: u32 = 8192;
    let mut b = vec![0u8; total as usize * 512];
    let fat_sectors: u16 = 32;
    b[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]);
    b[3..11].copy_from_slice(b"mkfs.fat");
    b[11..13].copy_from_slice(&512u16.to_le_bytes());
    b[13] = 1;
    b[14..16].copy_from_slice(&1u16.to_le_bytes());
    b[16] = 2;
    b[17..19].copy_from_slice(&512u16.to_le_bytes());
    b[19..21].copy_from_slice(&(total as u16).to_le_bytes());
    b[21] = 0xF8;
    b[22..24].copy_from_slice(&fat_sectors.to_le_bytes());
    b[38] = 0x29;
    let mut l = [b' '; 11];
    l[..label.len()].copy_from_slice(label.as_bytes());
    b[43..54].copy_from_slice(&l);
    b[54..62].copy_from_slice(b"FAT16   ");
    b[510] = 0x55;
    b[511] = 0xAA;
    for fat in 0..2usize {
        let at = 512 + fat * usize::from(fat_sectors) * 512;
        b[at..at + 4].copy_from_slice(&[0xF8, 0xFF, 0xFF, 0xFF]);
    }
    b
}

/// An exFAT volume's boot sector, which Faraday does not read.
fn exfat() -> Vec<u8> {
    let mut b = vec![0u8; 1 << 20];
    b[0..3].copy_from_slice(&[0xEB, 0x76, 0x90]);
    b[3..11].copy_from_slice(b"EXFAT   ");
    b[510] = 0x55;
    b[511] = 0xAA;
    b
}

/// A stand-in machine: `sys/` and `dev/` under a fresh directory.
struct Machine {
    root: PathBuf,
}

impl Machine {
    fn new(name: &str) -> Machine {
        let root = std::env::temp_dir().join(format!("faraday-disk-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sys/class/block")).unwrap();
        fs::create_dir_all(root.join("dev")).unwrap();
        Machine { root }
    }

    fn sys(&self) -> PathBuf {
        self.root.join("sys")
    }

    fn dev(&self) -> PathBuf {
        self.root.join("dev")
    }

    /// A USB disk with these partitions: (partition name, image, handed
    /// out).
    fn disk(&self, disk: &str, seq: u32, parts: &[(&str, Vec<u8>, bool)]) {
        let d = self.root.join("sys/devices/usb").join(disk);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("diskseq"), format!("{seq}\n")).unwrap();
        let _ = fs::remove_file(self.root.join("sys/class/block").join(disk));
        symlink(&d, self.root.join("sys/class/block").join(disk)).unwrap();
        fs::write(self.dev().join(disk), b"whole disk").unwrap();
        for (k, (partname, image, handed)) in parts.iter().enumerate() {
            // The kernel puts a `p` between a disk name ending in a digit
            // and the partition number.
            let p = if disk.ends_with(|c: char| c.is_ascii_digit()) {
                "p"
            } else {
                ""
            };
            let node = format!("{disk}{p}{}", k + 1);
            let p = d.join(&node);
            fs::create_dir_all(&p).unwrap();
            fs::write(p.join("partition"), format!("{}\n", k + 1)).unwrap();
            fs::write(
                p.join("uevent"),
                format!("DEVNAME={node}\nPARTNAME={partname}\n"),
            )
            .unwrap();
            let link = self.root.join("sys/class/block").join(&node);
            let _ = fs::remove_file(&link);
            symlink(&p, &link).unwrap();
            let dev = self.dev().join(&node);
            fs::write(&dev, image).unwrap();
            let mode = if *handed { 0o600 } else { 0o000 };
            fs::set_permissions(&dev, fs::Permissions::from_mode(mode)).unwrap();
        }
    }
}

impl Machine {
    /// Marks a partition or a disk read-only, as the kernel does for a
    /// card with its lock switch set.
    fn read_only(&self, disk: &str, part: Option<&str>) {
        let d = self.root.join("sys/devices/usb").join(disk);
        let at = match part {
            Some(p) => d.join(p),
            None => d,
        };
        fs::write(at.join("ro"), "1\n").unwrap();
    }

    /// A USB disk with no partition table: the whole disk is the volume.
    fn bare(&self, disk: &str, seq: u32, image: Vec<u8>, sectors: u64) {
        let d = self.root.join("sys/devices/usb").join(disk);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("diskseq"), format!("{seq}\n")).unwrap();
        fs::write(d.join("size"), format!("{sectors}\n")).unwrap();
        let _ = fs::remove_file(self.root.join("sys/class/block").join(disk));
        symlink(&d, self.root.join("sys/class/block").join(disk)).unwrap();
        let dev = self.dev().join(disk);
        fs::write(&dev, image).unwrap();
        fs::set_permissions(&dev, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

impl Drop for Machine {
    /// The tree is a test's own; leaving it fills the temp directory.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn sticks(r: Response) -> Vec<faraday_files::proto::Stick> {
    match r {
        Response::Sticks(s) => s,
        other => panic!("not a listing: {other:?}"),
    }
}

#[test]
fn only_handed_out_fat_partitions_are_listed_and_never_the_boot_one() {
    let c = compute();
    let m = Machine::new("list");
    // The boot stick: its boot partition, readable here to show it is
    // refused anyway, and its data partition.
    m.disk(
        "sda",
        3,
        &[
            ("FARADAYBOOT", fat16("FARADAYBOOT"), true),
            ("FARADAYDATA", fat16("FARADAYDATA"), true),
        ],
    );
    // A stick with one FAT partition, one that is not FAT, and a boot
    // partition whose GPT name is not FARADAYBOOT (`esp`) but whose FAT
    // label is: handed out, because the grant helper reads no labels.
    m.disk(
        "sdb",
        7,
        &[
            ("", fat16("TESTSTICK"), true),
            ("", vec![0u8; 1 << 20], true),
            ("esp", fat16("FARADAYBOOT"), true),
        ],
    );
    // A stick inserted while the app was not clean: not handed out, so
    // listed with nothing on it.
    m.disk("sdc", 8, &[("", fat16("LATE"), false)]);

    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    let shown: Vec<(&str, &str, bool)> = list
        .iter()
        .map(|s| (s.id.as_str(), s.label.as_str(), s.boot))
        .collect();
    assert_eq!(
        shown,
        vec![
            ("sda2@sda#3", "FARADAYDATA", true),
            ("sdb1@sdb#7", "TESTSTICK", false),
            // Handed out, not FAT: listed, saying so, with nothing on it.
            ("sdb2@sdb#7", "sdb2", false),
            ("sdc1@sdc#8", "sdc1", false)
        ]
    );
    assert_eq!(list[0].problem, None);
    assert_eq!(list[2].problem, Some(Problem::NotFat));
    assert!(list[2].files.is_empty());
    assert!(list[3].files.is_empty());
    let closed = d.handle(
        &c,
        Request::Read {
            stick: "sdc1@sdc#8".into(),
            name: "anything".into(),
        },
    );
    assert!(matches!(closed, Response::Failed(_)));
    let refused = d.handle(
        &c,
        Request::Read {
            stick: "sda1@sda#3".into(),
            name: "anything".into(),
        },
    );
    assert!(matches!(refused, Response::Failed(_)));
}

#[test]
fn a_file_written_reads_back_and_a_vault_goes_back_over_its_own() {
    let c = compute();
    let m = Machine::new("write");
    m.disk("sdb", 7, &[("", fat16("TESTSTICK"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let id = "sdb1@sdb#7".to_string();
    let w = |name: &str, bytes: &[u8]| Request::Write {
        stick: id.clone(),
        name: name.into(),
        replace: false,
        renamed: false,
        bytes: bytes.to_vec(),
    };
    assert_eq!(
        d.handle(&c, w("savings-signed.psbt", b"psbt bytes")),
        Response::Written("savings-signed.psbt".into())
    );
    // The same name again is a new file beside it.
    assert_eq!(
        d.handle(&c, w("savings-signed.psbt", b"other")),
        Response::Written("savings-signed-2.psbt".into())
    );
    // Replace, chosen by the person, goes over the old one.
    assert_eq!(
        d.handle(
            &c,
            Request::Write {
                stick: id.clone(),
                name: "savings-signed-2.psbt".into(),
                replace: true,
                renamed: false,
                bytes: b"replaced".to_vec(),
            }
        ),
        Response::Written("savings-signed-2.psbt".into())
    );
    assert_eq!(
        d.handle(
            &c,
            Request::Read {
                stick: id.clone(),
                name: "savings-signed-2.psbt".into()
            }
        ),
        Response::Bytes(b"replaced".to_vec())
    );
    let mut vault = vec![7u8; 53 + 4 * (65_536 + 40)];
    vault[0..4].copy_from_slice(b"OFVT");
    vault[17..21].copy_from_slice(&65_536u32.to_le_bytes());
    vault[21..53].fill(1);
    assert_eq!(
        d.handle(&c, w("vault.ofv", &vault)),
        Response::Written("vault.ofv".into())
    );
    vault[100] = 9;
    // Over the same vault only where the app says its session wrote it.
    assert!(matches!(
        d.handle(&c, w("vault.ofv", &vault)),
        Response::Failed(_)
    ));
    assert_eq!(
        d.handle(
            &c,
            Request::Write {
                stick: id.clone(),
                name: "vault.ofv".into(),
                replace: true,
                renamed: false,
                bytes: vault.clone(),
            }
        ),
        Response::Written("vault.ofv".into())
    );
    assert_eq!(
        d.handle(
            &c,
            Request::Read {
                stick: id.clone(),
                name: "vault.ofv".into()
            }
        ),
        Response::Bytes(vault.clone())
    );
    // The listing shows what was written, and nothing hidden.
    let list = sticks(d.handle(&c, Request::List));
    let names: Vec<&str> = list[0].files.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["savings-signed-2.psbt", "savings-signed.psbt", "vault.ofv"]
    );
    // And the partition is FAT another reader takes.
    let mut v = faraday_fat::Volume::open(fs::read(m.dev().join("sdb1")).unwrap()).unwrap();
    assert_eq!(v.read("savings-signed.psbt", 100).unwrap(), b"psbt bytes");
}

#[test]
fn a_stick_put_back_is_another_stick() {
    let c = compute();
    let m = Machine::new("again");
    m.disk("sdb", 7, &[("", fat16("TESTSTICK"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    assert_eq!(sticks(d.handle(&c, Request::List))[0].id, "sdb1@sdb#7");
    m.disk("sdb", 9, &[("", fat16("TESTSTICK"), true)]);
    let gone = d.handle(
        &c,
        Request::Read {
            stick: "sdb1@sdb#7".into(),
            name: "x".into(),
        },
    );
    assert_eq!(gone, Response::Failed("the stick is gone".into()));
    assert_eq!(sticks(d.handle(&c, Request::List))[0].id, "sdb1@sdb#9");
}

#[test]
fn the_binary_answers_over_its_pipes_and_serves_the_next_shell() {
    let m = Machine::new("pipes");
    m.disk("sdb", 7, &[("", fat16("TESTSTICK"), true)]);
    let req = m.root.join("requests");
    let resp = m.root.join("responses");
    for p in [&req, &resp] {
        let ok = std::process::Command::new("mkfifo")
            .arg(p)
            .status()
            .unwrap();
        assert!(ok.success());
    }
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_faraday-disk"))
        .arg(&req)
        .arg(&resp)
        .arg(m.sys())
        .arg(m.dev())
        .spawn()
        .unwrap();
    let open = |p: &Path| {
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(p)
            .unwrap()
    };
    for _shell in 0..2 {
        let mut tx = open(&req);
        let mut rx = open(&resp);
        proto::send(&mut tx, 41, &Request::List.encode()).unwrap();
        let (seq, frame) = proto::receive(&mut rx).unwrap().unwrap();
        assert_eq!(seq, 41);
        let list = sticks(Response::decode(&frame).unwrap());
        assert_eq!(list[0].label, "TESTSTICK");
        // A malformed request ends that conversation; the next shell
        // still gets an answer.
        tx.write_all(&3u32.to_le_bytes()).unwrap();
        tx.write_all(&42u32.to_le_bytes()).unwrap();
        tx.write_all(&[9, 9, 9]).unwrap();
        let (_, frame) = proto::receive(&mut rx).unwrap().unwrap();
        assert!(matches!(Response::decode(&frame), Ok(Response::Failed(_))));
        drop(tx);
        drop(rx);
    }
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn the_pis_own_card_is_the_boot_medium_and_its_first_partition_is_never_listed() {
    let c = compute();
    let m = Machine::new("pi");
    m.disk(
        "mmcblk0",
        1,
        &[
            ("", fat16("FARADAYBOOT"), true),
            ("", fat16("FARADAYDATA"), true),
        ],
    );
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "mmcblk0p2@mmcblk0#1");
    assert!(list[0].boot);
}

#[test]
fn an_exfat_stick_a_locked_card_and_a_bare_disk_are_listed_with_their_problem() {
    let c = compute();
    let m = Machine::new("problems");
    // The factory format of most large sticks: not FAT as Faraday reads it.
    m.disk("sdb", 4, &[("", exfat(), true)]);
    // A card with its lock switch set: FAT, listed, and read-only.
    m.disk("sdc", 5, &[("", fat16("LOCKED"), true)]);
    m.read_only("sdc", Some("sdc1"));
    // The switch can be on the disk instead of the partition.
    m.disk("sdd", 6, &[("", fat16("SWITCH"), true)]);
    m.read_only("sdd", None);
    // A FAT volume made with no partition table, and one that is not FAT.
    m.bare("sde", 7, fat16("BARE"), 8192);
    m.bare("sdf", 8, exfat(), 2048);
    // A card reader with nothing in it has no size and is not a stick.
    m.bare("sdg", 9, Vec::new(), 0);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    let shown: Vec<(&str, Option<Problem>)> =
        list.iter().map(|s| (s.label.as_str(), s.problem)).collect();
    assert_eq!(
        shown,
        vec![
            ("sdb1", Some(Problem::NotFat)),
            ("LOCKED", Some(Problem::ReadOnly)),
            ("SWITCH", Some(Problem::ReadOnly)),
            ("BARE", None),
            ("sdf", Some(Problem::NotFat)),
        ]
    );
    // A read-only stick still shows what is on it.
    assert!(list[1].serial.is_some());
}

#[test]
fn a_stick_not_handed_out_yet_is_listed_as_waiting_not_as_unusable() {
    let c = compute();
    let m = Machine::new("closed");
    m.disk("sdb", 4, &[("", exfat(), false)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    // Not handed out: nothing can be read or written, and it says so
    // instead of waiting to be read.
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].problem, Some(Problem::NotYet));
    assert!(list[0].files.is_empty());
}
