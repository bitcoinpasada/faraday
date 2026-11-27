use faraday_kernel::session::{self, FileView, KeyView, WaitingItem, WalletView};
use faraday_kernel::{AtLock, Edits, KeyId, WalletId};
use faraday_keys::Keys;

#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    skeleton: (),
}

impl session::Read for State {
    fn keys(&self) -> &Keys {
        todo!()
    }

    fn key(&self, _id: KeyId) -> Option<KeyView<'_>> {
        todo!()
    }

    fn wallets(&self) -> &[WalletView] {
        todo!()
    }

    fn wallet(&self, _id: WalletId) -> Option<&WalletView> {
        todo!()
    }

    fn waiting(&self) -> &[WaitingItem] {
        todo!()
    }

    fn inbox(&self) -> &[FileView] {
        todo!()
    }

    fn outbox(&self) -> &[FileView] {
        todo!()
    }

    fn seeded(&self) -> bool {
        todo!()
    }

    fn edits(&self) -> Edits {
        todo!()
    }
}
