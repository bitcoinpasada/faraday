//! The writer against regular files holding images, as
//! `faraday/image/stick-boot.sh` allows: what a person sees is what is
//! on the stick afterwards, and nothing when a check refuses.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use faraday_starter::api::{Mode, Reason, Source as Confirmed, StickFault, WriteOrder, WriteStage};
use faraday_starter_shell::gpt;
use faraday_starter_shell::payload::{Image, Payload};
use faraday_starter_shell::writer::{self, Device, Identity, Job};

use crate::sticks::{
    self, BASIC, BOOT_FIRST, DATA_FIRST, DATA_SECTORS, ESP, FILES, IDS, IMAGE, NAMES, SECTOR,
    SERIALS, STICK, Sparse, boot_range, read, stick, stick_image, stick_with, unchanged,
    untouched_outside,
};

const XZ: &[u8] = include_bytes!("../../../../faraday-payload/tests/vectors/fixture.img.xz");
const XZ_SHA: &str = "d7e055e6f9f0d8a3c019d1e6b272b793dd607c3929a2a8f922cee3a5eae71c4f";
const IMG_SHA: &str = "c3a69d3c8e5599d5dc08dc6780f76b30a8e4d53082a85d40c6cd2effd28dfb0c";
const IMG_SIZE: usize = 39_936;
const NAME: &str = "faraday-stick-0.2.0-x86_64-uefi-strict.img.xz";
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
            name: None,
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
        "# Faraday Starter payload 0.2.0, commit {COMMIT}\n{NAME}\t{XZ_SHA}\tfaraday-stick-0.2.0-x86_64-uefi-strict.img\t{IMG_SHA}\t{IMG_SIZE}\t0.2.0+0123456789ab\n"
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

/// The Starter's payload carrying `image` as a release's stick image,
/// and the order's source for it.
pub(crate) fn release_payload(image: &Sparse) -> (Job, Confirmed) {
    let img = image.to_vec();
    let xz = sticks::stored_xz(&img);
    let img_sha = sticks::sha(&img);
    drop(img);
    let manifest = format!(
        "# Faraday Starter payload 0.2.0, commit {COMMIT}\n{NAME}\t{}\tfaraday-stick-0.2.0-x86_64-uefi-strict.img\t{img_sha}\t{}\t0.2.0+0123456789ab\n",
        sticks::sha(&xz),
        image.len,
    );
    let payload =
        Payload::from_parts(manifest.as_bytes(), vec![(NAME.to_string(), xz)]).expect("a payload");
    let job = payload.job(NAME, &img_sha).expect("carried");
    let source = Confirmed::Payload {
        name: NAME.to_string(),
        inner_sha256: img_sha,
    };
    (job, source)
}

/// `image` as a developer's image file, hashed as `--image` hashes it,
/// and the order's source for it.
pub(crate) fn image_job(name: &str, image: &Sparse) -> (Image, Confirmed) {
    let path = tmp(name);
    image.write(&path);
    let image = Image::hash(&path).expect("hashed");
    let source = Confirmed::Image {
        path: image.path.display().to_string(),
        sha256: image.sum.sha256.to_string(),
    };
    (image, source)
}

/// A disk of `len` bytes at `name`, holding `disk`.
pub(crate) fn disk_file(name: &str, disk: &Sparse) -> PathBuf {
    let path = tmp(name);
    disk.write(&path);
    path
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

/// A blank disk of [`STICK`] bytes, with a mark past where the image
/// ends that a whole write must leave.
fn blank(name: &str) -> (PathBuf, Sparse) {
    let mut d = Sparse::new(STICK);
    d.put(IMAGE + 4096, &[0xEE; 4096]);
    (disk_file(name, &d), d)
}

/// What a whole write left on the disk at `path`: the image's length of
/// it, with the image's own IDs put back.
fn written_with_ids_back(path: &Path, image: &Sparse) -> Vec<u8> {
    let mut back = read(path, 0, image.len as usize);
    sticks::ids_put_back(&mut back, image, &sticks::stick_id_places(image.len));
    back
}

#[test]
fn a_whole_write_from_the_payload_reads_back_as_the_image_with_the_sticks_own_ids() {
    let image = stick_image();
    let (job, source) = release_payload(&image);
    let (path, before) = blank("whole");
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::Whole);
    let (r, stages) = run(&order, &job, &mut disk);
    assert_eq!(r, Ok(()));
    assert_eq!(stages, [WriteStage::Writing, WriteStage::ReadingBack]);
    // With the image's IDs put back, every byte is the image's.
    assert!(written_with_ids_back(&path, &image) == image.to_vec());
    untouched_outside(&before, &path, 0, IMAGE);
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
    let (image, source) = image_job("swapped.img", &stick_image());
    let job = Job::Image(image);
    for change in ["serial", "size", "device"] {
        let (path, before) = blank(&format!("swapped-{change}"));
        let mut disk = FileDisk::new(&path);
        let order = disk.order(source.clone(), Mode::Whole);
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
        unchanged(&before, &path);
    }
}

#[test]
fn a_disk_that_reports_no_serial_is_written_as_any_other() {
    let image = stick_image();
    let (img, source) = image_job("no-serial.img", &image);
    let (path, _) = blank("no-serial");
    let mut disk = FileDisk::new(&path);
    disk.now.serial = String::new();
    let order = disk.order(source, Mode::Whole);
    let (r, _) = run(&order, &Job::Image(img), &mut disk);
    assert_eq!(r, Ok(()));
    assert!(written_with_ids_back(&path, &image) == image.to_vec());
}

#[test]
fn an_image_with_no_place_for_a_sticks_own_ids_is_not_written_whole() {
    // The payload fixture is text: no partition table at all.
    let path = tmp("no-table");
    std::fs::write(&path, vec![0xEE; 64 << 10]).expect("a disk");
    let job = payload(XZ).job(NAME, IMG_SHA).expect("carried");
    let mut disk = FileDisk::new(&path);
    let order = disk.order(payload_source(), Mode::Whole);
    let (r, _) = run(&order, &job, &mut disk);
    assert!(matches!(r, Err(Reason::NotStickImage(_))), "{r:?}");
    assert_eq!(disk.calls, ["identity"]);
    // A stick image whose data partition is elsewhere.
    let mut moved = Sparse::new(IMAGE);
    sticks::gpt(
        &mut moved,
        IDS[0],
        &[
            (ESP, IDS[1], BOOT_FIRST, DATA_FIRST - 1, NAMES[0]),
            (
                BASIC,
                IDS[2],
                DATA_FIRST + 2048,
                DATA_FIRST + DATA_SECTORS - 1,
                NAMES[1],
            ),
        ],
    );
    let (img, source) = image_job("moved.img", &moved);
    let (path, before) = blank("moved");
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::Whole);
    let (r, _) = run(&order, &Job::Image(img), &mut disk);
    assert_eq!(r, Err(Reason::NotStickImage(StickFault::DataPlace)));
    unchanged(&before, &path);
}

// ---- keep data ------------------------------------------------------------

/// A boot partition image of `len` bytes, as `-boot.vfat` is: a FAT32
/// boot sector labelled EFI with its backup, with `serial`, then `fill`.
fn boot_partition(len: usize, fill: u8, serial: u32) -> Sparse {
    let mut s = Sparse::new(len as u64);
    s.put(0, &vec![fill; len]);
    sticks::fat(&mut s, 0, true, (len / 512) as u32, b"EFI        ", serial);
    s
}

/// `bytes` with the FAT32 serial at its boot sector and backup set to
/// `serial`: what keep data writes.
fn with_serial(mut bytes: Vec<u8>, serial: u32) -> Vec<u8> {
    for at in [67, 6 * 512 + 67] {
        bytes[at..at + 4].copy_from_slice(&serial.to_le_bytes());
    }
    bytes
}

#[test]
fn keep_data_writes_only_the_boot_partition_keeping_the_sticks_own_ids() {
    let before = stick();
    let path = disk_file("keep-boot-only", &before);
    let boot = boot_partition(1536 << 10, 0xB0, 0x0B0B_0B0B);
    let (image, source) = image_job("faraday-x86_64-uefi-strict-dev-boot.vfat", &boot);
    assert!(image.boot_only);
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::KeepData);
    let (r, _) = run(&order, &Job::Image(image.clone()), &mut disk);
    assert_eq!(r, Ok(()));
    let (from, _) = boot_range();
    let to = from + image.sum.size;
    // The boot partition's serial is the one the stick had.
    assert!(read(&path, from, image.sum.size as usize) == with_serial(boot.to_vec(), SERIALS[0]));
    // Its GUIDs and the data partition are as they were.
    untouched_outside(&before, &path, from, to);
}

#[test]
fn keep_data_from_a_whole_image_writes_only_its_boot_partition() {
    let before = stick();
    let path = disk_file("keep-whole", &before);
    // A new image: its own IDs and boot serial, its boot partition new,
    // its data partition empty.
    let mut new = stick_with(IMAGE, IDS, NAMES, 0);
    let (from, to) = boot_range();
    let boot = boot_partition((to - from) as usize, 0xB1, 0x0B0B_0B0B);
    new.put(from, &boot.to_vec());
    let (image, source) = image_job("faraday-x86_64-uefi-strict-dev.img", &new);
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::KeepData);
    let (r, _) = run(&order, &Job::Image(image), &mut disk);
    assert_eq!(r, Ok(()));
    assert!(read(&path, from, (to - from) as usize) == with_serial(boot.to_vec(), SERIALS[0]));
    // The vaults and settings on the data partition keep their bytes.
    untouched_outside(&before, &path, from, to);
}

#[test]
fn keep_data_from_a_whole_image_counts_only_its_boot_partition_as_written() {
    let path = disk_file("keep-whole-progress", &stick());
    let mut new = stick_image();
    let (from, to) = boot_range();
    new.put(
        from,
        &boot_partition((to - from) as usize, 0xB2, 1).to_vec(),
    );
    let (image, source) = image_job("faraday-x86_64-uefi-dev-progress.img", &new);
    assert!(image.sum.size > to - from);
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::KeepData);
    let mut seen = Vec::new();
    let r = writer::write(
        &order,
        Job::Image(image).source(),
        &mut disk,
        &mut |stage, done, total| {
            seen.push((stage, done, total));
        },
    );
    assert_eq!(r, Ok(()));
    let boot = to - from;
    // Writing and reading back each count the boot partition, to its
    // end, and never the image's data partition.
    for stage in [WriteStage::Writing, WriteStage::ReadingBack] {
        let of: Vec<_> = seen.iter().filter(|s| s.0 == stage).collect();
        assert!(
            of.iter()
                .all(|&&(_, done, total)| total == boot && done <= total)
        );
        assert_eq!(of.last().map(|s| s.1), Some(boot));
    }
}

/// A disk with an EFI system partition and a basic data partition
/// placed as another tool places them, or of other types.
fn other_layout(boot_first: u64, data_first: u64, data_type: &str) -> Sparse {
    let mut s = Sparse::new(STICK);
    sticks::gpt(
        &mut s,
        IDS[0],
        &[
            (ESP, IDS[1], boot_first, data_first - 1, NAMES[0]),
            (
                data_type,
                IDS[2],
                data_first,
                data_first + DATA_SECTORS - 1,
                NAMES[1],
            ),
        ],
    );
    s.put(data_first * SECTOR + FILES[0], &[0xDA; 4096]);
    s
}

/// A stick with Faraday's boot partition and its data partition 1 MiB
/// further on.
fn data_moved() -> Sparse {
    let mut s = Sparse::new(STICK);
    let first = DATA_FIRST + 2048;
    sticks::gpt(
        &mut s,
        IDS[0],
        &[
            (ESP, IDS[1], BOOT_FIRST, DATA_FIRST - 1, NAMES[0]),
            (BASIC, IDS[2], first, first + DATA_SECTORS - 1, NAMES[1]),
        ],
    );
    s
}

#[test]
fn keep_data_over_a_disk_of_another_layout_writes_nothing() {
    let linux = "0FC63DAF-8483-4772-8E79-3D69D8477DE4";
    for (name, before, fault) in [
        (
            "keep-at-2048",
            other_layout(2048, DATA_FIRST + 2048, BASIC),
            StickFault::BootPlace,
        ),
        ("keep-data-moved", data_moved(), StickFault::DataPlace),
        (
            "keep-linux-data",
            other_layout(BOOT_FIRST, DATA_FIRST, linux),
            StickFault::DataType,
        ),
    ] {
        let path = disk_file(name, &before);
        let (image, source) = image_job(
            &format!("{name}-boot.vfat"),
            &boot_partition(1536 << 10, 0xB0, 1),
        );
        let mut disk = FileDisk::new(&path);
        let order = disk.order(source, Mode::KeepData);
        let (r, _) = run(&order, &Job::Image(image), &mut disk);
        assert!(
            matches!(&r, Err(Reason::NotStick { fault: f, .. }) if *f == fault),
            "{name}: {r:?}"
        );
        unchanged(&before, &path);
    }
}

#[test]
fn keep_data_over_a_stick_made_before_with_faradays_old_names_and_guids_is_written() {
    // F67's GUIDs and names (docs/STICK-IDENTITY.md §5.4): the same
    // layout, so the same stick.
    let before = stick_with(
        STICK,
        [
            "46617261-6461-792D-5374-69636B000000",
            "46617261-6461-792D-5374-69636B000001",
            "46617261-6461-792D-5374-69636B000002",
        ],
        ["FARADAYBOOT", "FARADAYDATA"],
        0xDA,
    );
    let path = disk_file("keep-made-before", &before);
    let (image, source) = image_job(
        "made-before-boot.vfat",
        &boot_partition(1536 << 10, 0xB0, 1),
    );
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::KeepData);
    let (r, _) = run(&order, &Job::Image(image), &mut disk);
    assert_eq!(r, Ok(()));
    let (from, to) = boot_range();
    untouched_outside(&before, &path, from, to);
}

// ---- --image --------------------------------------------------------------

fn image_file(name: &str, bytes: &[u8]) -> Image {
    let path = tmp(name);
    std::fs::write(&path, bytes).expect("written");
    Image::hash(&path).expect("hashed")
}

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
fn a_developer_image_written_whole_reads_back_as_it_with_the_sticks_own_ids() {
    let image = stick_image();
    let (img, source) = image_job("whole.img", &image);
    let (path, before) = blank("whole-image");
    let mut disk = FileDisk::new(&path);
    let order = disk.order(source, Mode::Whole);
    let (r, _) = run(&order, &Job::Image(img), &mut disk);
    assert_eq!(r, Ok(()));
    assert!(written_with_ids_back(&path, &image) == image.to_vec());
    untouched_outside(&before, &path, 0, IMAGE);
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
struct StrictDisk {
    disk: Sparse,
    pos: u64,
}

impl StrictDisk {
    fn new(disk: Sparse) -> StrictDisk {
        StrictDisk { disk, pos: 0 }
    }
}

impl std::io::Read for StrictDisk {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if !self.pos.is_multiple_of(4096)
            || !buf.len().is_multiple_of(4096)
            || !(buf.as_ptr() as usize).is_multiple_of(4096)
        {
            return Err(std::io::Error::from_raw_os_error(22));
        }
        let n = (buf.len() as u64).min(self.disk.len.saturating_sub(self.pos)) as usize;
        self.disk.read_at(self.pos, &mut buf[..n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl std::io::Seek for StrictDisk {
    fn seek(&mut self, to: std::io::SeekFrom) -> std::io::Result<u64> {
        self.pos = match to {
            std::io::SeekFrom::Start(n) => n,
            std::io::SeekFrom::End(d) => self.disk.len.saturating_add_signed(d),
            std::io::SeekFrom::Current(d) => self.pos.saturating_add_signed(d),
        };
        Ok(self.pos)
    }
}

#[test]
fn keep_data_checks_the_partition_table_with_reads_a_stick_accepts() {
    let image = stick();
    let table = read_table(&image).expect("a Faraday stick's table");
    let mut disk = StrictDisk::new(image.clone());
    let read = writer::regions(&mut disk, &table).expect("read the way a stick allows");
    let expected: Vec<Vec<u8>> = table
        .regions
        .iter()
        .map(|&(from, to)| image.bytes(from, (to - from) as usize))
        .collect();
    assert_eq!(read, expected);
}

/// `image` with the primary GPT header's eight-byte field at `at` set to
/// `value` and the header's checksum written again.
fn with_header_field(mut image: Sparse, at: usize, value: u64) -> Sparse {
    let mut h = image.bytes(512, 92);
    h[at..at + 8].copy_from_slice(&value.to_le_bytes());
    h[16..20].fill(0);
    let crc = faraday_crypto::crc32(&h);
    h[16..20].copy_from_slice(&crc.to_le_bytes());
    image.put(512, &h);
    image
}

/// The table of `image`, read as a stick is: a read past its end fails.
fn read_table(image: &Sparse) -> Result<gpt::Table, StickFault> {
    gpt::read(
        &mut |at, buf| {
            if at
                .checked_add(buf.len() as u64)
                .is_none_or(|end| end > image.len)
            {
                return Err(std::io::Error::other("past the end"));
            }
            image.read_at(at, buf);
            Ok(())
        },
        image.len,
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
    let image = with_header_field(stick(), 32, past - 1);
    let table = read_table(&image).expect("a header whose backup is past the end reads");
    assert_eq!(table.regions.len(), 1);
    assert!(gpt::faraday_boot(&table, 0).is_ok());
    // One sector further, its end is past what 64 bits hold.
    let image = with_header_field(stick(), 32, past);
    assert!(read_table(&image).is_err());
    // The entries the same way.
    for lba in [past + 1, u64::MAX] {
        let image = with_header_field(stick(), 72, lba);
        assert!(read_table(&image).is_err());
    }
}

/// A boot partition whose sectors run past what a disk addresses is
/// damaged, not wrapped round; one that ends at the last addressable
/// byte is read, and is not where a Faraday stick's is.
#[test]
fn a_boot_partition_past_what_a_disk_addresses_is_damaged() {
    let mut table = read_table(&stick()).expect("a Faraday stick's table");
    let past = u64::MAX / SECTOR;
    table.parts[0].first = past - 10;
    table.parts[0].last = past - 1;
    table.parts[1].first = past;
    table.parts[1].last = past;
    assert_eq!(gpt::faraday_boot(&table, 0), Err(StickFault::BootPlace));
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
    let len = IMAGE + 100;
    let mut image = stick_with(len, IDS, NAMES, 0);
    image.put(IMAGE, &[0x5C; 100]);
    let (img, source) = image_job("raw-blocks.img", &image);
    let mut before = Sparse::new(STICK);
    before.put(IMAGE - 8192, &[0xA5; 16384]);
    let path = disk_file("raw-blocks", &before);
    let mut disk = FileDisk::new(&path);
    disk.raw = Some(4096);
    let order = disk.order(source, Mode::Whole);
    let (r, stages) = run(&order, &Job::Image(img), &mut disk);
    assert_eq!(r, Ok(()));
    assert_eq!(stages, [WriteStage::Writing, WriteStage::ReadingBack]);
    assert!(written_with_ids_back(&path, &image) == image.to_vec());
    untouched_outside(&before, &path, 0, len);
}

// ---- which Faraday a stick holds, read by the elevated copy (#21) ---------

/// A Faraday stick whose boot partition holds `release` as its kernel
/// does, a string among other bytes, near the partition's end.
fn stick_holding(release: &[u8]) -> Sparse {
    let mut image = stick();
    let (_, to) = boot_range();
    image.put(to - 1003, release);
    image
}

#[test]
fn the_elevated_copy_reads_which_faraday_a_stick_holds_from_its_boot_partition() {
    let read = |image: Sparse| {
        let len = image.len;
        writer::release_on_stick(&mut StrictDisk::new(image), len, "Disk 2")
    };
    assert_eq!(
        read(stick_holding(
            b"\x006.6.84-faraday-0.2.0+0123456789ab.test (buildroot@buildroot) #1 SMP\x00"
        )),
        Ok(Some(String::from("6.6.84-faraday-0.2.0+0123456789ab.test")))
    );
    // A stick made before Faraday put its version there.
    assert_eq!(read(stick()), Ok(None));
    // The data partition is not where the version is looked for.
    let mut image = stick();
    image.put(DATA_FIRST * SECTOR + 8192, b"\x006.6.84-faraday-0.9.0\x00");
    assert_eq!(read(image), Ok(None));
    // A disk that is not a Faraday stick says why.
    assert!(matches!(
        read(data_moved()),
        Err(Reason::NotStick {
            fault: StickFault::DataPlace,
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
            name: None,
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
