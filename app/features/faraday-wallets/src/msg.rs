use faraday_kernel::{Gate, Message, ScanFor};
use faraday_organisms::Pressed;

use crate::Page;

#[derive(Message)]
pub enum Msg {
    Press(Press),
}

impl From<Press> for Msg {
    fn from(_press: Press) -> Msg {
        todo!()
    }
}

/// What the feature tells its parent.
#[derive(Message)]
pub enum Out {
    /// Show one of its pages (06 §4.3).
    Open(Page),
    /// Ask for a scan.
    Scan(ScanFor),
    /// A press that would show a secret.
    Reveal(Gate),
}

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {}
