use faraday_kernel::{Ending, Gate, Message, SampleAsk, ScanFor};
use faraday_organisms::Pressed;
use faraday_present::FlowPress;

use crate::Page;

#[derive(Message)]
pub enum Msg {
    Wallets(faraday_wallets::Msg),
    Create(faraday_create::Msg),
    Restore(faraday_restore::Msg),
    Seeds(faraday_seeds::Msg),
    PublicFiles(faraday_public_files::Msg),
    CoreImport(faraday_core_import::Msg),
    Addresses(faraday_addresses::Msg),
    Silent(faraday_silent::Msg),
    Backup(faraday_backup::Msg),
    BackupFlow(faraday_backup_flow::Msg),
    BackupsList(faraday_backups_list::Msg),
    SpendBackup(faraday_spend_backup::Msg),
    Press(Press),
}

impl From<Press> for Msg {
    fn from(_press: Press) -> Msg {
        todo!()
    }
}

/// What the composite tells its parent.
#[derive(Message)]
pub enum Out {
    Open(Page),
    Leave(Ending),
    Scan(ScanFor),
    Reveal(Gate),
    Sample(SampleAsk),
}

/// What a press on a child's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Wallets(faraday_wallets::Press),
    Create(FlowPress<faraday_create::Step, faraday_create::Press>),
    Restore(FlowPress<faraday_restore::Step, faraday_restore::Press>),
    Seeds(faraday_seeds::Press),
    PublicFiles(faraday_public_files::Press),
    CoreImport(FlowPress<faraday_core_import::Step, faraday_core_import::Press>),
    Addresses(faraday_addresses::Press),
    Silent(FlowPress<faraday_silent::Step, faraday_silent::Press>),
    Backup(FlowPress<faraday_backup::Step, faraday_backup::Press>),
    BackupFlow(FlowPress<faraday_backup_flow::Step, faraday_backup_flow::Press>),
    BackupsList(faraday_backups_list::Press),
    SpendBackup(FlowPress<faraday_spend_backup::Step, faraday_spend_backup::Press>),
}
