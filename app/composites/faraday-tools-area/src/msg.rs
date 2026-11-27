use faraday_kernel::{Message, ScanFor, Tool};
use faraday_organisms::Pressed;

use crate::Page;
use crate::SampleAsk;

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
    OtsCheck(faraday_ots_check::Press),
}
