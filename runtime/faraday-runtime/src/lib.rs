//! The runtime: the model's owner. The queue, effect dispatch, the worker pool, the memo, the
//! lock and the frame (05; 09 §5).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod dispatch;
mod facts;
mod frame;
mod lock;
mod log;
mod memo;
mod pool;
mod queue;
pub mod root;
pub mod runtime;
pub mod seen;

pub use log::LogLine;
pub use root::{Addr, Nothing, OwnerState, Root, Stream, Tops};
pub use runtime::{Phase, Runtime, Workers};
pub use seen::{KeptImage, Mark, Seen};

#[cfg(test)]
mod tests;
