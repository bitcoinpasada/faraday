//! The PC stick image as built (`docs/STICK-IDENTITY.md` §4.1, §10): after
//! `faraday/image/stick-finish.py`, util-linux's `sfdisk` and gdisk's
//! `sgdisk` find no problem, both GPT headers give 34 as the first usable
//! sector with the partitions where genimage put them, and dosfstools'
//! `fsck.fat` finds the boot partition clean with FAT[1] set in both FATs
//! and FSInfo's hint at the first free cluster, for two boot files of
//! different sizes. The image is made by those tools (an ESP at 2048 and
//! a data partition at 133120, as genimage lays them out); where one is
//! not installed the check is skipped.

use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use faraday_crypto::crc32;

use crate::ids::{runs, tool};
use crate::sticks::{BASIC, BOOT_FIRST, DATA_FIRST, ESP, IDS, IMAGE, SECTOR};
use crate::writer::tmp;

fn installed() -> bool {
    let ok = runs("sfdisk", &["--version"])
        && runs("sgdisk", &["--version"])
        && runs("mkfs.fat", &["--help"])
        && runs("fsck.fat", &["--help"])
        && runs("mcopy", &["--version"]);
    if !ok {
        eprintln!("sfdisk, sgdisk, dosfstools or mtools is not installed: the check is skipped");
        if std::env::var_os("FARADAY_JUDGES_REQUIRED").is_some() {
            panic!(
                "FARADAY_JUDGES_REQUIRED is set and sfdisk, sgdisk, dosfstools or mtools is not installed"
            );
        }
    }
    ok
}

/// An image as genimage makes it, by other tools: the GPT with its first
/// usable sector at 2048, a FAT32 boot partition holding one file of
/// `file` bytes, and the data partition's volume. Returns the image and
/// the boot partition as genimage leaves it beside the image.
fn genimage_shaped(dir: &Path, file: usize) -> (PathBuf, PathBuf) {
    std::fs::create_dir_all(dir).expect("a folder");
    let img = dir.join("stick.img");
    std::fs::File::create(&img)
        .and_then(|f| f.set_len(IMAGE))
        .expect("an image");
    let script = format!(
        "label: gpt\nlabel-id: {}\nfirst-lba: 2048\n\
         start=2048, size=131072, type={ESP}, uuid={}, name=\"EFI system partition\"\n\
         start=133120, size=262144, type={BASIC}, uuid={}, name=\"Basic data partition\"\n",
        IDS[0], IDS[1], IDS[2]
    );
    let mut sfdisk = Command::new(tool("sfdisk"))
        .args(["--no-reread", "--no-tell-kernel", "-q"])
        .arg(&img)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("sfdisk");
    use std::io::Write;
    sfdisk
        .stdin
        .take()
        .expect("its input")
        .write_all(script.as_bytes())
        .expect("written");
    assert!(sfdisk.wait().expect("sfdisk ran").success());

    let boot = dir.join("esp.vfat");
    let made = Command::new(tool("mkfs.fat"))
        .args(["-F", "32", "-n", "EFI", "-i", "6E1F2A01", "-C"])
        .arg(&boot)
        .arg("65536")
        .stdout(Stdio::null())
        .status()
        .expect("mkfs.fat");
    assert!(made.success());
    let payload = dir.join("BOOTX64.EFI");
    std::fs::write(&payload, vec![0x4D; file]).expect("the boot file");
    for folder in ["::EFI", "::EFI/BOOT"] {
        let mmd = Command::new(tool("mmd"))
            .arg("-i")
            .arg(&boot)
            .arg(folder)
            .status()
            .expect("mmd");
        assert!(mmd.success());
    }
    let copied = Command::new(tool("mcopy"))
        .arg("-i")
        .arg(&boot)
        .arg(&payload)
        .arg("::EFI/BOOT/BOOTX64.EFI")
        .status()
        .expect("mcopy");
    assert!(copied.success());

    let data = dir.join("data.vfat");
    let made = Command::new(tool("mkfs.fat"))
        .args(["-F", "32", "-s", "2", "-i", "6E1F2A02", "-C"])
        .arg(&data)
        .arg("131072")
        .stdout(Stdio::null())
        .status()
        .expect("mkfs.fat");
    assert!(made.success());
    let out = std::fs::OpenOptions::new()
        .write(true)
        .open(&img)
        .expect("the image");
    out.write_all_at(&std::fs::read(&boot).expect("boot"), BOOT_FIRST * SECTOR)
        .expect("boot in");
    out.write_all_at(&std::fs::read(&data).expect("data"), DATA_FIRST * SECTOR)
        .expect("data in");
    (img, boot)
}

fn finish(img: &Path, boot: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../image/stick-finish.py");
    let out = Command::new("python3")
        .arg(script)
        .arg(img)
        .arg(boot)
        .output()
        .expect("python3");
    assert!(
        out.status.success(),
        "stick-finish: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn read(path: &Path, at: u64, len: usize) -> Vec<u8> {
    let mut b = vec![0; len];
    std::fs::File::open(path)
        .and_then(|f| f.read_exact_at(&mut b, at))
        .expect("read");
    b
}

fn le(b: &[u8], at: usize, n: usize) -> u64 {
    b[at..at + n]
        .iter()
        .rev()
        .fold(0, |a, &x| (a << 8) | u64::from(x))
}

fn output(name: &str, args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(tool(name))
        .args(args)
        .arg(path)
        .output()
        .expect(name);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

#[test]
fn the_pc_stick_image_is_left_as_windows_leaves_it_and_other_tools_find_no_problem() {
    if !installed() {
        return;
    }
    let mut hints = Vec::new();
    for (n, file) in [64 << 10, (3 << 20) + 5].into_iter().enumerate() {
        let dir = tmp(&format!("image-finish-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        let (img, boot) = genimage_shaped(&dir, file);
        let before = read(&img, SECTOR, 512);
        assert_eq!(le(&before, 40, 8), 2048, "the tools made genimage's table");
        finish(&img, &boot);

        let (ok, text) = output("sgdisk", &["-v"], &img);
        assert!(ok && text.contains("No problems found"), "{text}");
        let (ok, text) = output("sfdisk", &["--verify"], &img);
        assert!(ok, "{text}");
        let (_, dump) = output("sfdisk", &["--dump"], &img);
        assert!(dump.contains("first-lba: 34"), "{dump}");
        assert!(dump.contains("start=        2048"), "{dump}");
        assert!(dump.contains("start=      133120"), "{dump}");
        let last = IMAGE / SECTOR - 1;
        for lba in [1, last] {
            let h = read(&img, lba * SECTOR, 512);
            assert_eq!(le(&h, 40, 8), 34, "first usable sector at {lba}");
        }

        let (ok, text) = output("fsck.fat", &["-n"], &boot);
        assert!(ok, "{text}");
        let vol = std::fs::read(&boot).expect("boot");
        assert_eq!(
            read(&img, BOOT_FIRST * SECTOR, vol.len()),
            vol,
            "the boot file is the image's partition"
        );
        let spc = vol[13] as usize;
        let (reserved, fat) = (le(&vol, 14, 2) as usize, le(&vol, 36, 4) as usize);
        for i in 0..2 {
            let at = (reserved + i * fat) * 512 + 4;
            assert_eq!(&vol[at..at + 4], &[0xFF; 4], "FAT[1] of FAT {}", i + 1);
        }
        // Clusters 2 to 4 hold the root and the two folders; the boot
        // file follows them.
        let hint = 5 + file.div_ceil(spc * 512) as u64;
        let info = le(&vol, 48, 2) as usize * 512;
        assert_eq!(le(&vol, info + 492, 4), hint, "FSInfo's next free cluster");
        hints.push(hint);
    }
    assert_ne!(hints[0], hints[1]);
}

/// What Windows did to a stick bigger than the image (`docs/STICK-
/// IDENTITY.md` §2): the primary header's backup place, last usable
/// sector and entries' place moved and its CRC again, and a new backup at
/// the disk's end.
fn windows_rewrite(disk: &Path, sectors: u64) {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(disk)
        .expect("scratch disk");
    let at = |lba: u64| lba * SECTOR;
    let mut p = vec![0; 512];
    f.read_exact_at(&mut p, at(1)).expect("header");
    let mut entries = vec![0; 32 * 512];
    f.read_exact_at(&mut entries, at(2)).expect("entries");
    let last = sectors - 1;
    let first = le(&p, 40, 8);
    let put = |h: &mut [u8], off: usize, v: u64| h[off..off + 8].copy_from_slice(&v.to_le_bytes());
    let crc = |h: &mut [u8]| {
        h[16..20].fill(0);
        let c = crc32(&h[..92]);
        h[16..20].copy_from_slice(&c.to_le_bytes());
    };
    put(&mut p, 32, last);
    put(&mut p, 48, last - 33);
    put(&mut p, 72, first - 32);
    crc(&mut p);
    let mut b = p.clone();
    put(&mut b, 24, last);
    put(&mut b, 32, 1);
    put(&mut b, 72, last - 32);
    crc(&mut b);
    f.write_all_at(&p, at(1)).expect("primary");
    f.write_all_at(&entries, at(last - 32))
        .expect("backup entries");
    f.write_all_at(&b, at(last)).expect("backup header");
}

#[test]
fn windows_rewriting_the_finished_image_on_a_larger_disk_leaves_a_table_sfdisk_reads() {
    if !installed() {
        return;
    }
    let dir = tmp("image-finish-windows");
    let _ = std::fs::remove_dir_all(&dir);
    let (img, boot) = genimage_shaped(&dir, 64 << 10);
    finish(&img, &boot);
    let disk = dir.join("scratch.img");
    std::fs::copy(&img, &disk).expect("a copy");
    std::fs::OpenOptions::new()
        .write(true)
        .open(&disk)
        .and_then(|f| f.set_len(1 << 30))
        .expect("1 GiB");
    windows_rewrite(&disk, (1 << 30) / SECTOR);

    let primary = read(&disk, SECTOR, 512);
    assert_eq!(le(&primary, 72, 8), 2, "the entries are where they are");
    let entries = read(&disk, 2 * SECTOR, 32 * 512);
    assert_eq!(le(&primary, 88, 4), u64::from(crc32(&entries)));
    let (ok, dump) = output("sfdisk", &["--dump"], &disk);
    assert!(ok && !dump.contains("corrupt"), "{dump}");
    assert!(dump.contains("start=        2048"), "{dump}");
    assert!(dump.contains("start=      133120"), "{dump}");
    let (ok, text) = output("sfdisk", &["--verify"], &disk);
    assert!(ok, "{text}");
}
