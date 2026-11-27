use faraday_device::frame::FrameSheetModel;

/// The page under the sheets (06 §4.2).
pub enum Body<'s> {
    Device(faraday_device::Body<'s>),
    Vaults(faraday_vaults::Model<'s>),
    Settings(faraday_settings::Model<'s>),
    SelfTestFailed(faraday_device::FailedModel<'s>),
}

/// The sheets over it (06 §4.2).
// Spec: 06 §4.2 gives `vaults::SheetModel`, which the vault owner does not export: its `Model`.
pub enum Over<'s> {
    Device(faraday_device::Over<'s>),
    Vaults(faraday_vaults::Model<'s>),
    Frame(FrameSheetModel<'s>),
}
