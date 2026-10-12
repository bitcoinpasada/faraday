//! The seam between the hub app's window and the operating system
//! (`docs/HUB.md` §2): everything that differs from one system to
//! another is behind [`Platform`], and the window calls nothing else.
//! Linux is [`crate::linux`]; Windows comes in batch W, macOS from its
//! own spec. A system without a module of its own gets [`Elsewhere`],
//! which writes nothing.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use faraday_hub::api::{BitLocker, Computer, HubEvent, WriteOrder};

use crate::writer::Job;

/// What the hub app asks of the operating system.
pub trait Platform: Send + Sync {
    /// This computer's facts (§7.1).
    fn computer(&self) -> Computer;

    /// BitLocker on the system drive (§6); `None` where there is none.
    fn bitlocker(&self) -> Option<BitLocker> {
        None
    }

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
    fn restart_into_firmware(&self) -> Result<(), String>;

    /// The folder Transfer sends from and saves into.
    fn downloads(&self) -> PathBuf;
}

/// The operating system this hub app runs on.
pub fn this_computer() -> Arc<dyn Platform> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(crate::linux::Linux)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Arc::new(Elsewhere)
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
        let _ = out.send(HubEvent::Written(Err(String::from(
            "this computer cannot write a disk from here",
        ))));
    }

    fn restart_into_firmware(&self) -> Result<(), String> {
        Err(String::from("not on this computer"))
    }

    fn downloads(&self) -> PathBuf {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Downloads"))
            .unwrap_or_else(|| PathBuf::from("Downloads"))
    }
}
