//! Faraday Demo's product crate: the owners, the online app, and the runtime's `Root` for it.
//! Section 09 §4.8: the product's shape (06 §2, §4, §5.2).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

mod demo;
mod l1;
mod press;

pub use demo::{Demo, DemoOptions, MODES};
pub use press::Press;
