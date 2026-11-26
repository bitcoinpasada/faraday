//! The menu page container (`docs/MENU-PAGE.md`): the width, inset, centring,
//! search bar, columns, gaps, top and foot of the pages a person picks a
//! place or a tool from. A page declares its name and its sections; this
//! draws the rest.

use super::catalog::{Bar, Face, Mark};
use super::hub_layout::{self, Left, Narrow, Panes, Tile};
use crate::drawing::View;
use crate::drawing::{CardSplit, SplitKey};
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Region, Ui};
use faraday_draw::tokens::{
    BODY_L, BODY_M, CATALOG_TILE_H as TILE_H, CATALOG_TILE_MIN, CHEVRON, GRID, HOME_TILE_GLYPH,
    LABEL_S, LEAD_MAX, LIST_1, LIST_2, LIST_PAD, LIST_PAD_V, MARGIN_WIDE, MENU_CARD_COLUMNS,
    MENU_GAP, MENU_PAGE_W, MENU_ROW_DIVIDERS, MENU_ROW_ICON, MENU_TAG_GAP, MENU_TAG_H,
    MENU_TAG_PAD, PAGE_INSET, PANE_TOP_BAR, SHAPE_M, TITLE_S,
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
    /// Home's top row: the three tonal buttons beside the search bar, the
    /// row at the top of the page (`docs/HOME.md` §5).
    pub ways: bool,
    /// The page's panes on the wide layout (`screens::hub_layout`): a
    /// session pane beside its tile; none where the page has no pane.
    pub panes: Option<Panes>,
}

/// What a page puts above its search bar: how tall it is, known before it
/// is drawn, and how to draw it from `at`, `w` wide.
pub(crate) struct Above<'a> {
    pub height: f32,
    pub draw: BlockFn<'a>,
}

/// A search's results where the page has no panes of its own (Home): Home's
/// rows in the session pane, the results in the tile.
const RESULTS: Panes = Panes {
    left: Left::Session(None),
    tile: Tile::Fill,
    stacks: true,
    narrow: Narrow::OneColumn,
};

/// Where a page starts and how wide it is in an area `cw` wide from `x0`.
pub(super) fn page_span(compact: bool, x0: f32, cw: f32) -> (f32, f32) {
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

impl Grid {
    /// Tiles to a row.
    pub fn columns(&self) -> usize {
        self.per
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
    if !compact {
        if crate::row_open::opened(app).is_some() {
            super::row_details::bar(app, ui, page.name);
        } else if page.back {
            super::bar_head_back(app, ui, page.name);
        } else {
            super::bar_head(ui, None, page.name);
        }
    }
    let typed = page.search.is_some() && !app.tools.home_find().is_empty();
    // A page with a pane stands placed on the wide layout (stacked, or the
    // pair); where it cannot (a search is typed, the room is narrow) it
    // stands in Home's column, as one page.
    let (mut x0, mut cw) = (x0, cw);
    // A search's results stand in a pane of their own, placed as the page
    // is: stacked under This session's card, else beside it. Where the
    // search is not in the top bar, or the page is the phone's, they take
    // the page.
    // Pages stand side by side, not stacked, once a page or flow stood so,
    // until Home (`docs/WORKSPACE.md` §4 rule 5); Home's results reached
    // from Home stack, nothing having stood so yet.
    let apart = app.menu_apart();
    if let (true, false, true, Some(_)) = (typed, compact, app.search_in_bar(), page.search) {
        let mut panes = page.panes.unwrap_or(RESULTS);
        panes.stacks &= !apart;
        if hub_layout::draw_results(app, ui, (x0, cw, h), panes) {
            return;
        }
    }
    // A row of Home's card opened: its details stand where a search's
    // results do, under the card or beside it.
    if !compact && page.search == Some(Bar::Home) && crate::row_open::opened(app).is_some() {
        let mut panes = page.panes.unwrap_or(RESULTS);
        panes.stacks &= !apart;
        if hub_layout::draw_details(app, ui, (x0, cw, h), panes) {
            return;
        }
    }
    if let (false, Some(mut panes)) = (compact, page.panes) {
        panes.stacks &= !apart;
        if !typed && hub_layout::draw(app, ui, (x0, cw, h), (page, sections), panes) {
            return;
        }
        let (ox, area) = hub_layout::home_area(cw);
        (x0, cw) = (x0 + ox, area);
    }
    let (x, width) = page_span(compact, x0, cw);
    // On the wide layout the search stands in the top app bar
    // (`screens::bar_search`), so the page draws none.
    let in_bar = app.search_in_bar() && page.search.is_some();
    let search_bar = page.search.filter(|_| !in_bar);
    let ways = compact && !in_bar && page.ways && page.search == Some(Bar::Home);
    let search = |ui: &mut Ui, y: f32, page_top: Option<f32>| {
        if ways {
            return super::home::top_row(app, ui, (x, y, width));
        }
        let sw = if compact { width } else { width.min(LEAD_MAX) };
        super::catalog::home_search_bar(app, ui, (x + (width - sw) / 2.0, y, sw), page_top)
    };
    let above_h = page.above.as_ref().map_or(0.0, |a| a.height);
    let above = |ui: &mut Ui, y: f32| {
        page.above
            .as_ref()
            .map_or(0.0, |a| (a.draw)(ui, (x, y), width))
    };
    let pinned = search_bar.is_some()
        && (page.bar_still || super::home_pinned(PANE_TOP_BAR + above_h, h, TILE_H));
    let mut top = 0.0;
    // Home's row stands level with its place in the navigation.
    let mut y = if ways { 0.0 } else { PANE_TOP_BAR };
    if pinned {
        y += above(ui, y);
        top = y + search(ui, y, None);
        y = top;
    }
    let clip = ui.rect(x0, top, cw, (h - top).max(0.0));
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    if !pinned {
        y += above(ui, y);
        if search_bar.is_some() {
            y += search(ui, y, Some(0.0));
        }
    }
    if typed && page.search.is_some() {
        y += super::catalog::home_results(app, ui, (x, y), width);
    } else {
        y += sections_height(app, ui, (x, y), width, page, sections);
    }
    let content = y - top + MARGIN_WIDE;
    app.scroll.content_h().set(content);
    ui.region_close(open, content - (h - top));
}

/// The sections from `at`, `width` wide, one after the other. A run of
/// sections of tiles is a set of section cards (`docs/MENU-PAGE.md` §4).
/// Returns their height.
pub(super) fn sections_height(
    _app: &View,
    ui: &mut Ui,
    (x, y): (f32, f32),
    width: f32,
    page: &Page,
    sections: &[Section],
) -> f32 {
    let mut used = 0.0;
    let mut i = 0;
    while i < sections.len() {
        let s = &sections[i];
        if i > 0 {
            used += MENU_GAP;
        }
        if let Body::Tiles(_) = s.body {
            let run = sections[i..]
                .iter()
                .take_while(|s| matches!(s.body, Body::Tiles(_)))
                .count();
            used += section_cards(ui, (x, y + used, width), page, &sections[i..i + run]);
            i += run;
            continue;
        }
        if let Some(label) = s.label {
            section_label(ui, x, y + used, label);
            used += ui.line_as(TITLE_S) + GRID;
        }
        used += match &s.body {
            Body::Tiles(_) => 0.0,
            Body::Block(draw) => draw(ui, (x, y + used), width),
        };
        i += 1;
    }
    used
}

/// The faces of a section of tiles.
fn faces<'a, 'b>(s: &'b Section<'a>) -> &'b [Face<'a>] {
    match &s.body {
        Body::Tiles(f) => f,
        Body::Block(_) => &[],
    }
}

/// The room a row's text has in a card `w` wide.
fn row_room(ui: &Ui, face: &Face, w: f32) -> f32 {
    let lead = match face.mark {
        Mark::None => 0.0,
        Mark::Beside(_) | Mark::Above(_) => MENU_ROW_ICON + LIST_PAD,
    };
    w - 2.0 * LIST_PAD - lead - CHEVRON - LIST_PAD - tags_w(ui, face)
}

/// The width a row's tags take before its chevron, with the gap that
/// keeps its name and line clear of them: nothing for a row without tags.
fn tags_w(ui: &Ui, face: &Face) -> f32 {
    if face.tags.is_empty() {
        return 0.0;
    }
    face.tags
        .iter()
        .map(|tag| ui.measure_as(LABEL_S.mono(), tag) + MENU_TAG_PAD + MENU_TAG_GAP)
        .sum::<f32>()
        + GRID
}

/// A row's height in a card `w` wide: Material's two-line list item, taller
/// where its name or line wraps.
fn row_height(ui: &Ui, face: &Face, w: f32) -> f32 {
    let room = row_room(ui, face, w);
    let name = ui.wrap_height_as(BODY_L, face.name, room);
    if face.line.is_empty() {
        return (name + 2.0 * LIST_PAD_V).max(LIST_1);
    }
    let line = ui.wrap_height_as(BODY_M, face.line, room);
    (name + line + 2.0 * LIST_PAD_V).max(LIST_2)
}

/// A card's top: the heading's line and the space under it, or the corner
/// radius where it has no heading.
fn card_head(ui: &Ui, label: Option<&str>) -> f32 {
    label.map_or(SHAPE_M, |_| LIST_PAD + ui.line_as(TITLE_S) + GRID)
}

/// The height of the card of `s` `w` wide.
fn card_height(ui: &Ui, s: &Section, w: f32) -> f32 {
    card_head(ui, s.label) + faces(s).iter().map(|f| row_height(ui, f, w)).sum::<f32>() + SHAPE_M
}

/// The card of `s` at `(x, y)`, `w` wide, `h` tall: the heading, then a
/// list of rows with an icon where the face has one, the name, the line
/// under it and a chevron, a state layer across each row, and a 1 dp
/// outline-variant line at the edge.
fn draw_card(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32), s: &Section) {
    if !ui.shows(y, h) {
        return;
    }
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainer);
    if let Some(label) = s.label {
        section_label(ui, x + LIST_PAD, y + LIST_PAD, label);
    }
    let mut ry = y + card_head(ui, s.label);
    for (n, face) in faces(s).iter().enumerate() {
        let rh = row_height(ui, face, w);
        if n > 0 && MENU_ROW_DIVIDERS {
            ui.fill(
                x + LIST_PAD,
                ry,
                w - 2.0 * LIST_PAD,
                1.0,
                0.0,
                Role::OutlineVariant,
            );
        }
        draw_row(ui, (x, ry, w, rh), face);
        ry += rh;
    }
    // Material 3's outlined card: the 1 dp line is drawn last, so a row's
    // state layer that reaches the card's edge does not cover it.
    ui.stroke(x, y, w, h, SHAPE_M, Role::OutlineVariant);
}

/// One row at `(x, y)`, `w` wide, `h` tall.
fn draw_row(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32), face: &Face) {
    if face.opens {
        ui.press(
            Shape::new(x, y, w, h, 0.0),
            Role::OnSurface.paint(),
            face.action,
        );
    }
    let ink = if face.opens {
        Role::OnSurface.paint()
    } else {
        crate::ui::off()
    };
    let mut tx = x + LIST_PAD;
    if let Mark::Beside(glyph) | Mark::Above(glyph) = face.mark {
        let (bg, fg) = if face.opens {
            (
                Role::SecondaryContainer.paint(),
                Role::OnSecondaryContainer.paint(),
            )
        } else {
            (
                Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTAINER),
                crate::ui::off(),
            )
        };
        let iy = y + (h - MENU_ROW_ICON) / 2.0;
        ui.fill(tx, iy, MENU_ROW_ICON, MENU_ROW_ICON, SHAPE_M, bg);
        ui.icon(tx, iy, MENU_ROW_ICON, glyph, HOME_TILE_GLYPH, fg);
        tx += MENU_ROW_ICON + LIST_PAD;
    }
    let room = row_room(ui, face, w);
    let name = ui.wrap_height_as(BODY_L, face.name, room);
    let line = if face.line.is_empty() {
        0.0
    } else {
        ui.wrap_height_as(BODY_M, face.line, room)
    };
    let top = y + (h - name - line) / 2.0;
    ui.wrap_as((tx, top, room), BODY_L, ink, face.name);
    let muted = if face.warn {
        Role::Warning
    } else {
        Role::OnSurfaceVariant
    };
    ui.wrap_as((tx, top + name, room), BODY_M, muted, face.line);
    let tag_end = x + w - LIST_PAD - CHEVRON - GRID;
    let mut gx = tag_end;
    for tag in face.tags.iter().rev() {
        let gw = ui.measure_as(LABEL_S.mono(), tag) + MENU_TAG_PAD;
        let ty = y + (h - MENU_TAG_H) / 2.0;
        gx -= gw;
        if gx < tx {
            break;
        }
        ui.fill(
            gx,
            ty,
            gw,
            MENU_TAG_H,
            faraday_draw::tokens::shape_full(MENU_TAG_H),
            Role::SurfaceContainerHighest,
        );
        ui.text_mid_as(
            (gx + MENU_TAG_PAD / 2.0, ty, MENU_TAG_H),
            LABEL_S.mono(),
            Role::OnSurfaceVariant,
            tag,
        );
        gx -= MENU_TAG_GAP;
    }
    ui.row_chevron(x + w - LIST_PAD, y, h, Role::OnSurfaceVariant);
}

/// The run of sections of tiles `run` from `at`, `width` wide, as flat
/// section cards: two or three to a row where the width holds two
/// `CATALOG_TILE_MIN`, one on a page under it and where the page holds one
/// column; each as tall as its own list, divided into columns where their
/// heights come out closest. Returns the height of the tallest column.
fn section_cards(ui: &mut Ui, (x, y, width): (f32, f32, f32), page: &Page, run: &[Section]) -> f32 {
    let fit = page
        .fixed_columns
        .unwrap_or_else(|| grid(width, page.most_columns).per);
    let columns = if page.fixed_columns.is_some() {
        1
    } else {
        fit.clamp(1, MENU_CARD_COLUMNS).min(run.len())
    };
    let col_w = (width - (columns as f32 - 1.0) * MENU_GAP) / columns as f32;
    let heights: Vec<f32> = run.iter().map(|s| card_height(ui, s, col_w)).collect();
    let starts = column_starts(&heights, columns);
    let mut used = vec![0.0_f32; columns];
    for (i, s) in run.iter().enumerate() {
        let c = starts.iter().filter(|&&k| i >= k).count();
        let gap = if used[c] > 0.0 { MENU_GAP } else { 0.0 };
        let cx = x + c as f32 * (col_w + MENU_GAP);
        draw_card(ui, (cx, y + used[c] + gap, col_w, heights[i]), s);
        used[c] += gap + heights[i];
    }
    used.into_iter().fold(0.0, f32::max)
}

/// The indices the second and later columns start at, so that `columns`
/// columns (one, two or three) of consecutive cards of `heights` with
/// `MENU_GAP` between come out the shortest, then closest in height.
fn column_starts(heights: &[f32], columns: usize) -> Vec<usize> {
    let n = heights.len();
    let col = |a: usize, b: usize| {
        heights[a..b].iter().sum::<f32>() + MENU_GAP * (b - a).saturating_sub(1) as f32
    };
    let score = |cuts: &[usize]| {
        let mut at = 0;
        let (mut tall, mut squares) = (0.0_f32, 0.0_f32);
        for &end in cuts.iter().chain(std::iter::once(&n)) {
            let h = col(at, end);
            tall = tall.max(h);
            squares += h * h;
            at = end;
        }
        (tall, squares)
    };
    let mut best: Option<((f32, f32), Vec<usize>)> = None;
    let mut consider = |cuts: Vec<usize>| {
        let s = score(&cuts);
        if best.as_ref().is_none_or(|(b, _)| s < *b) {
            best = Some((s, cuts));
        }
    };
    match columns {
        0 | 1 => {}
        2 => (1..n).for_each(|i| consider(vec![i])),
        _ => {
            for i in 1..n {
                for j in i + 1..n {
                    consider(vec![i, j]);
                }
            }
        }
    }
    best.map_or_else(Vec::new, |(_, cuts)| cuts)
}

/// `cards` from `at`, `w` wide, in `columns` columns (one or two) with
/// `MENU_GAP` between cards and between the columns. On two, the cards
/// divide where the columns' heights come out closest, the order read down
/// the first and then the second (owner, 2026-11-21). The division is kept
/// for the column width, scale and text size, and read while drawing; a
/// card whose height drawn differs from the one kept (its content changed)
/// replaces it, and the next frame divides by it. Returns the height of
/// the taller column.
pub(super) fn draw_cards(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    cards: &[BlockFn],
    columns: usize,
) -> f32 {
    let col_w = (w - (columns as f32 - 1.0) * MENU_GAP) / columns as f32;
    let key = SplitKey {
        col_w,
        scale: ui.f,
        text: ui.text_factor(),
    };
    let split = if columns > 1 {
        match app
            .card_split_kept(key)
            .filter(|s| s.heights.len() == cards.len())
        {
            Some(s) => s,
            None => {
                let heights: Vec<f32> = cards
                    .iter()
                    .map(|card| crate::menu::measure(ui, &mut |ui| card(ui, (x, 0.0), col_w)))
                    .collect();
                let split = CardSplit {
                    second: balanced_split(&heights),
                    heights,
                };
                app.card_split_keep(key, split.clone());
                split
            }
        }
    } else {
        CardSplit {
            heights: Vec::new(),
            second: cards.len(),
        }
    };
    let mut used = [0.0_f32; 2];
    let mut drawn = Vec::with_capacity(cards.len());
    for (i, card) in cards.iter().enumerate() {
        let c = usize::from(columns > 1 && i >= split.second);
        let gap = if used[c] > 0.0 { MENU_GAP } else { 0.0 };
        let cx = x + c as f32 * (col_w + MENU_GAP);
        let h = card(ui, (cx, y + used[c] + gap), col_w);
        used[c] += gap + h;
        drawn.push(h);
    }
    if columns > 1 && drawn != split.heights {
        let second = balanced_split(&drawn);
        app.card_split_keep(
            key,
            CardSplit {
                heights: drawn,
                second,
            },
        );
    }
    used[0].max(used[1])
}

/// The index the second column starts at so the two columns, cards of
/// `heights` with `MENU_GAP` between, come out closest in height (the
/// shorter taller column on a tie), at least one card in each.
fn balanced_split(heights: &[f32]) -> usize {
    let column = |h: &[f32]| h.iter().sum::<f32>() + MENU_GAP * h.len().saturating_sub(1) as f32;
    let mut best = (f32::MAX, f32::MAX, heights.len());
    for k in 1..heights.len() {
        let (a, b) = (column(&heights[..k]), column(&heights[k..]));
        let score = ((a - b).abs(), a.max(b));
        if score < (best.0, best.1) {
            best = (score.0, score.1, k);
        }
    }
    best.2
}
