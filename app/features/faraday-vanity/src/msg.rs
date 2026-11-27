use faraday_kernel::{Message, Gate};
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
    /// A press that would show a secret.
    Reveal(Gate),
}

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {}
