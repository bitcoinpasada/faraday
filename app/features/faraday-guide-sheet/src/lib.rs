//! A feature of the app: the Guide button and its side pane.
//! Section 09 §4.4 and §4.6, §4.7: the feature's shape.

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

pub use model::{Model, PaneModel};
pub use msg::{Msg, Out, Place, Press, Room};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`, `Present` and `faraday_present::Page`.
pub struct GuideSheet;

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
/// Each page and sheet (06 §4.2).
pub const ANCHORS: &[Anchor] = &[
    Anchor {
        scope: "guide_sheet.Guide",
        n: 0,
    },
    SCROLL,
    TARGET,
];

/// Keys the text's scroll, which a section's jump and a page's start order.
pub(crate) const SCROLL: Anchor = Anchor {
    scope: "guide_sheet.Scroll",
    n: 1,
};

/// The section the guide is to scroll to, registered at its place while drawing.
pub(crate) const TARGET: Anchor = Anchor {
    scope: "guide_sheet.Target",
    n: 2,
};
