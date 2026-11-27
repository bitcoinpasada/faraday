use faraday_kernel::Message;
use faraday_organisms::Pressed;

use crate::Page;

#[derive(Message)]
pub enum Msg {
    Device(faraday_device::Msg),
    Press(AppPress),
}

impl From<AppPress> for Msg {
    fn from(_press: AppPress) -> Msg {
        todo!()
    }
}

/// What a press on the app's frame or the device composite's page sends (06 §4.2's
/// `device_app::Press`).
// Spec: 06 §4.2 names `device_app::Press` and gives no variants; `Open` is a destination and
// `Back` the top bar's.
#[derive(Clone, PartialEq, Pressed)]
pub enum AppPress {
    Device(faraday_device::Press),
    Open(Page),
    Back,
}

/// One variant per top state's press (`Root::Press`, 06 §4.2).
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Settings(faraday_settings::Press),
    Session(faraday_session::Press),
    Vaults(faraday_vaults::Press),
    App(AppPress),
}
