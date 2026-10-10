//! The boot copier over a stand-in `/sys`, `/dev` and `/proc/version`:
//! partitions are image files, and a partition is handed out when its
//! node can be opened. Which stick it takes as the source, what it
//! refuses to write, what a write leaves behind, and the same answers
//! over the real pipes from the real binary.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use faraday_boot::Boots;
use faraday_boot::client::Client;
use faraday_boot::proto::{self, Part, Request, Response};

/// The running kernel's release: what the image build makes of the
/// Faraday version and commit.
const RUNNING: &str = "6.6.84-faraday-0.2.0+4d0680b1a2b3";

/// `/proc/version` on a machine running [`RUNNING`].
fn proc_version() -> String {
    format!(
        "Linux version {RUNNING} (user@host) (x86_64-buildroot-linux-musl-gcc 12.4.0) #1 SMP PREEMPT_DYNAMIC Thu Apr 17 13:23:26 UTC 2025\n"
    )
}

/// A boot partition of `len` bytes carrying `release` where a bzImage's
/// setup code does, as `6.6.84… (user@host) #1 …`, among bytes that are
/// not text.
fn boot_image(len: usize, release: &str, seed: u8) -> Vec<u8> {
    let mut b: Vec<u8> = (0..len)
        .map(|i| (i as u32).wrapping_mul(2_654_435_761).to_le_bytes()[3] ^ seed)
        .collect();
    // A bzImage's kernel_version string, after a FAT and a PE header's
    // worth of other bytes.
    let at = 300_000.min(len / 2);
    let text = format!("{release} (user@host) #1 SMP PREEMPT_DYNAMIC\0");
    b[at - 1] = 0;
    b[at..at + text.len()].copy_from_slice(text.as_bytes());
    b
}

/// A data partition: a vault and a settings file, as far as this test
/// cares.
fn data_image(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

struct Machine {
    root: PathBuf,
}

impl Machine {
    fn new(name: &str) -> Machine {
        let root = std::env::temp_dir().join(format!("faraday-boot-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sys/class/block")).unwrap();
        fs::create_dir_all(root.join("dev")).unwrap();
        fs::write(root.join("version"), proc_version()).unwrap();
        Machine { root }
    }

    fn boots(&self) -> Boots {
        Boots::new(
            &self.root.join("sys"),
            &self.root.join("dev"),
            &self.root.join("version"),
        )
    }

    /// A disk on `bus` with partitions of (GPT name, image, handed out to
    /// the copier).
    fn disk(&self, bus: &str, disk: &str, seq: u32, parts: &[(&str, Vec<u8>, bool)]) {
        let d = self.root.join("sys/devices").join(bus).join(disk);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("diskseq"), format!("{seq}\n")).unwrap();
        for (k, (partname, image, handed)) in parts.iter().enumerate() {
            let node = format!("{disk}{}", k + 1);
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
            let dev = self.root.join("dev").join(&node);
            fs::write(&dev, image).unwrap();
            let mode = if *handed { 0o600 } else { 0o000 };
            fs::set_permissions(&dev, fs::Permissions::from_mode(mode)).unwrap();
        }
    }

    /// A Faraday stick on USB: its boot partition and its data partition,
    /// both handed out as the grant helper does in the upgrade flow.
    fn stick(&self, disk: &str, seq: u32, boot: Vec<u8>, data: Vec<u8>) {
        let bus = format!("pci0/usb1/1-{seq}/host{seq}/block");
        self.disk(
            &bus,
            disk,
            seq,
            &[("OSKBOOT", boot, true), ("OSKDATA", data, true)],
        );
    }

    fn node(&self, name: &str) -> Vec<u8> {
        fs::read(self.root.join("dev").join(name)).unwrap()
    }

    /// The stick is pulled: its `/sys` entries and node go.
    fn pull(&self, disk: &str) {
        for e in fs::read_dir(self.root.join("sys/class/block")).unwrap() {
            let e = e.unwrap();
            if e.file_name().to_string_lossy().starts_with(disk) {
                fs::remove_file(e.path()).unwrap();
                let _ = fs::remove_file(self.root.join("dev").join(e.file_name()));
            }
        }
    }
}

impl Drop for Machine {
    /// The tree is a test's own; leaving it fills the temp directory.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn parts(r: Response) -> Vec<Part> {
    match r {
        Response::Parts(p) => p,
        other => panic!("not a listing: {other:?}"),
    }
}

const MIB: usize = 1 << 20;

#[test]
fn the_stick_holding_the_running_faraday_is_the_source_whenever_it_went_in() {
    let m = Machine::new("source");
    // An older stick went in first, then the stick Faraday started from,
    // pulled after boot and put back: it has another disk number, and the
    // copier knows it by what it holds.
    m.stick("sdb", 5, boot_image(2 * MIB, "6.6.84", 1), data_image(MIB));
    let source = boot_image(3 * MIB + 17, RUNNING, 2);
    m.stick("sdc", 9, source.clone(), data_image(MIB));
    let mut b = m.boots();
    let read = b.handle(Request::ReadSource);
    assert_eq!(
        read,
        Response::Source {
            id: "sdc1@sdc#9".into(),
            release: RUNNING.into(),
            size: source.len() as u64,
        }
    );
    let list = parts(b.handle(Request::List));
    assert_eq!(
        list,
        vec![
            Part {
                id: "sdb1@sdb#5".into(),
                size: 2 * MIB as u64,
                release: None,
                source: false,
            },
            Part {
                id: "sdc1@sdc#9".into(),
                size: source.len() as u64,
                release: Some(RUNNING.into()),
                source: true,
            },
        ]
    );
}

#[test]
fn a_stick_carrying_another_release_is_not_the_source() {
    let m = Machine::new("other");
    // An older Faraday with a version, one whose release only starts with
    // the running one, and one with none at all.
    m.stick(
        "sdb",
        2,
        boot_image(MIB, "6.6.84-faraday-0.1.9+0123456789ab", 3),
        data_image(MIB),
    );
    m.stick(
        "sdc",
        3,
        boot_image(MIB, &format!("{RUNNING}.dirty"), 4),
        data_image(MIB),
    );
    m.stick("sdd", 4, boot_image(MIB, "6.6.84", 5), data_image(MIB));
    let mut b = m.boots();
    assert_eq!(
        b.handle(Request::ReadSource),
        Response::Failed("No stick in holds this Faraday".into())
    );
    // Each one's version is still read, for the app to show.
    let releases: Vec<Option<String>> = parts(b.handle(Request::List))
        .into_iter()
        .map(|p| p.release)
        .collect();
    assert_eq!(
        releases,
        vec![
            Some("6.6.84-faraday-0.1.9+0123456789ab".into()),
            Some(format!("{RUNNING}.dirty")),
            None,
        ]
    );
    // Nothing read, nothing to write.
    assert_eq!(
        b.handle(Request::Write {
            target: "sdd1@sdd#4".into()
        }),
        Response::Failed("The stick Faraday started from has not been read".into())
    );
    assert_eq!(m.node("sdd1"), boot_image(MIB, "6.6.84", 5));
}

#[test]
fn a_kernel_that_names_no_faraday_version_has_no_source() {
    let m = Machine::new("unversioned");
    fs::write(
        m.root.join("version"),
        "Linux version 6.6.84 (buildroot@buildroot) #1 SMP\n",
    )
    .unwrap();
    m.stick("sda", 1, boot_image(MIB, "6.6.84", 1), data_image(MIB));
    assert_eq!(
        m.boots().handle(Request::ReadSource),
        Response::Failed("This Faraday's kernel carries no Faraday version".into())
    );
}

#[test]
fn the_copy_is_written_read_back_and_the_data_partition_is_untouched() {
    let m = Machine::new("write");
    let source = boot_image(3 * MIB + 17, RUNNING, 7);
    m.stick("sda", 1, source.clone(), data_image(MIB));
    // The stick to upgrade: a larger boot partition than the source's,
    // and a data partition holding its vaults.
    let old = boot_image(4 * MIB, "6.6.84", 8);
    let vaults = data_image(2 * MIB + 3);
    m.stick("sdb", 2, old.clone(), vaults.clone());
    let mut b = m.boots();
    assert!(matches!(
        b.handle(Request::ReadSource),
        Response::Source { .. }
    ));
    assert_eq!(
        b.handle(Request::Write {
            target: "sdb1@sdb#2".into()
        }),
        Response::Written {
            id: "sdb1@sdb#2".into(),
            release: RUNNING.into(),
        }
    );
    let now = m.node("sdb1");
    assert_eq!(now.len(), old.len());
    assert!(now[..source.len()] == source[..], "the source's bytes");
    assert!(
        now[source.len()..] == old[source.len()..],
        "past the source, as it was"
    );
    assert_eq!(m.node("sdb2"), vaults, "the data partition");
    assert_eq!(m.node("sda1"), source, "the source stick");
    // It now shows as this Faraday.
    let list = parts(b.handle(Request::List));
    assert_eq!(list[1].release.as_deref(), Some(RUNNING));
    // The source, once forgotten, writes nothing more.
    assert_eq!(b.handle(Request::Forget), Response::Forgotten);
    assert!(matches!(
        b.handle(Request::Write {
            target: "sdb1@sdb#2".into()
        }),
        Response::Failed(_)
    ));
}

#[test]
fn a_target_too_small_the_source_itself_and_a_stick_gone_are_refused() {
    let m = Machine::new("refused");
    let source = boot_image(3 * MIB, RUNNING, 9);
    m.stick("sda", 1, source, data_image(MIB));
    let small = boot_image(2 * MIB, "6.6.84", 10);
    m.stick("sdb", 2, small.clone(), data_image(MIB));
    let mut b = m.boots();
    assert!(matches!(
        b.handle(Request::ReadSource),
        Response::Source { .. }
    ));
    assert_eq!(
        b.handle(Request::Write {
            target: "sdb1@sdb#2".into()
        }),
        Response::Failed("Its boot partition is 2 MB; this Faraday's is 3 MB".into())
    );
    assert_eq!(m.node("sdb1"), small);
    assert_eq!(
        b.handle(Request::Write {
            target: "sda1@sda#1".into()
        }),
        Response::Failed("That is the stick being copied".into())
    );
    m.pull("sdb");
    assert_eq!(
        b.handle(Request::Write {
            target: "sdb1@sdb#2".into()
        }),
        Response::Failed("The stick is gone".into())
    );
}

#[test]
fn only_a_handed_out_boot_partition_of_a_faraday_stick_on_usb_is_touched() {
    let m = Machine::new("layout");
    m.stick("sda", 1, boot_image(3 * MIB, RUNNING, 11), data_image(MIB));
    let image = boot_image(4 * MIB, "6.6.84", 12);
    // Not handed out: the upgrade marker was not up.
    m.disk(
        "pci0/usb1/1-3/block",
        "sdc",
        3,
        &[
            ("OSKBOOT", image.clone(), false),
            ("OSKDATA", data_image(MIB), true),
        ],
    );
    // A stick with a partition named OSKBOOT and nothing beside it, and
    // one with a third partition.
    m.disk(
        "pci0/usb1/1-4/block",
        "sdd",
        4,
        &[("OSKBOOT", image.clone(), true)],
    );
    m.disk(
        "pci0/usb1/1-5/block",
        "sde",
        5,
        &[
            ("OSKBOOT", image.clone(), true),
            ("OSKDATA", data_image(MIB), true),
            ("", data_image(MIB), true),
        ],
    );
    // The laptop's own disk, laid out as a Faraday stick.
    m.disk(
        "pci0/0000:00:1f.2/ata1/block",
        "sdf",
        6,
        &[
            ("OSKBOOT", image.clone(), true),
            ("OSKDATA", data_image(MIB), true),
        ],
    );
    let mut b = m.boots();
    let ids: Vec<String> = parts(b.handle(Request::List))
        .into_iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(ids, vec!["sda1@sda#1".to_string()]);
    assert!(matches!(
        b.handle(Request::ReadSource),
        Response::Source { .. }
    ));
    for target in ["sdc1@sdc#3", "sdd1@sdd#4", "sde1@sde#5", "sdf1@sdf#6"] {
        assert_eq!(
            b.handle(Request::Write {
                target: target.into()
            }),
            Response::Failed("The stick is gone".into()),
            "{target}"
        );
    }
    for node in ["sdd1", "sde1", "sdf1"] {
        assert_eq!(m.node(node), image, "{node}");
    }
}

fn fifo(p: &Path) {
    let ok = std::process::Command::new("mkfifo")
        .arg(p)
        .status()
        .unwrap();
    assert!(ok.success());
}

#[test]
fn the_binary_upgrades_a_stick_over_its_pipes_for_one_shell_after_another() {
    let m = Machine::new("pipes");
    let source = boot_image(2 * MIB, RUNNING, 13);
    m.stick("sda", 1, source.clone(), data_image(MIB));
    m.stick("sdb", 2, boot_image(2 * MIB, "6.6.84", 14), data_image(MIB));
    let req = m.root.join("requests");
    let resp = m.root.join("responses");
    fifo(&req);
    fifo(&resp);
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_faraday-boot"))
        .arg(&req)
        .arg(&resp)
        .arg(m.root.join("sys"))
        .arg(m.root.join("dev"))
        .arg(m.root.join("version"))
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    // One shell reads the source, and goes (the app locked). Its ends of
    // the pipes are opened by hand, so nothing of it is left reading
    // when it has gone.
    {
        let open = |p: &Path| {
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(p)
                .unwrap()
        };
        let mut tx = open(&req);
        let mut rx = open(&resp);
        proto::send(&mut tx, 7, &Request::ReadSource.encode()).unwrap();
        let (seq, frame) = proto::receive(&mut rx).unwrap().unwrap();
        assert_eq!(seq, 7);
        assert!(matches!(
            Response::decode(&frame),
            Ok(Response::Source { .. })
        ));
    }
    // The next one finds it still read, and writes.
    let mut c = Client::open(&req, &resp).unwrap();
    let list = match c.ask(&Request::List) {
        Ok(Response::Parts(p)) => p,
        other => panic!("{other:?}"),
    };
    assert!(list[0].source && !list[1].source);
    assert_eq!(
        c.ask(&Request::Write {
            target: "sdb1@sdb#2".into()
        }),
        Ok(Response::Written {
            id: "sdb1@sdb#2".into(),
            release: RUNNING.into(),
        })
    );
    assert_eq!(m.node("sdb1"), source);
    child.kill().unwrap();
    let _ = child.wait();
}
