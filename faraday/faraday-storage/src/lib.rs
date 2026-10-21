//! The file side of a stick, shared by Faraday's shells.
//!
//! A stick is a directory: a mounted FAT partition on the device, a plain
//! folder on a desktop. Each shell finds its sticks its own way and hands
//! the directories here; this crate lists them, reads a file the app asked
//! for, writes one as `PLAN.md` §4.3 says (a new file, read back and
//! compared, then renamed into place, never over a file already there),
//! and keeps the Inbox and Outbox where the next process finds them.

use std::fs;
use std::path::{Path, PathBuf};

use faraday_core::{BootPart, Faraday, StickInfo, StorageCommand, StorageEvent};
use faraday_files::{Dir, checked};

pub use faraday_files::{MAX_PIXELS, MAX_READ, foreign};

/// What a stick directory holds at its top level, as the app sees it.
pub fn stick_info(dir: &Path, label: &str, boot: bool) -> StickInfo {
    StickInfo {
        id: dir.display().to_string(),
        label: label.to_string(),
        boot,
        serial: None,
        files: faraday_files::listing(&mut Dir(dir.to_path_buf())),
    }
}

fn dir(stick: &str) -> Dir {
    Dir(PathBuf::from(stick))
}

/// Reads one file from a stick directory.
pub fn read(stick: &str, name: &str) -> Result<Vec<u8>, String> {
    faraday_files::read(&mut dir(stick), name)
}

/// Writes a file to a stick directory as a new file, reads it back,
/// compares, and renames it into place. Returns the name it was written
/// under.
pub fn write(stick: &str, name: &str, bytes: &[u8]) -> Result<String, String> {
    faraday_files::write(&mut dir(stick), name, bytes)
}

/// Writes a sealed vault back over its own file (`docs/VAULT.md` §6).
pub fn write_vault(stick: &str, name: &str, bytes: &[u8]) -> Result<String, String> {
    faraday_files::write_vault(&mut dir(stick), name, bytes)
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
    /// A file written, over a file of its name when `replace`; the name
    /// it went under. `renamed`: the person renamed it For the stick, and
    /// a vault going over its same vault takes the name
    /// (`faraday_files::write_file`).
    fn write(
        &mut self,
        stick: &str,
        name: &str,
        bytes: &[u8],
        replace: bool,
        renamed: bool,
    ) -> Result<String, String>;
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
    /// A file renamed in place; refused, nothing written, where another
    /// file has the name (`faraday_files::rename`).
    fn rename(&mut self, stick: &str, from: &str, to: &str)
    -> Result<(), faraday_files::Unrenamed>;
}

/// The boot copier (`PLAN.md` §5.5), as a shell reaches it: the process
/// over its pipes on the device, or the copier itself in a test.
pub trait Boot {
    /// The boot partitions it has now.
    fn list(&mut self) -> Result<Vec<BootPart>, String>;
    /// Reads the boot partition holding the running Faraday:
    /// [`StorageEvent::BootSource`] or [`StorageEvent::BootSourceFailed`].
    fn read_source(&mut self) -> StorageEvent;
    /// Writes the source over `target`, reads back and compares:
    /// [`StorageEvent::BootWritten`] or [`StorageEvent::BootWriteFailed`].
    fn write(&mut self, target: &str) -> StorageEvent;
    /// Drops the source.
    fn forget(&mut self);
}

#[cfg(unix)]
mod boot {
    use std::path::{Path, PathBuf};

    use faraday_boot::Boots;
    use faraday_boot::client::Client;
    use faraday_boot::proto::{Request, Response};
    use faraday_core::{BootPart, StorageEvent};

    /// One request to the copier, wherever it is.
    pub trait Ask {
        /// The answer, or why there is none.
        fn ask(&mut self, req: Request) -> Result<Response, String>;
    }

    impl Ask for Client {
        fn ask(&mut self, req: Request) -> Result<Response, String> {
            Client::ask(self, &req)
        }
    }

    impl Ask for Boots {
        fn ask(&mut self, req: Request) -> Result<Response, String> {
            Ok(self.handle(req))
        }
    }

    /// The copier as the stick shell reaches it: its two FIFOs, opened
    /// the first time the app asks, so nothing reads its answers until
    /// then.
    pub struct BootProcess {
        requests: PathBuf,
        responses: PathBuf,
        client: Option<Client>,
    }

    impl BootProcess {
        /// The copier behind these FIFOs, not opened yet.
        pub fn new(requests: &Path, responses: &Path) -> BootProcess {
            BootProcess {
                requests: requests.to_path_buf(),
                responses: responses.to_path_buf(),
                client: None,
            }
        }
    }

    impl Ask for BootProcess {
        fn ask(&mut self, req: Request) -> Result<Response, String> {
            if self.client.is_none() {
                self.client =
                    Some(Client::open(&self.requests, &self.responses).map_err(|e| e.to_string())?);
            }
            match self.client.as_mut() {
                Some(c) => c.ask(&req),
                None => Err("the boot copier cannot be reached".into()),
            }
        }
    }

    impl<T: Ask> super::Boot for T {
        fn list(&mut self) -> Result<Vec<BootPart>, String> {
            match self.ask(Request::List)? {
                Response::Parts(parts) => Ok(parts
                    .into_iter()
                    .map(|p| BootPart {
                        id: p.id,
                        size: p.size,
                        release: p.release,
                        source: p.source,
                    })
                    .collect()),
                Response::Failed(why) => Err(why),
                _ => Err("an answer that is not a listing".into()),
            }
        }

        fn read_source(&mut self) -> StorageEvent {
            let reason = match self.ask(Request::ReadSource) {
                Ok(Response::Source { id, release, size }) => {
                    return StorageEvent::BootSource { id, release, size };
                }
                Ok(Response::Failed(why)) | Err(why) => why,
                Ok(_) => "an answer that is not the source".into(),
            };
            StorageEvent::BootSourceFailed { reason }
        }

        fn write(&mut self, target: &str) -> StorageEvent {
            let (reason, pulled) = match self.ask(Request::Write {
                target: target.to_string(),
            }) {
                Ok(Response::Written { id, release }) => {
                    return StorageEvent::BootWritten { id, release };
                }
                Ok(Response::Pulled) => ("the stick was removed".to_string(), true),
                Ok(Response::Failed(why)) | Err(why) => (why, false),
                Ok(_) => ("an answer that is not a write".to_string(), false),
            };
            StorageEvent::BootWriteFailed {
                id: target.to_string(),
                reason,
                pulled,
            }
        }

        fn forget(&mut self) {
            let _ = self.ask(Request::Forget);
        }
    }
}

#[cfg(unix)]
pub use boot::BootProcess;

/// While the app is upgrading a stick, tells it which boot partitions the
/// copier has: the shell calls this as it looks at the sticks.
pub fn boot_poll(app: &mut Faraday, boot: &mut dyn Boot) {
    if app.upgrading()
        && let Ok(parts) = boot.list()
    {
        app.storage(StorageEvent::Boots(parts));
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
        stick: &str,
        name: &str,
        bytes: &[u8],
        replace: bool,
        renamed: bool,
    ) -> Result<String, String> {
        faraday_files::write_file(&mut dir(stick), name, bytes, replace, renamed)
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
        stick: &str,
        from: &str,
        to: &str,
    ) -> Result<(), faraday_files::Unrenamed> {
        faraday_files::rename(&mut dir(stick), from, to)
    }
}

/// The memory this machine has free for a vault's Argon2id, in MiB, from
/// the system's own estimate: Linux's `MemAvailable`, Windows' available
/// physical memory. `None` where there is none (macOS for now): the vault
/// then offers its largest unlock memory and checks none against this
/// computer.
pub fn memory_available_mib() -> Option<u32> {
    #[cfg(windows)]
    {
        windows_memory::available_mib()
    }
    #[cfg(not(windows))]
    {
        let info = fs::read_to_string("/proc/meminfo").ok()?;
        let line = info.lines().find(|l| l.starts_with("MemAvailable:"))?;
        let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
        u32::try_from(kib / 1024).ok()
    }
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
    Err(format!("no free name for {name}"))
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
    serve_all(app, boxes, host, sticks, None)
}

/// [`serve_with`], and the boot copier's requests answered by `boot`
/// (`PLAN.md` §5.5); without one they are refused.
pub fn serve_all(
    app: &mut Faraday,
    boxes: &mut Boxes,
    host: Option<&Host>,
    sticks: &mut dyn Sticks,
    mut boot: Option<&mut dyn Boot>,
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
        if !app.storage_pending() {
            break;
        }
        serve_queued(app, boxes, host, print, sticks, &mut boot, &mut said);
    }
    said
}

/// Answers the storage requests the app has queued now.
fn serve_queued(
    app: &mut Faraday,
    boxes: &mut Boxes,
    host: Option<&Host>,
    print: Option<&Path>,
    sticks: &mut dyn Sticks,
    boot: &mut Option<&mut dyn Boot>,
    said: &mut Vec<String>,
) {
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                let ev = match sticks.read(&stick, &name) {
                    Ok(bytes) => StorageEvent::Read { stick, name, bytes },
                    Err(reason) => StorageEvent::ReadFailed {
                        stick,
                        name,
                        reason,
                    },
                };
                app.storage(ev);
            }
            StorageCommand::Write {
                stick,
                name,
                bytes,
                replace,
                renamed,
            } => {
                let written = sticks.write(&stick, &name, &bytes, replace, renamed);
                let ev = match written {
                    Ok(wrote_as) => {
                        said.push(format!("wrote {stick}/{wrote_as}"));
                        StorageEvent::Written {
                            stick,
                            name,
                            wrote_as,
                        }
                    }
                    Err(reason) => {
                        said.push(format!("cannot write {stick}/{name}: {reason}"));
                        StorageEvent::WriteFailed {
                            stick,
                            name,
                            reason,
                        }
                    }
                };
                app.storage(ev);
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
                        said.push(format!("cannot zero {stick}: {reason}"));
                        StorageEvent::ZeroFailed { stick, reason }
                    }
                };
                app.storage(ev);
            }
            StorageCommand::MakeEmpty { stick } => {
                let ev = match sticks.make_empty(&stick) {
                    Ok(()) => {
                        said.push(format!("made {stick} empty"));
                        StorageEvent::Zeroing {
                            stick,
                            next: 2,
                            total: 0,
                            done: false,
                        }
                    }
                    Err(reason) => {
                        said.push(format!("cannot make {stick} empty: {reason}"));
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
            StorageCommand::Rename { stick, from, to } => {
                let refused = sticks.rename(&stick, &from, &to).err().map(|r| match r {
                    faraday_files::Unrenamed::Taken => faraday_core::Unrenamed::Taken,
                    faraday_files::Unrenamed::SameVault => faraday_core::Unrenamed::SameVault,
                    faraday_files::Unrenamed::Failed(why) => faraday_core::Unrenamed::Failed(why),
                });
                said.push(match &refused {
                    None => format!("renamed {stick}/{from} {to}"),
                    Some(r) => format!("cannot rename {stick}/{from}: {r:?}"),
                });
                app.storage(StorageEvent::Renamed {
                    stick,
                    from,
                    to,
                    refused,
                });
            }
            // A file read for its Details' SHA-256: the bytes are wiped
            // here, and only the hash goes to the app.
            StorageCommand::Hash { stick, name } => {
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
                        reason: "this device does not print".into(),
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
                        reason: "this device does not read this computer's files".into(),
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
                        reason: "this device does not save downloads".into(),
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
                    said.push(format!("cannot open nakamotoinstitute.org: {e}"));
                }
            }
            StorageCommand::OpenDownloads => {
                if let Some(h) = host.filter(|h| h.opens)
                    && let Err(e) = open_folder(&h.downloads)
                {
                    said.push(format!("cannot open {}: {e}", h.downloads.display()));
                }
            }
            StorageCommand::BootRead => {
                let ev = match boot.as_deref_mut() {
                    Some(b) => b.read_source(),
                    None => StorageEvent::BootSourceFailed {
                        reason: "this device does not upgrade sticks".into(),
                    },
                };
                said.push(format!("boot copier: {ev:?}"));
                app.storage(ev);
            }
            StorageCommand::BootWrite { target } => {
                let ev = match boot.as_deref_mut() {
                    Some(b) => b.write(&target),
                    None => StorageEvent::BootWriteFailed {
                        id: target,
                        reason: "this device does not upgrade sticks".into(),
                        pulled: false,
                    },
                };
                said.push(format!("boot copier: {ev:?}"));
                app.storage(ev);
                // What it wrote changes the listing.
                if let Some(b) = boot.as_deref_mut() {
                    boot_poll(app, b);
                }
            }
            StorageCommand::BootForget => {
                if let Some(b) = boot.as_deref_mut() {
                    b.forget();
                }
            }
            StorageCommand::SaveBoxes {
                inbox,
                outbox,
                kept,
            } => {
                if let Err(e) = boxes.save(inbox, outbox, kept) {
                    said.push(format!("cannot keep the Inbox and Outbox: {e}"));
                    app.storage(StorageEvent::NotKept);
                }
            }
        }
    }
}

/// The disk process, from the shell's side (`PLAN.md` §4.3): requests go
/// down one FIFO and answers come up the other. A thread reads the
/// answers, so a disk process that never answers costs a timeout rather
/// than the shell; an answer that comes after its timeout is known by its
/// sequence number and dropped.
pub struct DiskProcess {
    tx: fs::File,
    answers: std::sync::mpsc::Receiver<(u32, Vec<u8>)>,
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
        })
    }

    /// An answer that is not to the request asked now: the picture's,
    /// kept for [`Sticks::pictures_read`]; any other is a late one,
    /// wiped.
    fn aside(&mut self, seq: u32, mut frame: Vec<u8>) {
        use faraday_files::proto::Response;
        if self.picture.as_ref().is_some_and(|p| p.0 == seq)
            && let Some((_, stick, name, _)) = self.picture.take()
        {
            let picture = match Response::decode(&frame) {
                Ok(Response::Qr { bytes, codes }) => Ok((bytes, codes)),
                Ok(Response::Failed(why)) => Err(why),
                Ok(_) => Err("an answer that is not codes".to_string()),
                Err(_) => Err("a malformed answer".to_string()),
            };
            self.pictures_done.push((stick, name, picture));
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
        let wait = if matches!(req, Request::Write { .. }) {
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
                    return answer.map_err(|_| "a malformed answer".to_string());
                }
                Ok((seq, other)) => {
                    self.aside(seq, other);
                    continue;
                }
                Err(_) => return Err("the disk process does not answer".to_string()),
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
                    })
                    .collect())
            }
            Response::Failed(why) => Err(why),
            _ => Err("an answer that is not a listing".to_string()),
        }
    }
}

impl Sticks for DiskProcess {
    fn read(&mut self, stick: &str, name: &str) -> Result<Vec<u8>, String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::Read {
            stick: stick.into(),
            name: name.into(),
        })? {
            Response::Bytes(b) => Ok(b),
            Response::Failed(why) => Err(why),
            _ => Err("an answer that is not a file".to_string()),
        }
    }

    fn write(
        &mut self,
        stick: &str,
        name: &str,
        bytes: &[u8],
        replace: bool,
        renamed: bool,
    ) -> Result<String, String> {
        use faraday_files::proto::{Request, Response};
        let mut req = Request::Write {
            stick: stick.into(),
            name: name.into(),
            replace,
            renamed,
            bytes: bytes.to_vec(),
        };
        let answer = self.ask(&req);
        req.wipe();
        match answer? {
            Response::Written(n) => Ok(n),
            Response::Failed(why) => Err(why),
            _ => Err("an answer that is not a write".to_string()),
        }
    }

    fn wipe(&mut self, stick: &str, name: &str) -> Result<(), String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::Wipe {
            stick: stick.into(),
            name: name.into(),
        })? {
            Response::Wiped => Ok(()),
            Response::Failed(why) => Err(why),
            _ => Err("an answer that is not a delete".to_string()),
        }
    }

    fn space(&mut self, stick: &str) -> Result<(u64, u64), String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::Space {
            stick: stick.into(),
        })? {
            Response::Space { free, size } => Ok((free, size)),
            Response::Failed(why) => Err(why),
            _ => Err("an answer that is not a stick's space".to_string()),
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
            _ => Err("an answer that is not zeroing".to_string()),
        }
    }

    fn make_empty(&mut self, stick: &str) -> Result<(), String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::MakeEmpty {
            stick: stick.into(),
        })? {
            Response::Zeroing { .. } => Ok(()),
            Response::Failed(why) => Err(why),
            _ => Err("an answer that is not zeroing".to_string()),
        }
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
            let why = "the disk process does not answer".to_string();
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
        stick: &str,
        from: &str,
        to: &str,
    ) -> Result<(), faraday_files::Unrenamed> {
        use faraday_files::Unrenamed as U;
        use faraday_files::proto::{Request, Response};
        match self
            .ask(&Request::Rename {
                stick: stick.into(),
                from: from.into(),
                to: to.into(),
            })
            .map_err(U::Failed)?
        {
            Response::Renamed => Ok(()),
            Response::NameTaken { same_vault: false } => Err(U::Taken),
            Response::NameTaken { same_vault: true } => Err(U::SameVault),
            Response::Failed(why) => Err(U::Failed(why)),
            _ => Err(U::Failed("an answer that is not a rename".to_string())),
        }
    }
}
