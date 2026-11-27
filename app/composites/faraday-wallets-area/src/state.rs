use faraday_kernel::AtLock;

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
