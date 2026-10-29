//! Material 3's tabs (`docs/DESIGN.md` §4): a row of labels over a rule
//! in outline-variant, the one shown marked by an indicator in primary.
//! Primary tabs stand at the top of a pane and name its main pages (the
//! Learn sheet's and pane's pages); secondary tabs part what is inside
//! one card (codex32's worksheets). Where the labels all fit, each tab
//! takes an equal share of the row; where they do not, the row scrolls,
//! the tab shown kept in view with its neighbours at the edges, so a
//! press on one at an edge brings the next into view.

use crate::pal::{Paint, Role};
use crate::{Shape, Ui};
use faraday_draw::tokens::{
    CODEX32_TAB_GLYPH, CODEX32_TAB_ICON, GAP_SMALL, TAB_INDICATOR_MIN, TAB_INDICATOR_PRIMARY,
    TAB_INDICATOR_SECONDARY, TAB_PAD, TITLE_S,
};
use faraday_draw::widgets::Icon;

/// Which of Material's two kinds of tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    /// At the top of a pane, its main pages: the label of the one shown
    /// in primary, a 3 dp indicator as wide as its content.
    Primary,
    /// Inside a part of a page: the label of the one shown in
    /// on-surface, a 2 dp indicator across the whole tab.
    Secondary,
}

/// One tab: its label, whether it is the one shown, whether it is done
/// (a check before its label), and what a press does.
pub struct Tab<'s, A> {
    /// What it says.
    pub label: &'s str,
    /// It is the one shown.
    pub on: bool,
    /// It is done: a check before its label.
    pub done: bool,
    /// What a press does.
    pub action: A,
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Costs measuring each label. `items` as a row of tabs of `kind`
    /// from (x, y), `w` wide and `h` tall, or taller where the text size
    /// needs it. Returns the row's height.
    pub fn tabs(
        &mut self,
        (x, y, w, h): (f32, f32, f32, f32),
        kind: TabKind,
        items: &[Tab<'_, A>],
    ) -> f32 {
        let h = self.button_height(h);
        let icon = CODEX32_TAB_ICON + GAP_SMALL;
        let content: Vec<f32> = items
            .iter()
            .map(|t| self.measure_as(TITLE_S, t.label) + if t.done { icon } else { 0.0 })
            .collect();
        let natural: Vec<f32> = content.iter().map(|c| c + 2.0 * TAB_PAD).collect();
        let total: f32 = natural.iter().sum();
        let equal = w / items.len().max(1) as f32;
        // Fixed tabs share the row where every label fits its share.
        let fixed = natural.iter().all(|&n| n <= equal);
        let widths: Vec<f32> = if fixed {
            vec![equal; items.len()]
        } else {
            natural
        };
        let on = items.iter().position(|t| t.on).unwrap_or(0);
        // Scrolled so the tab shown stands in the middle where it can.
        let before: f32 = widths[..on].iter().sum();
        let shift = if fixed {
            0.0
        } else {
            let mid = before + widths.get(on).copied().unwrap_or(0.0) / 2.0;
            (mid - w / 2.0).clamp(0.0, (total - w).max(0.0))
        };
        self.rule(x, y + h - 1.0, w, Role::OutlineVariant);
        let row = self.rect(x, y, w, h);
        self.c.push_clip(row);
        let cut_from = (self.c.cut_mark(), self.cut.len());
        let mut tx = x - shift;
        for (i, item) in items.iter().enumerate() {
            let tw = widths[i];
            if tx + tw > x && tx < x + w {
                self.tab(item, kind, (tx, y, tw, h), content[i]);
            }
            tx += tw;
        }
        // A row that scrolls holds the tabs past its sides scrolled
        // there, not cut off.
        if !fixed {
            self.c.scrolled_across(cut_from.0);
            self.cut.truncate(cut_from.1);
        }
        self.c.pop_clip();
        h
    }

    /// One tab of [`Ui::tabs`] in the box (x, y, w, h), its label and
    /// check `content` wide.
    fn tab(
        &mut self,
        item: &Tab<'_, A>,
        kind: TabKind,
        (x, y, w, h): (f32, f32, f32, f32),
        content: f32,
    ) {
        let ink: Paint = match (item.on, kind) {
            (true, TabKind::Primary) => Role::Primary.paint(),
            (true, TabKind::Secondary) => Role::OnSurface.paint(),
            (false, _) => Role::OnSurfaceVariant.paint(),
        };
        self.press(Shape::new(x, y, w, h, 0.0), ink, item.action);
        let mut lx = x + (w - content) / 2.0;
        if item.done {
            let at = y + (h - CODEX32_TAB_ICON) / 2.0;
            self.icon(
                lx,
                at,
                CODEX32_TAB_ICON,
                Icon::Done,
                CODEX32_TAB_GLYPH,
                Role::Ok,
            );
            lx += CODEX32_TAB_ICON + GAP_SMALL;
        }
        self.text_mid_as((lx, y, h), TITLE_S, ink, item.label);
        if item.on {
            match kind {
                TabKind::Primary => {
                    // Round at its top corners: a shape twice as tall,
                    // its lower half cut off at the tab's foot.
                    let iw = content.max(TAB_INDICATOR_MIN);
                    let ix = x + (w - iw) / 2.0;
                    let t = TAB_INDICATOR_PRIMARY;
                    let foot = self.rect(ix, y, iw, h);
                    self.c.push_clip(foot);
                    self.fill(ix, y + h - t, iw, 2.0 * t, t, Role::Primary);
                    self.c.pop_clip();
                }
                TabKind::Secondary => {
                    let t = TAB_INDICATOR_SECONDARY;
                    self.fill(x, y + h - t, w, t, 0.0, Role::Primary);
                }
            }
        }
    }
}
