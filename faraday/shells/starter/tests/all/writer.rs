//! The writer against regular files holding images, as
//! `faraday/image/stick-boot.sh` allows: what a person sees is what is
//! on the stick afterwards, and nothing when a check refuses.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use faraday_starter::api::{Mode, Reason, Source as Confirmed, StickFault, WriteOrder, WriteStage};
use faraday_starter_shell::gpt;
use faraday_starter_shell::payload::{Image, Payload};
use faraday_starter_shell::writer::{self, Device, Identity, Job};

const XZ: &[u8] = include_bytes!("../../../../faraday-payload/tests/vectors/fixture.img.xz");
const XZ_SHA: &str = "d7e055e6f9f0d8a3c019d1e6b272b793dd607c3929a2a8f922cee3a5eae71c4f";
const IMG_SHA: &str = "c3a69d3c8e5599d5dc08dc6780f76b30a8e4d53082a85d40c6cd2effd28dfb0c";
const IMG_SIZE: usize = 39_936;
const NAME: &str = "faraday-stick-0.2.0-x86_64-uefi.img.xz";
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// The image inside `fixture.img.xz`, made again as its README says.
fn fixture_img() -> Vec<u8> {
    (0u64..1024)
        .flat_map(|i| {
            format!(
                "{i:05} {:08x} faraday payload fixture\n",
                (i * 2_654_435_761) % 4_294_967_296
            )
            .into_bytes()
        })
        .collect()
}

pub(crate) fn tmp(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("starter-writer-{name}"))
}

/// A disk that is a regular file, as confirmed or as it is now.
pub(crate) struct FileDisk {
    path: PathBuf,
    now: Identity,
    pub(crate) calls: Vec<&'static str>,
    /// Its blocks, when it is a raw disk that takes whole blocks only:
    /// then it is written straight to the disk too (`O_DIRECT`), which
    /// refuses an unaligned offset or length.
    raw: Option<u64>,
    /// What the disk is once it is open, when it is not what it was:
    /// another stick swapped in between the check and the open.
    after_open: Option<Identity>,
}

impl FileDisk {
    pub(crate) fn new(path: &Path) -> FileDisk {
        FileDisk {
            path: path.to_path_buf(),
            now: Identity {
                device: path.display().to_string(),
                serial: String::from("SERIAL1"),
                size: std::fs::metadata(path).expect("there").len(),
                in_use: None,
            },
            calls: Vec::new(),
            raw: None,
            after_open: None,
        }
    }

    /// The order a person confirmed for this disk as it was then.
    pub(crate) fn order(&self, source: Confirmed, mode: Mode) -> WriteOrder {
        WriteOrder {
            source,
            disk: String::from("/org/freedesktop/UDisks2/block_devices/sdx"),
            device: self.now.device.clone(),
            serial: self.now.serial.clone(),
            size: self.now.size,
            mode,
            boot_file: None,
        }
    }
}

impl Device for FileDisk {
    fn identity(&mut self) -> Result<Identity, Reason> {
        self.calls.push("identity");
        Ok(self.now.clone())
    }
    fn unmount(&mut self) -> Result<(), Reason> {
        self.calls.push("unmount");
        Ok(())
    }
    fn open_write(&mut self) -> Result<File, Reason> {
        self.calls.push("open_write");
        if let Some(now) = self.after_open.take() {
            self.now = now;
        }
        use std::os::unix::fs::OpenOptionsExt;
        const O_DIRECT: i32 = 0o40_000;
        OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(if self.raw.is_some() { O_DIRECT } else { 0 })
            .open(&self.path)
            .map_err(|e| Reason::OpenWrite(e.to_string()))
    }
    fn block(&mut self) -> u64 {
        self.raw.unwrap_or(512)
    }
    fn open_read(&mut self) -> Result<File, Reason> {
        self.calls.push("open_read");
        // As udisks2 hands it back: a read straight from the disk
        // (`O_DIRECT`), which refuses an unaligned offset, length or
        // buffer, as a real stick does.
        use std::os::unix::fs::OpenOptionsExt;
        const O_DIRECT: i32 = 0o40_000;
        OpenOptions::new()
            .read(true)
            .custom_flags(O_DIRECT)
            .open(&self.path)
            .map_err(|e| Reason::OpenRead(e.to_string()))
    }
    fn rescan(&mut self) -> Result<(), Reason> {
        self.calls.push("rescan");
        Ok(())
    }
}

fn manifest() -> Vec<u8> {
    format!(
        "# Faraday Starter payload 0.2.0, commit {COMMIT}\n{NAME}\t{XZ_SHA}\tfaraday-stick-0.2.0-x86_64-uefi.img\t{IMG_SHA}\t{IMG_SIZE}\t0.2.0+0123456789ab\n"
    )
    .into_bytes()
}

fn payload(xz: &[u8]) -> Payload {
    Payload::from_parts(&manifest(), vec![(NAME.to_string(), xz.to_vec())]).expect("a payload")
}

fn payload_source() -> Confirmed {
    Confirmed::Payload {
        name: NAME.to_string(),
        inner_sha256: IMG_SHA.to_string(),
    }
}

pub(crate) fn run(
    order: &WriteOrder,
    job: &Job,
    disk: &mut FileDisk,
) -> (Result<(), Reason>, Vec<WriteStage>) {
    let mut stages = Vec::new();
    let r = writer::write(order, job.source(), disk, &mut |stage, done, total| {
        assert!(done <= total);
        if stages.last() != Some(&stage) {
            stages.push(stage);
        }
    });
    (r, stages)
}

#[test]
fn a_whole_write_from_the_payload_reads_back_equal() {
    let path = tmp("whole");
    std::fs::write(&path, vec![0xEE; 64 << 10]).expect("a disk");
    let mut disk = FileDisk::new(&path);
    let order = disk.order(payload_source(), Mode::Whole);
    let job = payload(XZ).job(NAME, IMG_SHA).expect("carried");
    let (r, stages) = run(&order, &job, &mut disk);
    assert_eq!(r, Ok(()));
    assert_eq!(stages, [WriteStage::Writing, WriteStage::ReadingBack]);
    let after = std::fs::read(&path).expect("read");
    assert_eq!(&after[..IMG_SIZE], fixture_img().as_slice());
    assert!(after[IMG_SIZE..].iter().all(|&b| b == 0xEE));
    assert_eq!(
        disk.calls,
        [
            "identity",
            "unmount",
            "open_write",
            "identity",
            "open_read",
            "rescan"
        ]
    );
}

#[test]
fn a_payload_file_that_does_not_match_its_manifest_writes_nothing() {
    let path = tmp("bad-payload");
    std::fs::write(&path, vec![0xEE; 64 << 10]).expect("a disk");
    let mut disk = FileDisk::new(&path);
    let order = disk.order(payload_source(), Mode::Whole);
    let mut damaged = XZ.to_vec();
    damaged[200] ^= 1;
    let job = payload(&damaged).job(NAME, IMG_SHA).expect("carried");
    let (r, _) = run(&order, &job, &mut disk);
    assert_eq!(r, Err(Reason::PayloadSum));
    assert!(
        std::fs::read(&path)
            .expect("read")
            .iter()
            .all(|&b| b == 0xEE)
    );
    assert_eq!(disk.calls, ["identity"]);
}

#[test]
fn a_disk_whose_serial_or_size_changed_since_confirm_is_refused_before_any_byte() {
    let path = tmp("changed");
    std::fs::write(&path, vec![0xEE; 64 << 10]).expect("a disk");
    let job = payload(XZ).job(NAME, IMG_SHA).expect("carried");
    for change in ["serial", "size"] {
        let mut disk = FileDisk::new(&path);
        let order = disk.order(payload_source(), Mode::Whole);
        match change {
            "serial" => disk.now.serial = String::from("SERIAL2"),
            _ => disk.now.size += 512,
        }
        let (r, _) = run(&order, &job, &mut disk);
        assert!(matches!(r, Err(Reason::Changed { .. })), "{r:?}");
        assert_eq!(disk.calls, ["identity"]);
        assert!(
            std::fs::read(&path)
                .expect("read")
                .iter()
                .all(|&b| b == 0xEE)
        );
    }
}

#[test]
fn a_disk_that_is_another_once_opened_gets_no_byte() {
    let path = tmp("swapped");
    std::fs::write(&path, vec![0xEE; 64 << 10]).expect("a disk");
    let job = payload(XZ).job(NAME, IMG_SHA).expect("carried");
    for change in ["serial", "size", "device"] {
        let mut disk = FileDisk::new(&path);
        let order = disk.order(payload_source(), Mode::Whole);
        let mut other = disk.now.clone();
        match change {
            "serial" => other.serial = String::from("SERIAL2"),
            "size" => other.size += 512,
            _ => other.device.push('2'),
        }
        disk.after_open = Some(other);
        let (r, _) = run(&order, &job, &mut disk);
        assert!(matches!(r, Err(Reason::Changed { .. })), "{change}: {r:?}");
        assert_eq!(
            disk.calls,
            ["identity", "unmount", "open_write", "identity"],
            "{change}"
        );
        assert!(
            std::fs::read(&path)
                .expect("read")
                .iter()
                .all(|&b| b == 0xEE),
            "{change}"
        );
    }
}

#[test]
fn a_disk_that_reports_no_serial_is_written_as_any_other() {
    let path = tmp("no-serial");
    std::fs::write(&path, vec![0xEE; 64 << 10]).expect("a disk");
    let job = payload(XZ).job(NAME, IMG_SHA).expect("carried");
    let mut disk = FileDisk::new(&path);
    disk.now.serial = String::new();
    let order = disk.order(payload_source(), Mode::Whole);
    let (r, _) = run(&order, &job, &mut disk);
    assert_eq!(r, Ok(()));
    assert_eq!(
        &std::fs::read(&path).expect("read")[..IMG_SIZE],
        fixture_img().as_slice()
    );
}

// ---- keep data ------------------------------------------------------------

pub(crate) const SECTOR: u64 = 512;
/// A small stick: 8 MiB, FARADAYBOOT at 1 MiB for 2 MiB, FARADAYDATA after it.
pub(crate) const STICK: u64 = 8 << 20;
pub(crate) const BOOT_FIRST: u64 = 2048;
const BOOT_SECTORS: u64 = 4096;
pub(crate) const DATA_FIRST: u64 = BOOT_FIRST + BOOT_SECTORS;
const DATA_LAST: u64 = STICK / SECTOR - 34;

/// A GUID string as GPT stores it.
fn guid_bytes(s: &str) -> [u8; 16] {
    let hex: Vec<u8> = s
        .bytes()
        .filter(|&c| c != b'-')
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).expect("hex"), 16).expect("hex"))
        .collect();
    let mut b = [0u8; 16];
    b.copy_from_slice(&hex);
    b[..4].reverse();
    b[4..6].reverse();
    b[6..8].reverse();
    b
}

/// A stick image with Faraday's partition table (or another disk UUID),
/// its data partition filled with `data`.
pub(crate) fn stick_image(disk_uuid: &str, data: u8) -> Vec<u8> {
    stick_layout(
        [disk_uuid, gpt::BOOT_UUID, gpt::DATA_UUID],
        ["FARADAYBOOT", "FARADAYDATA"],
        data,
    )
}

/// A stick image with the given disk, boot and data partition UUIDs and
/// partition names, its data partition filled with `data`.
fn stick_layout(
    [disk_uuid, boot_uuid, data_uuid]: [&str; 3],
    names: [&str; 2],
    data: u8,
) -> Vec<u8> {
    let mut img = vec![0u8; STICK as usize];
    let mut entries = vec![0u8; 128 * 128];
    let mut entry = |i: usize, ty: &str, id: &str, first: u64, last: u64, name: &str| {
        let e = &mut entries[i * 128..(i + 1) * 128];
        e[..16].copy_from_slice(&guid_bytes(ty));
        e[16..32].copy_from_slice(&guid_bytes(id));
        e[32..40].copy_from_slice(&first.to_le_bytes());
        e[40..48].copy_from_slice(&last.to_le_bytes());
        for (k, u) in name.encode_utf16().enumerate() {
            e[56 + 2 * k..58 + 2 * k].copy_from_slice(&u.to_le_bytes());
        }
    };
    entry(
        0,
        gpt::ESP_TYPE,
        boot_uuid,
        BOOT_FIRST,
        DATA_FIRST - 1,
        names[0],
    );
    entry(
        1,
        "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7",
        data_uuid,
        DATA_FIRST,
        DATA_LAST,
        names[1],
    );
    let last = STICK / SECTOR - 1;
    let header = |current: u64, backup: u64, entries_lba: u64| {
        let mut h = vec![0u8; 92];
        h[..8].copy_from_slice(b"EFI PART");
        h[8..12].copy_from_slice(&0x0001_0000u32.to_le_bytes());
        h[12..16].copy_from_slice(&92u32.to_le_bytes());
        h[24..32].copy_from_slice(&current.to_le_bytes());
        h[32..40].copy_from_slice(&backup.to_le_bytes());
        h[40..48].copy_from_slice(&34u64.to_le_bytes());
        h[48..56].copy_from_slice(&(last - 33).to_le_bytes());
        h[56..72].copy_from_slice(&guid_bytes(disk_uuid));
        h[72..80].copy_from_slice(&entries_lba.to_le_bytes());
        h[80..84].copy_from_slice(&128u32.to_le_bytes());
        h[84..88].copy_from_slice(&128u32.to_le_bytes());
        h[88..92].copy_from_slice(&faraday_crypto::crc32(&entries).to_le_bytes());
        let crc = faraday_crypto::crc32(&h);
        h[16..20].copy_from_slice(&crc.to_le_bytes());
        h
    };
    let primary = header(1, last, 2);
    let backup = header(last, 1, last - 32);
    img[512..512 + 92].copy_from_slice(&primary);
    img[1024..1024 + entries.len()].copy_from_slice(&entries);
    let b = ((last - 32) * SECTOR) as usize;
    img[b..b + entries.len()].copy_from_slice(&entries);
    img[(last * SECTOR) as usize..(last * SECTOR) as usize + 92].copy_from_slice(&backup);
    let d = (DATA_FIRST * SECTOR) as usize;
    let e = ((DATA_LAST + 1) * SECTOR) as usize;
    img[d..e].fill(data);
    img
}

/// A boot partition of `len` bytes: a FAT32 boot sector labelled
/// FARADAYBOOT, then `fill`.
fn boot_partition(len: usize, fill: u8) -> Vec<u8> {
    let mut v = vec![fill; len];
    v[..512].fill(0);
    v[11..13].copy_from_slice(&512u16.to_le_bytes());
    v[13] = 8;
    v[66] = 0x29;
    v[71..82].copy_from_slice(b"FARADAYBOOT");
    v[82..90].copy_from_slice(b"FAT32   ");
    v[510] = 0x55;
    v[511] = 0xAA;
    v
}

fn image_file(name: &str, bytes: &[u8]) -> Image {
    let path = tmp(name);
    std::fs::write(&path, bytes).expect("written");
    Image::hash(&path).expect("hashed")
}

/// Every byte outside `[from, to)` is as it was.
pub(crate) fn untouched_outside(before: &[u8], after: &[u8], from: usize, to: usize) {
    assert_eq!(before.len(), after.len());
    assert_eq!(
        &before[..from],
        &after[..from],
        "bytes before the boot partition changed"
    );
    assert_eq!(
        &before[to..],
        &after[to..],
        "bytes after the boot partition changed"
    );
}

#[test]
fn keep_data_writes_only_the_boot_partition_and_leaves_every_byte_after_it() {
    let path = tmp("keep-boot-only");
    let before = stick_image(gpt::DISK_UUID, 0xDA);
    std::fs::write(&path, &before).expect("a stick");
    let image = image_file(
        "faraday-x86_64-uefi-dev-boot.vfat",
        &boot_partition(1536 << 10, 0xB0),
    );
    assert!(image.boot_only);
    let mut disk = FileDisk::new(&path);
    let order = disk.order(
        Confirmed::Image {
            path: image.path.display().to_string(),
            sha256: image.sum.sha256.to_string(),
        },
        Mode::KeepData,
    );
    let (r, _) = run(&order, &Job::Image(image.clone()), &mut disk);
    assert_eq!(r, Ok(()));
    let after = std::fs::read(&path).expect("read");
    let from = (BOOT_FIRST * SECTOR) as usize;
    let to = from + image.sum.size as usize;
    assert_eq!(
        &after[from..to],
        std::fs::read(&image.path).expect("read").as_slice()
    );
    untouched_outside(&before, &after, from, to);
}

#[test]
fn keep_data_from_a_whole_image_writes_only_its_boot_partition() {
    let path = tmp("keep-whole");
    let before = stick_image(gpt::DISK_UUID, 0xDA);
    std::fs::write(&path, &before).expect("a stick");
    // A new image: its boot partition new, its data partition empty.
    let mut new = stick_image(gpt::DISK_UUID, 0x00);
    let from = (BOOT_FIRST * SECTOR) as usize;
    let to = (DATA_FIRST * SECTOR) as usize;
    new[from..to].copy_from_slice(&boot_partition(to - from, 0xB1));
    let image = image_file("faraday-x86_64-uefi-dev.img", &new);
    let mut disk = FileDisk::new(&path);
    let order = disk.order(
        Confirmed::Image {
            path: image.path.display().to_string(),
            sha256: image.sum.sha256.to_string(),
        },
        Mode::KeepData,
    );
    let (r, _) = run(&order, &Job::Image(image), &mut disk);
    assert_eq!(r, Ok(()));
    let after = std::fs::read(&path).expect("read");
    assert_eq!(&after[from..to], &new[from..to]);
    // The vaults and settings on FARADAYDATA keep their bytes.
    untouched_outside(&before, &after, from, to);
}

#[test]
fn keep_data_over_a_disk_that_is_not_a_faraday_stick_writes_nothing() {
    let path = tmp("keep-not-faraday");
    let before = stick_image("11111111-2222-3333-4444-555555555555", 0xDA);
    std::fs::write(&path, &before).expect("a disk");
    let image = image_file("other-boot.vfat", &boot_partition(1 << 20, 0xB0));
    let mut disk = FileDisk::new(&path);
    let order = disk.order(
        Confirmed::Image {
            path: image.path.display().to_string(),
            sha256: image.sum.sha256.to_string(),
        },
        Mode::KeepData,
    );
    let (r, _) = run(&order, &Job::Image(image), &mut disk);
    assert!(
        matches!(
            r,
            Err(Reason::NotStick {
                fault: StickFault::DiskUuid,
                ..
            })
        ),
        "{r:?}"
    );
    assert_eq!(std::fs::read(&path).expect("read"), before);
}

#[test]
fn keep_data_over_a_stick_with_the_old_partition_table_writes_nothing() {
    // A stick made before the partitions were renamed: OSKBOOT and
    // OSKDATA, and the UUIDs of that layout.
    let old = [
        "4F70656E-5369-676E-6572-4B6974000000",
        "4F70656E-5369-676E-6572-4B6974000001",
        "4F70656E-5369-676E-6572-4B6974000002",
    ];
    let new = [gpt::DISK_UUID, gpt::BOOT_UUID, gpt::DATA_UUID];
    for (name, uuids, fault) in [
        ("keep-old-table", old, StickFault::DiskUuid),
        ("keep-old-names", new, StickFault::BootName),
    ] {
        let path = tmp(name);
        let before = stick_layout(uuids, ["OSKBOOT", "OSKDATA"], 0xDA);
        std::fs::write(&path, &before).expect("a stick");
        let image = image_file(
            &format!("{name}-boot.vfat"),
            &boot_partition(1536 << 10, 0xB0),
        );
        let mut disk = FileDisk::new(&path);
        let order = disk.order(
            Confirmed::Image {
                path: image.path.display().to_string(),
                sha256: image.sum.sha256.to_string(),
            },
            Mode::KeepData,
        );
        let (r, _) = run(&order, &Job::Image(image), &mut disk);
        assert!(
            matches!(&r, Err(Reason::NotStick { fault: f, .. }) if *f == fault),
            "{name}: {r:?}"
        );
        assert_eq!(std::fs::read(&path).expect("read"), before, "{name}");
    }
}

// ---- --image --------------------------------------------------------------

#[test]
fn image_gives_the_sha256sum_of_an_img_and_of_the_img_inside_an_xz() {
    let img = image_file("fixture.img", &fixture_img());
    assert_eq!(img.sum.sha256.to_string(), IMG_SHA);
    assert_eq!(img.sum.size, IMG_SIZE as u64);
    let xz = image_file("fixture.img.xz", XZ);
    assert_eq!(xz.sum.sha256.to_string(), IMG_SHA);
    assert_eq!(xz.sum.size, IMG_SIZE as u64);
    assert!(!img.boot_only && !xz.boot_only);
}

#[test]
fn a_developer_image_written_whole_reads_back_equal() {
    let path = tmp("whole-image");
    std::fs::write(&path, vec![0xEE; 64 << 10]).expect("a disk");
    let image = image_file("whole.img.xz", XZ);
    let mut disk = FileDisk::new(&path);
    let order = disk.order(
        Confirmed::Image {
            path: image.path.display().to_string(),
            sha256: image.sum.sha256.to_string(),
        },
        Mode::Whole,
    );
    let (r, _) = run(&order, &Job::Image(image), &mut disk);
    assert_eq!(r, Ok(()));
    assert_eq!(
        &std::fs::read(&path).expect("read")[..IMG_SIZE],
        fixture_img().as_slice()
    );
}

// ---- what a refusal reads as ------------------------------------------------

#[test]
fn a_refused_write_reads_as_its_string_from_the_strings_file() {
    use faraday_core::strings::EN;
    use faraday_core::strings::fill;
    let path = tmp("reads-as");
    std::fs::write(&path, vec![0xEE; 64 << 10]).expect("a disk");
    let job = payload(XZ).job(NAME, IMG_SHA).expect("carried");
    let mut disk = FileDisk::new(&path);
    let order = disk.order(payload_source(), Mode::Whole);
    disk.now.serial = String::from("SERIAL2");
    let size = disk.now.size.to_string();
    let (r, _) = run(&order, &job, &mut disk);
    let reason = r.expect_err("refused");
    assert_eq!(
        faraday_starter::reason_text(&reason),
        fill(
            EN.starter_why_changed,
            &[
                &disk.now.device,
                "SERIAL2",
                &fill(EN.starter_why_bytes, &[&size])
            ]
        )
    );
}

/// A disk that answers only reads a stick opened straight (`O_DIRECT`)
/// answers: an offset, a length and a buffer address all on a 4096-byte
/// boundary. Anything else is refused, as the kernel refuses it (EINVAL),
/// whatever the filesystem these tests run on allows.
struct StrictDisk(std::io::Cursor<Vec<u8>>);

impl std::io::Read for StrictDisk {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let pos = self.0.position();
        if !pos.is_multiple_of(4096)
            || !buf.len().is_multiple_of(4096)
            || !(buf.as_ptr() as usize).is_multiple_of(4096)
        {
            return Err(std::io::Error::from_raw_os_error(22));
        }
        self.0.read(buf)
    }
}

impl std::io::Seek for StrictDisk {
    fn seek(&mut self, to: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(to)
    }
}

#[test]
fn keep_data_checks_the_partition_table_with_reads_a_stick_accepts() {
    let image = stick_image(gpt::DISK_UUID, 0xDA);
    let table = gpt::read(
        &mut |at, buf| {
            buf.copy_from_slice(&image[at as usize..at as usize + buf.len()]);
            Ok(())
        },
        image.len() as u64,
    )
    .expect("a Faraday stick's table");
    let mut disk = StrictDisk(std::io::Cursor::new(image.clone()));
    let read = writer::regions(&mut disk, &table).expect("read the way a stick allows");
    let expected: Vec<Vec<u8>> = table
        .regions
        .iter()
        .map(|&(from, to)| image[from as usize..to as usize].to_vec())
        .collect();
    assert_eq!(read, expected);
}

/// `image` with the primary GPT header's eight-byte field at `at` set to
/// `value` and the header's checksum written again.
fn with_header_field(mut image: Vec<u8>, at: usize, value: u64) -> Vec<u8> {
    image[512 + at..512 + at + 8].copy_from_slice(&value.to_le_bytes());
    image[512 + 16..512 + 20].fill(0);
    let crc = faraday_crypto::crc32(&image[512..512 + 92]);
    image[512 + 16..512 + 20].copy_from_slice(&crc.to_le_bytes());
    image
}

/// The table of `image`, read as a stick is: a read past its end fails.
fn read_table(image: &[u8]) -> Result<gpt::Table, StickFault> {
    gpt::read(
        &mut |at, buf| {
            let from = usize::try_from(at)
                .ok()
                .filter(|&a| {
                    a.checked_add(buf.len())
                        .is_some_and(|end| end <= image.len())
                })
                .ok_or_else(|| std::io::Error::other("past the end"))?;
            buf.copy_from_slice(&image[from..from + buf.len()]);
            Ok(())
        },
        image.len() as u64,
    )
}

/// A header whose backup or entries sit at a sector no disk can address
/// is refused rather than wrapped round to a small offset; one at the
/// last sector a disk can address is read.
#[test]
fn a_partition_table_whose_sectors_run_past_what_a_disk_addresses_is_refused() {
    let past = u64::MAX / SECTOR;
    // The backup header at the last addressable sector: past this stick's
    // end, so no backup region, but the table reads.
    let image = with_header_field(stick_image(gpt::DISK_UUID, 0xDA), 32, past - 1);
    let table = read_table(&image).expect("a header whose backup is past the end reads");
    assert_eq!(table.regions.len(), 1);
    assert!(gpt::faraday_boot(&table, 0).is_ok());
    // One sector further, its end is past what 64 bits hold.
    let image = with_header_field(stick_image(gpt::DISK_UUID, 0xDA), 32, past);
    assert!(read_table(&image).is_err());
    // The entries the same way.
    for lba in [past + 1, u64::MAX] {
        let image = with_header_field(stick_image(gpt::DISK_UUID, 0xDA), 72, lba);
        assert!(read_table(&image).is_err());
    }
}

/// A boot partition whose sectors run past what a disk addresses is
/// damaged; one that ends at the last addressable byte is read.
#[test]
fn a_boot_partition_past_what_a_disk_addresses_is_damaged() {
    let image = stick_image(gpt::DISK_UUID, 0xDA);
    let mut table = read_table(&image).expect("a Faraday stick's table");
    let past = u64::MAX / SECTOR;
    table.parts[0].first = past - 10;
    table.parts[0].last = past - 1;
    table.parts[1].first = past;
    assert_eq!(
        gpt::faraday_boot(&table, 0),
        Ok(gpt::Boot {
            offset: (past - 10) * SECTOR,
            len: 10 * SECTOR,
        })
    );
    table.parts[0].first = past + 1;
    table.parts[0].last = past + 10;
    table.parts[1].first = past + 11;
    assert_eq!(gpt::faraday_boot(&table, 0), Err(StickFault::BootDamaged));
    table.parts[0].first = 0;
    table.parts[0].last = u64::MAX;
    assert_eq!(gpt::faraday_boot(&table, 0), Err(StickFault::BootDamaged));
}

#[test]
fn an_image_that_ends_inside_a_block_is_written_to_a_disk_that_takes_whole_blocks_only() {
    // A developer's image 100 bytes past a whole number of sectors, to a
    // disk written straight to the disk, as macOS's raw disk is: the
    // write succeeds, the image is on the disk, and the rest of the last
    // block keeps what the disk held there.
    let path = tmp("raw-blocks");
    let before = vec![0xA5u8; 64 << 10];
    std::fs::write(&path, &before).expect("a disk");
    let mut bytes = fixture_img();
    bytes.truncate(IMG_SIZE - 512 + 100);
    let image = image_file("raw-blocks.img", &bytes);
    let mut disk = FileDisk::new(&path);
    disk.raw = Some(4096);
    let order = disk.order(
        Confirmed::Image {
            path: image.path.display().to_string(),
            sha256: image.sum.sha256.to_string(),
        },
        Mode::Whole,
    );
    let (r, stages) = run(&order, &Job::Image(image), &mut disk);
    assert_eq!(r, Ok(()));
    assert_eq!(stages, [WriteStage::Writing, WriteStage::ReadingBack]);
    let after = std::fs::read(&path).expect("read");
    assert_eq!(&after[..bytes.len()], bytes.as_slice());
    untouched_outside(&before, &after, 0, bytes.len());
}

// ---- which Faraday a stick holds, read by the elevated copy (#21) ---------

/// A Faraday stick whose boot partition holds `release` as its kernel
/// does, a string among other bytes, near the partition's end.
fn stick_holding(release: &[u8]) -> Vec<u8> {
    let mut image = stick_image(gpt::DISK_UUID, 0xDA);
    let from = (BOOT_FIRST * SECTOR) as usize;
    let to = (DATA_FIRST * SECTOR) as usize;
    let mut boot = boot_partition(to - from, 0x00);
    let at = boot.len() - 1003;
    boot[at..at + release.len()].copy_from_slice(release);
    image[from..to].copy_from_slice(&boot);
    image
}

#[test]
fn the_elevated_copy_reads_which_faraday_a_stick_holds_from_its_boot_partition() {
    let read = |image: Vec<u8>| {
        let len = image.len() as u64;
        writer::release_on_stick(&mut StrictDisk(std::io::Cursor::new(image)), len, "Disk 2")
    };
    assert_eq!(
        read(stick_holding(
            b"\x006.6.84-faraday-0.2.0+0123456789ab.test (buildroot@buildroot) #1 SMP\x00"
        )),
        Ok(Some(String::from("6.6.84-faraday-0.2.0+0123456789ab.test")))
    );
    // A stick made before Faraday put its version there.
    assert_eq!(read(stick_image(gpt::DISK_UUID, 0xDA)), Ok(None));
    // The data partition is not where the version is looked for.
    let mut image = stick_image(gpt::DISK_UUID, 0xDA);
    let d = (DATA_FIRST * SECTOR) as usize + 4096;
    image[d..d + 22].copy_from_slice(b"\x006.6.84-faraday-0.9.0\x00");
    assert_eq!(read(image), Ok(None));
    // A disk that is not a Faraday stick says why.
    assert!(matches!(
        read(stick_image("11111111-2222-3333-4444-555555555555", 0xDA)),
        Err(Reason::NotStick {
            fault: StickFault::DiskUuid,
            ..
        })
    ));
}

#[test]
fn the_elevated_writer_refuses_keep_data_of_the_build_the_stick_already_holds() {
    use faraday_core::strings::EN;
    let on = Some("6.6.84-faraday-0.2.0+0123456789ab");
    assert_eq!(
        writer::refuse_same_build(on, Some("0.2.0+0123456789ab")),
        Err(Reason::SameBuild)
    );
    assert_eq!(
        faraday_starter::reason_text(&Reason::SameBuild),
        EN.starter_why_same_build
    );
    assert_eq!(writer::refuse_same_build(on, Some("0.3.0")), Ok(()));
    // An older Faraday over a newer one: Confirm warned, and it is written.
    assert_eq!(
        writer::refuse_same_build(Some("6.6.84-faraday-0.3.0"), Some("0.2.0")),
        Ok(())
    );
    // A stick made before versions, and an image with no build ID.
    assert_eq!(writer::refuse_same_build(None, Some("0.2.0")), Ok(()));
    assert_eq!(writer::refuse_same_build(on, None), Ok(()));
}

/// udisks2 answering from a recording, remembering what the writer asked.
#[cfg(target_os = "linux")]
struct Recording {
    objects: faraday_starter_shell::linux::udisks::Objects,
    calls: Vec<String>,
}

#[cfg(target_os = "linux")]
impl faraday_starter_shell::linux::Udisks for Recording {
    fn objects(&mut self) -> Result<faraday_starter_shell::linux::udisks::Objects, String> {
        Ok(self.objects.clone())
    }
    fn wait(&mut self, _: std::time::Duration) -> Result<bool, String> {
        Ok(false)
    }
    fn mount_ro(&mut self, object: &str) -> Result<PathBuf, String> {
        self.calls.push(format!("mount {object}"));
        Err(String::from("no"))
    }
    fn unmount(&mut self, object: &str) -> Result<(), String> {
        self.calls.push(format!("unmount {object}"));
        Ok(())
    }
    fn open(
        &mut self,
        object: &str,
        _: faraday_starter_shell::linux::Open,
    ) -> Result<File, String> {
        self.calls.push(format!("open {object}"));
        Err(String::from("no"))
    }
    fn rescan(&mut self, object: &str) -> Result<(), String> {
        self.calls.push(format!("rescan {object}"));
        Ok(())
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_disk_the_computer_uses_but_not_through_a_desktop_mount_is_refused_with_nothing_unmounted_or_written()
 {
    use faraday_starter::api::InUse;
    use faraday_starter_shell::linux::Target;
    use faraday_starter_shell::linux::udisks::Objects;
    let recorded = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/udisks/in-use.txt"),
    )
    .expect("a recorded answer");
    let objects = Objects::parse(&recorded).expect("a well-formed recording");
    let job = payload(XZ).job(NAME, IMG_SHA).expect("carried");
    let cases = [
        (
            "sdh",
            2_000_398_934_016,
            "JJJ101",
            InUse::Mounted("/mnt/backup".into()),
        ),
        ("sdi", 1_000_204_886_016, "KKK202", InUse::Held),
        ("sdj", 4_000_787_030_016, "LLL303", InUse::Held),
        ("sdk", 16_008_609_792, "MMM404", InUse::Unchecked),
    ];
    for (name, size, serial, why) in cases {
        let block = format!("/org/freedesktop/UDisks2/block_devices/{name}");
        let order = WriteOrder {
            source: payload_source(),
            disk: block.clone(),
            device: format!("/dev/{name}"),
            serial: serial.into(),
            size,
            mode: Mode::Whole,
            boot_file: None,
        };
        let mut target = Target {
            bus: Recording {
                objects: objects.clone(),
                calls: Vec::new(),
            },
            block,
        };
        let r = writer::write(&order, job.source(), &mut target, &mut |_, _, _| {});
        assert_eq!(r, Err(Reason::InUse(why)), "{name}");
        assert_eq!(target.bus.calls, Vec::<String>::new(), "{name}");
    }
}
