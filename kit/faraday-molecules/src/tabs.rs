//! The tabs molecule: Material 3's tabs (09 §3.2; `docs/DESIGN.md` §4).
//!
//! A row of labels over a rule in outline-variant, the one shown marked by an indicator in
//! primary. Primary tabs stand at the top of a pane and name its main pages (the guide sheet's
//! and pane's pages); secondary tabs part what is inside one card (codex32's worksheets). Where
//! the labels all fit, each tab takes an equal share of the row, or its own width where one
//! label is wider than a share; where they do not, the row scrolls (Material 3 Tabs, scrollable
//! tabs). A label is never cut: a scrolled row shows only whole tabs, the one shown among them,
//! sharing the row's room, and a chevron at an end with tabs past it, whose press shows the next
//! tab that way.

use faraday_atoms::hits::Shape;
use faraday_atoms::{Icon, Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::{
    CODEX32_TAB_GLYPH, CODEX32_TAB_ICON, GAP_SMALL, ICON_BUTTON, ICON_GLYPH, TAB_INDICATOR_MIN,
    TAB_INDICATOR_PRIMARY, TAB_INDICATOR_SECONDARY, TAB_PAD, TITLE_S, shape_full,
};
use faraday_tokens::palette::{Paint, Role};

/// Which of Material's two kinds of tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    /// At the top of a pane, its main pages: the label of the one shown in primary, a 3 dp
    /// indicator as wide as its content.
    Primary,
    /// Inside a part of a page: the label of the one shown in on-surface, a 2 dp indicator
    /// across the whole tab.
    Secondary,
}

/// One tab: its label, whether it is the one shown, whether it is done (a check before its
/// label), and what a press offers.
pub struct Tab<'s, A> {
    pub label: Shown<'s>,
    /// It is the one shown.
    pub on: bool,
    /// It is done: a check before its label.
    pub done: bool,
    pub offered: Offered<A>,
}

/// A row of tabs.
pub struct TabsModel<'s, A> {
    pub kind: TabKind,
    pub items: Vec<Tab<'s, A>>,
}

/// Costs measuring each label. Draws the row from (x, y), `w` wide and `h` tall, or taller where
/// the text size needs it. Returns the row's height.
pub fn draw<A: Pressed>(
    m: &TabsModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
) -> f32 {
    let items = &m.items;
    let h = ui.button_height(h);
    let icon = CODEX32_TAB_ICON + GAP_SMALL;
    let mut content: Vec<f32> = items
        .iter()
        .map(|t| ui.measure_as(TITLE_S, t.label.as_str()) + if t.done { icon } else { 0.0 })
        .collect();
    let natural: Vec<f32> = content.iter().map(|c| c + 2.0 * TAB_PAD).collect();
    let total: f32 = natural.iter().sum();
    let equal = w / items.len().max(1) as f32;
    let on = items.iter().position(|t| t.on).unwrap_or(0);
    // Fixed tabs share the row where every label fits its share; tabs that fit only at their own
    // widths take those, from the start.
    let (shown, widths, before, after) = if natural.iter().all(|&n| n <= equal) {
        (0..items.len(), vec![equal; items.len()], false, false)
    } else if total <= w {
        (0..items.len(), natural.clone(), false, false)
    } else {
        whole_tabs(&natural, on, w, ui.touch)
    };
    // A tab wider than the whole row by itself wraps its label onto as many lines as it needs,
    // at word breaks (Material 3 Tabs allow two lines), and the row grows to hold them.
    let line = ui.line_as(TITLE_S);
    let mut lines = vec![1usize; items.len()];
    let mut h = h;
    for (i, tw) in shown.clone().zip(&widths) {
        if let Some(item) = items.get(i).filter(|_| *tw < natural[i]) {
            content[i] = (tw - 2.0 * TAB_PAD).max(0.0);
            lines[i] = ui.wrap_lines_as(TITLE_S, item.label.as_str(), content[i]);
            h = h.max(lines[i] as f32 * line + 2.0 * GAP_SMALL);
        }
    }
    ui.rule(x, y + h - 1.0, w, Role::OutlineVariant);
    let row = ui.rect(x, y, w, h);
    ui.c.push_clip(row);
    let mut tx = x + if before { ui.touch } else { 0.0 };
    for (i, tw) in shown.clone().zip(&widths) {
        if let Some(item) = items.get(i) {
            tab(item, m.kind, ui, (tx, y, *tw, h), (content[i], lines[i]));
        }
        tx += tw;
    }
    let prev = shown.start.checked_sub(1).and_then(|i| items.get(i));
    if let Some(prev) = prev.filter(|_| before) {
        chevron(ui, (x, y, h), Icon::ChevronLeft, &prev.offered);
    }
    if let Some(next) = items.get(shown.end).filter(|_| after) {
        let at = (x + w - ui.touch, y, h);
        chevron(ui, at, Icon::ChevronRight, &next.offered);
    }
    ui.c.pop_clip();
    h
}

/// A scrolled row's chevron at (x, y), `h` tall: Material's standard icon button in a touch
/// target, whose press is `offered`, the next tab past that end.
fn chevron<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, h): (f32, f32, f32),
    icon: Icon,
    offered: &Offered<A>,
) {
    let touch = ui.touch;
    let edge = (touch - ICON_BUTTON) / 2.0;
    let (bx, by) = (x + edge, y + (h - ICON_BUTTON) / 2.0);
    let shape = Shape::new(bx, by, ICON_BUTTON, ICON_BUTTON, shape_full(ICON_BUTTON));
    let ink = Role::OnSurfaceVariant.paint();
    ui.offered(offered, |ui, press, _| {
        if let Some(a) = press {
            ui.press_to(shape, (x, y, touch, h), ink, a);
        }
    });
    let glyph = (ICON_BUTTON - ICON_GLYPH) / 2.0;
    ui.icon(bx + glyph, by + glyph, ICON_GLYPH, icon, ICON_GLYPH, ink);
}

/// One tab of [`draw`] in the box (x, y, w, h), its label and check `content` wide, over
/// `lines` lines.
fn tab<A: Pressed>(
    item: &Tab<'_, A>,
    kind: TabKind,
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
    (content, lines): (f32, usize),
) {
    let ink: Paint = match (item.on, kind) {
        (true, TabKind::Primary) => Role::Primary.paint(),
        (true, TabKind::Secondary) => Role::OnSurface.paint(),
        (false, _) => Role::OnSurfaceVariant.paint(),
    };
    ui.offered(&item.offered, |ui, press, _| {
        if let Some(a) = press {
            ui.press(Shape::new(x, y, w, h, 0.0), ink, a);
        }
    });
    let mut lx = x + (w - content) / 2.0;
    if item.done {
        let at = y + (h - CODEX32_TAB_ICON) / 2.0;
        ui.icon(
            lx,
            at,
            CODEX32_TAB_ICON,
            Icon::Done,
            CODEX32_TAB_GLYPH,
            Role::Ok,
        );
        lx += CODEX32_TAB_ICON + GAP_SMALL;
    }
    ui.showing(&item.label, |ui| {
        if lines > 1 {
            let top = y + (h - lines as f32 * ui.line_as(TITLE_S)) / 2.0;
            ui.wrap_as((lx, top, content), TITLE_S, ink, item.label.as_str());
        } else {
            ui.text_mid_as((lx, y, h), TITLE_S, ink, item.label.as_str());
        }
    });
    if item.on {
        match kind {
            TabKind::Primary => {
                // Round at its top corners: a shape twice as tall, its lower half cut off at the
                // tab's foot.
                let iw = content.max(TAB_INDICATOR_MIN);
                let ix = x + (w - iw) / 2.0;
                let t = TAB_INDICATOR_PRIMARY;
                let foot = ui.rect(ix, y, iw, h);
                ui.c.push_clip(foot);
                ui.fill(ix, y + h - t, iw, 2.0 * t, t, Role::Primary);
                ui.c.pop_clip();
            }
            TabKind::Secondary => {
                let t = TAB_INDICATOR_SECONDARY;
                ui.fill(x, y + h - t, w, t, 0.0, Role::Primary);
            }
        }
    }
}

/// The tabs a scrolled row shows whole, `w` wide, tabs `natural` wide: the one shown (`on`) and
/// as many before it as fit, then after it, each end with tabs past it keeping a chevron's
/// `touch` of the row; the room left is shared out among the tabs shown, and a tab shown that is
/// wider than the row by itself is narrowed to it. Returns their range, their widths, and
/// whether a chevron stands before and after. Arithmetic.
fn whole_tabs(
    natural: &[f32],
    on: usize,
    w: f32,
    touch: f32,
) -> (std::ops::Range<usize>, Vec<f32>, bool, bool) {
    let n = natural.len();
    let room = |a: usize, b: usize| {
        let sum: f32 = natural[a..b].iter().sum();
        let ends = f32::from(u8::from(a > 0)) + f32::from(u8::from(b < n));
        w - sum - ends * touch
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

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::*;

    fn tabs(labels: &[&'static str], on: usize) -> TabsModel<'static, P> {
        let items = labels
            .iter()
            .enumerate()
            .map(|(i, l)| Tab {
                label: Shown::plain(*l),
                on: i == on,
                done: false,
                offered: Offered::Yes(P(i as u8)),
            })
            .collect();
        TabsModel {
            kind: TabKind::Primary,
            items,
        }
    }

    fn presses(m: &TabsModel<'_, P>, w: f32) -> (Vec<u8>, f32) {
        let mut canvas = canvas(160);
        let mut h = 0.0;
        let out = frame(&mut canvas, &at_rest(None), |ui| {
            ui.clear(page());
            h = draw(m, ui, (0.0, 0.0, w, 48.0));
        });
        (out.hits.iter().map(|h| h.press.0).collect(), h)
    }

    /// Invariant: Tabs that all fit are each a press; where they do not, the row shows whole tabs with the one shown among them, and a chevron whose press shows the next tab past that end.
    /// Regression: A scrolled row loses the tabs past its end, or its chevron shows a tab other than the next.
    /// Owner: kit
    #[test]
    fn a_row_that_does_not_fit_scrolls_by_a_chevron_to_the_next_tab() {
        let few = tabs(&["One", "Two", "Three"], 0);
        assert_eq!(presses(&few, 600.0).0, [0, 1, 2]);
        let many = tabs(
            &["Overview", "Details", "Backups", "Advanced", "History"],
            0,
        );
        let (hits, _) = presses(&many, 300.0);
        // The tabs shown from the first, then the chevron sending the one after them.
        let shown = hits.len() - 1;
        assert!((1..5).contains(&shown), "{hits:?}");
        assert_eq!(hits, (0..=shown as u8).collect::<Vec<_>>());
    }
}
