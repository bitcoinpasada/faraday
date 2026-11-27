use faraday_kernel::{Ending, Message, ScanFor};
use faraday_organisms::Pressed;
use faraday_present::FlowPress;

use crate::Page;

#[derive(Message)]
pub enum Msg {
    Spend(faraday_spend::Msg),
    Decode(faraday_decode::Msg),
    Messages(faraday_messages::Msg),
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
}

/// What a press on a child's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Spend(FlowPress<faraday_spend::Step, faraday_spend::Press>),
    Decode(faraday_decode::Press),
    Messages(faraday_messages::Press),
}
