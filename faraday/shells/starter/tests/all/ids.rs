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
    self, BOOT_FIRST, DATA_FIRST, DATA_ROOT, DATA_SECTORS, IDS, IMAGE, SECTOR, SERIALS, STICK,
    Sparse, card_id_places, card_image, ids_put_back, read, stick_id_places, stick_image,
};
use crate::writer::{FileDisk, image_job, release_payload, run, tmp};

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
fn two_sticks_written_from_one_image_have_their_own_ids_and_each_checks_against_the_manifest() {
    let image = stick_image();
    let (job, source) = release_payload(&image);
    let manifest = match &job {
        Job::Payload { file, .. } => file.inner_sha256.to_string(),
        Job::Image(i) => i.sum.sha256.to_string(),
    };
    let mut seen = Vec::new();
    for n in 0..2 {
        let path = tmp(&format!("ids-stick-{n}"));
        Sparse::new(STICK).write(&path);
        let mut disk = FileDisk::new(&path);
        let order = disk.order(source.clone(), Mode::Whole);
        assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
        let ids = ids_on(&path, STICK);
        // Its own: none of the image's, in GUIDs of version 4's form, and
        // each FAT volume's backup boot sector the same as its own.
        assert!(
            !IDS.iter()
                .any(|i| ids.disk == *i || ids.parts.contains(&i.to_string()))
        );
        assert!(v4(&ids.disk) && ids.parts.iter().all(|p| v4(p)), "{ids:?}");
        for (k, &(own, backup)) in ids.serials.iter().enumerate() {
            assert_eq!(own, backup);
            assert_ne!(own, SERIALS[k]);
        }
        // With the image's IDs put back, its SHA-256 is the manifest's.
        let mut back = read(&path, 0, IMAGE as usize);
        ids_put_back(&mut back, &image, &stick_id_places(IMAGE));
        assert_eq!(sticks::sha(&back), manifest);
        seen.push(ids);
    }
    // And the two sticks have nothing in common.
    let (a, b) = (&seen[0], &seen[1]);
    assert_ne!(a.disk, b.disk);
    assert!(a.parts.iter().all(|p| !b.parts.contains(p)));
    for k in 0..2 {
        assert_ne!(a.serials[k].0, b.serials[k].0);
    }
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
    volume.volume_label().expect("its root directory read")
}

#[test]
fn a_stick_written_with_a_name_shows_it_and_still_checks_against_the_manifest() {
    let image = stick_image();
    let (job, source) = release_payload(&image);
    let path = tmp("name-stick");
    Sparse::new(STICK).write(&path);
    let mut disk = FileDisk::new(&path);
    let order = faraday_starter::api::WriteOrder {
        name: Some(String::from("MY STICK")),
        ..disk.order(source, Mode::Whole)
    };
    // The writer's own check: read back, and with the IDs and the name
    // taken back out, the manifest's SHA-256.
    assert_eq!(run(&order, &job, &mut disk).0, Ok(()));
    let data = DATA_FIRST * SECTOR;
    assert_eq!(
        name_on(&path, data, DATA_SECTORS * SECTOR).as_deref(),
        Some("MY STICK")
    );
    // The boot partition keeps its own label; the name is the data
    // partition's alone.
    assert_eq!(
        faraday_fat::boot_sector_label(&read(&path, BOOT_FIRST * SECTOR, 512)),
        Some(*b"EFI        ")
    );
    // What a person puts back by hand: the IDs, the boot sectors' label
    // and the root directory's first entry.
    let mut back = read(&path, 0, IMAGE as usize);
    let mut places = stick_id_places(IMAGE);
    places.extend([
        (data + 71, data + 82),
        (data + 6 * SECTOR + 71, data + 6 * SECTOR + 82),
        (data + DATA_ROOT, data + DATA_ROOT + 32),
    ]);
    ids_put_back(&mut back, &image, &places);
    assert!(back == image.to_vec());
}

#[test]
fn a_pi_card_written_with_a_name_shows_it() {
    let image = card_image(0x5A5A_0001);
    let (img, source) = image_job("name-card.img", &image);
    let path = tmp("name-card");
    Sparse::new(STICK).write(&path);
    let mut disk = FileDisk::new(&path);
    let order = faraday_starter::api::WriteOrder {
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
    let (job, source) = release_payload(&image);
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

fn tool(name: &str) -> PathBuf {
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

fn runs(name: &str, args: &[&str]) -> bool {
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
fn a_stick_written_from_an_image_other_tools_made_reads_in_those_tools_with_its_own_ids() {
    if !(runs("sfdisk", &["--version"])
        && runs("mkfs.fat", &["--help"])
        && runs("fsck.fat", &["--help"])
        && runs("fatlabel", &["--help"]))
    {
        eprintln!("sfdisk or dosfstools is not installed: the check against them is skipped");
        return;
    }
    let dir = tmp("ids-tools");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let img = tools_image(&dir);
    let image = faraday_starter_shell::payload::Image::hash(&img).expect("hashed");
    let source = faraday_starter::api::Source::Image {
        path: img.display().to_string(),
        sha256: image.sum.sha256.to_string(),
    };
    let stick = dir.join("stick");
    Sparse::new(STICK).write(&stick);
    let mut disk = FileDisk::new(&stick);
    let order = disk.order(source, Mode::Whole);
    assert_eq!(run(&order, &Job::Image(image), &mut disk).0, Ok(()));
    // sfdisk reads the stick's GPT, both copies, with its own IDs.
    let dump = Command::new(tool("sfdisk"))
        .args(["--dump"])
        .arg(&stick)
        .output()
        .expect("sfdisk --dump");
    assert!(
        dump.status.success(),
        "{}",
        String::from_utf8_lossy(&dump.stderr)
    );
    let text = String::from_utf8_lossy(&dump.stdout).to_uppercase();
    let ids = ids_on(&stick, STICK);
    assert!(text.contains(&format!("LABEL-ID: {}", ids.disk)), "{text}");
    for (p, image_id) in ids.parts.iter().zip(&IDS[1..]) {
        assert!(text.contains(&format!("UUID={p}")), "{text}");
        assert!(!text.contains(image_id), "{text}");
    }
    let stderr = String::from_utf8_lossy(&dump.stderr).to_lowercase();
    assert!(
        !stderr.contains("corrupt") && !stderr.contains("checksum"),
        "{stderr}"
    );
    // Each volume passes fsck.fat, its backup boot sector the same as its
    // own, and fatlabel reads the serial Faraday put there.
    for (k, (at, len)) in [(BOOT_FIRST, 65536u64 * 1024), (DATA_FIRST, 131_072 * 1024)]
        .into_iter()
        .enumerate()
    {
        let vol = dir.join(format!("written-{k}.vfat"));
        std::fs::write(&vol, read(&stick, at * SECTOR, len as usize)).expect("the volume");
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
            .arg("-i")
            .arg(&vol)
            .output()
            .expect("fatlabel -i");
        let shown = String::from_utf8_lossy(&label.stdout).trim().to_lowercase();
        let serial = format!("{:08x}", ids.serials[k].0);
        assert!(
            label.status.success() && shown.contains(&serial),
            "{shown} {serial}"
        );
        assert_ne!(serial, format!("{:08x}", SERIALS[k]));
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
        return;
    }
    let dir = tmp("name-tools");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let img = tools_image(&dir);
    let image = faraday_starter_shell::payload::Image::hash(&img).expect("hashed");
    let source = faraday_starter::api::Source::Image {
        path: img.display().to_string(),
        sha256: image.sum.sha256.to_string(),
    };
    let stick = dir.join("stick");
    Sparse::new(STICK).write(&stick);
    let mut disk = FileDisk::new(&stick);
    let order = faraday_starter::api::WriteOrder {
        name: Some(String::from("SAVINGS 2")),
        ..disk.order(source, Mode::Whole)
    };
    assert_eq!(run(&order, &Job::Image(image), &mut disk).0, Ok(()));
    let vol = dir.join("named.vfat");
    std::fs::write(
        &vol,
        read(
            &stick,
            DATA_FIRST * SECTOR,
            (DATA_SECTORS * SECTOR) as usize,
        ),
    )
    .expect("the volume");
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
