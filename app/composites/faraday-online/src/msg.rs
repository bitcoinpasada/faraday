use faraday_kernel::Message;
use faraday_present::Pressed;

use crate::Page;

#[derive(Message)]
pub enum Msg {
    Device(faraday_device::Msg),
    Strip(faraday_strip::Msg),
    Transfer(faraday_transfer::Msg),
    Press(Press),
}

impl From<Press> for Msg {
    fn from(_press: Press) -> Msg {
        todo!()
    }
}

pub use faraday_kernel::Top as Out;

/// What a press on the online app's frame or page sends: the owners' presses are the product's
/// (06 §4.2). `Open` is a destination, `Back` the top bar's.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Device(faraday_device::Press),
    Strip(faraday_strip::Press),
    Transfer(faraday_transfer::Press),
    Open(Page),
    Back,
}
