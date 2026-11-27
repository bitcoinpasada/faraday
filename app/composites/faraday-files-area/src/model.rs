use core::marker::PhantomData;

/// What `present` works out for the composite's own page.
// Spec: 09 §4.6 gives the composite its own page and no shape for its model.
pub struct Model<'s>(PhantomData<&'s ()>);

/// The page under the sheets: one variant per page.
pub enum Body<'s> {
    Files(Model<'s>),
}

/// The sheets over a page.
pub enum Over<'s> {
    BootImport(faraday_boot_import::Model<'s>),
    StickVisit(faraday_stick_visit::Model<'s>),
    Zeroing(faraday_zeroing::Model<'s>),
}
