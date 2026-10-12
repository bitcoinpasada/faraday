//! The hub app's shell (`docs/HUB.md` §2): `faraday_hub::Hub` in a
//! window, and what it reads and writes on this computer.
//!
//! - [`payload`]: the payload built in (`FARADAY_HUB_PAYLOAD`), and a
//!   developer's image given with `--image`.
//! - [`writer`]: the writer, the same on every system, over a
//!   [`writer::Device`] each system opens its own way; [`gpt`] checks a
//!   Faraday stick's partition table for keep data.
//! - [`platform`]: the seam. Everything that differs between operating
//!   systems is behind [`platform::Platform`]; [`linux`] is Linux's,
//!   [`windows`] Windows'.
//! - [`stick`]: which Faraday a stick holds, read from its boot
//!   partition alone.
//!
//! The window loop is `main.rs`, built on `faraday-window` as the
//! desktop app is.

pub mod gpt;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod payload;
pub mod platform;
pub mod stick;
pub mod windows;
pub mod writer;
