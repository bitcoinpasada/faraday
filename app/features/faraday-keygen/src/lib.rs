//! A step-flow feature of the app: New key.
//! Section 09 §4.4: the `StepFlow` type in place of the unit struct, and its `Step`.

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

mod model;
mod msg;
mod present;
mod state;
mod update;

use faraday_kernel::{CarryName, JobKind};
use faraday_port::values::Anchor;

pub use model::Model;
pub use msg::{Msg, Out, Own, Press};
pub use state::{KeyGen, State};
pub use update::{Answer, Refusal, Step};

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
/// Each page (06 §4.2).
pub const ANCHORS: &[Anchor] = &[Anchor {
    scope: "keygen.KeyGen",
    n: 0,
}];
