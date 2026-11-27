//! The device composite: every feature of the device app that is not a shared owner.
//! Section 09 §4.6: the composite's shape (03 §6, 06 §3.1, §4.2).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod frame;
mod model;
mod msg;
mod present;
mod state;
mod update;

use faraday_kernel::{CarryName, JobKind, Public};
use faraday_port::values::Anchor;

pub use frame::FailedModel;
pub use model::{Body, Over};
pub use msg::{Msg, Out, Press};
pub use present::{body, over};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`.
pub struct Device;

/// Its screens (06 §4.2): each a child's own page type.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Keys(faraday_keys_area::Page),
    Wallets(faraday_wallets_area::Page),
    Signing(faraday_signing_area::Page),
    Checks(faraday_checks_area::Page),
    Tools(faraday_tools_area::Page),
    Files(faraday_files_area::Page),
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
