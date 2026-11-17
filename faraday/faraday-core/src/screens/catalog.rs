//! `Screen::Catalog`: Tools, every flow on one page, with its standards.

use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Region;
use crate::ui::Shape;
use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens::{BODY_M, BODY_S, LABEL_S, TITLE_S};
use faraday_draw::tokens::{
    CATALOG_COLUMNS_MIN, CATALOG_ROW_GAP, CATALOG_TILE_H as TILE_H,
    CATALOG_TILE_H_PLAIN as TILE_H_PLAIN, CATALOG_TILE_MIN, PAGE_INSET,
};
use faraday_ui::parts::section_label;

pub(crate) use super::settings::rows::{ROWS, Row};

/// Tools: every flow by group, as many tiles to a row as fit and at
/// least three, each with its standards; Find a tool narrows them as it is typed.
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
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let top = if compact {
        14.0
    } else {
        super::bar_head(ui, None, EN.catalog_tools)
    };
    let mut y = top;
    // Find a tool: typing on this page goes here.
    let fw = if compact {
        width
    } else {
        320.0f32.min(width * 0.4)
    };
    let fx = x + width - fw;
    let here = Action::TypeHere(crate::edit::Here::Find);
    search_field(
        ui,
        (fx, y - 2.0, fw),
        (app.tools.find(), EN.catalog_find_a_tool),
        here,
    );
    y += if compact { 54.0 } else { 64.0 };
    let g = grid(compact, width);
    let tw3 = g.tile_w;
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
        y += ui.line_as(faraday_draw::tokens::TITLE_S) + faraday_draw::tokens::GRID;
        for (k, (i, tile)) in tiles.iter().enumerate() {
            let (tx, ty) = g.at(x, y, k);
            let need = app.tile_need(tile.go);
            let face = Face {
                name: tile.name,
                line: need.unwrap_or(tile.line),
                warn: need.is_some(),
                tags: tile.tags,
                opens: app.tile_opens(tile.go),
                action: Action::Catalog(*i as u8),
            };
            face.draw(ui, tx, ty, tw3);
        }
        y += g.height(tiles.len()) + 16.0;
    }
    // The rows of Settings the same words find, under Settings' name.
    let settings: Vec<u8> = crate::tools::search::hits(app.tools.find())
        .into_iter()
        .filter_map(|h| match h {
            crate::tools::search::Hit::Setting(i) => Some(i),
            crate::tools::search::Hit::Tile(_) => None,
        })
        .collect();
    if !settings.is_empty() {
        any = true;
        section_label(ui, x, y, EN.settings_settings);
        y += ui.line_as(faraday_draw::tokens::TITLE_S) + faraday_draw::tokens::GRID;
        for i in settings {
            y += setting_hit(ui, (x, y, tw3), i);
        }
    }
    if !any {
        ui.text_as(
            x,
            y,
            BODY_M,
            Role::OnSurfaceVariant,
            EN.catalog_no_tool_matches,
        );
        y += ui.advance_as(BODY_M, 30.0);
    }
    app.scroll.content_h().set(y - top + 40.0);
    ui.region_close(open, app.scroll.content_h().get() - h);
}

/// A setting found by [`crate::tools::search::hits`], drawn from (x, y),
/// `w` wide, as Material's list item: its label and the section it is in,
/// a press of which opens Settings at it. Returns its height. Tools' Find
/// and Home's search both draw it.
pub(crate) fn setting_hit(ui: &mut Ui, at: (f32, f32, f32), i: u8) -> f32 {
    let Some(row) = crate::tools::search::row(i) else {
        return 0.0;
    };
    let item = crate::panes::screen::Item {
        headline: row.label,
        supporting: row.section(),
        trailing: None,
        action: Action::SettingAt(i),
    };
    crate::panes::screen::list_item(ui, at, &item, false)
}

/// A search field at (x, y), `w` wide: Material's search field, in
/// Tools' look, with `hint` until something is typed. Typing on its page
/// always lands here, with no click to focus it first, so the caret is
/// drawn whether or not anything is typed yet: the one sign that this
/// box is already live. A press places it (`docs/TEXT-CURSOR.md` §3).
/// Tools' Find a tool and the guide's Find in the guide both draw it.
pub(super) fn search_field(
    ui: &mut Ui,
    (fx, fy, fw): (f32, f32, f32),
    (typed, hint): (&str, &str),
    here: Action,
) {
    ui.fill(fx, fy, fw, 40.0, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(fx, fy, fw, 40.0, 8.0, Role::Outline);
    let (tx, fh, room) = (
        fx + faraday_draw::tokens::TYPED_INSET,
        faraday_draw::tokens::TEXT_FIELD,
        fw - 28.0,
    );
    ui.press_box(fx, fy, fw, fh, here);
    if typed.is_empty() {
        let shown = ui.fit_as(BODY_M, hint, room);
        ui.text_mid_as((tx, fy, fh), BODY_M, Role::OnSurfaceVariant, &shown);
        ui.caret(
            tx,
            fy + (fh - faraday_draw::tokens::CARET_MIN) / 2.0,
            faraday_draw::tokens::CARET_MIN,
        );
    } else {
        ui.typed(
            tx,
            fy,
            fh,
            BODY_M.size,
            crate::ui::W::R,
            Role::OnSurface,
            typed,
            room,
            true,
            here,
        );
    }
}

/// The grid tiles stand in: as many columns as `CATALOG_TILE_MIN` leaves
/// room for, never fewer than three beside the navigation (`docs/LAYOUT.md`
/// §17), one on a small panel. Tools and Check both use it.
pub(crate) struct Grid {
    /// Tiles to a row.
    pub per: usize,
    /// A tile's width.
    pub tile_w: f32,
}

const TILE_GAP: f32 = 14.0;

/// The grid for a page `width` wide.
pub(crate) fn grid(compact: bool, width: f32) -> Grid {
    let per = if compact {
        1
    } else {
        (((width + TILE_GAP) / (CATALOG_TILE_MIN + TILE_GAP)) as usize).max(CATALOG_COLUMNS_MIN)
    };
    Grid {
        per,
        tile_w: (width - (per as f32 - 1.0) * TILE_GAP) / per as f32,
    }
}

impl Grid {
    /// The space between tiles.
    pub fn gap(&self) -> f32 {
        TILE_GAP
    }

    /// Where tile `k` of a group starting at (`x`, `y`) goes.
    pub fn at(&self, x: f32, y: f32, k: usize) -> (f32, f32) {
        (
            x + (k % self.per) as f32 * (self.tile_w + TILE_GAP),
            y + (k / self.per) as f32 * (TILE_H + CATALOG_ROW_GAP),
        )
    }

    /// The height of `n` tiles.
    pub fn height(&self, n: usize) -> f32 {
        n.div_ceil(self.per) as f32 * (TILE_H + CATALOG_ROW_GAP)
    }

    /// Where each of tiles `faces` of a group starting at (`x`, `y`) goes, and
    /// the group's height: a row is as tall as its tallest tile.
    pub fn place(&self, x: f32, y: f32, faces: &[Face]) -> (Vec<(f32, f32)>, f32) {
        let mut at = Vec::with_capacity(faces.len());
        let mut top = y;
        for row in faces.chunks(self.per) {
            let rh = row.iter().map(Face::height).fold(0.0, f32::max);
            for c in 0..row.len() {
                at.push((x + c as f32 * (self.tile_w + TILE_GAP), top));
            }
            top += rh + CATALOG_ROW_GAP;
        }
        (at, top - y)
    }
}

/// What a tile shows.
pub(crate) struct Face<'a> {
    /// Its name.
    pub name: &'a str,
    /// The line under it.
    pub line: &'a str,
    /// The line is a warning (what the tile needs first).
    pub warn: bool,
    /// Small tags at its foot.
    pub tags: &'a [&'a str],
    /// Whether a press acts.
    pub opens: bool,
    /// What a press does.
    pub action: Action,
}

impl Face<'_> {
    /// The tile's height: the tags' row only where it has tags.
    pub fn height(&self) -> f32 {
        if self.tags.is_empty() {
            TILE_H_PLAIN
        } else {
            TILE_H
        }
    }

    /// Draws the tile at (`tx`, `ty`), `tw3` wide.
    pub fn draw(&self, ui: &mut Ui, tx: f32, ty: f32, tw3: f32) {
        let th = self.height();
        let r = faraday_draw::tokens::SHAPE_M;
        ui.fill(tx, ty, tw3, th, r, Role::SurfaceContainerLow);
        ui.stroke(tx, ty, tw3, th, r, Role::OutlineVariant);
        if self.opens {
            ui.press(
                Shape::new(tx, ty, tw3, th, r),
                Role::OnSurface.paint(),
                self.action,
            );
        }
        let name = ui.fit_as(TITLE_S, self.name, tw3 - 32.0);
        ui.text_as(
            tx + 16.0,
            ty + 12.0,
            TITLE_S,
            if self.opens {
                Role::OnSurface.paint()
            } else {
                crate::ui::off()
            },
            &name,
        );
        let line = ui.fit_as(BODY_S, self.line, tw3 - 32.0);
        ui.text_as(
            tx + 16.0,
            ty + 34.0,
            BODY_S,
            if self.warn {
                Role::Warning
            } else {
                Role::OnSurfaceVariant
            },
            &line,
        );
        // The standards, as small tags.
        let mut gx = tx + 16.0;
        for tag in self.tags {
            let gw = ui.measure_as(LABEL_S.mono(), tag) + 14.0;
            if gx + gw > tx + tw3 - 12.0 {
                break;
            }
            ui.fill(
                gx,
                ty + 56.0,
                gw,
                18.0,
                faraday_draw::tokens::shape_full(18.0),
                Role::SurfaceContainerHighest,
            );
            ui.text_mid_as(
                (gx + 7.0, ty + 56.0, 18.0),
                LABEL_S.mono(),
                Role::OnSurfaceVariant,
                tag,
            );
            gx += gw + 6.0;
        }
    }
}
