//! The menu page container: the page span and tile grid that Tools, Check and Wallets start draw in (docs/LAYOUT.md §17).

use super::catalog::Face;
use crate::ui::Ui;
use faraday_draw::tokens::{
    CATALOG_COLUMNS_MIN, CATALOG_ROW_GAP, CATALOG_TILE_GAP as TILE_GAP, CATALOG_TILE_H as TILE_H,
    CATALOG_TILE_MIN, PAGE_INSET, TITLE_S,
};
use faraday_ui::parts::section_label;

/// Where a page of tiles starts and how wide it is in an area `cw` wide
/// from `x0`: Tools and Check share it, so their grids match.
pub(crate) fn page_span(compact: bool, x0: f32, cw: f32) -> (f32, f32) {
    if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (
            x0 + PAGE_INSET,
            (cw - 2.0 * PAGE_INSET).min(faraday_draw::tokens::TRANSFER_W),
        )
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
    /// The least height of a tile: the tallest of the faces it was
    /// matched to, so that every tile on a page is one height.
    pub tile_h: f32,
}

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
        tile_h: 0.0,
    }
}

impl Grid {
    /// The grid with every tile as tall as the tallest of `faces`.
    pub fn matching<'a>(mut self, ui: &Ui, faces: impl IntoIterator<Item = &'a Face<'a>>) -> Self {
        self.tile_h = faces
            .into_iter()
            .map(|f| f.height(ui, self.tile_w))
            .fold(0.0, f32::max);
        self
    }

    /// The space between tiles.
    pub fn gap(&self) -> f32 {
        TILE_GAP
    }

    /// Draws `faces` in rows from `at`, every tile in a row as tall as the
    /// tallest in it and never under `CATALOG_TILE_H` or `tile_h`, and
    /// returns their height.
    pub fn draw_rows(&self, ui: &mut Ui, (x, y): (f32, f32), faces: &[Face]) -> f32 {
        let mut top = y;
        for row in faces.chunks(self.per) {
            let rh = row
                .iter()
                .map(|f| f.height(ui, self.tile_w))
                .fold(TILE_H.max(self.tile_h), f32::max);
            for (c, face) in row.iter().enumerate() {
                face.draw(
                    ui,
                    x + c as f32 * (self.tile_w + TILE_GAP),
                    top,
                    (self.tile_w, rh),
                );
            }
            top += rh + CATALOG_ROW_GAP;
        }
        top - y
    }
}

/// A group of tiles under its name from `at`, in `g`: the layout Tools and
/// Check share. Returns the height up to where the next group starts.
pub(crate) fn section(
    ui: &mut Ui,
    (x, y): (f32, f32),
    g: &Grid,
    name: &str,
    faces: &[Face],
) -> f32 {
    section_label(ui, x, y, name);
    let head = ui.line_as(TITLE_S) + faraday_draw::tokens::GRID;
    head + g.draw_rows(ui, (x, y + head), faces) + faraday_draw::tokens::GRID * 2.0
}
