//! The pads molecule (09 §3.2).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

// Spec: 09 §3.2 names this model without a shape.
pub struct PadModel<A>(PhantomData<(A,)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &PadModel<A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
