use faraday_kernel::{AtLock, Carried, Ctx, Feature};

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    wallets: faraday_wallets::State,
    #[lock(kept)]
    create: faraday_create::State,
    #[lock(kept)]
    restore: faraday_restore::State,
    #[lock(kept)]
    seeds: faraday_seeds::State,
    #[lock(kept)]
    public_files: faraday_public_files::State,
    #[lock(kept)]
    core_import: faraday_core_import::State,
    #[lock(kept)]
    addresses: faraday_addresses::State,
    #[lock(kept)]
    silent: faraday_silent::State,
    #[lock(kept)]
    backup: faraday_backup::State,
    #[lock(kept)]
    backup_flow: faraday_backup_flow::State,
    #[lock(kept)]
    backups_list: faraday_backups_list::State,
    #[lock(kept)]
    spend_backup: faraday_spend_backup::State,
}

impl State {
    /// Every child started from what the last build carried; what a child's start tells is
    /// nothing yet.
    pub(crate) fn start(carried: &mut Carried, cx: &Ctx<'_>) -> State {
        State {
            wallets: <faraday_wallets::Wallets as Feature>::start(carried, cx).0,
            create: <faraday_create::MakeWallet as Feature>::start(carried, cx).0,
            restore: <faraday_restore::Restore as Feature>::start(carried, cx).0,
            seeds: <faraday_seeds::Seeds as Feature>::start(carried, cx).0,
            public_files: <faraday_public_files::PublicFiles as Feature>::start(carried, cx).0,
            core_import: <faraday_core_import::CoreImport as Feature>::start(carried, cx).0,
            addresses: <faraday_addresses::Addresses as Feature>::start(carried, cx).0,
            silent: <faraday_silent::Silent as Feature>::start(carried, cx).0,
            backup: <faraday_backup::BackUp as Feature>::start(carried, cx).0,
            backup_flow: <faraday_backup_flow::BackupFlow as Feature>::start(carried, cx).0,
            backups_list: <faraday_backups_list::BackupsList as Feature>::start(carried, cx).0,
            spend_backup: <faraday_spend_backup::SpendBackup as Feature>::start(carried, cx).0,
        }
    }
}
