//! A shared owner (feature): the person's choices, Set the time, the settings file (03 §7.2).
//! Section 09 §4.4 and §4.5: the feature's shape, `impl shared::Owner` where it is a top state,
//! its `Press`; the vaults and the settings re-export the kernel's `Page`.

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

use faraday_kernel::{CarryName, JobKind};
use faraday_port::values::Anchor;

pub use model::Model;
pub use msg::{Msg, Out, Press};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`, `Present` and `faraday_present::Page`.
pub struct Settings;

/// Its pages: the kernel's, so the online app names them (06 §4.2).
pub use faraday_kernel::settings::Page;

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
