use faraday_kernel::{Ending, FlowMsg, Gate, Message, ScanFor, WalletId};
use faraday_present::Pressed;

use crate::{Page, Step};

/// What a flow's `update` takes: the runner's messages and the flow's own.
pub type Msg = FlowMsg<Step, Own>;

/// The flow's own messages (presses on a step's card, answers).
#[derive(Message)]
pub enum Own {
    /// Begin Back up of a wallet (03 §6.1).
    Begin { wallet: WalletId, from: From },
}

/// Where a backup was begun from (03 §6.1).
// Spec: 03 §6.1 names `backup::From::Create` without the other variants.
#[derive(Message)]
pub enum From {
    Create,
}

/// What the flow tells its parent.
#[derive(Message)]
pub enum Out {
    /// Show one of its pages (06 §4.3).
    Open(Page),
    /// The flow ended.
    Leave(Ending),
    /// Ask for a scan.
    Scan(ScanFor),
    /// A press that would show a secret.
    Reveal(Gate),
}

impl core::convert::From<Ending> for Out {
    fn from(_ending: Ending) -> Out {
        todo!()
    }
}

/// What a press on one of the flow's own controls sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {}

impl core::convert::From<Press> for Own {
    fn from(_press: Press) -> Own {
        todo!()
    }
}
