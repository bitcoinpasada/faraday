use core::marker::PhantomData;
use faraday_kernel::Never;

/// What `present` works out for the composite's own page.
pub struct Model<'s>(PhantomData<&'s ()>);

/// The page under the sheets: one variant per page.
pub enum Body<'s> {
    Catalog(Model<'s>),
    Tools(faraday_calculators::Model<'s>),
    Lightning(faraday_lightning::Model<'s>),
    Invoice(faraday_lightning::Model<'s>),
    OtsCheck(Box<faraday_ots_check::Model<'s>>),
}

/// The sheets over a page.
pub enum Over<'s> {
    #[doc(hidden)]
    None(Never, PhantomData<&'s ()>),
}
