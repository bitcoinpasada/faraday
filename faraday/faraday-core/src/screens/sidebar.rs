//! The sidebar: the tabs, what is under way, the sticks and, at its foot, what holds the idle lock.

use super::SIDEBAR_W;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Glyph, NameSuffix, Ui};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    BODY_S, FAB_H, FAB_PAD, ICON_GAP, ICON_GLYPH, LABEL_L, LABEL_M, LABEL_S,
};
use faraday_draw::tokens::{GRID, TOUCH};
use faraday_draw::widgets::Icon;

/// The word after the wordmark: DEMO in the desktop app, OFFLINE on the
/// device (the stick and the Pi).
pub(crate) fn name_suffix(app: &View) -> NameSuffix {
    if app.online {
        NameSuffix::Demo
    } else {
        NameSuffix::Offline
    }
}

/// A flow in progress's row's height in the sidebar.
const FLOW_ROW: f32 = faraday_draw::tokens::TOUCH;

/// A flow in progress's row, indented under its tab: its name and, at
/// the right end in the muted tone, its open step; lit while the flow is
/// on screen (`docs/FLOWS.md`, Flows in progress).
fn flow_row(ui: &mut Ui, y: f32, r: &crate::in_progress::Row, lit: bool) {
    let action = Action::Resume(r.going);
    let (x, w) = (46.0, SIDEBAR_W - 58.0);
    if lit {
        ui.fill(x, y, w, FLOW_ROW, FLOW_ROW / 2.0, Role::SecondaryContainer);
    }
    ui.press(
        Shape::new(x, y, w, FLOW_ROW, FLOW_ROW / 2.0),
        Role::OnSurface.paint(),
        action,
    );
    let (tx, right) = (54.0, SIDEBAR_W - 24.0);
    let room = right - tx;
    // The name keeps what it needs up to a little over half the row; the
    // step has the rest.
    let nw = ui.measure_as(LABEL_M, &r.name).min(room * 0.6);
    let step = ui.fit_as(BODY_S, &r.step, room - nw - 8.0);
    let sw = ui.measure_as(BODY_S, &step);
    let name = ui.fit_as(LABEL_M, &r.name, room - sw - 8.0);
    let fg = crate::ui::pal::on_selected(lit, Role::OnSurfaceVariant);
    ui.text_mid_as((tx, y, FLOW_ROW), LABEL_M, fg, &name);
    ui.text_right_as((right, y, FLOW_ROW), BODY_S, fg, &step);
}

/// The shield's box at the sidebar's top left, and the margin the shield
/// leaves inside it on every side: Scan's FAB stands on the shield's left
/// edge and as far above the foot as the shield's top is from the top.
const MARK_X: f32 = 24.0;
const MARK_TOP: f32 = 20.0;
const MARK_W: f32 = 24.0;
const MARK_H: f32 = 32.0;
const MARK_MARGIN: f32 = 2.0;

pub(super) fn sidebar(app: &View, ui: &mut Ui, h: f32) {
    ui.fill(0.0, 0.0, SIDEBAR_W, h, 0.0, Role::SurfaceContainerLow);
    ui.fill(SIDEBAR_W - 1.0, 0.0, 1.0, h, 0.0, Role::OutlineVariant);
    ui.mark(MARK_X, MARK_TOP, MARK_W, MARK_H);
    // The name with its word (DEMO, OFFLINE) centred on the mark, its
    // capitals 9 tall, which ends it within the rows' labels' edge.
    ui.wordmark_named(58.0, 31.5, 9.0, Role::OnSurface, name_suffix(app));

    // The places that are always offered, in order; the guide is the top
    // app bar's ?.
    let items = [
        (Glyph::Icon(Icon::House), EN.sidebar_home, Screen::Home),
        (Glyph::Icon(Icon::Verified), EN.sidebar_check, Screen::Check),
        (
            Glyph::Icon(Icon::Wallet),
            EN.sidebar_wallets,
            Screen::WalletsStart,
        ),
        (Glyph::Icon(Icon::File), EN.sidebar_files, Screen::Files),
        (Glyph::Icon(Icon::Lock), EN.sidebar_vaults, Screen::Vaults),
        (Glyph::Icon(Icon::Tools), EN.sidebar_tools, Screen::Catalog),
    ];
    #[cfg_attr(not(feature = "demo"), expect(unused_mut))]
    let mut tail = vec![(
        Glyph::Icon(Icon::Settings),
        EN.sidebar_settings,
        Screen::Settings,
    )];
    // The online app is the device's QR link, above Settings.
    #[cfg(feature = "demo")]
    if app.online {
        tail.insert(
            0,
            (Glyph::Icon(Icon::Qr), EN.sidebar_transfer, Screen::Transfer),
        );
    }
    // The tabs are a touch target tall each, from just under the name.
    let mut y = 64.0;
    let (x, w) = (12.0, SIDEBAR_W - 24.0);
    let flows = app.flows_in_progress();
    let on = app.flow_on_screen();
    // Each tab is one of Material's destinations: icon and label in a
    // row, the current one on a pill (`docs/LAYOUT.md` §3).
    let row = |ui: &mut Ui, y: f32, icon: Glyph, label: &str, active: bool, action: Action| {
        let d = faraday_ui::nav::Dest {
            icon,
            label,
            action,
            current: active,
            badge: match action {
                Action::Nav(s) => crate::nav::screen::badge_of(app, s),
                _ => faraday_ui::nav::Badge::None,
            },
        };
        faraday_ui::nav::destination_row(ui, (x, y, w), &d, None);
    };
    for (icon, label, screen) in items.into_iter().chain(tail) {
        // A flow lights the tab it belongs to, Spend from a backup the one
        // it was opened from (Home or Wallets).
        let active = crate::nav::lit_destination(app) == screen;
        // A flow in progress on screen lights its own row, not its tab.
        let mine = || flows.iter().filter(move |r| r.tab == screen);
        let row_lit = mine().any(|r| Some(r.going) == on);
        row(ui, y, icon, label, active && !row_lit, Action::Nav(screen));
        y += TOUCH;
        for r in mine() {
            flow_row(ui, y, r, Some(r.going) == on);
            y += FLOW_ROW + 2.0;
        }
        if mine().next().is_some() {
            y += 4.0;
        }
    }

    // Lock and Power off are the top app bar's power button's; what holds
    // the idle lock stands at the sidebar's foot.
    // Scan is the extended FAB at the foot, as wide as its icon and word,
    // fixed there on every page, its left edge on the shield's and as far
    // from the sidebar's foot as the shield is from its top; the pill
    // stands above it.
    let fab_y = h - FAB_H - MARK_TOP - MARK_MARGIN;
    let fab_w = ui.measure_as(LABEL_L, EN.home_scan) + 2.0 * FAB_PAD + ICON_GLYPH + ICON_GAP;
    ui.fab_top(
        (MARK_X + MARK_MARGIN, fab_y),
        Some(fab_w),
        Icon::Scan,
        EN.home_scan,
        Action::Scan,
    );
    if app.lock_held() {
        lock_held_pill(ui, 16.0, fab_y - LOCK_PILL_H - GRID, SIDEBAR_W - 32.0);
    }
}

/// What the power button reads: Quit on the online desktop build, which
/// closes the window, Power off on a device.
pub(crate) fn leave_label(app: &View) -> &'static str {
    if app.online {
        EN.sidebar_quit
    } else {
        EN.nav_power_off
    }
}

/// The pill's height.
pub(crate) const LOCK_PILL_H: f32 = 24.0;

/// "Auto-lock off until backup complete" as an outlined pill at (x, y),
/// `w` wide at most. Returns its height.
pub(crate) fn lock_held_pill(ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let (role, pad) = (LABEL_S.regular(), 8.0);
    let text = ui.fit_as(role, EN.lock_held_pill, w - 2.0 * pad);
    let pw = (ui.measure_as(role, &text) + 2.0 * pad).min(w);
    let r = LOCK_PILL_H / 2.0;
    ui.fill(x, y, pw, LOCK_PILL_H, r, Role::SurfaceContainerLow);
    ui.stroke(x, y, pw, LOCK_PILL_H, r, Role::Primary);
    ui.text_mid_as((x + pad, y, LOCK_PILL_H), role, Role::Primary, &text);
    LOCK_PILL_H
}
