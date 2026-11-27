//! The online composite: the device composite, the Demo's strip and Transfer.
//! Section 09 §4.7: the composite's shape (06 §1, §4.1, §4.2).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod model;
mod msg;
mod present;
mod state;
mod update;

use faraday_kernel::{CarryName, JobKind, Public};
use faraday_port::values::Anchor;

pub use faraday_device::frame::FrameSheetModel;
pub use model::{Body, Over, Screen};
pub use msg::{Msg, Out, Press};
pub use present::frame;
pub use state::{OnlineApp, State};
pub use update::Refusal;

/// The feature: `Feature<State = OnlineApp, Out = Top>`.
pub struct Online;

/// Its screens (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Device(faraday_device::Page),
    Transfer,
    // Spec: 06 §4.2 gives `Vaults(vaults::Page)` and `Settings(settings::Page)`, and 09 §4.5 lets
    // no crate but an owner, a product or a tool depend on an owner: the page is its index.
    Vaults(OwnerPage),
    Settings(OwnerPage),
    SelfTestFailed,
}

/// An owner's page, by its index in the owner's own `Page`; the product maps it.
// Spec: see `Page`.
#[derive(Public, Copy, PartialEq, Eq)]
pub struct OwnerPage(pub u8);

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
