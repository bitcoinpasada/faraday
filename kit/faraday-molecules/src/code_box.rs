//! The code box molecule (09 §3.2).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

/// The model. Its fields are those of `main`'s component, ported (09 §3.4).
pub struct CodeBoxModel<'s>(PhantomData<(&'s (),)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &CodeBoxModel<'_>, _ui: &mut Ui<'_, A>) {
    todo!()
}
