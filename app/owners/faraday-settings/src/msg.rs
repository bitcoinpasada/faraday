use faraday_kernel::{AnyChange, ChangeId, Message};
use faraday_port::storage::StickId;
use faraday_present::Pressed;

use crate::kept::ModeAsked;

#[derive(Message)]
#[expect(
    clippy::large_enum_variant,
    reason = "the enum carries the domain's real types, which differ in size; a value is moved once"
)]
pub enum Msg {
    Press(Press),
    /// The boot stick's settings file, read: its bytes, or `None` when it could not be.
    Read(StickId, Option<Vec<u8>>),
    #[withheld]
    Given(ChangeId, AnyChange),
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        Msg::Press(press)
    }
}

pub use faraday_kernel::Top as Out;

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    /// A mode picked on the sheet; the lock's own question follows (`main`'s `mode_act`).
    Mode(ModeAsked),
    /// The mode change is not made: nothing is asked any more.
    ModeCancel,
    /// Lock and use the mode asked for: a lock that keeps nothing but the mode.
    ModeLock,
}
