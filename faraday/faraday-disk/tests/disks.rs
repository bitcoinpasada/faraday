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
use faraday_text::Text;

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

    /// The disk process told which medium the machine started from, by
    /// the record `record` as the finder writes it, with the image's data
    /// serial `image_serial`.
    fn disks(&self, record: &str, image_serial: Option<u32>) -> Disks {
        let at = self.root.join("exchange");
        fs::write(&at, record).unwrap();
        Disks::new(&self.sys(), &self.dev()).with_medium(&at, image_serial)
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

    /// What a disk reports under `device/` and `size`; `None` writes no file.
    fn reports(&self, disk: &str, files: &[(&str, &str)]) {
        let d = self.root.join("sys/devices/usb").join(disk);
        fs::create_dir_all(d.join("device")).unwrap();
        for (name, text) in files {
            match *name {
                "size" => fs::write(d.join("size"), text).unwrap(),
                _ => fs::write(d.join("device").join(name), text).unwrap(),
            }
        }
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
    // The boot stick, made before F174: its boot partition, readable here
    // to show it is refused anyway, and its data partition.
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

    let mut d = m.disks("/dev/sda2 3\n", None);
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
            stick: Text::of("sdc1@sdc#8"),
            name: Text::of("anything"),
        },
    );
    assert!(matches!(closed, Response::Failed(_)));
    let refused = d.handle(
        &c,
        Request::Read {
            stick: Text::of("sda1@sda#3"),
            name: Text::of("anything"),
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
    let id = Text::of("sdb1@sdb#7");
    let w = |name: &str, bytes: &[u8]| Request::Write {
        stick: id.clone(),
        name: Text::of(name),
        copies: Vec::new(),
        replace: false,
        renamed: false,
        on_taken: faraday_files::OnTaken::Beside,
        total: bytes.len() as u64,
        bytes: bytes.to_vec(),
    };
    assert_eq!(
        d.handle(&c, w("savings-signed.psbt", b"psbt bytes")),
        Response::Written(Text::of("savings-signed.psbt"))
    );
    // The same name again is a new file beside it.
    assert_eq!(
        d.handle(&c, w("savings-signed.psbt", b"other")),
        Response::Written(Text::of("savings-signed-2.psbt"))
    );
    // Replace, chosen by the person, goes over the old one.
    assert_eq!(
        d.handle(
            &c,
            Request::Write {
                stick: id.clone(),
                name: Text::of("savings-signed-2.psbt"),
                copies: Vec::new(),
                replace: true,
                renamed: false,
                on_taken: faraday_files::OnTaken::Beside,
                total: 8,
                bytes: b"replaced".to_vec(),
            }
        ),
        Response::Written(Text::of("savings-signed-2.psbt"))
    );
    assert_eq!(
        d.handle(
            &c,
            Request::Read {
                stick: id.clone(),
                name: Text::of("savings-signed-2.psbt")
            }
        ),
        Response::Bytes(b"replaced".to_vec())
    );
    // A whole KDBX 4 file of a vault's shape: a header that ends at once,
    // the 64 bytes of its hash and HMAC, one block and the empty block.
    let mut vault = vec![0x03, 0xD9, 0xA2, 0x9A, 0x67, 0xFB, 0x4B, 0xB5, 0, 0, 4, 0];
    vault.extend_from_slice(&[0, 0, 0, 0, 0]);
    vault.extend_from_slice(&[1; 64]);
    vault.extend_from_slice(&[9; 32]);
    vault.extend_from_slice(&65_536u32.to_le_bytes());
    vault.resize(vault.len() + 65_536, 7);
    vault.extend_from_slice(&[9; 32]);
    vault.extend_from_slice(&0u32.to_le_bytes());
    let body = 12 + 5 + 64 + 36;
    assert_eq!(
        d.handle(&c, w("vault.kdbx", &vault)),
        Response::Written(Text::of("vault.kdbx"))
    );
    let first = faraday_crypto::sha256(&vault);
    vault[body + 100] = 9;
    // Over a copy only where the app says its session wrote it.
    let back = |replace: bool| Request::Write {
        stick: id.clone(),
        name: Text::of("vault.kdbx"),
        copies: vec![first],
        replace,
        renamed: false,
        on_taken: faraday_files::OnTaken::Beside,
        total: vault.len() as u64,
        bytes: vault.clone(),
    };
    assert!(matches!(d.handle(&c, back(false)), Response::Failed(_)));
    assert_eq!(
        d.handle(&c, back(true)),
        Response::Written(Text::of("vault.kdbx"))
    );
    assert_eq!(
        d.handle(
            &c,
            Request::Read {
                stick: id.clone(),
                name: Text::of("vault.kdbx")
            }
        ),
        Response::Bytes(vault.clone())
    );
    // The listing shows what was written, and nothing hidden.
    let list = sticks(d.handle(&c, Request::List));
    let names: Vec<&str> = list[0].files.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["savings-signed-2.psbt", "savings-signed.psbt", "vault.kdbx"]
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
            stick: Text::of("sdb1@sdb#7"),
            name: Text::of("x"),
        },
    );
    assert_eq!(gone, Response::Failed(Text::of("the stick is gone")));
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

#[test]
fn a_locked_card_that_cannot_be_opened_is_listed_as_locked_not_as_waiting() {
    let c = compute();
    let m = Machine::new("locked");
    // A card with its lock switch set, which this process cannot open
    // for writing: it is locked, and never comes to be handed out.
    m.disk("sdb", 4, &[("", fat16("LOCKED"), false)]);
    m.read_only("sdb", Some("sdb1"));
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].problem, Some(Problem::ReadOnly));
}

/// A FAT16 volume as [`fat16`] makes one, labelled `label` in its root
/// directory alone, as Windows names a drive, its boot sector saying
/// `NO NAME`.
fn named_by_windows(label: &str) -> Vec<u8> {
    let mut v = faraday_fat::Volume::open(fat16("NO NAME")).unwrap();
    v.set_label(faraday_fat::label_bytes(label).unwrap())
        .unwrap();
    let mut b = v.into_disk();
    b[43..54].copy_from_slice(b"NO NAME    ");
    b
}

/// A Faraday stick as F174 makes it (`docs/STICK-IDENTITY.md` §3): the
/// boot medium is the disk the record names, its first partition is never
/// listed, and every other stick's boot partition is refused by its name
/// or its label, a label Windows wrote included; its data partition, with
/// no label, is listed by its GPT name, as any stick with none is.
#[test]
fn the_boot_medium_is_the_one_the_record_names_and_no_boot_partition_is_listed() {
    let c = compute();
    let m = Machine::new("medium");
    let faraday = |disk: &str, seq: u32| {
        m.disk(
            disk,
            seq,
            &[
                ("EFI system partition", fat16("EFI"), true),
                ("Basic data partition", fat16("NO NAME"), true),
            ],
        );
    };
    faraday("sda", 3);
    faraday("sdb", 5);
    // An ordinary stick whose first partition someone renamed, labelled
    // EFI; and one whose only FAT volume Windows named bootfs.
    m.disk(
        "sdc",
        6,
        &[("", fat16("EFI"), true), ("", fat16("PHOTOS"), true)],
    );
    m.disk("sdd", 7, &[("", named_by_windows("bootfs"), true)]);
    let mut d = m.disks("/dev/sdb2 5\n", None);
    let list = sticks(d.handle(&c, Request::List));
    let shown: Vec<(&str, &str, bool)> = list
        .iter()
        .map(|s| (s.id.as_str(), s.label.as_str(), s.boot))
        .collect();
    assert_eq!(
        shown,
        vec![
            ("sda2@sda#3", "Basic data partition", false),
            ("sdb2@sdb#5", "Basic data partition", true),
            ("sdc2@sdc#6", "PHOTOS", false),
        ]
    );
    // The same disk under another sequence number is another stick.
    let mut d = m.disks("/dev/sdb2 9\n", None);
    assert!(sticks(d.handle(&c, Request::List)).iter().all(|s| !s.boot));
    // Two sticks of the build in, or none: no boot medium at all.
    for record in ["two\n", "none\n", "layout\n"] {
        let mut d = m.disks(record, None);
        let list = sticks(d.handle(&c, Request::List));
        assert!(list.iter().all(|s| !s.boot), "{record}");
        assert!(list.iter().all(|s| !s.id.starts_with("sdb1")), "{record}");
    }
}

/// A stick written with dd has the image's data serial; at its first
/// start the device gives it one of its own, and keeps it at the next
/// (`docs/STICK-IDENTITY.md` §7). Another stick with that serial is left
/// as it is.
#[test]
fn a_stick_written_by_another_tool_gets_its_own_serial_once() {
    let c = compute();
    let m = Machine::new("serial");
    let image = 0x5678_ABCD_u32;
    let with_image_serial = || {
        let mut b = fat16("NO NAME");
        b[39..43].copy_from_slice(&image.to_le_bytes());
        b
    };
    m.disk(
        "sda",
        3,
        &[
            ("EFI system partition", fat16("EFI"), true),
            ("Basic data partition", with_image_serial(), true),
        ],
    );
    m.disk("sdb", 4, &[("", with_image_serial(), true)]);
    let serial_of =
        |list: &[proto::Stick], id: &str| list.iter().find(|s| s.id == id).and_then(|s| s.serial);
    let own = 0x0BAD_CAFE;
    let mut d = m
        .disks("/dev/sda2 3\n", Some(image))
        .with_serials(Box::new(move || Some(own)));
    let list = sticks(d.handle(&c, Request::List));
    assert_eq!(serial_of(&list, "sda2@sda#3"), Some(own));
    assert_eq!(serial_of(&list, "sdb1@sdb#4"), Some(image));
    // On the stick itself, where the next start reads it.
    let bytes = fs::read(m.dev().join("sda2")).unwrap();
    assert_eq!(faraday_fat::boot_sector_serial(&bytes[..512]), Some(own));
    let mut next = m
        .disks("/dev/sda2 3\n", Some(image))
        .with_serials(Box::new(|| Some(0x1111_2222)));
    let list = sticks(next.handle(&c, Request::List));
    assert_eq!(serial_of(&list, "sda2@sda#3"), Some(own));
}

/// A second Faraday stick plugged in after the device started from
/// another (`docs/STICK-IDENTITY.md` §5.2) is not the boot medium: its
/// data partition is listed as any stick's and can be written, and its
/// boot partition is never listed nor opened.
#[test]
fn a_second_faraday_stick_is_not_the_boot_medium_and_its_data_partition_is_usable() {
    let c = compute();
    let m = Machine::new("second");
    for (disk, seq) in [("sda", 3), ("sdb", 11)] {
        m.disk(
            disk,
            seq,
            &[
                ("EFI system partition", fat16("EFI"), true),
                ("Basic data partition", fat16("NO NAME"), true),
            ],
        );
    }
    let mut d = m.disks("/dev/sda2 3\n", None);
    let list = sticks(d.handle(&c, Request::List));
    // The boot flags of two Faraday sticks are the medium test's.
    assert_eq!(list[1].id, "sdb2@sdb#11");
    assert_eq!(list[1].problem, None);
    let wrote = d.handle(
        &c,
        Request::Write {
            stick: Text::of("sdb2@sdb#11"),
            name: Text::of("backup.kdbx"),
            copies: Vec::new(),
            replace: false,
            renamed: false,
            on_taken: faraday_files::OnTaken::Beside,
            total: 7,
            bytes: b"a vault".to_vec(),
        },
    );
    assert!(matches!(wrote, Response::Written(_)), "{wrote:?}");
    let refused = d.handle(
        &c,
        Request::Read {
            stick: Text::of("sdb1@sdb#11"),
            name: Text::of("anything"),
        },
    );
    assert!(matches!(refused, Response::Failed(_)));
}

#[test]
fn a_sticks_maker_model_and_size_are_read_from_its_disk() {
    let c = compute();
    let m = Machine::new("make-usb");
    m.disk("sdb", 1, &[("", fat16("BACKUP"), true)]);
    m.reports(
        "sdb",
        &[
            ("vendor", "SanDisk \n"),
            ("model", "Ultra           \n"),
            ("size", "31260672\n"),
        ],
    );
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    assert_eq!(list[0].make.maker.as_deref(), Some("SanDisk"));
    assert_eq!(list[0].make.model.as_deref(), Some("Ultra"));
    assert_eq!(list[0].make.size, Some(31_260_672 * 512));
}

#[test]
fn a_cards_name_and_size_are_read_and_it_has_no_maker() {
    let c = compute();
    let m = Machine::new("make-sd");
    m.disk(
        "mmcblk1",
        2,
        &[("", fat16("FARADAYBOOT"), true), ("", fat16("DATA"), true)],
    );
    m.reports("mmcblk1", &[("name", "SD32G\n"), ("size", "62333952\n")]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    let s = list.iter().find(|s| s.label == "DATA").unwrap();
    assert_eq!(s.make.maker, None);
    assert_eq!(s.make.model.as_deref(), Some("SD32G"));
    assert_eq!(s.make.size, Some(62_333_952 * 512));
}

#[test]
fn a_disk_that_reports_nothing_is_listed_with_nothing_told() {
    let c = compute();
    let m = Machine::new("make-none");
    m.disk("sdc", 3, &[("", fat16("PLAIN"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].make, faraday_files::proto::Make::default());
}

#[test]
fn a_maker_or_model_that_is_not_plain_text_is_not_shown() {
    let c = compute();
    let m = Machine::new("make-odd");
    m.disk("sdd", 4, &[("", fat16("ODD"), true)]);
    m.reports(
        "sdd",
        &[
            ("vendor", "Real\u{202e}kniS\n"),
            ("model", "Line\u{7}bell\n"),
            ("size", "31260672\n"),
        ],
    );
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    assert_eq!(list[0].make.maker, None);
    assert_eq!(list[0].make.model, None);
    assert_eq!(list[0].make.size, Some(31_260_672 * 512));
}

/// A file sent in pieces is written as the whole of them, and read back in
/// pieces it is the same bytes.
#[test]
fn a_file_sent_in_pieces_is_written_whole_and_read_back_in_pieces() {
    let c = compute();
    let m = Machine::new("pieces");
    m.disk("sdb", 1, &[("", fat16("BACKUP"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(&c, Request::List));
    let stick = list[0].id.clone();
    let file: Vec<u8> = (0..3000).map(|i| (i % 253) as u8).collect();
    let mut at = 0u64;
    for piece in file[..2000].chunks(700) {
        let held = d.handle(
            &c,
            Request::Piece {
                stick: stick.clone(),
                name: Text::of("big.bin"),
                at,
                total: file.len() as u64,
                bytes: piece.to_vec(),
            },
        );
        at += piece.len() as u64;
        assert_eq!(held, Response::Held(at));
    }
    let wrote = d.handle(
        &c,
        Request::Write {
            stick: stick.clone(),
            name: Text::of("big.bin"),
            copies: Vec::new(),
            replace: false,
            renamed: false,
            on_taken: faraday_files::OnTaken::Beside,
            total: file.len() as u64,
            bytes: file[2000..].to_vec(),
        },
    );
    assert!(matches!(wrote, Response::Written(_)), "{wrote:?}");
    let mut got = Vec::new();
    while got.len() < file.len() {
        match d.handle(
            &c,
            Request::ReadPart {
                stick: stick.clone(),
                name: Text::of("big.bin"),
                from: got.len() as u64,
                len: 1000,
            },
        ) {
            Response::Part { total, bytes } => {
                assert_eq!(total, file.len() as u64);
                assert!(!bytes.is_empty());
                got.extend_from_slice(&bytes);
            }
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(got, file);
    // A piece out of order drops what was held and is refused.
    let skipped = d.handle(
        &c,
        Request::Piece {
            stick,
            name: Text::of("x.bin"),
            at: 5,
            total: 9,
            bytes: vec![1],
        },
    );
    assert!(matches!(skipped, Response::Failed(_)));
}

/// Request helpers for the pieced writes below.
fn piece(stick: &str, name: &str, at: u64, total: u64, bytes: &[u8]) -> Request {
    Request::Piece {
        stick: Text::of(stick),
        name: Text::of(name),
        at,
        total,
        bytes: bytes.to_vec(),
    }
}

fn last(stick: &str, name: &str, total: u64, bytes: &[u8]) -> Request {
    Request::Write {
        stick: Text::of(stick),
        name: Text::of(name),
        copies: Vec::new(),
        replace: true,
        renamed: false,
        on_taken: faraday_files::OnTaken::Beside,
        total,
        bytes: bytes.to_vec(),
    }
}

fn read_all(d: &mut Disks, c: &faraday_crypto::work::Compute, stick: &str, name: &str) -> Vec<u8> {
    match d.handle(
        c,
        Request::ReadPart {
            stick: Text::of(stick),
            name: Text::of(name),
            from: 0,
            len: 1 << 20,
        },
    ) {
        Response::Part { bytes, .. } => bytes,
        other => panic!("{other:?}"),
    }
}

/// A write cut off after its first piece, followed by a shorter write of
/// the same name, leaves the stick holding the second file alone.
#[test]
fn a_write_cut_off_part_way_never_joins_the_next_one() {
    let c = compute();
    let m = Machine::new("cutoff");
    m.disk("sdb", 1, &[("", fat16("BACKUP"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let stick = sticks(d.handle(&c, Request::List))[0].id.clone();
    assert_eq!(
        d.handle(&c, piece(&stick, "v.bin", 0, 3000, &[1; 2000])),
        Response::Held(2000)
    );
    // The next write says it is the whole file.
    let wrote = d.handle(&c, last(&stick, "v.bin", 500, &[2; 500]));
    assert!(matches!(wrote, Response::Written(_)), "{wrote:?}");
    assert_eq!(read_all(&mut d, &c, &stick, "v.bin"), vec![2; 500]);
    // A last piece that does not make the stated total is refused.
    d.handle(&c, piece(&stick, "w.bin", 0, 3000, &[1; 2000]));
    let short = d.handle(&c, last(&stick, "w.bin", 3000, &[2; 500]));
    assert!(matches!(short, Response::Failed(_)), "{short:?}");
    // A new app process drops what was held.
    d.handle(&c, piece(&stick, "x.bin", 0, 3000, &[1; 2000]));
    d.forget_held();
    let alone = d.handle(&c, last(&stick, "x.bin", 1000, &[3; 1000]));
    assert!(matches!(alone, Response::Written(_)), "{alone:?}");
    assert_eq!(read_all(&mut d, &c, &stick, "x.bin"), vec![3; 1000]);
    // A total that changes between pieces, or is over the longest vault
    // file, is refused.
    d.handle(&c, piece(&stick, "y.bin", 0, 3000, &[1; 1000]));
    let changed = d.handle(&c, piece(&stick, "y.bin", 1000, 4000, &[1; 1000]));
    assert!(matches!(changed, Response::Failed(_)), "{changed:?}");
    let huge = d.handle(
        &c,
        piece(&stick, "z.bin", 0, faraday_files::MAX_VAULT_READ + 1, &[1]),
    );
    assert!(matches!(huge, Response::Failed(_)), "{huge:?}");
}

/// Another file written while pieces are held lands, and the pieced file
/// lands whole.
#[test]
fn another_file_written_between_pieces_leaves_them_alone() {
    let c = compute();
    let m = Machine::new("between");
    m.disk("sdb", 1, &[("", fat16("BACKUP"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let stick = sticks(d.handle(&c, Request::List))[0].id.clone();
    d.handle(&c, piece(&stick, "v.bin", 0, 3000, &[1; 2000]));
    let other = d.handle(&c, last(&stick, "settings.json", 2, b"{}"));
    assert!(matches!(other, Response::Written(_)), "{other:?}");
    let wrote = d.handle(&c, last(&stick, "v.bin", 3000, &[4; 1000]));
    assert!(matches!(wrote, Response::Written(_)), "{wrote:?}");
    let mut want = vec![1; 2000];
    want.extend_from_slice(&[4; 1000]);
    assert_eq!(read_all(&mut d, &c, &stick, "v.bin"), want);
    assert_eq!(read_all(&mut d, &c, &stick, "settings.json"), b"{}");
}

/// A pieced seal cut off after some pieces, then the same name sent again
/// from its first piece, lands whole, with nothing of the first try in it.
#[test]
fn a_pieced_write_sent_again_after_a_cut_off_one_lands() {
    let c = compute();
    let m = Machine::new("retry");
    m.disk("sdb", 1, &[("", fat16("BACKUP"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let stick = sticks(d.handle(&c, Request::List))[0].id.clone();
    assert_eq!(
        d.handle(&c, piece(&stick, "v.bin", 0, 3000, &[1; 1000])),
        Response::Held(1000)
    );
    assert_eq!(
        d.handle(&c, piece(&stick, "v.bin", 1000, 3000, &[1; 1000])),
        Response::Held(2000)
    );
    // The app gave up; its next seal of the name starts at 0.
    assert_eq!(
        d.handle(&c, piece(&stick, "v.bin", 0, 3000, &[5; 1000])),
        Response::Held(1000)
    );
    assert_eq!(
        d.handle(&c, piece(&stick, "v.bin", 1000, 3000, &[5; 1000])),
        Response::Held(2000)
    );
    let wrote = d.handle(&c, last(&stick, "v.bin", 3000, &[6; 1000]));
    assert!(matches!(wrote, Response::Written(_)), "{wrote:?}");
    let mut want = vec![5; 2000];
    want.extend_from_slice(&[6; 1000]);
    assert_eq!(read_all(&mut d, &c, &stick, "v.bin"), want);
}

/// A pieced seal abandoned leaves nothing for a later write to join: after
/// the app's side has gone (`forget_held`) or its stick has been pulled, a
/// last piece for that name is refused and writes no file made of the old
/// pieces.
#[test]
fn an_abandoned_pieced_write_leaves_nothing_to_join() {
    let c = compute();
    let m = Machine::new("abandoned");
    m.disk("sdb", 1, &[("", fat16("BACKUP"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let stick = sticks(d.handle(&c, Request::List))[0].id.clone();
    // The app's side gone: the shell closes.
    assert_eq!(
        d.handle(&c, piece(&stick, "v.bin", 0, 3000, &[1; 2000])),
        Response::Held(2000)
    );
    d.forget_held();
    let late = d.handle(&c, last(&stick, "v.bin", 3000, &[2; 1000]));
    assert!(matches!(late, Response::Failed(_)), "{late:?}");
    let read = d.handle(
        &c,
        Request::Read {
            stick: stick.clone(),
            name: Text::of("v.bin"),
        },
    );
    assert!(
        matches!(read, Response::Failed(_)),
        "a file of old pieces: {read:?}"
    );
    // The stick pulled with pieces held: the next listing lets them go.
    d.handle(&c, piece(&stick, "x.bin", 0, 3000, &[1; 2000]));
    for node in ["sdb", "sdb1"] {
        let _ = std::fs::remove_file(m.sys().join("class/block").join(node));
    }
    let _ = d.handle(&c, Request::List);
    let late = d.handle(&c, last(&stick, "x.bin", 3000, &[2; 1000]));
    assert!(matches!(late, Response::Failed(_)), "{late:?}");
}

/// What a node reads after a flush, from the medium: any offset and
/// length, aligned or not, returns what was written. A node of a file
/// system that takes no `O_DIRECT` reads as it did.
#[test]
fn a_node_reads_back_what_was_written_at_any_offset_and_length() {
    use faraday_fat::Disk;
    let dir = std::env::temp_dir().join(format!("faraday-node-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("node.img");
    fs::write(&path, vec![0u8; 20_000]).unwrap();
    let mut node = faraday_disk::Node::open(&path).unwrap();
    let data: Vec<u8> = (0..9_001u32).map(|i| (i % 251) as u8).collect();
    node.write_at(4_093, &data).unwrap();
    node.flush().unwrap();
    for (at, len) in [
        (4_093u64, 9_001usize),
        (4_096, 5),
        (0, 20_000),
        (19_999, 1),
        (5_000, 4_096),
    ] {
        let mut got = vec![0u8; len];
        node.read_back(at, &mut got).unwrap();
        let mut want = vec![0u8; 20_000];
        want[4_093..4_093 + data.len()].copy_from_slice(&data);
        assert_eq!(got, want[at as usize..at as usize + len], "at {at}, {len}");
    }
    let _ = fs::remove_dir_all(&dir);
}
