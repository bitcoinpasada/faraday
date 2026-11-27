use core::marker::PhantomData;

/// What `present` works out for the screen (04 §2).
// Spec: 09 §4.4 names `Model` without a shape.
pub struct Model<'s>(PhantomData<&'s ()>);

/// The model of the failed self-test page.
// Spec: 06 §4.2 names `power::FailedModel` without a shape.
pub struct FailedModel<'s>(PhantomData<&'s ()>);
