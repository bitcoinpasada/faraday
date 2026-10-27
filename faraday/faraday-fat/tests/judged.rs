//! FAT as other implementations see it (`docs/history/PLAN.md` §12.5): volumes made by
//! `mkfs.fat`, files put there by mtools and read here, files written
//! here and read by mtools, and every volume written here passed by
//! `fsck.fat -n`.
//!
//! The tools are found in `FARADAY_FATTOOLS` (a directory holding
//! `mkfs.fat`, `fsck.fat`, `mcopy`, `mmd` and `mdir`), in `/usr/sbin` or
//! `/sbin`, or on PATH; a machine without them says so and skips.

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_fat::{
    Disk, Error, FAT_PIECE, Kind, MAX_ROOT_BYTES, Volume, boot_sector_label, boot_sector_serial,
};

fn tool(name: &str) -> PathBuf {
    if let Some(dir) = std::env::var_os("FARADAY_FATTOOLS") {
        return Path::new(&dir).join(name);
    }
    // Debian puts mkfs.fat and fsck.fat in /usr/sbin, which a shell that
    // is not a login one may not have on PATH.
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
        && Command::new(tool("mcopy")).arg("-V").output().is_ok();
    if !ok {
        eprintln!("mkfs.fat or mtools is not installed: the FAT checks are skipped");
    }
    ok
}

/// A test's own scratch tree; removed when it drops.
struct Scratch(PathBuf);

impl Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("faraday-fat-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

/// A fresh volume from `mkfs.fat` with these arguments, `kib` long.
fn mkfs(dir: &Path, args: &[&str], kib: u32) -> PathBuf {
    let img = dir.join("vol.img");
    let _ = std::fs::remove_file(&img);
    let out = Command::new(tool("mkfs.fat"))
        .arg("-C")
        .args(args)
        .arg(&img)
        .arg(kib.to_string())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    img
}

fn mtools(cmd: &str, args: &[&str]) -> (bool, Vec<u8>) {
    let out = Command::new(tool(cmd))
        .env("MTOOLS_SKIP_CHECK", "1")
        .args(args)
        .output()
        .unwrap();
    (out.status.success(), out.stdout)
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

fn open(img: &Path) -> Volume<Vec<u8>> {
    Volume::open(std::fs::read(img).unwrap()).unwrap()
}

fn save(v: Volume<Vec<u8>>, img: &Path) {
    std::fs::write(img, v.into_disk()).unwrap();
}

/// Bytes that differ along their length, so a cluster out of place shows.
fn pattern(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| (i as u32 * 31 + u32::from(seed)) as u8)
        .collect()
}

fn volumes() -> Vec<(&'static str, Vec<&'static str>, u32, Kind)> {
    vec![
        (
            "fat16",
            vec!["-F", "16", "-n", "FARADAYDATA"],
            32 * 1024,
            Kind::Fat16,
        ),
        (
            "fat32",
            vec!["-F", "32", "-s", "1", "-n", "FARADAY"],
            40 * 1024,
            Kind::Fat32,
        ),
    ]
}

#[test]
fn files_mtools_wrote_read_back_the_same() {
    if !have_tools() {
        return;
    }
    for (name, args, kib, kind) in volumes() {
        let dir = scratch(&format!("read-{name}"));
        let img = mkfs(&dir, &args, kib);
        let i = img.to_str().unwrap();
        let files = [
            ("savings-unsigned.psbt", pattern(1_234, 1)),
            ("README.TXT", pattern(40, 2)),
            (
                "A long name with spaces and ünïcode.txt",
                pattern(70_000, 3),
            ),
            ("empty.txt", Vec::new()),
        ];
        for (n, bytes) in &files {
            let src = dir.join("src");
            std::fs::write(&src, bytes).unwrap();
            let (ok, _) = mtools(
                "mcopy",
                &["-i", i, src.to_str().unwrap(), &format!("::{n}")],
            );
            assert!(ok, "mcopy {n}");
        }
        let (ok, _) = mtools("mmd", &["-i", i, "::Photos"]);
        assert!(ok);

        let mut v = open(&img);
        assert_eq!(v.kind(), kind, "{name}");
        assert_eq!(v.label().as_deref(), Some(args[args.len() - 1]), "{name}");
        let list = v.list().unwrap();
        for (n, bytes) in &files {
            let e = list
                .iter()
                .find(|e| e.name == *n)
                .unwrap_or_else(|| panic!("{name}: {n} not listed: {list:?}"));
            assert_eq!(e.size as usize, bytes.len());
            assert!(!e.dir);
            assert_eq!(&v.read(n, 1 << 20).unwrap(), bytes, "{name}: {n}");
        }
        assert!(list.iter().any(|e| e.name == "Photos" && e.dir));
        assert_eq!(v.read("savings-unsigned.psbt", 100), Err(Error::TooLarge));
        assert_eq!(v.read("Photos", 100), Err(Error::NotFound));
    }
}

#[test]
fn files_written_here_pass_fsck_and_read_back_in_mtools() {
    if !have_tools() {
        return;
    }
    for (name, args, kib, _) in volumes() {
        let dir = scratch(&format!("write-{name}"));
        let img = mkfs(&dir, &args, kib);
        let i = img.to_str().unwrap();
        // One file there already, from mtools.
        let before = dir.join("before");
        std::fs::write(&before, pattern(5_000, 9)).unwrap();
        assert!(
            mtools(
                "mcopy",
                &["-i", i, before.to_str().unwrap(), "::wallet.txt"]
            )
            .0
        );

        let mut v = open(&img);
        let files = [
            ("vault.ofv", pattern(262_144 * 4 + 64, 4)),
            ("SIGNED.PSB", pattern(900, 5)),
            ("savings-unsigned-signed.psbt", pattern(3_000, 6)),
            ("café.txt", pattern(10, 7)),
            ("nothing.txt", Vec::new()),
        ];
        for (n, bytes) in &files {
            v.create(n, bytes)
                .unwrap_or_else(|e| panic!("{name}: {n}: {e:?}"));
        }
        assert_eq!(v.create("VAULT.OFV", b"x"), Err(Error::Exists));
        // The write-back pattern: a new file, renamed over the old one.
        v.create("wallet.txt.new", &pattern(6_000, 8)).unwrap();
        v.delete("wallet.txt").unwrap();
        v.rename("wallet.txt.new", "wallet.txt").unwrap();
        v.delete("nothing.txt").unwrap();
        assert_eq!(v.delete("nothing.txt"), Err(Error::NotFound));
        save(v, &img);

        fsck_clean(&img);
        let out = dir.join("out");
        for (n, bytes) in files.iter().filter(|(n, _)| *n != "nothing.txt") {
            let _ = std::fs::remove_file(&out);
            let (ok, _) = mtools(
                "mcopy",
                &["-n", "-i", i, &format!("::{n}"), out.to_str().unwrap()],
            );
            assert!(ok, "{name}: mtools could not read {n}");
            assert_eq!(&std::fs::read(&out).unwrap(), bytes, "{name}: {n}");
        }
        let _ = std::fs::remove_file(&out);
        assert!(
            mtools(
                "mcopy",
                &["-n", "-i", i, "::wallet.txt", out.to_str().unwrap()]
            )
            .0
        );
        assert_eq!(std::fs::read(&out).unwrap(), pattern(6_000, 8));
        let (_, listing) = mtools("mdir", &["-i", i, "-b", "::"]);
        let listing = String::from_utf8_lossy(&listing);
        assert!(!listing.contains("nothing.txt"), "{listing}");
        assert!(!listing.contains("wallet.txt.new"), "{listing}");
    }
}

#[test]
fn a_fat32_root_grows_past_its_first_cluster() {
    if !have_tools() {
        return;
    }
    let dir = scratch("grow");
    let img = mkfs(&dir, &["-F", "32", "-s", "1"], 40 * 1024);
    let mut v = open(&img);
    // 512-byte clusters hold 16 entries; each of these names takes 3.
    for k in 0..120 {
        v.create(
            &format!("a file with a long name number {k}.txt"),
            &[k as u8],
        )
        .unwrap();
    }
    assert_eq!(v.list().unwrap().len(), 120);
    save(v, &img);
    fsck_clean(&img);
    let (_, listing) = mtools("mdir", &["-i", img.to_str().unwrap(), "-b", "::"]);
    assert_eq!(String::from_utf8_lossy(&listing).lines().count(), 120);
}

#[test]
fn a_full_fat16_root_is_refused_and_left_whole() {
    if !have_tools() {
        return;
    }
    let dir = scratch("full");
    let img = mkfs(&dir, &["-F", "16", "-r", "16"], 32 * 1024);
    let mut v = open(&img);
    let mut made = 0;
    let err = loop {
        match v.create(&format!("name-{made}.psbt"), &pattern(2_000, 1)) {
            Ok(()) => made += 1,
            Err(e) => break e,
        }
    };
    assert_eq!(err, Error::DirFull);
    assert!(made > 0);
    save(v, &img);
    fsck_clean(&img);
}

#[test]
fn what_is_not_fat16_or_fat32_is_refused() {
    assert!(matches!(
        Volume::open(vec![0x5a_u8; 1 << 20]),
        Err(Error::NotFat(_))
    ));
    if !have_tools() {
        return;
    }
    let dir = scratch("refuse");
    let img = mkfs(&dir, &["-F", "12"], 2 * 1024);
    assert!(matches!(
        Volume::open(std::fs::read(&img).unwrap()),
        Err(Error::Fat12)
    ));
    // A volume that claims more sectors than its partition has.
    let img = mkfs(&dir, &["-F", "16"], 32 * 1024);
    let mut bytes = std::fs::read(&img).unwrap();
    bytes.truncate(bytes.len() / 2);
    assert!(matches!(Volume::open(bytes), Err(Error::NotFat(_))));
}

#[test]
fn a_chain_that_loops_is_refused() {
    if !have_tools() {
        return;
    }
    let dir = scratch("loop");
    let img = mkfs(&dir, &["-F", "16"], 32 * 1024);
    let mut v = open(&img);
    v.create("looped.bin", &pattern(20_000, 3)).unwrap();
    let mut bytes = v.into_disk();
    // FAT16's root is outside the data area, so the file starts at
    // cluster 2. Point its second cluster back at its first.
    let reserved = u16::from_le_bytes([bytes[14], bytes[15]]) as usize;
    let fat = reserved * 512;
    bytes[fat + 3 * 2..fat + 3 * 2 + 2].copy_from_slice(&2u16.to_le_bytes());
    let mut v = Volume::open(bytes).unwrap();
    assert!(matches!(
        v.read("looped.bin", 1 << 20),
        Err(Error::Corrupt(_))
    ));
}

#[test]
fn the_boot_partitions_label_is_read_from_its_first_sector() {
    if !have_tools() {
        return;
    }
    let dir = scratch("label");
    for (args, kib) in [
        (vec!["-F", "32", "-n", "FARADAYBOOT"], 48 * 1024),
        (vec!["-F", "16", "-n", "FARADAYBOOT"], 32 * 1024),
    ] {
        let img = mkfs(&dir, &args, kib);
        let bytes = std::fs::read(&img).unwrap();
        assert_eq!(boot_sector_label(&bytes[..512]), Some(*b"FARADAYBOOT"));
    }
}

/// The volume serial number that tells two sticks apart
/// (`docs/VAULT.md` §6) is the one `mkfs.fat -i` wrote, on FAT16
/// and FAT32, and a volume copied sector for sector has the same.
#[test]
fn the_volume_serial_is_read_from_the_boot_sector() {
    if !have_tools() {
        return;
    }
    let dir = scratch("serial");
    for (fat, kib) in [("32", 48 * 1024), ("16", 32 * 1024)] {
        let img = mkfs(&dir, &["-F", fat, "-i", "1a2b3c4d", "-n", "STICK"], kib);
        let bytes = std::fs::read(&img).unwrap();
        assert_eq!(
            boot_sector_serial(&bytes[..512]),
            Some(0x1a2b_3c4d),
            "FAT{fat}"
        );
        assert_eq!(Volume::open(bytes).unwrap().serial(), Some(0x1a2b_3c4d));
        let other = mkfs(&dir, &["-F", fat, "-i", "5e6f7a8b", "-n", "STICK"], kib);
        let bytes = std::fs::read(&other).unwrap();
        assert_eq!(
            boot_sector_serial(&bytes[..512]),
            Some(0x5e6f_7a8b),
            "FAT{fat}"
        );
    }
}

/// A stick's bytes are its maker's to choose: a volume with bytes changed
/// in its boot sector, FATs or root directory is refused or read, never a
/// panic, and nothing written to it reaches past the partition.
#[test]
fn damaged_volumes_are_refused_without_a_panic() {
    if !have_tools() {
        return;
    }
    let dir = scratch("damage");
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    // A small FAT16 takes most of the damage; FAT32 cannot be smaller
    // than 33 MB, so its copies are fewer.
    let cases: [(&str, Vec<&str>, u32, usize); 2] = [
        ("fat16", vec!["-F", "16", "-s", "1"], 4 * 1024, 2_000),
        ("fat32", vec!["-F", "32", "-s", "1"], 40 * 1024, 150),
    ];
    for (name, args, kib, rounds) in cases {
        let img = mkfs(&dir, &args, kib);
        let mut v = open(&img);
        for k in 0..6 {
            v.create(
                &format!("file number {k}.psbt"),
                &pattern(3_000 * k, k as u8),
            )
            .unwrap();
        }
        let base = v.into_disk();
        // Where the structures are: the first 256 KiB holds the boot
        // sector, both FATs and the root directory on both volumes.
        let hot = base.len().min(256 * 1024);
        for _ in 0..rounds {
            let mut bytes = base.clone();
            for _ in 0..(1 + next() % 8) {
                let at = (next() % hot as u64) as usize;
                bytes[at] = next() as u8;
            }
            let Ok(mut v) = Volume::open(bytes) else {
                continue;
            };
            let _ = v.label();
            if let Ok(list) = v.list() {
                for e in list.iter().take(64) {
                    let _ = v.read(&e.name, 1 << 20);
                }
            }
            if v.create("new.txt", b"faraday").is_ok() {
                let _ = v.rename("new.txt", "renamed.txt");
                let _ = v.delete("renamed.txt");
            }
            assert_eq!(v.into_disk().len(), base.len(), "{name}");
        }
    }
}

/// A FAT32 volume smaller than the specification's 65525 clusters, as
/// mtools makes one on a small stick: read and written like any other.
#[test]
fn a_small_fat32_from_mtools_reads_and_writes() {
    if !have_tools() || Command::new(tool("mformat")).arg("-V").output().is_err() {
        return;
    }
    let dir = scratch("small32");
    let img = dir.join("small.img");
    std::fs::write(&img, vec![0u8; 63_488 * 512]).unwrap();
    let ok = Command::new(tool("mformat"))
        .env("MTOOLS_SKIP_CHECK", "1")
        .args(["-i", img.to_str().unwrap(), "-F", "-v", "TESTSTICK", "::"])
        .status()
        .unwrap();
    assert!(ok.success());
    let src = dir.join("src");
    std::fs::write(&src, pattern(3_000, 1)).unwrap();
    assert!(
        mtools(
            "mcopy",
            &[
                "-i",
                img.to_str().unwrap(),
                src.to_str().unwrap(),
                "::wallet.txt"
            ]
        )
        .0
    );
    let mut v = open(&img);
    assert_eq!(v.kind(), Kind::Fat32);
    assert_eq!(v.read("wallet.txt", 1 << 20).unwrap(), pattern(3_000, 1));
    v.create("signed.psbt", &pattern(900, 2)).unwrap();
    save(v, &img);
    fsck_clean(&img);
}

/// Whether `needle` is anywhere in `hay`.
fn holds(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// A file deleted with zeros: neither its bytes nor its name are left
/// anywhere in the volume, the files beside it read back the same, and
/// the volume passes fsck.
#[test]
fn a_wiped_file_leaves_neither_its_bytes_nor_its_name() {
    if !have_tools() {
        return;
    }
    for (name, args, kib, _) in volumes() {
        let dir = scratch(&format!("wipe-{name}"));
        let img = mkfs(&dir, &args, kib);
        let secret: Vec<u8> = b"abandon ability able about above absent "
            .iter()
            .copied()
            .cycle()
            .take(9_000)
            .collect();
        let mut v = open(&img);
        v.create("seed-words.txt", &secret).unwrap();
        v.create("wallet.txt", &pattern(5_000, 3)).unwrap();
        save(v, &img);
        let before = std::fs::read(&img).unwrap();
        assert!(holds(&before, b"abandon ability"), "{name}: written");

        let mut v = open(&img);
        v.wipe("seed-words.txt").unwrap();
        assert_eq!(v.wipe("seed-words.txt"), Err(Error::NotFound));
        assert_eq!(v.read("wallet.txt", 1 << 20).unwrap(), pattern(5_000, 3));
        save(v, &img);

        let after = std::fs::read(&img).unwrap();
        assert!(
            !holds(&after, b"abandon ability"),
            "{name}: its bytes are left"
        );
        let long: Vec<u8> = "seed-words"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(!holds(&after, &long), "{name}: its long name is left");
        assert!(!holds(&after, b"SEED-W"), "{name}: its short name is left");
        fsck_clean(&img);
        let (_, listing) = mtools("mdir", &["-i", img.to_str().unwrap(), "-b", "::"]);
        let listing = String::from_utf8_lossy(&listing);
        assert!(!listing.contains("seed-words"), "{name}: {listing}");
        assert!(listing.contains("wallet.txt"), "{name}: {listing}");
    }
}

/// Bytes that mark one file, repeated to `n`.
fn marked(mark: &[u8], n: usize) -> Vec<u8> {
    mark.iter().copied().cycle().take(n).collect()
}

/// A file put on the volume by mtools.
fn mput(dir: &Path, img: &Path, name: &str, bytes: &[u8]) {
    let src = dir.join("src");
    std::fs::write(&src, bytes).unwrap();
    let ok = mtools(
        "mcopy",
        &[
            "-o",
            "-i",
            img.to_str().unwrap(),
            src.to_str().unwrap(),
            &format!("::{name}"),
        ],
    )
    .0;
    assert!(ok, "mcopy {name}");
}

/// Zeroing the free space in small steps: what mtools deleted, its name
/// and its bytes, including those left in the tail of a later file's
/// last cluster, is gone; the files kept read back the same, and the
/// volume passes fsck.
#[test]
fn the_free_space_zeroed_keeps_the_files_and_nothing_deleted() {
    if !have_tools() {
        return;
    }
    for (name, args, kib, _) in volumes() {
        let dir = scratch(&format!("zero-free-{name}"));
        let img = mkfs(&dir, &args, kib);
        let i = img.to_str().unwrap();
        mput(&dir, &img, "keep-one.txt", &pattern(7_000, 1));
        mput(
            &dir,
            &img,
            "gone-words.txt",
            &marked(b"gone words gone words ", 20_000),
        );
        mput(&dir, &img, "keep-two.txt", &pattern(3_000, 2));
        assert!(mtools("mdel", &["-i", i, "::gone-words.txt"]).0);
        // A small file over the deleted one's first cluster: what is past
        // its end there is the deleted file's.
        mput(&dir, &img, "small.txt", b"small");
        let before = std::fs::read(&img).unwrap();
        assert!(
            holds(&before, b"gone words"),
            "{name}: deleted, still there"
        );

        let mut v = open(&img);
        let mut from = 2;
        let mut steps = 0;
        loop {
            let z = v.zero_free(from, 16 * 1024).unwrap();
            steps += 1;
            if z.done {
                break;
            }
            assert!(z.next > from, "{name}: a step moved on");
            from = z.next;
        }
        assert!(steps > 1, "{name}: in steps");
        save(v, &img);

        let after = std::fs::read(&img).unwrap();
        assert!(!holds(&after, b"gone words"), "{name}: deleted bytes left");
        let long: Vec<u8> = "gone-words"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(!holds(&after, &long), "{name}: a deleted name left");
        fsck_clean(&img);
        let mut v = open(&img);
        assert_eq!(v.read("keep-one.txt", 1 << 20).unwrap(), pattern(7_000, 1));
        assert_eq!(v.read("keep-two.txt", 1 << 20).unwrap(), pattern(3_000, 2));
        assert_eq!(v.read("small.txt", 1 << 20).unwrap(), b"small");
    }
}

/// The whole partition zeroed: made empty with its layout and label, then
/// every cluster zeroed; no file's bytes are left, mtools lists nothing,
/// and the volume passes fsck and takes a new file.
#[test]
fn the_whole_partition_zeroed_is_an_empty_volume_with_its_label() {
    if !have_tools() {
        return;
    }
    for (name, args, kib, _) in volumes() {
        let dir = scratch(&format!("zero-all-{name}"));
        let img = mkfs(&dir, &args, kib);
        let i = img.to_str().unwrap();
        mput(
            &dir,
            &img,
            "vault.ofv",
            &marked(b"sealed vault bytes ", 50_000),
        );
        mput(&dir, &img, "notes.txt", &marked(b"plain notes ", 4_000));
        let mut v = open(&img);
        let label = v.label();
        v.make_empty().unwrap();
        let mut from = 2;
        while !v
            .zero_free(from, 1 << 20)
            .map(|z| {
                from = z.next;
                z.done
            })
            .unwrap()
        {}
        assert_eq!(v.label(), label, "{name}: the label kept");
        assert!(v.list().unwrap().is_empty(), "{name}: files left");
        save(v, &img);

        let after = std::fs::read(&img).unwrap();
        assert!(
            !holds(&after, b"sealed vault"),
            "{name}: a vault's bytes left"
        );
        assert!(
            !holds(&after, b"plain notes"),
            "{name}: a note's bytes left"
        );
        fsck_clean(&img);
        let (_, listing) = mtools("mdir", &["-i", i, "-b", "::"]);
        assert!(
            listing.is_empty(),
            "{name}: {}",
            String::from_utf8_lossy(&listing)
        );
        let mut v = open(&img);
        v.create("new.txt", b"after").unwrap();
        save(v, &img);
        fsck_clean(&img);
    }
}

/// A partition that remembers the most it was asked to read at once: what
/// the disk process holds in memory for it.
struct Watched {
    bytes: Vec<u8>,
    largest: usize,
}

impl Disk for Watched {
    fn size(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error> {
        self.largest = self.largest.max(buf.len());
        self.bytes.read_at(offset, buf)
    }
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), Error> {
        self.bytes.write_at(offset, buf)
    }
}

/// A FAT32 volume's root directory chained through `clusters` clusters,
/// in every copy of the FAT.
fn long_root(base: &[u8], clusters: u32) -> Vec<u8> {
    let mut bytes = base.to_vec();
    let le16 = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let le32 = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let bps = le16(11);
    let fat = le16(14) * bps;
    let fat_bytes = le32(36) as usize * bps;
    let copies = bytes[16] as usize;
    let root = le32(44);
    for k in 0..copies {
        let at = fat + k * fat_bytes;
        for c in root..root + clusters {
            let next = if c + 1 == root + clusters {
                0x0FFF_FFFF
            } else {
                c + 1
            };
            let e = at + c as usize * 4;
            bytes[e..e + 4].copy_from_slice(&next.to_le_bytes());
        }
    }
    bytes
}

/// A stick can make a FAT32 root directory's chain as long as its
/// partition. It is read only as far as a FAT directory can reach; one
/// longer is refused as damaged, and nothing on the way reads more than
/// a piece of the disk at once.
#[test]
fn a_fat32_root_longer_than_fat_allows_is_refused() {
    if !have_tools() {
        return;
    }
    let dir = scratch("longroot");
    // 512-byte clusters: the most a directory holds is 4096 of them.
    let img = mkfs(&dir, &["-F", "32", "-s", "1"], 40 * 1024);
    let base = std::fs::read(&img).unwrap();
    let most = (MAX_ROOT_BYTES / 512) as u32;

    let mut v = Volume::open(Watched {
        bytes: long_root(&base, most + 1),
        largest: 0,
    })
    .unwrap();
    assert!(matches!(v.list(), Err(Error::Corrupt(_))));
    assert!(matches!(
        v.read("anything.txt", 1 << 20),
        Err(Error::Corrupt(_))
    ));
    assert!(matches!(
        v.create("new.txt", b"faraday"),
        Err(Error::Corrupt(_))
    ));
    assert!(v.into_disk().largest <= FAT_PIECE);

    // As long as a directory may be: read, and written to.
    let mut v = Volume::open(long_root(&base, most)).unwrap();
    assert!(v.list().unwrap().is_empty());
    v.create("new.txt", b"faraday").unwrap();
    assert_eq!(v.read("new.txt", 1 << 20).unwrap(), b"faraday");
}

/// Counting the free space and finding room for a file read the FAT a
/// piece at a time, however large the volume's FAT is.
#[test]
fn the_fat_is_read_a_piece_at_a_time() {
    if !have_tools() {
        return;
    }
    let dir = scratch("fatpiece");
    // About 80,000 clusters: a FAT of over 300 KiB.
    let img = mkfs(&dir, &["-F", "32", "-s", "1"], 40 * 1024);
    let bytes = std::fs::read(&img).unwrap();
    let mut v = Volume::open(Watched { bytes, largest: 0 }).unwrap();
    let (free, total) = v.space().unwrap();
    v.create("one.psbt", &pattern(3_000, 1)).unwrap();
    v.create("two.psbt", &pattern(700, 2)).unwrap();
    let (after, same) = v.space().unwrap();
    assert_eq!(same, total);
    // 3,000 bytes take six clusters and 700 two.
    assert_eq!(free - after, 8 * 512);
    assert_eq!(v.read("two.psbt", 1 << 20).unwrap(), pattern(700, 2));
    let w = v.into_disk();
    assert!(w.largest <= FAT_PIECE, "{}", w.largest);
    std::fs::write(&img, w.bytes).unwrap();
    fsck_clean(&img);
}

/// A partition that stops taking writes after `left` of them, as a stick
/// pulled part way.
struct Pulled {
    bytes: Vec<u8>,
    left: usize,
}

impl Disk for Pulled {
    fn size(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error> {
        self.bytes.read_at(offset, buf)
    }
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), Error> {
        if self.left == 0 {
            return Err(Error::Io);
        }
        self.left -= 1;
        self.bytes.write_at(offset, buf)
    }
}

/// A rename interrupted at any write never leaves two entries on one
/// chain (the file is under one name, or under none with its clusters
/// left lost for a check to reclaim); one that finishes leaves the new
/// name.
#[test]
fn a_rename_cut_part_way_leaves_no_two_entries_on_one_chain() {
    if !have_tools() {
        return;
    }
    for (name, args, kib, _) in volumes() {
        let dir = scratch(&format!("rename-cut-{name}"));
        let img = mkfs(&dir, &args, kib);
        let mut v = open(&img);
        v.create("wallet.txt.new", &pattern(5_000, 9)).unwrap();
        let base = v.into_disk();
        let mut finished = false;
        for left in 0..40 {
            let mut v = Volume::open(Pulled {
                bytes: base.clone(),
                left,
            })
            .unwrap();
            let done = v.rename("wallet.txt.new", "wallet.txt").is_ok();
            let mut v = Volume::open(v.into_disk().bytes).unwrap();
            let names: Vec<String> = v.list().unwrap().into_iter().map(|e| e.name).collect();
            assert!(names.len() <= 1, "{name}: cut after {left}: {names:?}");
            for n in &names {
                assert_eq!(v.read(n, 1 << 20).unwrap(), pattern(5_000, 9), "{name}");
            }
            if done {
                assert_eq!(names, ["wallet.txt"], "{name}");
                finished = true;
                break;
            }
        }
        assert!(finished, "{name}");
    }
}

/// A name that differs only in case is a rename too.
#[test]
fn a_rename_that_changes_only_the_case_works() {
    if !have_tools() {
        return;
    }
    for (name, args, kib, _) in volumes() {
        let dir = scratch(&format!("rename-case-{name}"));
        let img = mkfs(&dir, &args, kib);
        let mut v = open(&img);
        v.create("README.TXT", b"hello").unwrap();
        v.rename("README.TXT", "readme.txt").unwrap();
        let names: Vec<String> = v.list().unwrap().into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["readme.txt"], "{name}");
        assert_eq!(v.read("readme.txt", 1 << 20).unwrap(), b"hello");
    }
}

/// What is past a file's end in its last cluster, and in clusters its
/// chain holds beyond its size, is zeroed with the free space: the
/// file's last cluster is the one its size says, not the chain's last.
#[test]
fn bytes_past_a_files_end_are_zeroed_by_its_size_not_by_its_chain() {
    if !have_tools() {
        return;
    }
    for (name, args, kib, _) in volumes() {
        let dir = scratch(&format!("zero-tail-{name}"));
        let img = mkfs(&dir, &args, kib);
        mput(&dir, &img, "LEFT.TXT", &marked(b"past-the-end ", 20_000));
        let mut bytes = std::fs::read(&img).unwrap();
        // The size in its entry cut to 1,300 bytes, as a file truncated
        // by a program that left the chain as it was.
        let at = bytes.windows(11).position(|w| w == b"LEFT    TXT").unwrap();
        bytes[at + 28..at + 32].copy_from_slice(&1_300u32.to_le_bytes());
        let count = |b: &[u8]| b.windows(13).filter(|w| *w == b"past-the-end ").count();
        let before = count(&bytes);
        let mut v = Volume::open(bytes).unwrap();
        v.zero_free(2, 1 << 30).unwrap();
        let after = count(&v.into_disk());
        // Only the file's first 1,300 bytes keep the text.
        assert!(before > 1_000, "{name}: {before}");
        assert!(after <= 1_300 / 13 + 1, "{name}: {after} left of {before}");
    }
}

/// A file whose size is whole clusters, in a chain that runs longer than
/// its size needs: the clusters past its size are zeroed with the free
/// space too.
#[test]
fn clusters_past_a_whole_cluster_size_are_zeroed_too() {
    if !have_tools() {
        return;
    }
    for (name, args, kib, _) in volumes() {
        let dir = scratch(&format!("zero-whole-{name}"));
        let img = mkfs(&dir, &args, kib);
        mput(&dir, &img, "LEFT.TXT", &marked(b"past-the-end ", 20_000));
        let mut bytes = std::fs::read(&img).unwrap();
        let at = bytes.windows(11).position(|w| w == b"LEFT    TXT").unwrap();
        // Two whole clusters, whatever the volume's cluster size is.
        let cb = u32::from(u16::from_le_bytes([bytes[11], bytes[12]])) * u32::from(bytes[13]);
        bytes[at + 28..at + 32].copy_from_slice(&(2 * cb).to_le_bytes());
        let count = |b: &[u8]| b.windows(13).filter(|w| *w == b"past-the-end ").count();
        let before = count(&bytes);
        let mut v = Volume::open(bytes).unwrap();
        v.zero_free(2, 1 << 30).unwrap();
        let after = count(&v.into_disk());
        assert!(before > 1_000, "{name}: {before}");
        assert!(
            after <= (2 * cb as usize) / 13 + 1,
            "{name}: {after} of {before}"
        );
    }
}

/// The Starter's signed boot file (`docs/STARTER.md` §4.5): a boot
/// partition made as the stick's is (`genimage.cfg`: FAT32, 512-byte
/// clusters, `EFI/BOOT/BOOTX64.EFI` put there by mtools) has that file
/// read and written over in its folder, longer and then shorter, and
/// mtools and `fsck.fat` read the result as the new file and nothing
/// else changed.
#[test]
fn a_file_in_a_folder_is_read_and_written_over_where_it_is() {
    if !have_tools() {
        return;
    }
    let dir = scratch("rewrite");
    let img = mkfs(
        &dir,
        &["-F", "32", "-s", "1", "-n", "FARADAYBOOT"],
        64 * 1024,
    );
    let i = img.to_str().unwrap();
    for folder in ["::EFI", "::EFI/BOOT"] {
        assert!(mtools("mmd", &["-i", i, folder]).0, "mmd {folder}");
    }
    let src = dir.join("src");
    let boot = pattern(30_000, 1);
    std::fs::write(&src, &boot).unwrap();
    assert!(
        mtools(
            "mcopy",
            &["-i", i, src.to_str().unwrap(), "::EFI/BOOT/BOOTX64.EFI"]
        )
        .0
    );
    std::fs::write(&src, pattern(700, 2)).unwrap();
    assert!(mtools("mcopy", &["-i", i, src.to_str().unwrap(), "::cmdline.txt"]).0);
    let path = ["EFI", "BOOT", "BOOTX64.EFI"];

    let mut v = open(&img);
    assert_eq!(v.read_in(&path, 1 << 20).unwrap(), boot);
    assert_eq!(v.read_in(&path, 100), Err(Error::TooLarge));
    assert_eq!(v.read_in(&["EFI", "BOOT"], 1 << 20), Err(Error::NotFound));
    assert_eq!(
        v.read_in(&["EFI", "BOOT", "BOOTIA32.EFI"], 1 << 20),
        Err(Error::NotFound)
    );
    let (free_before, _) = v.space().unwrap();
    // Longer by a signature: 1,800 bytes past the old end, four clusters.
    let mut longer = boot.clone();
    longer[100] ^= 0xFF;
    longer.extend_from_slice(&pattern(1_800, 3));
    v.rewrite_in(&path, &longer).unwrap();
    assert_eq!(v.read_in(&path, 1 << 20).unwrap(), longer);
    let (free_after, _) = v.space().unwrap();
    assert_eq!(free_before - free_after, 4 * 512);
    save(v, &img);
    fsck_clean(&img);
    let out = dir.join("out");
    let read_out = |name: &str| {
        let _ = std::fs::remove_file(&out);
        assert!(
            mtools("mcopy", &["-n", "-i", i, name, out.to_str().unwrap()]).0,
            "{name}"
        );
        std::fs::read(&out).unwrap()
    };
    assert_eq!(read_out("::EFI/BOOT/BOOTX64.EFI"), longer);
    assert_eq!(read_out("::cmdline.txt"), pattern(700, 2));

    // Shorter again: the clusters it no longer needs are free.
    let mut v = open(&img);
    v.rewrite_in(&path, &boot).unwrap();
    assert_eq!(v.space().unwrap().0, free_before);
    save(v, &img);
    fsck_clean(&img);
    assert_eq!(read_out("::EFI/BOOT/BOOTX64.EFI"), boot);
}
