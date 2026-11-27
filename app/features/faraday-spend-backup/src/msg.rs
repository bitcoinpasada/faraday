use faraday_kernel::{Ending, FlowMsg, Gate, Message, WalletId};
use faraday_organisms::Pressed;

use crate::Step;

/// What a flow's `update` takes: the runner's messages and the flow's own.
pub type Msg = FlowMsg<Step, Own>;

/// The flow's own messages (presses on a step's card, answers).
#[derive(Message)]
pub enum Own {
    /// Begin Spend from a backup (03 §6.1).
    // Spec: 03 §6.1 names `from` without a type; the wallet moved from is the plainest.
    Begin { from: WalletId },
}

/// What the flow tells its parent.
#[derive(Message)]
pub enum Out {
    /// The flow ended.
    Leave(Ending),
    /// A press that would show a secret.
    Reveal(Gate),
    /// Load a published sample.
    Sample(SampleAsk),
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

/// A published sample to load.
// Spec: 09 §4.4 names `SampleAsk` without a shape; it belongs to the kernel, which no
// feature may widen, so each feature that asks holds its own.
#[derive(Message)]
pub enum SampleAsk {}
