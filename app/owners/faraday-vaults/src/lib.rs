//! A shared owner (composite): the open vaults and the vault features; impl vaults::Read.
//! Section 09 §4.4 and §4.5: the feature's shape, `impl shared::Owner` where it is a top state,
//! its `Press` and its `Page`.

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

use faraday_kernel::{CarryName, JobKind, Public};
use faraday_port::values::Anchor;

pub use model::Model;
pub use msg::{Msg, Out, Press};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`, `Present` and `faraday_present::Page`.
pub struct Vaults;

/// Its screens (06 §4.2).
// Spec: 09 §4.5 says each owner exports its `Page` and names no variants.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Vaults,
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
