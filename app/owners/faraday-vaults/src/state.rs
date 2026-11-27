use faraday_kernel::vaults;
use faraday_kernel::{AtLock, Edits};
use faraday_keys::vault::{OpenVaults, VaultId};

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    list: faraday_vault_list::State,
    #[lock(kept)]
    unlock: faraday_vault_unlock::State,
    #[lock(kept)]
    contents: faraday_vault_contents::State,
    #[lock(kept)]
    seal: faraday_vault_seal::State,
    #[lock(kept)]
    totp: faraday_totp::State,
    #[lock(kept)]
    gpg: faraday_gpg::State,
}

impl vaults::Read for State {
    fn open(&self) -> &OpenVaults {
        todo!()
    }

    fn current(&self) -> Option<VaultId> {
        todo!()
    }

    fn edits(&self) -> Edits {
        todo!()
    }
}
