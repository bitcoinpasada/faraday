//! Faraday Demo's product crate: the owners, the online app, and the runtime's `Root` for it.
//! Section 09 §4.8: the product's shape (06 §2, §4, §5.2).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod demo;
mod l1;
mod model;
mod page;
mod practice;
mod press;

pub use demo::{Demo, DemoOptions, MODES};
pub use model::{Body, Over};
pub use practice::practice_samples;
pub use press::Press;
