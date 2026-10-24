//! The Starter on Windows, on Windows: [`Windows`] answers the
//! platform seam from the calls in `sys` and the readings in the pure
//! modules, and [`elevated_write`] is the elevated copy's whole life
//! when it writes ([`elevated_read_version`] when it reads a stick's
//! version for keep data).

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use faraday_core::strings::EN;
use faraday_core::{Program, parse_version_output};
use faraday_payload::{ImageKind, ImageSum, Sha256Sum};
use faraday_starter::api::{
    BitLocker, BitLockerStep, Bus, Computer, Concern, Disk, Installed, Mode, Protection, Reason,
    RecoveryKey, Source, StarterEvent, WriteOrder, WriteStage,
};
use faraday_starter::concern;
use windows_sys::Win32::Foundation::GENERIC_READ;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_NO_BUFFERING, FILE_FLAG_WRITE_THROUGH, FILE_SHARE_READ, FILE_SHARE_WRITE,
    IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
};
use windows_sys::Win32::System::Ioctl::{
    FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
    IOCTL_DISK_GET_DRIVE_LAYOUT_EX, IOCTL_DISK_GET_LENGTH_INFO, IOCTL_DISK_UPDATE_PROPERTIES,
    IOCTL_STORAGE_QUERY_PROPERTY,
};
use windows_sys::Win32::System::Registry::HKEY_LOCAL_MACHINE;
use windows_sys::Win32::UI::Shell::{FOLDERID_Downloads, FOLDERID_LocalAppData, FOLDERID_Programs};

use zeroize::Zeroizing;

use super::bitlocker::{self, Answer, Order, Reply, Verb};
use super::devices::{self, Output};
use super::disks::{RawDisk, RawVolume, Snapshot, disk_device, disk_id, disk_number, stick_read};
use super::elevated::{self, Elevated, ReadVersion};
use super::install::{self, Dirs, Left};
use super::pipe::{self, Message};
use super::registry::{self, Facts};
use super::sys::{self, Elevation};
use crate::payload::{Image, Payload};
use crate::platform::Platform;
use crate::stick;
use crate::writer::{self, Device, Identity, Job};

/// The most `\\.\PhysicalDriveN` looked at.
const MAX_DISKS: u32 = 64;
/// How often the disks are read again. Polling, not `WM_DEVICECHANGE`:
/// that needs a window of the Starter's own and a second message loop
/// beside winit's, and the reading costs a few milliseconds (no disk is
/// read, only asked about); a stick plugged in shows within this time.
const POLL: Duration = Duration::from_millis(1500);
/// How long a copy gets to say its version.
const VERSION_WAIT: Duration = Duration::from_secs(5);
/// `CREATE_NO_WINDOW`: a console program started with no window.
const NO_WINDOW: u32 = 0x0800_0000;
/// How long a started desktop app gets to fail before it counts as started.
const START_WAIT: Duration = Duration::from_millis(400);
/// `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`: a program started on its
/// own, which closing the Starter does not end.
const DETACHED: u32 = 0x0000_0008 | 0x0000_0200;
/// How often progress goes over the pipe.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// The command that removes the installed copy once the Starter exits.
static AFTER_EXIT: Mutex<Option<String>> = Mutex::new(None);

/// The Starter on Windows.
pub struct Windows;

impl Platform for Windows {
    fn computer(&self) -> Computer {
        let bios = |v: &str| sys::reg_get(HKEY_LOCAL_MACHINE, registry::BIOS, v);
        registry::computer(&Facts {
            manufacturer: bios("SystemManufacturer"),
            product: bios("SystemProductName"),
            family: bios("SystemFamily"),
            secure_boot: sys::reg_get(
                HKEY_LOCAL_MACHINE,
                registry::SECURE_BOOT,
                "UEFISecureBootEnabled",
            ),
            firmware: sys::firmware_type(),
            machine: sys::native_machine(),
        })
    }

    fn concerns(&self) -> Vec<Concern> {
        let c = self.computer();
        let vm = concern::virtual_machine(&c.maker, &c.model, sys::hypervisor_guest());
        let outputs: Vec<Output> = sys::display_outputs()
            .into_iter()
            .map(|mut o| {
                o.edid = devices::edid_key(&o.path)
                    .and_then(|k| sys::reg_get(HKEY_LOCAL_MACHINE, &k, "EDID"));
                o
            })
            .collect();
        devices::concerns(vm, &outputs, &sys::usb_devices())
    }

    fn other_encryption(&self) -> Option<String> {
        sys::reg_get(HKEY_LOCAL_MACHINE, registry::VERACRYPT, "Start")
            .map(|_| String::from("VeraCrypt"))
    }

    fn bitlocker(&self) -> Option<BitLocker> {
        let drive = system_drive();
        let protection = bitlocker::shell_protection(sys::bitlocker_protection(&drive));
        Some(BitLocker {
            drive,
            protection,
            starts_left: None,
        })
    }

    fn bitlocker_step(&self, step: BitLockerStep) -> (Result<(), Reason>, Option<BitLocker>) {
        let verb = match step {
            BitLockerStep::Suspend => Verb::Suspend,
            BitLockerStep::Resume => Verb::Resume,
        };
        let reply = match elevate_bitlocker(verb) {
            Ok(r) => r,
            Err(why) => return (Err(why), None),
        };
        let read = reply.state.map(|(protection, starts_left)| BitLocker {
            drive: system_drive(),
            protection,
            starts_left: starts_left.filter(|_| protection == Protection::Suspended),
        });
        let call = match step {
            BitLockerStep::Suspend => "DisableKeyProtectors",
            BitLockerStep::Resume => "EnableKeyProtectors",
        };
        let result = reply.done.unwrap_or_else(|| {
            Err(Reason::BitLocker {
                call: call.to_string(),
                code: 0,
            })
        });
        (result, read)
    }

    fn recovery_key(&self) -> Result<RecoveryKey, Reason> {
        let reply = elevate_bitlocker(Verb::Key)?;
        match (reply.done, reply.key) {
            (Some(Ok(())), Some(k)) => Ok(k),
            (Some(Err(why)), _) => Err(why),
            _ => Err(Reason::BitLocker {
                call: String::from("GetKeyProtectorNumericalPassword"),
                code: 0,
            }),
        }
    }

    fn read_version(&self, disk: &str, serial: &str, size: u64) -> Result<Option<String>, Reason> {
        elevate_read_version(disk, serial, size)
    }

    fn watch(&self, out: Sender<StarterEvent>, forget: Receiver<String>) {
        std::thread::spawn(move || follow(&out, &forget));
    }

    fn write(&self, order: WriteOrder, job: Job, out: Sender<StarterEvent>) {
        std::thread::spawn(move || {
            let result = elevate_write(order, &job, &out);
            let _ = out.send(StarterEvent::Written(result));
        });
    }

    fn restart_into_firmware(&self) -> Result<(), Reason> {
        let shutdown = shutdown_exe().map_err(Reason::Restart)?;
        let p = match sys::run_elevated(&shutdown, "/r /fw /t 0") {
            Ok(p) => p,
            Err(Elevation::Declined) => return Err(Reason::Declined),
            Err(Elevation::Failed(e)) => return Err(Reason::Restart(e.to_string())),
        };
        while !p.ended(Duration::from_millis(200)) {}
        match p.code() {
            Some(0) => Ok(()),
            code => Err(Reason::Restart(format!(
                "shutdown /r /fw: {}",
                code.map_or_else(|| String::from("?"), |c| c.to_string())
            ))),
        }
    }

    fn restart_to_choose_device(&self) -> Result<(), Reason> {
        let shutdown = shutdown_exe().map_err(Reason::Restart)?;
        let out = Command::new(shutdown)
            .args(["/r", "/o", "/t", "0"])
            .creation_flags(NO_WINDOW)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| Reason::Restart(e.to_string()))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(Reason::Restart(format!("shutdown /r /o: {}", out.status)))
        }
    }

    fn installed(&self) -> Option<Installed> {
        install::installed(&dirs().ok()?, &faraday_of, &mut sys::CurrentUser)
    }

    fn install(&self, payload: Option<&Payload>) -> Result<String, Reason> {
        // The desktop app, decompressed and checked against the
        // manifest, before anything is written.
        let app = match payload {
            Some(p) => p.desktop_bytes()?,
            None => None,
        };
        let dirs = dirs()?;
        let running = running()?;
        let size = std::fs::metadata(&running).map(|m| m.len()).unwrap_or(0);
        let version = faraday_core::BUILD.unwrap_or(faraday_starter::VERSION);
        let steps = install::plan(&dirs, version, EN.starter_menu_name, size, app.as_deref());
        install::install(&dirs, &steps, &running, &faraday_of, &mut sys::CurrentUser)
    }

    fn uninstall(&self) -> Result<(), Reason> {
        let dirs = dirs()?;
        let left = install::uninstall(&dirs, &running()?, &faraday_of, &mut sys::CurrentUser)?;
        if let Left::AfterExit(c) = left
            && let Ok(mut pending) = AFTER_EXIT.lock()
        {
            *pending = Some(c);
        }
        Ok(())
    }

    fn run_desktop(&self, payload: Option<&Payload>) -> Result<(), Reason> {
        let path = install::desktop_app(&dirs()?, &faraday_of, payload)?;
        start(&path)
    }

    fn downloads(&self) -> PathBuf {
        sys::known_folder(&FOLDERID_Downloads)
            .or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("Downloads")))
            .unwrap_or_else(|| PathBuf::from("Downloads"))
    }

    fn exiting(&self) {
        let pending = AFTER_EXIT.lock().ok().and_then(|mut p| p.take());
        if let (Some(args), Some(dir)) = (pending, sys::system_dir()) {
            // Started in the system folder: a command whose own folder
            // is the one it removes cannot remove it.
            let _ = Command::new(dir.join("cmd.exe"))
                .raw_arg(args)
                .current_dir(&dir)
                .creation_flags(NO_WINDOW)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
    }
}

/// The system drive (`C:`), from the Windows folder.
fn system_drive() -> String {
    bitlocker::drive_of(&sys::windows_dir())
}

/// The window's side of a BitLocker step: a pipe, the elevated copy
/// started on it with `--bitlocker`, and everything it answered. The
/// key arrives over the pipe only this account can open, never through a
/// file, read into a buffer wiped after each line. A declined prompt on
/// an account that is not an administrator's says so.
fn elevate_bitlocker(verb: Verb) -> Result<Reply, Reason> {
    let fail = |call: &str, e: std::io::Error| Reason::BitLocker {
        call: call.to_string(),
        code: e.raw_os_error().unwrap_or(0) as u32,
    };
    let name = pipe_name();
    let pipe = sys::pipe(&name).map_err(|e| fail("CreateNamedPipe", e))?;
    let args = Order { verb, pipe: name }.args();
    let child = match sys::run_elevated(&running()?, &elevated::command_line(&args)) {
        Ok(p) => p,
        Err(Elevation::Declined) if sys::administrator() => return Err(Reason::Declined),
        Err(Elevation::Declined) => return Err(Reason::NoAdmin),
        Err(Elevation::Failed(e)) => return Err(fail("ShellExecuteEx", e)),
    };
    loop {
        if sys::pipe_connected(&pipe).map_err(|e| fail("ConnectNamedPipe", e))? {
            break;
        }
        if child.ended(Duration::from_millis(50)) {
            return Err(ended_bitlocker(&child));
        }
    }
    if sys::pipe_client(&pipe) != Some(child.id()) {
        return Err(Reason::BitLocker {
            call: String::from("GetNamedPipeClientProcessId"),
            code: 0,
        });
    }
    let mut pipe = pipe;
    let reply = bitlocker::reply(&mut pipe).map_err(|e| Reason::BitLocker { call: e, code: 0 })?;
    while !child.ended(Duration::from_millis(200)) {}
    if reply.done.is_none() {
        return Err(ended_bitlocker(&child));
    }
    Ok(reply)
}

fn ended_bitlocker(child: &sys::Process) -> Reason {
    Reason::BitLocker {
        call: String::from("--bitlocker"),
        code: child.code().unwrap_or(u32::MAX),
    }
}

/// A new pipe's name, for the elevated copy to answer on.
fn pipe_name() -> String {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!(
        "{}{:x}-{:x}",
        elevated::PIPE_PREFIX,
        std::process::id(),
        stamp
    )
}

fn shutdown_exe() -> Result<PathBuf, String> {
    sys::system_dir()
        .map(|d| d.join("shutdown.exe"))
        .ok_or_else(|| String::from("the system folder is not known"))
}

fn running() -> Result<PathBuf, Reason> {
    std::env::current_exe().map_err(|e| Reason::File {
        path: String::from(install::APP),
        detail: e.to_string(),
    })
}

/// Starts `path` on its own and says why not when it ends at once with a
/// failure.
fn start(path: &Path) -> Result<(), Reason> {
    let fail = |detail: String| Reason::Start {
        path: path.display().to_string(),
        detail,
    };
    let mut child = Command::new(path)
        .creation_flags(DETACHED)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| fail(e.to_string()))?;
    let started = Instant::now();
    while started.elapsed() < START_WAIT {
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => return Err(fail(status.to_string())),
            Ok(Some(_)) => return Ok(()),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(fail(e.to_string())),
        }
    }
    // Reaped when it ends.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn dirs() -> Result<Dirs, Reason> {
    let local = sys::known_folder(&FOLDERID_LocalAppData).ok_or(Reason::NoHome)?;
    let programs = sys::known_folder(&FOLDERID_Programs).ok_or(Reason::NoHome)?;
    Ok(Dirs { local, programs })
}

/// Which Faraday program `path` is and its build string, from what it
/// prints for `--version` within a few seconds.
fn faraday_of(path: &Path) -> Option<(Program, String)> {
    let mut child = Command::new(path)
        .arg("--version")
        .creation_flags(NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(s) if s.success() => break,
            Some(_) => return None,
            None if started.elapsed() > VERSION_WAIT => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    let mut said = String::new();
    child.stdout.take()?.read_to_string(&mut said).ok()?;
    let line = said.lines().next()?.trim();
    parse_version_output(line).map(|(p, b)| (p, b.to_string()))
}

/// Opens `path` (a disk or a volume) with no access: enough for the
/// queries, and no elevation.
fn query(path: &str) -> std::io::Result<File> {
    OpenOptions::new()
        .access_mode(0)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(path)
}

/// Everything about the disks now.
pub fn snapshot() -> Snapshot {
    sys::quiet_errors();
    let reg = |key: &str, value: &str| sys::reg_get(HKEY_LOCAL_MACHINE, key, value);
    let mut s = Snapshot {
        windows: sys::windows_dir(),
        system_partition: reg(registry::SETUP, "SystemPartition")
            .and_then(|b| registry::sz(&b))
            .unwrap_or_default(),
        page_files: reg(registry::MEMORY, "ExistingPageFiles")
            .map(|b| registry::multi_sz(&b))
            .unwrap_or_default(),
        ..Snapshot::default()
    };
    // STORAGE_PROPERTY_QUERY: StorageDeviceProperty, PropertyStandardQuery.
    let standard = [0u8; 12];
    for n in 0..MAX_DISKS {
        let Ok(f) = query(&disk_id(n)) else { continue };
        let Ok(descriptor) = sys::ioctl(&f, IOCTL_STORAGE_QUERY_PROPERTY, &standard, 4096) else {
            continue;
        };
        s.disks.push(RawDisk {
            number: n,
            descriptor,
            geometry: sys::ioctl(&f, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, &[], 256)
                .unwrap_or_default(),
            layout: sys::ioctl(&f, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, &[], 48 + 144 * 256)
                .unwrap_or_default(),
        });
    }
    for name in sys::volumes() {
        let (label, filesystem) = sys::volume_info(&name);
        let extents = query(name.trim_end_matches('\\'))
            .and_then(|f| sys::ioctl(&f, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, &[], 8 + 24 * 64))
            .unwrap_or_default();
        s.volumes.push(RawVolume {
            device: sys::dos_device(&name),
            paths: sys::volume_paths(&name),
            name,
            extents,
            label,
            filesystem,
        });
    }
    s
}

/// Lists the disks now and again every [`POLL`], sending each list
/// that differs from the last; a Faraday stick's version is read once
/// while it stays attached, from its boot volume (read-only, and only
/// two files there; never FARADAYDATA).
fn follow(out: &Sender<StarterEvent>, forget: &Receiver<String>) {
    let mut versions: BTreeMap<String, ((String, u64), Option<String>)> = BTreeMap::new();
    let mut last: Option<Vec<Disk>> = None;
    loop {
        loop {
            match forget.try_recv() {
                Ok(id) => {
                    versions.remove(&id);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        let found = snapshot().disks();
        versions.retain(|id, _| found.iter().any(|f| &f.disk.id == id));
        let mut disks = Vec::new();
        for f in found {
            let mut disk = f.disk;
            if disk.faraday.is_some() {
                let key = (disk.serial.clone(), disk.size);
                let release = match versions.get(&disk.id) {
                    Some((k, r)) if *k == key => r.clone(),
                    _ => {
                        let r = f
                            .boot
                            .as_deref()
                            .and_then(|b| stick::release_at(Path::new(b)));
                        versions.insert(disk.id.clone(), (key, r.clone()));
                        r
                    }
                };
                disk.faraday = Some(stick_read(release));
            }
            disks.push(disk);
        }
        if last.as_ref() != Some(&disks) {
            if out.send(StarterEvent::Disks(disks.clone())).is_err() {
                return;
            }
            last = Some(disks);
        }
        std::thread::sleep(POLL);
    }
}

/// The Starter's side of a write: a pipe, the elevated copy started on
/// it, and what it reports passed on.
fn elevate_write(
    mut order: WriteOrder,
    job: &Job,
    out: &Sender<StarterEvent>,
) -> Result<(), Reason> {
    // The elevated copy starts in the system folder: a developer's image
    // given as a relative path is passed as a full one.
    if let Source::Image { path, .. } = &mut order.source
        && let Ok(full) = std::path::absolute(&*path)
    {
        *path = full.display().to_string();
    }
    let image_size = match job {
        Job::Image(i) => Some(i.sum.size),
        Job::Payload { .. } => None,
    };
    let pipe_name = pipe_name();
    let fail = |e: std::io::Error| Reason::WriteFailed(e.to_string());
    let pipe = sys::pipe(&pipe_name).map_err(fail)?;
    let args = Elevated {
        pipe: pipe_name,
        order,
        image_size,
    }
    .args();
    let child = match sys::run_elevated(&running()?, &elevated::command_line(&args)) {
        Ok(p) => p,
        Err(Elevation::Declined) => return Err(Reason::Declined),
        Err(Elevation::Failed(e)) => return Err(fail(e)),
    };
    // Waits for the elevated copy to connect, or to end without.
    loop {
        if sys::pipe_connected(&pipe).map_err(fail)? {
            break;
        }
        if child.ended(Duration::from_millis(50)) {
            return Err(ended(&child));
        }
    }
    if sys::pipe_client(&pipe) != Some(child.id()) {
        return Err(Reason::WriteFailed(String::from(
            "another program answered on the writer's pipe",
        )));
    }
    let mut reader = BufReader::new(pipe);
    let mut result = None;
    while let Some(m) = pipe::next(&mut reader) {
        match m.map_err(Reason::WriteFailed)? {
            Message::Progress { stage, done, total } => {
                let _ = out.send(StarterEvent::WriteProgress { stage, done, total });
            }
            Message::Done(r) => result = Some(r),
            Message::Release(_) => {}
        }
    }
    while !child.ended(Duration::from_millis(200)) {}
    result.unwrap_or_else(|| Err(ended(&child)))
}

/// The window's side of keep data's version read: a pipe,
/// the elevated copy started on it with `--read-version`, and the
/// release string it answered. A declined prompt on an account that is
/// not an administrator's says so.
fn elevate_read_version(disk: &str, serial: &str, size: u64) -> Result<Option<String>, Reason> {
    let device = disk_number(disk).map_or_else(|| disk.to_string(), disk_device);
    let fail = |detail: String| Reason::Device {
        device: device.clone(),
        detail,
    };
    let name = pipe_name();
    let pipe = sys::pipe(&name).map_err(|e| fail(e.to_string()))?;
    let args = ReadVersion {
        pipe: name,
        disk: disk.to_string(),
        serial: serial.to_string(),
        size,
    }
    .args();
    let child = match sys::run_elevated(&running()?, &elevated::command_line(&args)) {
        Ok(p) => p,
        Err(Elevation::Declined) if sys::administrator() => return Err(Reason::Declined),
        Err(Elevation::Declined) => return Err(Reason::NoAdmin),
        Err(Elevation::Failed(e)) => return Err(fail(e.to_string())),
    };
    let ended = |child: &sys::Process| {
        fail(format!(
            "{} ended with code {}",
            elevated::READ_VERSION,
            child
                .code()
                .map_or_else(|| String::from("?"), |c| c.to_string())
        ))
    };
    loop {
        if sys::pipe_connected(&pipe).map_err(|e| fail(e.to_string()))? {
            break;
        }
        if child.ended(Duration::from_millis(50)) {
            return Err(ended(&child));
        }
    }
    if sys::pipe_client(&pipe) != Some(child.id()) {
        return Err(fail(String::from(
            "another program answered on the Starter's pipe",
        )));
    }
    let mut reader = BufReader::new(pipe);
    let (mut release, mut result) = (None, None);
    while let Some(m) = pipe::next(&mut reader) {
        match m.map_err(fail)? {
            Message::Release(r) => release = Some(r),
            Message::Done(r) => result = Some(r),
            Message::Progress { .. } => {}
        }
    }
    while !child.ended(Duration::from_millis(200)) {}
    match (result, release) {
        (Some(Ok(())), Some(r)) => Ok(r),
        (Some(Err(why)), _) => Err(why),
        _ => Err(ended(&child)),
    }
}

fn ended(child: &sys::Process) -> Reason {
    Reason::WriteFailed(format!(
        "the writer ended with code {}",
        child
            .code()
            .map_or_else(|| String::from("?"), |c| c.to_string())
    ))
}

/// What runs before anything else when the arguments ask for it:
/// `--write` (the elevated copy) and `--record-disks` (this computer's
/// disks as `tests/windows/` records them, on standard output).
pub fn early(args: &[String]) -> Option<ExitCode> {
    match args.first().map(String::as_str) {
        Some(elevated::WRITE) => Some(elevated_write(args)),
        Some(elevated::READ_VERSION) => Some(elevated_read_version(args)),
        Some(bitlocker::BITLOCKER) => Some(elevated_bitlocker(args)),
        Some("--record-bitlocker") => Some(record_bitlocker()),
        Some("--record-disks") => {
            print!("{}", snapshot().record());
            Some(ExitCode::SUCCESS)
        }
        _ => None,
    }
}

/// The elevated copy of a BitLocker step: reads its arguments, refusing
/// a malformed list before anything is opened, connects to the window's
/// pipe, then to WMI, and answers there. The key goes over the pipe from
/// a buffer wiped once it is written.
fn elevated_bitlocker(args: &[String]) -> ExitCode {
    let Ok(order) = Order::parse(args) else {
        return ExitCode::from(2);
    };
    let Ok(mut pipe) = OpenOptions::new().write(true).open(&order.pipe) else {
        return ExitCode::from(3);
    };
    let drive = system_drive();
    let mut wmi = match sys::WmiSession::connect() {
        Ok(w) => w,
        Err(f) => {
            let mut scratch = Zeroizing::new(Vec::new());
            let _ = bitlocker::send(&mut pipe, &Answer::Done(Err(f.into())), &mut scratch);
            return ExitCode::FAILURE;
        }
    };
    if bitlocker::run(&mut wmi, order.verb, &drive, &mut pipe) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `--record-bitlocker`, run as an administrator: this computer's
/// BitLocker as `tests/windows/*.wmi` records it, and the shell's
/// unelevated reading, on standard output. No recovery password is read.
fn record_bitlocker() -> ExitCode {
    let drive = system_drive();
    println!(
        "# shell System.Volume.BitLockerProtection for {drive}: {:?}",
        sys::bitlocker_protection(&drive)
    );
    match sys::WmiSession::connect() {
        Ok(mut wmi) => {
            print!("{}", bitlocker::record(&mut wmi));
            ExitCode::SUCCESS
        }
        Err(f) => {
            println!("# {}: {:08x}", f.call, f.code);
            ExitCode::FAILURE
        }
    }
}

/// The elevated copy: reads its arguments, refusing a malformed list
/// before anything is opened, connects to the Starter's pipe, writes
/// through the shared writer, and reports there.
fn elevated_write(args: &[String]) -> ExitCode {
    let Ok(e) = Elevated::parse(args) else {
        return ExitCode::from(2);
    };
    let Ok(mut pipe) = OpenOptions::new().write(true).open(&e.pipe) else {
        return ExitCode::from(3);
    };
    let mut send = |m: Message| {
        let _ = pipe.write_all(m.encode().as_bytes());
    };
    let result = job(&e).and_then(|job| {
        let n = disk_number(&e.order.disk).ok_or(Reason::Gone)?;
        let mut device = PhysicalDrive {
            n,
            locks: Vec::new(),
        };
        // Keep data's second check: the window could not
        // read the stick's version without elevation, so this copy reads
        // it again and refuses the same build.
        if e.order.mode == Mode::KeepData {
            let build = match &job {
                Job::Payload { file, .. } => Some(file.build_id.as_str()),
                Job::Image(i) => i.build_id.as_deref(),
            };
            let on_stick = release_of(&mut device, &e.order.device)?;
            writer::refuse_same_build(on_stick.as_deref(), build)?;
        }
        let mut last = None::<Instant>;
        let mut progress = |stage: WriteStage, done, total| {
            let now = Instant::now();
            if done == total || last.is_none_or(|t| now - t >= PROGRESS_EVERY) {
                last = Some(now);
                send(Message::Progress { stage, done, total });
            }
        };
        writer::write(&e.order, job.source(), &mut device, &mut progress)
    });
    let ok = result.is_ok();
    send(Message::Done(result));
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The elevated copy that reads a stick's version for keep data:
/// reads its arguments, refusing a malformed list before
/// anything is opened, connects to the window's pipe, checks the disk is
/// the one the window listed, reads its partition table and boot
/// partition, and answers there.
fn elevated_read_version(args: &[String]) -> ExitCode {
    let Ok(r) = ReadVersion::parse(args) else {
        return ExitCode::from(2);
    };
    let Ok(mut pipe) = OpenOptions::new().write(true).open(&r.pipe) else {
        return ExitCode::from(3);
    };
    let mut send = |m: Message| {
        let _ = pipe.write_all(m.encode().as_bytes());
    };
    match read_listed(&r) {
        Ok(release) => {
            send(Message::Release(release));
            send(Message::Done(Ok(())));
            ExitCode::SUCCESS
        }
        Err(why) => {
            send(Message::Done(Err(why)));
            ExitCode::FAILURE
        }
    }
}

/// The release string on the disk `r` names, once it is checked to be
/// the one the window listed: the same serial and size, a disk the
/// Starter offers.
fn read_listed(r: &ReadVersion) -> Result<Option<String>, Reason> {
    let n = disk_number(&r.disk).ok_or(Reason::Gone)?;
    let mut device = PhysicalDrive {
        n,
        locks: Vec::new(),
    };
    let now = device.identity()?;
    if now.serial != r.serial || now.size != r.size {
        return Err(Reason::Changed {
            device: now.device,
            serial: now.serial,
            size: now.size,
        });
    }
    release_of(&mut device, &now.device)
}

/// Opens the disk read-only and reads its partition table and up to
/// `faraday_boot::MAX_PARTITION` bytes of its boot partition: the
/// release string on it (`writer::release_on_stick`).
fn release_of(device: &mut PhysicalDrive, name: &str) -> Result<Option<String>, Reason> {
    let mut f = device.open_read()?;
    let len = device.length(&mut f).map_err(|e| Reason::Device {
        device: name.to_string(),
        detail: e.to_string(),
    })?;
    writer::release_on_stick(&mut f, len, name)
}

/// What the elevated copy writes: the one payload file named, from the
/// payload built into this same file, or the developer's image held to
/// the sum Confirm showed.
fn job(e: &Elevated) -> Result<Job, Reason> {
    match &e.order.source {
        Source::Payload { name, inner_sha256 } => Payload::built_in()
            .and_then(|p| p.job(name, inner_sha256))
            .ok_or(Reason::NotCarried),
        Source::Image { path, sha256 } => {
            let sha256 = Sha256Sum::from_hex(sha256).ok_or(Reason::NotCarried)?;
            let path = PathBuf::from(path);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(Job::Image(Image {
                kind: ImageKind::of_name(&name),
                sum: ImageSum {
                    sha256,
                    size: e.image_size.ok_or(Reason::NotCarried)?,
                },
                boot_only: name.ends_with("-boot.vfat"),
                build_id: None,
                path,
            }))
        }
    }
}

/// `\\.\PhysicalDriveN` for the writer, holding every volume on it
/// locked and dismounted from [`Device::unmount`] until it is dropped.
struct PhysicalDrive {
    n: u32,
    locks: Vec<File>,
}

impl PhysicalDrive {
    fn path(&self) -> String {
        disk_id(self.n)
    }
}

impl Device for PhysicalDrive {
    fn identity(&mut self) -> Result<Identity, Reason> {
        let (identity, bus) = snapshot().identity(self.n).ok_or(Reason::Gone)?;
        // Only a disk the Starter offers is written (§4.2).
        if bus == Bus::Other {
            return Err(Reason::Changed {
                device: identity.device,
                serial: identity.serial,
                size: identity.size,
            });
        }
        Ok(identity)
    }

    fn unmount(&mut self) -> Result<(), Reason> {
        let s = snapshot();
        for v in s.volumes_on(self.n) {
            let path = v.name.trim_end_matches('\\');
            let f = OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .open(path)
                .map_err(|e| Reason::Unmounting(format!("{}: {e}", label(v))))?;
            // A volume in use takes a moment to let go.
            let mut tries = 0;
            while let Err(e) = sys::ioctl(&f, FSCTL_LOCK_VOLUME, &[], 0) {
                tries += 1;
                if tries >= 10 {
                    return Err(Reason::Unmounting(format!("{}: {e}", label(v))));
                }
                std::thread::sleep(Duration::from_millis(300));
            }
            sys::ioctl(&f, FSCTL_DISMOUNT_VOLUME, &[], 0)
                .map_err(|e| Reason::Unmounting(format!("{}: {e}", label(v))))?;
            self.locks.push(f);
        }
        Ok(())
    }

    fn open_write(&mut self) -> Result<File, Reason> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH)
            .open(self.path())
            .map_err(|e| Reason::OpenWrite(e.to_string()))
    }

    fn open_read(&mut self) -> Result<File, Reason> {
        OpenOptions::new()
            .access_mode(GENERIC_READ)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_NO_BUFFERING)
            .open(self.path())
            .map_err(|e| Reason::OpenRead(e.to_string()))
    }

    fn rescan(&mut self) -> Result<(), Reason> {
        let f = query(&self.path()).map_err(|e| Reason::Device {
            device: self.path(),
            detail: e.to_string(),
        })?;
        let done = sys::ioctl(&f, IOCTL_DISK_UPDATE_PROPERTIES, &[], 0);
        self.locks.clear();
        done.map(drop).map_err(|e| Reason::Device {
            device: self.path(),
            detail: e.to_string(),
        })
    }

    fn length(&mut self, file: &mut File) -> std::io::Result<u64> {
        let b = sys::ioctl(file, IOCTL_DISK_GET_LENGTH_INFO, &[], 8)?;
        b.get(..8)
            .and_then(|b| b.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or_else(|| std::io::Error::other("no length"))
    }
}

/// A volume as a person would know it: its letter, else its label.
fn label(v: &RawVolume) -> String {
    v.paths
        .first()
        .cloned()
        .or_else(|| (!v.label.is_empty()).then(|| v.label.clone()))
        .unwrap_or_else(|| v.name.clone())
}
