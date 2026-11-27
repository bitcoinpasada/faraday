//! The files composite of the device app: Files, the file menu, the boot import, sticks coming and going, delete and zeroing.
//! Section 09 §4.6: the composite's shape (03 §6, 06 §4.2).

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod model;
mod msg;
mod page;
mod present;
mod state;
mod update;

use faraday_kernel::{CarryName, JobKind, Public};
use faraday_port::values::Anchor;

pub use model::{Body, Model, Over};
pub use msg::{FilesPress, Msg, Out, Press};
pub use present::{body, over};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`.
pub struct Files;

/// Its screens (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Files,
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
