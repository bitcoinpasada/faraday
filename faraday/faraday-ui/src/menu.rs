//! Material's menu (`docs/THEMES.md` §7, F108): on a wide window a
//! popup under what opened it (above it where it does not fit below),
//! surface-container with extra-small corners at elevation level 2, its
//! items 48 dp tall, the chosen one in secondary-container with a tick;
//! on the small panel a sheet with its way out at its foot. Its items may
//! stand under group headings. A press outside it takes the frame's
//! [`Ui::outside`]. Which items there are, which is chosen and which the
//! keyboard is on is the app's: this draws what it is given.

use crate::pal::*;
use crate::sheet;
use crate::{Glyph, Style, Ui};
use faraday_draw::tokens::{
    ELEVATION_MENU, FOCUS_RING, GAP, GRID, ICON_GAP, ICON_GLYPH_BUTTON, LABEL_L, LABEL_M,
    LIST_PAD_V, MENU_HEAD_H, MENU_INDENT, MENU_ITEM_H, MENU_ITEM_PAD, MENU_MAX_W, MENU_MIN_W,
    MENU_PAD_V, SHAPE_XS,
};
use faraday_draw::widgets::Icon;

/// One line of a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuLine<'s, A> {
    /// A group's heading, over the items after it, which stand in under
    /// it.
    Head(&'s str),
    /// An item: what it reads, the press it is, and whether it is the
    /// one chosen now.
    Item(&'s str, A, bool),
}

/// A menu as it is drawn.
#[derive(Debug, Clone, Copy)]
pub struct Menu<'s, A> {
    /// Its title: the sheet's head on the small panel, a heading over its
    /// items in the popup; none for a popup with no title.
    pub title: Option<&'s str>,
    /// The glyph before its title on the small panel.
    pub icon: Glyph,
    /// Its lines, in order.
    pub lines: &'s [MenuLine<'s, A>],
    /// The item the arrow keys are on, by its place among the items:
    /// ringed.
    pub at: Option<usize>,
    /// Where what opened it was drawn, in pixels (x, y, w, h): the popup
    /// goes under it; centred where there is none.
    pub anchor: Option<(i32, i32, i32, i32)>,
    /// The small panel's way out at the sheet's foot.
    pub cancel: (&'s str, A),
    /// How much of the popup shows, 0 to 1, from its top edge down,
    /// while it opens or closes; 1 at rest.
    pub shown: f32,
}

/// An item's height at the text size ([`MENU_ITEM_H`] at Standard).
fn item_h<A: Copy + PartialEq>(ui: &Ui<A>) -> f32 {
    ui.row_height(MENU_ITEM_H, ui.line_as(LABEL_L), LIST_PAD_V)
}

/// A heading's height at the text size.
fn head_h<A: Copy + PartialEq>(ui: &Ui<A>) -> f32 {
    MENU_HEAD_H + ui.line_grown_as(LABEL_M)
}

/// The room a chosen item's tick takes at its end.
const TICK: f32 = ICON_GLYPH_BUTTON + ICON_GAP;

/// Costs measuring every line. The width the popup takes for `m`'s
/// lines: the widest with its room, between Material's least and most.
pub fn menu_width<A: Copy + PartialEq>(ui: &Ui<A>, m: &Menu<A>) -> f32 {
    let mut widest = m.title.map_or(0.0, |t| ui.measure_as(LABEL_M, t));
    let mut under = 0.0;
    for line in m.lines {
        match *line {
            MenuLine::Head(name) => {
                widest = widest.max(ui.measure_as(LABEL_M, name));
                under = MENU_INDENT;
            }
            MenuLine::Item(label, ..) => {
                widest = widest.max(ui.measure_as(LABEL_L.regular(), label) + under + TICK);
            }
        }
    }
    (widest + 2.0 * MENU_ITEM_PAD).clamp(MENU_MIN_W, MENU_MAX_W)
}

/// Costs measuring every line. How tall `m`'s lines are, its title
/// among them when `titled`, with the popup's room above and below.
fn content_h<A: Copy + PartialEq>(ui: &Ui<A>, m: &Menu<A>, titled: bool) -> f32 {
    let (ih, hh) = (item_h(ui), head_h(ui));
    let title = if titled && m.title.is_some() { hh } else { 0.0 };
    let lines: f32 = m
        .lines
        .iter()
        .map(|l| match l {
            MenuLine::Head(_) => hh,
            MenuLine::Item(..) => ih,
        })
        .sum();
    title + lines + 2.0 * MENU_PAD_V
}

/// The lines from (x, y), `w` wide: each heading over the items under
/// it; the chosen item in secondary-container with a tick at its end, the
/// one the keyboard is on ringed inside its bounds. Returns their height.
fn lines<A: Copy + PartialEq>(ui: &mut Ui<A>, m: &Menu<A>, (x, y, w): (f32, f32, f32)) -> f32 {
    let (ih, hh) = (item_h(ui), head_h(ui));
    let mut cy = y;
    let mut k = 0;
    let mut under = 0.0;
    for line in m.lines {
        match *line {
            MenuLine::Head(name) => {
                let room = w - 2.0 * MENU_ITEM_PAD;
                let at = (x + MENU_ITEM_PAD, cy, hh, room);
                ui.text_fit_as(at, LABEL_M, Role::OnSurfaceVariant, name);
                under = MENU_INDENT;
                cy += hh;
            }
            MenuLine::Item(label, a, chosen) => {
                item(
                    ui,
                    (x, cy, w, ih),
                    (label, a, chosen),
                    under,
                    m.at == Some(k),
                );
                cy += ih;
                k += 1;
            }
        }
    }
    cy - y
}

/// One item at (x, y), `w` by `h`, standing `under` in from its start.
fn item<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w, h): (f32, f32, f32, f32),
    (label, a, chosen): (&str, A, bool),
    under: f32,
    keys: bool,
) {
    if chosen {
        ui.fill(x, y, w, h, 0.0, Role::SecondaryContainer);
    }
    let fg = on_selected(chosen, Role::OnSurface);
    ui.state_layer((x, y, w, h, 0.0), fg, a);
    if keys || ui.focused == Some(a) {
        // Ringed inside the item: the popup's edge would cut a ring
        // outside it.
        let rect = ui.rect(
            x + FOCUS_RING / 2.0,
            y + FOCUS_RING / 2.0,
            w - FOCUS_RING,
            h - FOCUS_RING,
        );
        let color = ui.col(Role::Secondary);
        ui.c.stroke_rounded_rect(rect, SHAPE_XS * ui.f, FOCUS_RING * ui.f, color);
    }
    let ix = x + MENU_ITEM_PAD + under;
    let room = (x + w - MENU_ITEM_PAD - TICK - ix).max(0.0);
    ui.text_fit_as((ix, y, h, room), LABEL_L.regular(), fg, label);
    if chosen {
        let tx = x + w - MENU_ITEM_PAD - ICON_GLYPH_BUTTON;
        let ty = y + (h - ICON_GLYPH_BUTTON) / 2.0;
        ui.icon(tx, ty, ICON_GLYPH_BUTTON, Icon::Done, ICON_GLYPH_BUTTON, fg);
    }
    ui.hit_drawn(x, y, w, h, a);
}

/// Costs measuring every line. Menu `m` in a frame `w` by `h`: a popup
/// under its anchor on a wide window, a sheet on the small panel. What
/// does not fit scrolls, its edges fading into the menu's own container.
pub fn menu<A: Copy + PartialEq>(ui: &mut Ui<A>, w: f32, h: f32, m: &Menu<A>) {
    if ui.compact {
        let place = (GAP, w - 2.0 * GAP);
        sheet::scroll_sheet(ui, place, h, GAP, 2.0 * GRID, &mut |ui, x, y, iw| {
            let mut cy = y;
            if let Some(t) = m.title {
                cy += sheet::sheet_head(ui, x, cy, iw, m.icon, Role::OnSurface, t);
            }
            cy += lines(ui, m, (x, cy, iw)) + GAP;
            cy += sheet::buttons(ui, x, cy, iw, &[(m.cancel.0, Style::Secondary, m.cancel.1)]);
            cy - y
        });
        return;
    }
    let margin = GAP;
    let pw = menu_width(ui, m).min(w - 2.0 * margin);
    let content = content_h(ui, m, true);
    let ph = content.min(h - 2.0 * margin);
    let f = ui.f.max(0.1);
    let (x, y) = match m.anchor {
        Some((ax, ay, aw, ah)) => {
            let (ax, ay) = (ax as f32 / f - ui.ox, ay as f32 / f - ui.oy);
            let (aw, ah) = (aw as f32 / f, ah as f32 / f);
            let x = (ax + aw - pw).clamp(margin, (w - pw - margin).max(margin));
            let y = if ay + ah + ph <= h - margin {
                ay + ah
            } else if ay - ph >= margin {
                ay - ph
            } else {
                (h - ph - margin).max(margin)
            };
            (x, y)
        }
        None => ((w - pw) / 2.0, (h - ph) / 2.0),
    };
    if let Some(a) = ui.outside {
        ui.hit_around(x, y, pw, ph, a);
    }
    let max = (content - ph).max(0.0);
    // Opening or closing, it unrolls from its top edge.
    let ph = (ph * m.shown.clamp(0.0, 1.0)).round();
    if ph <= 0.0 {
        return;
    }
    ui.shadow(x, y, pw, ph, SHAPE_XS, ELEVATION_MENU);
    ui.fill(x, y, pw, ph, SHAPE_XS, Role::SurfaceContainer);
    // At High contrast, where it casts no shadow, its outline sets it
    // apart (`docs/THEMES.md` §9).
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, pw, ph, SHAPE_XS, Role::Outline);
    }
    let off = ui.offset.clamp(0.0, max);
    let clip = ui.rect(x, y, pw, ph);
    ui.c.push_clip(clip);
    let mark = ui.hits.len();
    let mut cy = y + MENU_PAD_V - off;
    if let Some(t) = m.title {
        let hh = head_h(ui);
        let at = (x + MENU_ITEM_PAD, cy, hh, pw - 2.0 * MENU_ITEM_PAD);
        ui.text_fit_as(at, LABEL_M, Role::OnSurface, t);
        cy += hh;
    }
    lines(ui, m, (x, cy, pw));
    // What is scrolled out of the popup takes no press.
    let mut k = mark;
    while k < ui.hits.len() {
        let r = ui.hits[k].0.intersect(&clip);
        if r.w <= 0 || r.h <= 0 {
            ui.hits.remove(k);
        } else {
            ui.hits[k].0 = r;
            k += 1;
        }
    }
    ui.c.pop_clip();
    ui.report_scroll_on(clip, max, Role::SurfaceContainer);
}
