//! The session pane (`docs/SESSION-PANE.md`): what the session holds, drawn
//! as rows in groups. Home's card is its wide form; the phone and the Pi
//! draw their box from the same module through their own rows. The rows are
//! data (`rows`), the geometry is here; no file outside this folder draws a
//! row of the session (`lint-components`).

mod rows;

use super::home::{CardLead, lead_card};
use super::menu_page::Section;
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Glyph, Region, Shape, Slot, Style, Ui};
use faraday_draw::tokens::{
    BODY_L, BODY_M, BUTTON_H, ELEVATION_FAB, GAP_SMALL, GRID, HOME_BOX_KEYS, HOME_BOX_MIN_ROWS,
    HOME_CARD_GAP, HOME_CARD_HEAD, ICON_SMALL, LIST_1, LIST_2, LIST_PAD, MARGIN_WIDE, MENU_PAGE_W,
    NAV_ICON, PAD, SESSION_COL_MIN, SHAPE_M, SPACER, TITLE_S,
};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::section_label;
use rows::{Group, compact_rows, groups};

/// One row of the pane: its leading icon, its headline, the line under it
/// (empty for none), the small text at its end (the box's, empty for none;
/// `accent` draws it as an action) and what a press opens. A row ends in a
/// chevron, or carries one button under its two lines at the text's start
/// instead, `(label, enabled)`, which does what a press does. `wrap` lets
/// the second line take as many lines as it needs.
#[derive(Clone)]
pub(crate) struct SessionRow {
    pub icon: Glyph,
    pub head: String,
    pub sub: String,
    pub trailing: String,
    pub accent: bool,
    pub action: Action,
    pub button: Option<(&'static str, bool)>,
    pub wrap: bool,
}

/// Where a row's text starts, from the row's left edge.
fn text_dx() -> f32 {
    LIST_PAD + ICON_SMALL + LIST_PAD
}

/// The width a row's lines have in a row `w` wide, the chevron and the
/// small text at its end taken off.
fn text_room(ui: &Ui, w: f32, r: &SessionRow) -> f32 {
    let mut right = w - LIST_PAD;
    if r.button.is_none() {
        right -= ICON_SMALL + GRID;
    }
    if !r.trailing.is_empty() {
        right -= ui.measure_as(BODY_M, &r.trailing) + GRID;
    }
    (right - text_dx()).max(0.0)
}

/// How many lines a row's second line takes.
fn sub_lines(ui: &Ui, w: f32, r: &SessionRow) -> usize {
    if r.sub.is_empty() {
        0
    } else if r.wrap {
        ui.wrap_lines_as(BODY_M, &r.sub, text_room(ui, w, r))
    } else {
        1
    }
}

/// The height of a row `w` wide.
fn row_h(ui: &Ui, w: f32, r: &SessionRow) -> f32 {
    let base = match sub_lines(ui, w, r) {
        0 => LIST_1,
        n => LIST_2 + (n - 1) as f32 * ui.line_as(BODY_M),
    };
    base + r.button.map_or(0.0, |_| GRID + BUTTON_H)
}

/// The height of a list of `rows`, each `w` wide.
fn rows_h(ui: &Ui, w: f32, rows: &[SessionRow]) -> f32 {
    rows.iter().map(|r| row_h(ui, w, r)).sum::<f32>() + 2.0 * GAP_SMALL
}

/// `rows` as one tinted, rounded box (the surface of the Pi's box of what
/// is loaded) one under another from `(x, y)`, `w` wide. Returns the box's
/// height.
fn list_box(ui: &mut Ui, (x, y): (f32, f32), w: f32, rows: &[SessionRow]) -> f32 {
    let h = rows_h(ui, w, rows);
    crate::nav::screen::loaded_surface(ui, x, y, w, h);
    list_rows(ui, (x, y), w, rows);
    h
}

/// How many of `keys` key rows (two lines each) the box shows: up to `cap`,
/// and only as many as `avail` holds besides the other rows' `rest_h`.
fn keys_fitting(avail: f32, rest_h: f32, keys: usize, cap: usize) -> usize {
    let room = ((avail - rest_h) / LIST_2).floor().max(0.0) as usize;
    keys.min(cap).min(room)
}

/// `rows` from the top of a list at `(x, y)`, `w` wide: the rows alone.
fn list_rows(ui: &mut Ui, (x, y): (f32, f32), w: f32, rows: &[SessionRow]) {
    let mut cy = y + GAP_SMALL;
    let text_x = x + text_dx();
    let chevron_x = x + w - LIST_PAD - ICON_SMALL;
    for r in rows {
        let rh = row_h(ui, w, r);
        // A row with a button is not a press: the button is.
        if r.button.is_none() {
            ui.press(
                Shape::new(x + GAP_SMALL, cy, w - 2.0 * GAP_SMALL, rh, SHAPE_M),
                Role::OnSurface.paint(),
                r.action,
            );
        }
        draw_row(ui, (x, cy, w, rh), (text_x, chevron_x), r);
        cy += rh;
    }
}

/// One row, `rh` high at `(x, cy)`: icon, lines, then the chevron and the
/// small text at its end, or the one button under the lines.
fn draw_row(
    ui: &mut Ui,
    (x, cy, w, rh): (f32, f32, f32, f32),
    (text_x, chevron_x): (f32, f32),
    r: &SessionRow,
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
            top
        } else {
            cy + (rh - ICON_SMALL) / 2.0
        },
        ICON_SMALL,
        r.icon,
        NAV_ICON + GAP_SMALL,
        tone,
    );
    let mut right = x + w - LIST_PAD;
    if r.button.is_none() {
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
    if !r.trailing.is_empty() {
        let tone = if r.accent {
            Role::Primary
        } else {
            Role::OnSurfaceVariant
        };
        let tw = ui.measure_as(BODY_M, &r.trailing);
        ui.text_right_as((right, cy, rh), BODY_M, tone, &r.trailing);
        right -= tw + GRID;
    }
    let room = (right - text_x).max(0.0);
    let head = ui.fit_as(BODY_L, &r.head, room);
    ui.text_mid_as((text_x, top, line), BODY_L, Role::OnSurface, &head);
    if lines > 0 {
        if r.wrap {
            ui.wrap_as(
                (text_x, top + line, room),
                BODY_M,
                Role::OnSurfaceVariant,
                &r.sub,
            );
        } else {
            let sub = ui.fit_as(BODY_M, &r.sub, room);
            ui.text_mid_as(
                (text_x, top + line, sub_line),
                BODY_M,
                Role::OnSurfaceVariant,
                &sub,
            );
        }
    }
    if let Some((label, enabled)) = r.button {
        let style = if enabled {
            Style::Tonal
        } else {
            Style::TonalOff
        };
        ui.button(
            text_x,
            top + content + GRID,
            None,
            BUTTON_H,
            label,
            style,
            r.action,
        );
    }
}

/// The card's surface: a raised card of the surface-container-low fill with
/// the Scan button's shadow.
fn card_surface(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.shadow(x, y, w, h, SHAPE_M, ELEVATION_FAB);
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
}

/// How the groups stand in the card: the groups of each column, in their
/// order, `n` columns of at least `SESSION_COL_MIN` across `inner`, `SPACER`
/// apart, each group whole and the next into the shortest column. Returns
/// the columns' groups, their width and their heights.
fn columns(ui: &Ui, inner: f32, groups: &[Group]) -> (Vec<Vec<usize>>, f32, Vec<f32>) {
    let fit = ((inner + SPACER) / (SESSION_COL_MIN + SPACER)).floor() as usize;
    let n = fit.clamp(1, groups.len().max(1));
    let col_w = (inner - (n - 1) as f32 * SPACER) / n as f32;
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

/// Home's card in the window `w` wide from `x`, `HOME_CARD_GAP` under what
/// is above it: its heading, then the groups in as many columns as fit. It
/// is at most `MENU_PAGE_W` wide and centred, as tall as its groups, and
/// scrolls within itself, as one, when the window is short. Nothing loaded:
/// one line. Returns its height with the gap.
fn pane_card(
    ui: &mut Ui,
    (x, y): (f32, f32),
    w: f32,
    (window, offset): (Option<f32>, f32),
    groups: &[Group],
) -> f32 {
    let cw = w.min(MENU_PAGE_W);
    let cx = x + (w - cw) / 2.0;
    // Under the search row on the one-column layouts; on the wide layout
    // the search is in the top bar and the card starts where a page's content does.
    let gap = if ui.compact { HOME_CARD_GAP } else { 0.0 };
    let top = y + gap;
    let head_h = ui.line_as(HOME_CARD_HEAD);
    let inner = cw - 2.0 * PAD;
    let rows_y = top + PAD + head_h + GRID;
    let (cols, col_w, heights) = columns(ui, inner, groups);
    let tallest = heights.iter().copied().fold(0.0, f32::max);
    let avail = window.map_or(f32::INFINITY, |h| (h - rows_y - MARGIN_WIDE).floor());
    let scrolls = tallest > avail && avail >= HOME_BOX_MIN_ROWS * LIST_2;
    let none_h = ui.line_as(BODY_L) + PAD;
    let body = if groups.is_empty() {
        none_h
    } else if scrolls {
        avail
    } else {
        tallest
    };
    let card_h = rows_y - top + body + GRID;
    card_surface(ui, (cx, top, cw, card_h));
    ui.text_as(
        cx + PAD + LIST_PAD,
        top + PAD,
        HOME_CARD_HEAD,
        Role::OnSurface,
        EN.home_box_heading,
    );
    if groups.is_empty() {
        ui.text_as(
            cx + PAD + LIST_PAD,
            rows_y,
            BODY_L,
            Role::OnSurfaceVariant,
            EN.session_nothing_loaded,
        );
        return gap + card_h;
    }
    let draw_cols = |ui: &mut Ui| {
        let mut colx = cx + PAD;
        for c in &cols {
            let mut cy = rows_y;
            for &gi in c {
                let g = &groups[gi];
                section_label(ui, colx + LIST_PAD, cy, g.label);
                cy += ui.line_as(TITLE_S) + GRID;
                list_rows(ui, (colx, cy), col_w, &g.rows);
                cy += rows_h(ui, col_w, &g.rows) + SPACER;
            }
            colx += col_w + SPACER;
        }
    };
    if scrolls {
        let clip = ui.rect(cx, rows_y, cw, avail);
        let mark = ui.presses_mark();
        let open = ui.region_open(Region::scrolled(Slot::HomeBox, clip, offset));
        draw_cols(ui);
        ui.clip_presses_since(mark, clip);
        ui.region_close(open, (tallest - avail).max(0.0));
    } else {
        draw_cols(ui);
    }
    gap + card_h
}

/// What is loaded as one section of the menu page (`docs/HOME.md` §3): on a
/// wide window the card of the pane's groups, on the small panel a tinted
/// box of list rows (what needs you, the keys no wallet uses, All keys).
/// `lead` is the box's lead; the card has every waiting job in its Waiting
/// group.
pub(crate) fn loaded_sections(
    app: &View,
    window: Option<f32>,
    lead: Option<CardLead>,
) -> Vec<Section<'static>> {
    let offset = app.scroll.home_box_offset();
    if !app.compact {
        let groups = groups(app);
        return vec![Section::block(None, move |ui, (x, y), w| {
            pane_card(ui, (x, y), w, (window, offset), &groups)
        })];
    }
    let (mut rest, keys, all_keys) = compact_rows(app, lead.as_ref());
    rest.extend(all_keys);
    if rest.is_empty() && keys.is_empty() && lead.is_none() {
        return Vec::new();
    }
    vec![Section::block(None, move |ui, (x, y), w| {
        compact_box(ui, (x, y, w), (window, offset), &rest, &keys, lead.as_ref())
    })]
}

/// The phone's and the Pi's box: its heading, the lead, then the rows and as
/// many key rows as the window holds, scrolling within itself where it
/// holds fewer than two rows.
fn compact_box(
    ui: &mut Ui,
    (bx, y, bw): (f32, f32, f32),
    (window, offset): (Option<f32>, f32),
    rest: &[SessionRow],
    keys: &[SessionRow],
    lead: Option<&CardLead>,
) -> f32 {
    section_label(ui, bx, y, EN.home_box_heading);
    let head = ui.line_as(TITLE_S) + GRID;
    let by = y + head;
    let lead_h = lead.map_or(0.0, |l| lead_card(ui, l.card(), (bx, by), bw, true) + GRID);
    let by = by + lead_h;
    let head = head + lead_h;
    let rest_h = rows_h(ui, bw, rest);
    // Without a window height the page scrolls as a whole: up to three
    // keys. With one, the box's room runs to a margin above the foot;
    // where the fixed part leaves less than two rows the page scrolls
    // as one.
    let avail = window.map_or(f32::INFINITY, |h| (h - by - MARGIN_WIDE).floor());
    let n = keys_fitting(avail, rest_h, keys.len(), HOME_BOX_KEYS);
    let all_h = rest_h + n as f32 * LIST_2;
    let mut rows: Vec<SessionRow> = rest.to_vec();
    rows.extend(keys.iter().take(n).cloned());
    if all_h <= avail || avail < HOME_BOX_MIN_ROWS * LIST_2 {
        return head + list_box(ui, (bx, by), bw, &rows);
    }
    // Scrolls within itself: the surface stays, the rows move.
    crate::nav::screen::loaded_surface(ui, bx, by, bw, avail);
    let clip = ui.rect(bx, by, bw, avail);
    let mark = ui.presses_mark();
    let open = ui.region_open(Region::scrolled(Slot::HomeBox, clip, offset));
    list_rows(ui, (bx, by), bw, &rows);
    ui.clip_presses_since(mark, clip);
    ui.region_close(open, (all_h - avail).max(0.0));
    head + avail
}
