//! A feature of the app: Home and the bar search.
//! Section 09 §4.4 and §4.6, §4.7: the feature's shape.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod lead;
mod model;
mod msg;
mod page;
mod present;
mod state;
mod update;

use faraday_kernel::{CarryName, JobKind};
use faraday_port::values::Anchor;

pub use lead::{Going, small_panel};
pub use model::Model;
pub use msg::{Msg, Out, Place, Press};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`, `Present` and `faraday_present::Page`.
pub struct Home;

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
/// Where the page's scroll stands, keyed in the motion store beside the page's own anchor.
pub(crate) const SCROLL: Anchor = Anchor {
    scope: "home.Home",
    n: 1,
};

/// Each page and sheet (06 §4.2).
pub const ANCHORS: &[Anchor] = &[Anchor {
    scope: "home.Home",
    n: 0,
}];
