//! The hub app on Windows, on Windows: [`Windows`] answers the
//! platform seam from the calls in `sys` and the readings in the pure
//! modules, and [`elevated_write`] is the elevated copy's whole life.

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
use faraday_hub::api::{
    BitLocker, Bus, Computer, Concern, Disk, FaradayStick, HubEvent, Installed, KeyShown,
    Protection, Reason, Source, WriteOrder, WriteStage,
};
use faraday_hub::concern;
use faraday_payload::{ImageKind, ImageSum, Sha256Sum};
use faraday_qr::bitlocker::KeyText;
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
use super::disks::{RawDisk, RawVolume, Snapshot, disk_id, disk_number};
use super::elevated::{self, Elevated};
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
/// that needs a window of the hub app's own and a second message loop
/// beside winit's, and the reading costs a few milliseconds (no disk is
/// read, only asked about); a stick plugged in shows within this time.
const POLL: Duration = Duration::from_millis(1500);
/// How long a copy gets to say its version.
const VERSION_WAIT: Duration = Duration::from_secs(5);
/// `CREATE_NO_WINDOW`: a console program started with no window.
const NO_WINDOW: u32 = 0x0800_0000;
/// How often progress goes over the pipe.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// The command that removes the installed copy once the hub app exits.
static AFTER_EXIT: Mutex<Option<String>> = Mutex::new(None);

/// The hub app on Windows.
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

    fn bitlocker(&self) -> Option<BitLocker> {
        let drive = system_drive();
        let marker = dirs().ok().and_then(|d| bitlocker::read_marker(&d.local));
        let mut protection = bitlocker::shell_protection(sys::bitlocker_protection(&drive));
        // The shell may read a suspended drive as not encrypted; the
        // marker says the hub app suspended it.
        if marker.is_some() && protection == Protection::None {
            protection = Protection::Suspended;
        }
        Some(BitLocker {
            drive,
            protection,
            marker: marker.map(|m| m.marker(sys::boot_time())),
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
        devices::concerns(vm, sys::remote_session(), &outputs, &sys::usb_devices())
    }

    fn other_encryption(&self) -> Option<String> {
        sys::reg_get(HKEY_LOCAL_MACHINE, registry::VERACRYPT, "Start")
            .map(|_| String::from("VeraCrypt"))
    }

    fn show_key(&self) -> (Result<KeyShown, Reason>, Option<Protection>) {
        let reply = match elevate_bitlocker(Verb::ShowKey) {
            Ok(r) => r,
            Err(why) => return (Err(why), None),
        };
        let result = match (reply.done, reply.key) {
            (Some(Ok(())), Some(f)) => Ok(KeyShown {
                text: KeyText {
                    computer: computer_name(&self.computer()),
                    drive: f.drive,
                    key: f.key,
                },
                id: f.id,
            }),
            (Some(Err(why)), _) => Err(why),
            _ => Err(Reason::BitLocker {
                call: String::from("GetKeyProtectorNumericalPassword"),
                code: 0,
            }),
        };
        (result, reply.state)
    }

    fn suspend(&self) -> (Result<(), Reason>, Option<Protection>) {
        let reply = match elevate_bitlocker(Verb::Suspend) {
            Ok(r) => r,
            Err(why) => return (Err(why), None),
        };
        let result = match reply.done {
            Some(Ok(())) => {
                // Secure Boot as it is at this moment (§6.3), read again.
                let secure_boot = self.computer().secure_boot;
                let boot = sys::boot_time();
                dirs().and_then(|d| {
                    bitlocker::after_suspend(
                        &d,
                        &reply.suspended,
                        secure_boot,
                        boot,
                        &faraday_of,
                        &mut sys::CurrentUser,
                    )
                })
            }
            Some(Err(why)) => Err(why),
            None => Err(Reason::BitLocker {
                call: String::from("DisableKeyProtectors"),
                code: 0,
            }),
        };
        (result, reply.state)
    }

    fn resume(&self) -> (Result<(), Reason>, Option<Protection>) {
        let d = match dirs() {
            Ok(d) => d,
            Err(why) => return (Err(why), None),
        };
        let Some(marker) = bitlocker::read_marker(&d.local) else {
            // Nothing the hub app suspended: nothing to resume here.
            return (bitlocker::after_resume(&d, &mut sys::CurrentUser), None);
        };
        let reply = match elevate_bitlocker(Verb::Resume(marker.devices())) {
            Ok(r) => r,
            Err(why) => return (Err(why), None),
        };
        let result = match reply.done {
            Some(Ok(())) => bitlocker::after_resume(&d, &mut sys::CurrentUser),
            Some(Err(why)) => Err(why),
            None => Err(Reason::BitLocker {
                call: String::from("EnableKeyProtectors"),
                code: 0,
            }),
        };
        (result, reply.state)
    }

    fn save_key(&self, disk: &str, key: &KeyShown) -> Result<String, Reason> {
        let n = disk_number(disk).ok_or(Reason::Gone)?;
        let s = snapshot();
        let found = s
            .disks()
            .into_iter()
            .find(|f| f.disk.id == disk)
            .ok_or(Reason::Gone)?;
        let d = &found.disk;
        // Only a removable disk that is not a Faraday stick (§6.2).
        if !matches!(d.bus, Bus::Usb | Bus::SdReader) || d.faraday.is_some() || d.in_use.is_some() {
            return Err(Reason::Changed {
                device: d.device.clone(),
                serial: d.serial.clone(),
                size: d.size,
            });
        }
        let v = s
            .volumes_on(n)
            .into_iter()
            .find(|v| !v.filesystem.is_empty())
            .ok_or(Reason::Gone)?;
        let folder = v.paths.first().cloned().unwrap_or_else(|| v.name.clone());
        bitlocker::save_key_to(Path::new(&folder), key, EN.hub_key_file)
            .map(|p| p.display().to_string())
    }

    fn remove_reminder(&self) {
        if let Ok(d) = dirs() {
            let _ = install::remove_reminder(&d, &mut sys::CurrentUser);
        }
    }

    fn watch(&self, out: Sender<HubEvent>, forget: Receiver<String>) {
        std::thread::spawn(move || follow(&out, &forget));
    }

    fn write(&self, order: WriteOrder, job: Job, out: Sender<HubEvent>) {
        std::thread::spawn(move || {
            let result = elevate_write(order, &job, &out);
            let _ = out.send(HubEvent::Written(result));
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

    fn install(&self, desktop: bool, _payload: Option<&Payload>) -> Result<String, Reason> {
        // The desktop app comes to Windows in batch C (§2).
        if desktop {
            return Err(Reason::NoDesktopApp);
        }
        let dirs = dirs()?;
        let running = running()?;
        let size = std::fs::metadata(&running).map(|m| m.len()).unwrap_or(0);
        let version = faraday_core::BUILD.unwrap_or(faraday_hub::VERSION);
        let steps = install::plan(&dirs, version, EN.hub_menu_name, size);
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

    fn run_desktop(&self, _payload: Option<&Payload>) -> Result<(), Reason> {
        Err(Reason::NoDesktopApp)
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

/// The computer's name, as the key's text names it: Windows' name for
/// it, else its maker and model.
fn computer_name(c: &Computer) -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("{} {}", c.maker, c.model).trim().to_string())
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

/// The window's side of a BitLocker step: a pipe, the elevated copy
/// started on it with `--bitlocker`, and everything it answered. The
/// key arrives over the pipe only this account can open, never through a
/// file, read into a buffer wiped after each line. A declined prompt on
/// an account that is not an administrator's says so (§6.4).
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
/// two files there; never OSKDATA).
fn follow(out: &Sender<HubEvent>, forget: &Receiver<String>) {
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
                disk.faraday = Some(FaradayStick { release });
            }
            disks.push(disk);
        }
        if last.as_ref() != Some(&disks) {
            if out.send(HubEvent::Disks(disks.clone())).is_err() {
                return;
            }
            last = Some(disks);
        }
        std::thread::sleep(POLL);
    }
}

/// The hub app's side of a write: a pipe, the elevated copy started on
/// it, and what it reports passed on.
fn elevate_write(mut order: WriteOrder, job: &Job, out: &Sender<HubEvent>) -> Result<(), Reason> {
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
                let _ = out.send(HubEvent::WriteProgress { stage, done, total });
            }
            Message::Done(r) => result = Some(r),
        }
    }
    while !child.ended(Duration::from_millis(200)) {}
    result.unwrap_or_else(|| Err(ended(&child)))
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
        Some(bitlocker::BITLOCKER) => Some(elevated_bitlocker(args)),
        Some("--record-disks") => {
            print!("{}", snapshot().record());
            Some(ExitCode::SUCCESS)
        }
        Some("--record-bitlocker") => Some(record_bitlocker()),
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
    if bitlocker::run(&mut wmi, &order.verb, &drive, &mut pipe) {
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
/// before anything is opened, connects to the hub app's pipe, writes
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
        // Only a disk the hub app offers is written (§4.2).
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
