//! The dice sheet's column (`docs/LAYOUT.md` §16.1, owner 2026-10-24):
//! on a window wide enough, Make codex32 shares' dice sheet stands in a
//! supporting pane at the right (`docs/LAYOUT.md` §6), below the eye,
//! `MARGIN_WIDE` from the window's edge and `SPACER` from the page, and
//! holds its place while the page scrolls beside it, scrolling within
//! itself where it is taller than the window. What only Demo offers on
//! the entry (Use a sample, the computer's walk) stands in Faraday
//! Desktop's host strip, outside the app (`docs/DESKTOP-HOST.md` §3.1).
//! Drawing reads the flow's state and works nothing out.

use faraday_draw::geom::WindowClass;
use faraday_draw::tokens as t;

use super::{Card, Codex32Make};
use crate::drawing::View;
use crate::ui::{Slot, Ui};

/// The room the dice sheet's column takes from the page at its right: a
/// supporting pane's, or none where it stands in the page.
pub(crate) fn side_room(column: bool) -> f32 {
    if column {
        crate::screens::support_pane_room()
    } else {
        0.0
    }
}

/// Whether the dice sheet stands in a column at the right for the entry
/// in a page area `cw` wide in a window `h` tall, on a window of `class`
/// with one-column pages or not (`compact`): only where the checksum
/// worksheet still draws whole beside it and the column fits the
/// window's height, over the input panel at the foot. Elsewhere both
/// sheets stand in the page. Arithmetic.
pub(crate) fn column_now(
    m: &Codex32Make,
    cw: f32,
    h: f32,
    (class, compact): (WindowClass, bool),
) -> bool {
    let entering = m.open() == Some(Card::Enter)
        && m.dice_sheet_shown()
        && m.dice_sheet_open()
        && m.front().is_some();
    let side = crate::screens::support_pane_room();
    if compact || !entering || cw - side - t::MARGIN_WIDE < t::CODEX32_MAIN_MIN {
        return false;
    }
    let Some((sheet_w, sheet_h)) = m.sheet_least() else {
        return false;
    };
    let panel = super::input_panel::place_in(m, cw, h, class, side);
    let start = panel.start_room();
    let lead = if start > 0.0 {
        start - t::KEYGEN_COLUMN_IN
    } else {
        0.0
    };
    let work = crate::keygen::screen::codex32_work_w(cw, side) - lead;
    let tall = h - 2.0 * t::MARGIN_WIDE - panel.foot_h();
    sheet_w <= work
        && sheet_h <= tall
        && t::CODEX32_DICE_COLUMN_H <= h - t::SUPPORT_PANE_TOP - t::MARGIN_WIDE - panel.foot_h()
}

/// Draws the dice sheet's column for the page area at `x0`, `cw` wide,
/// in a window `h` tall, where it stands (`column`), scrolled by its own
/// offset where it is taller (`Slot::DemoPane`).
pub(crate) fn draw(
    _app: &View,
    m: &Codex32Make,
    ui: &mut Ui,
    column: bool,
    (x0, cw, h): (f32, f32, f32),
) {
    if !column {
        return;
    }
    let (x, _, w, _) = crate::screens::support_pane_place((x0, cw, h));
    let y = t::SUPPORT_PANE_TOP;
    let view = (h - y - t::MARGIN_WIDE).max(t::TOUCH);
    let clip = ui.rect(x, y, w, view);
    let off = m.pane_offset();
    ui.c.push_clip(clip);
    let mark = ui.hits.len();
    let all = super::dice_screen::dice_sheet(m, ui, (x, y - off, w), false, false);
    // What is scrolled out of the pane takes no press.
    let mut k = mark;
    while k < ui.hits.len() {
        let r = ui.hits[k].0.intersect(&clip);
        if r.w <= 0 || r.h <= 0 {
            ui.hits.remove(k);
        } else {
            ui.hits[k].0 = r;
            k += 1;
        }
    }
    ui.c.pop_clip();
    ui.report_scroll_beside(Slot::DemoPane, clip, (all - view).max(0.0), off);
}
