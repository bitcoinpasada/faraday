//! The dice sheet's column (`docs/LAYOUT.md` §16.1, owner 2026-10-24):
//! on a window wide enough, Make codex32 shares' dice sheet stands in a
//! supporting pane at the right (`docs/LAYOUT.md` §6), below the eye,
//! `MARGIN_WIDE` from the window's edge and `SPACER` from the page, and
//! holds its place while the page scrolls beside it, scrolling within
//! itself where it is taller than the window. What only Practice offers on
//! the entry (Use a sample) stands in Faraday
//! Desktop's host strip, outside the app (`docs/DESKTOP-HOST.md` §3.1).
//! Drawing reads the flow's state and works nothing out.

use crate::ui::Region;
use faraday_draw::tokens as t;

use super::{Card, Codex32Make};
use crate::drawing::View;
use crate::ui::{Slot, Ui};

/// The room the dice sheet's column takes from the page at its right: a
/// supporting pane's, or none where it stands in the page.
pub(crate) fn side_room(column: bool) -> f32 {
    if column {
        crate::screens::hub_layout::guide_room()
    } else {
        0.0
    }
}

/// Whether the dice sheet stands in a column at the right for the entry
/// in a page area `cw` wide, on a window with one-column pages or not
/// (`compact`): wherever the page keeps its least width beside it (the
/// worksheet beside it draws whole or panning in its view). The column
/// scrolls within itself where it is taller than the window. Elsewhere
/// both sheets stand in the page. Arithmetic.
pub(crate) fn column_now(m: &Codex32Make, cw: f32, compact: bool) -> bool {
    let entering = m.open() == Some(Card::Enter)
        && m.dice_sheet_shown()
        && m.dice_sheet_open()
        && m.front().is_some();
    let side = crate::screens::hub_layout::guide_room();
    !compact && entering && cw - side - t::MARGIN_WIDE >= t::CODEX32_MAIN_MIN
}

/// Draws the dice sheet's column at the right of the container's `room`,
/// a spacer from it, in a window `h` tall, where it stands (`column`),
/// scrolled by its own offset where it is taller (`Slot::DemoPane`).
pub(crate) fn draw(
    _app: &View,
    m: &Codex32Make,
    ui: &mut Ui,
    column: bool,
    (room, h): (&crate::flow::Room, f32),
) {
    if !column {
        return;
    }
    let (x, y, w, view) =
        crate::screens::hub_layout::guide_place_at(room.area_x + room.area_w + t::SPACER, h);
    let clip = ui.rect(x, y, w, view);
    // Kept within the reach the column had at its last drawing, as the
    // input panel's is: a smaller column or a taller window brings it back.
    let off = m.pane_offset().clamp(0.0, (m.pane_all() - view).max(0.0));
    let open = ui.region_open(Region::beside(Slot::DemoPane, clip, off));
    let mark = ui.presses_mark();
    let scale = super::dice_screen::column_scale(m.dice_choice().faces, w, view);
    let all = super::dice_screen::dice_sheet(m, ui, (x, y - off, w), false, false, scale);
    // What is scrolled out of the pane takes no press.
    ui.clip_presses_since(mark, clip);
    m.note_pane_all(all);
    ui.region_close(open, (all - view).max(0.0));
}
