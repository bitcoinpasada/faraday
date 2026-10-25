//! The "Demo only" card (`docs/DESIGN.md` §4.18, `docs/DEMO.md` §14): what
//! a page or a sheet offers only in Demo, or only on Faraday Desktop as a
//! stand-in for the person's own input (a sample, the computer's dice,
//! the demo's passphrase), kept out of the page's own column. The page
//! hands each item to the frame ([`Ui::demo_offer`], [`Ui::demo_line`])
//! where it would have drawn it; the frame draws them in one outlined
//! card headed "Demo only", a supporting pane at the window's right or a
//! strip pinned at its foot (`docs/LAYOUT.md` §6). Material 3's outlined
//! card: an outline in outline-variant on the surface, a title, its
//! supporting text and its buttons.

use crate::pal::*;
use crate::{Style, Ui, W};
use faraday_draw::tokens as t;

/// What a page hands to its "Demo only" card.
#[derive(Debug, Clone, PartialEq)]
pub enum DemoItem<A> {
    /// A button: its label and its press.
    Button(&'static str, A),
    /// A line saying what Demo does in place of the person's own.
    Line(String),
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Hands the frame's "Demo only" card a button, once however often
    /// the page offers it.
    pub fn demo_offer(&mut self, label: &'static str, action: A) {
        let known = self
            .demo
            .iter()
            .any(|i| matches!(i, DemoItem::Button(_, a) if *a == action));
        if !known {
            self.demo.push(DemoItem::Button(label, action));
        }
    }

    /// Hands the frame's "Demo only" card a line.
    pub fn demo_line(&mut self, text: impl Into<String>) {
        let text = text.into();
        if !self.demo.iter().any(|i| *i == DemoItem::Line(text.clone())) {
            self.demo.push(DemoItem::Line(text));
        }
    }

    /// What the page handed to its "Demo only" card, taken.
    pub fn take_demo(&mut self) -> Vec<DemoItem<A>> {
        std::mem::take(&mut self.demo)
    }
}

/// The card's heading, "Demo only" as `head`, at (`x`, `y`): Material's
/// title small. Returns the height with the gap under it.
pub fn head<A: Copy + PartialEq>(ui: &mut Ui<A>, (x, y): (f32, f32), head: &str) -> f32 {
    ui.text(x, y, t::BODY, W::S, Role::OnSurface, head);
    ui.line(t::BODY, W::S) + t::GAP
}

/// The heading's height, measured and not drawn.
pub fn head_h<A: Copy + PartialEq>(ui: &Ui<A>) -> f32 {
    ui.line(t::BODY, W::S) + t::GAP
}

/// The card's lines, then its buttons in rows, at (`x`, `y`) in `w`.
/// Returns the height used.
pub fn items<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    items: &[DemoItem<A>],
) -> f32 {
    let mut cy = y;
    for i in items {
        if let DemoItem::Line(s) = i {
            cy += ui.wrap(x, cy, w, t::LABEL, W::R, Role::OnSurfaceVariant, s) + t::GAP;
        }
    }
    let buttons: Vec<(&str, Style, A)> = items
        .iter()
        .filter_map(|i| match i {
            DemoItem::Button(l, a) => Some((*l, Style::Secondary, *a)),
            DemoItem::Line(_) => None,
        })
        .collect();
    if !buttons.is_empty() {
        cy += crate::parts::wrap_buttons(ui, x, cy, w, t::TOUCH, &buttons);
    }
    cy - y
}

/// The height [`items`] takes in `w`, measured and not drawn.
pub fn items_h<A: Copy + PartialEq>(ui: &mut Ui<A>, w: f32, items: &[DemoItem<A>]) -> f32 {
    let mut h = 0.0;
    for i in items {
        if let DemoItem::Line(s) = i {
            h += crate::parts::measure_wrap(ui, w, t::LABEL, W::R, s) + t::GAP;
        }
    }
    let labels: Vec<&str> = items
        .iter()
        .filter_map(|i| match i {
            DemoItem::Button(l, _) => Some(*l),
            DemoItem::Line(_) => None,
        })
        .collect();
    if !labels.is_empty() {
        h += crate::parts::wrap_buttons_height(ui, w, t::TOUCH, &labels);
    }
    h
}

/// The card's outline around (`x`, `y`, `w`, `h`): Material's outlined
/// card.
pub fn outline<A: Copy + PartialEq>(ui: &mut Ui<A>, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.stroke(x, y, w, h, t::SHAPE_M, Role::OutlineVariant);
}

/// A strip's surface over what is under it at (`x`, `y`, `w`, `h`), with
/// a rule at its top: a press there reaches nothing the page drew under
/// it.
pub fn cover<A: Copy + PartialEq>(ui: &mut Ui<A>, (x, y, w, h): (f32, f32, f32, f32)) {
    let area = ui.rect(x, y, w, h);
    let mut k = 0;
    while k < ui.hits.len() {
        let r = ui.hits[k].0;
        let across = r.x < area.x + area.w && r.x + r.w > area.x;
        if across && r.y >= area.y {
            ui.hits.remove(k);
            continue;
        }
        if across && r.y + r.h > area.y {
            ui.hits[k].0.h = area.y - r.y;
        }
        k += 1;
    }
    ui.fill(x, y, w, h, t::SHAPE_NONE, Role::Surface);
    ui.rule(x, y, w, Role::OutlineVariant);
}
