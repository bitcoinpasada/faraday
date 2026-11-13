//! Each stick's and card's own IDs (`docs/STICK-IDENTITY.md` §4, §10):
//! two sticks written by the Starter from one image differ in every ID,
//! each reads back as the image with its IDs, and with the image's IDs
//! put back its SHA-256 is the manifest's; a Pi card the same. A name
//! given the data partition (§6) is on the stick, read as a computer
//! reads it, and the check still holds. Checked against util-linux's
//! `sfdisk` and dosfstools' `mkfs.fat`, `fsck.fat` and `fatlabel`, which
//! Faraday did not write, where they are installed.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use faraday_starter::api::{Mode, Reason, StickFault};
use faraday_starter_shell::gpt;
use faraday_starter_shell::writer::Job;

use crate::sticks::{
    self, BOOT_FIRST, DATA_FIRST, IDS, IMAGE, SECTOR, SERIALS, STICK, Sparse, card_id_places,
    card_image, ids_put_back, read, stick_image,
};
use crate::writer::{FileDisk, image_job, made_from, release_image, run, tmp};

/// The data partition's length on a stick made for a disk of `disk_len`
/// bytes: from sector 133120 to the last usable sector.
fn data_len(disk_len: u64) -> u64 {
    (disk_len / SECTOR - 33) * SECTOR - DATA_FIRST * SECTOR
}

/// The FAT32 volume at `at` of the stick at `path`, `len` bytes, as a
/// sparse file of its own at `to`: only what the Starter wrote (its
/// reserved sectors, FATs and root directory) is copied, so a volume of
/// hundreds of gigabytes is not read whole.
fn extract_volume(path: &Path, at: u64, len: u64, to: &Path) {
    use std::os::unix::fs::FileExt;
    let bs = read(path, at, 512);
    let u16_at = |i: usize| u64::from(u16::from_le_bytes([bs[i], bs[i + 1]]));
    let fat = u64::from(u32::from_le_bytes(bs[36..40].try_into().expect("four")));
    let head = (u16_at(14) + 2 * fat + u64::from(bs[13])) * SECTOR;
    let out = std::fs::File::create(to).expect("a volume");
    out.set_len(len).expect("its length");
    let src = std::fs::File::open(path).expect("the stick");
    let mut done = 0;
    while done < head {
        let n = (head - done).min(1 << 20) as usize;
        let mut buf = vec![0u8; n];
        src.read_exact_at(&mut buf, at + done).expect("read");
        if buf.iter().any(|&b| b != 0) {
            out.write_all_at(&buf, done).expect("written");
        }
        done += n as u64;
    }
}

/// A stick's IDs as read back from the disk at `path`: its disk GUID,
/// its partitions' GUIDs, and each FAT volume's serial in its boot
/// sector and its backup.
#[derive(Debug, PartialEq, Eq)]
struct Read {
    disk: String,
    parts: Vec<String>,
    serials: Vec<(u32, u32)>,
}

fn ids_on(path: &Path, len: u64) -> Read {
    let file = std::fs::File::open(path).expect("open");
    let table = gpt::read(
        &mut |at, buf| {
            use std::os::unix::fs::FileExt;
            file.read_exact_at(buf, at)
        },
        len,
    )
    .expect("a GPT whose CRCs hold");
    let serial = |at: u64| faraday_fat::boot_sector_serial(&read(path, at, 512)).expect("a serial");
    Read {
        disk: table.disk_guid.clone(),
        parts: table.parts.iter().map(|p| p.guid.clone()).collect(),
        serials: [BOOT_FIRST, DATA_FIRST]
            .iter()
            .map(|s| s * SECTOR)
            .map(|at| (serial(at), serial(at + 6 * SECTOR)))
            .collect(),
    }
}

/// Version 4 in form (RFC 9562): `xxxxxxxx-xxxx-4xxx-[89AB]xxx-…`.
fn v4(guid: &str) -> bool {
    let c: Vec<char> = guid.chars().collect();
    c.len() == 36 && c[14] == '4' && "89AB".contains(c[19])
}

#[test]
fn two_sticks_written_from_one_image_have_their_own_ids_and_the_same_boot_partition() {
    let image = stick_image();
    let (job, source) = release_image(&image);
    let (from, to) = sticks::boot_range();
    let boot = image.bytes(from, (to - from) as usize);
    let mut seen = Vec::new();
    for n in 0..2 {
        let path = tmp(&format!("ids-stick-{n}"));
        Sparse::new(STICK).write(&path);
        let mut disk = FileDisk::new(&path);
        let order = disk.order(source.clone(), Mode::Whole);
        assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
        made_from(&path, &image, STICK);
        let ids = ids_on(&path, STICK);
        // Its own: none of the image's, in GUIDs of version 4's form.
        assert!(
            !IDS.iter()
                .any(|i| ids.disk == *i || ids.parts.contains(&i.to_string()))
        );
        assert!(v4(&ids.disk) && ids.parts.iter().all(|p| v4(p)), "{ids:?}");
        // The boot partition keeps the release's serial, and so is the
        // release's byte for byte; the data partition has its own, in
        // its backup boot sector too.
        assert_eq!(ids.serials[0], (SERIALS[0], SERIALS[0]));
        assert_eq!(ids.serials[1].0, ids.serials[1].1);
        assert_ne!(ids.serials[1].0, SERIALS[1]);
        assert!(read(&path, from, boot.len()) == boot);
        seen.push(ids);
    }
    // And the two sticks have nothing in common but the boot partition.
    let (a, b) = (&seen[0], &seen[1]);
    assert_ne!(a.disk, b.disk);
    assert!(a.parts.iter().all(|p| !b.parts.contains(p)));
    assert_ne!(a.serials[1].0, b.serials[1].0);
}

#[test]
fn a_stick_written_from_a_found_release_file_has_its_own_ids_and_the_releases_boot_partition() {
    let image = stick_image();
    let (job, source, _) = crate::writer::found_job("ids-found", &image);
    let (from, to) = sticks::boot_range();
    let path = tmp("ids-found-stick");
    Sparse::new(STICK).write(&path);
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::Whole);
    assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
    made_from(&path, &image, STICK);
    let ids = ids_on(&path, STICK);
    assert!(
        !IDS.iter()
            .any(|i| ids.disk == *i || ids.parts.contains(&i.to_string()))
    );
    assert!(v4(&ids.disk) && ids.parts.iter().all(|p| v4(p)), "{ids:?}");
    assert_eq!(ids.serials[0], (SERIALS[0], SERIALS[0]));
    assert_ne!(ids.serials[1].0, SERIALS[1]);
    assert!(read(&path, from, (to - from) as usize) == image.bytes(from, (to - from) as usize));
}

#[test]
fn a_system_that_mounts_a_new_table_at_once_cannot_fail_a_card_whose_table_goes_on_last() {
    let image = card_image(0x5A5A_0001);
    let (img, source) = image_job("ids-card-mounts.img", &image);
    let job = Job::Image(img);
    let path = tmp("ids-card-mounts");
    Sparse::new(STICK).write(&path);
    let mut disk = FileDisk::new(&path);
    disk.mounts_at = Some((4 << 20) + 3);
    disk.table_last = true;
    let order = disk.order(source.clone(), Mode::Whole);
    assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
    let signature = u32::from_le_bytes(read(&path, 440, 4).try_into().expect("four"));
    assert!(signature != 0 && signature != 0x5A5A_0001);
    assert_eq!(read(&path, 510, 2), [0x55, 0xAA]);
    let mut back = read(&path, 0, image.len as usize);
    // Only what the system changed once the table was on differs.
    back[(4 << 20) + 3] ^= 1;
    ids_put_back(&mut back, &image, &card_id_places());
    assert!(back == image.to_vec());
    // The same system, with the table first, fails the same write.
    let path = tmp("ids-card-mounts-first");
    Sparse::new(STICK).write(&path);
    let mut disk = FileDisk::new(&path);
    disk.mounts_at = Some((4 << 20) + 3);
    let order = disk.order(source, Mode::Whole);
    let r = run(&order, &job, &mut disk).0;
    assert!(matches!(r, Err(Reason::BackDiffers { .. })), "{r:?}");
}

#[test]
fn a_pi_card_is_given_its_own_disk_signature_and_serials() {
    let image = card_image(0x5A5A_0001);
    let (img, source) = image_job("ids-card.img", &image);
    let job = Job::Image(img);
    let mut signatures = Vec::new();
    for n in 0..2 {
        let path = tmp(&format!("ids-card-{n}"));
        Sparse::new(STICK).write(&path);
        let mut disk = FileDisk::new(&path);
        let order = disk.order(source.clone(), Mode::Whole);
        assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
        let mbr = read(&path, 0, 512);
        let signature = u32::from_le_bytes(mbr[440..444].try_into().expect("four"));
        assert!(signature != 0 && signature != 0x5A5A_0001);
        let boot = faraday_fat::boot_sector_serial(&read(&path, 4 << 20, 512));
        let data = faraday_fat::boot_sector_serial(&read(&path, 68 << 20, 512));
        assert!(boot.is_some_and(|s| s != SERIALS[0]));
        assert!(data.is_some_and(|s| s != SERIALS[1]));
        let mut back = read(&path, 0, image.len as usize);
        ids_put_back(&mut back, &image, &card_id_places());
        assert!(back == image.to_vec());
        signatures.push(signature);
    }
    assert_ne!(signatures[0], signatures[1]);
}

/// The name a computer shows for the FAT volume of `len` bytes at `at`
/// on the disk at `path`, as `faraday_fat` reads it.
fn name_on(path: &Path, at: u64, len: u64) -> Option<String> {
    let mut volume = faraday_fat::Volume::open(read(path, at, len as usize)).expect("a FAT volume");
    let label = volume.volume_label().expect("its root directory read");
    label.map(|l| String::from(&*l))
}

#[test]
fn a_stick_written_with_a_name_shows_it_on_the_data_partition_alone() {
    let image = stick_image();
    let (job, source) = release_image(&image);
    let path = tmp("name-stick");
    Sparse::new(STICK).write(&path);
    let mut disk = FileDisk::new(&path);
    let order = faraday_starter::api::WriteOrder {
        clear_first: false,
        name: Some(String::from("MY STICK")),
        ..disk.order(source, Mode::Whole)
    };
    assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
    made_from(&path, &image, STICK);
    assert_eq!(
        name_on(&path, DATA_FIRST * SECTOR, data_len(STICK)).as_deref(),
        Some("MY STICK")
    );
    // The boot partition keeps its own label.
    assert_eq!(
        faraday_fat::boot_sector_label(&read(&path, BOOT_FIRST * SECTOR, 512)),
        Some(*b"EFI        ")
    );
}

#[test]
fn a_pi_card_written_with_a_name_shows_it() {
    let image = card_image(0x5A5A_0001);
    let (img, source) = image_job("name-card.img", &image);
    let path = tmp("name-card");
    Sparse::new(STICK).write(&path);
    let mut disk = FileDisk::new(&path);
    let order = faraday_starter::api::WriteOrder {
        clear_first: false,
        name: Some(String::from("CARD-1")),
        ..disk.order(source, Mode::Whole)
    };
    assert_eq!(run(&order, &Job::Image(img), &mut disk).0, Ok(()));
    assert_eq!(
        name_on(&path, 68 << 20, 128 << 20).as_deref(),
        Some("CARD-1")
    );
}

#[test]
fn a_name_a_fat_label_cannot_be_or_a_boot_partitions_is_refused_and_nothing_is_written() {
    let image = stick_image();
    let (job, source) = release_image(&image);
    for name in [
        "EFI",
        "bootfs",
        "BOOTFS",
        "FARADAYBOOT",
        "my stick",
        "TWELVE CHARS",
        "A/B",
        "",
    ] {
        let path = tmp("name-refused");
        Sparse::new(STICK).write(&path);
        let mut disk = FileDisk::new(&path);
        let order = faraday_starter::api::WriteOrder {
            clear_first: false,
            name: Some(name.to_string()),
            ..disk.order(source.clone(), Mode::Whole)
        };
        assert_eq!(
            run(&order, &job, &mut disk).0,
            Err(Reason::NotStickImage(StickFault::Name)),
            "{name}"
        );
        assert!(read(&path, 0, 1 << 20).iter().all(|&b| b == 0), "{name}");
    }
}

// ---- against tools Faraday did not write ---------------------------------

pub(crate) fn tool(name: &str) -> PathBuf {
    if let Some(dir) = std::env::var_os("FARADAY_FATTOOLS") {
        return Path::new(&dir).join(name);
    }
    // Debian puts these in /usr/sbin, which a shell that is not a login
    // one may not have on PATH.
    ["/usr/sbin", "/sbin"]
        .iter()
        .map(|d| Path::new(d).join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

pub(crate) fn runs(name: &str, args: &[&str]) -> bool {
    Command::new(tool(name))
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// A stick image made as genimage makes one, by other tools: the GPT by
/// `sfdisk`, the two FAT32 volumes by `mkfs.fat`.
fn tools_image(dir: &Path) -> PathBuf {
    let img = dir.join("tools.img");
    let f = std::fs::File::create(&img).expect("an image");
    f.set_len(IMAGE).expect("its length");
    drop(f);
    let script = format!(
        "label: gpt\nlabel-id: {}\nfirst-lba: 34\n\
         start=2048, size=131072, type={}, uuid={}, name=\"EFI system partition\"\n\
         start=133120, size=262144, type={}, uuid={}, name=\"Basic data partition\"\n",
        IDS[0],
        sticks::ESP,
        IDS[1],
        sticks::BASIC,
        IDS[2]
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
    for (name, args, at) in [
        ("boot.vfat", vec!["-n", "EFI", "-i", "6E1F2A01"], BOOT_FIRST),
        ("data.vfat", vec!["-s", "2", "-i", "6E1F2A02"], DATA_FIRST),
    ] {
        let vol = dir.join(name);
        let kib = if at == BOOT_FIRST { "65536" } else { "131072" };
        let ok = Command::new(tool("mkfs.fat"))
            .args(["-F", "32"])
            .args(&args)
            .arg("-C")
            .arg(&vol)
            .arg(kib)
            .stdout(Stdio::null())
            .status()
            .expect("mkfs.fat");
        assert!(ok.success());
        let bytes = std::fs::read(&vol).expect("the volume");
        use std::os::unix::fs::FileExt;
        let out = std::fs::OpenOptions::new()
            .write(true)
            .open(&img)
            .expect("the image");
        out.write_all_at(&bytes, at * SECTOR).expect("written in");
    }
    img
}

#[test]
fn a_stick_made_for_its_size_reads_clean_in_other_tools_at_1_32_and_256_gb() {
    if !(runs("sfdisk", &["--version"])
        && runs("mkfs.fat", &["--help"])
        && runs("fsck.fat", &["--help"]))
    {
        eprintln!("sfdisk or dosfstools is not installed: the check against them is skipped");
        if std::env::var_os("FARADAY_JUDGES_REQUIRED").is_some() {
            panic!("FARADAY_JUDGES_REQUIRED is set and sfdisk or dosfstools is not installed");
        }
        return;
    }
    let sgdisk = runs("sgdisk", &["--version"]);
    let dir = tmp("ids-tools");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let img = tools_image(&dir);
    let image = faraday_starter_shell::image::Image::hash(&img).expect("hashed");
    let source = faraday_starter::api::Source::Image {
        path: img.display().to_string(),
        sha256: image.sum.sha256.to_string(),
    };
    let (from, to) = sticks::boot_range();
    let released = sticks::sha(&read(&img, from, (to - from) as usize));
    for gb in [1u64, 32, 256] {
        let size = gb << 30;
        let stick = dir.join(format!("stick-{gb}"));
        Sparse::new(size).write(&stick);
        let mut disk = FileDisk::new(&stick);
        let order = disk.order(source.clone(), Mode::Whole);
        assert_eq!(run(&order, &Job::Image(image.clone()), &mut disk).0, Ok(()));
        // The table, both copies, and both partitions.
        let verify = Command::new(tool("sfdisk"))
            .arg("--verify")
            .arg(&stick)
            .output()
            .expect("sfdisk --verify");
        let said = String::from_utf8_lossy(&verify.stdout).to_string()
            + &String::from_utf8_lossy(&verify.stderr);
        assert!(
            verify.status.success() && said.contains("No errors"),
            "{gb} GB: {said}"
        );
        let list = Command::new(tool("sfdisk"))
            .arg("--list")
            .arg(&stick)
            .output()
            .expect("sfdisk --list");
        let listed = String::from_utf8_lossy(&list.stdout).to_lowercase();
        assert!(
            listed.contains("efi system") && listed.contains("microsoft basic data"),
            "{gb} GB: {listed}"
        );
        if sgdisk {
            let v = Command::new(tool("sgdisk"))
                .arg("-v")
                .arg(&stick)
                .output()
                .expect("sgdisk -v");
            let said = String::from_utf8_lossy(&v.stdout).to_string();
            assert!(
                v.status.success() && said.contains("No problems found"),
                "{gb} GB: {said}"
            );
        }
        // The data partition is clean to fsck.fat, at the disk's size.
        let vol = dir.join(format!("data-{gb}.vfat"));
        extract_volume(&stick, DATA_FIRST * SECTOR, data_len(size), &vol);
        let fsck = Command::new(tool("fsck.fat"))
            .arg("-n")
            .arg(&vol)
            .output()
            .expect("fsck.fat");
        assert!(
            fsck.status.success(),
            "{gb} GB: {}",
            String::from_utf8_lossy(&fsck.stdout)
        );
        // The boot partition is the image's: the manifest's value.
        assert_eq!(
            sticks::sha(&read(&stick, from, (to - from) as usize)),
            released
        );
        std::fs::remove_file(&stick).expect("removed");
    }
}

#[test]
fn a_name_the_starter_wrote_reads_the_same_in_fatlabel_and_the_volume_passes_fsck() {
    if !(runs("sfdisk", &["--version"])
        && runs("mkfs.fat", &["--help"])
        && runs("fsck.fat", &["--help"])
        && runs("fatlabel", &["--help"]))
    {
        eprintln!("sfdisk or dosfstools is not installed: the check against them is skipped");
        if std::env::var_os("FARADAY_JUDGES_REQUIRED").is_some() {
            panic!("FARADAY_JUDGES_REQUIRED is set and sfdisk or dosfstools is not installed");
        }
        return;
    }
    let dir = tmp("name-tools");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let img = tools_image(&dir);
    let image = faraday_starter_shell::image::Image::hash(&img).expect("hashed");
    let source = faraday_starter::api::Source::Image {
        path: img.display().to_string(),
        sha256: image.sum.sha256.to_string(),
    };
    let stick = dir.join("stick");
    Sparse::new(STICK).write(&stick);
    let mut disk = FileDisk::new(&stick);
    let order = faraday_starter::api::WriteOrder {
        clear_first: false,
        name: Some(String::from("SAVINGS 2")),
        ..disk.order(source, Mode::Whole)
    };
    assert_eq!(run(&order, &Job::Image(image), &mut disk).0, Ok(()));
    let vol = dir.join("named.vfat");
    extract_volume(&stick, DATA_FIRST * SECTOR, data_len(STICK), &vol);
    let fsck = Command::new(tool("fsck.fat"))
        .arg("-n")
        .arg(&vol)
        .output()
        .expect("fsck.fat");
    assert!(
        fsck.status.success(),
        "{}",
        String::from_utf8_lossy(&fsck.stdout)
    );
    let label = Command::new(tool("fatlabel"))
        .arg(&vol)
        .output()
        .expect("fatlabel");
    assert!(label.status.success());
    assert_eq!(String::from_utf8_lossy(&label.stdout).trim(), "SAVINGS 2");
}
