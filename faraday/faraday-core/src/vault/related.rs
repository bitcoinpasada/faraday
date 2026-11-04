//! A second passphrase for a key, or a share of a set the vault already
//! holds, is saved only after **Continue** (never refused): the pending
//! records wait here while the dialog is up.

use faraday_keys::vault::VaultId;
use faraday_vault::records::Record;

/// Records waiting for **Continue**, all for one open vault.
#[derive(Debug)]
pub struct SaveHold {
    /// The open vault they go into.
    pub id: VaultId,
    /// The record and what its toast says, in the order saved.
    pub(super) pending: Vec<(Record, String)>,
    /// The lines the dialog says, one per kind held already.
    pub why: Vec<String>,
}
