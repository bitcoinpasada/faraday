//! The keyboard organism (09 §3.3).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

// Spec: 09 §3.2 names this model without a shape.
pub struct KeyboardModel<A>(PhantomData<(A,)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &KeyboardModel<A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
