use faraday_bip::keys::Network;
use faraday_kernel::session::{self, FileView, KeyView, WaitingItem, WalletView};
use faraday_kernel::{AtLock, Edits, KeyId, WalletId};
use faraday_keys::Keys;

use crate::random::Random;
use crate::store::Store;

/// What the session holds: the keys and which of them are held for their backup, and its
/// randomness. The wallets, what else waits, the Inbox and the Outbox are not built yet.
#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    pub(crate) store: Store,
    #[lock(session)]
    pub(crate) random: Random,
    /// A count, never a secret.
    #[lock(public)]
    edits: Edits,
}

impl State {
    /// An empty session on `network`.
    pub(crate) fn new(network: Network) -> State {
        State {
            store: Store::new(network),
            random: Random::default(),
            edits: Edits::NONE,
        }
    }

    /// The session held something different: a memo keyed by `edits` is worked out again.
    pub(crate) fn changed(&mut self) {
        self.edits = self.edits.after();
    }

    /// The keys' signing contexts blinded again from the session's seed, when the seed arrives
    /// and whenever the person's randomness changes it (`docs/KEYS.md` §2).
    pub(crate) fn reblind(&mut self, c: &faraday_crypto::work::Compute) {
        if let Some(blind) = self.random.draw(b"faraday key blinding", b"") {
            self.store.keys_mut().reblind(c, blind.expose());
        }
    }
}

impl session::Read for State {
    fn keys(&self) -> &Keys {
        self.store.keys()
    }

    fn key(&self, id: KeyId) -> Option<KeyView<'_>> {
        self.store.view(id)
    }

    fn wallets(&self) -> &[WalletView] {
        &[]
    }

    fn wallet(&self, _id: WalletId) -> Option<&WalletView> {
        None
    }

    /// The keys held for their backup; the PSBTs waiting to be signed are not built yet.
    fn waiting(&self) -> &[WaitingItem] {
        self.store.waiting()
    }

    fn inbox(&self) -> &[FileView] {
        &[]
    }

    fn outbox(&self) -> &[FileView] {
        &[]
    }

    fn seeded(&self) -> bool {
        self.random.seeded()
    }

    fn edits(&self) -> Edits {
        self.edits
    }
}
