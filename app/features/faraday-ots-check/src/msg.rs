use faraday_kernel::{Ending, FlowMsg, Message};
use faraday_organisms::Pressed;

use crate::Step;

/// What a flow's `update` takes: the runner's messages and the flow's own.
pub type Msg = FlowMsg<Step, Own>;

/// The flow's own messages (presses on a step's card, answers).
#[derive(Message)]
pub enum Own {}

/// What the flow tells its parent.
#[derive(Message)]
pub enum Out {
    /// The flow ended.
    Leave(Ending),
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
