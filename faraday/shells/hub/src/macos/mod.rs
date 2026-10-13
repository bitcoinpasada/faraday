//! The hub app on macOS (`docs/HUB.md` §2, §4, §5.2, §7;
//! `docs/MAC.md` §6): disks from `diskutil`, written through the
//! descriptor `/usr/libexec/authopen` hands back after the system's own
//! password prompt, so no part of the hub app runs as root; this
//! computer's facts from `sysctl` and `uname`; install per user into
//! `~/Applications`.
//!
//! Every program is run by its absolute path, never found on `PATH`:
//! `/usr/sbin/diskutil`, `/usr/sbin/sysctl`, `/usr/bin/uname`,
//! `/usr/bin/ditto`, `/usr/libexec/authopen`.
//!
//! What macOS answers is read by modules with no macOS call in them, so
//! they build and are tested on every system, against answers written
//! from documentation (`tests/macos/`; batch A3c replaces each with one
//! recorded on a Mac):
//!
//! - [`plist`]: the XML property lists `diskutil -plist` prints.
//! - [`diskutil`]: its answers as the disks offered and refused.
//! - [`facts`]: this computer's facts (§7.1).
//! - [`install`]: install and uninstall into a home folder.
//!
//! On macOS alone, `mac` runs the programs and puts them behind
//! [`crate::platform::Platform`]; its one `unsafe` block receives
//! authopen's descriptor.

pub mod diskutil;
pub mod facts;
pub mod install;
pub mod plist;

#[cfg(target_os = "macos")]
mod mac;

#[cfg(target_os = "macos")]
pub use mac::MacOs;
