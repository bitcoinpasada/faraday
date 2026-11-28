use faraday_kernel::{AnyChange, ChangeId, Message};
use faraday_port::Failure;
use faraday_port::bytes::Entropy;
use faraday_present::Pressed;

#[derive(Message)]
#[expect(
    clippy::large_enum_variant,
    reason = "the enum carries the domain's real types, which differ in size; a value is moved once"
)]
pub enum Msg {
    Press(Press),
    #[withheld]
    Given(ChangeId, AnyChange),
    /// The adapter's answer to the entropy the session asked for itself.
    #[withheld]
    Entropy(Result<Entropy, Failure>),
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        Msg::Press(press)
    }
}

pub use faraday_kernel::Top as Out;

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {}
