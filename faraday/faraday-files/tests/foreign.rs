//! A FAT data partition as Windows and a Mac leave it (`docs/STARTER.md`
//! §4.4, D11): their own files and folders beside a PSBT, some of them
//! marked hidden and system as Windows marks them, some not. The PSBT is
//! the one file listed and read; a file Faraday writes leaves the PSBT
//! readable and their files where they were, and the volume passes
//! `fsck.fat -n`.
//!
//! The volume is made by `mkfs.fat` and filled by mtools, found as
//! `faraday-fat`'s own FAT tests find them (`FARADAY_FATTOOLS`,
//! `/usr/sbin`, `/sbin`, PATH); a machine without them says so and skips.

use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_fat::Volume;
use faraday_files::{Fat, listing, read, write_any};

const PSBT: &str = "savings-unsigned.psbt";

/// Folders Windows and macOS make at a volume's top level, and the
/// attributes Windows gives its own.
const DIRS: [(&str, &str); 5] = [
    ("System Volume Information", "+h +s"),
    ("$RECYCLE.BIN", "+h +s"),
    (".Spotlight-V100", ""),
    (".fseventsd", ""),
    (".Trashes", ""),
];

/// Files they write, with the same attributes.
const FILES: [(&str, &str); 7] = [
    ("System Volume Information/IndexerVolumeGuid", "+h +s"),
    ("System Volume Information/WPSettings.dat", "+h +s"),
    ("$RECYCLE.BIN/desktop.ini", "+h +s"),
    ("desktop.ini", "+h +s"),
    ("Thumbs.db", ""),
    (".DS_Store", ""),
    ("._savings-unsigned.psbt", ""),
];

fn tool(name: &str) -> PathBuf {
    if let Some(dir) = std::env::var_os("FARADAY_FATTOOLS") {
        return Path::new(&dir).join(name);
    }
    ["/usr/sbin", "/sbin"]
        .iter()
        .map(|d| Path::new(d).join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

fn run(cmd: &str, args: &[&str]) -> (bool, Vec<u8>) {
    let out = Command::new(tool(cmd))
        .env("MTOOLS_SKIP_CHECK", "1")
        .args(args)
        .output()
        .unwrap();
    (out.status.success(), out.stdout)
}

fn have_tools() -> bool {
    let ok = Command::new(tool("mkfs.fat"))
        .arg("--help")
        .output()
        .is_ok()
        && Command::new(tool("mcopy")).arg("-V").output().is_ok();
    if !ok {
        eprintln!("mkfs.fat or mtools is not installed: the FAT check is skipped");
    }
    ok
}

/// The bytes an operating system's file is given here.
fn own(name: &str) -> Vec<u8> {
    format!("{name}: an operating system's own bytes").into_bytes()
}

fn psbt() -> Vec<u8> {
    let mut b = b"psbt\xff".to_vec();
    b.extend((0..900u32).map(|i| (i * 7) as u8));
    b
}

#[test]
fn a_partition_windows_and_a_mac_wrote_to_lists_and_keeps_the_psbt_alone() {
    if !have_tools() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("faraday-files-foreign-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let img = dir.join("faradaydata.img");
    let i = img.to_str().unwrap();
    let (ok, _) = run(
        "mkfs.fat",
        &["-C", "-F", "16", "-n", "FARADAYDATA", i, "32768"],
    );
    assert!(ok, "mkfs.fat");
    let src = dir.join("src");
    for (d, attrs) in DIRS {
        let at = format!("::{d}");
        assert!(run("mmd", &["-i", i, &at]).0, "mmd {d}");
        for a in attrs.split_whitespace() {
            assert!(run("mattrib", &["-i", i, a, &at]).0, "mattrib {d}");
        }
    }
    for (f, attrs) in FILES {
        std::fs::write(&src, own(f)).unwrap();
        let at = format!("::{f}");
        assert!(
            run("mcopy", &["-i", i, src.to_str().unwrap(), &at]).0,
            "mcopy {f}"
        );
        for a in attrs.split_whitespace() {
            assert!(run("mattrib", &["-i", i, a, &at]).0, "mattrib {f}");
        }
    }
    std::fs::write(&src, psbt()).unwrap();
    assert!(
        run(
            "mcopy",
            &["-i", i, src.to_str().unwrap(), &format!("::{PSBT}")]
        )
        .0
    );

    let mut place = Fat(Volume::open(std::fs::read(&img).unwrap()).unwrap());
    assert_eq!(
        listing(&mut place),
        [(PSBT.to_string(), psbt().len() as u64)]
    );
    assert_eq!(read(&mut place, PSBT).unwrap(), psbt());
    for name in [
        "desktop.ini",
        "DESKTOP.INI",
        "Thumbs.db",
        "._savings-unsigned.psbt",
    ] {
        assert!(read(&mut place, name).is_err(), "{name} was read");
    }

    let signed = b"psbt\xffsigned".to_vec();
    let wrote = write_any(&mut place, "savings-signed.psbt", &signed, false).unwrap();
    assert_eq!(wrote, "savings-signed.psbt");
    assert_eq!(read(&mut place, PSBT).unwrap(), psbt());
    assert_eq!(
        listing(&mut place),
        [
            (wrote.clone(), signed.len() as u64),
            (PSBT.to_string(), psbt().len() as u64)
        ]
    );
    std::fs::write(&img, place.0.into_disk()).unwrap();

    // As another implementation reads it: their files as they were, the
    // PSBT and the new file whole, and nothing for a check to fix.
    for (f, _) in FILES {
        let (ok, bytes) = run("mtype", &["-i", i, &format!("::{f}")]);
        assert!(ok, "{f} is gone");
        assert_eq!(bytes, own(f), "{f}");
    }
    assert_eq!(run("mtype", &["-i", i, &format!("::{PSBT}")]).1, psbt());
    assert_eq!(run("mtype", &["-i", i, &format!("::{wrote}")]).1, signed);
    let out = Command::new(tool("fsck.fat"))
        .args(["-n", "-v"])
        .arg(&img)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "fsck.fat found faults:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
