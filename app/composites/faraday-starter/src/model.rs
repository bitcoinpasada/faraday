use core::marker::PhantomData;

/// What `present` works out for the composite's own page, the Tools page.
pub struct Model<'s>(PhantomData<&'s ()>);

/// The page under the sheets: one variant per page, the Demo's apart (06 §4.2).
pub enum Body<'s> {
    Home(faraday_starter_home::Model<'s>),
    Make(faraday_make_stick::Model<'s>),
    Start(faraday_starter_start::Model<'s>),
    Tools(Model<'s>),
    Templates(faraday_print_templates::Model<'s>),
    Guide(faraday_starter_guide::Model<'s>),
    Install(faraday_install::Model<'s>),
    Appearance(faraday_starter_appearance::Model<'s>),
    About(faraday_starter_appearance::Model<'s>),
    Transfer(faraday_transfer::Model<'s>),
}

/// The sheets over a page.
pub enum Over<'s> {
    Scan(faraday_scan::Model<'s>),
    Install(faraday_install::Model<'s>),
    Start(faraday_starter_start::Model<'s>),
    Guide(faraday_starter_guide::Model<'s>),
}
