//! The top app bar (`docs/LAYOUT.md` §3, DESIGN §4.1): `APP_BAR_H`
//! across the page, the way back where the page has one, the page's
//! name, and its actions at the right end, each a touch target.

use crate::pal::*;
use crate::{Glyph, Ui, W};
use faraday_draw::tokens::{
    APP_BAR_H, BODY_S, GAP_SMALL, ICON_SMALL, LABEL, MARGIN_COMPACT, NAV_ICON, TITLE_L, TOUCH,
};
use faraday_draw::widgets::Icon;

/// What the bar shows at its left.
pub struct AppBar<'s, A> {
    /// The way back, and the name of where it lands, small over the
    /// page's name (F77).
    pub back: Option<(&'s str, A)>,
    /// The page's name.
    pub title: &'s str,
    /// What the way back's tooltip says.
    pub back_tip: &'static str,
}

/// The bar at the top of the page, `w` wide, its name kept left of
/// `right`, where the actions drawn after it begin.
pub fn app_bar<A: Copy + PartialEq>(ui: &mut Ui<A>, w: f32, bar: &AppBar<A>, right: f32) {
    ui.fill(0.0, 0.0, w, APP_BAR_H, 0.0, Role::Surface);
    ui.fill(0.0, APP_BAR_H - 1.0, w, 1.0, 0.0, Role::OutlineVariant);
    let mut x = MARGIN_COMPACT;
    let room = |x: f32| (right - x).max(0.0);
    if let Some((label, action)) = bar.back {
        let (bx, by) = (GAP_SMALL, (APP_BAR_H - TOUCH) / 2.0);
        ui.state_layer(
            (bx, by, TOUCH, TOUCH, TOUCH / 2.0),
            Role::OnSurfaceVariant.paint(),
            action,
        );
        ui.icon(
            bx,
            by,
            TOUCH,
            Icon::ChevronLeft,
            NAV_ICON,
            Role::OnSurfaceVariant,
        );
        ui.hit_quiet(bx, by, TOUCH, TOUCH, action);
        ui.tip_plain((bx, by, TOUCH, TOUCH), action, bar.back_tip);
        ui.back(action);
        x = bx + TOUCH;
        let label = ui.fit_as(BODY_S, label, room(x));
        let name = ui.fit_as(TITLE_L, bar.title, room(x));
        let (lh, th) = (ui.line_as(BODY_S), ui.line_as(TITLE_L));
        let top = (APP_BAR_H - lh - th) / 2.0;
        ui.text_mid_as((x, top, lh), BODY_S, Role::OnSurfaceVariant, &label);
        ui.text_mid_as((x, top + lh, th), TITLE_L, Role::OnSurface, &name);
    } else {
        let name = ui.fit_as(TITLE_L, bar.title, room(x));
        ui.text_mid_as((x, 0.0, APP_BAR_H), TITLE_L, Role::OnSurface, &name);
    }
}

/// An action at the bar's right end, its right edge at `right`: an icon
/// in a touch target, lit while `on`, named `label` by its tooltip.
/// Returns where the next action ends.
pub fn icon_action<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    right: f32,
    icon: impl Into<Glyph>,
    action: A,
    on: bool,
    label: &'static str,
) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    // Lit: a toggled icon button's selected state, its container
    // secondary-container and its icon on it.
    if on {
        ui.fill(x, y, TOUCH, TOUCH, TOUCH / 2.0, Role::SecondaryContainer);
    }
    let fg = if on {
        Role::OnSecondaryContainer
    } else {
        Role::OnSurfaceVariant
    };
    ui.state_layer((x, y, TOUCH, TOUCH, TOUCH / 2.0), fg.paint(), action);
    ui.icon(x, y, TOUCH, icon, NAV_ICON, fg);
    ui.hit_quiet(x, y, TOUCH, TOUCH, action);
    ui.tip_plain((x, y, TOUCH, TOUCH), action, label);
    x
}

/// The ? that opens what explains the page, its right edge at `right`,
/// named `label` by its tooltip. Returns where the next action ends.
pub fn help<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    right: f32,
    action: A,
    label: &'static str,
) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    let d = ICON_SMALL + GAP_SMALL * 2.0;
    let (cx, cy) = (x + (TOUCH - d) / 2.0, y + (TOUCH - d) / 2.0);
    ui.state_layer(
        (cx, cy, d, d, d / 2.0),
        Role::OnSurfaceVariant.paint(),
        action,
    );
    ui.stroke(cx, cy, d, d, d / 2.0, Role::Outline);
    let tw = ui.measure(LABEL, W::S, "?");
    ui.text_mid(
        cx + (d - tw) / 2.0,
        cy,
        d,
        LABEL,
        W::S,
        Role::OnSurfaceVariant,
        "?",
    );
    ui.hit_quiet(x, y, TOUCH, TOUCH, action);
    ui.tip_plain((x, y, TOUCH, TOUCH), action, label);
    x
}

/// The ⋮ that opens the page's menu, its right edge at `right`, named
/// `label` by its tooltip. Returns where the next action ends.
pub fn more<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    right: f32,
    action: A,
    label: &'static str,
) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    ui.state_layer(
        (x, y, TOUCH, TOUCH, TOUCH / 2.0),
        Role::OnSurfaceVariant.paint(),
        action,
    );
    let (cx, cy) = (x + TOUCH / 2.0, y + TOUCH / 2.0);
    let r = GAP_SMALL / 2.0;
    for k in [-1.0f32, 0.0, 1.0] {
        ui.dot(
            cx,
            cy + k * (GAP_SMALL + r * 2.0),
            r,
            Role::OnSurfaceVariant,
        );
    }
    ui.hit_quiet(x, y, TOUCH, TOUCH, action);
    ui.tip_plain((x, y, TOUCH, TOUCH), action, label);
    x
}
