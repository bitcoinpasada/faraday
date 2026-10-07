//! The disk process over a stand-in `/sys` and `/dev`: partitions are
//! image files, and a partition is handed out when its node can be
//! opened read-write. What it lists, what it refuses, and the same
//! answers over the real pipes from the real binary.

use std::fs;
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use faraday_disk::Disks;
use faraday_files::proto::{self, Request, Response};

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

fn sticks(r: Response) -> Vec<faraday_files::proto::Stick> {
    match r {
        Response::Sticks(s) => s,
        other => panic!("not a listing: {other:?}"),
    }
}

#[test]
fn only_handed_out_fat_partitions_are_listed_and_never_the_boot_one() {
    let m = Machine::new("list");
    // The boot stick: its boot partition, readable here to show it is
    // refused anyway, and its data partition.
    m.disk(
        "sda",
        3,
        &[
            ("OSKBOOT", fat16("OSKBOOT"), true),
            ("OSKDATA", fat16("OSKDATA"), true),
        ],
    );
    // A stick with one FAT partition, one that is not FAT, and an older
    // OpenSigner stick's boot partition: named `esp`, labelled OSKBOOT,
    // handed out because the grant helper reads no labels.
    m.disk(
        "sdb",
        7,
        &[
            ("", fat16("TESTSTICK"), true),
            ("", vec![0u8; 1 << 20], true),
            ("esp", fat16("OSKBOOT"), true),
        ],
    );
    // A stick inserted while the app was not clean: not handed out, so
    // listed with nothing on it.
    m.disk("sdc", 8, &[("", fat16("LATE"), false)]);

    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(Request::List));
    let shown: Vec<(&str, &str, bool)> = list
        .iter()
        .map(|s| (s.id.as_str(), s.label.as_str(), s.boot))
        .collect();
    assert_eq!(
        shown,
        vec![
            ("sda2@sda#3", "OSKDATA", true),
            ("sdb1@sdb#7", "TESTSTICK", false),
            ("sdc1@sdc#8", "sdc1", false)
        ]
    );
    assert!(list[2].files.is_empty());
    let closed = d.handle(Request::Read {
        stick: "sdc1@sdc#8".into(),
        name: "anything".into(),
    });
    assert!(matches!(closed, Response::Failed(_)));
    let refused = d.handle(Request::Read {
        stick: "sda1@sda#3".into(),
        name: "anything".into(),
    });
    assert!(matches!(refused, Response::Failed(_)));
}

#[test]
fn a_file_written_reads_back_and_a_vault_goes_back_over_its_own() {
    let m = Machine::new("write");
    m.disk("sdb", 7, &[("", fat16("TESTSTICK"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    let id = "sdb1@sdb#7".to_string();
    let w = |name: &str, bytes: &[u8]| Request::Write {
        stick: id.clone(),
        name: name.into(),
        bytes: bytes.to_vec(),
    };
    assert_eq!(
        d.handle(w("savings-signed.psbt", b"psbt bytes")),
        Response::Written("savings-signed.psbt".into())
    );
    // The same name again is a new file beside it.
    assert_eq!(
        d.handle(w("savings-signed.psbt", b"other")),
        Response::Written("savings-signed-2.psbt".into())
    );
    let mut vault = vec![7u8; 53 + 4 * (65_536 + 40)];
    vault[0..4].copy_from_slice(b"OFVT");
    vault[17..21].copy_from_slice(&65_536u32.to_le_bytes());
    vault[21..53].fill(1);
    assert_eq!(
        d.handle(w("vault.ofv", &vault)),
        Response::Written("vault.ofv".into())
    );
    vault[100] = 9;
    assert_eq!(
        d.handle(w("vault.ofv", &vault)),
        Response::Written("vault.ofv".into())
    );
    assert_eq!(
        d.handle(Request::Read {
            stick: id.clone(),
            name: "vault.ofv".into()
        }),
        Response::Bytes(vault.clone())
    );
    // The listing shows what was written, and nothing hidden.
    let list = sticks(d.handle(Request::List));
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
    let m = Machine::new("again");
    m.disk("sdb", 7, &[("", fat16("TESTSTICK"), true)]);
    let mut d = Disks::new(&m.sys(), &m.dev());
    assert_eq!(sticks(d.handle(Request::List))[0].id, "sdb1@sdb#7");
    m.disk("sdb", 9, &[("", fat16("TESTSTICK"), true)]);
    let gone = d.handle(Request::Read {
        stick: "sdb1@sdb#7".into(),
        name: "x".into(),
    });
    assert_eq!(gone, Response::Failed("the stick is gone".into()));
    assert_eq!(sticks(d.handle(Request::List))[0].id, "sdb1@sdb#9");
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
    let m = Machine::new("pi");
    m.disk(
        "mmcblk0",
        1,
        &[("", fat16("OSKBOOT"), true), ("", fat16("OSKDATA"), true)],
    );
    let mut d = Disks::new(&m.sys(), &m.dev());
    let list = sticks(d.handle(Request::List));
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "mmcblk0p2@mmcblk0#1");
    assert!(list[0].boot);
}
