//! The tools composite of the device app: the Tools catalog, calculators, Lightning, invoices and the OTS check.
//! Section 09 §4.6: the composite's shape (03 §6, 06 §4.2).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

mod model;
mod msg;
mod present;
mod state;
mod update;

use faraday_kernel::{CarryName, JobKind, Public};
use faraday_port::values::Anchor;

pub use model::{Body, Model, Over};
pub use msg::{Msg, Out, Press};
pub use present::{body, over};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`.
pub struct Tools;

/// Its screens (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Catalog,
    Tools,
    Lightning,
    Invoice,
    OtsCheck,
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
