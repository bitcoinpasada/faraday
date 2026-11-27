//! The session pane organism (09 §3.3): what the session holds, drawn as This session's card
//! (`docs/SESSION-PANE.md`). It draws names, fingerprints and marks the model gives it, in
//! groups (Keys, Wallets, the Waiting band at the foot), as rows. It holds no key material and
//! keeps no copy of a model's text between frames.
//!
//! The model is data: which rows the session has, which one is open, what a vault holds of each
//! (the mark at a row's end and Save where one is offered) is worked out by the feature that
//! fills it. This file is the geometry and the drawing: the card with its groups in columns, the
//! pane in a menu page's mode (one list in the pane's width), the Waiting band, the scan in the
//! pane's place and the pick's rows, which a flow's first step draws through [`draw_pick`].
//!
//! Not built: the small panel's tinted box of rows (the menu page's compact form).

use faraday_atoms::button::Look;
use faraday_atoms::hits::Shape;
use faraday_atoms::region::Region;
use faraday_atoms::{Anchor, Glyph, Icon, Pressed, Shown, Ui};
use faraday_molecules::buttons::{self, ButtonsModel};
use faraday_tokens::dims::{
    BODY_L, BODY_M, BUTTON_H, DISABLED_CONTENT, ELEVATION_SHEET, GAP, GAP_SMALL, GRID,
    HOME_BOX_MIN_ROWS, HOME_CARD_GAP, HOME_CARD_HEAD, ICON_GLYPH, ICON_GLYPH_BUTTON, ICON_SMALL,
    LABEL_L, LIST_1, LIST_2, LIST_PAD, LIST_PAD_V, MARGIN_WIDE, MENU_PAGE_W, NAV_ICON, PAD,
    PICK_RADIO_DOT_INSET, ROW_CHEVRON_ROOM, SCAN_PANE_ASPECT, SCAN_PANE_MIN_H, SCAN_PANE_RADIUS,
    SESSION_BAND_SHARE, SESSION_COL_MIN, SHAPE_M, SHAPE_S, SPACER, TITLE_M, TITLE_S,
};
use faraday_tokens::palette::{Paint, Role};

use crate::content::{self, Content};
use crate::viewfinder::{self, ViewfinderModel};

/// One row of the pane: its icon, its headline, the line under it, the small text at its end
/// (a vault's mark; `accent` draws it as an action), what a press does, and either a chevron or
/// one button under the lines. `open` is what a press on a row with a button opens (its
/// details); the button stays its own press.
pub struct SessionRow<'s, A> {
    pub icon: Glyph,
    pub head: Shown<'s>,
    /// The line under the headline; empty for none.
    pub sub: Shown<'s>,
    /// The small text at the row's end; empty for none.
    pub trailing: Shown<'s>,
    pub accent: bool,
    pub press: A,
    /// The button under the row's lines at the text's start: its label and whether it can be
    /// pressed (a vault's Save).
    pub button: Option<(Shown<'s>, bool)>,
    /// The second line takes as many lines as it needs.
    pub wrap: bool,
    /// The row whose details stand open in the page's second pane.
    pub selected: bool,
    pub open: Option<A>,
}

impl<A> SessionRow<'_, A> {
    /// The row ends in a chevron: a press of it opens something, a button's row too where its
    /// press beyond the button opens details.
    fn chevron(&self) -> bool {
        self.button.is_none() || self.open.is_some()
    }
}

/// A group of rows under a small label. The band is the pane's last group, drawn across the
/// foot of the card outside the part that scrolls.
pub struct Group<'s, A> {
    pub label: Shown<'s>,
    pub rows: Vec<SessionRow<'s, A>>,
    pub band: bool,
}

/// What the scan in the pane's place holds: a title, then the camera's picture with its ways
/// under it, or the card of what was read with its ways.
pub struct ScanModel<'s, A> {
    pub title: Shown<'s>,
    pub body: ScanBody<'s, A>,
}

/// The two states of a scan in the pane.
pub enum ScanBody<'s, A> {
    Camera(Box<CameraModel<'s, A>>),
    Read(Box<ReadModel<'s, A>>),
}

/// The camera's picture and the buttons under it (Paste, Cancel).
pub struct CameraModel<'s, A> {
    pub finder: ViewfinderModel<'s, A>,
    pub ways: ButtonsModel<'s, A>,
}

/// What was read: its card, then the ways to use it and the buttons that leave the scan.
pub struct ReadModel<'s, A> {
    pub card: Content<'s, A>,
    pub ways: ButtonsModel<'s, A>,
    pub leaves: ButtonsModel<'s, A>,
}

/// The pane's model: its heading, the groups the session holds and, while nothing is held, the
/// line that says so.
pub struct SessionPaneModel<'s, A> {
    /// The card's heading.
    pub heading: Shown<'s>,
    /// The line shown while the session holds nothing.
    pub none: Shown<'s>,
    /// The search stands in the top bar: the card starts where a page's content does, with no
    /// gap over it.
    pub in_bar: bool,
    pub groups: Vec<Group<'s, A>>,
    /// A scan stands in the pane's place.
    pub scan: Option<ScanModel<'s, A>>,
    /// The scroll of the groups, and of the band at the foot.
    pub scroll: Anchor,
    pub band_scroll: Anchor,
}

impl<'s, A> SessionPaneModel<'s, A> {
    /// The pane with nothing held.
    pub fn empty(heading: Shown<'s>, none: Shown<'s>, in_bar: bool) -> Self {
        Self {
            heading,
            none,
            in_bar,
            groups: Vec::new(),
            scan: None,
            scroll: Anchor {
                scope: "session-pane",
                n: 0,
            },
            band_scroll: Anchor {
                scope: "session-pane",
                n: 1,
            },
        }
    }

    /// The same pane holding `groups`.
    pub fn holding(mut self, groups: Vec<Group<'s, A>>) -> Self {
        self.groups = groups;
        self
    }

    fn gap(&self) -> f32 {
        if self.in_bar { 0.0 } else { HOME_CARD_GAP }
    }
}

// ---- rows ----

/// Where a row's text starts, from the row's left edge.
fn text_dx() -> f32 {
    LIST_PAD + ICON_SMALL + LIST_PAD
}

/// The width a row's lines have in a row `w` wide, the chevron and the small text at its end
/// taken off.
fn text_room<A: Pressed>(ui: &Ui<'_, A>, w: f32, r: &SessionRow<'_, A>) -> f32 {
    let mut right = w - LIST_PAD;
    if r.chevron() {
        right -= ICON_SMALL + GRID;
    }
    if !r.trailing.as_str().is_empty() {
        right -= ui.measure_as(BODY_M, r.trailing.as_str()) + GRID;
    }
    (right - text_dx()).max(0.0)
}

fn sub_lines<A: Pressed>(ui: &Ui<'_, A>, w: f32, r: &SessionRow<'_, A>) -> usize {
    if r.sub.as_str().is_empty() {
        0
    } else if r.wrap {
        ui.wrap_lines_as(BODY_M, r.sub.as_str(), text_room(ui, w, r))
    } else {
        1
    }
}

fn row_h<A: Pressed>(ui: &Ui<'_, A>, w: f32, r: &SessionRow<'_, A>) -> f32 {
    let base = match sub_lines(ui, w, r) {
        0 => LIST_1,
        n => LIST_2 + (n - 1) as f32 * ui.line_as(BODY_M),
    };
    base + r.button.as_ref().map_or(0.0, |_| GRID + BUTTON_H)
}

fn rows_h<'a, 's: 'a, A: Pressed + 'a>(
    ui: &Ui<'_, A>,
    w: f32,
    rows: impl IntoIterator<Item = &'a SessionRow<'s, A>>,
) -> f32 {
    rows.into_iter().map(|r| row_h(ui, w, r)).sum::<f32>() + 2.0 * GAP_SMALL
}

/// `rows` from the top of a list at `(x, y)`, `w` wide: the rows alone.
fn list_rows<'a, 's: 'a, A: Pressed + 'a>(
    ui: &mut Ui<'_, A>,
    (x, y): (f32, f32),
    w: f32,
    rows: impl IntoIterator<Item = &'a SessionRow<'s, A>>,
) {
    let mut cy = y + GAP_SMALL;
    let text_x = x + text_dx();
    let chevron_x = x + w - LIST_PAD - ICON_SMALL;
    for r in rows {
        let rh = row_h(ui, w, r);
        if r.selected {
            // Material 3's selected list row.
            ui.fill(
                x + GAP_SMALL,
                cy,
                w - 2.0 * GAP_SMALL,
                rh,
                SHAPE_M,
                Role::SecondaryContainer,
            );
        }
        // A row with a button is not a press, but for what opens its details: the button is,
        // drawn after it and so on top.
        let pressed = if r.button.is_none() {
            Some(r.press.clone())
        } else {
            r.open.clone()
        };
        if let Some(press) = pressed {
            ui.press(
                Shape::new(x + GAP_SMALL, cy, w - 2.0 * GAP_SMALL, rh, SHAPE_M),
                Role::OnSurface.paint(),
                press,
            );
        }
        draw_row(ui, (x, cy, w, rh), (text_x, chevron_x), r);
        cy += rh;
    }
}

/// One row, `rh` high at `(x, cy)`: icon, lines, then the chevron and the small text at its
/// end, or the one button under the lines.
fn draw_row<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, cy, w, rh): (f32, f32, f32, f32),
    (text_x, chevron_x): (f32, f32),
    r: &SessionRow<'_, A>,
) {
    let lines = sub_lines(ui, w, r);
    let line = ui.line_as(BODY_L);
    let sub_line = ui.line_as(BODY_M);
    let content = line + lines as f32 * sub_line;
    let boxed = if r.button.is_some() {
        rh - GRID - BUTTON_H
    } else {
        rh
    };
    let top = cy + (boxed - content) / 2.0;
    let tone = if r.icon == Glyph::Icon(Icon::Warning) {
        Role::Warning
    } else {
        Role::OnSurfaceVariant
    };
    ui.icon(
        x + LIST_PAD,
        if r.wrap {
            // On the headline's own line: its centre, not the line's top.
            top + (line - ICON_SMALL) / 2.0
        } else {
            cy + (rh - ICON_SMALL) / 2.0
        },
        ICON_SMALL,
        r.icon,
        NAV_ICON + GAP_SMALL,
        tone,
    );
    let mut right = x + w - LIST_PAD;
    if r.chevron() {
        ui.icon(
            chevron_x,
            cy + (rh - ICON_SMALL) / 2.0,
            ICON_SMALL,
            Icon::ChevronRight,
            NAV_ICON + GAP_SMALL,
            Role::OnSurfaceVariant,
        );
        right = chevron_x - GRID;
    }
    if !r.trailing.as_str().is_empty() {
        let tone = if r.accent {
            Role::Primary
        } else {
            Role::OnSurfaceVariant
        };
        let tw = ui.measure_as(BODY_M, r.trailing.as_str());
        ui.showing(&r.trailing, |ui| {
            ui.text_right_as((right, cy, rh), BODY_M, tone, r.trailing.as_str());
        });
        right -= tw + GRID;
    }
    let room = (right - text_x).max(0.0);
    let head = ui.fit_as(BODY_L, r.head.as_str(), room);
    ui.showing(&r.head, |ui| {
        ui.text_mid_as((text_x, top, line), BODY_L, Role::OnSurface, &head);
    });
    if lines > 0 {
        ui.showing(&r.sub, |ui| {
            if r.wrap {
                ui.wrap_as(
                    (text_x, top + line, room),
                    BODY_M,
                    Role::OnSurfaceVariant,
                    r.sub.as_str(),
                );
            } else {
                let sub = ui.fit_as(BODY_M, r.sub.as_str(), room);
                ui.text_mid_as(
                    (text_x, top + line, sub_line),
                    BODY_M,
                    Role::OnSurfaceVariant,
                    &sub,
                );
            }
        });
    }
    if let Some((label, enabled)) = &r.button {
        let look = if *enabled { Look::Tonal } else { Look::TonalOff };
        ui.button(
            text_x,
            top + content + GRID,
            None,
            BUTTON_H,
            label.as_str(),
            look,
            r.press.clone(),
        );
    }
}

fn section_label<A: Pressed>(ui: &mut Ui<'_, A>, x: f32, y: f32, label: &Shown<'_>) {
    ui.showing(label, |ui| {
        ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, label.as_str());
    });
}

/// The card's surface: raised at level 1, an outline in its place at High contrast.
fn card_surface<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.shadow(x, y, w, h, SHAPE_M, ELEVATION_SHEET);
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, w, h, SHAPE_M, Role::Outline);
    }
}

// ---- columns ----

fn column_count(inner: f32) -> usize {
    (((inner + SPACER) / (SESSION_COL_MIN + SPACER)).floor() as usize).max(1)
}

fn column_w(inner: f32, n: usize) -> f32 {
    (inner - (n - 1) as f32 * SPACER) / n as f32
}

/// How the groups stand in the card: the groups of each column in order, `n` columns of at
/// least `SESSION_COL_MIN` across `inner`, each group whole and the next into the shortest
/// column. Returns the columns' groups, their width and their heights.
fn columns<A: Pressed>(
    ui: &Ui<'_, A>,
    inner: f32,
    groups: &[Group<'_, A>],
) -> (Vec<Vec<usize>>, f32, Vec<f32>) {
    let n = column_count(inner).min(groups.len().max(1));
    let col_w = column_w(inner, n);
    let label_h = ui.line_as(TITLE_S) + GRID;
    let mut cols: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut heights = vec![0.0_f32; n];
    for (i, g) in groups.iter().enumerate() {
        let c = (0..n)
            .min_by(|&a, &b| heights[a].total_cmp(&heights[b]))
            .unwrap_or(0);
        if !cols[c].is_empty() {
            heights[c] += SPACER;
        }
        heights[c] += label_h + rows_h(ui, col_w, &g.rows);
        cols[c].push(i);
    }
    (cols, col_w, heights)
}

// ---- the band ----

/// The groups with the band split off: the band is the last group, when there is one.
fn split<'g, 's, A>(groups: &'g [Group<'s, A>]) -> (&'g [Group<'s, A>], Option<&'g Group<'s, A>>) {
    match groups.split_last() {
        Some((last, rest)) if last.band => (rest, Some(last)),
        _ => (groups, None),
    }
}

/// How the band's rows stand: the rows of each column in order, the columns' width and the
/// height the rows take whole, label and padding included.
struct BandLayout {
    cols: Vec<Vec<usize>>,
    col_w: f32,
    need: f32,
}

fn band_layout<A: Pressed>(ui: &Ui<'_, A>, w: f32, g: &Group<'_, A>) -> BandLayout {
    let inner = w - 2.0 * PAD;
    let n = column_count(inner).min(g.rows.len()).max(1);
    let col_w = column_w(inner, n);
    let mut cols: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut heights = vec![0.0_f32; n];
    for (i, r) in g.rows.iter().enumerate() {
        let c = (0..n)
            .min_by(|&a, &b| heights[a].total_cmp(&heights[b]))
            .unwrap_or(0);
        heights[c] += row_h(ui, col_w, r);
        cols[c].push(i);
    }
    let tallest = heights.iter().copied().fold(0.0, f32::max);
    let need = GRID + ui.line_as(TITLE_S) + GRID + tallest + 2.0 * GAP_SMALL;
    BandLayout { cols, col_w, need }
}

/// The height the band takes in a pane `pane_h` tall: all it needs, or a third of the pane, but
/// never less than its label and one row.
fn band_shown_h<A: Pressed>(ui: &Ui<'_, A>, need: f32, pane_h: f32) -> f32 {
    let least = GRID + ui.line_as(TITLE_S) + GRID + LIST_2 + 2.0 * GAP_SMALL;
    need.min((pane_h * SESSION_BAND_SHARE).max(least))
}

/// The band at `(x, y)`, `w` wide and `h` tall: the hairline across the card, the heading, then
/// the rows in columns, scrolling within where `h` cuts them.
fn draw_band<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
    scroll: Anchor,
    g: &Group<'_, A>,
    lay: &BandLayout,
) {
    ui.rule(x, y, w, Role::OutlineVariant);
    let top = y + GRID;
    section_label(ui, x + PAD + LIST_PAD, top, &g.label);
    let rows_y = top + ui.line_as(TITLE_S) + GRID;
    let view = (y + h - rows_y).max(0.0);
    let clip = ui.rect(x, rows_y, w, view);
    let mark = ui.presses_mark();
    let open = ui.region_open(Region::new(scroll, clip));
    let mut colx = x + PAD;
    let mut tallest = 0.0_f32;
    for c in &lay.cols {
        let rows = || c.iter().map(|&i| &g.rows[i]);
        list_rows(ui, (colx, rows_y), lay.col_w, rows());
        tallest = tallest.max(rows_h(ui, lay.col_w, rows()));
        colx += lay.col_w + SPACER;
    }
    ui.clip_presses_since(mark, clip);
    ui.region_close(open, (tallest - view).max(0.0));
}

// ---- the card ----

/// The card as measured: where its parts stand, read once for the height and for the drawing.
struct Card<'g, 's, A> {
    gap: f32,
    rest: &'g [Group<'s, A>],
    cols: Vec<Vec<usize>>,
    col_w: f32,
    band: Option<(&'g Group<'s, A>, BandLayout, f32)>,
    head: f32,
    body: f32,
    tallest: f32,
    scrolls: bool,
    card_h: f32,
    cw: f32,
}

/// Measures the card in `w`, `y` from the top of a window `window` tall (none: as tall as its
/// groups): its heading, the groups in as many columns as fit, the band at its foot.
fn card_layout<'g, 's, A: Pressed>(
    m: &'g SessionPaneModel<'s, A>,
    ui: &Ui<'_, A>,
    (y, w): (f32, f32),
    window: Option<f32>,
) -> Card<'g, 's, A> {
    let cw = w.min(MENU_PAGE_W);
    let gap = m.gap();
    let head = PAD + ui.line_as(HOME_CARD_HEAD) + GRID;
    let (rest, band_group) = split(&m.groups);
    let (cols, col_w, heights) = columns(ui, cw - 2.0 * PAD, rest);
    let tallest = heights.iter().copied().fold(0.0, f32::max);
    let tail = if band_group.is_some() { 0.0 } else { GRID };
    let band_lay = band_group.map(|g| band_layout(ui, cw, g));
    let need = band_lay.as_ref().map_or(0.0, |l| l.need);
    let none_h = ui.line_as(BODY_L) + PAD;
    let content = if m.groups.is_empty() { none_h } else { tallest };
    let natural = head + content + need + tail;
    let room = window.map(|h| (h - (y + gap) - MARGIN_WIDE).floor());
    let pane_h = room.map_or(natural, |r| r.min(natural));
    // The band is held to its share only where the card does not stand whole.
    let shown = match room {
        Some(r) if natural > r => band_shown_h(ui, need, r),
        _ => need,
    };
    let avail = pane_h - head - shown - tail;
    let scrolls = !rest.is_empty() && tallest > avail && avail >= HOME_BOX_MIN_ROWS * LIST_2;
    let body = if scrolls { avail } else { content };
    let band_h = shown.min(need);
    Card {
        gap,
        rest,
        cols,
        col_w,
        band: band_group.zip(band_lay).map(|(g, lay)| (g, lay, band_h)),
        head,
        body,
        tallest,
        scrolls,
        card_h: head + body + band_h + tail,
        cw,
    }
}

/// The height the card takes in `w`, with the gap over it: what a page stacks the next thing
/// under. A scan in the pane's place takes the room of its own content.
pub fn height<A: Pressed>(m: &SessionPaneModel<'_, A>, ui: &mut Ui<'_, A>, w: f32) -> f32 {
    if let Some(scan) = &m.scan {
        return m.gap() + ui.measured(|ui| scan_content(scan, ui, (0.0, 0.0, w.min(MENU_PAGE_W)), f32::INFINITY));
    }
    let c = card_layout(m, ui, (0.0, w), None);
    c.gap + c.card_h
}

/// Draws the card whole from (x, y) in `w`: at most `MENU_PAGE_W` wide and centred. Returns its
/// height with the gap.
pub fn draw<A: Pressed>(
    m: &SessionPaneModel<'_, A>,
    ui: &mut Ui<'_, A>,
    at: (f32, f32, f32),
) -> f32 {
    draw_in(m, ui, at, None)
}

/// [`draw`] in a window `window` tall: the card scrolls within itself, as one, the band and the
/// heading staying, where the window is short.
pub fn draw_in<A: Pressed>(
    m: &SessionPaneModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    window: Option<f32>,
) -> f32 {
    if let Some(scan) = &m.scan {
        return m.gap() + scan_block(scan, ui, (x, y + m.gap(), w.min(MENU_PAGE_W)), f32::INFINITY);
    }
    let c = card_layout(m, ui, (y, w), window);
    let cx = x + (w - c.cw) / 2.0;
    let top = y + c.gap;
    let rows_y = top + c.head;
    card_surface(ui, (cx, top, c.cw, c.card_h));
    ui.showing(&m.heading, |ui| {
        ui.text_as(
            cx + PAD + LIST_PAD,
            top + PAD,
            HOME_CARD_HEAD,
            Role::OnSurface,
            m.heading.as_str(),
        );
    });
    if m.groups.is_empty() {
        ui.text_as(
            cx + PAD + LIST_PAD,
            rows_y,
            BODY_L,
            Role::OnSurfaceVariant,
            m.none.as_str(),
        );
        return c.gap + c.card_h;
    }
    let draw_cols = |ui: &mut Ui<'_, A>| {
        let mut colx = cx + PAD;
        for col in &c.cols {
            let mut cy = rows_y;
            for &gi in col {
                let g = &c.rest[gi];
                section_label(ui, colx + LIST_PAD, cy, &g.label);
                cy += ui.line_as(TITLE_S) + GRID;
                list_rows(ui, (colx, cy), c.col_w, &g.rows);
                cy += rows_h(ui, c.col_w, &g.rows) + SPACER;
            }
            colx += c.col_w + SPACER;
        }
    };
    if c.scrolls {
        let clip = ui.rect(cx, rows_y, c.cw, c.body);
        let mark = ui.presses_mark();
        let open = ui.region_open(Region::new(m.scroll, clip));
        draw_cols(ui);
        ui.clip_presses_since(mark, clip);
        ui.region_close(open, (c.tallest - c.body).max(0.0));
    } else {
        draw_cols(ui);
    }
    if let Some((g, lay, shown)) = &c.band {
        draw_band(ui, (cx, rows_y + c.body, c.cw, *shown), m.band_scroll, g, lay);
    }
    c.gap + c.card_h
}

// ---- the pane in a menu page's mode ----

/// The groups in the pane's width as one list: each group's small heading then its rows, whole.
fn page_groups_h<A: Pressed>(m: &SessionPaneModel<'_, A>, ui: &Ui<'_, A>, w: f32) -> f32 {
    let label_h = ui.line_as(TITLE_S) + GRID;
    split(&m.groups)
        .0
        .iter()
        .map(|g| label_h + rows_h(ui, w - 2.0 * PAD, &g.rows))
        .sum()
}

/// The height the pane in a page's mode needs, `w` wide, to show everything it holds: the
/// level of a pair is the longer of this and the tile's.
pub fn page_content_h<A: Pressed>(
    m: &SessionPaneModel<'_, A>,
    ui: &mut Ui<'_, A>,
    w: f32,
) -> f32 {
    if let Some(scan) = &m.scan {
        return ui.measured(|ui| scan_content(scan, ui, (0.0, 0.0, w), f32::INFINITY));
    }
    let head = PAD + ui.line_as(TITLE_M) + GRID;
    let (_, band) = split(&m.groups);
    let band_h = band.map_or(0.0, |g| band_layout(ui, w, g).need);
    let none_h = if m.groups.is_empty() {
        ui.line_as(BODY_L)
    } else {
        0.0
    };
    head + page_groups_h(m, ui, w) + none_h + band_h + PAD / 2.0
}

/// The pane in a page's mode in the box `(x, y, w, h)`: the heading, the groups scrolling
/// within where `h` cuts them, and the band at its foot with its own scroll. A scan stands in
/// its place, a read card taller than the pane scrolling as the groups do.
pub fn draw_page<A: Pressed>(
    m: &SessionPaneModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
) {
    card_surface(ui, (x, y, w, h));
    if let Some(scan) = &m.scan {
        let clip = ui.rect(x, y, w, h);
        let mark = ui.presses_mark();
        let open = ui.region_open(Region::new(m.scroll, clip));
        let used = scan_content(scan, ui, (x, y, w), h);
        ui.clip_presses_since(mark, clip);
        ui.region_close(open, (used - h).max(0.0));
        return;
    }
    let head_h = ui.line_as(TITLE_M) + GRID;
    ui.showing(&m.heading, |ui| {
        ui.text_as(
            x + PAD + LIST_PAD,
            y + PAD,
            TITLE_M,
            Role::OnSurface,
            m.heading.as_str(),
        );
    });
    let top = y + PAD + head_h;
    let (rest, band) = split(&m.groups);
    let band = band.map(|g| (g, band_layout(ui, w, g)));
    let shown = band
        .as_ref()
        .map_or(0.0, |(_, lay)| band_shown_h(ui, lay.need, h).min(lay.need));
    let view = if band.is_some() {
        (h - (top - y) - shown).max(0.0)
    } else {
        (h - (top - y) - PAD / 2.0).max(0.0)
    };
    let clip = ui.rect(x, top, w, view);
    let total = page_groups_h(m, ui, w);
    let mark = ui.presses_mark();
    let open = ui.region_open(Region::new(m.scroll, clip));
    if m.groups.is_empty() {
        ui.text_as(
            x + PAD + LIST_PAD,
            top,
            BODY_L,
            Role::OnSurfaceVariant,
            m.none.as_str(),
        );
    }
    let mut cy = top;
    for g in rest {
        section_label(ui, x + PAD + LIST_PAD, cy, &g.label);
        cy += ui.line_as(TITLE_S) + GRID;
        list_rows(ui, (x + PAD, cy), w - 2.0 * PAD, &g.rows);
        cy += rows_h(ui, w - 2.0 * PAD, &g.rows);
    }
    ui.clip_presses_since(mark, clip);
    ui.region_close(open, (total - view).max(0.0));
    if let Some((g, lay)) = &band {
        draw_band(ui, (x, y + h - shown, w, shown), m.band_scroll, g, lay);
    }
}

// ---- the scan in the pane ----

/// What the pane holds while it scans, in a card `w` wide with at most `room` of height: the
/// title, the camera's picture with its ways under it, or the read card with its ways. Returns
/// the height it takes.
fn scan_content<A: Pressed>(
    m: &ScanModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    room: f32,
) -> f32 {
    let (cx, cw) = (x + PAD + LIST_PAD, w - 2.0 * (PAD + LIST_PAD));
    let mut cy = y + PAD;
    ui.showing(&m.title, |ui| {
        cy += ui.wrap_as((cx, cy, cw), BODY_L, Role::OnSurface, m.title.as_str()) + GAP;
    });
    match &m.body {
        ScanBody::Read(read) => {
            // The card's lines stand on the title's and the buttons' edge.
            let at = (x + LIST_PAD, cy - PAD, w - 2.0 * LIST_PAD);
            cy = at.1 + content::draw(&read.card, ui, at);
            if !read.ways.items.is_empty() {
                cy += buttons::draw(&read.ways, ui, (cx, cy, cw)) + GAP;
            }
            cy += buttons::draw(&read.leaves, ui, (cx, cy, cw));
        }
        ScanBody::Camera(cam) => {
            let under = ui.button_height(BUTTON_H) + GAP + PAD;
            let h = (cw * SCAN_PANE_ASPECT).min((room - (cy - y) - under).max(SCAN_PANE_MIN_H));
            let fw = h / SCAN_PANE_ASPECT;
            let fx = cx + (cw - fw) / 2.0;
            ui.fill(
                fx,
                cy,
                fw,
                h,
                SCAN_PANE_RADIUS,
                Role::SurfaceContainerHighest,
            );
            viewfinder::draw(&cam.finder, ui, (fx, cy, fw, h));
            cy += h + GAP;
            cy += buttons::draw(&cam.ways, ui, (cx, cy, cw));
        }
    }
    cy + PAD - y
}

/// The card where the scan stands among a menu page's sections or over a stacked page, as tall
/// as what it holds, its picture sized to at most `room` of height. Returns that height.
pub fn scan_block<A: Pressed>(
    m: &ScanModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    room: f32,
) -> f32 {
    let h = ui.measured(|ui| scan_content(m, ui, (x, y, w), room));
    card_surface(ui, (x, y, w, h));
    scan_content(m, ui, (x, y, w), room);
    h
}

// ---- the pick ----

/// What a pick row is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickKind {
    /// Something loaded the flow takes: a radio, ticked when `chosen`.
    Pick { chosen: bool },
    /// Something loaded a step that takes several uses: a checkbox.
    Tick { ticked: bool },
    /// A way to get what the flow takes (Add a key): no radio.
    WayIn,
    /// An item whose press leads on from it rather than chooses it: a chevron at the end.
    Opens,
}

/// One row of a pick: a Material single-choice list row, the chosen one in the secondary
/// container. A dimmed row stays, keeps its reason as its line and takes no press.
pub struct PickRow<'s, A> {
    pub icon: Icon,
    pub head: Shown<'s>,
    /// The second line; the reason when the row is dimmed.
    pub line: Shown<'s>,
    pub kind: PickKind,
    pub dimmed: bool,
    /// The second line is drawn in the error role.
    pub error: bool,
    pub press: A,
}

/// A group of pick rows under its heading.
pub struct PickGroup<'s, A> {
    pub head: Shown<'s>,
    pub rows: Vec<PickRow<'s, A>>,
}

fn pick_line_role(error: bool, on: bool, dimmed: bool) -> Paint {
    if dimmed {
        return Role::OnSurface.at(DISABLED_CONTENT);
    }
    match (error, on) {
        (true, true) => Role::OnErrorContainer.paint(),
        (true, false) => Role::Error.paint(),
        (_, true) => Role::OnSecondaryContainer.paint(),
        _ => Role::OnSurfaceVariant.paint(),
    }
}

/// Draws a pick's groups from `y`, each under its heading, one under another: the pane's and
/// the narrow card's alike. Returns the height used.
pub fn draw_pick<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    groups: &[PickGroup<'_, A>],
) -> f32 {
    let mut cy = y;
    for g in groups {
        let h = ui.advance_as(LABEL_L, ICON_GLYPH + GAP);
        let text = ui.fit_as(LABEL_L, g.head.as_str(), w);
        ui.showing(&g.head, |ui| {
            ui.text_mid_as((x, cy, h), LABEL_L, Role::OnSurfaceVariant, &text);
        });
        cy += h;
        for r in &g.rows {
            cy += pick_row(ui, (x, cy, w), r) + GAP_SMALL;
        }
    }
    cy - y
}

fn pick_row<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32), r: &PickRow<'_, A>) -> f32 {
    let two = !r.line.as_str().is_empty();
    let radio = if matches!(r.kind, PickKind::Pick { .. } | PickKind::Tick { .. }) {
        ICON_GLYPH_BUTTON + GAP
    } else {
        0.0
    };
    let opens = r.kind == PickKind::Opens;
    let chevron = if opens { ROW_CHEVRON_ROOM } else { 0.0 };
    let room = w - LIST_PAD - chevron - (LIST_PAD + radio + ICON_GLYPH + GAP);
    // Both lines wrap, so a long head or verdict is read whole; each is broken once and drawn
    // from that.
    let head_laid = ui.wrapped_as(BODY_L, r.head.as_str(), room);
    let line_laid = two.then(|| ui.wrapped_as(BODY_M, r.line.as_str(), room));
    let (lh, vh) = (ui.line_as(BODY_L), ui.line_as(BODY_M));
    let head_lines = (head_laid.h / lh).round() as usize;
    let line_lines = line_laid
        .as_ref()
        .map_or(0, |l| (l.h / vh).round() as usize);
    let lines = head_laid.h + line_laid.as_ref().map_or(0.0, |l| l.h);
    let mut h = ui.row_height(
        if two { LIST_2 } else { LIST_1 },
        if two { lh + vh } else { lh },
        LIST_PAD_V,
    );
    if head_lines + line_lines > if two { 2 } else { 1 } {
        h = h.max(lines + 2.0 * LIST_PAD_V);
    }
    let on = matches!(
        r.kind,
        PickKind::Pick { chosen: true } | PickKind::Tick { ticked: true }
    );
    let ink = |role: Role| -> Paint {
        if r.dimmed {
            Role::OnSurface.at(DISABLED_CONTENT)
        } else if on {
            Role::OnSecondaryContainer.paint()
        } else {
            role.paint()
        }
    };
    let ground = if on {
        Role::SecondaryContainer
    } else {
        Role::SurfaceContainerHighest
    };
    ui.fill(x, y, w, h, SHAPE_S, ground);
    let mut tx = x + LIST_PAD;
    if let PickKind::Pick { chosen } = r.kind {
        let d = ICON_GLYPH_BUTTON / 2.0;
        let (cx, cy) = (tx + d, y + h / 2.0);
        ui.dot(cx, cy, d, ink(Role::OnSurfaceVariant));
        ui.dot(cx, cy, d - 2.0, ground);
        if chosen {
            ui.dot(cx, cy, d - PICK_RADIO_DOT_INSET, ink(Role::OnSurface));
        }
        tx += ICON_GLYPH_BUTTON + GAP;
    }
    if let PickKind::Tick { ticked } = r.kind {
        ui.checkbox(tx, y + (h - ICON_GLYPH_BUTTON) / 2.0, ticked, !r.dimmed);
        tx += ICON_GLYPH_BUTTON + GAP;
    }
    ui.icon(
        tx,
        y + (h - ICON_GLYPH) / 2.0,
        ICON_GLYPH,
        r.icon,
        ICON_GLYPH,
        ink(Role::OnSurfaceVariant),
    );
    tx += ICON_GLYPH + GAP;
    if opens {
        ui.row_chevron(x + w - LIST_PAD, y, h, ink(Role::OnSurfaceVariant));
    }
    let top = y + (h - lines) / 2.0;
    ui.showing(&r.head, |ui| {
        ui.draw_wrapped((tx, top), ink(Role::OnSurface), &head_laid);
    });
    if let Some(laid) = &line_laid {
        let tone = pick_line_role(r.error, on, r.dimmed);
        ui.showing(&r.line, |ui| {
            ui.draw_wrapped((tx, top + head_laid.h), tone, laid);
        });
    }
    // A dimmed row takes no press: nothing is registered for it.
    if !r.dimmed {
        ui.press_box(x, y, w, h, r.press.clone());
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame};

    fn row(head: &'static str, n: u8) -> SessionRow<'static, P> {
        SessionRow {
            icon: Glyph::Icon(Icon::Keys),
            head: Shown::plain(head),
            sub: Shown::plain("a line"),
            trailing: Shown::plain(""),
            accent: false,
            press: P(n),
            button: None,
            wrap: false,
            selected: false,
            open: None,
        }
    }

    fn group(label: &'static str, rows: Vec<SessionRow<'static, P>>, band: bool) -> Group<'static, P> {
        Group {
            label: Shown::plain(label),
            rows,
            band,
        }
    }

    fn pane(groups: Vec<Group<'static, P>>) -> SessionPaneModel<'static, P> {
        SessionPaneModel::empty(Shown::plain("This session"), Shown::plain("Nothing loaded"), false)
            .holding(groups)
    }

    /// Invariant: With nothing held the card is centred in the room, at most the menu page's width, and the height it reports is the height it drew (a page stacks under it by it).
    /// Regression: The next section drawn over the card, or the card off the page's column.
    /// Owner: kit
    #[test]
    fn the_empty_card_reports_the_height_it_takes() {
        let mut c = canvas(160);
        let mut drawn = 0.0;
        let mut said = 0.0;
        frame(&mut c, &at_rest(None), |ui| {
            let m = SessionPaneModel::<P>::empty(
                Shown::plain("This session"),
                Shown::plain("Nothing loaded"),
                false,
            );
            said = height(&m, ui, 400.0);
            drawn = draw(&m, ui, (0.0, 0.0, 400.0));
        });
        assert_eq!(drawn, said);
        assert!(drawn > 0.0);
    }

    /// Invariant: A card holding keys, wallets and what waits reports the height it drew, each row of it takes its own press, and the band's row takes one too.
    /// Regression: A group's rows drawn but not pressable, or the band left out of the card's height so the page under it overlaps.
    /// Owner: kit
    #[test]
    fn a_card_with_groups_reports_its_height_and_each_row_takes_a_press() {
        let mut c = canvas(160);
        let (mut drawn, mut said) = (0.0, 0.0);
        let out = frame(&mut c, &at_rest(None), |ui| {
            let m = pane(vec![
                group("Keys", vec![row("Savings", 1), row("Spending", 2)], false),
                group("Wallets", vec![row("Joint", 3)], false),
                group("Waiting", vec![row("Spend", 4)], true),
            ]);
            said = height(&m, ui, 700.0);
            drawn = draw(&m, ui, (0.0, 0.0, 700.0));
        });
        assert_eq!(drawn, said);
        let empty = {
            let mut e = 0.0;
            frame(&mut c, &at_rest(None), |ui| {
                e = height(&pane(Vec::new()), ui, 700.0);
            });
            e
        };
        assert!(drawn > empty);
        assert_eq!(out.hits.len(), 4);
    }

    /// Invariant: A key's vault mark is drawn at its row's end, and a row with Save takes two presses, its button and what opens its details, never the row's own.
    /// Regression: Save pressed as the row (loading the key's page) or the mark lost.
    /// Owner: kit
    #[test]
    fn a_marked_row_with_save_takes_the_button_and_its_details() {
        let mut c = canvas(160);
        let out = frame(&mut c, &at_rest(None), |ui| {
            let mut r = row("Savings", 1);
            r.trailing = Shown::plain("Not saved");
            r.button = Some((Shown::plain("Save"), true));
            r.open = Some(P(9));
            let m = pane(vec![group("Keys", vec![r], false)]);
            draw(&m, ui, (0.0, 0.0, 700.0));
        });
        let mut pressed: Vec<u8> = out.hits.iter().map(|h| h.press.0).collect();
        pressed.sort_unstable();
        assert_eq!(pressed, vec![1, 9]);
    }

    /// Invariant: The pane in a page's mode needs the height it draws groups in, and a dimmed pick row takes no press while the others do.
    /// Regression: A pair levelled to a shorter side than the pane needs, or a row the step cannot take that still answers a press.
    /// Owner: kit
    #[test]
    fn a_dimmed_pick_row_takes_no_press() {
        let mut c = canvas(160);
        let pick = |head: &'static str, dimmed: bool, n: u8| PickRow {
            icon: Icon::Keys,
            head: Shown::plain(head),
            line: Shown::plain(if dimmed { "Not for this" } else { "" }),
            kind: PickKind::Pick { chosen: false },
            dimmed,
            error: false,
            press: P(n),
        };
        let mut used = 0.0;
        let out = frame(&mut c, &at_rest(None), |ui| {
            let g = PickGroup {
                head: Shown::plain("Keys"),
                rows: vec![pick("Savings", false, 1), pick("Old", true, 2)],
            };
            used = draw_pick(ui, (0.0, 0.0, 400.0), &[g]);
        });
        assert!(used > 0.0);
        let pressed: Vec<u8> = out.hits.iter().map(|h| h.press.0).collect();
        assert_eq!(pressed, vec![1]);
    }

    /// Invariant: The pane in a page's mode takes at least the height of its heading and its rows, more as groups are added.
    /// Regression: A pane that reports the same height whatever it holds, so a tile beside it is levelled to the wrong end.
    /// Owner: kit
    #[test]
    fn the_page_pane_needs_more_height_for_more_rows() {
        let mut c = canvas(160);
        let (mut few, mut many) = (0.0, 0.0);
        frame(&mut c, &at_rest(None), |ui| {
            few = page_content_h(&pane(vec![group("Keys", vec![row("A", 1)], false)]), ui, 400.0);
            many = page_content_h(
                &pane(vec![group("Keys", vec![row("A", 1), row("B", 2), row("C", 3)], false)]),
                ui,
                400.0,
            );
        });
        assert!(many > few);
    }
}
