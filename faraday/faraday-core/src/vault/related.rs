//! A second passphrase for a key, or a share of a set the vault already
//! holds, is saved only after **Continue** (never refused): the pending
//! records wait here while the dialog is up.

use faraday_keys::vault::Staged;
use faraday_keys::vault::VaultId;

/// Records waiting for **Continue**, all for one open vault.
#[derive(Debug)]
pub struct SaveHold {
    /// The open vault they go into.
    pub id: VaultId,
    /// The record and what its toast says, in the order saved.
    pub(super) pending: Vec<(Staged, String)>,
    /// The lines the dialog says, one per kind held already.
    pub why: Vec<String>,
    /// A move waiting on this answer: the vault the item is in, its place
    /// there, and its place in `pending`. It leaves that vault only if
    /// its record is saved and that vault is as it was: its version, the
    /// last of the four, is the one the place was taken at.
    pub(super) moved: Option<(VaultId, usize, usize, u64)>,
}
