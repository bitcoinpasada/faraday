//! The destination molecule (09 §3.2).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

/// The model. Its fields are those of `main`'s component, ported (09 §3.4).
pub struct DestinationModel<'s, A>(PhantomData<(&'s (), A)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &DestinationModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
