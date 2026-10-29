//! The frame's navigation (`docs/STARTER.md` §3.1d): the top app bar, the
//! navigation rail (medium and expanded windows) or bar (compact), and
//! the ⋮ menu, drawn with `faraday-ui`'s components (DESIGN §4.1).

use super::*;
use faraday_draw::tokens::{
    APP_BAR_H, BUTTON_H, GAP, GAP_SMALL, GRID, ICON_SMALL, LABEL_L, LABEL_M, LABEL_S,
    MARGIN_COMPACT, NAV_BAR_H, PAD, TOUCH,
};
use faraday_ui::menu::{self as fmenu, Menu, MenuLine};
use faraday_ui::nav::{self as fnav, AppBar, Rail};

/// The four destinations, the one shown lit.
fn destinations(starter: &Starter) -> [fnav::Dest<'static, Action>; 4] {
    let at = starter.on_destination();
    let one =
        |icon: Glyph, label: &'static str, action: StarterAction, here: Destination| fnav::Dest {
            icon,
            label,
            action: action.host(),
            current: at == Some(here),
            badge: fnav::Badge::None,
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
    let mut rows = vec![
        (S.starter_row_appearance, StarterAction::Appearance),
        (S.starter_row_about, StarterAction::About),
    ];
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
    let button_w = install.map(|(label, _)| ui.measure_as(LABEL_L, label) + 2.0 * PAD);
    let mut right = aw - GAP_SMALL;
    let mut ends = right;
    if dest.is_some() {
        ends -= TOUCH;
    }
    if let Some(bw) = button_w {
        ends -= bw + GAP;
    }
    // **How Faraday does it**, at the bar's end on a task page (which
    // has no ⋮) and left of the ⋮ on a destination.
    let how = starter.how_offered();
    if how {
        ends -= TOUCH;
    }
    let name = page_name(starter);
    let back = back_name(starter);
    let back = back
        .as_deref()
        .map(|label| (label, StarterAction::Back.host()));
    fnav::app_bar(
        ui,
        aw,
        &AppBar {
            back,
            title: &name,
            back_tip: S.tip_back,
        },
        ends,
    );
    if dest == Some(Destination::Home) {
        // The mark and the name with Starter beside it (F16).
        let (mw, mh) = (ICON_SMALL, ICON_SMALL + GAP_SMALL);
        ui.mark(MARGIN_COMPACT, (APP_BAR_H - mh) / 2.0, mw, mh);
        let nx = MARGIN_COMPACT + mw + GRID;
        // A compact window's name is a size smaller, to leave the install
        // action room beside it.
        let cap = if starter.compact() {
            LABEL_S.size
        } else {
            LABEL_M.size
        };
        // Where the name and Starter do not fit before the install action
        // (a narrow window, larger text), Starter under the name, made
        // smaller; where that does not fit either, the mark alone.
        let at = (nx, (APP_BAR_H - cap) / 2.0, ends - nx);
        ui.wordmark_fitted(at, cap, Role::OnSurface, NameSuffix::Starter);
    }
    if dest.is_some() {
        right = fnav::more(ui, right, StarterAction::Menu.host(), S.tip_more);
        if let (Some((label, action)), Some(bw)) = (install, button_w) {
            ui.button(
                right - GAP - bw,
                (APP_BAR_H - BUTTON_H) / 2.0,
                None,
                BUTTON_H,
                label,
                Style::Secondary,
                action.host(),
            );
        }
    }
    if how {
        fnav::icon_action(
            ui,
            right,
            Icon::Method,
            StarterAction::How.host(),
            false,
            S.tip_how,
        );
    }
    ui.ox = 0.0;
}

/// Costs measuring its rows. The ⋮ menu: Material's menu
/// (`faraday_ui::menu`) under the bar's ⋮ at the window's right, a row
/// for each of [`menu_rows`], in a frame `w` by `h`; a press outside it
/// closes it. A popup at every window size: the Starter is a desktop
/// app, with no small panel's sheet.
pub(super) fn menu(starter: &Starter, ui: &mut Ui, w: f32, h: f32) {
    let rows = menu_rows(starter);
    let lines: Vec<MenuLine<'_, Action>> = rows
        .iter()
        .map(|&(label, action)| MenuLine::Item(label, action.host(), false))
        .collect();
    // The ⋮ as the top bar draws it ([`top_bar`]'s first `right`).
    let (mx, my) = (w - GAP_SMALL - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    let anchor = ui.rect(mx, my, TOUCH, TOUCH);
    let m = Menu {
        title: None,
        icon: Glyph::Icon(Icon::List),
        lines: &lines,
        at: None,
        anchor: Some((anchor.x, anchor.y, anchor.w, anchor.h)),
        cancel: (S.starter_cancel, StarterAction::CloseMenu.host()),
        shown: 1.0,
    };
    let kept = (ui.compact, ui.outside, ui.offset, ui.ox, ui.oy);
    ui.compact = false;
    ui.outside = Some(StarterAction::CloseMenu.host());
    ui.offset = 0.0;
    (ui.ox, ui.oy) = (0.0, 0.0);
    fmenu::menu(ui, w, h, &m);
    (ui.compact, ui.outside, ui.offset, ui.ox, ui.oy) = kept;
}
