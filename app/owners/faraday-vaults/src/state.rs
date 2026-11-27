use faraday_kernel::vaults;
use faraday_kernel::{AtLock, Carried, Ctx, Edits, Feature};
use faraday_keys::vault::{OpenVaults, VaultId};

/// What the vault composite holds: each child's state and the open vaults.
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
    /// The vaults open now. None is opened yet, so a lock has nothing to wipe: it takes the
    /// `kept` mark once the store is an `AtLock`.
    #[lock(public)]
    open: OpenVaults,
}

impl State {
    pub(crate) fn start(carried: &mut Carried, cx: &Ctx<'_>) -> State {
        State {
            list: <faraday_vault_list::VaultList as Feature>::start(carried, cx).0,
            unlock: <faraday_vault_unlock::VaultUnlock as Feature>::start(carried, cx).0,
            contents: <faraday_vault_contents::VaultContents as Feature>::start(carried, cx).0,
            seal: <faraday_vault_seal::VaultSeal as Feature>::start(carried, cx).0,
            totp: <faraday_totp::Totp as Feature>::start(carried, cx).0,
            gpg: <faraday_gpg::Gpg as Feature>::start(carried, cx).0,
            open: OpenVaults::default(),
        }
    }
}

impl vaults::Read for State {
    fn open(&self) -> &OpenVaults {
        &self.open
    }

    fn current(&self) -> Option<VaultId> {
        None
    }

    fn edits(&self) -> Edits {
        Edits::NONE
    }
}
