/// The page under the sheets: one variant per page this composite presents. An owner's page is
/// the product's to present (06 §4.2), so it has no variant here.
pub enum Body<'s> {
    Device(Box<faraday_device::Body<'s>>),
    Transfer(faraday_transfer::Model<'s>),
    SelfTestFailed(faraday_device::FailedModel<'s>),
}

/// The sheets over a page.
pub enum Over<'s> {
    Device(faraday_device::Over<'s>),
    Strip(faraday_strip::Model<'s>),
    Frame(faraday_device::frame::FrameSheetModel<'s>),
}
