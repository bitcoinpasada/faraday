//! The macOS calls behind [`Platform`]: the programs run, and the one
//! descriptor received.
//!
//! Writing (`docs/MAC.md` §6): `diskutil unmountDisk` unmounts what
//! macOS mounted from the disk; `/usr/libexec/authopen -stdoutpipe -o
//! <O_RDWR> /dev/rdiskN` asks for the administrator's password in the
//! system's own prompt, opens the raw disk (past the buffer cache) and
//! sends the open descriptor back over the socket that is its standard
//! output, as an `SCM_RIGHTS` message. From authopen's manual
//! (authopen(1), 2013; `-stdoutpipe`, `-o flags`) and Raspberry Pi
//! Imager's macOS code (`src/mac/macfile.cpp`, `openViaHelper` and
//! `MacFile::authOpen`, 2026-09: a `socketpair`, `recvmsg`, the
//! descriptor used only when authopen then exits with status 0;
//! `src/mac/file_operations_macos.cpp`: `/dev/rdisk*` takes whole blocks
//! only, and its size is not `st_size`). Without `-extauth`, authopen
//! shows its prompt itself, so the Starter makes no Security framework
//! call. The manual gives no exit status per cause: a prompt that ends
//! without a descriptor is taken as declined.
//!
//! The same descriptor reads the write back, so one prompt covers both;
//! afterwards `diskutil unmountDisk` again, so the volumes macOS mounts
//! from the new stick are not left mounted while it stays listed.

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;

use faraday_starter::api::{
    Computer, Concern, Disk, FaradayStick, Installed, Reason, StarterEvent, WriteOrder,
};

use super::diskutil::{Answers, BootPart, whole_of};
use super::{facts, install, plist};
use crate::platform::Platform;
use crate::stick;
use crate::writer::{self, Device, Identity, Job};

const DISKUTIL: &str = "/usr/sbin/diskutil";
const SYSCTL: &str = "/usr/sbin/sysctl";
const UNAME: &str = "/usr/bin/uname";
const DITTO: &str = "/usr/bin/ditto";
const AUTHOPEN: &str = "/usr/libexec/authopen";

/// How often the disks are listed again.
const EVERY: Duration = Duration::from_secs(2);

/// The Starter on macOS.
pub struct MacOs;

/// Runs `program` with `args`: what it printed, or its words (or its
/// status) when it failed.
fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(if said.is_empty() {
        format!("{program} {}: {}", args.join(" "), out.status)
    } else {
        said
    })
}

fn answers() -> facts::Answers {
    let ask = |program: &str, args: &[&str]| run(program, args).ok();
    facts::Answers {
        model: ask(SYSCTL, &["-n", "hw.model"]),
        machine: ask(UNAME, &["-m"]),
        translated: ask(SYSCTL, &["-n", "sysctl.proc_translated"]),
        vmm: ask(SYSCTL, &["-n", "kern.hv_vmm_present"]),
    }
}

fn list() -> Result<Answers, String> {
    Answers::from_list(&run(DISKUTIL, &["list", "-plist"])?)
}

/// Adds `diskutil info` for `id`; a failure leaves it out, which leaves
/// its disk unlisted.
fn add_info(a: &mut Answers, id: &str) {
    let r = run(DISKUTIL, &["info", "-plist", id]).and_then(|t| a.add_info(id, &t));
    if let Err(e) = r {
        eprintln!("faraday-starter: diskutil info {id}: {e}");
    }
}

impl Platform for MacOs {
    fn computer(&self) -> Computer {
        facts::computer(&answers())
    }

    fn concerns(&self) -> Vec<Concern> {
        facts::concerns(&answers())
    }

    fn watch(&self, out: Sender<StarterEvent>, forget: Receiver<String>) {
        std::thread::spawn(move || follow(&out, &forget));
    }

    fn write(&self, order: WriteOrder, job: Job, out: Sender<StarterEvent>) {
        let id = order.disk.clone();
        writer::spawn(order, job, out, move || {
            if whole_of(&id) != Some(id.as_str()) {
                return Err(Reason::Gone);
            }
            Ok(Box::new(Raw {
                id,
                size: 0,
                block: 512,
                file: None,
            }) as Box<dyn Device>)
        });
    }

    /// A Mac has no firmware settings to restart into.
    fn restart_into_firmware(&self) -> Result<(), Reason> {
        Err(Reason::Unsupported)
    }

    fn installed(&self) -> Option<Installed> {
        install::installed(&install::Dirs::from_env().ok()?)
    }

    fn install(&self) -> Result<String, Reason> {
        let dirs = install::Dirs::from_env()?;
        let running = std::env::current_exe().map_err(|e| Reason::File {
            path: String::from("faraday"),
            detail: e.to_string(),
        })?;
        install::install(&dirs, &running, &mut |from, to| {
            let out = Command::new(DITTO)
                .arg(from)
                .arg(to)
                .stdin(Stdio::null())
                .output()
                .map_err(|e| format!("{DITTO}: {e}"))?;
            if out.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
            }
        })
    }

    fn uninstall(&self) -> Result<(), Reason> {
        install::uninstall(&install::Dirs::from_env()?)
    }

    fn downloads(&self) -> PathBuf {
        install::Dirs::from_env()
            .map(|d| d.downloads())
            .unwrap_or_else(|_| PathBuf::from("Downloads"))
    }
}

/// What was read of one whole disk: `diskutil list`'s entry for it when
/// its `info` was read, and those answers, its own and its partitions'.
type Seen = (Option<plist::Value>, Vec<(String, String)>);

/// Lists the disks every two seconds, sending each list that differs
/// from the last. A disk's `info` is read again only when `diskutil
/// list` says something else of it than before, or `forget` names it
/// (it was written); a Faraday stick's version is read once while it
/// stays attached.
fn follow(out: &Sender<StarterEvent>, forget: &Receiver<String>) {
    let mut read: BTreeMap<String, Seen> = BTreeMap::new();
    let mut versions: BTreeMap<String, (u64, Option<String>)> = BTreeMap::new();
    let mut last: Option<Vec<Disk>> = None;
    let mut failed = false;
    loop {
        loop {
            match forget.try_recv() {
                Ok(id) => {
                    read.remove(&id);
                    versions.remove(&id);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        match list() {
            Ok(mut a) => {
                failed = false;
                let wholes: Vec<String> = a.whole_disks().into_iter().map(str::to_string).collect();
                read.retain(|id, _| wholes.contains(id));
                versions.retain(|id, _| wholes.contains(id));
                for id in &wholes {
                    let entry = a.listed(id).cloned();
                    let fresh = read.get(id).is_some_and(|(e, _)| *e == entry);
                    if !fresh {
                        let ids: Vec<String> = std::iter::once(id.clone())
                            .chain(a.partitions(id).into_iter().map(str::to_string))
                            .collect();
                        let texts = ids
                            .into_iter()
                            .filter_map(|p| match run(DISKUTIL, &["info", "-plist", &p]) {
                                Ok(t) => Some((p, t)),
                                Err(e) => {
                                    eprintln!("faraday-starter: diskutil info {p}: {e}");
                                    None
                                }
                            })
                            .collect();
                        read.insert(id.clone(), (entry, texts));
                    }
                    for (p, t) in &read[id].1 {
                        if let Err(e) = a.add_info(p, t) {
                            eprintln!("faraday-starter: diskutil info {p}: {e}");
                        }
                    }
                }
                let mut disks = Vec::new();
                for f in a.disks() {
                    let mut disk = f.disk;
                    if let Some(boot) = f.boot {
                        let release = match versions.get(&disk.id) {
                            Some((size, r)) if *size == disk.size => r.clone(),
                            _ => {
                                let r = read_version(&boot);
                                versions.insert(disk.id.clone(), (disk.size, r.clone()));
                                r
                            }
                        };
                        disk.faraday = Some(FaradayStick {
                            release,
                            read: true,
                        });
                    }
                    disks.push(disk);
                }
                if last.as_ref() != Some(&disks) {
                    if out.send(StarterEvent::Disks(disks.clone())).is_err() {
                        return;
                    }
                    last = Some(disks);
                }
            }
            Err(e) if last.is_none() => {
                eprintln!("faraday-starter: diskutil does not answer: {e}");
                let _ = out.send(StarterEvent::NoWriter);
                return;
            }
            Err(e) => {
                if !failed {
                    eprintln!("faraday-starter: diskutil: {e}");
                }
                failed = true;
            }
        }
        std::thread::sleep(EVERY);
    }
}

/// The version on a Faraday stick's boot partition (§4.1): read where
/// macOS mounted it, or mounted read-only for the moment it takes and
/// unmounted again. The data partition is never mounted, opened or read.
fn read_version(boot: &BootPart) -> Option<String> {
    if let Some(at) = &boot.mounted {
        return stick::release_at(at);
    }
    let id = boot.id.as_str();
    if whole_of(id).is_none_or(|w| w == id) {
        return None;
    }
    if let Err(e) = run(DISKUTIL, &["mount", "readOnly", id]) {
        eprintln!("faraday-starter: the stick's boot partition could not be read: {e}");
        return None;
    }
    let at = run(DISKUTIL, &["info", "-plist", id])
        .ok()
        .and_then(|t| plist::parse(&t).ok())
        .and_then(|v| {
            v.get("MountPoint")
                .and_then(plist::Value::as_str)
                .map(PathBuf::from)
        })
        .filter(|p| !p.as_os_str().is_empty());
    let release = at.as_deref().and_then(stick::release_at);
    if let Err(e) = run(DISKUTIL, &["unmount", id]) {
        eprintln!("faraday-starter: unmounting the stick's boot partition: {e}");
    }
    release
}

/// A whole disk to write, `/dev/diskN`, reached as `/dev/rdiskN`.
struct Raw {
    id: String,
    /// Its size and block size, as [`Device::identity`] last read them.
    size: u64,
    block: u64,
    /// authopen's descriptor, once opened.
    file: Option<File>,
}

impl Raw {
    fn device(&self) -> String {
        format!("/dev/{}", self.id)
    }
}

impl Device for Raw {
    fn identity(&mut self) -> Result<Identity, Reason> {
        let fail = |detail: String| Reason::Device {
            device: format!("/dev/{}", self.id),
            detail,
        };
        let mut a = list().map_err(fail)?;
        let ids: Vec<String> = std::iter::once(self.id.clone())
            .chain(a.partitions(&self.id).into_iter().map(str::to_string))
            .collect();
        for p in &ids {
            add_info(&mut a, p);
        }
        let (now, block) = a.identity(&self.id).ok_or(Reason::Gone)?;
        self.size = now.size;
        self.block = block;
        Ok(now)
    }

    fn unmount(&mut self) -> Result<(), Reason> {
        run(DISKUTIL, &["unmountDisk", &self.device()])
            .map(|_| ())
            .map_err(Reason::Unmounting)
    }

    fn open_write(&mut self) -> Result<File, Reason> {
        let file = authopen(&format!("/dev/r{}", self.id))?;
        let ours = file
            .try_clone()
            .map_err(|e| Reason::OpenWrite(e.to_string()))?;
        self.file = Some(file);
        Ok(ours)
    }

    fn open_read(&mut self) -> Result<File, Reason> {
        self.file
            .as_ref()
            .ok_or_else(|| Reason::OpenRead(self.device()))?
            .try_clone()
            .map_err(|e| Reason::OpenRead(e.to_string()))
    }

    /// The raw disk's `st_size` is 0, so its end is not where a seek
    /// finds it: its size is `diskutil`'s.
    fn length(&mut self, _file: &mut File) -> io::Result<u64> {
        Ok(self.size)
    }

    fn block(&mut self) -> u64 {
        self.block
    }

    fn rescan(&mut self) -> Result<(), Reason> {
        self.file = None;
        self.unmount()
    }
}

/// Opens `path` read and write through authopen, which asks for the
/// administrator's password itself: the descriptor, or why not.
fn authopen(path: &str) -> Result<File, Reason> {
    let (ours, theirs) = UnixStream::pair().map_err(|e| Reason::OpenWrite(e.to_string()))?;
    let flags = libc::O_RDWR.to_string();
    // The command, and with it this process's copy of authopen's end,
    // is dropped once authopen has started, so the socket ends when
    // authopen does.
    let child = Command::new(AUTHOPEN)
        .args(["-stdoutpipe", "-o", &flags, path])
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(theirs)))
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Reason::Start {
            path: String::from(AUTHOPEN),
            detail: e.to_string(),
        })?;
    let received = receive(&ours);
    drop(ours);
    let done = child.wait_with_output().map_err(|e| Reason::Start {
        path: String::from(AUTHOPEN),
        detail: e.to_string(),
    })?;
    let said = String::from_utf8_lossy(&done.stderr).trim().to_string();
    let mut fds = match received {
        Ok(fds) => fds,
        Err(e) => return Err(Reason::OpenWrite(e.to_string())),
    };
    if !done.status.success() {
        // Whatever arrived is closed as it drops.
        eprintln!("faraday-starter: authopen {path}: {} {said}", done.status);
        return Err(if fds.is_empty() {
            Reason::Declined
        } else {
            Reason::OpenWrite(said)
        });
    }
    match (fds.pop(), fds.is_empty()) {
        (Some(fd), true) => Ok(File::from(fd)),
        _ => Err(Reason::OpenWrite(format!("authopen {path}: no descriptor"))),
    }
}

/// The descriptors in the one message on `sock`; none when it closed
/// without one.
#[allow(unsafe_code)]
fn receive(sock: &UnixStream) -> io::Result<Vec<OwnedFd>> {
    let mut data = [0u8; 64];
    // Room for a few descriptors' control messages, aligned for
    // `cmsghdr`.
    let mut control = [0u64; 16];
    let mut fds = Vec::new();
    // SAFETY: `msg` points at `iov`, `data` and `control`, which live to
    // the end of this block and are as long as it says; `recvmsg` writes
    // only within them and sets `msg_controllen` to what it wrote. The
    // control messages are walked with the system's own CMSG_ macros,
    // which stay within `msg_controllen`; each one's header is read
    // unaligned, and its descriptors are read only up to both its own
    // length and the end of what was received. Each descriptor read is
    // one the kernel just installed in this process for this message and
    // nothing else holds, so `OwnedFd` takes sole ownership of it and
    // closes it when dropped.
    unsafe {
        let mut iov = libc::iovec {
            iov_base: data.as_mut_ptr().cast(),
            iov_len: data.len(),
        };
        let mut msg: libc::msghdr = std::mem::zeroed();
        msg.msg_iov = &mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = control.as_mut_ptr().cast();
        msg.msg_controllen = std::mem::size_of_val(&control) as libc::socklen_t;
        loop {
            if libc::recvmsg(sock.as_raw_fd(), &mut msg, 0) >= 0 {
                break;
            }
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::Interrupted {
                return Err(e);
            }
        }
        let end = (msg.msg_control as usize).saturating_add(msg.msg_controllen as usize);
        let mut c = libc::CMSG_FIRSTHDR(&msg);
        while !c.is_null() {
            let h: libc::cmsghdr = std::ptr::read_unaligned(c);
            if h.cmsg_level == libc::SOL_SOCKET && h.cmsg_type == libc::SCM_RIGHTS {
                let first = libc::CMSG_DATA(c);
                let len = (h.cmsg_len as usize)
                    .saturating_sub(libc::CMSG_LEN(0) as usize)
                    .min(end.saturating_sub(first as usize));
                let n = len / std::mem::size_of::<libc::c_int>();
                for i in 0..n {
                    let fd: libc::c_int =
                        std::ptr::read_unaligned(first.cast::<libc::c_int>().add(i));
                    if fd >= 0 {
                        fds.push(OwnedFd::from_raw_fd(fd));
                    }
                }
            }
            c = libc::CMSG_NXTHDR(&msg, c);
        }
        if msg.msg_flags & libc::MSG_CTRUNC != 0 {
            return Err(io::Error::other("authopen's message was cut short"));
        }
    }
    Ok(fds)
}
