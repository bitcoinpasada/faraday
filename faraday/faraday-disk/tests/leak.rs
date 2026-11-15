//! A file's bytes, read from or written to a stick by the disk process,
//! are in no freed block afterwards: every buffer that held them is wiped
//! before it is freed. The picture reader is not run here: the png and
//! rqrr libraries free their own buffers unwiped.

use std::fs;
use std::mem::ManuallyDrop;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;

use faraday_disk::Disks;
use faraday_files::OnTaken;
use faraday_files::proto::{Request, Response};
use faraday_leak_scan::Form;
use faraday_text::Text;

#[global_allocator]
static ALLOC: faraday_leak_scan::LeakScan = faraday_leak_scan::LeakScan;

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-disk's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

/// A 4 MiB FAT16 volume: 512-byte sectors and clusters, two FATs, a
/// 512-entry root.
fn fat16() -> Vec<u8> {
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
    b[43..54].copy_from_slice(b"LEAKSTICK  ");
    b[54..62].copy_from_slice(b"FAT16   ");
    b[510] = 0x55;
    b[511] = 0xAA;
    for fat in 0..2usize {
        let at = 512 + fat * usize::from(fat_sectors) * 512;
        b[at..at + 4].copy_from_slice(&[0xF8, 0xFF, 0xFF, 0xFF]);
    }
    b
}

/// A stand-in `/sys` and `/dev` with one stick whose partition is an image.
fn machine() -> PathBuf {
    let root = std::env::temp_dir().join(format!("faraday-disk-leak-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("sys/class/block")).unwrap();
    fs::create_dir_all(root.join("dev")).unwrap();
    let d = root.join("sys/devices/usb/sdb");
    let p = d.join("sdb1");
    fs::create_dir_all(&p).unwrap();
    fs::write(d.join("diskseq"), "7\n").unwrap();
    symlink(&d, root.join("sys/class/block/sdb")).unwrap();
    fs::write(root.join("dev/sdb"), b"whole disk").unwrap();
    fs::write(p.join("partition"), "1\n").unwrap();
    fs::write(p.join("uevent"), "DEVNAME=sdb1\nPARTNAME=\n").unwrap();
    symlink(&p, root.join("sys/class/block/sdb1")).unwrap();
    let dev = root.join("dev/sdb1");
    fs::write(&dev, fat16()).unwrap();
    fs::set_permissions(&dev, fs::Permissions::from_mode(0o600)).unwrap();
    root
}

/// About 1.3 KiB of text, longer than two clusters.
fn seed_text() -> Vec<u8> {
    let words = [
        "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd",
        "abuse", "access", "accident",
    ];
    let mut t = String::new();
    let mut k = 0usize;
    while t.len() < 1300 {
        t.push_str(words[k % words.len()]);
        t.push(' ');
        k += 7;
    }
    t.into_bytes()
}

fn write(name: &str, bytes: &[u8], on_taken: OnTaken) -> Request {
    Request::Write {
        stick: Text::of("sdb1@sdb#7"),
        name: Text::of(name),
        copies: Vec::new(),
        replace: false,
        renamed: false,
        on_taken,
        total: bytes.len() as u64,
        bytes: bytes.to_vec(),
    }
}

#[test]
fn a_files_bytes_read_or_written_leave_no_freed_copy() {
    let c = compute();
    let root = machine();
    let mut d = Disks::new(&root.join("sys"), &root.join("dev"));
    let seed = seed_text();
    // What the test keeps for itself, never dropped.
    let seed = ManuallyDrop::new(seed);
    let done = d.handle(&c, write("seed.txt", &seed, OnTaken::Beside));
    assert!(matches!(done, Response::Written(_)), "{done:?}");

    // The whole text, and a piece of it that stays within one cluster.
    let forms = [
        Form::new("seed", seed.to_vec()),
        Form::new("seed piece", seed[100..260].to_vec()),
    ];
    let armed = faraday_leak_scan::arm(&forms);
    let mut answers = Vec::new();
    let stick = || Text::of("sdb1@sdb#7");
    answers.push(d.handle(
        &c,
        Request::Read {
            stick: stick(),
            name: Text::of("seed.txt"),
        },
    ));
    answers.push(d.handle(
        &c,
        Request::ReadPart {
            stick: stick(),
            name: Text::of("seed.txt"),
            from: 150,
            len: 300,
        },
    ));
    // A plain seed written: the file is read back from the medium.
    answers.push(d.handle(&c, write("other.txt", &seed, OnTaken::Beside)));
    // Over a same-named file that holds it.
    answers.push(d.handle(&c, write("seed.txt", &seed, OnTaken::Ask)));
    // In pieces, as a long file is sent.
    for (k, piece) in seed[..900].chunks(300).enumerate() {
        answers.push(d.handle(
            &c,
            Request::Piece {
                stick: stick(),
                name: Text::of("pieced.txt"),
                at: (k * 300) as u64,
                total: seed.len() as u64,
                bytes: piece.to_vec(),
            },
        ));
    }
    answers.push(d.handle(
        &c,
        Request::Write {
            stick: stick(),
            name: Text::of("pieced.txt"),
            copies: Vec::new(),
            replace: false,
            renamed: false,
            on_taken: OnTaken::Beside,
            total: seed.len() as u64,
            bytes: seed[900..].to_vec(),
        },
    ));
    let report = armed.finish();
    let answers = ManuallyDrop::new(answers);
    for a in answers.iter() {
        assert!(!matches!(a, Response::Failed(_)), "{a:?}");
    }
    let _ = fs::remove_dir_all(&root);
    assert!(
        report.freed.is_empty(),
        "a freed block held the file: {:?}",
        report
            .freed
            .iter()
            .map(|h| h.backtrace())
            .collect::<Vec<_>>()
    );
}
