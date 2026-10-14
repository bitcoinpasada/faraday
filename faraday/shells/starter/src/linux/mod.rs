//! The Starter on Linux (`docs/STARTER.md` §2, §4, §7): disks through
//! udisks2 over D-Bus, never as root; this computer's facts from `/sys`
//! and `/etc/crypttab`; the restart into firmware settings through
//! systemd; the restart from the stick through `efibootmgr` ([`boot`]);
//! what is attached that is of concern from `/sys` ([`devices`]).
//!
//! udisks2's answers are read by [`udisks`], with no D-Bus in it, so the
//! tests feed it recorded answers; [`bus`] is the transport.

pub mod boot;
pub mod bus;
pub mod devices;
pub mod install;
pub mod udisks;

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;

use faraday_starter::api::{
    Computer, Concern, Disk, FaradayStick, Installed, Os, Reason, StarterEvent, WriteOrder,
};

use crate::payload::Payload;
use crate::platform::Platform;
use crate::stick;
use crate::writer::{self, Device, Identity, Job};
use udisks::Objects;

/// How a disk is opened through udisks2's `Block.OpenDevice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Open {
    /// To write: `rw`, `O_EXCL | O_SYNC`.
    Write,
    /// To read back from the disk itself: `r`, `O_DIRECT`.
    ReadDirect,
    /// To read: `r`.
    Read,
}

/// What the Starter asks of udisks2.
pub trait Udisks {
    /// Every object udisks2 manages, as they are now.
    fn objects(&mut self) -> Result<Objects, String>;
    /// Waits up to `timeout` for udisks2 to say something changed.
    fn wait(&mut self, timeout: Duration) -> Result<bool, String>;
    /// Mounts the filesystem `object` read-only, and says where.
    fn mount_ro(&mut self, object: &str) -> Result<PathBuf, String>;
    /// Unmounts the filesystem `object`.
    fn unmount(&mut self, object: &str) -> Result<(), String>;
    /// Opens the block `object`.
    fn open(&mut self, object: &str, how: Open) -> Result<File, String>;
    /// Asks the system to read the block `object`'s partition table again.
    fn rescan(&mut self, object: &str) -> Result<(), String>;
}

/// The Starter on Linux.
pub struct Linux;

impl Platform for Linux {
    fn computer(&self) -> Computer {
        computer(Path::new("/"))
    }

    fn watch(&self, out: Sender<StarterEvent>, forget: Receiver<String>) {
        std::thread::spawn(move || {
            let bus = match bus::Bus::connect() {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("faraday-starter: no udisks2: {e}");
                    let _ = out.send(StarterEvent::NoWriter);
                    return;
                }
            };
            follow(bus, &out, &forget);
        });
    }

    fn write(&self, order: WriteOrder, job: Job, out: Sender<StarterEvent>) {
        let block = order.disk.clone();
        writer::spawn(order, job, out, move || {
            let bus = bus::Bus::connect().map_err(Reason::Bus)?;
            Ok(Box::new(Target { bus, block }) as Box<dyn Device>)
        });
    }

    fn restart_into_firmware(&self) -> Result<(), Reason> {
        let out = std::process::Command::new("systemctl")
            .args(["reboot", "--firmware-setup"])
            .output()
            .map_err(|e| Reason::Restart(format!("systemctl: {e}")))?;
        if out.status.success() {
            return Ok(());
        }
        let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(Reason::Restart(if said.is_empty() {
            format!("systemctl reboot --firmware-setup: {}", out.status)
        } else {
            said
        }))
    }

    fn restart_from_stick(&self, device: &str) -> Result<(), Reason> {
        let path = std::env::var_os("PATH").unwrap_or_default();
        boot::restart_from_stick(&boot::Tools::find(&path)?, device)
    }

    fn remove_stick_entry(&self) -> Result<(), Reason> {
        boot::remove_entry(&std::env::var_os("PATH").unwrap_or_default())
    }

    fn boot_entry(&self) -> bool {
        boot::entry_exists(&std::env::var_os("PATH").unwrap_or_default())
    }

    fn concerns(&self) -> Vec<Concern> {
        devices::read(Path::new("/"), std::env::var("DISPLAY").ok().as_deref())
    }

    fn installed(&self) -> Option<Installed> {
        install::installed(&install::Dirs::from_env().ok()?)
    }

    fn install(&self, desktop: bool, payload: Option<&Payload>) -> Result<String, Reason> {
        let dirs = install::Dirs::from_env()?;
        let running = std::env::current_exe().map_err(|e| Reason::File {
            path: String::from("faraday"),
            detail: e.to_string(),
        })?;
        let app = if desktop {
            Some(payload.ok_or(Reason::NoDesktopApp)?.desktop_app()?.1)
        } else {
            None
        };
        install::install(&dirs, &running, app.as_deref())
    }

    fn uninstall(&self) -> Result<(), Reason> {
        install::uninstall(&install::Dirs::from_env()?)
    }

    fn run_desktop(&self, payload: Option<&Payload>) -> Result<(), Reason> {
        let dirs = install::Dirs::from_env()?;
        let path = install::desktop_app(&dirs, payload)?;
        install::start(&dirs, &path)
    }

    fn downloads(&self) -> PathBuf {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Downloads"))
            .unwrap_or_else(|| PathBuf::from("Downloads"))
    }
}

/// Lists the disks now and again whenever udisks2 says something
/// changed, sending each list that differs from the last; `forget`
/// names a disk whose version is read again (it was written).
pub fn follow<U: Udisks>(mut bus: U, out: &Sender<StarterEvent>, forget: &Receiver<String>) {
    let mut versions = Versions::default();
    let mut last: Option<Vec<Disk>> = None;
    let mut failed = false;
    loop {
        loop {
            match forget.try_recv() {
                Ok(id) => versions.forget(&id),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        match bus.objects() {
            Ok(objects) => {
                failed = false;
                let disks = list(&mut bus, &objects, &mut versions);
                if last.as_ref() != Some(&disks) {
                    if out.send(StarterEvent::Disks(disks.clone())).is_err() {
                        return;
                    }
                    last = Some(disks);
                }
            }
            Err(e) if last.is_none() => {
                eprintln!("faraday-starter: udisks2 does not answer: {e}");
                let _ = out.send(StarterEvent::NoWriter);
                return;
            }
            Err(e) => {
                if !failed {
                    eprintln!("faraday-starter: udisks2: {e}");
                }
                failed = true;
            }
        }
        // Signals bring a change at once; the timeout looks again in
        // case one was missed.
        if let Err(e) = bus.wait(Duration::from_secs(2)) {
            eprintln!("faraday-starter: udisks2: {e}");
            std::thread::sleep(Duration::from_secs(2));
        }
    }
}

/// The versions read from Faraday sticks' boot partitions, by disk, so a
/// stick is read once while it stays attached.
#[derive(Debug, Default)]
pub struct Versions(BTreeMap<String, ((String, u64), Option<String>)>);

impl Versions {
    /// Reads `id`'s version again next time (it was written).
    pub fn forget(&mut self, id: &str) {
        self.0.remove(id);
    }
}

/// The disks `objects` describe, each Faraday stick with the version
/// its boot partition holds (§4.1): read where the desktop mounted it,
/// or mounted read-only for the moment it takes and unmounted again.
/// The data partition is never mounted, opened or read.
pub fn list<U: Udisks + ?Sized>(
    bus: &mut U,
    objects: &Objects,
    versions: &mut Versions,
) -> Vec<Disk> {
    let found = objects.disks();
    versions
        .0
        .retain(|id, _| found.iter().any(|f| &f.disk.id == id));
    let mut disks = Vec::new();
    for f in found {
        let mut disk = f.disk;
        if let Some(boot) = f.boot {
            let key = (disk.serial.clone(), disk.size);
            let release = match versions.0.get(&disk.id) {
                Some((k, release)) if *k == key => release.clone(),
                _ => {
                    let release = read_version(bus, &boot);
                    versions.0.insert(disk.id.clone(), (key, release.clone()));
                    release
                }
            };
            disk.faraday = Some(FaradayStick { release });
        }
        disks.push(disk);
    }
    disks
}

fn read_version<U: Udisks + ?Sized>(bus: &mut U, boot: &udisks::BootPart) -> Option<String> {
    if let Some(at) = &boot.mounted {
        return stick::release_at(at);
    }
    let at = match bus.mount_ro(&boot.object) {
        Ok(at) => at,
        Err(e) => {
            eprintln!("faraday-starter: the stick's boot partition could not be read: {e}");
            return None;
        }
    };
    let release = stick::release_at(&at);
    if let Err(e) = bus.unmount(&boot.object) {
        eprintln!("faraday-starter: unmounting the stick's boot partition: {e}");
    }
    release
}

/// A disk to write, reached through udisks2 by its whole-disk block.
pub struct Target<U: Udisks> {
    /// The connection.
    pub bus: U,
    /// The whole-disk block's object.
    pub block: String,
}

impl<U: Udisks> Device for Target<U> {
    fn identity(&mut self) -> Result<Identity, Reason> {
        self.bus
            .objects()
            .map_err(Reason::Bus)?
            .identity(&self.block)
            .ok_or(Reason::Gone)
    }

    fn unmount(&mut self) -> Result<(), Reason> {
        let objects = self.bus.objects().map_err(Reason::Bus)?;
        for fs in objects.desktop_mounted(&self.block) {
            self.bus.unmount(&fs).map_err(Reason::Unmounting)?;
        }
        Ok(())
    }

    fn open_write(&mut self) -> Result<File, Reason> {
        self.bus
            .open(&self.block, Open::Write)
            .map_err(Reason::OpenWrite)
    }

    fn open_read(&mut self) -> Result<File, Reason> {
        self.bus
            .open(&self.block, Open::ReadDirect)
            .or_else(|_| self.bus.open(&self.block, Open::Read))
            .map_err(Reason::OpenRead)
    }

    fn rescan(&mut self) -> Result<(), Reason> {
        self.bus.rescan(&self.block).map_err(Reason::Bus)
    }
}

/// This computer's facts (§7.1), with `/` at `root`: maker, model and
/// family from DMI, UEFI or not, Secure Boot from its EFI variable, ARM
/// or not, and whether a disk is unlocked by the TPM (§6.4).
pub fn computer(root: &Path) -> Computer {
    let read = |p: &str| {
        std::fs::read_to_string(root.join(p))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let efi = root.join("sys/firmware/efi");
    // Four bytes of attributes, then the value: 1 on, 0 off.
    let secure_boot =
        std::fs::read(efi.join("efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c"))
            .ok()
            .and_then(|v| v.get(4).copied())
            .map(|b| b == 1);
    let arch = match read("proc/sys/kernel/arch") {
        a if a.is_empty() => std::env::consts::ARCH.to_string(),
        a => a,
    };
    Computer {
        os: Os::Linux,
        maker: read("sys/class/dmi/id/sys_vendor"),
        model: read("sys/class/dmi/id/product_name"),
        family: read("sys/class/dmi/id/product_family"),
        uefi: efi.is_dir(),
        arm: arch.starts_with("arm") || arch.starts_with("aarch64"),
        secure_boot,
        tpm_unlock: tpm_unlock(&read("etc/crypttab")),
    }
}

/// Whether a line of `/etc/crypttab` unlocks a disk through the TPM: a
/// `tpm2-device` among its options (the fourth field).
pub fn tpm_unlock(crypttab: &str) -> bool {
    crypttab
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_whitespace().nth(3))
        .any(|opts| opts.split(',').any(|o| o.starts_with("tpm2-device")))
}
