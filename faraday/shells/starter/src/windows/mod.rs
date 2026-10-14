//! The Starter on Windows (`docs/STARTER.md` §2, §4.3, §5.2, §7): disks and
//! volumes through their IOCTLs, the writer as the same file started
//! elevated, install per user, this computer's facts from the registry,
//! and the restarts.
//!
//! What Windows answers is read by modules with no Win32 call in them,
//! so they build and are tested on every system, against recorded
//! answers (`tests/windows/`):
//!
//! - [`disks`]: storage descriptors, geometry, partition layouts and
//!   volume extents, as the disks offered and refused.
//! - [`registry`]: this computer's facts (§7.1).
//! - [`elevated`]: the elevated writer's `--write` arguments.
//! - [`pipe`]: what the elevated writer reports over its pipe.
//! - [`install`] and [`shortcut`]: install and uninstall as a plan of
//!   files and registry values.
//! - [`bitlocker`]: the BitLocker guard (§6): the elevated copy's
//!   `--bitlocker` arguments, its verbs against WMI as recorded, what it
//!   answers over its pipe, the marker and Save.
//!
//! On Windows alone, `sys` makes the calls (every `unsafe` block of the
//! module) and `win` puts them behind [`crate::platform::Platform`].

pub mod bitlocker;
pub mod devices;
pub mod disks;
pub mod elevated;
pub mod install;
pub mod pipe;
pub mod registry;
pub mod shortcut;

#[cfg(windows)]
mod sys;
#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::{Windows, early, snapshot};
