//! The rule control (09 §3.2).

use crate::Pressed;
use crate::ui::Ui;

// Spec: 09 §3.2 names this model without a shape.
pub struct RuleModel;

/// Draws the model.
pub fn draw<A: Pressed>(_m: &RuleModel, _ui: &mut Ui<'_, A>) {
    todo!()
}
