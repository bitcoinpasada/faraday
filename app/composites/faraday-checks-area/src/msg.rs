use faraday_kernel::{Message, ScanFor, Tool};
use faraday_present::Pressed;

use crate::Page;

#[derive(Message)]
pub enum Msg {
    SeedChecks(faraday_seed_checks::Msg),
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
    Go(Tool),
}

/// What a press on a child's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Check(Tool),
    SeedChecks(faraday_seed_checks::Press),
}
