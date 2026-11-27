//! The frost control (09 §3.2).

use crate::Pressed;
use crate::ui::Ui;

/// The model. Its fields are those of `main`'s component, ported (09 §3.4).
pub struct FrostModel;

/// Draws the model.
pub fn draw<A: Pressed>(_m: &FrostModel, _ui: &mut Ui<'_, A>) {
    todo!()
}
