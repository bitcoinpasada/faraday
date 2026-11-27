//! The code box molecule (09 §3.2).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

// Spec: 09 §3.2 names this model without a shape.
pub struct CodeBoxModel<'s>(PhantomData<(&'s (),)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &CodeBoxModel<'_>, _ui: &mut Ui<'_, A>) {
    todo!()
}
