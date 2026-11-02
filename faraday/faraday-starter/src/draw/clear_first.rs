//! What to write's Clear the whole stick first and Confirm's row for it
//! (`docs/STICK-IDENTITY.md` §4.2): Material 3's checkbox (m3.material.io,
//! Checkbox) with its label, and under the label the time it adds.

use faraday_draw::tokens::{ICON_GLYPH_BUTTON as BOX_SIDE, LIST_2};
use faraday_strings::fill;
use faraday_ui::Role;

use super::row::{ROW_ROOM, ROW_TEXT, press_row};
use super::{S, Ui, fact_row, two_lines};
use crate::clear::minutes;
use crate::{Starter, StarterAction};

/// What to write's tick, for a new stick. Returns its height.
pub(super) fn section(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(disk) = starter
        .chosen_disk()
        .filter(|_| starter.clear_first_offered())
    else {
        return 0.0;
    };
    let on = starter.making.as_ref().is_some_and(|m| m.clear_first);
    let y = y0 + 20.0;
    let rh = LIST_2;
    press_row(ui, x, y, w, rh, StarterAction::ClearFirst);
    ui.checkbox(x + 3.0, y + (rh - BOX_SIDE) / 2.0, on, true);
    let adds = fill(S.starter_clear_adds, &[&minutes(disk.size).to_string()]);
    let room = w - ROW_ROOM;
    two_lines(
        ui,
        (x + ROW_TEXT, y, rh),
        (room, room),
        (S.starter_clear_first, Role::OnSurface.paint()),
        (&adds, Role::OnSurfaceVariant.paint()),
    );
    ui.rule(x, y + rh, w, Role::OutlineVariant);
    y + rh - y0
}

/// Confirm's row, when the tick is on. Returns its height.
pub(super) fn confirm_row(starter: &Starter, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if !starter.clear_first_order() {
        return 0.0;
    }
    let size = starter
        .making
        .as_ref()
        .and_then(|m| m.confirmed.as_ref())
        .map_or(0, |c| c.3);
    let value = fill(S.starter_clear_row_value, &[&minutes(size).to_string()]);
    fact_row(ui, x, y, w, S.starter_clear_row, &value)
}
