//! What the device app and the online app share (09 §4.6).

use faraday_kernel::{Frame, Public, Show, View};
use faraday_templates::screen::FrameModel;

use crate::{Page, Press, State};

pub use faraday_power::FailedModel;

/// The models of the frame sheets of the features this composite holds directly (06 §4.4).
pub enum FrameSheetModel<'s> {
    Power(faraday_power::Model<'s>),
    Devices(faraday_devices::Model<'s>),
    Guide(faraday_guide_sheet::Model<'s>),
}

pub fn frame_model<'s>(
    _state: &'s State,
    _frame: &'s Frame<Page>,
    _view: &View<'s>,
    _show: &Show<'s>,
) -> FrameModel<'s, Press> {
    todo!()
}

/// Every child that may hold a secret, for the leak crate's exhaustive match (07 TS18).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Child {
    Home,
    Power,
    Devices,
    Scan,
    GuideSheet,
    KeyGen,
    Randomness,
    CodexShares,
    AddKey,
    Krux,
    Bip85,
    Convert,
    Vanity,
    SecureBoot,
    Wallets,
    Create,
    Restore,
    Seeds,
    PublicFiles,
    CoreImport,
    Addresses,
    Silent,
    Backup,
    BackupFlow,
    BackupsList,
    SpendBackup,
    Spend,
    Decode,
    Messages,
    SeedChecks,
    Calculators,
    Lightning,
    OtsCheck,
    BootImport,
    StickVisit,
    Zeroing,
}
