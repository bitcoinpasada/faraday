//! The presentation kernel: `Present`, `Page`, `ShowFlow`, the step-flow fill and the helpers
//! every `present` shares. Section 09 §4.2 and section 04 §2 to §5.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod flow;
pub mod help;
pub mod screen;
pub mod stages;

/// The kit's press bound and its derive, for a feature's `msg` module, which names no kit crate
/// (01 §5).
pub use faraday_organisms::Pressed;

pub use flow::{ChoiceLine, FlowPress, PressOf, ShowFlow, fill};
pub use help::{busy, line, offer, pass_model, refused};
pub use screen::{Drawn, ScreenModel, draw};
pub use stages::{Page, Present};
