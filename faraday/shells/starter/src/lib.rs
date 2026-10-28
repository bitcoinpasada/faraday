//! The Starter's shell (`docs/STARTER.md` §2): `faraday_starter::Starter` in a
//! window, and what it reads and writes on this computer.
//!
//! - [`payload`]: the payload built in (`FARADAY_STARTER_PAYLOAD`), and a
//!   developer's image given with `--image`.
//! - [`writer`]: the writer, the same on every system, over a
//!   [`writer::Device`] each system opens its own way; [`gpt`] checks a
//!   Faraday stick's partition table for keep data.
//! - [`platform`]: the seam. Everything that differs between operating
//!   systems is behind [`platform::Platform`]; [`linux`] is Linux's,
//!   [`windows`] Windows', [`macos`] macOS's.
//! - [`signed`]: a boot file the person signed, checked to be the
//!   release's own program before the writer puts it in its place (F133).
//! - [`stick`]: which Faraday a stick holds, read from its boot
//!   partition alone.
//! - [`scheme`]: Light or Dark at start, from this computer's own
//!   setting (F126).
//! - [`settings`]: the installed Starter's settings file, its Text size
//!   (F85).
//!
//! Faraday Demo runs in the same window (F126, `docs/STARTER.md` §5.3):
//! `faraday_desktop::Demo` serves it, as in the Demo's own window.
//!
//! The window loop is `main.rs`, built on `faraday-window` as the
//! desktop app is.

pub mod gpt;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod macos;
pub mod payload;
pub mod pdf;
pub mod platform;
pub mod scheme;
pub mod settings;
pub mod signed;
pub mod stick;
pub mod windows;
pub mod writer;
