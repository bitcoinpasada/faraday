//! The seam between the Starter's window and the operating system
//! (`docs/STARTER.md` §2): everything that differs from one system to
//! another is behind [`Platform`], and the window calls nothing else.
//! Linux is [`crate::linux`], Windows [`crate::windows`], macOS
//! [`crate::macos`]. A system without a module of its own gets
//! [`Elsewhere`], which writes nothing.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use faraday_starter::api::{Computer, Concern, Installed, Reason, StarterEvent, WriteOrder};

use crate::payload::Payload;
use crate::writer::Job;

/// What the Starter asks of the operating system.
pub trait Platform: Send + Sync {
    /// This computer's facts (§7.1).
    fn computer(&self) -> Computer;

    /// Another encryption product's driver on this computer (§6.4), by
    /// name; `None` where there is none.
    fn other_encryption(&self) -> Option<String> {
        None
    }

    /// Starts following the disks on a thread of its own: sends
    /// [`StarterEvent::Disks`] now and whenever they change, or
    /// [`StarterEvent::NoWriter`] once when this computer cannot write a
    /// disk from here. A disk named on `forget` (one just written) has
    /// what is on it read again.
    fn watch(&self, out: Sender<StarterEvent>, forget: Receiver<String>);

    /// Writes as confirmed, on a thread of its own: progress and the
    /// result go to `out` ([`StarterEvent::WriteProgress`],
    /// [`StarterEvent::Written`]).
    fn write(&self, order: WriteOrder, job: Job, out: Sender<StarterEvent>);

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
    /// few seconds off the window's thread. What a system does not read
    /// yet it knows nothing of, which holds nothing and says nothing: on
    /// macOS a virtual machine is read, and the screens and USB devices
    /// wait for batch A3c.
    fn concerns(&self) -> Vec<Concern> {
        Vec::new()
    }

    /// The Starter is exiting: on Windows an uninstall run from the
    /// installed copy leaves its last step for now (§5.4).
    fn exiting(&self) {}

    /// What is installed (§5.2), found on disk; `None` when nothing is.
    /// May run the installed copy, so it is called off the window's
    /// thread.
    fn installed(&self) -> Option<Installed>;

    /// Installs the running Starter, and with `desktop` the payload's
    /// desktop app, per user (§5.2). Where the Starter went.
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

/// The operating system this Starter runs on.
pub fn this_computer() -> Arc<dyn Platform> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(crate::linux::Linux)
    }
    #[cfg(windows)]
    {
        Arc::new(crate::windows::Windows)
    }
    #[cfg(target_os = "macos")]
    {
        Arc::new(crate::macos::MacOs)
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
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

/// A system the Starter has no module for yet: no disks, no writer.
pub struct Elsewhere;

impl Platform for Elsewhere {
    fn computer(&self) -> Computer {
        Computer::default()
    }

    fn watch(&self, out: Sender<StarterEvent>, _forget: Receiver<String>) {
        let _ = out.send(StarterEvent::NoWriter);
    }

    fn write(&self, _order: WriteOrder, _job: Job, out: Sender<StarterEvent>) {
        let _ = out.send(StarterEvent::Written(Err(Reason::NoWriter)));
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
