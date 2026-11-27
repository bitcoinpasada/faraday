//! A shared owner (composite): the open vaults and the vault features; impl vaults::Read.
//! Section 09 §4.4 and §4.5: the feature's shape, `impl shared::Owner` where it is a top state,
//! its `Press`; the vaults and the settings re-export the kernel's `Page`.

#![expect(dead_code, reason = "the fields are read once the bodies are written")]
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

pub use model::{Model, SheetModel};
pub use msg::{Msg, Out, Press};
pub use present::sheets;
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`, `Present` and `faraday_present::Page`.
pub struct Vaults;

/// Its pages: the kernel's, so the online app names them (06 §4.2).
pub use faraday_kernel::vaults::Page;

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
