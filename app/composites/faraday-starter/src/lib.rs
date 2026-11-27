//! The Starter's composite: home, install, appearance, templates, its guide, make-stick and start.
//! Section 09 §4.7: the composite's shape (06 §3.2, §4.2, §5.3).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]
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

use faraday_kernel::{CarryName, JobKind, Public};
use faraday_port::values::Anchor;

pub use model::{Body, Model, Over};
pub use msg::{Msg, Out, Press};
pub use present::{body, over};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`.
pub struct Starter;

/// Its screens (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Make(faraday_make_stick::Page),
    Start,
    Tools,
    Templates(faraday_print_templates::Page),
    Guide,
    Install(faraday_install::Page),
    Appearance,
    About,
    Transfer,
    /// The lock scope's own page, shown by its own `Nav`.
    Demo,
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
