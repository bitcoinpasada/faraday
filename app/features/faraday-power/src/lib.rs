//! A feature of the app: Lock, Power, the idle lock and warning, held for backup.
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

pub use model::{FailedModel, Model};
pub use msg::{Msg, Out, Press};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`, `Present` and `faraday_present::Page`.
pub struct Power;

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
/// Each page and sheet (06 §4.2).
pub const ANCHORS: &[Anchor] = &[
    Anchor {
        scope: "power.Lock",
        n: 0,
    },
    Anchor {
        scope: "power.LockAsk",
        n: 1,
    },
    Anchor {
        scope: "power.Locked",
        n: 2,
    },
    Anchor {
        scope: "power.Power",
        n: 3,
    },
    Anchor {
        scope: "power.IdleWarn",
        n: 4,
    },
];
