//! The writer (`docs/STARTER.md` §4.1), the same on every operating system:
//! what it is given to write ([`Source`]) and the disk it writes
//! ([`Device`], which each system's module opens its own way).
//!
//! In order, and nothing written until every check before the write has
//! passed:
//!
//! 1. The disk is still the one confirmed: the same device, serial and
//!    size, and the computer does not use it.
//! 2. What is written fits it; a payload file matches its manifest, and
//!    a developer's image still has the SHA-256 Confirm showed. A boot
//!    file the person signed is still the file chosen, and the release's
//!    own program signed (`crate::signed`, §4.5). A whole stick or card
//!    is given its own IDs, drawn from the system's randomness, and the
//!    image must be one they have places in: a stick's layout, or a Pi
//!    card's (`crate::ids`, `docs/STICK-IDENTITY.md` §4.2).
//! 3. What the desktop mounted from it is unmounted, and it is opened
//!    to write, exclusively and synchronously; its identity is read
//!    again after the open and must be what it was.
//! 4. *Keep data*: its partition table is a Faraday stick's by its
//!    layout, as `faraday/image/stick-boot.sh` checks it, and only the
//!    boot partition's range is written, keeping the serial the stick's
//!    boot partition had (§4.4); a whole image's own boot partition is
//!    taken from inside it as it streams, and the progress counts that
//!    partition's bytes alone, the only ones that reach the stick.
//! 5. The source is decompressed as it is written, and its SHA-256 must
//!    be the manifest's (or Confirm's); a signed boot file's partition
//!    and the stick's own IDs go in place of the source's own; then
//!    everything is flushed.
//! 6. Everything written is read back from the disk and compared with
//!    the source, decompressed again with the same changes in place;
//!    for a whole write, the SHA-256 of what was read, with the source's
//!    own bytes put back where the changes went, must be the manifest's
//!    (§4.3).
//! 7. *Keep data*: the partition table is what it was.
//! 8. The system is asked to read the partition table again.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use faraday_payload::{ImageKind, PayloadError, PayloadFile, Sha256Sum};
use faraday_starter::api::{InUse, Mode, Reason, StarterEvent, StickFault, WriteOrder, WriteStage};
use sha2::{Digest, Sha256};

use crate::gpt::{self, Boot};
use crate::ids::{Capture, Patch, Shape};
use crate::payload::Image;
use crate::signed;

/// What is written and compared at a time.
const CHUNK: usize = 4 << 20;
/// The alignment a read straight from the disk needs (`O_DIRECT`), and
/// the largest block a disk may have.
pub const ALIGN: usize = 4096;
/// The start of an image held back until its partition table is read.
const HEAD: usize = 64 << 10;

/// A disk as it is now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Its device (`/dev/sdb`).
    pub device: String,
    /// Its serial.
    pub serial: String,
    /// Its size in bytes.
    pub size: u64,
    /// Why the computer uses it, when it does.
    pub in_use: Option<InUse>,
}

/// A disk to write, as an operating system reaches it.
pub trait Device {
    /// The disk as it is now, or why it cannot be found.
    fn identity(&mut self) -> Result<Identity, Reason>;
    /// Unmounts what the desktop mounted from it.
    fn unmount(&mut self) -> Result<(), Reason>;
    /// Opens it to write: no one else may hold it open exclusively,
    /// and each write reaches the disk before it returns.
    fn open_write(&mut self) -> Result<File, Reason>;
    /// Opens it to read back what was written, from the disk itself
    /// rather than from what the system keeps of it in memory.
    fn open_read(&mut self) -> Result<File, Reason>;
    /// Asks the system to read its partition table again.
    fn rescan(&mut self) -> Result<(), Reason>;
    /// The length in bytes of the disk `file` (from [`Device::open_write`])
    /// reaches: where it ends, unless the system answers otherwise (a
    /// Windows physical disk says it in an IOCTL).
    fn length(&mut self, file: &mut File) -> io::Result<u64> {
        file.seek(SeekFrom::End(0))
    }
    /// The size in bytes of the disk's blocks. Every write the writer
    /// makes is a whole number of them at a multiple of it, and every
    /// read a whole number of [`ALIGN`] bytes at a multiple of that, so a
    /// disk that refuses anything else (macOS's raw `/dev/rdiskN`) is
    /// written as any other. A power of two from 512 to [`ALIGN`].
    fn block(&mut self) -> u64 {
        512
    }
}

/// What a write takes its bytes from, held by the writer's thread.
#[derive(Debug, Clone)]
pub enum Job {
    /// A payload file and its manifest line.
    Payload {
        /// Its manifest line.
        file: PayloadFile,
        /// Its bytes, the `.xz`.
        bytes: Cow<'static, [u8]>,
    },
    /// A developer's image.
    Image(Image),
}

impl Job {
    /// The source to write.
    pub fn source(&self) -> Source<'_> {
        match self {
            Job::Payload { file, bytes } => Source::Payload { file, bytes },
            Job::Image(i) => Source::Image(i),
        }
    }
}

/// How often progress goes back to the window.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// Writes `job` as `order` says on a thread of its own, with the device
/// `open` reaches, sending progress and the result to `out`.
pub fn spawn<F>(order: WriteOrder, job: Job, out: Sender<StarterEvent>, open: F)
where
    F: FnOnce() -> Result<Box<dyn Device>, Reason> + Send + 'static,
{
    std::thread::spawn(move || {
        let result = open().and_then(|mut device| {
            let mut last = None::<Instant>;
            let mut sent = None;
            let mut progress = |stage, done, total| {
                let now = Instant::now();
                // The end is sent once: keep data from a whole image
                // stays at its boot partition's end while the rest of
                // the image streams past.
                let end = done == total && sent != Some((stage, done, total));
                if end || last.is_none_or(|t| now - t >= PROGRESS_EVERY) {
                    last = Some(now);
                    sent = Some((stage, done, total));
                    let _ = out.send(StarterEvent::WriteProgress { stage, done, total });
                }
            };
            write(&order, job.source(), &mut *device, &mut progress)
        });
        let _ = out.send(StarterEvent::Written(result));
    });
}

/// What a write takes its bytes from.
#[derive(Debug, Clone, Copy)]
pub enum Source<'a> {
    /// A payload file and its manifest line.
    Payload {
        /// Its manifest line.
        file: &'a PayloadFile,
        /// Its bytes, the `.xz`.
        bytes: &'a [u8],
    },
    /// A developer's image, as hashed when the Starter started.
    Image(&'a Image),
}

impl Source<'_> {
    /// The bytes written, decompressed.
    fn size(&self) -> u64 {
        match self {
            Source::Payload { file, .. } => file.inner_size,
            Source::Image(i) => i.sum.size,
        }
    }

    /// The SHA-256 of the bytes written, decompressed: the manifest's,
    /// or Confirm's.
    fn sha256(&self) -> Sha256Sum {
        match self {
            Source::Payload { file, .. } => file.inner_sha256,
            Source::Image(i) => i.sum.sha256,
        }
    }

    fn boot_only(&self) -> bool {
        matches!(self, Source::Image(i) if i.boot_only)
    }

    /// Costs a decompression of the whole source and its SHA-256: the
    /// boot partition it writes (its own, or the whole of a
    /// boot partition image) and where that starts in it, checked as a
    /// write checks them: the manifest's sum (or Confirm's), a Faraday
    /// stick's partition table, a FAT32 boot sector labelled EFI.
    fn boot_partition(&self) -> Result<(u64, Vec<u8>), Reason> {
        if self.boot_only() {
            let mut out = Placed::new(Memory(Vec::new()), 0, true);
            self.pump(&mut out, &mut |_, _| {})
                .map_err(payload_reason)?;
            return Ok((0, out.out.sink.0));
        }
        let everything = Boot {
            offset: 0,
            len: u64::MAX,
        };
        let range = Cell::new(None);
        let mut out = Window::new(Memory(Vec::new()), everything, self.size(), &range);
        self.pump(&mut out, &mut |_, _| {})
            .map_err(payload_reason)?;
        let at = range.get().map_or(0, |(from, _)| from);
        Ok((at, out.out.sink.0))
    }

    /// Streams the source into `out`, decompressed, and succeeds only if
    /// what went in has the expected size and SHA-256.
    fn pump(
        &self,
        out: &mut dyn Write,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<(), PayloadError> {
        match self {
            Source::Payload { file, bytes } => {
                faraday_payload::write_payload(Cursor::new(*bytes), file, out, progress)
            }
            Source::Image(i) => {
                let f = File::open(&i.path).map_err(PayloadError::Read)?;
                faraday_payload::write_image(
                    BufReader::with_capacity(1 << 20, f),
                    i.kind,
                    &i.sum,
                    out,
                    progress,
                )
            }
        }
    }

    /// Checks the source before the disk is touched: a payload file
    /// against its manifest, an image against the sum Confirm showed.
    fn check(&self) -> Result<(), Reason> {
        match self {
            Source::Payload { file, bytes } => {
                let actual = sum(Cursor::new(*bytes), ImageKind::Plain)?.0;
                if actual != file.sha256 {
                    return Err(Reason::PayloadSum);
                }
            }
            Source::Image(i) => {
                let f = File::open(&i.path).map_err(|e| Reason::File {
                    path: i.path.display().to_string(),
                    detail: e.to_string(),
                })?;
                let (actual, size) = sum(BufReader::with_capacity(1 << 20, f), i.kind)?;
                if actual != i.sum.sha256 || size != i.sum.size {
                    return Err(Reason::ImageChanged {
                        path: i.path.display().to_string(),
                        sha256: actual.to_string(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Costs a decompression of the whole source and its SHA-256: the boot
/// partition `source` writes and where it starts in it, as a write would
/// write it ([`Source`]'s own check), for a signed boot file's check
/// (`crate::signed`).
pub fn boot_partition(source: &Source<'_>) -> Result<(u64, Vec<u8>), Reason> {
    source.boot_partition()
}

fn sum<R: Read>(src: R, kind: ImageKind) -> Result<(Sha256Sum, u64), Reason> {
    let s = faraday_payload::hash_image(src, kind).map_err(payload_reason)?;
    Ok((s.sha256, s.size))
}

/// A reason carried through an `io::Error`, for the places the writer
/// is reached through `Write`.
#[derive(Debug)]
struct Fail(Reason);

impl std::fmt::Display for Fail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl std::error::Error for Fail {}

fn fail(reason: Reason) -> io::Error {
    io::Error::other(Fail(reason))
}

/// The reason an `io::Error` carries, when [`fail`] made it.
fn carried(e: &io::Error) -> Option<Reason> {
    e.get_ref()?.downcast_ref::<Fail>().map(|f| f.0.clone())
}

/// The reason for a decoder or writer failure while writing.
pub fn payload_reason(e: PayloadError) -> Reason {
    match e {
        PayloadError::Read(e) => Reason::SourceRead(e.to_string()),
        PayloadError::Write(e) => carried(&e).unwrap_or_else(|| Reason::WriteFailed(e.to_string())),
        PayloadError::PayloadSum { .. } => Reason::PayloadSum,
        PayloadError::PayloadChanged => Reason::PayloadChanged,
        PayloadError::Damaged(d) => Reason::Damaged(d),
        PayloadError::Longer { expected } => Reason::Longer(expected),
        PayloadError::Shorter { expected, actual } => Reason::Shorter { expected, actual },
        PayloadError::Sum { actual, .. } => Reason::WrongSum(actual.to_string()),
    }
}

/// The same while reading the disk back, which names the device.
fn read_back_reason(e: PayloadError, device: &str) -> Reason {
    match e {
        PayloadError::Write(e) => match carried(&e) {
            Some(Reason::BackEnds { at, .. }) => Reason::BackEnds {
                device: device.to_string(),
                at,
            },
            Some(Reason::BackDiffers { at, .. }) => Reason::BackDiffers {
                device: device.to_string(),
                at,
            },
            Some(other) => other,
            None => Reason::BackFailed {
                device: device.to_string(),
                detail: e.to_string(),
            },
        },
        other => payload_reason(other),
    }
}

/// Why the computer's use of a disk stops a write.
pub fn in_use_reason(why: &InUse) -> Reason {
    Reason::InUse(why.clone())
}

/// Writes `source` to `device` as `order` says, reads it back and
/// compares, reporting progress as it goes. The error is the reason a
/// person is shown (as `faraday_starter::reason_text` words it).
pub fn write(
    order: &WriteOrder,
    source: Source<'_>,
    device: &mut dyn Device,
    progress: &mut dyn FnMut(WriteStage, u64, u64),
) -> Result<(), Reason> {
    // 1. Still the disk that was confirmed.
    let now = device.identity()?;
    check_identity(order, &now)?;
    if let Some(why) = &now.in_use {
        return Err(in_use_reason(why));
    }
    // 2. It fits, and the source is what was confirmed; what is changed
    // in it: a signed boot file and the stick's own IDs.
    let total = source.size();
    if order.mode == Mode::Whole {
        if source.boot_only() {
            return Err(Reason::BootOnly);
        }
        if total > now.size {
            return Err(Reason::TooSmall {
                disk: now.size,
                image: total,
            });
        }
    }
    source.check()?;
    let (mut patches, captured) = changes(order, &source)?;
    let block = device.block();
    if !block.is_power_of_two() || !(512..=ALIGN as u64).contains(&block) {
        return Err(Reason::Device {
            device: order.device.clone(),
            detail: format!("block size {block}"),
        });
    }
    // 3. Unmounted, and opened.
    device.unmount()?;
    let mut disk = device.open_write()?;
    // The name may reach another disk by now: the identity read again
    // after the open narrows that window, though it does not prove that
    // the handle is the disk it names.
    let opened = device.identity()?;
    check_opened(order, &now, &opened)?;
    let len = device.length(&mut disk).map_err(|e| Reason::Device {
        device: order.device.clone(),
        detail: e.to_string(),
    })?;
    // 4. Where it goes; keep data keeps the stick's boot serial.
    let (plan, table, serial) = place(order, &source, &mut disk, len)?;
    if let Some(serial) = serial {
        patches.extend(
            captured
                .boot_serial(serial)
                .map_err(Reason::NotStickImage)?,
        );
    }
    // Where a whole image's own boot partition lies in it, once the
    // window has read the image's table: keep data's progress counts its
    // bytes alone.
    let range = Cell::new(None);
    let mut report = |stage, done, total| match plan {
        Plan::Within(stick, _) => {
            let (done, total) = boot_progress(range.get(), stick.len, done);
            progress(stage, done, total);
        }
        Plan::At(_) | Plan::Boot(_) => progress(stage, done, total),
    };
    // 5. Written and flushed.
    {
        let out = plan.writer(
            Writing {
                disk: &mut disk,
                buf: None,
                block: block as usize,
            },
            &range,
        );
        let mut out = Patched::new(out, &patches);
        source
            .pump(&mut out, &mut |d, t| report(WriteStage::Writing, d, t))
            .map_err(payload_reason)?;
    }
    disk.sync_all().map_err(|e| Reason::Flush {
        device: order.device.clone(),
        detail: e.to_string(),
    })?;
    // Closed before it is read back: the last close drops what the
    // system kept of it in memory, so the read comes from the disk.
    drop(disk);
    // 6. Read back and compared, and with the patches taken back out
    // of what was read, the source's own SHA-256.
    let mut back = device.open_read()?;
    read_back(
        order,
        &source,
        plan,
        &patches,
        &mut back,
        &range,
        &mut |d, t| report(WriteStage::ReadingBack, d, t),
    )?;
    // 7. The partition table as it was.
    if let Some((table, before)) = table
        && regions(&mut back, &table)? != before
    {
        return Err(Reason::TableChanged(order.device.clone()));
    }
    drop(back);
    // 8. The system reads the new table. The write has been read back
    // and matched whatever this says.
    if let Err(e) = device.rescan() {
        eprintln!("faraday-starter: {}: {e:?}", order.device);
    }
    Ok(())
}

/// Costs a decompression of the whole source, and another for a boot
/// file the person signed: what a write changes in the source
/// (`docs/STICK-IDENTITY.md` §4.2), in the order it is applied: the
/// signed boot file's partition, then, for a whole stick or card, its
/// own IDs drawn from the system's randomness and the data partition's
/// name, where one is asked for (§6); and the sectors keep data's boot
/// serial is placed by. A name is written only with a whole stick or
/// card: keep data does not touch the data partition.
fn changes(order: &WriteOrder, source: &Source<'_>) -> Result<(Vec<Patch>, Capture), Reason> {
    let mut patches = Vec::new();
    if let Some(f) = &order.boot_file {
        patches.push(signed::prepare(f, source)?);
    }
    let shape = if source.boot_only() {
        Shape::Boot
    } else {
        Shape::Whole
    };
    let mut captured = Capture::new(shape, source.size());
    source
        .pump(&mut captured, &mut |_, _| {})
        .map_err(payload_reason)?;
    if order.mode == Mode::Whole {
        let mut random = Random::new()?;
        let fresh = captured
            .fresh(&mut |b| random.fill(b))
            .map_err(Reason::NotStickImage)?;
        patches.extend(fresh);
    }
    if let Some(name) = &order.name {
        let label = crate::ids::label_of(name)
            .filter(|_| order.mode == Mode::Whole && shape == Shape::Whole)
            .ok_or(Reason::NotStickImage(StickFault::Name))?;
        patches.extend(captured.name(label).map_err(Reason::NotStickImage)?);
    }
    Ok((patches, captured))
}

/// Bytes from the system's randomness (`getrandom`), drawn once and
/// handed out: the IDs are not secret, only each stick's own.
struct Random {
    pool: [u8; 256],
    at: usize,
}

impl Random {
    fn new() -> Result<Self, Reason> {
        let mut pool = [0u8; 256];
        getrandom::fill(&mut pool).map_err(|e| Reason::Random(e.to_string()))?;
        Ok(Random { pool, at: 0 })
    }

    /// Fills `out` from the pool. A stick's IDs take 56 bytes, a card's
    /// 12, so the pool is never drawn again; were it, a draw that fails
    /// would hand the same bytes out twice on one stick.
    fn fill(&mut self, out: &mut [u8]) {
        if self.at + out.len() > self.pool.len() {
            let _ = getrandom::fill(&mut self.pool);
            self.at = 0;
        }
        out.copy_from_slice(&self.pool[self.at..self.at + out.len()]);
        self.at += out.len();
    }
}

/// Costs a decompression of the whole source and a read of all that
/// was written: step 6. What `back` reads is compared with the source,
/// decompressed again with `patches` in place; for a whole write, the
/// SHA-256 of what was read, with the source's own bytes put back where
/// the patches went (`docs/STICK-IDENTITY.md` §4.3), must be the
/// source's.
fn read_back(
    order: &WriteOrder,
    source: &Source<'_>,
    plan: Plan,
    patches: &[Patch],
    back: &mut File,
    range: &Cell<Option<(u64, u64)>>,
    report: &mut dyn FnMut(u64, u64),
) -> Result<(), Reason> {
    let revert = RefCell::new(matches!(plan, Plan::At(0)).then(|| Revert {
        patches,
        hasher: Sha256::new(),
    }));
    {
        let compare = Comparing {
            disk: back,
            revert: &revert,
        };
        let mut out = Patched::new(plan.writer(compare, range), patches);
        source
            .pump(&mut out, report)
            .map_err(|e| read_back_reason(e, &order.device))?;
    }
    if let Some(r) = revert.into_inner() {
        let got = Sha256Sum(r.hasher.finalize().into());
        if got != source.sha256() {
            return Err(Reason::BackSum {
                device: order.device.clone(),
                sha256: got.to_string(),
            });
        }
    }
    Ok(())
}

/// A stick's partition table and the bytes of the regions it names,
/// read before keep data writes, to compare after.
type KeptTable = (gpt::Table, Vec<Vec<u8>>);

/// Where a write goes ([`place`]): the plan, keep data's table as it was,
/// and the boot serial it keeps.
type Placement = (Plan, Option<KeptTable>, Option<[u8; 4]>);

/// Where `source` goes on `disk` (`len` bytes long): from its first
/// byte, or, for keep data, at the stick's boot partition, with the
/// stick's partition table and what it holds, to check after the write,
/// and the serial its boot partition has, which the new one keeps
/// (`docs/STICK-IDENTITY.md` §4.4): a boot partition with none is given
/// its own.
fn place(
    order: &WriteOrder,
    source: &Source<'_>,
    disk: &mut File,
    len: u64,
) -> Result<Placement, Reason> {
    match order.mode {
        Mode::Whole => Ok((Plan::At(0), None, None)),
        Mode::KeepData => {
            let not_stick = |fault| Reason::NotStick {
                device: order.device.clone(),
                fault,
            };
            let table =
                gpt::read(&mut |at, buf| read_exact_at(disk, at, buf), len).map_err(not_stick)?;
            let total = source.size();
            let needs = if source.boot_only() { total } else { 0 };
            let boot = gpt::faraday_boot(&table, needs).map_err(not_stick)?;
            let before = regions(disk, &table)?;
            let plan = if source.boot_only() {
                Plan::Boot(boot)
            } else {
                Plan::Within(boot, total)
            };
            let mut sector = [0u8; 512];
            read_exact_at(disk, boot.offset, &mut sector)
                .map_err(|e| Reason::ReadingTable(e.to_string()))?;
            let serial = match faraday_fat::boot_sector_serial(&sector) {
                Some(s) if s != 0 => s.to_le_bytes(),
                _ => {
                    let mut own = [0u8; 4];
                    Random::new()?.fill(&mut own);
                    own[0] |= 1;
                    own
                }
            };
            Ok((plan, Some((table, before)), Some(serial)))
        }
    }
}

/// Keep data from a whole image: of the image's first `done` bytes,
/// those inside its own boot partition (`range`, once its table is
/// read), out of that partition's length, which is all that reaches the
/// stick; before the table is read, none of the stick's boot partition
/// (`stick` bytes).
fn boot_progress(range: Option<(u64, u64)>, stick: u64, done: u64) -> (u64, u64) {
    match range {
        Some((from, to)) => (done.clamp(from, to) - from, to - from),
        None => (0, stick),
    }
}

/// Whether `seen` is the disk `order` confirmed: the same device,
/// serial and size.
fn check_identity(order: &WriteOrder, seen: &Identity) -> Result<(), Reason> {
    if seen.device != order.device || seen.serial != order.serial || seen.size != order.size {
        return Err(Reason::Changed {
            device: order.device.clone(),
            serial: seen.serial.clone(),
            size: seen.size,
        });
    }
    Ok(())
}

/// Whether `opened`, read after the disk was opened, is the disk `now`
/// was: the same device and size, and the same serial when both have
/// one (some disks report none).
fn check_opened(order: &WriteOrder, now: &Identity, opened: &Identity) -> Result<(), Reason> {
    let serials = now.serial.is_empty() || opened.serial.is_empty() || now.serial == opened.serial;
    if opened.device != now.device || opened.size != now.size || !serials {
        return Err(Reason::Changed {
            device: order.device.clone(),
            serial: opened.serial.clone(),
            size: opened.size,
        });
    }
    Ok(())
}

/// Reads the stick's partition table and up to `faraday_boot::MAX_PARTITION`
/// bytes of its boot partition: which Faraday the stick on `disk` holds
/// (`len` bytes long, `device` as a person knows it): the kernel's
/// release string searched for in the boot partition's bytes
/// (`faraday_boot::find_release`). The partition table must be a
/// Faraday stick's, as keep data checks it; `None` for a stick made
/// before Faraday put its version there. Read in aligned blocks, as a
/// disk opened unbuffered (Windows) takes its reads. The elevated copy's
/// `--read-version` and its keep data's second check read the stick.
pub fn release_on_stick<R: Read + Seek>(
    disk: &mut R,
    len: u64,
    device: &str,
) -> Result<Option<String>, Reason> {
    let not_stick = |fault| Reason::NotStick {
        device: device.to_string(),
        fault,
    };
    let failed = |e: io::Error| Reason::Device {
        device: device.to_string(),
        detail: e.to_string(),
    };
    let table = gpt::read(
        &mut |at, buf| {
            let (raw, skip, got) = aligned_read_at(disk, at, buf.len())?;
            if got < skip + buf.len() {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }
            buf.copy_from_slice(&raw[skip..skip + buf.len()]);
            Ok(())
        },
        len,
    )
    .map_err(not_stick)?;
    let boot = gpt::faraday_boot(&table, 0).map_err(not_stick)?;
    let end = boot.offset + boot.len.min(faraday_boot::MAX_PARTITION);
    // Each chunk keeps the end of the one before, so a string that
    // straddles two is still seen whole.
    let overlap = faraday_boot::MAX_RELEASE * 2;
    let mut hay: Vec<u8> = Vec::with_capacity(CHUNK + overlap);
    let mut at = boot.offset;
    while at < end {
        let n = (CHUNK as u64).min(end - at) as usize;
        let (raw, skip, got) = aligned_read_at(disk, at, n).map_err(failed)?;
        if got < skip + n {
            return Err(failed(io::Error::from(io::ErrorKind::UnexpectedEof)));
        }
        hay.extend_from_slice(&raw[skip..skip + n]);
        if let Some(r) = faraday_boot::find_release(&hay) {
            return Ok(Some(r));
        }
        hay.drain(..hay.len().saturating_sub(overlap));
        at += n as u64;
    }
    Ok(None)
}

/// Keep data's second check, in the elevated writer on Windows:
/// writing build `build` over a stick that holds the release
/// string `on_stick` is refused when it is the same build, by Keep
/// data's rule (`faraday_core::build::compare`). A source with
/// no build ID it can be compared by passes, as Confirm lets it.
pub fn refuse_same_build(on_stick: Option<&str>, build: Option<&str>) -> Result<(), Reason> {
    let on_stick = on_stick.and_then(faraday_core::build::build_id);
    match build {
        Some(b) if faraday_core::build::compare(on_stick, b) == faraday_core::build::Fit::Same => {
            Err(Reason::SameBuild)
        }
        _ => Ok(()),
    }
}

/// The bytes of the partition table's regions. Read in aligned blocks:
/// after the write the disk is open for reads straight from it
/// (`O_DIRECT`), which refuses an unaligned offset, length or buffer.
pub fn regions<R: Read + Seek>(disk: &mut R, table: &gpt::Table) -> Result<Vec<Vec<u8>>, Reason> {
    table
        .regions
        .iter()
        .map(|&(from, to)| {
            let len = (to - from) as usize;
            let (raw, skip, got) = aligned_read_at(disk, from, len)
                .map_err(|e| Reason::ReadingTable(e.to_string()))?;
            if got < skip + len {
                return Err(Reason::ReadingTable(String::from(
                    "the disk ends inside its partition table",
                )));
            }
            Ok(raw[skip..skip + len].to_vec())
        })
        .collect()
}

/// Reads the `len` bytes at `at` from an aligned offset, for an aligned
/// length, into an aligned buffer, as a read straight from the disk
/// needs. Returns the buffer, where the bytes at `at` start in it, and
/// how many bytes were read (fewer at the disk's end).
fn aligned_read_at<R: Read + Seek>(
    r: &mut R,
    at: u64,
    len: usize,
) -> io::Result<(AlignedBuf, usize, usize)> {
    let start = at - at % ALIGN as u64;
    let skip = (at - start) as usize;
    let want = (skip + len).div_ceil(ALIGN) * ALIGN;
    let mut raw = AlignedBuf::new(want);
    r.seek(SeekFrom::Start(start))?;
    let mut got = 0;
    while got < want {
        match r.read(&mut raw[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok((raw, skip, got))
}

/// A buffer whose first byte is aligned to [`ALIGN`].
struct AlignedBuf {
    raw: Vec<u8>,
    off: usize,
    len: usize,
}

impl AlignedBuf {
    fn new(len: usize) -> Self {
        let raw = vec![0u8; len + ALIGN];
        let off = raw.as_ptr().align_offset(ALIGN);
        AlignedBuf { raw, off, len }
    }
}

impl std::ops::Deref for AlignedBuf {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.raw[self.off..self.off + self.len]
    }
}

impl std::ops::DerefMut for AlignedBuf {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.raw[self.off..self.off + self.len]
    }
}

/// Reads `buf.len()` bytes at `at` through aligned blocks, as a disk
/// opened for unbuffered writing (Windows) also takes its reads.
fn read_exact_at(f: &mut File, at: u64, buf: &mut [u8]) -> io::Result<()> {
    let (raw, skip, got) = aligned_read_at(f, at, buf.len())?;
    if got < skip + buf.len() {
        return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
    }
    buf.copy_from_slice(&raw[skip..skip + buf.len()]);
    Ok(())
}

/// Where the source's bytes go on the disk.
#[derive(Debug, Clone, Copy)]
enum Plan {
    /// Byte `i` of the source to byte `base + i`.
    At(u64),
    /// A boot partition alone, at the stick's boot partition.
    Boot(Boot),
    /// A whole image of `total` bytes, of which only its own boot
    /// partition goes to the stick's.
    Within(Boot, u64),
}

impl Plan {
    /// A writer to `sink`; a whole image's window keeps where its boot
    /// partition lies in `range`.
    fn writer<'a, K: Flush + 'a>(
        self,
        sink: K,
        range: &'a Cell<Option<(u64, u64)>>,
    ) -> Box<dyn Write + 'a> {
        match self {
            Plan::At(base) => Box::new(Placed::new(sink, base, false)),
            Plan::Boot(b) => Box::new(Placed::new(sink, b.offset, true)),
            Plan::Within(b, total) => Box::new(Window::new(sink, b, total, range)),
        }
    }
}

/// Writes to the disk, a few megabytes at a time.
///
/// Each stretch goes out from a buffer aligned to [`ALIGN`], as a disk
/// opened unbuffered (Windows) needs, as whole blocks at a multiple of
/// the block size, as a raw disk (macOS) needs: a stretch that starts
/// or ends inside a block (an image whose length is not a whole number
/// of blocks, or of 4096-byte blocks on a disk that has them) is
/// written as the whole blocks it touches, with what the disk held
/// around it read first and written back unchanged.
struct Writing<'a> {
    disk: &'a mut File,
    buf: Option<AlignedBuf>,
    block: usize,
}

/// Compares with the disk, a few megabytes at a time; with a
/// [`Revert`], also hashes what was read with the source's own bytes put
/// back where the patches went.
struct Comparing<'a, 'p> {
    disk: &'a mut File,
    revert: &'a RefCell<Option<Revert<'p>>>,
}

/// What was read back of a whole write, with the patches taken back out
/// of it, hashed as it comes (`docs/STICK-IDENTITY.md` §4.3).
struct Revert<'p> {
    patches: &'p [Patch],
    hasher: Sha256,
}

/// Gathers a boot partition in memory (`Source::boot_partition`).
struct Memory(Vec<u8>);

impl Flush for Memory {
    fn flush_at(&mut self, at: u64, data: &[u8]) -> io::Result<()> {
        let from = usize::try_from(at).map_err(io::Error::other)?;
        let to = from + data.len();
        if self.0.len() < to {
            self.0.resize(to, 0);
        }
        self.0[from..to].copy_from_slice(data);
        Ok(())
    }
}

/// The source's stream on its way to the disk, with each patch's bytes
/// in place of the source's (a signed boot file's partition,
/// `crate::signed`; the stick's own IDs, `crate::ids`), applied in
/// order: written so, and compared so when it is read back. The
/// source's own SHA-256 is still the one checked, before this.
struct Patched<'a, 'p> {
    out: Box<dyn Write + 'a>,
    pos: u64,
    patches: &'p [Patch],
}

impl<'a, 'p> Patched<'a, 'p> {
    fn new(out: Box<dyn Write + 'a>, patches: &'p [Patch]) -> Self {
        Patched {
            out,
            pos: 0,
            patches,
        }
    }
}

/// Puts `from(p)` of each patch `p` over the bytes of `data`, which
/// starts at stream byte `at`, in the order `patches` goes.
fn overlay<'p>(
    data: &mut [u8],
    at: u64,
    patches: impl Iterator<Item = &'p Patch>,
    from: impl Fn(&'p Patch) -> &'p [u8],
) {
    let end = at + data.len() as u64;
    for p in patches {
        let bytes = from(p);
        let (a, b) = (at.max(p.at), end.min(p.at + bytes.len() as u64));
        if a < b {
            data[(a - at) as usize..(b - at) as usize]
                .copy_from_slice(&bytes[(a - p.at) as usize..(b - p.at) as usize]);
        }
    }
}

impl Write for Patched<'_, '_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let at = self.pos;
        let end = at + data.len() as u64;
        self.pos = end;
        let touched = self
            .patches
            .iter()
            .any(|p| p.at < end && at < p.at + p.bytes.len() as u64);
        if !touched {
            self.out.write_all(data)?;
            return Ok(data.len());
        }
        let mut chunk = data.to_vec();
        overlay(&mut chunk, at, self.patches.iter(), |p| &p.bytes);
        self.out.write_all(&chunk)?;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

/// Bytes gathered for one contiguous stretch of the disk.
struct Gathered {
    at: u64,
    buf: Vec<u8>,
}

/// A sink that gathers contiguous bytes and hands them on a chunk at a
/// time.
struct Buffered<K> {
    sink: K,
    pending: Option<Gathered>,
}

/// Where bytes for the disk go: written, or compared.
trait Flush {
    /// One contiguous stretch for the disk at offset `at`.
    fn flush_at(&mut self, at: u64, data: &[u8]) -> io::Result<()>;
}

impl Flush for Writing<'_> {
    fn flush_at(&mut self, at: u64, data: &[u8]) -> io::Result<()> {
        let block = self.block as u64;
        let end = at + data.len() as u64;
        let (start, stop) = (at - at % block, end.div_ceil(block) * block);
        if start == at && stop == end {
            let buf = self.buf.get_or_insert_with(|| AlignedBuf::new(CHUNK));
            self.disk.seek(SeekFrom::Start(at))?;
            for part in data.chunks(CHUNK) {
                let out = &mut buf[..part.len()];
                out.copy_from_slice(part);
                self.disk.write_all(out)?;
            }
            return Ok(());
        }
        // The whole blocks the stretch touches: the first and the last
        // as the disk holds them, then the stretch over them.
        let mut whole = AlignedBuf::new((stop - start) as usize);
        let b = self.block;
        if start < at {
            read_exact_at(self.disk, start, &mut whole[..b])?;
        }
        if end < stop {
            let last = whole.len() - b;
            read_exact_at(self.disk, stop - block, &mut whole[last..])?;
        }
        let from = (at - start) as usize;
        whole[from..from + data.len()].copy_from_slice(data);
        self.disk.seek(SeekFrom::Start(start))?;
        for part in whole.chunks(CHUNK) {
            self.disk.write_all(part)?;
        }
        Ok(())
    }
}

impl Flush for Comparing<'_, '_> {
    fn flush_at(&mut self, at: u64, data: &[u8]) -> io::Result<()> {
        let (buf, skip, got) = aligned_read_at(&mut *self.disk, at, data.len())?;
        let start = at - skip as u64;
        if got < skip + data.len() {
            return Err(fail(Reason::BackEnds {
                device: String::new(),
                at: start + got as u64,
            }));
        }
        if let Some(i) = buf[skip..skip + data.len()]
            .iter()
            .zip(data)
            .position(|(a, b)| a != b)
        {
            return Err(fail(Reason::BackDiffers {
                device: String::new(),
                at: at + i as u64,
            }));
        }
        if let Some(r) = self.revert.borrow_mut().as_mut() {
            // What was read, with the source's own bytes put back
            // where the patches went, the last patch first.
            let mut read = buf[skip..skip + data.len()].to_vec();
            overlay(&mut read, at, r.patches.iter().rev(), |p| &p.was);
            r.hasher.update(&read);
        }
        Ok(())
    }
}

impl<K: Flush> Buffered<K> {
    fn new(sink: K) -> Self {
        Buffered {
            sink,
            pending: None,
        }
    }

    fn put(&mut self, mut at: u64, mut data: &[u8]) -> io::Result<()> {
        while !data.is_empty() {
            let contiguous = self
                .pending
                .as_ref()
                .is_some_and(|p| p.at + p.buf.len() as u64 == at);
            if !contiguous {
                self.done()?;
                self.pending = Some(Gathered {
                    at,
                    buf: Vec::with_capacity(CHUNK),
                });
            }
            let p = self.pending.as_mut().expect("just set");
            let take = (CHUNK - p.buf.len()).min(data.len());
            p.buf.extend_from_slice(&data[..take]);
            data = &data[take..];
            at += take as u64;
            if p.buf.len() == CHUNK {
                self.done()?;
            }
        }
        Ok(())
    }

    fn done(&mut self) -> io::Result<()> {
        if let Some(p) = self.pending.take()
            && !p.buf.is_empty()
        {
            self.sink.flush_at(p.at, &p.buf)?;
        }
        Ok(())
    }
}

/// The source's bytes placed at `base` on the disk; with `label`, the
/// first sector must be a FAT32 boot sector labelled EFI before any
/// of it is let through.
struct Placed<K: Flush> {
    out: Buffered<K>,
    base: u64,
    pos: u64,
    /// The first sector is still to be checked.
    label: bool,
    first: Vec<u8>,
}

impl<K: Flush> Placed<K> {
    fn new(sink: K, base: u64, label: bool) -> Self {
        Placed {
            out: Buffered::new(sink),
            base,
            pos: 0,
            label,
            first: Vec::new(),
        }
    }
}

/// Whether `sector` is a FAT32 boot sector labelled EFI
/// (`docs/STICK-IDENTITY.md` §3), as
/// `stick-boot.sh` checks a boot partition before writing it.
fn faraday_boot(sector: &[u8]) -> io::Result<()> {
    let fat32 = sector.len() >= 512 && &sector[82..90] == b"FAT32   ";
    if fat32 && faraday_fat::boot_sector_label(sector) == Some(*b"EFI        ") {
        Ok(())
    } else {
        Err(fail(Reason::BootLabel))
    }
}

impl<K: Flush> Write for Placed<K> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.label {
            let take = (512 - self.first.len()).min(data.len());
            self.first.extend_from_slice(&data[..take]);
            if self.first.len() == 512 {
                faraday_boot(&self.first)?;
                self.label = false;
                let first = std::mem::take(&mut self.first);
                self.out.put(self.base, &first)?;
                self.pos = 512;
                self.out.put(self.base + self.pos, &data[take..])?;
                self.pos += (data.len() - take) as u64;
            }
            return Ok(data.len());
        }
        self.out.put(self.base + self.pos, data)?;
        self.pos += data.len() as u64;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.label {
            return Err(fail(Reason::BootShort));
        }
        self.out.done()
    }
}

/// A whole stick image streaming past, of which only its own boot
/// partition goes to the stick's: the image's partition table is read
/// from its first bytes, must be a Faraday stick's, and its boot
/// partition must fit the stick's; nothing reaches the disk before all
/// of that holds.
struct Window<'r, K: Flush> {
    out: Buffered<K>,
    target: Boot,
    total: u64,
    pos: u64,
    head: Vec<u8>,
    /// The image's boot partition, from its first byte to past its
    /// last, once its table is read; shared with the write's progress.
    range: &'r Cell<Option<(u64, u64)>>,
    first: Vec<u8>,
}

impl<'r, K: Flush> Window<'r, K> {
    fn new(sink: K, target: Boot, total: u64, range: &'r Cell<Option<(u64, u64)>>) -> Self {
        Window {
            out: Buffered::new(sink),
            target,
            total,
            pos: 0,
            head: Vec::new(),
            range,
            first: Vec::new(),
        }
    }

    /// Reads the image's table from what has been held back.
    fn locate(&mut self) -> io::Result<()> {
        let head = &self.head;
        let mut read_at = |at: u64, buf: &mut [u8]| -> io::Result<()> {
            let from = at as usize;
            let src = head.get(from..from + buf.len()).ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "past the image's first bytes")
            })?;
            buf.copy_from_slice(src);
            Ok(())
        };
        let bad = |fault: StickFault| fail(Reason::NotStickImage(fault));
        let table = gpt::read(&mut read_at, self.total).map_err(bad)?;
        let boot = gpt::faraday_boot(&table, 0).map_err(bad)?;
        if boot.offset + boot.len > self.total {
            return Err(bad(StickFault::BootPastEnd));
        }
        if boot.len > self.target.len {
            return Err(fail(Reason::BootLarger {
                image: boot.len,
                stick: self.target.len,
            }));
        }
        self.range.set(Some((boot.offset, boot.offset + boot.len)));
        let held = std::mem::take(&mut self.head);
        self.pass(0, &held)
    }

    /// The part of `data`, which starts at image byte `at`, inside the
    /// image's boot partition, to the stick's.
    fn pass(&mut self, at: u64, data: &[u8]) -> io::Result<()> {
        let Some((r0, r1)) = self.range.get() else {
            return Ok(());
        };
        let end = at + data.len() as u64;
        let (from, to) = (at.max(r0), end.min(r1));
        if from >= to {
            return Ok(());
        }
        let mut part = &data[(from - at) as usize..(to - at) as usize];
        let mut offset = from - r0;
        if offset < 512 {
            let take = (512 - offset as usize).min(part.len());
            self.first.extend_from_slice(&part[..take]);
            part = &part[take..];
            offset += take as u64;
            if self.first.len() < 512 {
                return Ok(());
            }
            faraday_boot(&self.first)?;
            let first = std::mem::take(&mut self.first);
            self.out.put(self.target.offset, &first)?;
        }
        self.out.put(self.target.offset + offset, part)
    }
}

impl<K: Flush> Write for Window<'_, K> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let at = self.pos;
        self.pos += data.len() as u64;
        if self.range.get().is_none() {
            let take = (HEAD - self.head.len()).min(data.len());
            self.head.extend_from_slice(&data[..take]);
            if self.head.len() < HEAD && self.pos < self.total {
                return Ok(data.len());
            }
            self.locate()?;
            self.pass(at + take as u64, &data[take..])?;
            return Ok(data.len());
        }
        self.pass(at, data)?;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.range.get().is_none() {
            self.locate()?;
        }
        self.out.done()
    }
}
