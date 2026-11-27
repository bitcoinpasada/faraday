use faraday_kernel::{AnyChange, ChangeId, Message};
use faraday_organisms::Pressed;

#[derive(Message)]
pub enum Msg {
    Press(Press),
    #[withheld]
    Given(ChangeId, AnyChange),
}

impl From<Press> for Msg {
    fn from(_press: Press) -> Msg {
        todo!()
    }
}

pub use faraday_kernel::Top as Out;

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {}
