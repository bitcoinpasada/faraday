//! A list row's parts that What to write's rows share: where its text
//! starts, the room it has, and its pressed layer.

use faraday_ui::{Role, Shape};

use super::{SHAPE_S, Ui};
use crate::StarterAction;

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
