/// The page under the sheets: one variant per child's page.
pub enum Body<'s> {
    Home(faraday_home::Model<'s>),
    Keys(faraday_keys_area::Body<'s>),
    Wallets(faraday_wallets_area::Body<'s>),
    Signing(faraday_signing_area::Body<'s>),
    Checks(faraday_checks_area::Body<'s>),
    Tools(faraday_tools_area::Body<'s>),
    Files(faraday_files_area::Body<'s>),
}

/// The sheets over a page: the scan feature's and each area's.
pub enum Over<'s> {
    Scan(faraday_scan::Model<'s>),
    Keys(faraday_keys_area::Over<'s>),
    Wallets(faraday_wallets_area::Over<'s>),
    Signing(faraday_signing_area::Over<'s>),
    Checks(faraday_checks_area::Over<'s>),
    Tools(faraday_tools_area::Over<'s>),
    Files(faraday_files_area::Over<'s>),
}
