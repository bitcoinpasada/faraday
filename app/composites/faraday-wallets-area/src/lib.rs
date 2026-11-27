//! The wallets composite of the device app: the Wallets tab, Make a new wallet, Restore, seeds, public files, a Core wallet, addresses, silent payments and the backups.
//! Section 09 §4.6: the composite's shape (03 §6, 06 §4.2).

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod model;
mod msg;
mod page;
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
pub struct Wallets;

/// Its screens (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    WalletsStart,
    Wallets,
    Create,
    Restore,
    CoreWallet,
    Explore,
    CheckAddress,
    Silent,
    Backup,
    BackupFlow,
    Backups,
    SpendBackup,
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
