use faraday_kernel::{Ending, FrameSheet, Message, Top};
use faraday_organisms::Pressed;
use faraday_present::FlowPress;

use crate::Page;

#[derive(Message)]
pub enum Msg {
    Home(faraday_starter_home::Msg),
    Install(faraday_install::Msg),
    Appearance(faraday_starter_appearance::Msg),
    Templates(faraday_print_templates::Msg),
    Guide(faraday_starter_guide::Msg),
    Make(faraday_make_stick::Msg),
    Start(faraday_starter_start::Msg),
    Transfer(faraday_transfer::Msg),
    Scan(faraday_scan::Msg),
    Press(Press),
}

impl From<Press> for Msg {
    fn from(_press: Press) -> Msg {
        todo!()
    }
}

/// What the composite tells its parent.
#[derive(Message)]
pub enum Out {
    /// Show one of its pages (06 §4.3).
    Open(Page),
    Leave(Ending),
    /// A child turned the camera on or off (06 §6.2).
    Camera(bool),
    /// A frame sheet (06 §4.4).
    Front(Option<FrameSheet>),
    /// A lock, an end, a broadcast.
    Top(Top),
}

/// What a press on a child's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    Home(faraday_starter_home::Press),
    Install(faraday_install::Press),
    Appearance(faraday_starter_appearance::Press),
    Templates(faraday_print_templates::Press),
    Guide(faraday_starter_guide::Press),
    Make(FlowPress<faraday_make_stick::Step, faraday_make_stick::Press>),
    Start(faraday_starter_start::Press),
    Transfer(faraday_transfer::Press),
    Scan(faraday_scan::Press),
}
