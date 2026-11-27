use core::marker::PhantomData;
use faraday_kernel::Never;

/// What `present` works out for the composite's own page.
// Spec: 09 §4.6 gives the composite its own page and no shape for its model.
pub struct Model<'s>(PhantomData<&'s ()>);

/// The page under the sheets: one variant per page.
pub enum Body<'s> {
    Check(Model<'s>),
    SeedCheck(faraday_seed_checks::Model<'s>),
    MultisigCheck(faraday_seed_checks::Model<'s>),
}

/// The sheets over a page.
pub enum Over<'s> {
    // Spec: 09 §4.6 gives this composite an `Over` and no sheet of its own to hold.
    #[doc(hidden)]
    None(Never, PhantomData<&'s ()>),
}
