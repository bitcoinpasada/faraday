//! The vault composite's reads and changes (03 §7.2, §7.3).

use faraday_keys::vault::{OpenVaults, VaultId};

use super::change;
use crate::cx::Edits;

pub trait Read {
    fn open(&self) -> &OpenVaults;
    /// Counts every change to what the vaults hold; a memo's key.
    fn edits(&self) -> Edits;
    // Spec: 03 §7.2 sketches `/* marks, current */` beside `open` and `edits`.
}

/// The only change no feature gives: only the runtime gives it, at a lock (05 §7.2).
pub struct Close {
    pub seal: bool,
}

// Spec: 03 §7.3 names `vaults::Change` and `vaults::Done` and gives only `Close`.
pub enum Change {
    Close(Close),
}

pub enum Done {
    Close(()),
}

// Spec: 03 §7.2 names `Notice::Shared` for a vault; `vaults::Changed` is not shaped.
pub enum Changed {
    Opened(VaultId),
    Closed(VaultId),
}

change!(Close => ());
