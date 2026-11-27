/// The page under the sheets: one variant per page.
pub enum Body<'s> {
    KeyGen(faraday_keygen::Model<'s>),
    Randomness(faraday_randomness::Model<'s>),
    Entry(faraday_add_key::Model<'s>),
    KefTool(faraday_krux::Model<'s>),
    Bip85(faraday_bip85::Model<'s>),
    Convert(faraday_convert::Model<'s>),
    Vanity(faraday_vanity::Model<'s>),
}

/// The sheets over a page.
pub enum Over<'s> {
    CodexShares(faraday_codex_shares::Model<'s>),
    SecureBoot(faraday_secure_boot::Model<'s>),
}
