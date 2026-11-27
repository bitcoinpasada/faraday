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

use faraday_kernel::{CarryName, JobKind, Public, settings, vaults};
use faraday_port::values::Anchor;

pub use faraday_device::frame::FrameSheetModel;
pub use model::{Body, Over};
pub use msg::{Msg, Out, Press};
pub use present::{body, frame_model, over, shown};
pub use state::{OnlineApp, State};
pub use update::Refusal;

/// The feature: `Feature<State = OnlineApp, Out = Top>`.
pub struct Online;

/// Its screens (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Device(faraday_device::Page),
    Transfer,
    /// An owner's page: the kernel's type; the product presents it (06 §4.2).
    Vaults(vaults::Page),
    Settings(settings::Page),
    SelfTestFailed,
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
