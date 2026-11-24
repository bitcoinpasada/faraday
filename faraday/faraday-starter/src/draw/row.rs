//! A list row's parts that What to write's rows share: where its text
//! starts, the room it has, and its pressed layer.

use faraday_draw::tokens::{BODY_L, BODY_M, GAP, GAP_SMALL, ICON_GLYPH_SMALL, LIST_1, LIST_2, PAD};
use faraday_draw::widgets::Icon;
use faraday_ui::{Paint, Role, Shape};

use super::{SHAPE_S, Ui};
use crate::StarterAction;

/// A list item's text: `head` in Body L over `sub` in Body M, each
/// wrapped to `room` and never cut (Material 3 Lists, a two-line
/// headline), in a row at least `min_h` tall that grows with them.
/// Returns the row's height; draws only when `draw` is set, for the
/// callers that need the height before the row's layer.
pub(in crate::draw) fn list_text(
    ui: &mut Ui,
    (tx, y, room): (f32, f32, f32),
    min_h: f32,
    (head, head_ink): (&str, Paint),
    (sub, sub_ink): (&str, Paint),
    draw: bool,
) -> f32 {
    let hh = ui.wrap_height_as(BODY_L, head, room);
    let sh = if sub.is_empty() {
        0.0
    } else {
        ui.wrap_height_as(BODY_M, sub, room)
    };
    let h = min_h.max(hh + sh + 2.0 * (GAP + GAP_SMALL));
    if draw {
        let top = y + (h - hh - sh) / 2.0;
        ui.wrap_as((tx, top, room), BODY_L, head_ink, head);
        if sh > 0.0 {
            ui.wrap_as((tx, top + hh, room), BODY_M, sub_ink, sub);
        }
    }
    h
}

/// A plain list row (Material 3 Lists): `head` over an optional `sub`,
/// wrapped, an arrow at its end, no card around it, the whole row
/// pressing `action`. Returns its height.
pub(in crate::draw) fn plain_row(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (head, sub): (&str, &str),
    action: StarterAction,
) -> f32 {
    let min_h = if sub.is_empty() { LIST_1 } else { LIST_2 };
    let room = w - ICON_GLYPH_SMALL - PAD;
    let ink = (Role::OnSurface.paint(), Role::OnSurfaceVariant.paint());
    let h = list_text(ui, (x, y, room), min_h, (head, ink.0), (sub, ink.1), false);
    press_row(ui, x, y, w, h, action);
    list_text(ui, (x, y, room), min_h, (head, ink.0), (sub, ink.1), true);
    ui.icon(
        x + w - ICON_GLYPH_SMALL,
        y + (h - ICON_GLYPH_SMALL) / 2.0,
        ICON_GLYPH_SMALL,
        Icon::ChevronRight,
        ICON_GLYPH_SMALL / 2.0,
        Role::OnSurfaceVariant,
    );
    h
}

/// Where a list row's text starts, after its ring or box.
pub(in crate::draw) const ROW_TEXT: f32 = 36.0;
/// The room a list row's text has: the row less its control and a gap.
pub(in crate::draw) const ROW_ROOM: f32 = 40.0;

/// A list row's pressed layer and its target: the row widened by a
/// little either side, the layer kept off the rules above and below.
pub(in crate::draw) fn press_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    rh: f32,
    action: StarterAction,
) {
    ui.press_to(
        Shape::new(x - 6.0, y + 2.0, w + 12.0, rh - 4.0, SHAPE_S),
        (x - 6.0, y, w + 12.0, rh),
        Role::OnSurface.paint(),
        action.host(),
    );
}
