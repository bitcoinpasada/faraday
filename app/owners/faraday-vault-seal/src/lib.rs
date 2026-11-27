//! A shared owner (feature): the seal at a lock (Job::seal).
//! Section 09 §4.4 and §4.5: the feature's shape, `impl shared::Owner` where it is a top state,
//! its `Press` and its `Page`.

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

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
pub struct VaultSeal;

/// Its screens (06 §4.2).
// Spec: 09 §4.5 says each owner exports its `Page` and names no variants.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Seal,
}

pub const JOBS: &[JobKind] = &[JobKind("vault-seal.seal")];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
