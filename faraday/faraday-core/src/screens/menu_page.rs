//! The menu page container (`docs/MENU-PAGE.md`): the width, inset, centring,
//! search bar, columns, gaps, top and foot of the pages a person picks a
//! place or a tool from. A page declares its name and its sections; this
//! draws the rest.

use super::catalog::{Bar, Face};
use crate::drawing::View;
use crate::ui::{Region, Ui};
use faraday_draw::tokens::{
    CATALOG_TILE_H as TILE_H, CATALOG_TILE_MIN, GRID, LEAD_MAX, MARGIN_WIDE, MENU_GAP, MENU_PAGE_W,
    PAGE_INSET, PANE_TOP_BAR, TITLE_S,
};
use faraday_ui::parts::section_label;

/// What a block draws: from `at`, `w` wide, returning its height.
pub(crate) type BlockFn<'a> = Box<dyn Fn(&mut Ui, (f32, f32), f32) -> f32 + 'a>;

/// What a section holds.
pub(crate) enum Body<'a> {
    /// Faces in the grid.
    Tiles(Vec<Face<'a>>),
    /// Something the page draws across its width from `at`, `w` wide,
    /// returning its height.
    Block(BlockFn<'a>),
    /// Cards of different heights (`docs/MENU-PAGE.md` §6), each drawn in
    /// the width it is given and returning its height: stacked in one
    /// column, or in two where the page holds two, the cards from index
    /// `second` on in the second. A card stays in its column as the
    /// window changes size.
    Cards {
        cards: Vec<BlockFn<'a>>,
        second: usize,
    },
}

/// An optional label over one body.
pub(crate) struct Section<'a> {
    pub label: Option<&'a str>,
    pub body: Body<'a>,
}

impl<'a> Section<'a> {
    /// Tiles under `label`.
    pub fn tiles(label: Option<&'a str>, faces: Vec<Face<'a>>) -> Self {
        Self {
            label,
            body: Body::Tiles(faces),
        }
    }

    /// Cards under `label`, those from `second` on in the second column
    /// where there are two.
    pub fn cards(label: Option<&'a str>, cards: Vec<BlockFn<'a>>, second: usize) -> Self {
        Self {
            label,
            body: Body::Cards { cards, second },
        }
    }

    /// A block across the page under `label`.
    pub fn block(
        label: Option<&'a str>,
        draw: impl Fn(&mut Ui, (f32, f32), f32) -> f32 + 'a,
    ) -> Self {
        Self {
            label,
            body: Body::Block(Box::new(draw)),
        }
    }
}

/// What a menu page declares of itself.
pub(crate) struct Page<'a> {
    /// The name the top app bar carries.
    pub name: &'a str,
    /// Whether the bar carries a way back (Check's does).
    pub back: bool,
    /// The search bar the page has, and its results.
    pub search: Option<Bar>,
    /// The most columns the page takes.
    pub most_columns: usize,
    /// Columns the page holds whatever the minimum says, where it has a
    /// reason to (the Pi's panel Home: its places two to a row).
    pub fixed_columns: Option<usize>,
    /// What stands above the search bar on a one-column layout.
    pub above: Option<Above<'a>>,
    /// The search bar stands still whatever the window's height: the rail's
    /// windows' Home. Elsewhere on a one-column layout it scrolls with the
    /// page where the window leaves no room under it for about three rows.
    pub bar_still: bool,
}

/// What a page puts above its search bar: how tall it is, known before it
/// is drawn, and how to draw it from `at`, `w` wide.
pub(crate) struct Above<'a> {
    pub height: f32,
    pub draw: BlockFn<'a>,
}

/// Where a page starts and how wide it is in an area `cw` wide from `x0`.
fn page_span(compact: bool, x0: f32, cw: f32) -> (f32, f32) {
    if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        let w = (cw - 2.0 * PAGE_INSET).min(MENU_PAGE_W);
        (x0 + (cw - w) / 2.0, w)
    }
}

/// The grid tiles stand in: as many columns as `CATALOG_TILE_MIN` leaves
/// room for, at least one.
pub(crate) struct Grid {
    /// Tiles to a row.
    per: usize,
    /// A tile's width.
    tile_w: f32,
    /// The least height of a tile: the tallest of the faces it was
    /// matched to, so that every tile on a page is one height.
    tile_h: f32,
}

/// The grid for a page `width` wide, at most `most` columns.
pub(crate) fn grid(width: f32, most: usize) -> Grid {
    let fit = ((width + MENU_GAP) / (CATALOG_TILE_MIN + MENU_GAP)) as usize;
    let per = fit.clamp(1, most.max(1));
    Grid {
        per,
        tile_w: (width - (per as f32 - 1.0) * MENU_GAP) / per as f32,
        tile_h: 0.0,
    }
}

/// The grid for a page `width` wide held to exactly `per` columns.
fn grid_columns(width: f32, per: usize) -> Grid {
    let per = per.max(1);
    Grid {
        per,
        tile_w: (width - (per as f32 - 1.0) * MENU_GAP) / per as f32,
        tile_h: 0.0,
    }
}

impl Grid {
    /// The grid with every tile as tall as the tallest of `faces`.
    fn matching<'a>(mut self, ui: &Ui, faces: impl IntoIterator<Item = &'a Face<'a>>) -> Self {
        self.tile_h = faces
            .into_iter()
            .map(|f| f.height(ui, self.tile_w))
            .fold(0.0, f32::max);
        self
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
                    x + c as f32 * (self.tile_w + MENU_GAP),
                    top,
                    (self.tile_w, rh),
                );
            }
            top += rh + MENU_GAP;
        }
        (top - y - MENU_GAP).max(0.0)
    }
}

/// Draws the page in the area `cw` wide from `x0`, `h` tall: the name in
/// the top app bar, the search bar standing still (on a window too short
/// for about three rows under it, scrolling with the page), the sections
/// under it, scrolling.
pub(crate) fn draw(
    app: &View,
    ui: &mut Ui,
    (x0, cw, h): (f32, f32, f32),
    page: &Page,
    sections: &[Section],
) {
    let compact = ui.compact;
    let (x, width) = page_span(compact, x0, cw);
    if !compact {
        if page.back {
            super::bar_head_back(app, ui, page.name);
        } else {
            super::bar_head(ui, None, page.name);
        }
    }
    let typed = page.search.is_some_and(|bar| match bar {
        Bar::Home => !app.tools.home_find().is_empty(),
        Bar::Tools => !app.tools.find().is_empty(),
    });
    let search = |ui: &mut Ui, y: f32, page_top: Option<f32>| {
        let sw = if compact { width } else { width.min(LEAD_MAX) };
        let bar = page.search.unwrap_or(Bar::Tools);
        super::catalog::home_search_bar(app, ui, (x + (width - sw) / 2.0, y, sw), page_top, bar)
    };
    let above_h = page.above.as_ref().map_or(0.0, |a| a.height);
    let above = |ui: &mut Ui, y: f32| {
        page.above
            .as_ref()
            .map_or(0.0, |a| (a.draw)(ui, (x, y), width))
    };
    let pinned = page.search.is_some()
        && (page.bar_still || !compact || super::home_pinned(PANE_TOP_BAR + above_h, h, TILE_H));
    let mut top = 0.0;
    let mut y = PANE_TOP_BAR;
    if pinned {
        y += above(ui, y);
        top = y + search(ui, y, None);
        y = top;
    }
    let clip = ui.rect(x0, top, cw, (h - top).max(0.0));
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    if !pinned {
        y += above(ui, y);
        if page.search.is_some() {
            y += search(ui, y, Some(0.0));
        }
    }
    if let (true, Some(bar)) = (typed, page.search) {
        y += super::catalog::home_results(app, ui, (x, y), width, bar);
    } else {
        y += sections_height(ui, (x, y), width, page, sections);
    }
    let content = y - top + MARGIN_WIDE;
    app.scroll.content_h().set(content);
    ui.region_close(open, content - (h - top));
}

/// The sections from `at`, `width` wide, one after the other. Returns
/// their height.
fn sections_height(
    ui: &mut Ui,
    (x, y): (f32, f32),
    width: f32,
    page: &Page,
    sections: &[Section],
) -> f32 {
    let faces = sections.iter().flat_map(|s| match &s.body {
        Body::Tiles(f) => f.as_slice(),
        Body::Block(_) | Body::Cards { .. } => &[],
    });
    let g = page
        .fixed_columns
        .map_or_else(
            || grid(width, page.most_columns),
            |n| grid_columns(width, n),
        )
        .matching(ui, faces);
    let mut used = 0.0;
    for (i, s) in sections.iter().enumerate() {
        if i > 0 {
            used += if s.label.is_some() {
                2.0 * GRID
            } else {
                MENU_GAP
            };
        }
        if let Some(label) = s.label {
            section_label(ui, x, y + used, label);
            used += ui.line_as(TITLE_S) + GRID;
        }
        used += match &s.body {
            Body::Tiles(faces) => g.draw_rows(ui, (x, y + used), faces),
            Body::Block(draw) => draw(ui, (x, y + used), width),
            Body::Cards { cards, second } => {
                let columns = grid(width, page.most_columns).per.min(2);
                draw_cards(ui, (x, y + used, width), cards, (columns, *second))
            }
        };
    }
    used
}

/// `cards` from `at`, `w` wide, in `columns` columns (one or two) with
/// `MENU_GAP` between cards and between the columns, the cards from index
/// `second` on in the second. Returns the height of the taller column.
fn draw_cards(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    cards: &[BlockFn],
    (columns, second): (usize, usize),
) -> f32 {
    let col_w = (w - (columns as f32 - 1.0) * MENU_GAP) / columns as f32;
    let mut used = [0.0_f32; 2];
    for (i, card) in cards.iter().enumerate() {
        let c = usize::from(columns > 1 && i >= second);
        let gap = if used[c] > 0.0 { MENU_GAP } else { 0.0 };
        let cx = x + c as f32 * (col_w + MENU_GAP);
        used[c] += gap + card(ui, (cx, y + used[c] + gap), col_w);
    }
    used[0].max(used[1])
}
