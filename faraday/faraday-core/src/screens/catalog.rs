//! `Screen::Catalog`: Tools, every flow on one page, with its standards.

use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Ui, W};
use faraday_ui::parts::{section_label, title};

/// Tools: every flow by group, three tiles to a row, each with its
/// standards; Find a tool narrows them as it is typed.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::catalog::{GROUPS, TILES, matches};
    // A small panel has the name in its bar, Find across the panel, and a
    // tile a row.
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, cw - 96.0)
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    let top = if compact { 14.0 } else { 36.0 } - app.list_offset;
    let mut y = top;
    if !compact {
        title(ui, x, y, EN.catalog_tools);
    }
    // Find a tool: typing on this page goes here.
    let fw = if compact {
        width
    } else {
        320.0f32.min(width * 0.4)
    };
    let fx = x + width - fw;
    ui.fill(fx, y - 2.0, fw, 40.0, 8.0, BG);
    ui.stroke(fx, y - 2.0, fw, 40.0, 8.0, ACCENT.with_alpha(110));
    let (shown, tone) = if app.tools.find().is_empty() {
        (EN.catalog_find_a_tool.to_string(), DIM)
    } else {
        (app.tools.find().to_string(), TEXT)
    };
    let shown = ui.fit(14.0, W::R, &shown, fw - 28.0);
    let tw = ui.text_mid(fx + 12.0, y - 2.0, 40.0, 14.0, W::R, tone, &shown);
    // Typing on this page always lands here, with no click to focus it
    // first, so the caret is drawn whether or not anything is typed yet:
    // the one sign that this box is already live.
    let caret_x = if app.tools.find().is_empty() {
        fx + 12.0
    } else {
        fx + 14.0 + tw
    };
    ui.caret(caret_x, y + 9.0, 18.0);
    y += if compact { 54.0 } else { 64.0 };
    let gap = 14.0;
    let per = if compact { 1 } else { 3 };
    let tw3 = (width - (per as f32 - 1.0) * gap) / per as f32;
    let th = 82.0;
    let mut any = false;
    for group in GROUPS {
        let tiles: Vec<(usize, &crate::catalog::Tile)> = TILES
            .iter()
            .enumerate()
            .filter(|(_, t)| t.group == group && app.tile_shown(t) && matches(t, app.tools.find()))
            .collect();
        if tiles.is_empty() {
            continue;
        }
        any = true;
        section_label(ui, x, y, group);
        y += 28.0;
        for (k, (i, tile)) in tiles.iter().enumerate() {
            let tx = x + (k % per) as f32 * (tw3 + gap);
            let ty = y + (k / per) as f32 * (th + 10.0);
            let need = app.tile_need(tile.go);
            let opens = app.tile_opens(tile.go);
            let action = Action::Catalog(*i as u8);
            let pressed = ui.is_pressed(action);
            ui.fill(tx, ty, tw3, th, 10.0, if pressed { INNER } else { SURFACE });
            ui.stroke(tx, ty, tw3, th, 10.0, BORDER);
            let name = ui.fit(14.0, W::S, tile.name, tw3 - 32.0);
            ui.text(
                tx + 16.0,
                ty + 12.0,
                14.0,
                W::S,
                if opens { TEXT } else { DIM },
                &name,
            );
            let line = ui.fit(12.0, W::R, need.unwrap_or(tile.line), tw3 - 32.0);
            ui.text(
                tx + 16.0,
                ty + 34.0,
                12.0,
                W::R,
                if need.is_some() { WARN } else { MUTED },
                &line,
            );
            // The standards, as small tags.
            let mut gx = tx + 16.0;
            for tag in tile.tags {
                let gw = ui.measure(11.0, W::M, tag) + 14.0;
                if gx + gw > tx + tw3 - 12.0 {
                    break;
                }
                ui.fill(gx, ty + 56.0, gw, 18.0, 9.0, INNER);
                ui.text_mid(gx + 7.0, ty + 56.0, 18.0, 11.0, W::M, MUTED, tag);
                gx += gw + 6.0;
            }
            if opens {
                ui.hit(tx, ty, tw3, th, action);
            }
        }
        y += tiles.len().div_ceil(per) as f32 * (th + 10.0) + 16.0;
    }
    if !any {
        ui.text(x, y, 14.0, W::R, MUTED, EN.catalog_no_tool_matches);
        y += 30.0;
    }
    ui.c.pop_clip();
    app.content_h.set(y - top + 40.0);
    ui.report_scroll(clip, app.content_h.get() - h);
}
