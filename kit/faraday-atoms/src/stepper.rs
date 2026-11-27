//! The stepper control (09 §3.2).

use core::marker::PhantomData;

use crate::Pressed;
use crate::ui::Ui;

// Spec: 09 §3.2 names this model without a shape.
pub struct StepperModel<'s, A>(PhantomData<(&'s (), A)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &StepperModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
