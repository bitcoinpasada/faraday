//! The seam between the hub app's window and the operating system
//! (`docs/HUB.md` §2): everything that differs from one system to
//! another is behind [`Platform`], and the window calls nothing else.
//! Linux is [`crate::linux`], Windows [`crate::windows`]; macOS comes
//! from its own spec. A system without a module of its own gets [`Elsewhere`],
//! which writes nothing.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use faraday_hub::api::{
    BitLocker, Computer, Concern, HubEvent, Installed, KeyShown, Protection, Reason, WriteOrder,
};

use crate::payload::Payload;
use crate::writer::Job;

/// What the hub app asks of the operating system.
pub trait Platform: Send + Sync {
    /// This computer's facts (§7.1).
    fn computer(&self) -> Computer;

    /// BitLocker on the system drive (§6); `None` where there is none.
    fn bitlocker(&self) -> Option<BitLocker> {
        None
    }

    /// Another encryption product's driver on this computer (§6.4), by
    /// name; `None` where there is none.
    fn other_encryption(&self) -> Option<String> {
        None
    }

    /// Show the recovery key (§6.2), through the elevated copy: the key,
    /// or why not; and the system drive's protection as the elevated copy
    /// read it then, when it did.
    fn show_key(&self) -> (Result<KeyShown, Reason>, Option<Protection>) {
        (Err(Reason::Unsupported), None)
    }

    /// Suspend BitLocker (§6.2), through the elevated copy, then the
    /// marker and, for an installed copy, the logon reminder (§6.3); and
    /// the protection read then.
    fn suspend(&self) -> (Result<(), Reason>, Option<Protection>) {
        (Err(Reason::Unsupported), None)
    }

    /// Resume BitLocker on the marker's volumes (§6.3), through the
    /// elevated copy, then the marker and the logon reminder go; and the
    /// protection read then.
    fn resume(&self) -> (Result<(), Reason>, Option<Protection>) {
        (Err(Reason::Unsupported), None)
    }

    /// Saves the key shown to a file on `disk` (§6.2), which must still
    /// be a removable disk that is not a Faraday stick. Where it went.
    fn save_key(&self, disk: &str, key: &KeyShown) -> Result<String, Reason> {
        let _ = (disk, key);
        Err(Reason::Unsupported)
    }

    /// Removes the logon reminder (§6.3), when it is the hub app's.
    fn remove_reminder(&self) {}

    /// Starts following the disks on a thread of its own: sends
    /// [`HubEvent::Disks`] now and whenever they change, or
    /// [`HubEvent::NoWriter`] once when this computer cannot write a
    /// disk from here. A disk named on `forget` (one just written) has
    /// what is on it read again.
    fn watch(&self, out: Sender<HubEvent>, forget: Receiver<String>);

    /// Writes as confirmed, on a thread of its own: progress and the
    /// result go to `out` ([`HubEvent::WriteProgress`],
    /// [`HubEvent::Written`]).
    fn write(&self, order: WriteOrder, job: Job, out: Sender<HubEvent>);

    /// Restarts into the firmware's settings (§7.3), or says why not.
    fn restart_into_firmware(&self) -> Result<(), Reason>;

    /// Restarts into Windows' advanced startup, whose *Use a device*
    /// starts the stick (§7.3); other systems have none.
    fn restart_to_choose_device(&self) -> Result<(), Reason> {
        Err(Reason::Unsupported)
    }

    /// Restarts from the attached Faraday stick at `device` (§3.1), or
    /// says why not: by default Windows' advanced startup
    /// ([`Platform::restart_to_choose_device`]); Linux starts the stick
    /// once at the next start.
    fn restart_from_stick(&self, device: &str) -> Result<(), Reason> {
        let _ = device;
        self.restart_to_choose_device()
    }

    /// Removes the firmware's `Faraday stick` boot entry that
    /// [`Platform::restart_from_stick`] made, when there is one (Linux,
    /// §3.1); uninstall runs it after its files are gone.
    fn remove_stick_entry(&self) -> Result<(), Reason> {
        Ok(())
    }

    /// Whether the firmware's `Faraday stick` entry exists (Linux,
    /// §3.1c), read without privilege; other systems have none.
    fn boot_entry(&self) -> bool {
        false
    }

    /// What is attached now that is of concern (§3.1a): the screens, the
    /// USB devices, a virtual machine, a remote session. Called every
    /// few seconds off the window's thread. A system not read yet
    /// (macOS) knows nothing, which holds nothing and says nothing.
    fn concerns(&self) -> Vec<Concern> {
        Vec::new()
    }

    /// The hub app is exiting: on Windows an uninstall run from the
    /// installed copy leaves its last step for now (§5.4).
    fn exiting(&self) {}

    /// What is installed (§5.2), found on disk; `None` when nothing is.
    /// May run the installed copy, so it is called off the window's
    /// thread.
    fn installed(&self) -> Option<Installed>;

    /// Installs the running hub app, and with `desktop` the payload's
    /// desktop app, per user (§5.2). Where the hub app went.
    fn install(&self, desktop: bool, payload: Option<&Payload>) -> Result<String, Reason>;

    /// Removes exactly what [`Platform::install`] put here and the
    /// desktop app's cache (§5.4).
    fn uninstall(&self) -> Result<(), Reason>;

    /// Runs the installed desktop app, or the payload's from the cache
    /// after checking it against the manifest (§5.3).
    fn run_desktop(&self, payload: Option<&Payload>) -> Result<(), Reason>;

    /// The folder Transfer sends from and saves into.
    fn downloads(&self) -> PathBuf;
}

/// The operating system this hub app runs on.
pub fn this_computer() -> Arc<dyn Platform> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(crate::linux::Linux)
    }
    #[cfg(windows)]
    {
        Arc::new(crate::windows::Windows)
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        Arc::new(Elsewhere)
    }
}

/// What a system runs before anything else when the arguments (after
/// the program's name) ask for it, and the exit code it ended with:
/// on Windows the elevated writer, `--write` (§2).
pub fn early(args: &[String]) -> Option<std::process::ExitCode> {
    #[cfg(windows)]
    {
        crate::windows::early(args)
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        None
    }
}

/// A system the hub app has no module for yet: no disks, no writer.
pub struct Elsewhere;

impl Platform for Elsewhere {
    fn computer(&self) -> Computer {
        Computer::default()
    }

    fn watch(&self, out: Sender<HubEvent>, _forget: Receiver<String>) {
        let _ = out.send(HubEvent::NoWriter);
    }

    fn write(&self, _order: WriteOrder, _job: Job, out: Sender<HubEvent>) {
        let _ = out.send(HubEvent::Written(Err(Reason::NoWriter)));
    }

    fn restart_into_firmware(&self) -> Result<(), Reason> {
        Err(Reason::Unsupported)
    }

    fn installed(&self) -> Option<Installed> {
        None
    }

    fn install(&self, _desktop: bool, _payload: Option<&Payload>) -> Result<String, Reason> {
        Err(Reason::Unsupported)
    }

    fn uninstall(&self) -> Result<(), Reason> {
        Err(Reason::Unsupported)
    }

    fn run_desktop(&self, _payload: Option<&Payload>) -> Result<(), Reason> {
        Err(Reason::Unsupported)
    }

    fn downloads(&self) -> PathBuf {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Downloads"))
            .unwrap_or_else(|| PathBuf::from("Downloads"))
    }
}
