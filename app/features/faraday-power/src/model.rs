use core::marker::PhantomData;

/// What `present` works out for the screen (04 §2).
pub struct Model<'s>(PhantomData<&'s ()>);

/// The model of the failed self-test page.
pub struct FailedModel<'s>(PhantomData<&'s ()>);
