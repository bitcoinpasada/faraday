//! The tooltip organism (09 §3.3).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

/// The model. Its fields are those of `main`'s component, ported (09 §3.4).
pub struct TipModel<'s>(PhantomData<(&'s (),)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &TipModel<'_>, _ui: &mut Ui<'_, A>) {
    todo!()
}
