//! The lines of a pair, and the limits its panes scroll between
//! (`docs/FLOW-CONTAINER.md` §4).

use crate::ui::{Ui, pal::*};
use faraday_draw::tokens;

/// How far the pair's outline stands above the card's top: the heading
/// starts a padding under the top bar, so the line stands half a padding
/// under it.
const OUTLINE_ABOVE: f32 = tokens::PAD / 2.0;

/// A flow's one card (F331): from `from` to `to` (the corners, x and y),
/// filled a step off the page at level 3, which a High contrast theme
/// casts no shadow for, so it keeps the thin outline there and only
/// there. Drawn before what stands on it.
pub(crate) fn card(ui: &mut Ui, from: (f32, f32), to: (f32, f32)) {
    let (x, y) = from;
    let (w, h) = (to.0 - x, to.1 - y);
    if h <= 0.0 {
        return;
    }
    // Material's level 3, the floating button's number.
    ui.shadow(x, y, w, h, tokens::SHAPE_L, tokens::ELEVATION_FAB);
    ui.fill(x, y, w, h, tokens::SHAPE_L, Role::SurfaceContainerLow);
    if ui.theme.is_high_contrast() {
        outline(ui, from, to);
    }
}

/// The pair's outline: one thin rounded edge from `from` to `to` (the
/// corners, x and y), round Your choices and the step; no fill, the page
/// is the surface.
fn outline(ui: &mut Ui, from: (f32, f32), to: (f32, f32)) {
    ui.stroke(
        from.0,
        from.1,
        to.0 - from.0,
        to.1 - from.1,
        tokens::SHAPE_L,
        Role::OutlineVariant,
    );
}

/// The card's top and foot limits: where the pair stands, its outline's top
/// and the line a margin above the window's foot; else the page's.
pub(crate) fn limits(left: bool, compact: bool, top: f32, view_h: f32) -> (f32, f32) {
    if left {
        (top - OUTLINE_ABOVE, view_h - tokens::PAD)
    } else {
        (if compact { top } else { 0.0 }, view_h)
    }
}

/// The lines of one card: the divider at `at` from `top` to `end`, and,
/// where the pair stands (its left edge and the card's right, the foot
/// limit), the outline a padding outside it.
pub(crate) fn lines(ui: &mut Ui, (at, top, end): (f32, f32, f32), pair: Option<((f32, f32), f32)>) {
    divider(ui, at, top, end);
    if let Some(((left, right), limit)) = pair {
        outline(
            ui,
            (left - tokens::PAD, top - OUTLINE_ABOVE),
            (right + tokens::PAD, (end + tokens::PAD).min(limit)),
        );
    }
}

/// One card's divider: at `at`, the middle of the gap, inset by the card's
/// padding, from `top` to where the taller side ends.
fn divider(ui: &mut Ui, at: f32, top: f32, end: f32) {
    let inset = tokens::PAD;
    if end - top > 2.0 * inset {
        ui.fill(
            at,
            top + inset,
            1.0,
            end - top - 2.0 * inset,
            0.0,
            Role::OutlineVariant,
        );
    }
}
