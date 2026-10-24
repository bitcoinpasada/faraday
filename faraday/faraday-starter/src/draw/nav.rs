//! The frame's navigation (`docs/STARTER.md` §3.1d): the top app bar, the
//! navigation rail (medium and expanded windows) or bar (compact), and
//! the ⋮ menu, drawn with `faraday-ui`'s components (DESIGN §4.1).

use super::*;
use faraday_draw::tokens::{
    APP_BAR_H, CAPTION, ELEVATION_RAISED, GAP, GAP_SMALL, GRID, ICON_SMALL, LABEL, MARGIN_COMPACT,
    NAV_BAR_H, PAD, SHAPE_S, TOUCH,
};
use faraday_ui::nav::{self as fnav, AppBar, Rail};

/// The height of a top app bar's own button.
const BAR_BUTTON_H: f32 = 40.0;
/// The ⋮ menu's width.
const MENU_W: f32 = 280.0;

/// The four destinations, the one shown lit.
fn destinations(starter: &Starter) -> [fnav::Dest<'static, Action>; 4] {
    let at = starter.on_destination();
    let one =
        |icon: Glyph, label: &'static str, action: StarterAction, here: Destination| fnav::Dest {
            icon,
            label,
            action: action.host(),
            current: at == Some(here),
        };
    [
        one(
            Icon::House.into(),
            S.starter_home,
            StarterAction::Home,
            Destination::Home,
        ),
        one(
            Glyph::Stick,
            S.starter_nav_make,
            StarterAction::MakePage,
            Destination::Make,
        ),
        one(
            Icon::Qr.into(),
            S.starter_nav_tools,
            StarterAction::ToolsPage,
            Destination::Tools,
        ),
        one(
            Icon::Learn.into(),
            S.starter_row_learn,
            StarterAction::Learn,
            Destination::Learn,
        ),
    ]
}

/// The rail down the window's left, `h` tall.
pub(super) fn rail(starter: &Starter, ui: &mut Ui, h: f32) {
    fnav::rail(
        ui,
        h,
        &Rail {
            top: &destinations(starter),
            under: &[],
            foot: &[],
        },
    );
}

/// The bar across the window's foot, `w` wide, in a window `h` tall.
pub(super) fn bottom_bar(starter: &Starter, ui: &mut Ui, h: f32, w: f32) {
    fnav::bar(ui, h - NAV_BAR_H, w, &destinations(starter));
}

/// The install action the bar carries while it applies (§3.5): Install
/// while nothing is installed, Update while the running copy is newer.
fn install_action(starter: &Starter) -> Option<(&'static str, StarterAction)> {
    match &starter.installed {
        None => Some((S.starter_row_install, StarterAction::Install)),
        Some(_) if starter.update_offered() => Some((S.starter_bar_update, StarterAction::Install)),
        Some(_) => None,
    }
}

/// The rows of the ⋮ menu: what is not a destination (§3.1d).
fn menu_rows(starter: &Starter) -> Vec<(&'static str, StarterAction)> {
    let mut rows = vec![(S.starter_row_about, StarterAction::About)];
    if starter.installed.is_some() {
        rows.push((S.starter_uninstall, StarterAction::Installed));
    }
    if starter.entry_offered() {
        rows.push((S.starter_remove_entry, StarterAction::RemoveEntry));
    }
    rows
}

/// The top app bar across the page beside the rail, `aw` wide, the rail
/// `rail_w` to its left: on Home the wordmark, on another destination
/// its name, on a task page the way back and the page's name; at the
/// right end of a destination's the install action and the ⋮ menu.
pub(super) fn top_bar(starter: &Starter, ui: &mut Ui, rail_w: f32, aw: f32) {
    let dest = starter.on_destination();
    ui.ox = rail_w;
    let install = dest.and(install_action(starter));
    let button_w = install.map(|(label, _)| ui.measure(LABEL, W::S, label) + 2.0 * PAD);
    let mut right = aw - GAP_SMALL;
    let mut ends = right;
    if dest.is_some() {
        ends -= TOUCH;
    }
    if let Some(bw) = button_w {
        ends -= bw + GAP;
    }
    let name = page_name(starter);
    let back = back_name(starter);
    let back = back
        .as_deref()
        .map(|label| (label, StarterAction::Back.host()));
    fnav::app_bar(ui, aw, &AppBar { back, title: &name }, ends);
    if dest == Some(Destination::Home) {
        // The mark and the name with Starter beside it (F16).
        let (mw, mh) = (ICON_SMALL, ICON_SMALL + GAP_SMALL);
        ui.mark(MARGIN_COMPACT, (APP_BAR_H - mh) / 2.0, mw, mh);
        let nx = MARGIN_COMPACT + mw + GRID;
        // A compact window's name is a size smaller, to leave the install
        // action room beside it.
        let cap = if starter.compact() {
            CAPTION - GAP / 4.0
        } else {
            CAPTION
        };
        ui.wordmark_named(nx, (APP_BAR_H - cap) / 2.0, cap, TEXT, NameSuffix::Starter);
    }
    if dest.is_some() {
        right = fnav::more(ui, right, StarterAction::Menu.host());
        if let (Some((label, action)), Some(bw)) = (install, button_w) {
            ui.button(
                right - GAP - bw,
                (APP_BAR_H - BAR_BUTTON_H) / 2.0,
                None,
                BAR_BUTTON_H,
                label,
                Style::Secondary,
                action.host(),
            );
        }
    }
    ui.ox = 0.0;
}

/// The ⋮ menu under the bar's ⋮ at the window's right: a popup with a
/// row for each of [`menu_rows`]; a press outside it closes it.
pub(super) fn menu(starter: &Starter, ui: &mut Ui, w: f32) {
    let rows = menu_rows(starter);
    let ph = rows.len() as f32 * TOUCH + GAP;
    let (px, py) = (w - GAP_SMALL - MENU_W, APP_BAR_H - GAP_SMALL);
    ui.hit_around(px, py, MENU_W, ph, StarterAction::CloseMenu.host());
    ui.shadow(px, py, MENU_W, ph, SHAPE_S, ELEVATION_RAISED);
    ui.fill(px, py, MENU_W, ph, SHAPE_S, SURFACE);
    ui.stroke(px, py, MENU_W, ph, SHAPE_S, OUTLINE);
    for (i, (label, action)) in rows.into_iter().enumerate() {
        let ry = py + GAP / 2.0 + i as f32 * TOUCH;
        let a = action.host();
        ui.state_layer((px, ry, MENU_W, TOUCH, SHAPE_S), TEXT, a);
        ui.text_mid(px + PAD, ry, TOUCH, LABEL, W::R, TEXT, label);
        ui.hit_quiet(px, ry, MENU_W, TOUCH, a);
    }
}
