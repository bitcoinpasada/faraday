use faraday_kernel::{FrameSheet, Message};
use faraday_present::Pressed;

use crate::Page;

#[derive(Message)]
pub enum Msg {
    Device(faraday_device::Msg),
    /// The app's own presses: Back and Forward.
    Press(AppPress),
    /// A frame sheet comes up or the top one goes (06 §4.4), from a child's `Out::Front`.
    Front(Option<FrameSheet>),
    /// A page is shown (06 §4.3), from a destination or a child's `Out::Open`.
    Open(Page),
}

impl From<AppPress> for Msg {
    fn from(press: AppPress) -> Msg {
        match press {
            AppPress::Device(p) => Msg::Device(p.into()),
            AppPress::Open(page) => Msg::Open(page),
            AppPress::Power => Msg::Device(faraday_device::Msg::Power(faraday_power::Msg::Show(
                faraday_power::Which::Power,
            ))),
            AppPress::Back | AppPress::Forward => Msg::Press(press),
        }
    }
}

/// What a press on the app's frame or the device composite's page sends (06 §4.2's
/// `device_app::Press`).
#[derive(Clone, PartialEq, Pressed)]
pub enum AppPress {
    Device(faraday_device::Press),
    Open(Page),
    /// The top bar's Power control: the Power sheet.
    Power,
    Back,
    Forward,
}

/// One variant per top state's press (`Root::Press`, 06 §4.2).
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Settings(faraday_settings::Press),
    Session(faraday_session::Press),
    Vaults(faraday_vaults::Press),
    App(AppPress),
}
