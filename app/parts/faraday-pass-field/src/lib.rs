//! An application part: the passphrase field and its eye, the one holder of a typed passphrase
//! (A3).
//! Section 09 §4.3: `State` (fields private), `Msg`, `Press`, `update`, a `Read<'s>` view for its
//! owner's `present`, and `AtLock` for `State`.
//!
//! One passphrase typed once, or a pair (the passphrase and the one typed again). The text is held
//! in `SecretText` alone: never `Clone`, never printed, never copied into a `String`, wiped by a
//! clear, a lock and a drop. The owner drives it with `Msg`: a key, a press, a fill, a clear.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod eye;
mod kind;
mod pair;
mod read;
mod state;
#[cfg(test)]
mod tests;
mod update;

pub use kind::{Kind, Refused};
pub use pair::{differ, ready};
pub use read::Read;
pub use state::{Last, State, Which};
pub use update::{By, Msg, Press, update};
