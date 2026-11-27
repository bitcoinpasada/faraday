/// The page under the sheets: the online app's, or an owner's, which the product presents from
/// `Tops` (06 §4.2).
pub enum Body<'s> {
    Online(faraday_online::Body<'s>),
    Vaults(faraday_vaults::Model<'s>),
    Settings(faraday_settings::Model<'s>),
}

/// The sheets over it: the online app's and the vault area's.
pub enum Over<'s> {
    Online(faraday_online::Over<'s>),
    Vaults(faraday_vaults::SheetModel<'s>),
}
