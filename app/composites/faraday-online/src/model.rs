use faraday_present::ScreenModel;

use crate::{OwnerPage, Press};

/// The page under the sheets: one variant per page.
pub enum Body<'s> {
    Device(faraday_device::Body<'s>),
    Transfer(faraday_transfer::Model<'s>),
    // Spec: see `Page`: an owner's page is presented by the product, from its `Tops`.
    Vaults(OwnerPage),
    Settings(OwnerPage),
    SelfTestFailed(faraday_device::FailedModel<'s>),
}

/// The sheets over a page.
pub enum Over<'s> {
    Device(faraday_device::Over<'s>),
    Strip(faraday_strip::Model<'s>),
    Frame(faraday_device::frame::FrameSheetModel<'s>),
}

/// The Demo's whole screen model, for the Starter's `Body::Demo`.
pub type Screen<'s> = ScreenModel<'s, Body<'s>, Over<'s>, Press>;
