//! What the device app and the online app share (09 §4.6).

use faraday_kernel::{Frame, FrameSheet, Public, Show, View};
use faraday_port::values::Anchor;
use faraday_present::Present;
use faraday_templates::screen::FrameModel;

use crate::{Page, Press, State};

pub use faraday_power::FailedModel;

/// The models of the frame sheets of the features this composite holds directly (06 §4.4).
pub enum FrameSheetModel<'s> {
    Power(faraday_power::Model<'s>),
    Devices(faraday_devices::Model<'s>),
    Guide(Box<faraday_guide_sheet::Model<'s>>),
}

pub fn frame_model<'s>(
    _state: &'s State,
    _frame: &'s Frame<Page>,
    _view: &View<'s>,
    _show: &Show<'s>,
) -> FrameModel<'s, Press> {
    todo!()
}

/// The frame sheets of the stack, each with its anchor, the lowest first (06 §4.4). `stack` is
/// the frame's, the top sheet first. The power feature's four sheets are one model: its state
/// says which is on show.
pub fn sheets<'s>(
    state: &'s State,
    stack: &[FrameSheet],
    view: &View<'s>,
    show: &Show<'s>,
) -> Vec<(Anchor, FrameSheetModel<'s>)> {
    let mut out = Vec::new();
    let mut power_shown = false;
    for sheet in stack.iter().rev() {
        match sheet {
            FrameSheet::Lock | FrameSheet::LockAsk | FrameSheet::IdleWarn | FrameSheet::Power => {
                if let (false, Some(which)) = (power_shown, state.power.sheet()) {
                    power_shown = true;
                    out.push((
                        which.anchor(),
                        FrameSheetModel::Power(faraday_power::Power::present(
                            &state.power,
                            view,
                            show,
                        )),
                    ));
                }
            }
            FrameSheet::Guide => {
                let model = faraday_guide_sheet::GuideSheet::present(&state.guide, view, show);
                if model.is_open() {
                    out.push((
                        faraday_guide_sheet::ANCHORS[0],
                        FrameSheetModel::Guide(Box::new(model)),
                    ));
                }
            }
            // The device check's and the other kinds' sheets are not built.
            _ => todo!(),
        }
    }
    out
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
