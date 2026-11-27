//! The shadow control (09 §3.2).

use crate::Pressed;
use crate::ui::Ui;

// Spec: 09 §3.2 names this model without a shape.
pub struct ShadowModel;

/// Draws the model.
pub fn draw<A: Pressed>(_m: &ShadowModel, _ui: &mut Ui<'_, A>) {
    todo!()
}
