//! The disk process's work (`docs/history/PLAN.md` §4.3): find the partitions it has
//! been handed, and answer one request about them.
//!
//! A partition is handed out by `faraday-grant` making its device node
//! this process's, mode `0600`. A USB or SD partition this process cannot
//! open was not handed out: it is listed with no files, so the app knows
//! a stick is there and asks to lock (`docs/history/PLAN.md` §5.4). Nothing here mounts anything: the
//! filesystem is read by `faraday-fat`. A boot partition is never touched,
//! even if it could be opened (`docs/STICK-IDENTITY.md` §5.2): the first
//! partition of the medium the machine started from, the Pi's own card's
//! first, one whose GPT name is a boot partition's, and, as the second
//! lock on the door the grant helper keeps, any FAT volume labelled as a
//! boot partition is (`EFI`, `bootfs`, or `FARADAYBOOT` on a stick made
//! before F174; `faraday_boot::layout`).
//!
//! Which disk the machine started from is the medium's record
//! (`faraday_boot::medium`), read at every listing. Its data partition is
//! the boot medium's; a stick written by another tool has the image's FAT
//! serial there, and the first listing gives it a serial of its own
//! (§7).
//!
//! Where `/sys` and `/dev` are is a parameter, so the tests can stand a
//! directory tree and image files in for them.

use faraday_crypto::work::Compute;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use faraday_boot::layout;
use faraday_boot::medium::Record;
use faraday_fat::{Disk, Volume};
use faraday_files::proto::{Make, Problem, Request, Response, Stick};
use faraday_files::{Fat, Place};
use faraday_strings::EN;

/// Where a serial of a stick's own comes from: four bytes of the
/// system's randomness, or `None` when it cannot be read. It is not a
/// secret; it tells two sticks apart.
pub(crate) type Serials = Box<dyn FnMut() -> Option<u32> + Send>;

/// Four bytes from `/dev/urandom`. Costs a file read.
fn urandom() -> Option<u32> {
    use std::io::Read;
    let mut b = [0u8; 4];
    File::open("/dev/urandom").ok()?.read_exact(&mut b).ok()?;
    Some(u32::from_le_bytes(b))
}

/// A partition's device node, as a FAT volume reads it.
pub struct Node {
    file: File,
    len: u64,
}

impl Node {
    /// Opens a node read-write; refused when it was not handed out.
    pub fn open(path: &Path) -> std::io::Result<Node> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        // A block device's metadata says length 0; its end says the truth.
        let len = file.seek(SeekFrom::End(0))?;
        Ok(Node { file, len })
    }
}

impl Disk for Node {
    fn size(&self) -> u64 {
        self.len
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), faraday_fat::Error> {
        if offset.saturating_add(buf.len() as u64) > self.len {
            return Err(faraday_fat::Error::Io);
        }
        self.file
            .read_exact_at(buf, offset)
            .map_err(|_| faraday_fat::Error::Io)
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), faraday_fat::Error> {
        if offset.saturating_add(buf.len() as u64) > self.len {
            return Err(faraday_fat::Error::Io);
        }
        self.file
            .write_all_at(buf, offset)
            .map_err(|_| faraday_fat::Error::Io)
    }

    fn flush(&mut self) -> Result<(), faraday_fat::Error> {
        self.file.sync_all().map_err(|_| faraday_fat::Error::Io)
    }
}

/// Why a partition's volume is not opened.
enum Unreadable {
    /// The boot partition, which is never touched.
    Boot,
    /// Not a FAT volume Faraday reads.
    NotFat(String),
    /// The node could not be read.
    Other(String),
}

impl Unreadable {
    fn text(&self) -> String {
        match self {
            Unreadable::Boot => EN.disk_boot.into(),
            Unreadable::NotFat(e) | Unreadable::Other(e) => e.clone(),
        }
    }
}

/// Whether a sysfs flag file says 1.
fn flag(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|s| s.trim() == "1")
}

/// A flag of the disk a partition is on.
fn disk_flag(disk: Option<&Path>, name: &str) -> bool {
    disk.is_some_and(|d| flag(&d.join(name)))
}

/// A partition handed out, before its files are read.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Found {
    id: String,
    node: PathBuf,
    partname: Option<String>,
    boot: bool,
    /// Handed out: this process can open its node.
    open: bool,
    /// The kernel says the device is read-only (`ro` under `/sys`).
    read_only: bool,
    /// What the disk reports of itself under `/sys`.
    make: Make,
}

/// What a disk reports of itself under `/sys`: a USB disk's
/// `device/vendor` and `device/model`, a card's `device/name` (kept as the
/// model), and `size` in 512-byte sectors. Whitespace is trimmed; a file
/// missing, unreadable or empty is `None`. Nothing is read from the stick.
/// The device's own words are printable ASCII, as SCSI's inquiry and a
/// card's CID state them, and at most 64 of them; anything else is `None`,
/// so a device cannot put a control or direction mark on the screen.
fn make_of(disk: &Path) -> Make {
    let text = |p: PathBuf| {
        fs::read_to_string(p)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let words = |p: PathBuf| {
        text(p).filter(|s| s.len() <= 64 && s.bytes().all(|b| (0x20..=0x7e).contains(&b)))
    };
    let dev = disk.join("device");
    Make {
        maker: words(dev.join("vendor")),
        model: words(dev.join("model")).or_else(|| words(dev.join("name"))),
        size: text(disk.join("size"))
            .and_then(|s| s.parse::<u64>().ok())
            .and_then(|n| n.checked_mul(512))
            .filter(|n| *n > 0),
    }
}

/// A partition's number on its disk, as `/sys` states it.
fn number(dir: &Path) -> Option<u32> {
    fs::read_to_string(dir.join("partition"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// The `PARTNAME=` line of a block device's `uevent`.
fn partname(dir: &Path) -> Option<String> {
    let uevent = fs::read_to_string(dir.join("uevent")).ok()?;
    uevent
        .lines()
        .find_map(|l| l.strip_prefix("PARTNAME="))
        .map(str::to_string)
}

/// What a disk is called this boot: its name and the sequence number the
/// kernel gave it when it appeared, so the same stick put back is another
/// disk.
fn disk_id(disk: &Path) -> String {
    let name = disk
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let seq = fs::read_to_string(disk.join("diskseq")).unwrap_or_default();
    format!("{name}#{}", seq.trim())
}

/// The disk process: its roots, and what it last read of each partition.
pub struct Disks {
    sys: PathBuf,
    dev: PathBuf,
    /// The medium's record, and the data partition's FAT serial as the
    /// image has it (`/etc/faraday/exchange`, `/etc/faraday/data-serial`).
    record: Option<PathBuf>,
    image_serial: Option<u32>,
    serials: Serials,
    /// Each partition's listing, kept until this process writes to it:
    /// nothing else writes to a stick while it is in the machine, and a
    /// stick read once a second would show it with its light.
    seen: BTreeMap<String, Stick>,
    /// The pieces of a vault's file held for the write that follows: the
    /// stick, the name, the length the first piece said the file has and the
    /// bytes so far (`faraday_files::proto::PIECE`). A file of another name
    /// written meanwhile leaves them alone.
    held: Vec<Held>,
}

/// The pieces of one file so far.
struct Held {
    stick: String,
    name: String,
    total: u64,
    bytes: Vec<u8>,
}

impl Disks {
    /// Costs a wipe of every piece held, up to the longest vault file
    /// (about 151 MiB). Drops (wipes) every piece held: a new app process
    /// is talking, and what an earlier one sent is not part of what it
    /// writes.
    pub fn forget_held(&mut self) {
        for mut h in self.held.drain(..) {
            zeroize::Zeroize::zeroize(&mut h.bytes);
        }
    }

    /// Takes out the pieces held for this file, if any.
    fn take_held(&mut self, stick: &str, name: &str) -> Option<Held> {
        let at = self
            .held
            .iter()
            .position(|h| h.stick == stick && h.name == name)?;
        Some(self.held.remove(at))
    }

    /// The disk process over these roots: `/sys` and `/dev` on the device.
    pub fn new(sys: &Path, dev: &Path) -> Disks {
        Disks {
            sys: sys.to_path_buf(),
            dev: dev.to_path_buf(),
            record: None,
            image_serial: None,
            serials: Box::new(urandom),
            seen: BTreeMap::new(),
            held: Vec::new(),
        }
    }

    /// The same process, told which medium the machine started from by
    /// the record at `record`, and given the image's data serial: the
    /// device's. Without it only the Pi's own card is a boot medium.
    #[must_use]
    pub fn with_medium(mut self, record: &Path, image_serial: Option<u32>) -> Disks {
        self.record = Some(record.to_path_buf());
        self.image_serial = image_serial;
        self
    }

    /// The same process with another source of serials: a test's.
    #[must_use]
    pub fn with_serials(mut self, serials: Serials) -> Disks {
        self.serials = serials;
        self
    }

    /// Every partition this process may open, with its disk's identity.
    fn found(&self) -> Vec<Found> {
        let record = self.record.as_ref().map_or(Record::Waiting, |p| {
            Record::parse(&fs::read_to_string(p).unwrap_or_default())
        });
        let mut out = Vec::new();
        let Ok(entries) = fs::read_dir(self.sys.join("class/block")) else {
            return out;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Ok(dir) = fs::canonicalize(e.path()) else {
                continue;
            };
            // A partition, never a whole disk.
            if !dir.join("partition").exists() {
                continue;
            }
            let Some(disk) = dir.parent() else { continue };
            let node = self.dev.join(&name);
            let open = Node::open(&node).is_ok();
            let read_only = flag(&dir.join("ro")) || disk_flag(dir.parent(), "ro");
            // A partition not handed out is listed only when it is on a
            // removable bus: a stick or a card someone just put in.
            let path = dir.to_string_lossy();
            if !open && !path.contains("/usb") && !name.starts_with("mmcblk") {
                continue;
            }
            let siblings: Vec<PathBuf> = fs::read_dir(disk)
                .map(|d| {
                    d.flatten()
                        .map(|s| s.path())
                        .filter(|p| p.join("partition").exists())
                        .collect()
                })
                .unwrap_or_default();
            // The boot medium: the disk the record names, or the Pi's own
            // card, which is MBR and has no names.
            let seq = fs::read_to_string(disk.join("diskseq")).unwrap_or_default();
            let children: Vec<String> = siblings
                .iter()
                .filter_map(|s| s.file_name().map(|n| n.to_string_lossy().into_owned()))
                .collect();
            let boot = disk.file_name().is_some_and(|n| n == "mmcblk0")
                || record.is_boot_disk(children.iter().map(String::as_str), &seq);
            let partname = partname(&dir);
            // A boot partition, as faraday-grant knows it: by GPT name, as
            // the Pi's own card's first partition, or as the boot medium's
            // first.
            if partname.as_deref().is_some_and(layout::is_boot_name)
                || name == "mmcblk0p1"
                || (boot && number(&dir) == Some(1))
            {
                continue;
            }
            out.push(Found {
                id: format!("{name}@{}", disk_id(disk)),
                node,
                partname,
                boot,
                open,
                read_only,
                make: make_of(disk),
            });
        }
        // A disk with no partition at all is a stick or card too: a FAT
        // volume made without a partition table, or one with nothing on
        // it. Every USB and SD device is listed, so that one that cannot be
        // used says so (an exFAT stick, an NTFS one, a card that is locked).
        out.extend(self.bare_disks());
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// The disks on a USB or SD bus that have no partition and some size:
    /// each is a candidate stick, its node the whole disk.
    fn bare_disks(&self) -> Vec<Found> {
        let mut out = Vec::new();
        let Ok(entries) = fs::read_dir(self.sys.join("class/block")) else {
            return out;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Ok(dir) = fs::canonicalize(e.path()) else {
                continue;
            };
            if dir.join("partition").exists() {
                continue;
            }
            let path = dir.to_string_lossy();
            if !path.contains("/usb") && !name.starts_with("mmcblk") {
                continue;
            }
            // An empty card reader slot is a disk of no size.
            let size: u64 = fs::read_to_string(dir.join("size"))
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let has_parts = fs::read_dir(&dir)
                .is_ok_and(|d| d.flatten().any(|c| c.path().join("partition").exists()));
            if size == 0 || has_parts {
                continue;
            }
            let node = self.dev.join(&name);
            out.push(Found {
                id: format!("{name}@{}", disk_id(&dir)),
                open: Node::open(&node).is_ok(),
                node,
                partname: None,
                boot: false,
                read_only: flag(&dir.join("ro")),
                make: make_of(&dir),
            });
        }
        out
    }

    /// Opens a partition's volume, unless it is the boot partition or not
    /// FAT.
    /// Costs a read of the root directory, for its label.
    fn volume(f: &Found) -> Result<Volume<Node>, Unreadable> {
        if f.partname.as_deref().is_some_and(layout::is_boot_name) {
            return Err(Unreadable::Boot);
        }
        let mut node = Node::open(&f.node).map_err(|e| Unreadable::Other(e.to_string()))?;
        let mut sector = [0u8; 512];
        node.read_at(0, &mut sector)
            .map_err(|e| Unreadable::Other(faraday_files::fat_text(e)))?;
        if faraday_fat::boot_sector_label(&sector).is_some_and(|l| layout::is_boot_label(&l)) {
            return Err(Unreadable::Boot);
        }
        let mut v =
            Volume::open(node).map_err(|e| Unreadable::NotFat(faraday_files::fat_text(e)))?;
        // The label a person sees, which Windows writes in the root
        // directory alone.
        if v.volume_label()
            .ok()
            .flatten()
            .is_some_and(|l| layout::is_boot_label(l.as_bytes()))
        {
            return Err(Unreadable::Boot);
        }
        Ok(v)
    }

    /// Costs a read of four bytes of each file of a vault's length on a
    /// partition not listed before: one partition, as the app sees it.
    fn stick(&mut self, c: &Compute, f: &Found) -> Option<Stick> {
        let named = |f: &Found| {
            f.partname
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| f.id.split('@').next().unwrap_or("").to_string())
        };
        if !f.open {
            // Not handed out yet: nothing on it can be read or written. It
            // waits, and is told from a stick that is not there. A card
            // whose lock switch is set cannot be opened for writing at all:
            // it is locked, and never comes to be handed out.
            return Some(Stick {
                id: f.id.clone(),
                label: named(f),
                boot: f.boot,
                serial: None,
                files: Vec::new(),
                vaults: Vec::new(),
                problem: Some(if f.read_only {
                    Problem::ReadOnly
                } else {
                    Problem::NotYet
                }),
                make: f.make.clone(),
            });
        }
        if let Some(s) = self.seen.get(&f.id) {
            return Some(s.clone());
        }
        let mut v = match Self::volume(f) {
            Ok(v) => v,
            // The boot partition of a stick is never listed.
            Err(Unreadable::Boot) => return None,
            // Handed out but not a FAT volume: listed with no files, and
            // saying so, so that a stick that cannot be used is told from
            // one that is not there.
            Err(_) => {
                let s = Stick {
                    id: f.id.clone(),
                    label: named(f),
                    boot: f.boot,
                    serial: None,
                    files: Vec::new(),
                    vaults: Vec::new(),
                    problem: Some(Problem::NotFat),
                    make: f.make.clone(),
                };
                self.seen.insert(f.id.clone(), s.clone());
                return Some(s);
            }
        };
        // The boot medium's data partition with the image's serial, as
        // every stick written by another tool has it: given its own, once
        // (`docs/STICK-IDENTITY.md` §7). A stick that cannot be written
        // keeps it and is used as it is.
        if f.boot
            && !f.read_only
            && self.image_serial.is_some()
            && v.serial() == self.image_serial
            && let Some(own) = (self.serials)().filter(|s| Some(*s) != self.image_serial)
        {
            let _ = v.set_serial(own);
        }
        let label = v
            .volume_label()
            .ok()
            .flatten()
            .or_else(|| f.partname.clone().filter(|n| !n.is_empty()))
            .unwrap_or_else(|| f.id.split('@').next().unwrap_or("").to_string());
        let serial = v.serial();
        let mut place = Fat(v);
        let files = faraday_files::listing(&mut place);
        let vaults = faraday_files::vault_names(c, &mut place, &files);
        let s = Stick {
            id: f.id.clone(),
            label,
            boot: f.boot,
            serial,
            files,
            vaults,
            problem: f.read_only.then_some(Problem::ReadOnly),
            make: f.make.clone(),
        };
        self.seen.insert(f.id.clone(), s.clone());
        Some(s)
    }

    /// Every partition considered, and what became of it: a line each,
    /// for `faraday-disk --list` on a machine being brought up.
    pub fn explain(&self) -> Vec<String> {
        self.found()
            .iter()
            .map(|f| {
                let what = if !f.open {
                    // not wording: the console's line for a machine being brought up
                    "not handed out".to_string()
                } else {
                    match Self::volume(f) {
                        // not wording: the console's line, as above
                        Ok(mut v) => format!("FAT, label {:?}", v.volume_label().ok().flatten()),
                        // not wording: the console's line, as above
                        Err(e) => format!("not listed: {}", e.text()),
                    }
                };
                format!(
                    // not wording: the console's line, as above
                    "{} node {} partname {:?} boot {}: {what}",
                    f.id,
                    f.node.display(),
                    f.partname,
                    f.boot
                )
            })
            .collect()
    }

    /// Costs what [`Disks::stick`] costs for each partition not listed
    /// before: the partitions handed out now.
    pub fn list(&mut self, c: &Compute) -> Vec<Stick> {
        let found = self.found();
        self.seen
            .retain(|id, _| found.iter().any(|f| &f.id == id && f.open));
        found.iter().filter_map(|f| self.stick(c, f)).collect()
    }

    fn place(&mut self, id: &str) -> Result<Fat<Node>, String> {
        let f = self
            .found()
            .into_iter()
            .find(|f| f.id == id)
            .ok_or(EN.disk_gone)?;
        if !f.open {
            return Err(faraday_files::proto::NOT_OPEN.into());
        }
        Ok(Fat(Self::volume(&f).map_err(|e| e.text())?))
    }

    /// Costs a wipe of what was held, up to the longest vault file (about
    /// 151 MiB) when a piece at 0 starts a file again, and a copy of the
    /// piece. Holds a piece of a file to be written: a piece at 0 starts the
    /// file again, wiping what was held for it; else the bytes so far must be `at`
    /// long and `total` what the first piece said (at most the longest
    /// vault file), else what is held for the file is dropped (wiped) and
    /// the piece refused. The length held now.
    fn hold(
        &mut self,
        stick: String,
        name: String,
        at: u64,
        total: u64,
        piece: &[u8],
    ) -> Result<u64, String> {
        // A first piece starts the file again: what an earlier, cut-off
        // seal of the name left is wiped.
        let earlier = self.take_held(&stick, &name);
        let mut held = match earlier {
            Some(h) if at != 0 => h,
            earlier => {
                if let Some(mut h) = earlier {
                    zeroize::Zeroize::zeroize(&mut h.bytes);
                }
                Held {
                    stick,
                    name,
                    total,
                    bytes: Vec::new(),
                }
            }
        };
        let now = (held.bytes.len() as u64).saturating_add(piece.len() as u64);
        if held.bytes.len() as u64 != at
            || held.total != total
            || total > faraday_files::MAX_VAULT_READ
            || piece.len() > faraday_files::proto::PIECE
            || now >= total
            || held.bytes.try_reserve(piece.len()).is_err()
        {
            zeroize::Zeroize::zeroize(&mut held.bytes);
            return Err(EN.fat_io.to_string());
        }
        held.bytes.extend_from_slice(piece);
        self.held.push(held);
        Ok(now)
    }

    /// Costs what `read_qr_png` costs. Answers one request.
    #[expect(clippy::too_many_lines, reason = "one arm for each request")]
    pub fn handle(&mut self, c: &Compute, req: Request) -> Response {
        let answer = match req {
            Request::List => return Response::Sticks(self.list(c)),
            Request::Read { stick, name } => self
                .place(&stick)
                .and_then(|mut p| faraday_files::read(&mut p, &name))
                .map(Response::Bytes),
            Request::ReadPart {
                stick,
                name,
                from,
                len,
            } => self
                .place(&stick)
                .and_then(|mut p| faraday_files::read_part(&mut p, &name, from, len as usize))
                .map(|(total, bytes)| Response::Part { total, bytes }),
            Request::Piece {
                stick,
                name,
                at,
                total,
                mut bytes,
            } => {
                let held = self.hold(stick, name, at, total, &bytes);
                zeroize::Zeroize::zeroize(&mut bytes);
                held.map(Response::Held)
            }
            Request::Write {
                stick,
                name,
                copies,
                replace,
                renamed,
                on_taken,
                total,
                bytes: last,
            } => {
                // The pieces held for this file, and the last piece, are
                // the file, and exactly `total` long; a write that says
                // it is all there is drops anything held.
                let held = self.take_held(&stick, &name);
                let mut last = last;
                let mut bytes = match held {
                    // A write that is the whole file leaves nothing before it.
                    Some(mut h) if last.len() as u64 == total => {
                        zeroize::Zeroize::zeroize(&mut h.bytes);
                        last
                    }
                    Some(mut h) => {
                        let fits = h.total == total
                            && h.bytes.len() as u64 + last.len() as u64 == total
                            && h.bytes.try_reserve_exact(last.len()).is_ok();
                        if !fits {
                            zeroize::Zeroize::zeroize(&mut h.bytes);
                            zeroize::Zeroize::zeroize(&mut last);
                            return Response::Failed(EN.fat_io.to_string());
                        }
                        h.bytes.extend_from_slice(&last);
                        zeroize::Zeroize::zeroize(&mut last);
                        h.bytes
                    }
                    None if last.len() as u64 == total => last,
                    None => {
                        zeroize::Zeroize::zeroize(&mut last);
                        return Response::Failed(EN.fat_io.to_string());
                    }
                };
                let written =
                    self.place(&stick).and_then(|mut p| {
                        match faraday_files::write_file(
                            c, &mut p, &name, &bytes, &copies, replace, renamed, on_taken,
                        ) {
                            Ok(n) => Ok(Response::Written(n)),
                            Err(faraday_files::WriteRefused::Failed(why)) => Err(why),
                            Err(faraday_files::WriteRefused::NameTaken { on_stick, sha }) => {
                                Ok(Response::WriteNameTaken { on_stick, sha })
                            }
                        }
                    });
                zeroize::Zeroize::zeroize(&mut bytes);
                // What this process wrote changes the listing.
                self.seen.remove(&stick);
                written
            }
            Request::Space { stick } => self
                .place(&stick)
                .and_then(|mut p| faraday_files::Place::space(&mut p))
                .map(|(free, size)| Response::Space { free, size }),
            Request::ZeroFree { stick, from } => self
                .place(&stick)
                .and_then(|mut p| faraday_files::Place::zero_free(&mut p, from))
                .map(|z| Response::Zeroing {
                    next: z.next,
                    total: z.total,
                    done: z.done,
                }),
            Request::MakeEmpty { stick } => {
                let made = self
                    .place(&stick)
                    .and_then(|mut p| faraday_files::Place::make_empty(&mut p));
                // Every file is gone from the listing.
                self.seen.remove(&stick);
                made.map(|()| Response::Zeroing {
                    next: 2,
                    total: 0,
                    done: false,
                })
            }
            Request::Wipe { stick, name } => {
                let wiped = self
                    .place(&stick)
                    .and_then(|mut p| faraday_files::wipe(&mut p, &name));
                // What this process changed changes the listing.
                self.seen.remove(&stick);
                wiped.map(|()| Response::Wiped)
            }
            Request::Rename {
                stick,
                from,
                to,
                vaults,
            } => {
                let renamed = self
                    .place(&stick)
                    .map_err(faraday_files::Unrenamed::Failed)
                    .and_then(|mut p| faraday_files::rename(c, &mut p, &from, &to, &vaults));
                // What this process changed changes the listing.
                self.seen.remove(&stick);
                return match renamed {
                    Ok(()) => Response::Renamed,
                    Err(faraday_files::Unrenamed::Taken) => {
                        Response::NameTaken { same_vault: false }
                    }
                    Err(faraday_files::Unrenamed::SameVault) => {
                        Response::NameTaken { same_vault: true }
                    }
                    Err(faraday_files::Unrenamed::Failed(why)) => Response::Failed(why),
                };
            }
            Request::Name { stick, label } => {
                let named = self
                    .place(&stick)
                    .and_then(|mut p| name_volume(&mut p, &label));
                // The listing carries the name.
                self.seen.remove(&stick);
                named.map(|()| Response::Renamed)
            }
            Request::ReadQr { stick, name } => self
                .place(&stick)
                .and_then(|mut p| faraday_files::read_qr_png(c, &mut p as &mut dyn Place, &name))
                .map(|(bytes, codes)| Response::Qr { bytes, codes }),
        };
        answer.unwrap_or_else(Response::Failed)
    }
}

/// Costs a read and a write of the root directory: names the volume
/// `label`, or takes its name away where it is empty
/// (`docs/STICK-IDENTITY.md` §6). A name FAT cannot hold is refused, and
/// so is a boot partition's (`EFI`, `bootfs`, `FARADAYBOOT`): a volume
/// so named is never handed out again.
fn name_volume(place: &mut Fat<Node>, label: &str) -> Result<(), String> {
    let bytes = faraday_fat::label_bytes(label).map_err(faraday_files::fat_text)?;
    if bytes.is_some_and(|b| layout::is_boot_label(&b)) {
        return Err(EN.stick_name_boot.into());
    }
    place.0.set_label(bytes).map_err(faraday_files::fat_text)
}
