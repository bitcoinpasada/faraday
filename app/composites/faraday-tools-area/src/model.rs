use core::marker::PhantomData;
use faraday_kernel::Never;

/// What `present` works out for the composite's own page.
// Spec: 09 §4.6 gives the composite its own page and no shape for its model.
pub struct Model<'s>(PhantomData<&'s ()>);

/// The page under the sheets: one variant per page.
pub enum Body<'s> {
    Catalog(Model<'s>),
    Tools(faraday_calculators::Model<'s>),
    Lightning(faraday_lightning::Model<'s>),
    Invoice(faraday_lightning::Model<'s>),
    OtsCheck(faraday_ots_check::Model<'s>),
}

/// The sheets over a page.
pub enum Over<'s> {
    // Spec: 09 §4.6 gives this composite an `Over` and no sheet of its own to hold.
    #[doc(hidden)]
    None(Never, PhantomData<&'s ()>),
}
