/// The page under the sheets: one variant per page.
pub enum Body<'s> {
    WalletsStart(faraday_wallets::Model<'s>),
    Wallets(faraday_wallets::Model<'s>),
    Create(faraday_create::Model<'s>),
    Restore(faraday_restore::Model<'s>),
    CoreWallet(faraday_core_import::Model<'s>),
    Explore(faraday_addresses::Model<'s>),
    CheckAddress(faraday_addresses::Model<'s>),
    Silent(faraday_silent::Model<'s>),
    Backup(faraday_backup::Model<'s>),
    BackupFlow(faraday_backup_flow::Model<'s>),
    Backups(faraday_backups_list::Model<'s>),
    SpendBackup(faraday_spend_backup::Model<'s>),
}

/// The sheets over a page.
pub enum Over<'s> {
    Seeds(faraday_seeds::Model<'s>),
}
