//! The vault composite's reads and changes (03 §7.2, §7.3).

use faraday_keys::Refusal;
use faraday_keys::vault::{OpenVaults, VaultId};
use faraday_vault::Record;

use super::change;
use crate::cx::Edits;
use crate::message::Public;

/// The vault area's pages, the vault composite's and its children's (06 §4.2). The kernel's, so
/// that the online app's `Nav` names them without an edge to an owner (09 §4.5).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Vaults,
    CreateVault,
    Unlock,
    Contents,
    Codes,
}

pub trait Read {
    /// Each open vault, its records and their marks.
    fn open(&self) -> &OpenVaults;
    /// The vault a save goes to.
    fn current(&self) -> Option<VaultId>;
    /// Counts every change to what the vaults hold; a memo's key.
    fn edits(&self) -> Edits;
}

/// A record into an open vault, from a feature outside the vault area (`None`: the current one).
pub struct Put {
    pub vault: Option<VaultId>,
    pub record: Record,
}

/// The only change no feature gives: only the runtime gives it, at a lock (05 §7.2).
pub struct Close {
    pub seal: bool,
}

pub enum Change {
    Put(Put),
    Close(Close),
}

pub enum Done {
    Put(Result<(), Refusal>),
    Close(()),
}

pub enum Changed {
    Opened(VaultId),
    Closed(VaultId),
    /// A record was put in or taken out.
    Edited(VaultId),
}

change!(Put => Result<(), Refusal>);
change!(Close => ());
