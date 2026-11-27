use core::marker::PhantomData;

/// What `present` works out for the screen (04 §2).
pub struct Model<'s>(PhantomData<&'s ()>);
