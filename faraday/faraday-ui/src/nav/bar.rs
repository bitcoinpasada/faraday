//! The navigation bar (`docs/LAYOUT.md` §3, DESIGN §4.1): `NAV_BAR_H`
//! across a compact window's foot, its destinations evenly spaced, each
//! drawn as the rail draws it.

use super::{Dest, item};
use crate::Ui;
use crate::pal::*;
use faraday_draw::tokens::NAV_BAR_H;

/// The bar across the window at `y`, `w` wide.
pub fn bar<A: Copy + PartialEq>(ui: &mut Ui<A>, y: f32, w: f32, dests: &[Dest<A>]) {
    ui.fill(0.0, y, w, NAV_BAR_H, 0.0, Role::SurfaceContainer);
    ui.fill(0.0, y, w, 1.0, 0.0, Role::OutlineVariant);
    let iw = w / dests.len().max(1) as f32;
    for (i, d) in dests.iter().enumerate() {
        item(ui, i as f32 * iw, y, iw, NAV_BAR_H, d);
    }
}
