//! The menu page template (09 §3.3, `docs/MENU-PAGE.md`): the width, inset, centring, the
//! search row of the small panel, the columns, gaps and foot of the pages a person picks a place
//! or a tool from. A page declares its sections; this draws the rest. The page's name and, on
//! the wide layout, its search are the screen template's (`FrameModel`).
//!
//! Built: tiles as section cards, a block across the page, cards of different heights, the box
//! of rows, the grid and its column split, the page's panes through the pane container (the tile
//! raised and padded), a typed search's results in place of the sections, Home's lead above the
//! search bar, the search bar's scroll with the page on a short window, the scroll.

use faraday_atoms::hits::Shape;
use faraday_atoms::region::Region;
use faraday_atoms::{Anchor, Glyph, Offered, Pressed, Shown, Ui};
use faraday_organisms::content::{self, Content};
use faraday_organisms::nav;
use faraday_organisms::row_details::RowDetailsModel;
use faraday_organisms::session_pane::{self, SessionRow};
use faraday_tokens::dims::{
    BODY_L, BODY_M, CATALOG_TILE_MIN, CHEVRON, DISABLED_CONTAINER, DISABLED_CONTENT, GAP, GRID,
    HOME_BOX_KEYS, HOME_BOX_MIN_ROWS, HOME_RESULTS_ROWS, HOME_TILE_GLYPH, LABEL_S, LEAD_MAX,
    LIST_1, LIST_2, LIST_PAD, LIST_PAD_V, MARGIN_COMPACT, MARGIN_WIDE, MENU_CARD_COLUMNS, MENU_GAP,
    MENU_PAGE_W, MENU_ROW_DIVIDERS, MENU_ROW_ICON, MENU_TAG_GAP, MENU_TAG_H, MENU_TAG_PAD, PAD,
    PAGE_INSET, PANE_TOP_BAR, SEARCH_BAR_H, SHAPE_M, TILE_H, TITLE_S, shape_full,
};
use faraday_tokens::palette::Role;

use crate::panes::{OnPlaced, Panes, Parts, tile_surface};
use crate::screen::{BarSearch, page_room};

/// What a tile's icon does for the layout.
pub enum Mark {
    /// None: the text starts at the tile's edge (Tools, Check).
    None,
    /// An icon in a container before the text, centred on the row's height.
    Beside(Glyph),
    /// An icon in a container over the text, for a tile too narrow to have the text beside it.
    Above(Glyph),
}

/// What a tile shows: its icon, its name, the line under it, whether the line is a warning, its
/// small tags and what a press does.
pub struct Face<'s, A> {
    pub mark: Mark,
    pub name: Shown<'s>,
    pub line: Shown<'s>,
    pub warn: bool,
    pub tags: Vec<Shown<'s>>,
    pub offered: Offered<A>,
}

/// What a section holds.
pub enum Body<'s, A> {
    /// Faces in the grid, drawn as one section card of rows.
    Tiles(Vec<Face<'s, A>>),
    /// Something drawn across the page's width.
    Block(Content<'s, A>),
    /// Cards of different heights (Settings): in one column, or two where two fit, each card
    /// under the one before it, divided where the columns come out closest in height.
    Cards(Vec<Content<'s, A>>),
    /// One tinted box of rows (the Pi's and the phone's Home): its heading, the lead, then the
    /// rows.
    Box(RowsBox<'s, A>),
}

/// The box of what is loaded: a heading, the lead card, the rows, then as many key rows as the
/// room holds.
pub struct RowsBox<'s, A> {
    pub heading: Shown<'s>,
    pub lead: Option<Content<'s, A>>,
    pub rows: Vec<SessionRow<'s, A>>,
    /// Key rows, shown after the rows up to `HOME_BOX_KEYS` and as many as the room holds.
    pub keys: Vec<SessionRow<'s, A>>,
    /// The box takes the page's room under the fixed part and scrolls within itself (the rail's
    /// windows); otherwise the page scrolls as one.
    pub fit: bool,
    /// The box's own scroll.
    pub scroll: Anchor,
}

/// An optional label over one body.
pub struct Section<'s, A> {
    pub label: Option<Shown<'s>>,
    pub body: Body<'s, A>,
}

/// The model: what a menu page declares of itself.
pub struct MenuPageModel<'s, A> {
    /// The page's scroll, keyed in the motion store.
    pub scroll: Anchor,
    /// The search row the small panel's page draws; on the wide layout the bar holds it.
    pub row: Option<BarSearch<'s, A>>,
    /// The most columns the page takes.
    pub most_columns: usize,
    /// Columns the page holds whatever the minimum says (the Pi's panel Home: places two to a
    /// row).
    pub fixed_columns: Option<usize>,
    /// The search row stands still while the sections scroll under it.
    pub bar_still: bool,
    /// The page's panes on the wide layout: a session pane beside its tile; none where the page
    /// has no pane.
    pub panes: Option<Panes<'s, A>>,
    pub sections: Vec<Section<'s, A>>,
    /// What stands above the search bar (Home's lead), across the page's width.
    pub above: Option<Content<'s, A>>,
    /// What a typed search shows in place of the sections.
    pub results: Option<Results<'s, A>>,
    /// The open row's details, which stand in the tile's place beside or under This session.
    pub details: Option<RowDetailsModel<'s, A>>,
    /// Where the page stood, reported as `Laid::Placed`.
    pub on_placed: Option<OnPlaced<A>>,
    /// The session pane glides to its new place when the pair moves, keyed by this anchor.
    pub glide: Option<Anchor>,
    /// The scroll of the tile in a pair, within the window's height.
    pub tile_scroll: Option<Anchor>,
}

/// A typed search's results: sections in the place of the page's own, placed as the page is, or
/// as `panes` where the page has none of its own.
pub struct Results<'s, A> {
    pub sections: Vec<Section<'s, A>>,
    pub panes: Option<Panes<'s, A>>,
}

/// Whether the page down to the search bar stays still while a search is typed, only the results
/// under it scrolling: where `above` and the bar leave `window` room for `HOME_RESULTS_ROWS` rows.
/// Where they do not, the whole page scrolls as one.
pub fn home_pinned(above: f32, window: f32) -> bool {
    above + SEARCH_BAR_H + HOME_RESULTS_ROWS * TILE_H <= window
}

/// Where a page starts and how wide it is in an area `cw` wide from `x0`.
pub fn page_span(compact: bool, x0: f32, cw: f32) -> (f32, f32) {
    if compact {
        (x0 + MARGIN_COMPACT, cw - 2.0 * MARGIN_COMPACT)
    } else {
        let w = (cw - 2.0 * PAGE_INSET).min(MENU_PAGE_W);
        (x0 + (cw - w) / 2.0, w)
    }
}

/// The grid tiles stand in: as many columns as `CATALOG_TILE_MIN` leaves room for, at least one.
pub struct Grid {
    per: usize,
    tile_w: f32,
}

/// The grid for a page `width` wide, at most `most` columns.
pub fn grid(width: f32, most: usize) -> Grid {
    let fit = ((width + MENU_GAP) / (CATALOG_TILE_MIN + MENU_GAP)) as usize;
    let per = fit.clamp(1, most.max(1));
    Grid {
        per,
        tile_w: (width - (per as f32 - 1.0) * MENU_GAP) / per as f32,
    }
}

impl Grid {
    /// Tiles to a row.
    pub fn columns(&self) -> usize {
        self.per
    }

    /// A tile's width.
    pub fn tile_w(&self) -> f32 {
        self.tile_w
    }
}

fn section_label<A: Pressed>(ui: &mut Ui<'_, A>, x: f32, y: f32, label: &Shown<'_>) {
    ui.showing(label, |ui| {
        ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, label.as_str());
    });
}

/// The room a row's text has in a card `w` wide.
fn row_room<A: Pressed>(ui: &Ui<'_, A>, face: &Face<'_, A>, w: f32) -> f32 {
    let lead = match face.mark {
        Mark::None => 0.0,
        Mark::Beside(_) | Mark::Above(_) => MENU_ROW_ICON + LIST_PAD,
    };
    w - 2.0 * LIST_PAD - lead - CHEVRON - LIST_PAD - tags_w(ui, face)
}

/// The width a row's tags take before its chevron, with the gap that keeps its name and line
/// clear of them: nothing for a row without tags.
fn tags_w<A: Pressed>(ui: &Ui<'_, A>, face: &Face<'_, A>) -> f32 {
    if face.tags.is_empty() {
        return 0.0;
    }
    face.tags
        .iter()
        .map(|tag| ui.measure_as(LABEL_S.mono(), tag.as_str()) + MENU_TAG_PAD + MENU_TAG_GAP)
        .sum::<f32>()
        + GRID
}

/// A row's height in a card `w` wide: Material's two-line list item, taller where its name or
/// line wraps.
fn row_height<A: Pressed>(ui: &Ui<'_, A>, face: &Face<'_, A>, w: f32) -> f32 {
    let room = row_room(ui, face, w);
    let name = ui.wrap_height_as(BODY_L, face.name.as_str(), room);
    if face.line.as_str().is_empty() {
        return (name + 2.0 * LIST_PAD_V).max(LIST_1);
    }
    let line = ui.wrap_height_as(BODY_M, face.line.as_str(), room);
    (name + line + 2.0 * LIST_PAD_V).max(LIST_2)
}

/// A card's top: the heading's line and the space under it, or the corner radius where it has no
/// heading.
fn card_head<A: Pressed>(ui: &Ui<'_, A>, label: Option<&Shown<'_>>) -> f32 {
    label.map_or(SHAPE_M, |_| LIST_PAD + ui.line_as(TITLE_S) + GRID)
}

fn faces<'a, 's, A>(s: &'a Section<'s, A>) -> &'a [Face<'s, A>] {
    match &s.body {
        Body::Tiles(f) => f,
        _ => &[],
    }
}

/// The height of the card of `s` `w` wide.
fn card_height<A: Pressed>(ui: &Ui<'_, A>, s: &Section<'_, A>, w: f32) -> f32 {
    card_head(ui, s.label.as_ref())
        + faces(s).iter().map(|f| row_height(ui, f, w)).sum::<f32>()
        + SHAPE_M
}

/// The card of `s` at `(x, y)`, `w` wide, `h` tall: the heading, then rows with an icon where
/// the face has one, the name, the line under it and a chevron, a state layer across each row,
/// and a 1 dp outline-variant line at the edge.
fn draw_card<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
    s: &Section<'_, A>,
) {
    if !ui.shows(y, h) {
        return;
    }
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainer);
    if let Some(label) = &s.label {
        section_label(ui, x + LIST_PAD, y + LIST_PAD, label);
    }
    let mut ry = y + card_head(ui, s.label.as_ref());
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
    // The outline is drawn last, so a row's state layer that reaches the card's edge does not
    // cover it.
    ui.stroke(x, y, w, h, SHAPE_M, Role::OutlineVariant);
}

/// One row at `(x, y)`, `w` wide, `h` tall.
fn draw_row<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
    face: &Face<'_, A>,
) {
    ui.offered(&face.offered, |ui, press, dimmed| {
        if let Some(action) = press {
            ui.press(Shape::new(x, y, w, h, 0.0), Role::OnSurface.paint(), action);
        }
        let ink = if dimmed {
            Role::OnSurface.at(DISABLED_CONTENT)
        } else {
            Role::OnSurface.paint()
        };
        let mut tx = x + LIST_PAD;
        if let Mark::Beside(glyph) | Mark::Above(glyph) = face.mark {
            let (bg, fg) = if dimmed {
                (
                    Role::OnSurface.at(DISABLED_CONTAINER),
                    Role::OnSurface.at(DISABLED_CONTENT),
                )
            } else {
                (
                    Role::SecondaryContainer.paint(),
                    Role::OnSecondaryContainer.paint(),
                )
            };
            let iy = y + (h - MENU_ROW_ICON) / 2.0;
            ui.fill(tx, iy, MENU_ROW_ICON, MENU_ROW_ICON, SHAPE_M, bg);
            ui.icon(tx, iy, MENU_ROW_ICON, glyph, HOME_TILE_GLYPH, fg);
            tx += MENU_ROW_ICON + LIST_PAD;
        }
        let room = row_room(ui, face, w);
        let (name, line) = (face.name.as_str(), face.line.as_str());
        let name_h = ui.wrap_height_as(BODY_L, name, room);
        let line_h = if line.is_empty() {
            0.0
        } else {
            ui.wrap_height_as(BODY_M, line, room)
        };
        let top = y + (h - name_h - line_h) / 2.0;
        ui.wrap_as((tx, top, room), BODY_L, ink, name);
        let muted = if face.warn {
            Role::Warning
        } else {
            Role::OnSurfaceVariant
        };
        ui.showing(&face.line, |ui| {
            ui.wrap_as((tx, top + name_h, room), BODY_M, muted, line);
        });
        let mut gx = x + w - LIST_PAD - CHEVRON - GRID;
        for tag in face.tags.iter().rev() {
            let gw = ui.measure_as(LABEL_S.mono(), tag.as_str()) + MENU_TAG_PAD;
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
                shape_full(MENU_TAG_H),
                Role::SurfaceContainerHighest,
            );
            ui.text_mid_as(
                (gx + MENU_TAG_PAD / 2.0, ty, MENU_TAG_H),
                LABEL_S.mono(),
                Role::OnSurfaceVariant,
                tag.as_str(),
            );
            gx -= MENU_TAG_GAP;
        }
        ui.row_chevron(x + w - LIST_PAD, y, h, Role::OnSurfaceVariant);
    });
}

/// The indices the second and later columns start at, so that `columns` columns (one, two or
/// three) of consecutive cards of `heights` with `MENU_GAP` between come out the shortest, then
/// closest in height.
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

/// The run of sections of tiles from `at`, `width` wide, as flat section cards: two or three to
/// a row where the width holds two `CATALOG_TILE_MIN`, one under it and where the page holds one
/// column; each as tall as its own list, divided into columns where their heights come out
/// closest. Returns the height of the tallest column.
fn section_cards<A: Pressed>(
    m: &MenuPageModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, width): (f32, f32, f32),
    run: &[Section<'_, A>],
) -> f32 {
    let fit = m
        .fixed_columns
        .unwrap_or_else(|| grid(width, m.most_columns).columns());
    let columns = if m.fixed_columns.is_some() {
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

/// The index the second column starts at so the two columns, cards of `heights` with `MENU_GAP`
/// between, come out closest in height (the shorter taller column on a tie), at least one card in
/// each.
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

/// `cards` from `(x, y)`, `width` wide, in `columns` columns (one or two) with `MENU_GAP` between
/// cards and between the columns; on two, divided where the columns' heights come out closest.
/// Returns the height of the taller column.
fn draw_cards<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, width): (f32, f32, f32),
    cards: &[Content<'_, A>],
    columns: usize,
) -> f32 {
    let col_w = (width - (columns as f32 - 1.0) * MENU_GAP) / columns as f32;
    let heights: Vec<f32> = cards
        .iter()
        .map(|c| content::height(c, ui, col_w))
        .collect();
    let second = if columns > 1 {
        balanced_split(&heights)
    } else {
        cards.len()
    };
    let mut used = [0.0_f32; 2];
    for (i, card) in cards.iter().enumerate() {
        let c = usize::from(columns > 1 && i >= second);
        let gap = if used[c] > 0.0 { MENU_GAP } else { 0.0 };
        let cx = x + c as f32 * (col_w + MENU_GAP);
        content::draw(card, ui, (cx, y + used[c] + gap, col_w));
        used[c] += gap + heights[i];
    }
    used[0].max(used[1])
}

/// How many of `keys` key rows (two lines each) the box shows: up to `cap`, and only as many as
/// `avail` holds besides the other rows' `rest_h`.
fn keys_fitting(avail: f32, rest_h: f32, keys: usize, cap: usize) -> usize {
    let room = ((avail - rest_h) / LIST_2).floor().max(0.0) as usize;
    keys.min(cap).min(room)
}

/// The surface of a box of what is loaded: a tinted, outlined, rounded container.
fn loaded_surface<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
    ui.stroke(x, y, w, h, SHAPE_M, Role::OutlineVariant);
}

/// The box of rows from `(bx, y)`, `bw` wide: its heading, the lead, then the rows and as many
/// key rows as the room holds, scrolling within itself where it holds fewer than two rows' worth.
/// `window` is the page's height where the box takes the room under the fixed part. Returns the
/// height it takes.
fn draw_box<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (bx, y, bw): (f32, f32, f32),
    b: &RowsBox<'_, A>,
    window: Option<f32>,
) -> f32 {
    section_label(ui, bx, y, &b.heading);
    let head = ui.line_as(TITLE_S) + GRID;
    let by = y + head;
    let lead_h = b
        .lead
        .as_ref()
        .map_or(0.0, |l| content::draw(l, ui, (bx, by, bw)) + GRID);
    let (by, head) = (by + lead_h, head + lead_h);
    let rest_h = session_pane::rows_h(ui, bw, &b.rows);
    // Without a window height the page scrolls as a whole: up to the most keys. With one, the
    // box's room runs to a margin above the foot.
    let avail = window
        .filter(|_| b.fit)
        .map_or(f32::INFINITY, |h| (h - by - MARGIN_WIDE).floor());
    let n = keys_fitting(avail, rest_h, b.keys.len(), HOME_BOX_KEYS);
    let all_h = rest_h + n as f32 * LIST_2;
    let rows: Vec<&SessionRow<'_, A>> = b.rows.iter().chain(b.keys.iter().take(n)).collect();
    if all_h <= avail || avail < HOME_BOX_MIN_ROWS * LIST_2 {
        let h = session_pane::rows_h(ui, bw, rows.iter().copied());
        loaded_surface(ui, (bx, by, bw, h));
        session_pane::list_rows(ui, (bx, by), bw, rows.iter().copied());
        return head + h;
    }
    // Scrolls within itself: the surface stays, the rows move.
    loaded_surface(ui, (bx, by, bw, avail));
    let clip = ui.rect(bx, by, bw, avail);
    let mark = ui.presses_mark();
    let open = ui.region_open(Region::new(b.scroll, clip).moves());
    session_pane::list_rows(ui, (bx, by), bw, rows.iter().copied());
    ui.clip_presses_since(mark, clip);
    ui.region_close(open, (all_h - avail).max(0.0));
    head + avail
}

/// The sections `all` from `at`, `width` wide, one after the other. A run of sections of tiles is
/// a set of section cards. `window` is the page's height where the page stands on its own.
/// Returns their height.
fn sections<A: Pressed>(
    m: &MenuPageModel<'_, A>,
    all: &[Section<'_, A>],
    ui: &mut Ui<'_, A>,
    (x, y): (f32, f32),
    width: f32,
    window: Option<f32>,
) -> f32 {
    let (mut used, mut i) = (0.0, 0);
    while i < all.len() {
        if i > 0 {
            used += MENU_GAP;
        }
        if let Body::Tiles(_) = all[i].body {
            let run = all[i..]
                .iter()
                .take_while(|s| matches!(s.body, Body::Tiles(_)))
                .count();
            used += section_cards(m, ui, (x, y + used, width), &all[i..i + run]);
            i += run;
            continue;
        }
        if let Some(label) = &all[i].label {
            section_label(ui, x, y + used, label);
            used += ui.line_as(TITLE_S) + GRID;
        }
        match &all[i].body {
            Body::Tiles(_) => {}
            Body::Block(content) => used += content::draw(content, ui, (x, y + used, width)),
            Body::Cards(cards) => {
                let columns = grid(width, m.most_columns).columns().clamp(1, 2);
                used += draw_cards(ui, (x, y + used, width), cards, columns);
            }
            Body::Box(b) => used += draw_box(ui, (x, y + used, width), b, window),
        }
        i += 1;
    }
    used
}

/// The small panel's search row at `y`: the field, with Home's first actions beside it where
/// the page has them. Returns the height it takes, the gap under it counted.
fn search_row<A: Pressed>(
    row: &BarSearch<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, width): (f32, f32, f32),
) -> f32 {
    nav::draw_group(
        &row.field,
        row.ways.as_ref(),
        ui,
        ((x, x + width), x + width / 2.0),
        (y, LEAD_MAX),
    );
    SEARCH_BAR_H + GAP
}

/// Draws the page in the room the screen template left it: Home's lead and the search row of the
/// small panel (standing still above the sections where the page says so or the window has room
/// for about three rows under it, else scrolling with them), then the sections or a typed
/// search's results, scrolling; on the wide layout the page's panes through the pane container,
/// the tile raised and padded.
pub fn draw<A: Pressed>(m: &MenuPageModel<'_, A>, ui: &mut Ui<'_, A>) {
    let (cw, h) = page_room(ui);
    let compact = ui.compact;
    let (x, width) = page_span(compact, 0.0, cw);
    let row = m.row.as_ref().filter(|_| compact);
    let panes = m
        .results
        .as_ref()
        .and_then(|r| r.panes.as_ref())
        .or(m.panes.as_ref())
        .filter(|_| !compact);
    let all: &[Section<'_, A>] = m.results.as_ref().map_or(&m.sections, |r| &r.sections);
    let above_h = match (&m.above, panes) {
        (Some(a), None) => content::height(a, ui, width),
        _ => 0.0,
    };
    let mut y = if row.is_some_and(|r| r.ways.is_some()) {
        0.0
    } else {
        PANE_TOP_BAR
    };
    let pinned = row.is_some() && (m.bar_still || home_pinned(PANE_TOP_BAR + above_h, h));
    let mut top = 0.0;
    let lead = |ui: &mut Ui<'_, A>, y: f32| {
        m.above
            .as_ref()
            .filter(|_| panes.is_none())
            .map_or(0.0, |a| content::draw(a, ui, (x, y, width)))
    };
    if let (true, Some(row)) = (pinned, row) {
        y += lead(ui, y);
        y += search_row(row, ui, (x, y, width));
        top = y;
    }
    let clip = ui.rect(0.0, top, cw, (h - top).max(0.0));
    let open = ui.region_open(Region::new(m.scroll, clip).moves());
    if let (false, Some(row)) = (pinned, row) {
        y += lead(ui, y);
        y += search_row(row, ui, (x, y, width));
    } else if !pinned {
        y += lead(ui, y);
    }
    y += match panes {
        Some(panes) => {
            let tile = |ui: &mut Ui<'_, A>, (tx, ty, tw): (f32, f32, f32)| {
                let inner = |ui: &mut Ui<'_, A>, x: f32, y: f32| {
                    sections(m, all, ui, (x + PAD, y + PAD), tw - 2.0 * PAD, None) + 2.0 * PAD
                };
                let th = ui.measured(|ui| inner(ui, 0.0, 0.0));
                tile_surface(ui, (tx, ty, tw, th));
                inner(ui, tx, ty)
            };
            let parts = Parts {
                own: None,
                details: m.details.as_ref(),
                guide: None,
                on_placed: m.on_placed,
                scroll: m.tile_scroll,
                glide: m.glide,
            };
            crate::panes::draw_with(panes, ui, (0.0, cw), y, parts, tile)
        }
        None => sections(m, all, ui, (x, y), width, Some(h)),
    };
    let content = y - top + MARGIN_WIDE;
    ui.region_close(open, content - (h - top));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::{FrameModel, draw as draw_screen};
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame};
    use faraday_organisms::nav::{NavModel, TopBarModel};

    fn face<'s>(name: &'static str, n: u8) -> Face<'s, P> {
        Face {
            mark: Mark::None,
            name: Shown::plain(name),
            line: Shown::plain("a line"),
            warn: false,
            tags: Vec::new(),
            offered: Offered::Yes(P(n)),
        }
    }

    fn page<'s>(sections: Vec<Section<'s, P>>) -> MenuPageModel<'s, P> {
        MenuPageModel {
            scroll: Anchor {
                scope: "menu",
                n: 0,
            },
            row: None,
            most_columns: 3,
            fixed_columns: None,
            bar_still: false,
            panes: None,
            sections,
            above: None,
            results: None,
            details: None,
            on_placed: None,
            glide: None,
            tile_scroll: None,
        }
    }

    /// Invariant: Two or three cards divide into columns so that the columns come out the shortest, then closest in height, read down the first column and on to the next.
    /// Regression: The cards left in the order of the page with one column far taller.
    /// Owner: kit
    #[test]
    fn cards_divide_into_the_shortest_columns() {
        assert_eq!(column_starts(&[100.0, 100.0, 200.0], 2), vec![2]);
        assert_eq!(column_starts(&[100.0, 100.0, 100.0], 3), vec![1, 2]);
        assert!(column_starts(&[100.0, 100.0], 1).is_empty());
    }

    /// Invariant: Each tile of a section card takes its own press, and a tile not offered takes none.
    /// Regression: A menu page's rows that open nothing, or a refused tile that still opens.
    /// Owner: kit
    #[test]
    fn each_offered_tile_takes_its_press() {
        let mut c = canvas(160);
        let mut off = face("Third", 3);
        off.offered = Offered::No;
        let m = page(vec![Section {
            label: Some(Shown::plain("Places")),
            body: Body::Tiles(vec![face("First", 1), face("Second", 2), off]),
        }]);
        let frame_model = FrameModel {
            nav: NavModel {
                dests: Vec::new(),
                many: Shown::plain("999+"),
            },
            top: TopBarModel {
                back: None,
                title: Shown::plain("Home"),
                pill_w: 0.0,
                item_name: false,
                in_sheet: false,
                under: None,
                right: 640.0,
                power: None,
            },
            busy: None,
            search: None,
            side: crate::screen::Side::None,
            keyboard: None,
            pills: None,
            tip: None,
        };
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.compact = true;
            draw_screen(
                Anchor { scope: "t", n: 1 },
                &frame_model,
                ui,
                |ui| draw(&m, ui),
                &[] as &[(Anchor, ())],
                |_, _| {},
                None,
            );
        });
        let presses: Vec<u8> = out.hits.iter().map(|h| h.press.0).collect();
        assert!(presses.contains(&1) && presses.contains(&2));
        assert!(!presses.contains(&3));
        assert_eq!(out.regions.len(), 1);
    }

    fn frame_for<'s>() -> crate::screen::FrameModel<'s, P> {
        crate::screen::FrameModel {
            nav: NavModel {
                dests: Vec::new(),
                many: Shown::plain("999+"),
            },
            top: TopBarModel {
                back: None,
                title: Shown::plain("Home"),
                pill_w: 0.0,
                item_name: false,
                in_sheet: false,
                under: None,
                right: 640.0,
                power: None,
            },
            busy: None,
            search: None,
            side: crate::screen::Side::None,
            keyboard: None,
            pills: None,
            tip: None,
        }
    }

    fn presses_of(m: &MenuPageModel<'_, P>) -> Vec<u8> {
        let mut c = canvas(160);
        let f = frame_for();
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.compact = true;
            draw_screen(
                Anchor { scope: "t", n: 1 },
                &f,
                ui,
                |ui| draw(m, ui),
                &[] as &[(Anchor, ())],
                |_, _| {},
                None,
            );
        });
        out.hits.iter().map(|h| h.press.0).collect()
    }

    /// Invariant: A typed search's results stand in place of the page's sections: only the results take presses.
    /// Regression: The sections drawn under the results, or the results never shown.
    /// Owner: kit
    #[test]
    fn a_typed_search_shows_its_results_in_place_of_the_sections() {
        let mut m = page(vec![Section {
            label: None,
            body: Body::Tiles(vec![face("Section", 1)]),
        }]);
        m.results = Some(Results {
            sections: vec![Section {
                label: None,
                body: Body::Tiles(vec![face("Result", 2)]),
            }],
            panes: None,
        });
        let presses = presses_of(&m);
        assert!(presses.contains(&2));
        assert!(!presses.contains(&1));
    }

    /// Invariant: Cards of different heights divide into two columns where the columns come out closest in height, in reading order, and at least one stands in each.
    /// Regression: Settings' cards left in one tall column beside a short one.
    /// Owner: kit
    #[test]
    fn cards_of_different_heights_divide_where_the_columns_come_out_closest() {
        assert_eq!(balanced_split(&[300.0, 100.0, 100.0, 100.0]), 1);
        assert_eq!(balanced_split(&[100.0, 100.0]), 1);
        assert_eq!(balanced_split(&[100.0, 100.0, 100.0, 100.0]), 2);
    }

    fn key_row<'s>(n: u8) -> SessionRow<'s, P> {
        SessionRow {
            icon: faraday_atoms::Icon::SEARCH.into(),
            head: Shown::plain("Key"),
            sub: Shown::plain("line"),
            trailing: Shown::plain(""),
            accent: false,
            press: P(n),
            button: None,
            wrap: false,
            selected: false,
            open: None,
        }
    }

    fn key_presses(height: u16) -> usize {
        let mut m = page(vec![Section {
            label: None,
            body: Body::Box(RowsBox {
                heading: Shown::plain("Loaded"),
                lead: None,
                rows: Vec::new(),
                keys: (10..20).map(key_row).collect(),
                fit: true,
                scroll: Anchor { scope: "box", n: 0 },
            }),
        }]);
        m.row = None;
        let mut c = faraday_atoms::ui::testing::canvas_sized(160, (480, height));
        let f = frame_for();
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.compact = true;
            draw_screen(
                Anchor { scope: "t", n: 1 },
                &f,
                ui,
                |ui| draw(&m, ui),
                &[] as &[(Anchor, ())],
                |_, _| {},
                None,
            );
        });
        out.hits.iter().filter(|h| h.press.0 >= 10).count()
    }

    /// Invariant: The box of what is loaded shows more key rows in a tall window than in a short one, and never every key it was given.
    /// Regression: The Pi's box of what is loaded running past the foot of the panel, or showing every key.
    /// Owner: kit
    #[test]
    fn the_box_shows_only_the_key_rows_the_room_holds() {
        let (tall, short) = (key_presses(1600), key_presses(250));
        assert!(tall > short, "{tall} {short}");
        assert!(tall < 10);
    }

    /// Invariant: The search row stands still above the scrolling sections in a tall window and scrolls with them in a short one: the scrolling region starts lower when it stands still.
    /// Regression: Home's search bar pinned on a window with no room left for results, or scrolling away on a tall one.
    /// Owner: kit
    #[test]
    fn the_search_row_pins_only_where_three_rows_fit_under_it() {
        let region_top = |height: u16| {
            let mut m = page(vec![Section {
                label: None,
                body: Body::Tiles(vec![face("One", 1)]),
            }]);
            m.row = Some(crate::screen::BarSearch {
                field: faraday_organisms::nav::SearchBarModel {
                    typed: Shown::plain(""),
                    hint: Shown::plain("Search"),
                    focused: false,
                    here: P(50),
                    clear: (P(51), Shown::plain("Clear")),
                    raise: None,
                },
                ways: None,
            });
            let mut c = faraday_atoms::ui::testing::canvas_sized(160, (480, height));
            let f = frame_for();
            let out = frame(&mut c, &at_rest(None), |ui| {
                ui.compact = true;
                draw_screen(
                    Anchor { scope: "t", n: 1 },
                    &f,
                    ui,
                    |ui| draw(&m, ui),
                    &[] as &[(Anchor, ())],
                    |_, _| {},
                    None,
                );
            });
            out.regions[0].view.y
        };
        assert!(region_top(1600) > region_top(300));
    }
}
