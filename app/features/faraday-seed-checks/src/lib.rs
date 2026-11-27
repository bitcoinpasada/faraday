//! A feature of the app: the seed checks.
//! Section 09 §4.4 and §4.6, §4.7: the feature's shape.

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
pub struct SeedChecks;

/// Its screens (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    SeedCheck,
    MultisigCheck,
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
/// Each page and sheet (06 §4.2).
pub const ANCHORS: &[Anchor] = &[
    Anchor {
        scope: "seed_checks.SeedCheck",
        n: 0,
    },
    Anchor {
        scope: "seed_checks.MultisigCheck",
        n: 1,
    },
];
