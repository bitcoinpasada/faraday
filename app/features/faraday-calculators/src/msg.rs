use faraday_kernel::{Message, SampleAsk, ScanFor};
use faraday_organisms::Pressed;

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
    /// Ask for a scan.
    Scan(ScanFor),
    /// Load a published sample.
    Sample(SampleAsk),
}

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {}
