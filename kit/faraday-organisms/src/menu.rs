//! The menu organism (09 §3.3): Material's menu (`docs/THEMES.md` §7). On a wide window a popup
//! under what opened it (above it where it does not fit below), surface-container with
//! extra-small corners at elevation level 2, its items 48 dp tall, the chosen one in
//! secondary-container with a tick; on the small panel a sheet with its way out at its foot. Its
//! items may stand under group headings. A press outside it is the press `beside` names. Which
//! items there are, which is chosen and which the keyboard is on is the app's: this draws what it
//! is given.
//!
//! Drawn at rest: `Ui` has no frame time, so the popup's unroll from its top edge is not drawn.

use faraday_atoms::region::Region;
use faraday_atoms::{Anchor, Glyph, Icon, Offered, Plain, Pressed, Shown, Ui};
use faraday_molecules::buttons::ButtonsModel;
use faraday_tokens::dims::{
    DISABLED_CONTENT, FOCUS_RING, GAP, GAP_SMALL, GRID, ICON_GAP, ICON_GLYPH_BUTTON, LABEL_L,
    LABEL_M, LIST_PAD_V, MENU_HEAD_H, MENU_INDENT, MENU_ITEM_H, MENU_ITEM_PAD, MENU_MAX_W,
    MENU_MIN_W, MENU_PAD_V, SHAPE_XS,
};
use faraday_tokens::elevation::ELEVATION_MENU;
use faraday_tokens::palette::{Paint, Role, on_selected};

use crate::sheet::{self, Head};

/// One line of a menu.
pub enum MenuLine<'s, A> {
    /// A group's heading, over the items after it, which stand in under it.
    Head(Shown<'s>),
    /// An item: what it reads, what a press on it does, and whether it is the one chosen now.
    Item(Shown<'s>, Offered<A>, bool),
    /// An item with its icon at its start (Material's menu item with a leading icon).
    Icon(Icon, Shown<'s>, Offered<A>),
    /// An item that begins a dangerous act, in the error tone.
    Danger(Shown<'s>, Offered<A>),
    /// A divider between groups of items (Material's menu divider).
    Divider,
    /// An item that is not available: what it reads and, under it, why. Faint, it takes no press
    /// and the keyboard passes it.
    Off(Shown<'s>, Shown<'s>),
}

/// A menu as it is drawn.
pub struct MenuModel<'s, A> {
    /// Its title: the sheet's head on the small panel, a heading over its items in the popup;
    /// none for a popup with no title.
    pub title: Option<Shown<'s>>,
    /// The glyph before its title on the small panel.
    pub icon: Glyph,
    /// Its lines, in order.
    pub lines: Vec<MenuLine<'s, A>>,
    /// The item the arrow keys are on, by its place among the items: ringed.
    pub at: Option<usize>,
    /// Where what opened it was drawn, in pixels (x, y, w, h): the popup goes under it; centred
    /// where there is none.
    pub anchor: Option<(i32, i32, i32, i32)>,
    /// The small panel's way out at the sheet's foot.
    pub cancel: ButtonsModel<'s, A>,
    /// The press taken on the page beside the menu.
    pub beside: Option<A>,
    /// Keys the scroll of what does not fit the popup.
    pub scroll: Anchor,
}

/// An item's height at the text size ([`MENU_ITEM_H`] at Standard).
fn item_h<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    ui.row_height(MENU_ITEM_H, ui.line_as(LABEL_L), LIST_PAD_V)
}

/// A heading's height at the text size.
fn head_h<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    MENU_HEAD_H + ui.line_grown_as(LABEL_M)
}

/// A divider's height: the menu's own padding, the rule across its middle.
fn divider_h() -> f32 {
    2.0 * MENU_PAD_V
}

/// An unavailable item's height: an item's, and a line of its reason.
fn off_h<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    item_h(ui) + ui.line_as(LABEL_M)
}

/// The room a chosen item's tick takes at its end.
const TICK: f32 = ICON_GLYPH_BUTTON + ICON_GAP;

/// Costs measuring every line. The width the popup takes for `m`'s lines: the widest with its
/// room, between Material's least and most.
pub fn width<A: Pressed>(m: &MenuModel<'_, A>, ui: &Ui<'_, A>) -> f32 {
    let mut widest = m.title.map_or(0.0, |t| ui.measure_as(LABEL_M, t.as_str()));
    let mut under = 0.0;
    for line in &m.lines {
        match line {
            MenuLine::Head(name) => {
                widest = widest.max(ui.measure_as(LABEL_M, name.as_str()));
                under = MENU_INDENT;
            }
            MenuLine::Item(label, ..) | MenuLine::Danger(label, _) => {
                widest =
                    widest.max(ui.measure_as(LABEL_L.regular(), label.as_str()) + under + TICK);
            }
            MenuLine::Icon(_, label, _) => {
                let lead = ICON_GLYPH_BUTTON + ICON_GAP;
                widest = widest
                    .max(ui.measure_as(LABEL_L.regular(), label.as_str()) + under + lead + TICK);
            }
            MenuLine::Off(label, why) => {
                widest =
                    widest.max(ui.measure_as(LABEL_L.regular(), label.as_str()) + under + TICK);
                widest = widest.max(ui.measure_as(LABEL_M, why.as_str()) + under + TICK);
            }
            MenuLine::Divider => {}
        }
    }
    (widest + 2.0 * MENU_ITEM_PAD).clamp(MENU_MIN_W, MENU_MAX_W)
}

/// Costs measuring every line. How tall `m`'s lines are, its title among them when `titled`,
/// with the popup's room above and below.
fn content_h<A: Pressed>(m: &MenuModel<'_, A>, ui: &Ui<'_, A>, titled: bool) -> f32 {
    let (ih, hh) = (item_h(ui), head_h(ui));
    let title = if titled && m.title.is_some() { hh } else { 0.0 };
    let lines: f32 = m
        .lines
        .iter()
        .map(|l| match l {
            MenuLine::Head(_) => hh,
            MenuLine::Item(..) | MenuLine::Icon(..) | MenuLine::Danger(..) => ih,
            MenuLine::Off(..) => off_h(ui),
            MenuLine::Divider => divider_h(),
        })
        .sum();
    title + lines + 2.0 * MENU_PAD_V
}

/// What one item is drawn as.
struct Look<'a, 's, A> {
    label: Shown<'s>,
    offered: &'a Offered<A>,
    chosen: bool,
    tone: Role,
    icon: Option<Icon>,
}

/// The lines from (x, y), `w` wide: each heading over the items under it; the chosen item in
/// secondary-container with a tick at its end, the one the keyboard is on ringed inside its
/// bounds. Returns their height.
fn lines<A: Pressed>(m: &MenuModel<'_, A>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    let (ih, hh) = (item_h(ui), head_h(ui));
    let mut cy = y;
    let mut k = 0;
    let mut under = 0.0;
    for line in &m.lines {
        match line {
            MenuLine::Head(name) => {
                let room = w - 2.0 * MENU_ITEM_PAD;
                let at = (x + MENU_ITEM_PAD, cy, hh, room);
                ui.showing(*name, |ui| {
                    ui.text_fit_as(at, LABEL_M, Role::OnSurfaceVariant, name.as_str())
                });
                under = MENU_INDENT;
                cy += hh;
            }
            MenuLine::Item(label, offered, chosen) => {
                let look = Look {
                    label: *label,
                    offered,
                    chosen: *chosen,
                    tone: Role::OnSurface,
                    icon: None,
                };
                item(ui, (x, cy, w, ih), &look, under, m.at == Some(k));
                cy += ih;
                k += 1;
            }
            MenuLine::Danger(label, offered) => {
                let look = Look {
                    label: *label,
                    offered,
                    chosen: false,
                    tone: Role::Error,
                    icon: None,
                };
                item(ui, (x, cy, w, ih), &look, under, m.at == Some(k));
                cy += ih;
                k += 1;
            }
            MenuLine::Icon(icon, label, offered) => {
                let look = Look {
                    label: *label,
                    offered,
                    chosen: false,
                    tone: Role::OnSurface,
                    icon: Some(*icon),
                };
                item(ui, (x, cy, w, ih), &look, under, m.at == Some(k));
                cy += ih;
                k += 1;
            }
            MenuLine::Divider => {
                ui.rule(x, cy + MENU_PAD_V, w, Role::OutlineVariant);
                cy += divider_h();
            }
            MenuLine::Off(label, why) => {
                let ix = x + MENU_ITEM_PAD + under;
                let room = (x + w - MENU_ITEM_PAD - ix).max(0.0);
                let fg: Paint = Role::OnSurface.at(DISABLED_CONTENT);
                ui.text_fit_as((ix, cy, ih, room), LABEL_L.regular(), fg, label.as_str());
                let sh = ui.line_as(LABEL_M);
                ui.text_fit_as(
                    (ix, cy + ih - GAP_SMALL, sh, room),
                    LABEL_M,
                    fg,
                    why.as_str(),
                );
                cy += off_h(ui);
            }
        }
    }
    cy - y
}

/// One item at (x, y), `w` by `h`, standing `under` in from its start. An item not offered is
/// drawn faint and takes no press.
fn item<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
    look: &Look<'_, '_, A>,
    under: f32,
    keys: bool,
) {
    ui.offered(look.offered, |ui, press, dimmed| {
        let chosen = look.chosen && !dimmed;
        if chosen {
            ui.fill(x, y, w, h, 0.0, Role::SecondaryContainer);
        }
        let fg: Paint = if dimmed {
            Role::OnSurface.at(DISABLED_CONTENT)
        } else {
            on_selected(chosen, look.tone)
        };
        if let Some(a) = press {
            ui.press_custom((x, y, w, h), a, |ui, a| {
                ui.state_layer((x, y, w, h, 0.0), fg, a);
                if keys || ui.is_focused(a) {
                    // Ringed inside the item: the popup's edge would cut a ring outside it.
                    let r = FOCUS_RING;
                    let rect = ui.rect(x + r / 2.0, y + r / 2.0, w - r, h - r);
                    let color = ui.col(Role::Secondary);
                    ui.c.stroke_rounded_rect(rect, SHAPE_XS * ui.f, r * ui.f, color);
                }
            });
        }
        let mut ix = x + MENU_ITEM_PAD + under;
        if let Some(icon) = look.icon {
            let iy = y + (h - ICON_GLYPH_BUTTON) / 2.0;
            ui.icon(ix, iy, ICON_GLYPH_BUTTON, icon, ICON_GLYPH_BUTTON, fg);
            ix += ICON_GLYPH_BUTTON + ICON_GAP;
        }
        let room = (x + w - MENU_ITEM_PAD - TICK - ix).max(0.0);
        ui.showing(look.label, |ui| {
            ui.text_fit_as((ix, y, h, room), LABEL_L.regular(), fg, look.label.as_str());
        });
        if chosen {
            let tx = x + w - MENU_ITEM_PAD - ICON_GLYPH_BUTTON;
            let ty = y + (h - ICON_GLYPH_BUTTON) / 2.0;
            ui.icon(tx, ty, ICON_GLYPH_BUTTON, Icon::Done, ICON_GLYPH_BUTTON, fg);
        }
    });
}

/// Costs measuring every line. Draws the menu in a frame `w` by `h`: a popup under its anchor on
/// a wide window, a sheet on the small panel. What does not fit scrolls, its edges fading into
/// the menu's own container.
pub fn draw<A: Pressed>(m: &MenuModel<'_, A>, ui: &mut Ui<'_, A>, (w, h): (f32, f32)) {
    if ui.compact {
        let body = |ui: &mut Ui<'_, A>, (x, y, iw): (f32, f32, f32)| {
            let mut cy = y;
            if let Some(t) = m.title {
                let head = Head {
                    icon: m.icon,
                    tone: Role::OnSurface.paint(),
                    title: t,
                };
                cy += sheet::sheet_head_at(ui, (x, cy, iw), &head);
            }
            cy += lines(m, ui, (x, cy, iw)) + GAP;
            cy - y
        };
        let held = sheet::Held {
            foot: &m.cancel,
            beside: m.beside.as_ref(),
            scroll: m.scroll,
        };
        sheet::framed_by(
            &body,
            &held,
            ui,
            (GAP, w - 2.0 * GAP),
            (h, GAP, 2.0 * GRID),
            (false, Role::SurfaceContainerLow),
        );
        return;
    }
    let margin = GAP;
    let pw = width(m, ui).min(w - 2.0 * margin);
    let content = content_h(m, ui, true);
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
    if let Some(a) = &m.beside {
        ui.press_plain((x, y, pw, ph), Plain::Beside, a.clone());
    }
    let max = (content - ph).max(0.0);
    let ph = ph.round();
    if ph <= 0.0 {
        return;
    }
    ui.shadow(x, y, pw, ph, SHAPE_XS, ELEVATION_MENU);
    ui.fill(x, y, pw, ph, SHAPE_XS, Role::SurfaceContainer);
    // At High contrast, where it casts no shadow, its outline sets it apart (`docs/THEMES.md` §9).
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, pw, ph, SHAPE_XS, Role::Outline);
    }
    let clip = ui.rect(x, y, pw, ph);
    let mark = ui.presses_mark();
    ui.region(
        Region::new(m.scroll, clip).on(Role::SurfaceContainer),
        |ui| {
            let mut cy = y + MENU_PAD_V;
            if let Some(t) = m.title {
                let hh = head_h(ui);
                let at = (x + MENU_ITEM_PAD, cy, hh, pw - 2.0 * MENU_ITEM_PAD);
                ui.showing(t, |ui| {
                    ui.text_fit_as(at, LABEL_M, Role::OnSurface, t.as_str());
                });
                cy += hh;
            }
            lines(m, ui, (x, cy, pw));
            max
        },
    );
    // What is scrolled out of the popup takes no press.
    ui.clip_presses_since(mark, clip);
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, pixel};
    use faraday_atoms::{MotionStore, Theme};
    use faraday_draw::Rect;
    use faraday_draw::canvas::InkKind;

    fn item(label: &'static str, press: u8, chosen: bool) -> MenuLine<'static, P> {
        MenuLine::Item(Shown::plain(label), Offered::Yes(P(press)), chosen)
    }

    /// Invariant: A menu's chosen item looks different from the others: it stands on its own fill.
    /// Regression: The chosen item is drawn as the others, so the person cannot tell which is set.
    /// Owner: kit
    #[test]
    fn a_menus_chosen_item_is_told_apart_from_the_others() {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let m = MenuModel {
            title: None,
            icon: Glyph::Icon(Icon::Done),
            lines: vec![item("Move here", 1, true), item("Copy here", 2, false)],
            at: None,
            anchor: None,
            cancel: ButtonsModel::none(),
            beside: None,
            scroll: Anchor {
                scope: "menu",
                n: 1,
            },
        };
        let mut ui = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        draw(&m, &mut ui, (640.0, 480.0));
        let lines: Vec<(String, Rect)> = ui
            .finish()
            .ink
            .expect("the record is on")
            .0
            .into_iter()
            .filter_map(|i| match i.kind {
                InkKind::Text { text, .. } => Some((text.to_string(), i.rect)),
                _ => None,
            })
            .collect();
        let at = |label: &str| lines.iter().find(|(t, _)| t == label).expect("drawn").1;
        let (chosen, other) = (at("Move here"), at("Copy here"));
        let beside = |r: Rect| pixel(&c, (r.x - 4) as usize, (r.y + r.h / 2) as usize);
        assert_ne!(beside(chosen), beside(other));
    }
}
