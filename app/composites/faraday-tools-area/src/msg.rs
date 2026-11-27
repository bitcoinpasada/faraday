use faraday_kernel::{Ending, Message, SampleAsk, ScanFor, Tool};
use faraday_present::{FlowPress, Pressed};

use crate::Page;

#[derive(Message)]
pub enum Msg {
    Calculators(faraday_calculators::Msg),
    Lightning(faraday_lightning::Msg),
    OtsCheck(faraday_ots_check::Msg),
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
    Sample(SampleAsk),
    Go(Tool),
}

/// What a press on a child's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Catalog(Tool),
    Calculators(faraday_calculators::Press),
    Lightning(faraday_lightning::Press),
    OtsCheck(FlowPress<faraday_ots_check::Step, faraday_ots_check::Press>),
}
