//! A feature of the app: the Starter's Appearance and About.
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
pub struct StarterAppearance;

/// Its screens (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Appearance,
    About,
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
/// Each page and sheet (06 §4.2).
pub const ANCHORS: &[Anchor] = &[
    Anchor {
        scope: "starter_appearance.Appearance",
        n: 0,
    },
    Anchor {
        scope: "starter_appearance.About",
        n: 1,
    },
];
