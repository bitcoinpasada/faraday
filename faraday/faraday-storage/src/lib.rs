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

use faraday_core::{Faraday, StickInfo, StorageCommand, StorageEvent};
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

/// The QR codes in a PNG in a stick directory.
pub fn read_qr_png(stick: &str, name: &str) -> Result<Vec<Vec<u8>>, String> {
    faraday_files::read_qr_png(&mut dir(stick), name)
}

/// Where the sticks' files are: folders, or the disk process.
pub trait Sticks {
    /// One file's bytes.
    fn read(&mut self, stick: &str, name: &str) -> Result<Vec<u8>, String>;
    /// A file written; the name it went under.
    fn write(&mut self, stick: &str, name: &str, bytes: &[u8]) -> Result<String, String>;
    /// The QR codes in a picture.
    fn read_qr(&mut self, stick: &str, name: &str) -> Result<Vec<Vec<u8>>, String>;
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
    fn read_qr(&mut self, stick: &str, name: &str) -> Result<Vec<Vec<u8>>, String> {
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

/// Answers every storage request the app has queued. `print` is where an
/// online shell saves PDFs; the device passes `None` and refuses. Returns
/// the lines a verbose shell prints.
pub fn serve(app: &mut Faraday, boxes: &mut Boxes, print: Option<&Path>) -> Vec<String> {
    serve_with(app, boxes, print, &mut Dirs)
}

/// [`serve`], with the sticks' files wherever `sticks` keeps them.
pub fn serve_with(
    app: &mut Faraday,
    boxes: &mut Boxes,
    print: Option<&Path>,
    sticks: &mut dyn Sticks,
) -> Vec<String> {
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
                    Ok(payloads) => StorageEvent::QrRead { name, payloads },
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

    /// The partitions handed out now. The boot medium's is called the boot
    /// stick, whatever its label.
    pub fn list(&mut self) -> Result<Vec<StickInfo>, String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::List)? {
            Response::Sticks(s) => Ok(s
                .into_iter()
                .map(|s| StickInfo {
                    label: if s.boot {
                        "Boot stick".to_string()
                    } else {
                        s.label
                    },
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

    fn read_qr(&mut self, stick: &str, name: &str) -> Result<Vec<Vec<u8>>, String> {
        use faraday_files::proto::{Request, Response};
        match self.ask(&Request::ReadQr {
            stick: stick.into(),
            name: name.into(),
        })? {
            Response::Qr(c) => Ok(c),
            Response::Failed(why) => Err(why),
            _ => Err("an answer that is not codes".to_string()),
        }
    }
}
