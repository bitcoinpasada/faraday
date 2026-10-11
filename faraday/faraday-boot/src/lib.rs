//! The boot copier's work (`PLAN.md` §5.5): find the boot partitions it
//! has been handed, read the one that holds the running Faraday, and
//! write it over another.
//!
//! A boot partition reaches this process only while the app is in the
//! upgrade flow: `faraday-grant` makes a partition named `OSKBOOT` this
//! process's, mode `0600`, while the app publishes its upgrade marker
//! beside the clean marker, and gives it back to root when either goes.
//! Here a partition is taken only if `/sys` also says it is the first of
//! exactly two on a USB disk, named `OSKBOOT` and `OSKDATA`: a Faraday
//! stick's layout (`genimage.cfg`). The GPT UUIDs `stick-boot.sh` checks
//! are in the partition table on the whole-disk node, which is never
//! handed out, so they are not checked here.
//!
//! It copies raw and reads no FAT. Apart from `/sys`, the only bytes it
//! reads from a stick are searched for a release string and kept only as
//! that string, bounded and printable:
//!
//! - The **source** is read whole into memory and accepted only if it
//!   contains the running kernel's release string, as `/proc/version`
//!   gives it, which the image build makes the Faraday version and commit
//!   (`CONFIG_LOCALVERSION`). The bzImage carries that string uncompressed
//!   in its setup code. This identifies a version; it is not a signature.
//! - A **target's** version is the first release string with
//!   [`MARK`] in it found on its boot partition, or none for a stick made
//!   before Faraday put one there (0.1.0 and earlier).
//!
//! A write is the source's bytes from the partition's first byte, flushed;
//! the node is closed, which drops the kernel's cached copy of a block
//! device on its last close, then opened again and read back and
//! compared. The data partition is another node and is never opened.
//!
//! Where `/sys`, `/dev` and `/proc/version` are is a parameter, so the
//! tests can stand a directory tree and image files in for them.

pub mod client;
pub mod proto;

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use proto::{Part, Request, Response};

/// The boot partition's GPT name.
pub const BOOT: &str = "OSKBOOT";
/// The data partition's GPT name.
pub const DATA: &str = "OSKDATA";

/// What the image build puts in the kernel's release string before the
/// Faraday version: `6.6.84-faraday-0.2.0+4d0680b1a2b3`.
pub const MARK: &[u8] = b"-faraday-";

/// The longest release string taken: the kernel's own limit.
pub const MAX_RELEASE: usize = 64;

/// The largest boot partition read or searched: the PC's is 48 MB, the
/// Pi's 32 MB.
pub const MAX_PARTITION: u64 = 64 << 20;

/// How much is read or written at a time.
const CHUNK: usize = 1 << 20;

/// How long a stick whose write failed is watched for, to tell a pull
/// from a failing stick: the kernel takes the node away a moment after
/// the I/O error.
const PULL_WAIT: Duration = Duration::from_secs(3);

/// A byte a kernel release string may hold.
fn release_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'+' | b'-' | b'_')
}

/// The running kernel's release string, from `/proc/version`'s text
/// (`Linux version 6.6.84-faraday-0.2.0+4d0680b1a2b3 (…) …`).
pub fn running_release(proc_version: &str) -> Option<String> {
    let r = proc_version
        .strip_prefix("Linux version ")?
        .split(' ')
        .next()?;
    (!r.is_empty() && r.len() <= MAX_RELEASE && r.bytes().all(release_byte)).then(|| r.to_string())
}

/// Whether `hay` holds `release` as a whole string: not inside a longer
/// one.
pub fn holds(hay: &[u8], release: &str) -> bool {
    let n = release.as_bytes();
    let Some(&first) = n.first() else {
        return false;
    };
    if hay.len() < n.len() {
        return false;
    }
    let last = hay.len() - n.len();
    let mut i = 0;
    while i <= last {
        match hay[i..=last].iter().position(|&b| b == first) {
            None => return false,
            Some(k) => i += k,
        }
        if &hay[i..i + n.len()] == n
            && (i == 0 || !release_byte(hay[i - 1]))
            && hay.get(i + n.len()).is_none_or(|&b| !release_byte(b))
        {
            return true;
        }
        i += 1;
    }
    false
}

/// The first Faraday release string in `hay`: a whole string of release
/// bytes with [`MARK`] in it, starting with a digit, no longer than
/// [`MAX_RELEASE`], and ending inside `hay`.
pub fn find_release(hay: &[u8]) -> Option<String> {
    let mut from = 0;
    while from + MARK.len() <= hay.len() {
        let at = from + hay[from..].windows(MARK.len()).position(|w| w == MARK)?;
        let mut start = at;
        while start > 0 && at - start < MAX_RELEASE && release_byte(hay[start - 1]) {
            start -= 1;
        }
        let mut end = at + MARK.len();
        while end < hay.len() && end - start <= MAX_RELEASE && release_byte(hay[end]) {
            end += 1;
        }
        let whole = (start == 0 || !release_byte(hay[start - 1]))
            && end < hay.len()
            && !release_byte(hay[end])
            && end - start <= MAX_RELEASE;
        if whole && hay[start].is_ascii_digit() && end > at + MARK.len() {
            return String::from_utf8(hay[start..end].to_vec()).ok();
        }
        from = at + 1;
    }
    None
}

/// A boot partition handed to this process.
#[derive(Debug, Clone)]
struct Found {
    id: String,
    node: PathBuf,
}

/// The source, read.
struct Source {
    id: String,
    release: String,
    bytes: Vec<u8>,
}

/// The `PARTNAME=` line of a block device's `uevent`.
fn partname(dir: &Path) -> Option<String> {
    fs::read_to_string(dir.join("uevent"))
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("PARTNAME="))
        .map(str::to_string)
}

/// A partition's number, from its `partition` attribute.
fn number(dir: &Path) -> Option<u32> {
    fs::read_to_string(dir.join("partition"))
        .ok()?
        .trim()
        .parse()
        .ok()
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

/// A whole node's length: a block device's metadata says 0.
fn length(f: &mut File) -> std::io::Result<u64> {
    f.seek(SeekFrom::End(0))
}

fn mb(bytes: u64) -> u64 {
    bytes.div_ceil(1 << 20)
}

/// The boot copier: its roots, the source once read, and the version
/// found on each partition it has looked at.
pub struct Boots {
    sys: PathBuf,
    dev: PathBuf,
    version: PathBuf,
    source: Option<Source>,
    /// Each partition's release string, kept while it stays in: nothing
    /// else writes to a boot partition, and a stick searched once a
    /// second would show it with its light.
    seen: BTreeMap<String, Option<String>>,
}

impl Boots {
    /// The copier over these roots: `/sys`, `/dev` and `/proc/version` on
    /// the device.
    pub fn new(sys: &Path, dev: &Path, version: &Path) -> Boots {
        Boots {
            sys: sys.to_path_buf(),
            dev: dev.to_path_buf(),
            version: version.to_path_buf(),
            source: None,
            seen: BTreeMap::new(),
        }
    }

    /// Every boot partition of a Faraday stick on USB that this process
    /// may open, by id.
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
            // A partition named OSKBOOT, the first, on a USB disk. The
            // Pi's own card (`mmcblk0p1`, no names) comes later
            // (`PLAN.md` §5.5).
            if !dir.to_string_lossy().contains("/usb")
                || number(&dir) != Some(1)
                || partname(&dir).as_deref() != Some(BOOT)
            {
                continue;
            }
            let Some(disk) = dir.parent() else { continue };
            let mut parts: Vec<(u32, Option<String>)> = fs::read_dir(disk)
                .map(|d| {
                    d.flatten()
                        .map(|s| s.path())
                        .filter(|p| p.join("partition").exists())
                        .map(|p| (number(&p).unwrap_or(0), partname(&p)))
                        .collect()
                })
                .unwrap_or_default();
            parts.sort();
            let faraday = parts.len() == 2
                && parts[0] == (1, Some(BOOT.to_string()))
                && parts[1] == (2, Some(DATA.to_string()));
            if !faraday {
                continue;
            }
            let node = self.dev.join(&name);
            // Handed out: this process can open it.
            if File::open(&node).is_err() {
                continue;
            }
            out.push(Found {
                id: format!("{name}@{}", disk_id(disk)),
                node,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// The release string on a partition: the first found in its first
    /// [`MAX_PARTITION`] bytes, read a chunk at a time.
    fn search(node: &Path) -> std::io::Result<Option<String>> {
        let mut f = File::open(node)?;
        let len = length(&mut f)?.min(MAX_PARTITION);
        // Each chunk overlaps the next by a release string's length, so
        // one that straddles two is still seen whole.
        let overlap = MAX_RELEASE * 2;
        let mut buf = vec![0u8; CHUNK + overlap];
        let mut at = 0u64;
        while at < len {
            let n = (len - at).min(buf.len() as u64) as usize;
            f.read_exact_at(&mut buf[..n], at)?;
            if let Some(r) = find_release(&buf[..n]) {
                return Ok(Some(r));
            }
            at += CHUNK as u64;
        }
        Ok(None)
    }

    /// The boot partitions handed out now, with their versions.
    pub fn list(&mut self) -> Vec<Part> {
        let found = self.found();
        self.seen.retain(|id, _| found.iter().any(|f| &f.id == id));
        let mut parts = Vec::new();
        for f in found {
            let Ok(size) = File::open(&f.node).and_then(|mut h| length(&mut h)) else {
                continue;
            };
            let source = self.source.as_ref().is_some_and(|s| s.id == f.id);
            let release = match self.seen.get(&f.id) {
                Some(r) => r.clone(),
                None => {
                    let Ok(r) = Self::search(&f.node) else {
                        continue;
                    };
                    self.seen.insert(f.id.clone(), r.clone());
                    r
                }
            };
            parts.push(Part {
                id: f.id,
                size,
                release,
                source,
            });
        }
        parts
    }

    /// Reads the boot partition that holds the running Faraday.
    pub fn read_source(&mut self) -> Response {
        self.source = None;
        let running = fs::read_to_string(&self.version)
            .ok()
            .and_then(|v| running_release(&v));
        let marked = |r: &String| r.as_bytes().windows(MARK.len()).any(|w| w == MARK);
        let Some(release) = running.filter(marked) else {
            return Response::Failed("This Faraday's kernel carries no Faraday version".into());
        };
        let found = self.found();
        if found.is_empty() {
            return Response::Failed("No Faraday stick is in".into());
        }
        for f in found {
            let Ok(mut h) = File::open(&f.node) else {
                continue;
            };
            let Ok(len) = length(&mut h) else { continue };
            if len > MAX_PARTITION {
                continue;
            }
            let mut bytes = vec![0u8; len as usize];
            if h.read_exact_at(&mut bytes, 0).is_err() {
                continue;
            }
            if holds(&bytes, &release) {
                self.seen.insert(f.id.clone(), Some(release.clone()));
                let size = bytes.len() as u64;
                self.source = Some(Source {
                    id: f.id.clone(),
                    release: release.clone(),
                    bytes,
                });
                return Response::Source {
                    id: f.id,
                    release,
                    size,
                };
            }
        }
        Response::Failed("No stick in holds this Faraday".into())
    }

    /// Whether the partition is still in, watched for a moment.
    fn still_in(&self, id: &str) -> bool {
        let until = Instant::now() + PULL_WAIT;
        loop {
            if !self.found().iter().any(|f| f.id == id) {
                return false;
            }
            if Instant::now() >= until {
                return true;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// What a failed write or read-back means: a pull, or a stick that
    /// fails.
    fn failed(&self, id: &str, e: &std::io::Error) -> Response {
        if self.still_in(id) {
            Response::Failed(format!("The stick failed: {e}"))
        } else {
            Response::Pulled
        }
    }

    /// Writes the source over the boot partition `target`, reads it back
    /// and compares.
    pub fn write(&mut self, target: &str) -> Response {
        let Some(src) = self.source.as_ref() else {
            return Response::Failed("The stick Faraday started from has not been read".into());
        };
        if src.id == target {
            return Response::Failed("That is the stick being copied".into());
        }
        let Some(f) = self.found().into_iter().find(|f| f.id == target) else {
            return Response::Failed("The stick is gone".into());
        };
        let mut file = match OpenOptions::new().read(true).write(true).open(&f.node) {
            Ok(h) => h,
            Err(e) => return Response::Failed(format!("The stick cannot be opened: {e}")),
        };
        let len = match length(&mut file) {
            Ok(n) => n,
            Err(e) => return self.failed(target, &e),
        };
        let release = src.release.clone();
        let need = src.bytes.len() as u64;
        if len < need {
            return Response::Failed(format!(
                "Its boot partition is {} MB; this Faraday's is {} MB",
                mb(len),
                mb(need)
            ));
        }
        let mut at = 0usize;
        while at < src.bytes.len() {
            let end = (at + CHUNK).min(src.bytes.len());
            if let Err(e) = file.write_all_at(&src.bytes[at..end], at as u64) {
                return self.failed(target, &e);
            }
            at = end;
        }
        if let Err(e) = file.sync_all() {
            return self.failed(target, &e);
        }
        drop(file);
        // Opened again, so what is read comes from the stick.
        let mut back = match File::open(&f.node) {
            Ok(h) => h,
            Err(e) => return self.failed(target, &e),
        };
        let bytes = &src.bytes;
        let mut buf = vec![0u8; CHUNK];
        let mut at = 0usize;
        while at < bytes.len() {
            let n = CHUNK.min(bytes.len() - at);
            if let Err(e) = back.read_exact(&mut buf[..n]) {
                return self.failed(target, &e);
            }
            if buf[..n] != bytes[at..at + n] {
                return Response::Failed("What was read back differs from what was written".into());
            }
            at += n;
        }
        self.seen.insert(target.to_string(), Some(release.clone()));
        Response::Written {
            id: target.to_string(),
            release,
        }
    }

    /// Answers one request.
    pub fn handle(&mut self, req: Request) -> Response {
        match req {
            Request::List => Response::Parts(self.list()),
            Request::ReadSource => self.read_source(),
            Request::Write { target } => self.write(&target),
            Request::Forget => {
                self.source = None;
                Response::Forgotten
            }
        }
    }

    /// Every boot partition handed out, with what was found on it: a line
    /// each, for `faraday-boot --list` on a machine being brought up.
    pub fn explain(&mut self) -> Vec<String> {
        let running = fs::read_to_string(&self.version)
            .ok()
            .and_then(|v| running_release(&v));
        let mut lines = vec![format!(
            "running {}",
            running.as_deref().unwrap_or("unknown")
        )];
        for p in self.list() {
            lines.push(format!(
                "{} {} bytes release {}",
                p.id,
                p.size,
                p.release.as_deref().unwrap_or("none")
            ));
        }
        lines
    }
}
