//! A file in both Faraday and on the stick, drawn on Files' rows
//! (decision 306): the row's box in the check's green at a faint state
//! layer over the surface, with the check at its end, so it is not told by colour alone.

use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens::{
    GRID, SHAPE_S, VISIT_ROW_BOX_H, VISIT_ROW_BOX_X, VISIT_SAME_ICON, VISIT_SAME_ICON_BOX,
    VISIT_SAME_LAYER, VISIT_SAME_TOP, VISIT_SAME_W, alpha_of,
};
use faraday_draw::widgets::Icon;

/// The fill: the check's green as a state layer over the card's surface.
fn same_layer() -> crate::ui::Paint {
    Role::Ok.paint().with_alpha(alpha_of(VISIT_SAME_LAYER))
}

/// What a row's check takes at its right end, when it is the same file.
pub(super) fn width(same: bool) -> f32 {
    if same { VISIT_SAME_W } else { 0.0 }
}

/// A row's box in the faint green with its check at the box's end.
/// `(x, colw, ry)` are the card's left edge, its width and the row's top;
/// `dw` is what the row keeps at its right end outside the box (its
/// button, its menu).
pub(super) fn row(ui: &mut Ui, (x, colw, ry): (f32, f32, f32), dw: f32) {
    let w = colw - 2.0 * VISIT_ROW_BOX_X - dw;
    ui.fill(
        x + VISIT_ROW_BOX_X,
        ry,
        w,
        VISIT_ROW_BOX_H,
        SHAPE_S,
        same_layer(),
    );
    let right = x + VISIT_ROW_BOX_X + w;
    ui.icon(
        right - VISIT_SAME_ICON_BOX - VISIT_SAME_TOP,
        ry + VISIT_SAME_TOP,
        VISIT_SAME_ICON_BOX,
        Icon::Check,
        VISIT_SAME_ICON,
        Role::Ok,
    );
}

/// A receipt row on the faint green, with its check before its state `sw` wide:
/// `(x, w)` span the row's text, `ry` its top, `line` its text's height.
pub(super) fn receipt_row(ui: &mut Ui, (x, w, ry): (f32, f32, f32), sw: f32, line: f32) {
    let top = ry - GRID / 2.0;
    ui.fill(
        x - GRID,
        top,
        w + 2.0 * GRID,
        line + GRID,
        SHAPE_S,
        same_layer(),
    );
    ui.icon(
        x + w - sw - VISIT_SAME_W,
        top,
        line + GRID,
        Icon::Check,
        VISIT_SAME_ICON,
        Role::Ok,
    );
}
