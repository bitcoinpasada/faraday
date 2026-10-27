//! `Screen::Catalog`: Tools, every flow on one page, with its standards.

use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens::{BODY_M, BODY_S, LABEL_S, TITLE_S};
use faraday_draw::tokens::{CATALOG_COLUMNS_MIN, CATALOG_TILE_MIN, PAGE_INSET};
use faraday_ui::parts::{section_label, title};

/// Tools: every flow by group, as many tiles to a row as fit and at
/// least three, each with its standards; Find a tool narrows them as it is typed.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::catalog::{GROUPS, TILES, matches};
    // A small panel has the name in its bar, Find across the panel, and a
    // tile a row.
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + PAGE_INSET, cw - 2.0 * PAGE_INSET)
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
    ui.fill(fx, y - 2.0, fw, 40.0, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(fx, y - 2.0, fw, 40.0, 8.0, Role::Primary.with_alpha(110));
    let (shown, tone) = if app.tools.find().is_empty() {
        (EN.catalog_find_a_tool.to_string(), Role::OnSurfaceVariant)
    } else {
        (app.tools.find().to_string(), Role::OnSurface)
    };
    let shown = ui.fit_as(BODY_M, &shown, fw - 28.0);
    let tw = ui.text_mid_as((fx + 12.0, y - 2.0, 40.0), BODY_M, tone, &shown);
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
    // As many columns as `CATALOG_TILE_MIN` leaves room for, never
    // fewer than three beside the navigation (`docs/LAYOUT.md` §17).
    let per = if compact {
        1
    } else {
        (((width + gap) / (CATALOG_TILE_MIN + gap)) as usize).max(CATALOG_COLUMNS_MIN)
    };
    let tw3 = (width - (per as f32 - 1.0) * gap) / per as f32;
    let th = 82.0;
    let mut any = false;
    for group in GROUPS {
        let tiles: Vec<(usize, &crate::catalog::Tile)> = TILES
            .iter()
            .enumerate()
            .filter(|(_, t)| t.group == group && matches(t, app.tools.find()))
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
            crate::ui::fill_pressed(
                ui,
                (tx, ty, tw3, th, 10.0),
                Role::SurfaceContainerLow,
                pressed,
            );
            ui.stroke(tx, ty, tw3, th, 10.0, Role::OutlineVariant);
            let name = ui.fit_as(TITLE_S, tile.name, tw3 - 32.0);
            ui.text_as(
                tx + 16.0,
                ty + 12.0,
                TITLE_S,
                if opens {
                    Role::OnSurface.paint()
                } else {
                    crate::ui::off()
                },
                &name,
            );
            let line = ui.fit_as(BODY_S, need.unwrap_or(tile.line), tw3 - 32.0);
            ui.text_as(
                tx + 16.0,
                ty + 34.0,
                BODY_S,
                if need.is_some() {
                    Role::Warning
                } else {
                    Role::OnSurfaceVariant
                },
                &line,
            );
            // The standards, as small tags.
            let mut gx = tx + 16.0;
            for tag in tile.tags {
                let gw = ui.measure_as(LABEL_S.mono(), tag) + 14.0;
                if gx + gw > tx + tw3 - 12.0 {
                    break;
                }
                ui.fill(gx, ty + 56.0, gw, 18.0, 9.0, Role::SurfaceContainerHighest);
                ui.text_mid_as(
                    (gx + 7.0, ty + 56.0, 18.0),
                    LABEL_S.mono(),
                    Role::OnSurfaceVariant,
                    tag,
                );
                gx += gw + 6.0;
            }
            if opens {
                ui.hit(tx, ty, tw3, th, action);
            }
        }
        y += tiles.len().div_ceil(per) as f32 * (th + 10.0) + 16.0;
    }
    if !any {
        ui.text_as(
            x,
            y,
            BODY_M,
            Role::OnSurfaceVariant,
            EN.catalog_no_tool_matches,
        );
        y += 30.0;
    }
    ui.c.pop_clip();
    app.scroll.content_h().set(y - top + 40.0);
    ui.report_scroll(clip, app.scroll.content_h().get() - h);
}
