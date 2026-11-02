//! The FAT32 formatter against dosfstools (`docs/ESTABLISHED.md` §3.2):
//! a volume it makes passes `fsck.fat -n` and has the boot sector,
//! FSInfo, FAT starts and root label `mkfs.fat -F 32` gives for the same
//! size, label and serial; files go in and out between this crate and
//! mtools. Sizes are sparse files. A machine without the tools skips.

use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_fat::{FormatError, Region, Volume, format32, label_bytes};

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

fn have_tools() -> bool {
    let ok = Command::new(tool("mkfs.fat"))
        .arg("--help")
        .output()
        .is_ok()
        && Command::new(tool("fsck.fat"))
            .arg("--help")
            .output()
            .is_ok()
        && Command::new(tool("mcopy")).arg("-V").output().is_ok();
    if !ok {
        eprintln!("dosfstools or mtools is not installed: the formatter checks are skipped");
    }
    ok
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("faraday-fmt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

const SERIAL: u32 = 0x1234_ABCD;

/// A sparse file of `sectors` sectors with the formatted volume's
/// non-zero regions written (a new file is zeros already).
fn formatted(dir: &Path, sectors: u64, hidden: u32, label: Option<[u8; 11]>) -> PathBuf {
    let img = dir.join("ours.img");
    let f = std::fs::File::create(&img).unwrap();
    f.set_len(sectors * 512).unwrap();
    for r in format32(sectors, hidden, SERIAL, label).unwrap().regions() {
        if let Region::Bytes { at, data } = r {
            f.write_all_at(&data, at).unwrap();
        }
    }
    img
}

/// The same size from `mkfs.fat -F 32`, its serial and label as given.
fn made_by_mkfs(dir: &Path, sectors: u64, hidden: u32, label: Option<&str>) -> (PathBuf, String) {
    let img = dir.join("theirs.img");
    let f = std::fs::File::create(&img).unwrap();
    f.set_len(sectors * 512).unwrap();
    let mut cmd = Command::new(tool("mkfs.fat"));
    cmd.args([
        "-F",
        "32",
        "-h",
        &hidden.to_string(),
        "-i",
        &format!("{SERIAL:08x}"),
    ]);
    if let Some(l) = label {
        cmd.args(["-n", l]);
    }
    let out = cmd.arg(&img).output().unwrap();
    assert!(out.status.success(), "mkfs.fat failed");
    (img, String::from_utf8_lossy(&out.stderr).into_owned())
}

fn bytes_at(img: &Path, at: u64, n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    std::fs::File::open(img)
        .unwrap()
        .read_exact_at(&mut b, at)
        .unwrap();
    b
}

fn fsck_clean(img: &Path) {
    let out = Command::new(tool("fsck.fat"))
        .args(["-n", "-v"])
        .arg(img)
        .output()
        .unwrap();
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "fsck.fat found faults:\n{text}");
}

/// What the two volumes must share: the boot sector but its code, the
/// backup boot sector, FSInfo and its copy but the hint, the first FAT
/// entries but FAT[1], and the root directory's label entry.
fn same_fields(ours: &Path, theirs: &Path, sectors: u64, label: bool) {
    let boot = bytes_at(ours, 0, 512);
    let their_boot = bytes_at(theirs, 0, 512);
    assert_eq!(
        boot[..90],
        their_boot[..90],
        "{sectors}: boot sector fields"
    );
    assert_eq!(boot[510..], their_boot[510..]);
    assert_eq!(bytes_at(ours, 6 * 512, 512), boot, "{sectors}: backup boot");
    let reserved = u64::from(u16::from_le_bytes([boot[14], boot[15]]));
    let fat = u64::from(u32::from_le_bytes(boot[36..40].try_into().unwrap()));
    for at in [512, 7 * 512] {
        let (mut ours_info, mut their_info) = (bytes_at(ours, at, 512), bytes_at(theirs, at, 512));
        assert_eq!(ours_info[492..496], [3, 0, 0, 0], "{sectors}: the hint");
        ours_info[492..496].fill(0);
        their_info[492..496].fill(0);
        assert_eq!(ours_info, their_info, "{sectors}: FSInfo at {at}");
    }
    for copy in 0..2 {
        let at = (reserved + copy * fat) * 512;
        let (ours_fat, their_fat) = (bytes_at(ours, at, 12), bytes_at(theirs, at, 12));
        assert_eq!(ours_fat[..4], their_fat[..4]);
        assert_eq!(ours_fat[8..], their_fat[8..]);
        assert_eq!(
            ours_fat[4..8],
            [0xFF; 4],
            "{sectors}: FAT[1] as Windows leaves it"
        );
    }
    let root = (reserved + 2 * fat) * 512;
    let (ours_root, their_root) = (bytes_at(ours, root, 12), bytes_at(theirs, root, 12));
    assert_eq!(ours_root, their_root, "{sectors}: root label");
    assert_eq!(ours_root[11] == 8, label);
}

#[test]
fn volumes_either_side_of_each_cluster_size_step_are_what_mkfs_makes() {
    if !have_tools() {
        return;
    }
    let label = label_bytes("faraday").unwrap();
    // 260 MiB, 8 GiB, 16 GiB and 32 GiB, in sectors, and one past each;
    // and the smallest sizes of FAT32.
    for sectors in [
        66_600, 532_480, 532_481, 16_777_216, 16_777_217, 33_554_432, 33_554_433, 67_108_864,
        67_108_865,
    ] {
        let dir = scratch(&format!("steps-{sectors}"));
        let ours = formatted(&dir.0, sectors, 133_120, label);
        let (theirs, _) = made_by_mkfs(&dir.0, sectors, 133_120, Some("FARADAY"));
        fsck_clean(&ours);
        same_fields(&ours, &theirs, sectors, true);
    }
}

#[test]
fn a_volume_without_a_label_has_none_in_the_boot_sector_or_the_root() {
    if !have_tools() {
        return;
    }
    let dir = scratch("nolabel");
    let sectors = 100_003;
    let ours = formatted(&dir.0, sectors, 0, None);
    let (theirs, _) = made_by_mkfs(&dir.0, sectors, 0, None);
    fsck_clean(&ours);
    same_fields(&ours, &theirs, sectors, false);
    assert_eq!(bytes_at(&ours, 71, 11), b"NO NAME    ");
}

#[test]
fn the_smallest_fat32_is_what_mkfs_makes_and_one_less_sector_is_refused() {
    if !have_tools() {
        return;
    }
    let smallest = (60_000..70_000u64)
        .find(|&s| format32(s, 133_120, SERIAL, None).is_ok())
        .unwrap();
    assert_eq!(
        format32(smallest - 1, 133_120, SERIAL, None),
        Err(FormatError::TooSmall)
    );
    assert_eq!(
        format32(0, 133_120, SERIAL, None),
        Err(FormatError::TooSmall)
    );
    assert!(
        format32(smallest, 133_120, SERIAL, None)
            .unwrap()
            .clusters()
            >= 65_525
    );
    let dir = scratch("smallest");
    let ours = formatted(&dir.0, smallest, 133_120, None);
    let (theirs, _) = made_by_mkfs(&dir.0, smallest, 133_120, None);
    fsck_clean(&ours);
    same_fields(&ours, &theirs, smallest, false);
    // mkfs.fat warns for the size below it.
    let (_, warning) = made_by_mkfs(&dir.0, smallest - 1, 133_120, None);
    assert!(warning.contains("less then suggested minimum"), "{warning}");
}

#[test]
fn files_written_here_and_by_mtools_read_back_in_the_other() {
    if !have_tools() {
        return;
    }
    let dir = scratch("files");
    let img = dir.0.join("vol.img");
    let sectors = 80_000;
    let mut disk = vec![0u8; sectors * 512];
    format32(
        sectors as u64,
        133_120,
        SERIAL,
        label_bytes("FILES").unwrap(),
    )
    .unwrap()
    .write(&mut disk)
    .unwrap();
    let mut v = Volume::open(disk).unwrap();
    assert_eq!(v.list().unwrap().len(), 0);
    let big: Vec<u8> = (0..300_000u32).map(|i| (i * 7 + 1) as u8).collect();
    v.create("wallet.psbt", &big).unwrap();
    std::fs::write(&img, v.into_disk()).unwrap();
    fsck_clean(&img);

    let i = img.to_str().unwrap();
    let out = dir.0.join("out");
    let run = |args: &[&str]| {
        Command::new(tool("mcopy"))
            .env("MTOOLS_SKIP_CHECK", "1")
            .args(args)
            .status()
            .unwrap()
            .success()
    };
    assert!(run(&[
        "-n",
        "-i",
        i,
        "::wallet.psbt",
        out.to_str().unwrap()
    ]));
    assert_eq!(std::fs::read(&out).unwrap(), big);

    let theirs = dir.0.join("theirs");
    std::fs::write(&theirs, b"put there by mtools").unwrap();
    assert!(run(&[
        "-i",
        i,
        theirs.to_str().unwrap(),
        "::from-mtools.txt"
    ]));
    fsck_clean(&img);
    let mut v = Volume::open(std::fs::read(&img).unwrap()).unwrap();
    assert_eq!(
        v.read("from-mtools.txt", 100).unwrap(),
        b"put there by mtools"
    );
    assert_eq!(v.read("wallet.psbt", 400_000).unwrap(), big);
    assert_eq!(v.label().as_deref(), Some("FILES"));
    assert_eq!(v.serial(), Some(SERIAL));
}
