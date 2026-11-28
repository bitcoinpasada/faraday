//! A feature of the app: the device check and held-back inputs.
//! Section 09 §4.4 and §4.6, §4.7: the feature's shape.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod check;
mod inputs;
mod model;
mod msg;
mod page;
mod present;
mod rules;
mod state;
mod update;

#[cfg(test)]
mod tests;

use faraday_kernel::{CarryName, JobKind};
use faraday_port::values::Anchor;

pub use check::DeviceCheck;
pub use inputs::{
    BESIDE_LIMIT, CODE_LEN, CODE_TRIES, Counts, Hit, Inputs, KEPT_OUT_LIMIT, TAP_TRIES,
    TARGETS_MAX, WRONG_LETTERS_LIMIT, target_count,
};
pub use model::Model;
pub use msg::{Msg, Out, Press, Said};
pub use rules::{Concern, Kind, Reminder, screen_external, usb_kind, usb_name, virtual_machine};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`, `Present` and `faraday_present::Page`.
pub struct Devices;

pub const JOBS: &[JobKind] = &[];
/// The door's counts: devices kept out, presses beside a target, wrong letters, kept across a
/// lock (any mode).
pub const COUNTS: CarryName = CarryName::new("devices.input-counts", false, true);
pub const CARRY: &[CarryName] = &[COUNTS];

/// Where a sheet's scroll stands, keyed beside the sheet's own anchor.
pub(crate) fn scroll(n: u32) -> Anchor {
    Anchor {
        scope: "devices.Scroll",
        n,
    }
}

/// The Devices sheet's Hold to continue.
pub(crate) const HOLD: Anchor = Anchor {
    scope: "devices.Hold",
    n: 2,
};
/// Each page and sheet (06 §4.2).
pub const ANCHORS: &[Anchor] = &[
    Anchor {
        scope: "devices.Devices",
        n: 0,
    },
    Anchor {
        scope: "devices.NewInput",
        n: 1,
    },
];
