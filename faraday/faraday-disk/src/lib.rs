//! The disk process's work (`docs/history/PLAN.md` §4.3): find the partitions it has
//! been handed, and answer one request about them.
//!
//! A partition is handed out by `faraday-grant` making its device node
//! this process's, mode `0600`. A USB or SD partition this process cannot
//! open was not handed out: it is listed with no files, so the app knows
//! a stick is there and asks to lock (`docs/history/PLAN.md` §5.4). Nothing here mounts anything: the
//! filesystem is read by `faraday-fat`. A partition named or labelled
//! `FARADAYBOOT` is never touched, even if it could be opened: the grant
//! helper keeps it back, and this is the second lock on the same door.
//!
//! Where `/sys` and `/dev` are is a parameter, so the tests can stand a
//! directory tree and image files in for them.

use faraday_crypto::work::Compute;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use faraday_fat::{Disk, Volume};
use faraday_files::proto::{Problem, Request, Response, Stick};
use faraday_files::{Fat, Place};

/// The label and partition name the boot partition carries.
pub const BOOT: &str = "FARADAYBOOT";

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
            Unreadable::Boot => "the boot partition is never opened".into(),
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
    /// Each partition's listing, kept until this process writes to it:
    /// nothing else writes to a stick while it is in the machine, and a
    /// stick read once a second would show it with its light.
    seen: BTreeMap<String, Stick>,
}

impl Disks {
    /// The disk process over these roots: `/sys` and `/dev` on the device.
    pub fn new(sys: &Path, dev: &Path) -> Disks {
        Disks {
            sys: sys.to_path_buf(),
            dev: dev.to_path_buf(),
            seen: BTreeMap::new(),
        }
    }

    /// Every partition this process may open, with its disk's identity.
    fn found(&self) -> Vec<Found> {
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
            // The boot medium: a disk with a partition named FARADAYBOOT, or
            // the Pi's own card, which is MBR and has no names.
            let boot = disk.file_name().is_some_and(|n| n == "mmcblk0")
                || siblings
                    .iter()
                    .any(|s| partname(s).as_deref() == Some(BOOT));
            let partname = partname(&dir);
            // The boot partition, as faraday-grant knows it: by GPT name,
            // or as the Pi's own card's first partition.
            if partname.as_deref() == Some(BOOT) || name == "mmcblk0p1" {
                continue;
            }
            out.push(Found {
                id: format!("{name}@{}", disk_id(disk)),
                node,
                partname,
                boot,
                open,
                read_only,
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
            });
        }
        out
    }

    /// Opens a partition's volume, unless it is the boot partition or not
    /// FAT.
    fn volume(f: &Found) -> Result<Volume<Node>, Unreadable> {
        if f.partname.as_deref() == Some(BOOT) {
            return Err(Unreadable::Boot);
        }
        let mut node = Node::open(&f.node).map_err(|e| Unreadable::Other(e.to_string()))?;
        let mut sector = [0u8; 512];
        node.read_at(0, &mut sector)
            .map_err(|e| Unreadable::Other(e.reason().to_string()))?;
        if faraday_fat::boot_sector_label(&sector).is_some_and(|l| l.starts_with(BOOT.as_bytes())) {
            return Err(Unreadable::Boot);
        }
        Volume::open(node).map_err(|e| Unreadable::NotFat(e.reason().to_string()))
    }

    fn stick(&mut self, f: &Found) -> Option<Stick> {
        let named = |f: &Found| {
            f.partname
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| f.id.split('@').next().unwrap_or("").to_string())
        };
        if !f.open {
            // Not handed out yet: nothing on it can be read or written. It
            // waits, and is told from a stick that is not there.
            return Some(Stick {
                id: f.id.clone(),
                label: named(f),
                boot: f.boot,
                serial: None,
                files: Vec::new(),
                problem: Some(Problem::NotYet),
            });
        }
        if let Some(s) = self.seen.get(&f.id) {
            return Some(s.clone());
        }
        let v = match Self::volume(f) {
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
                    problem: Some(Problem::NotFat),
                };
                self.seen.insert(f.id.clone(), s.clone());
                return Some(s);
            }
        };
        let label = v
            .label()
            .or_else(|| f.partname.clone())
            .unwrap_or_else(|| f.id.split('@').next().unwrap_or("").to_string());
        let serial = v.serial();
        let files = faraday_files::listing(&mut Fat(v));
        let s = Stick {
            id: f.id.clone(),
            label,
            boot: f.boot,
            serial,
            files,
            problem: f.read_only.then_some(Problem::ReadOnly),
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
                    "not handed out".to_string()
                } else {
                    match Self::volume(f) {
                        Ok(v) => format!("FAT, label {:?}", v.label()),
                        Err(e) => format!("not listed: {}", e.text()),
                    }
                };
                format!(
                    "{} node {} partname {:?} boot {}: {what}",
                    f.id,
                    f.node.display(),
                    f.partname,
                    f.boot
                )
            })
            .collect()
    }

    /// The partitions handed out now.
    pub fn list(&mut self) -> Vec<Stick> {
        let found = self.found();
        self.seen
            .retain(|id, _| found.iter().any(|f| &f.id == id && f.open));
        found.iter().filter_map(|f| self.stick(f)).collect()
    }

    fn place(&mut self, id: &str) -> Result<Fat<Node>, String> {
        let f = self
            .found()
            .into_iter()
            .find(|f| f.id == id)
            .ok_or("the stick is gone")?;
        if !f.open {
            return Err("the stick is not open to Faraday until it locks".into());
        }
        Ok(Fat(Self::volume(&f).map_err(|e| e.text())?))
    }

    /// Costs what `read_qr_png` costs. Answers one request.
    pub fn handle(&mut self, c: &Compute, req: Request) -> Response {
        let answer = match req {
            Request::List => return Response::Sticks(self.list()),
            Request::Read { stick, name } => self
                .place(&stick)
                .and_then(|mut p| faraday_files::read(&mut p, &name))
                .map(Response::Bytes),
            Request::Write {
                stick,
                name,
                replace,
                renamed,
                mut bytes,
            } => {
                let written = self.place(&stick).and_then(|mut p| {
                    faraday_files::write_file(&mut p, &name, &bytes, replace, renamed)
                });
                zeroize::Zeroize::zeroize(&mut bytes);
                // What this process wrote changes the listing.
                self.seen.remove(&stick);
                written.map(Response::Written)
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
            Request::Rename { stick, from, to } => {
                let renamed = self
                    .place(&stick)
                    .map_err(faraday_files::Unrenamed::Failed)
                    .and_then(|mut p| faraday_files::rename(&mut p, &from, &to));
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
            Request::ReadQr { stick, name } => self
                .place(&stick)
                .and_then(|mut p| faraday_files::read_qr_png(c, &mut p as &mut dyn Place, &name))
                .map(|(bytes, codes)| Response::Qr { bytes, codes }),
        };
        answer.unwrap_or_else(Response::Failed)
    }
}
