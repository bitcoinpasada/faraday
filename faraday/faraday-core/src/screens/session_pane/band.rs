//! The Waiting band at the session pane's foot (`docs/WORKSPACE.md` §1): the
//! last group of the pane, drawn across the card outside the part that
//! scrolls, a hairline over it, its rows in columns on a wide card, at most a
//! third of the pane's height and scrolling within itself beyond that. Its
//! height is read from its rows as they are drawn.

use super::rows::Group;
use super::{list_rows, row_h, rows_h};
use crate::ui::pal::*;
use crate::ui::{Region, Slot, Ui};
use faraday_draw::tokens::{GAP_SMALL, GRID, LIST_2, LIST_PAD, PAD, SESSION_BAND_SHARE, TITLE_S};
use faraday_ui::parts::section_label;

/// The pane's groups with the band split off: the band is the last group,
/// when the pane has one.
pub(super) fn split(groups: &[Group]) -> (&[Group], Option<&Group>) {
    match groups.split_last() {
        Some((last, rest)) if last.band => (rest, Some(last)),
        _ => (groups, None),
    }
}

/// How the band's rows stand: the rows of each column in order, the
/// columns' width, and the height the rows take whole, label and padding
/// included.
pub(super) struct Layout {
    cols: Vec<Vec<usize>>,
    col_w: f32,
    need: f32,
}

impl Layout {
    /// The height of the band whole, its hairline not counted.
    pub(super) fn need(&self) -> f32 {
        self.need
    }
}

/// The band's layout in a card `w` wide: as many columns of at least
/// `SESSION_COL_MIN` as fit across the padding, each row into the shortest
/// column, the lead first.
pub(super) fn layout(ui: &Ui, w: f32, g: &Group) -> Layout {
    let inner = w - 2.0 * PAD;
    let n = super::column_count(inner).min(g.rows.len()).max(1);
    let col_w = super::column_w(inner, n);
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
    Layout { cols, col_w, need }
}

/// The height the band takes in a pane `pane_h` tall: all it needs, or a
/// third of the pane, but never less than its label and one row.
pub(super) fn shown_h(ui: &Ui, need: f32, pane_h: f32) -> f32 {
    let least = GRID + ui.line_as(TITLE_S) + GRID + LIST_2 + 2.0 * GAP_SMALL;
    need.min((pane_h * SESSION_BAND_SHARE).max(least))
}

/// The band at `(x, y)`, `w` wide and `h` tall: the hairline across the
/// card, the heading, then the rows in columns, scrolling within where `h`
/// cuts them, `offset` down. Returns how far it can scroll.
pub(super) fn draw(
    ui: &mut Ui,
    (x, y, w, h): (f32, f32, f32, f32),
    offset: f32,
    g: &Group,
    lay: &Layout,
) -> f32 {
    ui.rule(x, y, w, Role::OutlineVariant);
    let top = y + GRID;
    section_label(ui, x + PAD + LIST_PAD, top, g.label);
    let rows_y = top + ui.line_as(TITLE_S) + GRID;
    let view = (y + h - rows_y).max(0.0);
    let clip = ui.rect(x, rows_y, w, view);
    let mark = ui.presses_mark();
    let open = ui.region_open(Region::scrolled(Slot::HomeBand, clip, offset));
    let mut colx = x + PAD;
    let mut tallest = 0.0_f32;
    for c in &lay.cols {
        let rows = || c.iter().map(|&i| &g.rows[i]);
        list_rows(ui, (colx, rows_y), lay.col_w, rows());
        tallest = tallest.max(rows_h(ui, lay.col_w, rows()));
        colx += lay.col_w + super::SPACER;
    }
    ui.clip_presses_since(mark, clip);
    let max = (tallest - view).max(0.0);
    ui.region_close(open, max);
    max
}
