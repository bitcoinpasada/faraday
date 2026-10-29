//! Material 3's tabs (`docs/DESIGN.md` §4): a row of labels over a rule
//! in outline-variant, the one shown marked by an indicator in primary.
//! Primary tabs stand at the top of a pane and name its main pages (the
//! Learn sheet's and pane's pages); secondary tabs part what is inside
//! one card (codex32's worksheets). Where the labels all fit, each tab
//! takes an equal share of the row, or its own width where one label is
//! wider than a share; where they do not, the row scrolls (Material 3
//! Tabs, scrollable tabs). A label is never cut: a scrolled row shows
//! only whole tabs, the one shown among them, sharing the row's room,
//! and a chevron at an end with tabs past it, whose press shows the next
//! tab that way.

use crate::pal::{Paint, Role};
use crate::{Shape, Ui};
use faraday_draw::tokens::{
    CODEX32_TAB_GLYPH, CODEX32_TAB_ICON, GAP_SMALL, ICON_BUTTON, ICON_GLYPH, TAB_INDICATOR_MIN,
    TAB_INDICATOR_PRIMARY, TAB_INDICATOR_SECONDARY, TAB_PAD, TITLE_S, TOUCH, shape_full,
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
        let on = items.iter().position(|t| t.on).unwrap_or(0);
        // Fixed tabs share the row where every label fits its share; tabs
        // that fit only at their own widths take those, from the start.
        let (shown, widths, before, after) = if natural.iter().all(|&n| n <= equal) {
            (0..items.len(), vec![equal; items.len()], false, false)
        } else if total <= w {
            (0..items.len(), natural.clone(), false, false)
        } else {
            whole_tabs(&natural, on, w)
        };
        self.rule(x, y + h - 1.0, w, Role::OutlineVariant);
        let row = self.rect(x, y, w, h);
        self.c.push_clip(row);
        let cut_from = (self.c.cut_mark(), self.cut.len());
        let mut tx = x + if before { TOUCH } else { 0.0 };
        let mut narrow = false;
        for (i, tw) in shown.clone().zip(&widths) {
            if let Some(item) = items.get(i) {
                // Only a tab wider than the whole row is narrowed: its
                // label is held to its own box.
                let own = *tw < natural[i];
                narrow |= own;
                if own {
                    let clip = self.rect(tx, y, *tw, h);
                    self.c.push_clip(clip);
                }
                self.tab(item, kind, (tx, y, *tw, h), content[i]);
                if own {
                    self.c.pop_clip();
                }
            }
            tx += tw;
        }
        // That tab's label runs on under the row's edge as a scrolled
        // row's would, not cut off.
        if narrow {
            self.c.scrolled_across(cut_from.0);
            self.cut.truncate(cut_from.1);
        }
        let prev = shown.start.checked_sub(1).and_then(|i| items.get(i));
        if let Some(prev) = prev.filter(|_| before) {
            self.tab_chevron((x, y, h), Icon::ChevronLeft, prev.action);
        }
        if let Some(next) = items.get(shown.end).filter(|_| after) {
            self.tab_chevron((x + w - TOUCH, y, h), Icon::ChevronRight, next.action);
        }
        self.c.pop_clip();
        h
    }

    /// A scrolled row's chevron at (x, y), `h` tall: Material's standard
    /// icon button in a `TOUCH` target, whose press does `action`, the
    /// next tab past that end.
    fn tab_chevron(&mut self, (x, y, h): (f32, f32, f32), icon: Icon, action: A) {
        let edge = (TOUCH - ICON_BUTTON) / 2.0;
        let (bx, by) = (x + edge, y + (h - ICON_BUTTON) / 2.0);
        let shape = Shape::new(bx, by, ICON_BUTTON, ICON_BUTTON, shape_full(ICON_BUTTON));
        let ink = Role::OnSurfaceVariant.paint();
        self.press_to(shape, (x, y, TOUCH, h), ink, action);
        let glyph = (ICON_BUTTON - ICON_GLYPH) / 2.0;
        self.icon(bx + glyph, by + glyph, ICON_GLYPH, icon, ICON_GLYPH, ink);
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

/// The tabs a scrolled row shows whole, `w` wide, tabs `natural` wide:
/// the one shown (`on`) and as many before it as fit, then after it,
/// each end with tabs past it keeping a chevron's `TOUCH` of the row;
/// the room left is shared out among the tabs shown, and a tab shown
/// that is wider than the row by itself is narrowed to it. Returns their
/// range, their widths, and whether a chevron stands before and after.
/// Arithmetic.
fn whole_tabs(
    natural: &[f32],
    on: usize,
    w: f32,
) -> (std::ops::Range<usize>, Vec<f32>, bool, bool) {
    let n = natural.len();
    let room = |a: usize, b: usize| {
        let sum: f32 = natural[a..b].iter().sum();
        let ends = f32::from(u8::from(a > 0)) + f32::from(u8::from(b < n));
        w - sum - ends * TOUCH
    };
    let on = on.min(n.saturating_sub(1));
    let a = (0..=on).find(|&a| room(a, on + 1) >= 0.0).unwrap_or(on);
    let mut b = on + 1;
    while b < n && room(a, b + 1) >= 0.0 {
        b += 1;
    }
    let spare = room(a, b) / (b - a).max(1) as f32;
    let widths = natural[a..b].iter().map(|t| (t + spare).max(0.0)).collect();
    (a..b, widths, a > 0, b < n)
}
