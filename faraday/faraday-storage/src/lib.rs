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

pub use faraday_files::{MAX_PIXELS, MAX_READ};

/// What a stick directory holds at its top level, as the app sees it.
pub fn stick_info(dir: &Path, label: &str, boot: bool) -> StickInfo {
    StickInfo {
        id: dir.display().to_string(),
        label: label.to_string(),
        boot,
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

/// A PNG in a stick directory: its bytes and its QR codes.
pub fn read_qr_png(stick: &str, name: &str) -> Result<faraday_files::Picture, String> {
    faraday_files::read_qr_png(&mut dir(stick), name)
}

/// Where the sticks' files are: folders, or the disk process.
pub trait Sticks {
    /// One file's bytes.
    fn read(&mut self, stick: &str, name: &str) -> Result<Vec<u8>, String>;
    /// A file written; the name it went under.
    fn write(&mut self, stick: &str, name: &str, bytes: &[u8]) -> Result<String, String>;
    /// A picture: its bytes and its QR codes.
    fn read_qr(&mut self, stick: &str, name: &str) -> Result<faraday_files::Picture, String>;
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

/// Sticks that are folders, named by their paths.
pub struct Dirs;

impl Sticks for Dirs {
    fn read(&mut self, stick: &str, name: &str) -> Result<Vec<u8>, String> {
        read(stick, name)
    }
    fn write(&mut self, stick: &str, name: &str, bytes: &[u8]) -> Result<String, String> {
        faraday_files::write_any(&mut dir(stick), name, bytes)
    }
    fn read_qr(&mut self, stick: &str, name: &str) -> Result<faraday_files::Picture, String> {
        read_qr_png(stick, name)
    }
}

/// The memory this machine has free for a vault's Argon2id, in MiB, from
/// the kernel's own estimate.
pub fn memory_available_mib() -> Option<u32> {
    let info = fs::read_to_string("/proc/meminfo").ok()?;
    let line = info.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    u32::try_from(kib / 1024).ok()
}

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
    fn save(
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
                for (sub, files) in [("inbox", inbox), ("outbox", outbox), ("kept", kept)] {
                    let d = dir.join(sub);
                    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
                    for e in fs::read_dir(&d).map_err(|e| e.to_string())?.flatten() {
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
    /// This computer opens a folder in its file manager (`xdg-open`).
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

/// Whether `xdg-open` is on the path, to open a folder with.
pub fn can_open_folders() -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|d| d.join("xdg-open").is_file()))
}

/// The files in a folder of this computer's, newest first, with their
/// sizes: no hidden ones, no folders.
pub fn host_listing(dir: &Path) -> Vec<(String, u64)> {
    let Ok(d) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<(std::time::SystemTime, String, u64)> = d
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            if name.starts_with('.') {
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

/// Opens a folder in this computer's file manager.
fn open_folder(dir: &Path) -> Result<(), String> {
    let mut child = std::process::Command::new("xdg-open")
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

/// Answers every storage request the app has queued. `host` is what an
/// online shell has of its computer's folders; the device passes `None`
/// and refuses to print, read or save there. Returns the lines a verbose
/// shell prints.
pub fn serve(app: &mut Faraday, boxes: &mut Boxes, host: Option<&Host>) -> Vec<String> {
    serve_with(app, boxes, host, &mut Dirs)
}

/// [`serve`], with the sticks' files wherever `sticks` keeps them.
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
            StorageCommand::Write { stick, name, bytes } => {
                let written = sticks.write(&stick, &name, &bytes);
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
            StorageCommand::ReadQr { stick, name } => {
                let ev = match sticks.read_qr(&stick, &name) {
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
                };
                app.storage(ev);
            }
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
                }
            }
        }
    }
    said
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
}

/// How long a list or a read may take.
const ASK: std::time::Duration = std::time::Duration::from_secs(15);
/// How long a write may take: a vault of four 4 MiB slots on a slow stick,
/// written and read back.
const ASK_WRITE: std::time::Duration = std::time::Duration::from_secs(120);

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
        })
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
                Ok((_, mut late)) => {
                    zeroize::Zeroize::zeroize(&mut late);
                    continue;
                }
                Err(_) => return Err("the disk process does not answer".to_string()),
            }
        }
    }

    /// The partitions handed out now, by their volume labels. The app
    /// names the boot medium itself (`Medium::boot`), whatever its label.
    pub fn list(&mut self) -> Result<Vec<StickInfo>, String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::List)? {
            Response::Sticks(s) => Ok(s
                .into_iter()
                .map(|s| StickInfo {
                    label: s.label,
                    id: s.id,
                    boot: s.boot,
                    files: s.files,
                })
                .collect()),
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

    fn write(&mut self, stick: &str, name: &str, bytes: &[u8]) -> Result<String, String> {
        use faraday_files::proto::{Request, Response};
        let mut req = Request::Write {
            stick: stick.into(),
            name: name.into(),
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

    fn read_qr(&mut self, stick: &str, name: &str) -> Result<faraday_files::Picture, String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::ReadQr {
            stick: stick.into(),
            name: name.into(),
        })? {
            Response::Qr { bytes, codes } => Ok((bytes, codes)),
            Response::Failed(why) => Err(why),
            _ => Err("an answer that is not codes".to_string()),
        }
    }
}
