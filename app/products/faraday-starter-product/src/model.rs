use faraday_present::ScreenModel;

use crate::Press;

/// The page under the sheets: the Starter's own, or the Demo's whole screen (06 §4.2).
pub enum Body<'s> {
    Starter(faraday_starter::Body<'s>),
    Demo(Box<DemoScreen<'s>>),
}

/// The sheets over it (06 §4.2).
pub enum Over<'s> {
    Starter(faraday_starter::Over<'s>),
    Frame(faraday_online::FrameSheetModel<'s>),
}

/// The Demo's whole screen under the Starter's bar, in the product's presses.
pub type DemoScreen<'s> = ScreenModel<'s, DemoBody<'s>, DemoOver<'s>, Press>;

/// The Demo's page: the online app's, or an owner's, which the product presents (06 §4.2).
pub enum DemoBody<'s> {
    Online(faraday_online::Body<'s>),
    Vaults(faraday_vaults::Model<'s>),
    Settings(faraday_settings::Model<'s>),
}

/// The Demo's sheets: the online app's and the vault area's.
pub enum DemoOver<'s> {
    Online(faraday_online::Over<'s>),
    Vaults(faraday_vaults::SheetModel<'s>),
}
