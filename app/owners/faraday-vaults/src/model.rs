use core::marker::PhantomData;

/// What `present` works out for the screen (04 §2).
pub struct Model<'s>(PhantomData<&'s ()>);

/// The vault area's sheets over a page, as 06 §4.2's `Over::Vaults` holds them.
pub struct SheetModel<'s>(PhantomData<&'s ()>);
