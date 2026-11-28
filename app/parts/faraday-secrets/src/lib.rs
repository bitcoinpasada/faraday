//! An application part: the Secret out sheet, the text checks (TextCheck), file classification.
//! Section 09 §4.3: `State` (fields private), `Msg`, `Press`, `update`, a `Read<'s>` view for its
//! owner's `present`, and `AtLock` for `State`.
//!
//! Public files go to the Outbox as they are. A secret goes into the open vault and leaves only
//! inside the vault file; only once the person has held "save without encryption" does one go to
//! the Outbox as a plain file. `classify` says what a file holds and `exposure` who may read it;
//! `check` says whether a text holds a secret.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod files;
mod key_file;
mod kind;
mod scan;
mod share;
mod sheet;
mod state;
mod text_check;
#[cfg(test)]
mod tests;
mod update;
mod words;

pub use files::{TEXT_MAX, classify};
pub use kind::{Exposure, SecretKind, exposure, may_be_secret};
pub use sheet::{Ack, Keep, Plain, SecretFile, SecretOut, Then};
pub use state::{Answer, Finished, Read, State};
pub use text_check::{MAX_TEXT, TextCheck, check};
pub use update::{Msg, Press, update};
