//! The top app bar (`docs/LAYOUT.md` §3, DESIGN §4.1): Material 3's
//! small top app bar, `APP_BAR_H` across the page, the way back where
//! the page has one, the page's name in title large with where Back goes
//! as its subtitle under it in label medium (Material's `AppBarSmallTokens`),
//! and its actions at the right end, each a touch target.

use crate::pal::*;
use crate::{Glyph, Ui, W};
use faraday_draw::tokens::{
    APP_BAR_H, GAP_SMALL, ICON_SMALL, LABEL, LABEL_M, MARGIN_COMPACT, NAV_ICON, TITLE_L, TITLE_M,
    TOUCH, TypeRole,
};
use faraday_draw::widgets::Icon;

/// What the bar shows at its left.
pub struct AppBar<'s, A> {
    /// The way back, and the name of where it lands, the subtitle under
    /// the page's name (F77).
    pub back: Option<(&'s str, A)>,
    /// The page's name.
    pub title: &'s str,
    /// What the way back's tooltip says.
    pub back_tip: &'static str,
}

/// The bar at the top of the page, `w` wide, its name kept left of
/// `right`, where the actions drawn after it begin. Surface, and
/// surface-container once the page is scrolled under it
/// ([`app_bar_fill`]); no rule under it (Material's top app bar).
pub fn app_bar<A: Copy + PartialEq>(ui: &mut Ui<A>, w: f32, bar: &AppBar<A>, right: f32) {
    let fill = app_bar_fill(ui.offset > 0.0 && !ui.in_sheet);
    ui.fill(0.0, 0.0, w, APP_BAR_H, 0.0, fill);
    let mut x = MARGIN_COMPACT;
    let room = |x: f32| (right - x).max(0.0);
    if let Some((label, action)) = bar.back {
        let (bx, by) = (GAP_SMALL, (APP_BAR_H - TOUCH) / 2.0);
        let round = (bx, by, TOUCH, TOUCH, TOUCH / 2.0);
        ui.state_layer(round, Role::OnSurfaceVariant.paint(), action);
        ui.icon(
            bx,
            by,
            TOUCH,
            Icon::ChevronLeft,
            NAV_ICON,
            Role::OnSurfaceVariant,
        );
        ui.hit_ringed(round, (bx, by, TOUCH, TOUCH), action);
        ui.tip_plain((bx, by, TOUCH, TOUCH), action, bar.back_tip);
        ui.back(action);
        x = bx + TOUCH;
        let (role, name) = bar_title(ui, bar.title, room(x));
        let label = ui.fit_as(LABEL_M, label, room(x));
        let (th, lh) = (ui.line_as(role), ui.line_as(LABEL_M));
        let top = (APP_BAR_H - th - lh) / 2.0;
        ui.text_mid_as((x, top, th), role, Role::OnSurface, &name);
        ui.text_mid_as((x, top + th, lh), LABEL_M, Role::OnSurfaceVariant, &label);
    } else {
        let (role, name) = bar_title(ui, bar.title, room(x));
        ui.text_mid_as((x, 0.0, APP_BAR_H), role, Role::OnSurface, &name);
    }
}

/// The top app bar's container (`docs/THEMES.md` §7): surface, and
/// surface-container while the page is `scrolled` under it.
pub fn app_bar_fill(scrolled: bool) -> Role {
    if scrolled {
        Role::SurfaceContainer
    } else {
        Role::Surface
    }
}

/// The page's name as the bar shows it in `room` beside its actions
/// (`docs/DESIGN.md` §4.1): whole in title large, Material's role for
/// the bar; else whole in title medium; else in title medium shortened
/// at a word, its first word at least, with an ellipsis; else left off
/// (empty), where no whole word would show. Cheap: a few widths measured.
fn bar_title<A: Copy + PartialEq>(ui: &Ui<A>, title: &str, room: f32) -> (TypeRole, String) {
    for role in [TITLE_L, TITLE_M] {
        if ui.measure_as(role, title) <= room {
            return (role, title.to_string());
        }
    }
    let words: Vec<&str> = title.split_whitespace().collect();
    for k in (1..words.len()).rev() {
        let short = format!("{}…", words[..k].join(" "));
        if ui.measure_as(TITLE_M, &short) <= room {
            return (TITLE_M, short);
        }
    }
    (TITLE_M, String::new())
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
    let round = (x, y, TOUCH, TOUCH, TOUCH / 2.0);
    ui.state_layer(round, fg.paint(), action);
    ui.icon(x, y, TOUCH, icon, NAV_ICON, fg);
    ui.hit_ringed(round, (x, y, TOUCH, TOUCH), action);
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
    let round = (cx, cy, d, d, d / 2.0);
    ui.state_layer(round, Role::OnSurfaceVariant.paint(), action);
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
    ui.hit_ringed(round, (x, y, TOUCH, TOUCH), action);
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
    let round = (x, y, TOUCH, TOUCH, TOUCH / 2.0);
    ui.state_layer(round, Role::OnSurfaceVariant.paint(), action);
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
    ui.hit_ringed(round, (x, y, TOUCH, TOUCH), action);
    ui.tip_plain((x, y, TOUCH, TOUCH), action, label);
    x
}
