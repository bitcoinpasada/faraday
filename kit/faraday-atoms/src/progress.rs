//! The progress control (09 §3.2).

use core::marker::PhantomData;

use crate::Pressed;
use crate::ui::Ui;

// Spec: 09 §3.2 names this model without a shape.
pub struct ProgressModel<'s>(PhantomData<(&'s (),)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &ProgressModel<'_>, _ui: &mut Ui<'_, A>) {
    todo!()
}
