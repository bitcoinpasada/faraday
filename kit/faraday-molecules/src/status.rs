//! The status molecule (09 §3.2).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

// Spec: 09 §3.2 names this model without a shape.
pub struct StatusModel<'s>(PhantomData<(&'s (),)>);

// Spec: 09 §3.2 names this model without a shape.
pub struct BusyModel<'s>(PhantomData<(&'s (),)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &StatusModel<'_>, _ui: &mut Ui<'_, A>) {
    todo!()
}
