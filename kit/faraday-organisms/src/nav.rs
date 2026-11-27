//! The navigation rail, the navigation bar and the top app bar (09 §3.3).

use faraday_atoms::{Pressed, Ui};

// Spec: 09 §3.3 names this model without a shape.
pub struct RailModel;

// Spec: 09 §3.3 names this model without a shape.
pub struct BarModel;

// Spec: 09 §3.3 names this model without a shape.
pub struct TopBarModel;

/// Draws the navigation rail.
pub fn draw<A: Pressed>(_m: &RailModel, _ui: &mut Ui<'_, A>) {
    todo!()
}

/// Draws the navigation bar.
pub fn draw_bar<A: Pressed>(_m: &BarModel, _ui: &mut Ui<'_, A>) {
    todo!()
}

/// Draws the top app bar.
pub fn draw_top<A: Pressed>(_m: &TopBarModel, _ui: &mut Ui<'_, A>) {
    todo!()
}
