/// The page under the sheets: the Starter's own, or the Demo's whole screen (06 §4.2).
pub enum Body<'s> {
    Starter(faraday_starter::Body<'s>),
    Demo(faraday_online::Screen<'s>),
}

/// The sheets over it (06 §4.2).
pub enum Over<'s> {
    Starter(faraday_starter::Over<'s>),
    Frame(faraday_online::FrameSheetModel<'s>),
}
