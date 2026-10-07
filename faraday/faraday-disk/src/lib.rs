//! The disk process's work (`PLAN.md` §4.3): find the partitions it has
//! been handed, and answer one request about them.
//!
//! A partition is handed out by `faraday-grant` making its device node
//! this process's, mode `0600`. A USB or SD partition this process cannot
//! open was not handed out: it is listed with no files, so the app knows
//! a stick is there and asks to lock (`PLAN.md` §5.4). Nothing here mounts anything: the
//! filesystem is read by `faraday-fat`. A partition named or labelled
//! `OSKBOOT` is never touched, even if it could be opened: the grant
//! helper keeps it back, and this is the second lock on the same door.
//!
//! Where `/sys` and `/dev` are is a parameter, so the tests can stand a
//! directory tree and image files in for them.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use faraday_fat::{Disk, Volume};
use faraday_files::proto::{Request, Response, Stick};
use faraday_files::{Fat, Place};

/// The label and partition name the boot partition carries.
pub const BOOT: &str = "OSKBOOT";

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

/// A partition handed out, before its files are read.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Found {
    id: String,
    node: PathBuf,
    partname: Option<String>,
    boot: bool,
    /// Handed out: this process can open its node.
    open: bool,
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
            // The boot medium: a disk with a partition named OSKBOOT, or
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
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// Opens a partition's volume, unless it is the boot partition or not
    /// FAT.
    fn volume(f: &Found) -> Result<Volume<Node>, String> {
        if f.partname.as_deref() == Some(BOOT) {
            return Err("the boot partition is never opened".into());
        }
        let mut node = Node::open(&f.node).map_err(|e| e.to_string())?;
        let mut sector = [0u8; 512];
        node.read_at(0, &mut sector)
            .map_err(|e| e.reason().to_string())?;
        if faraday_fat::boot_sector_label(&sector).is_some_and(|l| l.starts_with(BOOT.as_bytes())) {
            return Err("the boot partition is never opened".into());
        }
        Volume::open(node).map_err(|e| e.reason().to_string())
    }

    fn stick(&mut self, f: &Found) -> Option<Stick> {
        if !f.open {
            return Some(Stick {
                id: f.id.clone(),
                label: f
                    .partname
                    .clone()
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| f.id.split('@').next().unwrap_or("").to_string()),
                boot: f.boot,
                files: Vec::new(),
            });
        }
        if let Some(s) = self.seen.get(&f.id) {
            return Some(s.clone());
        }
        let v = Self::volume(f).ok()?;
        let label = v
            .label()
            .or_else(|| f.partname.clone())
            .unwrap_or_else(|| f.id.split('@').next().unwrap_or("").to_string());
        let files = faraday_files::listing(&mut Fat(v));
        let s = Stick {
            id: f.id.clone(),
            label,
            boot: f.boot,
            files,
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
                        Err(e) => format!("not listed: {e}"),
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
        Ok(Fat(Self::volume(&f)?))
    }

    /// Answers one request.
    pub fn handle(&mut self, req: Request) -> Response {
        let answer = match req {
            Request::List => return Response::Sticks(self.list()),
            Request::Read { stick, name } => self
                .place(&stick)
                .and_then(|mut p| faraday_files::read(&mut p, &name))
                .map(Response::Bytes),
            Request::Write { stick, name, bytes } => {
                let written = self
                    .place(&stick)
                    .and_then(|mut p| faraday_files::write_any(&mut p, &name, &bytes));
                // What this process wrote changes the listing.
                self.seen.remove(&stick);
                written.map(Response::Written)
            }
            Request::ReadQr { stick, name } => self
                .place(&stick)
                .and_then(|mut p| faraday_files::read_qr_png(&mut p as &mut dyn Place, &name))
                .map(Response::Qr),
        };
        answer.unwrap_or_else(Response::Failed)
    }
}
