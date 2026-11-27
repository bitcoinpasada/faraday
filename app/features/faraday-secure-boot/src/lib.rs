//! A feature of the app: Secure Boot keys: its flow in KeyGen and Backup.
//! Section 09 §4.4 and §4.6, §4.7: the feature's shape.

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

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
pub struct SecureBoot;

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
