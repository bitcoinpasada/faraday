//! A shared owner (feature): the person's choices, Set the time, the settings file (03 §7.2).
//! Section 09 §4.4 and §4.5: the feature's shape, `impl shared::Owner` where it is a top state,
//! its `Press`; the vaults and the settings re-export the kernel's `Page`.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod clock;
mod file;
mod kept;
mod model;
mod msg;
mod page;
mod present;
mod rows;
mod state;
mod update;
mod version;

use faraday_kernel::{CarryName, JobKind};

pub use clock::Focus;
use faraday_port::values::Anchor;
pub use file::{FILE, HEADER, MAX, is_file};

pub use kept::{Kept, MODE, ModeAsked, SETTINGS};
pub use model::Model;
pub use msg::{Msg, Out, Press};
pub use state::State;
pub use update::Refusal;
pub use version::{
    BUILD, Program, VERSION, is_release, parse_version_output, version_label, version_line,
};

/// The feature: `Feature`, `Present` and `faraday_present::Page`.
pub struct Settings;

/// Its pages: the kernel's, so the online app names them (06 §4.2).
pub use faraday_kernel::settings::Page;

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[SETTINGS, MODE];
/// Where Settings' scroll stands, keyed in the motion store beside the page's own anchor.
pub(crate) const SCROLL: Anchor = Anchor {
    scope: "settings.Settings",
    n: 1,
};

/// Each page (06 §4.2): Settings and Set the time.
/// The scroll anchor of a page of this owner (the first of `ANCHORS` for Settings).
#[must_use]
pub fn anchor(page: Page) -> Anchor {
    match page {
        Page::Settings => Anchor {
            scope: "settings.Settings",
            n: 0,
        },
        Page::SetTime => Anchor {
            scope: "settings.SetTime",
            n: 0,
        },
    }
}

pub const ANCHORS: &[Anchor] = &[
    Anchor {
        scope: "settings.Settings",
        n: 0,
    },
    Anchor {
        scope: "settings.SetTime",
        n: 0,
    },
];
