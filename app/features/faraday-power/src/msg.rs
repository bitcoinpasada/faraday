use faraday_kernel::{FrameSheet, Message, Top};
use faraday_present::Pressed;

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
    /// A frame sheet.
    Front(Option<FrameSheet>),
    /// A lock, an end, a broadcast.
    Top(Top),
}

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {}
