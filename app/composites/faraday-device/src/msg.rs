use faraday_kernel::{Ending, FrameSheet, Message, SampleAsk, Top};
use faraday_present::Pressed;

use crate::Page;

#[derive(Message)]
pub enum Msg {
    Home(faraday_home::Msg),
    Power(faraday_power::Msg),
    Devices(faraday_devices::Msg),
    Scan(faraday_scan::Msg),
    Guide(faraday_guide_sheet::Msg),
    Keys(faraday_keys_area::Msg),
    Wallets(faraday_wallets_area::Msg),
    Signing(faraday_signing_area::Msg),
    Checks(faraday_checks_area::Msg),
    Tools(faraday_tools_area::Msg),
    Files(faraday_files_area::Msg),
    Press(Press),
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        match press {
            Press::Home(p) => Msg::Home(p.into()),
            Press::Power(p) => Msg::Power(p.into()),
            Press::Devices(p) => match p {},
            Press::Scan(p) => match p {},
            Press::Guide(p) => match p {},
            Press::Keys(p) => Msg::Keys(p.into()),
            Press::Wallets(p) => Msg::Wallets(p.into()),
            Press::Signing(p) => Msg::Signing(p.into()),
            Press::Checks(p) => match p {},
            Press::Tools(p) => Msg::Tools(p.into()),
            Press::Files(p) => match p {},
        }
    }
}

/// What the composite tells its parent.
#[derive(Message)]
pub enum Out {
    /// Show one of its pages (06 §4.3).
    Open(Page),
    /// Show the vaults' page, which is the app's own (06 §4.3).
    Vaults,
    /// Show Settings, which is the app's own.
    Settings,
    Leave(Ending),
    /// A child turned the camera on or off (06 §6.2).
    Camera(bool),
    /// A frame sheet (06 §4.4).
    Front(Option<FrameSheet>),
    /// Load a published sample (03 §4.4).
    Sample(SampleAsk),
    /// A lock, an end, a broadcast.
    Top(Top),
    /// Home's search field changed: the words typed and whether the typing goes to it.
    Search {
        typed: faraday_text::Text,
        focused: bool,
    },
}

/// What a press on a child's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Home(faraday_home::Press),
    Power(faraday_power::Press),
    Devices(faraday_devices::Press),
    Scan(faraday_scan::Press),
    Guide(faraday_guide_sheet::Press),
    Keys(faraday_keys_area::Press),
    Wallets(faraday_wallets_area::Press),
    Signing(faraday_signing_area::Press),
    Checks(faraday_checks_area::Press),
    Tools(faraday_tools_area::Press),
    Files(faraday_files_area::Press),
}
