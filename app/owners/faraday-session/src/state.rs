use faraday_bip::keys::Network;
use faraday_kernel::session::{self, FileView, KeyView, WaitingItem, WalletView};
use faraday_kernel::{AtLock, Edits, KeyId, WalletId};
use faraday_keys::Keys;

/// What the session holds: the keys, and nothing else yet.
#[derive(AtLock)]
pub struct State {
    /// The key store. Nothing puts a key in it yet, so a lock has nothing to wipe: it takes the
    /// `kept` mark once the store is an `AtLock`.
    #[lock(public)]
    keys: Keys,
}

impl State {
    /// An empty session on `network`.
    pub(crate) fn new(network: Network) -> State {
        State {
            keys: Keys::on(network),
        }
    }
}

impl session::Read for State {
    fn keys(&self) -> &Keys {
        &self.keys
    }

    fn key(&self, _id: KeyId) -> Option<KeyView<'_>> {
        None
    }

    fn wallets(&self) -> &[WalletView] {
        &[]
    }

    fn wallet(&self, _id: WalletId) -> Option<&WalletView> {
        None
    }

    fn waiting(&self) -> &[WaitingItem] {
        &[]
    }

    fn inbox(&self) -> &[FileView] {
        &[]
    }

    fn outbox(&self) -> &[FileView] {
        &[]
    }

    fn seeded(&self) -> bool {
        false
    }

    fn edits(&self) -> Edits {
        Edits::NONE
    }
}
