//! The writer (`docs/HUB.md` §4.1), the same on every operating system:
//! what it is given to write ([`Source`]) and the disk it writes
//! ([`Device`], which each system's module opens its own way).
//!
//! In order, and nothing written until every check before the write has
//! passed:
//!
//! 1. The disk is still the one confirmed: the same device, serial and
//!    size, and the computer does not use it.
//! 2. What is written fits it; a payload file matches its manifest, and
//!    a developer's image still has the SHA-256 Confirm showed.
//! 3. What the desktop mounted from it is unmounted, and it is opened
//!    to write, exclusively and synchronously.
//! 4. *Keep data*: its partition table is a Faraday stick's, as
//!    `faraday/image/stick-boot.sh` checks it, and only the boot
//!    partition's range is written; a whole image's own boot partition
//!    is taken from inside it as it streams.
//! 5. The source is decompressed as it is written, and its SHA-256 must
//!    be the manifest's (or Confirm's); then everything is flushed.
//! 6. Everything written is read back from the disk and compared with
//!    the source, decompressed again.
//! 7. *Keep data*: the partition table is what it was.
//! 8. The system is asked to read the partition table again.

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use faraday_hub::api::{HubEvent, InUse, Mode, WriteOrder, WriteStage};
use faraday_payload::{ImageKind, PayloadError, PayloadFile, Sha256Sum};

use crate::gpt::{self, Boot};
use crate::payload::Image;

/// What is written and compared at a time.
const CHUNK: usize = 4 << 20;
/// The alignment a read straight from the disk needs (`O_DIRECT`).
const ALIGN: usize = 4096;
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
    fn identity(&mut self) -> Result<Identity, String>;
    /// Unmounts what the desktop mounted from it.
    fn unmount(&mut self) -> Result<(), String>;
    /// Opens it to write: no one else may hold it open exclusively,
    /// and each write reaches the disk before it returns.
    fn open_write(&mut self) -> Result<File, String>;
    /// Opens it to read back what was written, from the disk itself
    /// rather than from what the system keeps of it in memory.
    fn open_read(&mut self) -> Result<File, String>;
    /// Asks the system to read its partition table again.
    fn rescan(&mut self) -> Result<(), String>;
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
pub fn spawn<F>(order: WriteOrder, job: Job, out: Sender<HubEvent>, open: F)
where
    F: FnOnce() -> Result<Box<dyn Device>, String> + Send + 'static,
{
    std::thread::spawn(move || {
        let result = open().and_then(|mut device| {
            let mut last = None::<Instant>;
            let mut progress = |stage, done, total| {
                let now = Instant::now();
                if done == total || last.is_none_or(|t| now - t >= PROGRESS_EVERY) {
                    last = Some(now);
                    let _ = out.send(HubEvent::WriteProgress { stage, done, total });
                }
            };
            write(&order, job.source(), &mut *device, &mut progress)
        });
        let _ = out.send(HubEvent::Written(result));
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
    /// A developer's image, as hashed when the hub app started.
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

    fn boot_only(&self) -> bool {
        matches!(self, Source::Image(i) if i.boot_only)
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
    fn check(&self) -> Result<(), String> {
        match self {
            Source::Payload { file, bytes } => {
                let actual = sum(Cursor::new(*bytes), ImageKind::Plain)?.0;
                if actual != file.sha256 {
                    return Err(PayloadError::PayloadSum {
                        expected: file.sha256,
                        actual,
                    }
                    .to_string());
                }
            }
            Source::Image(i) => {
                let f = File::open(&i.path).map_err(|e| format!("{}: {e}", i.path.display()))?;
                let (actual, size) = sum(BufReader::with_capacity(1 << 20, f), i.kind)?;
                if actual != i.sum.sha256 || size != i.sum.size {
                    return Err(format!(
                        "{} has changed since the hub app started: its SHA-256 is now {actual}; nothing was written",
                        i.path.display()
                    ));
                }
            }
        }
        Ok(())
    }
}

fn sum<R: Read>(src: R, kind: ImageKind) -> Result<(Sha256Sum, u64), String> {
    let s = faraday_payload::hash_image(src, kind).map_err(|e| e.to_string())?;
    Ok((s.sha256, s.size))
}

/// Why the computer's use of a disk stops a write.
pub fn in_use_reason(why: &InUse) -> String {
    match why {
        InUse::Mounted(at) => format!("a partition on it is mounted at {at}; nothing was written"),
        InUse::Swap => String::from("a partition on it is in use as swap; nothing was written"),
        InUse::System => String::from("it holds this computer's system; nothing was written"),
    }
}

/// Writes `source` to `device` as `order` says, reads it back and
/// compares, reporting progress as it goes. The error is the reason a
/// person is shown.
pub fn write(
    order: &WriteOrder,
    source: Source<'_>,
    device: &mut dyn Device,
    progress: &mut dyn FnMut(WriteStage, u64, u64),
) -> Result<(), String> {
    // 1. Still the disk that was confirmed.
    let now = device.identity()?;
    if now.device != order.device || now.serial != order.serial || now.size != order.size {
        return Err(format!(
            "{} is no longer the disk that was confirmed (serial {}, {} bytes); nothing was written",
            order.device,
            if now.serial.is_empty() {
                "none"
            } else {
                &now.serial
            },
            now.size
        ));
    }
    if let Some(why) = &now.in_use {
        return Err(in_use_reason(why));
    }
    // 2. It fits, and the source is what was confirmed.
    let total = source.size();
    if order.mode == Mode::Whole {
        if source.boot_only() {
            return Err(String::from(
                "a boot partition alone is written only over a Faraday stick, keeping its data",
            ));
        }
        if total > now.size {
            return Err(format!(
                "the disk ({} bytes) is smaller than the image ({total} bytes); nothing was written",
                now.size
            ));
        }
    }
    source.check()?;
    // 3. Unmounted, and opened.
    device.unmount()?;
    let mut disk = device.open_write()?;
    let len = disk
        .seek(SeekFrom::End(0))
        .map_err(|e| format!("{}: {e}", order.device))?;
    // 4. Where it goes.
    let (plan, table) = match order.mode {
        Mode::Whole => (Plan::At(0), None),
        Mode::KeepData => {
            let table =
                gpt::read(&mut |at, buf| read_exact_at(&mut disk, at, buf), len).map_err(|e| {
                    format!(
                        "{} is not a Faraday stick: {e}; nothing was written",
                        order.device
                    )
                })?;
            let needs = if source.boot_only() { total } else { 0 };
            let boot = gpt::faraday_boot(&table, needs).map_err(|e| {
                format!(
                    "{} is not a Faraday stick: {e}; nothing was written",
                    order.device
                )
            })?;
            let before = regions(&mut disk, &table)?;
            let plan = if source.boot_only() {
                Plan::Boot(boot)
            } else {
                Plan::Within(boot, total)
            };
            (plan, Some((table, before)))
        }
    };
    // 5. Written and flushed.
    {
        let mut out = plan.writer(Writing(&mut disk));
        source
            .pump(&mut *out, &mut |d, t| progress(WriteStage::Writing, d, t))
            .map_err(|e| e.to_string())?;
    }
    disk.sync_all()
        .map_err(|e| format!("flushing {}: {e}", order.device))?;
    // Closed before it is read back: the last close drops what the
    // system kept of it in memory, so the read comes from the disk.
    drop(disk);
    // 6. Read back and compared.
    let mut back = device.open_read()?;
    {
        let mut out = plan.writer(Comparing(&mut back));
        source
            .pump(&mut *out, &mut |d, t| {
                progress(WriteStage::ReadingBack, d, t)
            })
            .map_err(|e| match e {
                PayloadError::Write(e) => format!("reading back {}: {e}", order.device),
                other => format!("reading back {}: {other}", order.device),
            })?;
    }
    // 7. The partition table as it was.
    if let Some((table, before)) = table
        && regions(&mut back, &table)? != before
    {
        return Err(format!("the partition table on {} changed", order.device));
    }
    drop(back);
    // 8. The system reads the new table. The write has been read back
    // and matched whatever this says.
    if let Err(e) = device.rescan() {
        eprintln!("faraday-hub: {}: {e}", order.device);
    }
    Ok(())
}

/// The bytes of the partition table's regions.
fn regions(disk: &mut File, table: &gpt::Table) -> Result<Vec<Vec<u8>>, String> {
    table
        .regions
        .iter()
        .map(|&(from, to)| {
            let mut buf = vec![0u8; (to - from) as usize];
            read_exact_at(disk, from, &mut buf)
                .map_err(|e| format!("reading the partition table: {e}"))?;
            Ok(buf)
        })
        .collect()
}

fn read_exact_at(f: &mut File, at: u64, buf: &mut [u8]) -> io::Result<()> {
    f.seek(SeekFrom::Start(at))?;
    f.read_exact(buf)
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
    fn writer<'a, K: Flush + 'a>(self, sink: K) -> Box<dyn Write + 'a> {
        match self {
            Plan::At(base) => Box::new(Placed::new(sink, base, false)),
            Plan::Boot(b) => Box::new(Placed::new(sink, b.offset, true)),
            Plan::Within(b, total) => Box::new(Window::new(sink, b, total)),
        }
    }
}

/// Writes to the disk, a few megabytes at a time.
struct Writing<'a>(&'a mut File);

/// Compares with the disk, a few megabytes at a time.
struct Comparing<'a>(&'a mut File);

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
        self.0.seek(SeekFrom::Start(at))?;
        self.0.write_all(data)
    }
}

impl Flush for Comparing<'_> {
    fn flush_at(&mut self, at: u64, data: &[u8]) -> io::Result<()> {
        // Read from an aligned offset, an aligned length, into an
        // aligned buffer, as a read straight from the disk needs.
        let start = at - at % ALIGN as u64;
        let skip = (at - start) as usize;
        let want = (skip + data.len()).div_ceil(ALIGN) * ALIGN;
        let mut raw = vec![0u8; want + ALIGN];
        let off = raw.as_ptr().align_offset(ALIGN);
        let buf = &mut raw[off..off + want];
        self.0.seek(SeekFrom::Start(start))?;
        let mut got = 0;
        while got < want {
            match self.0.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if got < skip + data.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("the disk ends at byte {}", start + got as u64),
            ));
        }
        if let Some(i) = buf[skip..skip + data.len()]
            .iter()
            .zip(data)
            .position(|(a, b)| a != b)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "what was read back differs from what was written at byte {}",
                    at + i as u64
                ),
            ));
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
/// first sector must be a FAT32 boot sector labelled OSKBOOT before any
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

/// Whether `sector` is a FAT32 boot sector labelled OSKBOOT, as
/// `stick-boot.sh` checks a boot partition before writing it.
fn oskboot(sector: &[u8]) -> io::Result<()> {
    let fat32 = sector.len() >= 512 && &sector[82..90] == b"FAT32   ";
    if fat32 && faraday_fat::boot_sector_label(sector) == Some(*b"OSKBOOT    ") {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the image's boot partition is not a FAT32 filesystem labelled OSKBOOT",
        ))
    }
}

impl<K: Flush> Write for Placed<K> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.label {
            let take = (512 - self.first.len()).min(data.len());
            self.first.extend_from_slice(&data[..take]);
            if self.first.len() == 512 {
                oskboot(&self.first)?;
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
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the boot partition image is shorter than one sector",
            ));
        }
        self.out.done()
    }
}

/// A whole stick image streaming past, of which only its own OSKBOOT
/// partition goes to the stick's: the image's partition table is read
/// from its first bytes, must be a Faraday stick's, and its boot
/// partition must fit the stick's; nothing reaches the disk before all
/// of that holds.
struct Window<K: Flush> {
    out: Buffered<K>,
    target: Boot,
    total: u64,
    pos: u64,
    head: Vec<u8>,
    range: Option<(u64, u64)>,
    first: Vec<u8>,
}

impl<K: Flush> Window<K> {
    fn new(sink: K, target: Boot, total: u64) -> Self {
        Window {
            out: Buffered::new(sink),
            target,
            total,
            pos: 0,
            head: Vec::new(),
            range: None,
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
        let bad = |e: String| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("the image is not a Faraday stick image: {e}"),
            )
        };
        let table = gpt::read(&mut read_at, self.total).map_err(bad)?;
        let boot = gpt::faraday_boot(&table, 0).map_err(bad)?;
        if boot.offset + boot.len > self.total {
            return Err(bad(String::from("its boot partition runs past its end")));
        }
        if boot.len > self.target.len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "the image's boot partition ({} bytes) is larger than the stick's ({} bytes)",
                    boot.len, self.target.len
                ),
            ));
        }
        self.range = Some((boot.offset, boot.offset + boot.len));
        let held = std::mem::take(&mut self.head);
        self.pass(0, &held)
    }

    /// The part of `data`, which starts at image byte `at`, inside the
    /// image's boot partition, to the stick's.
    fn pass(&mut self, at: u64, data: &[u8]) -> io::Result<()> {
        let Some((r0, r1)) = self.range else {
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
            oskboot(&self.first)?;
            let first = std::mem::take(&mut self.first);
            self.out.put(self.target.offset, &first)?;
        }
        self.out.put(self.target.offset + offset, part)
    }
}

impl<K: Flush> Write for Window<K> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let at = self.pos;
        self.pos += data.len() as u64;
        if self.range.is_none() {
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
        if self.range.is_none() {
            self.locate()?;
        }
        self.out.done()
    }
}
