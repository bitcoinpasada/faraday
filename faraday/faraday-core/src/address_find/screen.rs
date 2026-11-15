//! The address check's result card, drawn (`docs/DESIGN.md` §5, Check an
//! address, "The result"): the Result line in its tone with its icon, then
//! one fact a row with a rule between, each fact's label over its value.
//! Drawing reads the [`Answer`] and works nothing out.

use super::Answer;
use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens::{BODY_M, BODY_S, GAP, GAP_SMALL, ICON_GLYPH, PAD, PROGRESS_H, TITLE_M};
use faraday_draw::widgets::Icon;

/// The result's lines at `(x, y)`, `w` wide. Returns the height used.
pub(crate) fn result(ui: &mut Ui, (x, y, w): (f32, f32, f32), a: &Answer) -> f32 {
    let (role, icon) = if a.found {
        (Role::Ok, Icon::Success)
    } else {
        (Role::Warning, Icon::Warning)
    };
    let text_x = x + ICON_GLYPH + PAD;
    let text_w = w - ICON_GLYPH - PAD;
    let head_h = ui.wrap_height_as(TITLE_M, &a.head, text_w).max(ICON_GLYPH);
    ui.icon(
        x,
        y + (head_h - ICON_GLYPH) / 2.0,
        ICON_GLYPH,
        icon,
        PAD,
        role,
    );
    ui.wrap_as((text_x, y, text_w), TITLE_M, role, &a.head);
    let mut cy = y + head_h + GAP;
    ui.rule(x, cy, w, Role::OutlineVariant);
    cy += GAP;
    for (i, row) in a.rows.iter().enumerate() {
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, row.label) + GAP_SMALL;
        let size = if row.mono { BODY_M.mono() } else { BODY_M };
        for line in &row.lines {
            cy += ui.wrap_as((x, cy, w), size, Role::OnSurface, line);
        }
        if let (Some((done, depth)), true) = (
            a.looking,
            row.label == crate::strings::EN.address_check_searched,
        ) {
            cy += GAP_SMALL;
            ui.progress((x, cy, w), Some(done as f32 / depth.max(1) as f32), None);
            cy += PROGRESS_H;
        }
        cy += GAP_SMALL;
        // A rule between the facts, none under the last.
        if i + 1 < a.rows.len() {
            ui.rule(x, cy, w, Role::OutlineVariant);
            cy += GAP;
        }
    }
    cy - y
}
