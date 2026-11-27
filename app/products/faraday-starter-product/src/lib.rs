//! The Starter's product crate: the settings owner and the Starter's composite outside the lock
//! scope; the session, the vaults and the online app inside it.
//! Section 09 §4.8: the product's shape (06 §2, §4, §5.3).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod l1;
mod model;
mod msg;
mod page;
mod present;
mod starter;
mod state;
mod update;

pub use model::{Body, DemoBody, DemoOver, DemoScreen, Over};
pub use msg::{Msg, Press};
pub use starter::{MODES, Starter, StarterOptions};
pub use state::StarterApp;
pub use update::Refusal;

/// The Starter's composite as a top state: `Feature<State = StarterApp, Out = Top>`.
pub struct Outside;
