//! The top app bar (`docs/LAYOUT.md` §3, DESIGN §4.1): `APP_BAR_H`
//! across the page, the way back where the page has one, the page's
//! name, and its actions at the right end, each a touch target.

use crate::pal::*;
use crate::{Glyph, Ui, W};
use faraday_draw::tokens::{
    APP_BAR_H, CAPTION, GAP_SMALL, ICON_SMALL, LABEL, MARGIN_COMPACT, NAV_ICON, TITLE, TOUCH,
};
use faraday_draw::widgets::Icon;

/// What the bar shows at its left.
pub struct AppBar<'s, A> {
    /// The way back, and the name of where it lands, small over the
    /// page's name (F77).
    pub back: Option<(&'s str, A)>,
    /// The page's name.
    pub title: &'s str,
}

/// The bar at the top of the page, `w` wide, its name kept left of
/// `right`, where the actions drawn after it begin.
pub fn app_bar<A: Copy + PartialEq>(ui: &mut Ui<A>, w: f32, bar: &AppBar<A>, right: f32) {
    ui.fill(0.0, 0.0, w, APP_BAR_H, 0.0, SURFACE);
    ui.fill(0.0, APP_BAR_H - 1.0, w, 1.0, 0.0, LINE);
    let mut x = MARGIN_COMPACT;
    let room = |x: f32| (right - x).max(0.0);
    if let Some((label, action)) = bar.back {
        let (bx, by) = (GAP_SMALL, (APP_BAR_H - TOUCH) / 2.0);
        if ui.is_pressed(action) || ui.is_hovered(action) {
            ui.fill(bx, by, TOUCH, TOUCH, TOUCH / 2.0, INNER);
        }
        ui.icon(bx, by, TOUCH, Icon::ChevronLeft, NAV_ICON, TEXT);
        ui.hit_quiet(bx, by, TOUCH, TOUCH, action);
        ui.back(action);
        x = bx + TOUCH;
        let label = ui.fit(CAPTION, W::R, label, room(x));
        let name = ui.fit(TITLE, W::S, bar.title, room(x));
        let lh = CAPTION + GAP_SMALL;
        let top = (APP_BAR_H - lh - TITLE - GAP_SMALL) / 2.0;
        ui.text(x, top, CAPTION, W::R, MUTED, &label);
        ui.text(x, top + lh, TITLE, W::S, TEXT, &name);
    } else {
        let name = ui.fit(TITLE, W::S, bar.title, room(x));
        ui.text_mid(x, 0.0, APP_BAR_H, TITLE, W::S, TEXT, &name);
    }
}

/// An action at the bar's right end, its right edge at `right`: an icon
/// in a touch target, lit while `on`. Returns where the next action
/// ends.
pub fn icon_action<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    right: f32,
    icon: impl Into<Glyph>,
    action: A,
    on: bool,
) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    if on {
        ui.fill(x, y, TOUCH, TOUCH, TOUCH / 2.0, ACCENT.with_alpha(48));
    } else if ui.is_pressed(action) || ui.is_hovered(action) {
        ui.fill(x, y, TOUCH, TOUCH, TOUCH / 2.0, INNER);
    }
    ui.icon(x, y, TOUCH, icon, NAV_ICON, if on { ACCENT } else { MUTED });
    ui.hit_quiet(x, y, TOUCH, TOUCH, action);
    x
}

/// The ? that opens what explains the page, its right edge at `right`.
/// Returns where the next action ends.
pub fn help<A: Copy + PartialEq>(ui: &mut Ui<A>, right: f32, action: A) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    let d = ICON_SMALL + GAP_SMALL * 2.0;
    let (cx, cy) = (x + (TOUCH - d) / 2.0, y + (TOUCH - d) / 2.0);
    let bg = if ui.is_pressed(action) || ui.is_hovered(action) {
        INNER
    } else {
        SURFACE
    };
    ui.fill(cx, cy, d, d, d / 2.0, bg);
    ui.stroke(cx, cy, d, d, d / 2.0, BORDER);
    let tw = ui.measure(LABEL, W::S, "?");
    ui.text_mid(cx + (d - tw) / 2.0, cy, d, LABEL, W::S, MUTED, "?");
    ui.hit_quiet(x, y, TOUCH, TOUCH, action);
    x
}

/// The ⋮ that opens the page's menu, its right edge at `right`. Returns
/// where the next action ends.
pub fn more<A: Copy + PartialEq>(ui: &mut Ui<A>, right: f32, action: A) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    if ui.is_pressed(action) || ui.is_hovered(action) {
        ui.fill(x, y, TOUCH, TOUCH, TOUCH / 2.0, INNER);
    }
    let (cx, cy) = (x + TOUCH / 2.0, y + TOUCH / 2.0);
    let r = GAP_SMALL / 2.0;
    for k in [-1.0f32, 0.0, 1.0] {
        ui.dot(cx, cy + k * (GAP_SMALL + r * 2.0), r, MUTED);
    }
    ui.hit_quiet(x, y, TOUCH, TOUCH, action);
    x
}
