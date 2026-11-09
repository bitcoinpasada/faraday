//! The file side of a stick, shared by Faraday's shells.
//!
//! A stick is a directory: a mounted FAT partition on the device, a plain
//! folder on a desktop. Each shell finds its sticks its own way and hands
//! the directories here; this crate lists them, reads a file the app asked
//! for, writes one as `docs/history/PLAN.md` §4.3 says (a new file, read back and
//! compared, then renamed into place, never over a file already there),
//! and keeps the Inbox and Outbox where the next process finds them.

use std::fs;
use std::path::{Path, PathBuf};

use faraday_core::{Faraday, StickInfo, StorageCommand, StorageEvent};
use faraday_files::{Dir, checked};
use faraday_strings::{EN, fill};

pub use faraday_files::{MAX_PIXELS, MAX_READ, OnTaken, Unrenamed, WriteRefused, foreign};

/// Costs a look at the folder and a read of four bytes of each file
/// that could be a vault: what a stick directory holds at its top level,
/// as the app sees it.
pub fn stick_info(
    c: &faraday_crypto::work::Compute,
    dir: &Path,
    label: &str,
    boot: bool,
) -> StickInfo {
    stick_info_after(c, &[], dir, label, boot)
}

/// Costs what [`stick_info`] costs: a read of four bytes of each file of
/// a vault's length that `before`, the listing made last, did not tell
/// apart under the same name and size.
pub fn stick_info_after(
    c: &faraday_crypto::work::Compute,
    before: &[StickInfo],
    dir: &Path,
    label: &str,
    boot: bool,
) -> StickInfo {
    let id = dir.display().to_string();
    let mut place = Dir(dir.to_path_buf());
    let files = faraday_files::listing(&mut place);
    let vaults = match before.iter().find(|s| s.id == id && s.files == files) {
        Some(s) => s.vaults.clone(),
        None => faraday_files::vault_names(c, &mut place, &files),
    };
    StickInfo {
        make: Default::default(),
        id,
        label: label.to_string(),
        boot,
        serial: None,
        files,
        vaults,
    }
}

fn dir(stick: &str) -> Dir {
    Dir(PathBuf::from(stick))
}

/// Reads one file from a stick directory.
pub fn read(stick: &str, name: &str) -> Result<Vec<u8>, String> {
    faraday_files::read_whole(&mut dir(stick), name)
}

/// Writes a file to a stick directory as a new file, reads it back,
/// compares, and renames it into place. Returns the name it was written
/// under.
pub fn write(stick: &str, name: &str, bytes: &[u8]) -> Result<String, String> {
    faraday_files::write(&mut dir(stick), name, bytes)
}

/// Costs what `faraday_files::write_vault` costs. Writes a sealed vault
/// back over its own file, the one whose SHA-256 is among `copies`
/// (`docs/VAULT.md` §6).
pub fn write_vault(
    c: &faraday_crypto::work::Compute,
    stick: &str,
    name: &str,
    bytes: &[u8],
    copies: &[[u8; 32]],
) -> Result<String, String> {
    faraday_files::write_vault(c, &mut dir(stick), name, bytes, copies)
}

/// Finishes a vault write-back a pulled stick interrupted.
pub fn finish_vault_writes(stick: &Path) {
    faraday_files::finish_vault_writes(&mut Dir(stick.to_path_buf()));
}

/// Costs a read of the file and a PNG decode and QR search
/// (`faraday_files::qr_in_png`): a PNG in a stick directory, its bytes
/// and its QR codes.
pub fn read_qr_png(
    c: &faraday_crypto::work::Compute,
    stick: &str,
    name: &str,
) -> Result<faraday_files::Picture, String> {
    faraday_files::read_qr_png(c, &mut dir(stick), name)
}

/// Where the sticks' files are: folders, or the disk process.
pub trait Sticks {
    /// One file's bytes.
    fn read(&mut self, stick: &str, name: &str) -> Result<Vec<u8>, String>;
    /// Costs what `faraday_files::write_file` costs where the sticks are
    /// folders. A file written, over a file of its name when `replace`;
    /// the name it went under. `copies`: the SHA-256 of the copies of a
    /// vault the session has seen. `renamed`: the person renamed it For
    /// the stick, and a vault going over a copy takes the name
    /// (`faraday_files::write_file`). `on_taken`: what a vault does where
    /// a file has its name and no copy is proved.
    #[expect(
        clippy::too_many_arguments,
        reason = "one write, as the protocol carries it"
    )]
    fn write(
        &mut self,
        c: &faraday_crypto::work::Compute,
        stick: &str,
        name: &str,
        bytes: &[u8],
        copies: &[[u8; 32]],
        replace: bool,
        renamed: bool,
        on_taken: faraday_files::OnTaken,
    ) -> Result<String, faraday_files::WriteRefused>;
    /// Starts a transfer that takes long, a vault in pieces, off the
    /// shell's thread (F206 point 7); the answer comes from
    /// [`Sticks::transferred`] on a later pass. Where this place does not
    /// do that, or the transfer is short, it is handed back to be done now.
    #[expect(clippy::result_large_err, reason = "the transfer, handed back whole")]
    fn transfer_later(&mut self, transfer: Transfer) -> Result<(), Transfer> {
        Err(transfer)
    }
    /// The transfers finished since the last call; and the next one asked
    /// is started, one piece at a time.
    fn transferred(&mut self) -> Vec<Transferred> {
        Vec::new()
    }
    /// Starts the read of a vault for its SHA-256 off the shell's thread;
    /// false where this place does not, and the file is read now.
    fn hash_later(&mut self, _stick: &str, _name: &str) -> bool {
        false
    }
    /// Whether the finished read of this file was asked by
    /// [`Sticks::hash_later`] (and forgets it).
    fn hash_taken(&mut self, _stick: &str, _name: &str) -> bool {
        false
    }
    /// Deletes a file, zeros over it first.
    fn wipe(&mut self, stick: &str, name: &str) -> Result<(), String>;
    /// The free space and the whole data space, in bytes.
    fn space(&mut self, stick: &str) -> Result<(u64, u64), String>;
    /// One step of zeros over the free space: the next cluster, of how
    /// many, and whether it is done.
    fn zero_free(&mut self, stick: &str, from: u32) -> Result<(u32, u32, bool), String>;
    /// Every file gone, the volume empty, its layout and label kept.
    fn make_empty(&mut self, stick: &str) -> Result<(), String>;
    /// Starts reading a picture's bytes and QR codes off the shell's
    /// thread; the answer comes from [`Sticks::pictures_read`] on a later
    /// pass. Pictures are read one at a time, in the order asked.
    fn read_qr_later(&mut self, stick: &str, name: &str);
    /// The pictures read since the last call, each with its stick and
    /// name; and the next picture asked is started.
    fn pictures_read(&mut self) -> Vec<PictureRead>;
    /// Costs what `faraday_files::rename` costs where the sticks are
    /// folders. A file renamed in place; refused, nothing written, where
    /// another file has the name (`faraday_files::rename`). `vaults`: the
    /// SHA-256 of the copies of each vault the session has seen.
    fn rename(
        &mut self,
        c: &faraday_crypto::work::Compute,
        stick: &str,
        from: &str,
        to: &str,
        vaults: &[Vec<[u8; 32]>],
    ) -> Result<(), faraday_files::Unrenamed>;
    /// The volume named `label`, or its name taken away where it is
    /// empty (`docs/STICK-IDENTITY.md` §6).
    fn name(&mut self, stick: &str, label: &str) -> Result<(), String>;
}

/// A file read or written, as the app asked it.
pub enum Transfer {
    /// A file read.
    Read {
        /// The stick.
        stick: String,
        /// The file's name.
        name: String,
    },
    /// A file written, as [`Sticks::write`] says.
    Write {
        /// The stick.
        stick: String,
        /// The file's name.
        name: String,
        /// The file.
        bytes: Vec<u8>,
        /// The SHA-256 of the copies of a vault the session has seen.
        copies: Vec<[u8; 32]>,
        /// Over a file of its name.
        replace: bool,
        /// The person renamed it.
        renamed: bool,
        /// What a vault does where a file has its name.
        on_taken: faraday_files::OnTaken,
    },
}

/// A [`Transfer`] finished: its stick and name, and its result.
pub enum Transferred {
    /// A file read.
    Read {
        /// The stick.
        stick: String,
        /// The file's name.
        name: String,
        /// The bytes, or why not.
        result: Result<Vec<u8>, String>,
    },
    /// A file written.
    Write {
        /// The stick.
        stick: String,
        /// The name asked.
        name: String,
        /// The name it went under, or why not.
        result: Result<String, faraday_files::WriteRefused>,
    },
}

/// Costs what `Sticks::write` costs for a write, a read of the file for a
/// read. Does `t` now, on this thread.
fn transfer_now(
    c: &faraday_crypto::work::Compute,
    sticks: &mut dyn Sticks,
    t: Transfer,
) -> Transferred {
    match t {
        Transfer::Read { stick, name } => {
            let result = sticks.read(&stick, &name);
            Transferred::Read {
                stick,
                name,
                result,
            }
        }
        Transfer::Write {
            stick,
            name,
            bytes,
            copies,
            replace,
            renamed,
            on_taken,
        } => {
            let result = sticks.write(
                c, &stick, &name, &bytes, &copies, replace, renamed, on_taken,
            );
            Transferred::Write {
                stick,
                name,
                result,
            }
        }
    }
}

/// A picture read for its codes: its stick, its name, and its bytes and
/// codes or why it was not read.
pub type PictureRead = (String, String, Result<faraday_files::Picture, String>);

/// Pictures in folders read for their QR codes on a thread of their own,
/// one at a time in the order asked (`docs/SPEED.md` §9): a photograph
/// takes a third of a second on a desktop and seconds on a Pi, and the
/// frames go on being drawn meanwhile. The thread starts with the first
/// picture asked. Dropped, it reads no more: the picture it is reading
/// is finished and let go.
#[derive(Default)]
pub struct Pictures {
    worker: Option<Worker>,
}

/// The pictures' thread, as the shell reaches it: where pictures are
/// asked for, and where they come back read.
type Worker = (
    std::sync::mpsc::Sender<(String, String)>,
    std::sync::mpsc::Receiver<PictureRead>,
);

impl Pictures {
    /// Asks for the picture `name` in the folder `stick`.
    pub fn ask(&mut self, stick: &str, name: &str) {
        let (jobs, _) = self.worker.get_or_insert_with(|| {
            let (jobs, asked) = std::sync::mpsc::channel::<(String, String)>();
            let (read, done) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                // The thread's token, for the pictures it reads
                // (docs/COMPUTE.md §3). It draws nothing, so it counts
                // nothing.
                #[expect(
                    clippy::disallowed_methods,
                    reason = "a mint: the pictures' thread, which draws nothing (docs/COMPUTE.md §3)"
                )]
                let c = faraday_crypto::work::Compute::new(|_| {});
                for (stick, name) in asked {
                    let picture = read_qr_png(&c, &stick, &name);
                    if read.send((stick, name, picture)).is_err() {
                        return;
                    }
                }
            });
            (jobs, done)
        });
        // The thread ends only when this end is dropped.
        let _ = jobs.send((stick.to_string(), name.to_string()));
    }

    /// The pictures read since the last call.
    pub fn read(&mut self) -> Vec<PictureRead> {
        self.worker
            .as_ref()
            .map(|(_, done)| done.try_iter().collect())
            .unwrap_or_default()
    }
}

/// Sticks that are folders, named by their paths; their pictures read
/// on a thread of their own ([`Pictures`]).
#[derive(Default)]
pub struct Dirs {
    pictures: Pictures,
}

impl Sticks for Dirs {
    fn read(&mut self, stick: &str, name: &str) -> Result<Vec<u8>, String> {
        read(stick, name)
    }
    fn write(
        &mut self,
        c: &faraday_crypto::work::Compute,
        stick: &str,
        name: &str,
        bytes: &[u8],
        copies: &[[u8; 32]],
        replace: bool,
        renamed: bool,
        on_taken: faraday_files::OnTaken,
    ) -> Result<String, faraday_files::WriteRefused> {
        faraday_files::write_file(
            c,
            &mut dir(stick),
            name,
            bytes,
            copies,
            replace,
            renamed,
            on_taken,
        )
    }
    fn wipe(&mut self, stick: &str, name: &str) -> Result<(), String> {
        faraday_files::wipe(&mut dir(stick), name)
    }
    fn space(&mut self, stick: &str) -> Result<(u64, u64), String> {
        faraday_files::Place::space(&mut dir(stick))
    }
    fn zero_free(&mut self, stick: &str, from: u32) -> Result<(u32, u32, bool), String> {
        faraday_files::Place::zero_free(&mut dir(stick), from).map(|z| (z.next, z.total, z.done))
    }
    fn make_empty(&mut self, stick: &str) -> Result<(), String> {
        faraday_files::Place::make_empty(&mut dir(stick))
    }
    fn read_qr_later(&mut self, stick: &str, name: &str) {
        self.pictures.ask(stick, name);
    }
    fn pictures_read(&mut self) -> Vec<PictureRead> {
        self.pictures.read()
    }
    fn rename(
        &mut self,
        c: &faraday_crypto::work::Compute,
        stick: &str,
        from: &str,
        to: &str,
        vaults: &[Vec<[u8; 32]>],
    ) -> Result<(), faraday_files::Unrenamed> {
        faraday_files::rename(c, &mut dir(stick), from, to, vaults)
    }
    fn name(&mut self, _stick: &str, _label: &str) -> Result<(), String> {
        Err(EN.storage_name_folder.to_string())
    }
}

/// Costs a run of `/usr/bin/vm_stat` on macOS, a file read elsewhere.
/// The memory this machine has free for a vault's Argon2id, in MiB, from
/// the system's own estimate: Linux's `MemAvailable`, Windows' available
/// physical memory, macOS's free, inactive and speculative pages as
/// `vm_stat` prints them ([`vm_stat_available_mib`]). `None` where it
/// cannot be read: the vault then unlocks and makes only vaults of
/// Light's memory, which opens anywhere (`docs/VAULT.md` §3.1).
pub fn memory_available_mib() -> Option<u32> {
    #[cfg(windows)]
    {
        windows_memory::available_mib()
    }
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("/usr/bin/vm_stat")
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        vm_stat_available_mib(std::str::from_utf8(&out.stdout).ok()?)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let info = fs::read_to_string("/proc/meminfo").ok()?;
        let line = info.lines().find(|l| l.starts_with("MemAvailable:"))?;
        let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        u32::try_from(kib / 1024).ok()
    }
}

/// The memory free for a vault in `vm_stat`'s output, MiB: its free,
/// inactive and speculative pages times the page size its first line
/// names ("(page size of 16384 bytes)"). `None` when any of the four is
/// missing or not a number.
pub fn vm_stat_available_mib(text: &str) -> Option<u32> {
    let mut lines = text.lines();
    let size: u64 = lines
        .next()?
        .split("page size of ") // not wording: vm_stat's header line
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    let pages = |name: &str| -> Option<u64> {
        let line = text.lines().find(|l| l.starts_with(name))?;
        line[name.len()..].trim().trim_end_matches('.').parse().ok()
    };
    let free = pages("Pages free:")?; // not wording: a vm_stat field name
    let inactive = pages("Pages inactive:")?; // not wording: a vm_stat field name
    let speculative = pages("Pages speculative:")?; // not wording: a vm_stat field name
    let bytes = (free + inactive + speculative).checked_mul(size)?;
    u32::try_from(bytes >> 20).ok()
}

/// `GlobalMemoryStatusEx`, declared here by hand: one function of
/// `kernel32`, which every Windows process links, and its structure.
#[cfg(windows)]
mod windows_memory {
    /// `MEMORYSTATUSEX`, field for field.
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }

    // Windows' own size for the structure, which `length` must give.
    const _: () = assert!(size_of::<MemoryStatusEx>() == 64);

    // An `extern` block is `unsafe` by itself.
    #[allow(unsafe_code)]
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
    }

    /// The physical memory Windows can give a program now, in MiB.
    #[allow(unsafe_code)]
    pub fn available_mib() -> Option<u32> {
        let mut status = MemoryStatusEx {
            length: size_of::<MemoryStatusEx>() as u32,
            memory_load: 0,
            total_phys: 0,
            avail_phys: 0,
            total_page_file: 0,
            avail_page_file: 0,
            total_virtual: 0,
            avail_virtual: 0,
            avail_extended_virtual: 0,
        };
        // SAFETY: `status` is a live, writable `MEMORYSTATUSEX` (the
        // layout above, checked at 64 bytes) whose `length` is set, as
        // the function requires; it writes the structure and nothing
        // else, and returns non-zero on success.
        let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
        if ok == 0 {
            return None;
        }
        u32::try_from(status.avail_phys / (1024 * 1024)).ok()
    }
}

/// The kept entries that say which keys wait for their backup, and the
/// flows that finish it: each is written whole (a temporary file, then a
/// rename), so that a save cut part way never leaves one half written.
const ATOMIC: [&str; 2] = ["backup-gate", "backup-flow"];

/// Where the Inbox and Outbox wait between processes: a RAM directory on
/// the device, memory alone on a desktop.
pub enum Boxes {
    /// A directory with `inbox/` and `outbox/` in it.
    Dir(PathBuf),
    /// Held by the shell itself.
    Memory {
        /// Inbox files.
        inbox: Vec<(String, Vec<u8>)>,
        /// Outbox files.
        outbox: Vec<(String, Vec<u8>)>,
        /// What the app keeps for itself.
        kept: Vec<(String, Vec<u8>)>,
    },
}

impl Boxes {
    /// Keeps the Inbox, the Outbox and what the app keeps. The kept
    /// `backup-gate` and `backup-flow` go first and atomically (a
    /// temporary file, then a rename), so that a save that fails part way
    /// still leaves the keys that wait for their backup and a flow that
    /// reads.
    pub fn save(
        &mut self,
        inbox: Vec<(String, Vec<u8>)>,
        outbox: Vec<(String, Vec<u8>)>,
        kept: Vec<(String, Vec<u8>)>,
    ) -> Result<(), String> {
        match self {
            Boxes::Memory {
                inbox: i,
                outbox: o,
                kept: k,
            } => {
                *i = inbox;
                *o = outbox;
                *k = kept;
                Ok(())
            }
            Boxes::Dir(dir) => {
                for (sub, mut files) in [("kept", kept), ("inbox", inbox), ("outbox", outbox)] {
                    let d = dir.join(sub);
                    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
                    let mut kept_names: Vec<&str> = Vec::new();
                    for atomic in ATOMIC {
                        let Some(at) = files.iter().position(|(n, _)| n == atomic) else {
                            continue;
                        };
                        let (name, mut bytes) = files.remove(at);
                        let tmp = d.join(format!(".{name}.tmp"));
                        let wrote =
                            fs::write(&tmp, &bytes).and_then(|()| fs::rename(&tmp, d.join(&name)));
                        zeroize::Zeroize::zeroize(&mut bytes);
                        if wrote.is_err() {
                            let _ = fs::remove_file(&tmp);
                        }
                        wrote.map_err(|e| e.to_string())?;
                        kept_names.push(atomic);
                    }
                    for e in fs::read_dir(&d).map_err(|e| e.to_string())?.flatten() {
                        // What was just written stays; one no longer needed
                        // goes.
                        if kept_names.iter().any(|n| e.file_name() == *n) {
                            continue;
                        }
                        let _ = fs::remove_file(e.path());
                    }
                    for (name, mut bytes) in files {
                        let wrote = fs::write(d.join(checked(&name)?), &bytes);
                        // The shell's copy goes; the app keeps its own.
                        zeroize::Zeroize::zeroize(&mut bytes);
                        wrote.map_err(|e| e.to_string())?;
                    }
                }
                Ok(())
            }
        }
    }

    /// What the previous process left.
    pub fn restore(&self) -> StorageEvent {
        match self {
            Boxes::Memory {
                inbox,
                outbox,
                kept,
            } => StorageEvent::Restored {
                inbox: inbox.clone(),
                outbox: outbox.clone(),
                kept: kept.clone(),
            },
            Boxes::Dir(dir) => {
                let load = |sub: &str| -> Vec<(String, Vec<u8>)> {
                    let mut v: Vec<(String, Vec<u8>)> = fs::read_dir(dir.join(sub))
                        .map(|d| {
                            d.flatten()
                                .filter_map(|e| {
                                    let name = e.file_name().to_string_lossy().into_owned();
                                    // A temporary file of a save that was cut
                                    // off is not kept.
                                    if ATOMIC.iter().any(|a| name == format!(".{a}.tmp")) {
                                        let _ = fs::remove_file(e.path());
                                        return None;
                                    }
                                    fs::read(e.path()).ok().map(|b| (name, b))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    v.sort();
                    v
                };
                StorageEvent::Restored {
                    inbox: load("inbox"),
                    outbox: load("outbox"),
                    kept: load("kept"),
                }
            }
        }
    }
}

/// What an online shell (the desktop app) has of its computer's own
/// folders: where PDFs are saved, and the Downloads folder Transfer lists,
/// reads and saves into.
pub struct Host {
    /// Where PDFs are saved.
    pub print: PathBuf,
    /// The folder Transfer sends from and saves into.
    pub downloads: PathBuf,
    /// This computer opens a folder in its file manager (`xdg-open`, or
    /// `explorer.exe` on Windows).
    pub opens: bool,
}

impl Host {
    /// The Downloads folder as the app sees it.
    pub fn files(&self) -> StorageEvent {
        StorageEvent::HostFiles {
            dir: self.downloads.display().to_string(),
            files: host_listing(&self.downloads),
            opens: self.opens,
        }
    }
}

/// Whether this computer has a way to open a folder: `explorer.exe` on
/// Windows, always; elsewhere `xdg-open` on the path.
#[cfg(windows)]
pub fn can_open_folders() -> bool {
    true
}

/// Whether `xdg-open` is on the path, to open a folder with.
#[cfg(not(windows))]
pub fn can_open_folders() -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|d| d.join("xdg-open").is_file()))
}

/// The files in a folder of this computer's, newest first, with their
/// sizes: no folders, and none of an operating system's own
/// ([`foreign`]: a Windows Downloads folder's `desktop.ini`).
pub fn host_listing(dir: &Path) -> Vec<(String, u64)> {
    let Ok(d) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(std::time::SystemTime, String, u64)> = d
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            if foreign(&name) {
                return None;
            }
            // A link is listed as what it points to.
            let m = fs::metadata(e.path()).ok()?;
            m.is_file().then(|| {
                let when = m.modified().unwrap_or(std::time::UNIX_EPOCH);
                (when, name, m.len())
            })
        })
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    files.into_iter().map(|(_, n, s)| (n, s)).collect()
}

/// Reads a file of this computer's that Transfer sends: no more than the
/// file envelope carries and a byte, so that a larger one is known.
pub fn read_host(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    f.take(faraday_core::transfer::MAX_SEND + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// Saves a file Transfer received into `dir` under `name`, or `name (2)`,
/// `name (3)` and on before its extension when that is taken: a new file
/// each time, never over one already there. Returns its path.
pub fn save_download(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    use std::io::Write;
    let name = checked(name)?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    for n in 1..1000 {
        let candidate = if n == 1 {
            name.to_string()
        } else {
            format!("{stem} ({n}){ext}")
        };
        let path = dir.join(&candidate);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                f.write_all(bytes)
                    .and_then(|()| f.sync_all())
                    .map_err(|e| e.to_string())?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    Err(fill(EN.storage_no_free_name, &[name]))
}

/// Opens a folder in this computer's file manager, or a web address in
/// its browser.
fn open_folder(dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let program = {
        // From the system folder, not from the search path, so that no
        // other `explorer.exe` can answer.
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("explorer.exe")
    };
    #[cfg(not(windows))]
    let program = PathBuf::from("xdg-open");
    let mut child = std::process::Command::new(program)
        .arg(dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    // Reaped when it ends, without holding the shell up.
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Answers every storage request the app has queued, the sticks' files
/// wherever `sticks` keeps them, and hands it the pictures read since the
/// last pass: a picture's codes are read off the shell's thread and come
/// on a later pass ([`Sticks::read_qr_later`]), so the shell serves on
/// every pass, asked or not. `host` is what an online shell has of its
/// computer's folders; the device passes `None` and refuses to print,
/// read or save there. Returns the lines a verbose shell prints.
pub fn serve_with(
    app: &mut Faraday,
    boxes: &mut Boxes,
    host: Option<&Host>,
    sticks: &mut dyn Sticks,
) -> Vec<String> {
    let print = host.map(|h| h.print.as_path());
    let mut said = Vec::new();
    loop {
        // What was read of the pictures, as their answers.
        for (stick, name, picture) in sticks.pictures_read() {
            app.storage(match picture {
                Ok((bytes, payloads)) => StorageEvent::QrRead {
                    name,
                    bytes,
                    payloads,
                },
                Err(reason) => StorageEvent::ReadFailed {
                    stick,
                    name,
                    reason,
                },
            });
        }
        for done in sticks.transferred() {
            delivered(app, sticks, &mut said, done);
        }
        if !app.storage_pending() {
            break;
        }
        serve_queued(app, boxes, host, print, sticks, &mut said);
    }
    said
}

/// Hands the app a finished transfer.
fn delivered(
    app: &mut Faraday,
    sticks: &mut dyn Sticks,
    said: &mut Vec<String>,
    done: Transferred,
) {
    let ev = match done {
        // A vault read for its SHA-256: the bytes are wiped here, and only
        // the hash goes to the app.
        Transferred::Read {
            stick,
            name,
            result,
        } if sticks.hash_taken(&stick, &name) => {
            let sha = result.map(|mut bytes| {
                let sha = faraday_core::file_menu::sha256(&app.compute(), &bytes);
                zeroize::Zeroize::zeroize(&mut bytes);
                sha
            });
            StorageEvent::Hashed { stick, name, sha }
        }
        Transferred::Read {
            stick,
            name,
            result,
        } => match result {
            Ok(bytes) => StorageEvent::Read { stick, name, bytes },
            Err(reason) => StorageEvent::ReadFailed {
                stick,
                name,
                reason,
            },
        },
        Transferred::Write {
            stick,
            name,
            result,
        } => match result {
            Err(faraday_files::WriteRefused::NameTaken { on_stick, sha }) => {
                said.push(format!("{stick}/{on_stick} has the name of {name}"));
                StorageEvent::NameTaken {
                    stick,
                    name,
                    on_stick,
                    sha,
                }
            }
            Ok(wrote_as) => {
                said.push(format!("wrote {stick}/{wrote_as}"));
                StorageEvent::Written {
                    stick,
                    name,
                    wrote_as,
                }
            }
            Err(faraday_files::WriteRefused::Failed(reason)) => {
                said.push(format!("cannot write {stick}/{name}: {reason}"));
                StorageEvent::WriteFailed {
                    stick,
                    name,
                    reason,
                }
            }
        },
    };
    app.storage(ev);
}

/// Answers the storage requests the app has queued now.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn serve_queued(
    app: &mut Faraday,
    boxes: &mut Boxes,
    host: Option<&Host>,
    print: Option<&Path>,
    sticks: &mut dyn Sticks,
    said: &mut Vec<String>,
) {
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                if let Err(t) = sticks.transfer_later(Transfer::Read { stick, name }) {
                    let done = transfer_now(&app.compute(), sticks, t);
                    delivered(app, sticks, said, done);
                }
            }
            StorageCommand::Write {
                stick,
                name,
                bytes,
                copies,
                replace,
                renamed,
                on_taken,
            } => {
                let on_taken = match on_taken {
                    faraday_core::OnTaken::Ask => faraday_files::OnTaken::Ask,
                    faraday_core::OnTaken::Beside => faraday_files::OnTaken::Beside,
                    faraday_core::OnTaken::Over(sha) => faraday_files::OnTaken::Over(sha),
                };
                let t = Transfer::Write {
                    stick,
                    name,
                    bytes,
                    copies,
                    replace,
                    renamed,
                    on_taken,
                };
                if let Err(t) = sticks.transfer_later(t) {
                    let done = transfer_now(&app.compute(), sticks, t);
                    delivered(app, sticks, said, done);
                }
            }
            StorageCommand::StickSpace { stick } => {
                let ev = match sticks.space(&stick) {
                    Ok((free, size)) => StorageEvent::StickSpace { stick, free, size },
                    Err(reason) => StorageEvent::ZeroFailed { stick, reason },
                };
                app.storage(ev);
            }
            StorageCommand::ZeroFree { stick, from } => {
                let ev = match sticks.zero_free(&stick, from) {
                    Ok((next, total, done)) => StorageEvent::Zeroing {
                        stick,
                        next,
                        total,
                        done,
                    },
                    Err(reason) => {
                        said.push(format!("cannot zero {stick}: {reason}")); // not shown: a line for a verbose shell's console
                        StorageEvent::ZeroFailed { stick, reason }
                    }
                };
                app.storage(ev);
            }
            StorageCommand::MakeEmpty { stick } => {
                let ev = match sticks.make_empty(&stick) {
                    Ok(()) => {
                        said.push(format!("made {stick} empty")); // not shown: a line for a verbose shell's console
                        StorageEvent::Zeroing {
                            stick,
                            next: 2,
                            total: 0,
                            done: false,
                        }
                    }
                    Err(reason) => {
                        said.push(format!("cannot make {stick} empty: {reason}")); // not shown: a line for a verbose shell's console
                        StorageEvent::ZeroFailed { stick, reason }
                    }
                };
                app.storage(ev);
            }
            StorageCommand::Wipe { stick, name } => {
                let ev = match sticks.wipe(&stick, &name) {
                    Ok(()) => {
                        said.push(format!("deleted {stick}/{name}, zeros over it first"));
                        StorageEvent::Wiped { stick, name }
                    }
                    Err(reason) => {
                        said.push(format!("cannot delete {stick}/{name}: {reason}"));
                        StorageEvent::WipeFailed {
                            stick,
                            name,
                            reason,
                        }
                    }
                };
                app.storage(ev);
            }
            StorageCommand::Rename {
                stick,
                from,
                to,
                vaults,
            } => {
                let c = app.compute();
                let refused =
                    sticks
                        .rename(&c, &stick, &from, &to, &vaults)
                        .err()
                        .map(|r| match r {
                            faraday_files::Unrenamed::Taken => faraday_core::Unrenamed::Taken,
                            faraday_files::Unrenamed::SameVault => {
                                faraday_core::Unrenamed::SameVault
                            }
                            faraday_files::Unrenamed::Failed(why) => {
                                faraday_core::Unrenamed::Failed(why)
                            }
                        });
                said.push(match &refused {
                    None => format!("renamed {stick}/{from} {to}"),
                    Some(r) => format!("cannot rename {stick}/{from}: {r:?}"), // not shown: a line for a verbose shell's console
                });
                app.storage(StorageEvent::Renamed {
                    stick,
                    from,
                    to,
                    refused,
                });
            }
            StorageCommand::NameStick { stick, label } => {
                let refused = sticks.name(&stick, &label).err();
                said.push(match &refused {
                    None => format!("named {stick} {label:?}"), // not shown: a line for a verbose shell's console
                    Some(r) => format!("cannot name {stick}: {r}"), // not shown: as above
                });
                app.storage(StorageEvent::StickNamed {
                    stick,
                    label,
                    refused,
                });
            }
            // A file read for its Details' SHA-256: the bytes are wiped
            // here, and only the hash goes to the app.
            StorageCommand::Hash { stick, name } => {
                if sticks.hash_later(&stick, &name) {
                    continue;
                }
                let sha = sticks.read(&stick, &name).map(|mut bytes| {
                    let sha = faraday_core::file_menu::sha256(&app.compute(), &bytes);
                    zeroize::Zeroize::zeroize(&mut bytes);
                    sha
                });
                app.storage(StorageEvent::Hashed { stick, name, sha });
            }
            // Read off this thread: its answer comes on a later pass.
            StorageCommand::ReadQr { stick, name } => sticks.read_qr_later(&stick, &name),
            StorageCommand::Print { name, bytes } => {
                let ev = match print {
                    None => StorageEvent::PrintFailed {
                        reason: EN.storage_no_print.into(),
                    },
                    Some(dir) => {
                        let saved = fs::create_dir_all(dir)
                            .map_err(|e| e.to_string())
                            .and_then(|()| checked(&name).map(|n| dir.join(n)))
                            .and_then(|path| {
                                fs::write(&path, &bytes)
                                    .map(|()| path)
                                    .map_err(|e| e.to_string())
                            });
                        match saved {
                            Ok(path) => StorageEvent::Printed {
                                path: path.display().to_string(),
                            },
                            Err(reason) => StorageEvent::PrintFailed { reason },
                        }
                    }
                };
                app.storage(ev);
            }
            StorageCommand::ReadHost { path } => {
                let ev = match host {
                    None => StorageEvent::HostReadFailed {
                        path,
                        reason: EN.storage_no_host_read.into(),
                    },
                    Some(_) => match read_host(Path::new(&path)) {
                        Ok(bytes) => StorageEvent::HostRead { path, bytes },
                        Err(reason) => StorageEvent::HostReadFailed { path, reason },
                    },
                };
                app.storage(ev);
            }
            StorageCommand::SaveDownload { name, mut bytes } => {
                let ev = match host {
                    None => StorageEvent::SaveFailed {
                        reason: EN.storage_no_download.into(),
                    },
                    Some(h) => match save_download(&h.downloads, &name, &bytes) {
                        Ok(path) => {
                            said.push(format!("saved {}", path.display()));
                            StorageEvent::Saved {
                                path: path.display().to_string(),
                            }
                        }
                        Err(reason) => StorageEvent::SaveFailed { reason },
                    },
                };
                // The shell's copy goes; the file has it now.
                zeroize::Zeroize::zeroize(&mut bytes);
                app.storage(ev);
                // The folder has a new file.
                if let Some(h) = host {
                    app.storage(h.files());
                }
            }
            // The one web address the app opens, fixed here: the app
            // names no address of its own.
            StorageCommand::OpenInstitute => {
                if host.is_some_and(|h| h.opens)
                    && let Err(e) = open_folder(Path::new("https://nakamotoinstitute.org"))
                {
                    said.push(format!("cannot open nakamotoinstitute.org: {e}")); // not shown: a line for a verbose shell's console
                }
            }
            StorageCommand::OpenDownloads => {
                if let Some(h) = host.filter(|h| h.opens)
                    && let Err(e) = open_folder(&h.downloads)
                {
                    said.push(format!("cannot open {}: {e}", h.downloads.display())); // not shown: a line for a verbose shell's console
                }
            }
            StorageCommand::SaveBoxes {
                inbox,
                outbox,
                kept,
            } => {
                if let Err(e) = boxes.save(inbox, outbox, kept) {
                    said.push(format!("cannot keep the Inbox and Outbox: {e}")); // not shown: a line for a verbose shell's console
                    app.storage(StorageEvent::NotKept);
                }
            }
        }
    }
}

/// The disk process, from the shell's side (`docs/history/PLAN.md` §4.3): requests go
/// down one FIFO and answers come up the other. A thread reads the
/// answers, so a disk process that never answers costs a timeout rather
/// than the shell; an answer that comes after its timeout is known by its
/// sequence number and dropped.
pub struct DiskProcess {
    tx: fs::File,
    answers: std::sync::mpsc::Receiver<(u32, zeroize::Zeroizing<Vec<u8>>)>,
    seq: u32,
    /// The sticks the last listing found unusable: their ids, and whether
    /// the reason is that they are read-only (else not FAT).
    problems: Vec<(String, bool)>,
    /// The sticks the last listing found not handed out yet, by id.
    waiting: Vec<String>,
    /// Pictures asked for, not yet sent: one is asked of the process at a
    /// time, so it holds one picture's bytes.
    pictures: std::collections::VecDeque<(String, String)>,
    /// The picture the process is reading: its request's number, stick,
    /// name, and when it was asked.
    picture: Option<(u32, String, String, std::time::Instant)>,
    /// Pictures answered, not yet handed to the app.
    pictures_done: Vec<PictureRead>,
    /// Vaults asked for or written, not yet started.
    transfers: std::collections::VecDeque<Transfer>,
    /// The vault in flight, a piece at a time.
    transfer: Option<Flight>,
    /// Transfers finished, not yet handed to the app.
    transfers_done: Vec<Transferred>,
    /// Vaults read for their SHA-256, not yet answered.
    hashing: Vec<(String, String)>,
}

/// A transfer the disk process is at, with the piece asked now.
struct Flight {
    /// The request's number.
    seq: u32,
    /// When it was asked.
    asked: std::time::Instant,
    /// The answer, once it has come.
    answer: Option<zeroize::Zeroizing<Vec<u8>>>,
    /// What is being transferred and how far.
    what: Going,
}

/// How far a transfer has got.
enum Going {
    /// A file read: what has come so far.
    Read {
        stick: String,
        name: String,
        out: Vec<u8>,
        /// The length the first piece stated; every piece states it again.
        total: u64,
    },
    /// A file written: the whole of it, and where the piece in flight
    /// starts.
    Write {
        stick: String,
        name: String,
        bytes: Vec<u8>,
        copies: Vec<[u8; 32]>,
        replace: bool,
        renamed: bool,
        on_taken: faraday_files::OnTaken,
        at: usize,
    },
}

/// How long a list or a read may take.
const ASK: std::time::Duration = std::time::Duration::from_secs(15);
/// How long a write may take: a vault of four 4 MiB slots on a slow stick,
/// written and read back.
const ASK_WRITE: std::time::Duration = std::time::Duration::from_secs(120);
/// How long a picture may take to be read for its codes: the largest
/// (`MAX_PIXELS`) takes seconds on a Pi.
const ASK_PICTURE: std::time::Duration = std::time::Duration::from_secs(60);

impl DiskProcess {
    /// Opens the two FIFOs. Both are opened read-write, which on a FIFO
    /// never waits for the other end: a disk process that is not up yet
    /// answers when it is.
    pub fn open(requests: &Path, responses: &Path) -> std::io::Result<DiskProcess> {
        let rw = |p: &Path| fs::OpenOptions::new().read(true).write(true).open(p);
        let tx = rw(requests)?;
        let mut rx = rw(responses)?;
        let (send, answers) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(Some(frame)) = faraday_files::proto::receive(&mut rx) {
                if send.send(frame).is_err() {
                    return;
                }
            }
        });
        Ok(DiskProcess {
            tx,
            answers,
            seq: 0,
            problems: Vec::new(),
            waiting: Vec::new(),
            pictures: std::collections::VecDeque::new(),
            picture: None,
            pictures_done: Vec::new(),
            transfers: std::collections::VecDeque::new(),
            transfer: None,
            transfers_done: Vec::new(),
            hashing: Vec::new(),
        })
    }

    /// An answer that is not to the request asked now: the picture's,
    /// kept for [`Sticks::pictures_read`]; any other is a late one,
    /// wiped.
    fn aside(&mut self, seq: u32, mut frame: zeroize::Zeroizing<Vec<u8>>) {
        use faraday_files::proto::Response;
        if self.picture.as_ref().is_some_and(|p| p.0 == seq)
            && let Some((_, stick, name, _)) = self.picture.take()
        {
            let picture = match Response::decode(&frame) {
                Ok(Response::Qr { bytes, codes }) => Ok((bytes, codes)),
                Ok(Response::Failed(why)) => Err(why),
                Ok(_) => Err(EN.storage_answer_codes.to_string()),
                Err(_) => Err(EN.storage_answer_malformed.to_string()),
            };
            self.pictures_done.push((stick, name, picture));
        } else if let Some(f) = self.transfer.as_mut().filter(|f| f.seq == seq) {
            f.answer = Some(frame);
            return;
        }
        zeroize::Zeroize::zeroize(&mut frame);
    }

    /// The sticks the last listing found unusable, by id: true when the
    /// reason is that the device is read-only, else it is not FAT.
    pub fn problems(&self) -> &[(String, bool)] {
        &self.problems
    }

    /// The sticks the last listing found not handed out yet, by id: in,
    /// but nothing on them can be read until `faraday-grant` hands them
    /// over, which it does only while the app is clean.
    pub fn waiting(&self) -> &[String] {
        &self.waiting
    }

    /// Sends the request for the piece after `going`'s, or finishes it.
    fn send_piece(&mut self, going: Going) {
        use faraday_files::proto::{PIECE, Request, send};
        let (req, going) = match going {
            Going::Read {
                stick,
                name,
                out,
                total,
            } => (
                Request::ReadPart {
                    stick: stick.clone(),
                    name: name.clone(),
                    from: out.len() as u64,
                    len: PIECE as u32,
                },
                Going::Read {
                    stick,
                    name,
                    out,
                    total,
                },
            ),
            Going::Write {
                stick,
                name,
                bytes,
                copies,
                replace,
                renamed,
                on_taken,
                at,
            } => {
                // Pieces go before the last, which is the write itself.
                let last_at = (bytes.len() - 1) / PIECE * PIECE;
                let req = if at < last_at {
                    Request::Piece {
                        stick: stick.clone(),
                        name: name.clone(),
                        at: at as u64,
                        total: bytes.len() as u64,
                        bytes: bytes[at..at + PIECE].to_vec(),
                    }
                } else {
                    Request::Write {
                        stick: stick.clone(),
                        name: name.clone(),
                        copies: copies.clone(),
                        replace,
                        renamed,
                        on_taken,
                        total: bytes.len() as u64,
                        bytes: bytes[last_at..].to_vec(),
                    }
                };
                (
                    req,
                    Going::Write {
                        stick,
                        name,
                        bytes,
                        copies,
                        replace,
                        renamed,
                        on_taken,
                        at,
                    },
                )
            }
        };
        self.seq = self.seq.wrapping_add(1);
        let mut req = req;
        let mut encoded = req.encode();
        let sent = send(&mut self.tx, self.seq, &encoded);
        zeroize::Zeroize::zeroize(&mut encoded);
        req.wipe();
        match sent {
            Ok(()) => {
                self.transfer = Some(Flight {
                    seq: self.seq,
                    asked: std::time::Instant::now(),
                    answer: None,
                    what: going,
                });
            }
            Err(e) => self.finish(going, e.to_string()),
        }
    }

    /// A transfer over, wiping what it held.
    fn finish(&mut self, going: Going, why: String) {
        self.transfers_done.push(match going {
            Going::Read {
                stick,
                name,
                mut out,
                ..
            } => {
                zeroize::Zeroize::zeroize(&mut out);
                Transferred::Read {
                    stick,
                    name,
                    result: Err(why),
                }
            }
            Going::Write {
                stick,
                name,
                mut bytes,
                ..
            } => {
                zeroize::Zeroize::zeroize(&mut bytes);
                Transferred::Write {
                    stick,
                    name,
                    result: Err(faraday_files::WriteRefused::Failed(why)),
                }
            }
        });
    }

    /// The piece of a file read that has come: kept, and the next asked,
    /// or the file whole.
    fn read_part(
        &mut self,
        stick: String,
        name: String,
        mut out: Vec<u8>,
        known: u64,
        total: u64,
        bytes: Vec<u8>,
    ) {
        let mut part = bytes;
        let first = out.is_empty();
        let total_ok = if first {
            total <= faraday_files::MAX_VAULT_READ
        } else {
            total == known
        };
        let want = total
            .saturating_sub(out.len() as u64)
            .min(faraday_files::proto::PIECE as u64);
        let refused = !total_ok
            || part.len() as u64 != want
            || (first
                && out
                    .try_reserve_exact(usize::try_from(total).unwrap_or(usize::MAX))
                    .is_err());
        if refused {
            zeroize::Zeroize::zeroize(&mut part);
            let why = EN.storage_answer_file.to_string();
            self.finish(
                Going::Read {
                    stick,
                    name,
                    out,
                    total,
                },
                why,
            );
            return;
        }
        out.extend_from_slice(&part);
        zeroize::Zeroize::zeroize(&mut part);
        if out.len() as u64 >= total {
            self.transfers_done.push(Transferred::Read {
                stick,
                name,
                result: Ok(out),
            });
        } else {
            self.send_piece(Going::Read {
                stick,
                name,
                out,
                total,
            });
        }
    }

    /// Takes the answer of the piece in flight and asks the next, or
    /// starts the next transfer. Never waits.
    fn step_transfer(&mut self) {
        use faraday_files::proto::{PIECE, Response};
        if let Some(f) = self.transfer.take() {
            let Some(mut frame) = f.answer else {
                if f.asked.elapsed() > ASK_WRITE {
                    self.finish(f.what, EN.storage_disk_silent.to_string());
                } else {
                    self.transfer = Some(Flight { answer: None, ..f });
                }
                return;
            };
            let answer = Response::decode(&frame);
            zeroize::Zeroize::zeroize(&mut frame);
            let answer = answer.map_err(|_| EN.storage_answer_malformed.to_string());
            match (f.what, answer) {
                (what, Err(why)) => self.finish(what, why),
                (
                    Going::Read {
                        stick,
                        name,
                        out,
                        total: known,
                    },
                    Ok(Response::Part { total, bytes }),
                ) => {
                    self.read_part(stick, name, out, known, total, bytes);
                }
                (what @ Going::Read { .. }, Ok(Response::Failed(why))) => {
                    self.finish(what, why);
                }
                (what @ Going::Read { .. }, Ok(_)) => {
                    self.finish(what, EN.storage_answer_file.to_string());
                }
                (
                    Going::Write {
                        stick,
                        name,
                        mut bytes,
                        copies,
                        replace,
                        renamed,
                        on_taken,
                        at,
                    },
                    Ok(answer),
                ) => {
                    let last_at = (bytes.len() - 1) / PIECE * PIECE;
                    let result = if at < last_at {
                        match answer {
                            Response::Held(_) => {
                                self.send_piece(Going::Write {
                                    stick,
                                    name,
                                    bytes,
                                    copies,
                                    replace,
                                    renamed,
                                    on_taken,
                                    at: at + PIECE,
                                });
                                return;
                            }
                            Response::Failed(why) => Err(faraday_files::WriteRefused::Failed(why)),
                            _ => Err(faraday_files::WriteRefused::Failed(
                                EN.storage_answer_write.to_string(),
                            )),
                        }
                    } else {
                        match answer {
                            Response::Written(n) => Ok(n),
                            Response::WriteNameTaken { on_stick, sha } => {
                                Err(faraday_files::WriteRefused::NameTaken { on_stick, sha })
                            }
                            Response::Failed(why) => Err(faraday_files::WriteRefused::Failed(why)),
                            _ => Err(faraday_files::WriteRefused::Failed(
                                EN.storage_answer_write.to_string(),
                            )),
                        }
                    };
                    zeroize::Zeroize::zeroize(&mut bytes);
                    self.transfers_done.push(Transferred::Write {
                        stick,
                        name,
                        result,
                    });
                }
            }
            return;
        }
        self.start_next();
    }

    /// Starts the next transfer asked, if any.
    fn start_next(&mut self) {
        let going = match self.transfers.pop_front() {
            None => return,
            Some(Transfer::Read { stick, name }) => Going::Read {
                stick,
                name,
                out: Vec::new(),
                total: 0,
            },
            Some(Transfer::Write {
                stick,
                name,
                bytes,
                copies,
                replace,
                renamed,
                on_taken,
            }) => Going::Write {
                stick,
                name,
                bytes,
                copies,
                replace,
                renamed,
                on_taken,
                at: 0,
            },
        };
        self.send_piece(going);
    }

    fn ask(
        &mut self,
        req: &faraday_files::proto::Request,
    ) -> Result<faraday_files::proto::Response, String> {
        use faraday_files::proto::{Request, Response, send};
        self.seq = self.seq.wrapping_add(1);
        let mut encoded = req.encode();
        let sent = send(&mut self.tx, self.seq, &encoded);
        zeroize::Zeroize::zeroize(&mut encoded);
        sent.map_err(|e| e.to_string())?;
        let wait = if matches!(req, Request::Write { .. } | Request::Piece { .. }) {
            ASK_WRITE
        } else {
            ASK
        };
        let until = std::time::Instant::now() + wait;
        loop {
            let left = until.saturating_duration_since(std::time::Instant::now());
            match self.answers.recv_timeout(left) {
                Ok((seq, mut frame)) if seq == self.seq => {
                    let answer = Response::decode(&frame);
                    zeroize::Zeroize::zeroize(&mut frame);
                    return answer.map_err(|_| EN.storage_answer_malformed.to_string());
                }
                Ok((seq, other)) => {
                    self.aside(seq, other);
                    continue;
                }
                Err(_) => return Err(EN.storage_disk_silent.to_string()),
            }
        }
    }

    /// The partitions handed out now, by their volume labels. The app
    /// names the boot medium itself (`Medium::boot`), whatever its label.
    pub fn list(&mut self) -> Result<Vec<StickInfo>, String> {
        use faraday_files::proto::{Problem, Request, Response};
        match self.ask(&Request::List)? {
            Response::Sticks(s) => {
                self.problems = s
                    .iter()
                    .filter_map(|s| match s.problem {
                        // Not handed out yet: it waits, no problem is told.
                        None | Some(Problem::NotYet) => None,
                        Some(p) => Some((s.id.clone(), p == Problem::ReadOnly)),
                    })
                    .collect();
                self.waiting = s
                    .iter()
                    .filter(|s| s.problem == Some(Problem::NotYet))
                    .map(|s| s.id.clone())
                    .collect();
                Ok(s.into_iter()
                    .map(|s| StickInfo {
                        label: s.label,
                        id: s.id,
                        boot: s.boot,
                        serial: s.serial,
                        files: s.files,
                        vaults: s.vaults,
                        make: faraday_core::StickMake {
                            maker: s.make.maker,
                            model: s.make.model,
                            size: s.make.size,
                        },
                    })
                    .collect())
            }
            Response::Failed(why) => Err(why),
            _ => Err(EN.storage_answer_listing.to_string()),
        }
    }
}

impl DiskProcess {
    /// A file written through the disk process as [`Sticks::write`]
    /// writes it, a vault over the file with one of `copies` (the SHA-256
    /// of the copies the app has seen). The shell's own files, the
    /// timings and the boot report, are no vault: no copies.
    pub fn write_copies(
        &mut self,
        stick: &str,
        name: &str,
        bytes: &[u8],
        copies: &[[u8; 32]],
        replace: bool,
        renamed: bool,
    ) -> Result<String, String> {
        self.write_asking(
            stick,
            name,
            bytes,
            copies,
            replace,
            renamed,
            faraday_files::OnTaken::Beside,
        )
        .map_err(faraday_files::WriteRefused::text)
    }

    /// [`DiskProcess::write_copies`], a vault's same-name file dealt with as
    /// `on_taken` says, the refusal as it came.
    #[expect(
        clippy::too_many_arguments,
        reason = "one write, as the protocol carries it"
    )]
    pub(crate) fn write_asking(
        &mut self,
        stick: &str,
        name: &str,
        bytes: &[u8],
        copies: &[[u8; 32]],
        replace: bool,
        renamed: bool,
        on_taken: faraday_files::OnTaken,
    ) -> Result<String, faraday_files::WriteRefused> {
        use faraday_files::WriteRefused as W;
        use faraday_files::proto::{Request, Response};
        // A file longer than a piece goes in pieces, held by the disk
        // process until the write that carries the last one (F206
        // point 7); a frame keeps its limit.
        let piece = faraday_files::proto::PIECE;
        let last_at = if bytes.len() > piece {
            (bytes.len() - 1) / piece * piece
        } else {
            0
        };
        for (k, part) in bytes[..last_at].chunks(piece).enumerate() {
            let mut req = Request::Piece {
                stick: stick.into(),
                name: name.into(),
                at: (k * piece) as u64,
                total: bytes.len() as u64,
                bytes: part.to_vec(),
            };
            let answer = self.ask(&req);
            req.wipe();
            match answer.map_err(W::Failed)? {
                Response::Held(_) => {}
                Response::Failed(why) => return Err(W::Failed(why)),
                _ => return Err(W::Failed(EN.storage_answer_write.to_string())),
            }
        }
        let mut req = Request::Write {
            stick: stick.into(),
            name: name.into(),
            copies: copies.to_vec(),
            replace,
            renamed,
            on_taken,
            total: bytes.len() as u64,
            bytes: bytes[last_at..].to_vec(),
        };
        let answer = self.ask(&req);
        req.wipe();
        match answer.map_err(W::Failed)? {
            Response::Written(n) => Ok(n),
            Response::WriteNameTaken { on_stick, sha } => Err(W::NameTaken { on_stick, sha }),
            Response::Failed(why) => Err(W::Failed(why)),
            _ => Err(W::Failed(EN.storage_answer_write.to_string())),
        }
    }
}

impl Sticks for DiskProcess {
    fn read(&mut self, stick: &str, name: &str) -> Result<Vec<u8>, String> {
        use faraday_files::proto::{PIECE, Request, Response};
        // A file is read in pieces, so that a vault longer than a frame
        // comes whole (F206 point 7).
        let mut out: Vec<u8> = Vec::new();
        let mut known = 0;
        loop {
            let from = out.len() as u64;
            let (total, mut part) = match self.ask(&Request::ReadPart {
                stick: stick.into(),
                name: name.into(),
                from,
                len: PIECE as u32,
            })? {
                Response::Part { total, bytes } => (total, bytes),
                Response::Failed(why) => {
                    zeroize::Zeroize::zeroize(&mut out);
                    return Err(why);
                }
                _ => {
                    zeroize::Zeroize::zeroize(&mut out);
                    return Err(EN.storage_answer_file.to_string());
                }
            };
            let want = total.saturating_sub(from).min(PIECE as u64);
            let bad = if from == 0 {
                total > faraday_files::MAX_VAULT_READ
                    || out
                        .try_reserve_exact(usize::try_from(total).unwrap_or(usize::MAX))
                        .is_err()
            } else {
                total != known
            } || part.len() as u64 != want;
            if bad {
                zeroize::Zeroize::zeroize(&mut part);
                zeroize::Zeroize::zeroize(&mut out);
                return Err(EN.storage_answer_file.to_string());
            }
            known = total;
            out.extend_from_slice(&part);
            zeroize::Zeroize::zeroize(&mut part);
            if out.len() as u64 >= total {
                return Ok(out);
            }
        }
    }

    fn write(
        &mut self,
        _c: &faraday_crypto::work::Compute,
        stick: &str,
        name: &str,
        bytes: &[u8],
        copies: &[[u8; 32]],
        replace: bool,
        renamed: bool,
        on_taken: faraday_files::OnTaken,
    ) -> Result<String, faraday_files::WriteRefused> {
        self.write_asking(stick, name, bytes, copies, replace, renamed, on_taken)
    }

    fn wipe(&mut self, stick: &str, name: &str) -> Result<(), String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::Wipe {
            stick: stick.into(),
            name: name.into(),
        })? {
            Response::Wiped => Ok(()),
            Response::Failed(why) => Err(why),
            _ => Err(EN.storage_answer_delete.to_string()),
        }
    }

    fn space(&mut self, stick: &str) -> Result<(u64, u64), String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::Space {
            stick: stick.into(),
        })? {
            Response::Space { free, size } => Ok((free, size)),
            Response::Failed(why) => Err(why),
            _ => Err(EN.storage_answer_space.to_string()),
        }
    }

    fn zero_free(&mut self, stick: &str, from: u32) -> Result<(u32, u32, bool), String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::ZeroFree {
            stick: stick.into(),
            from,
        })? {
            Response::Zeroing { next, total, done } => Ok((next, total, done)),
            Response::Failed(why) => Err(why),
            _ => Err(EN.storage_answer_zeroing.to_string()),
        }
    }

    fn make_empty(&mut self, stick: &str) -> Result<(), String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::MakeEmpty {
            stick: stick.into(),
        })? {
            Response::Zeroing { .. } => Ok(()),
            Response::Failed(why) => Err(why),
            _ => Err(EN.storage_answer_zeroing.to_string()),
        }
    }

    fn transfer_later(&mut self, transfer: Transfer) -> Result<(), Transfer> {
        // A vault's file, the only one that may be longer than a frame.
        let vault = |name: &str| name.to_ascii_lowercase().ends_with(".kdbx");
        match &transfer {
            Transfer::Read { name, .. } if vault(name) => {}
            Transfer::Write { name, bytes, .. }
                if vault(name) && bytes.len() > faraday_files::proto::PIECE => {}
            _ => return Err(transfer),
        }
        self.transfers.push_back(transfer);
        Ok(())
    }

    fn transferred(&mut self) -> Vec<Transferred> {
        while let Ok((seq, frame)) = self.answers.try_recv() {
            self.aside(seq, frame);
        }
        self.step_transfer();
        std::mem::take(&mut self.transfers_done)
    }

    fn hash_later(&mut self, stick: &str, name: &str) -> bool {
        let read = Transfer::Read {
            stick: stick.to_string(),
            name: name.to_string(),
        };
        if self.transfer_later(read).is_err() {
            return false;
        }
        self.hashing.push((stick.to_string(), name.to_string()));
        true
    }

    fn hash_taken(&mut self, stick: &str, name: &str) -> bool {
        let at = self
            .hashing
            .iter()
            .position(|(s, n)| s == stick && n == name);
        at.map(|i| self.hashing.remove(i)).is_some()
    }

    fn read_qr_later(&mut self, stick: &str, name: &str) {
        self.pictures
            .push_back((stick.to_string(), name.to_string()));
    }

    fn pictures_read(&mut self) -> Vec<PictureRead> {
        use faraday_files::proto::{Request, send};
        while let Ok((seq, frame)) = self.answers.try_recv() {
            self.aside(seq, frame);
        }
        if self
            .picture
            .as_ref()
            .is_some_and(|p| p.3.elapsed() > ASK_PICTURE)
            && let Some((_, stick, name, _)) = self.picture.take()
        {
            let why = EN.storage_disk_silent.to_string();
            self.pictures_done.push((stick, name, Err(why)));
        }
        if self.picture.is_none()
            && let Some((stick, name)) = self.pictures.pop_front()
        {
            self.seq = self.seq.wrapping_add(1);
            let req = Request::ReadQr {
                stick: stick.clone(),
                name: name.clone(),
            };
            match send(&mut self.tx, self.seq, &req.encode()) {
                Ok(()) => {
                    self.picture = Some((self.seq, stick, name, std::time::Instant::now()));
                }
                Err(e) => self.pictures_done.push((stick, name, Err(e.to_string()))),
            }
        }
        std::mem::take(&mut self.pictures_done)
    }

    fn rename(
        &mut self,
        _c: &faraday_crypto::work::Compute,
        stick: &str,
        from: &str,
        to: &str,
        vaults: &[Vec<[u8; 32]>],
    ) -> Result<(), faraday_files::Unrenamed> {
        use faraday_files::Unrenamed as U;
        use faraday_files::proto::{Request, Response};
        match self
            .ask(&Request::Rename {
                stick: stick.into(),
                from: from.into(),
                to: to.into(),
                vaults: vaults.to_vec(),
            })
            .map_err(U::Failed)?
        {
            Response::Renamed => Ok(()),
            Response::NameTaken { same_vault: false } => Err(U::Taken),
            Response::NameTaken { same_vault: true } => Err(U::SameVault),
            Response::Failed(why) => Err(U::Failed(why)),
            _ => Err(U::Failed(EN.storage_answer_rename.to_string())),
        }
    }

    fn name(&mut self, stick: &str, label: &str) -> Result<(), String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::Name {
            stick: stick.into(),
            label: label.into(),
        })? {
            Response::Renamed => Ok(()),
            Response::Failed(why) => Err(why),
            _ => Err(EN.storage_answer_name.to_string()),
        }
    }
}
