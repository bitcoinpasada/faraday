//! The keys composite of the device app: New key, randomness, codex32 shares, Add a key, Krux codes, BIP-85, Convert, Vanity and Secure Boot keys.
//! Section 09 §4.6: the composite's shape (03 §6, 06 §4.2).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

mod model;
mod msg;
mod present;
mod state;
mod update;

use faraday_kernel::{CarryName, JobKind, Public};
use faraday_port::values::Anchor;

pub use model::{Body, Over};
pub use msg::{Msg, Out, Press};
pub use present::{body, over};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`.
pub struct Keys;

/// Its screens (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    KeyGen,
    Randomness,
    Entry,
    KefTool,
    Bip85,
    Convert,
    Vanity,
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
