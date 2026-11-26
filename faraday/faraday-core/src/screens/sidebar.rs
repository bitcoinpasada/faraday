//! The sidebar: the tabs, what is under way, the sticks and, at its foot, what holds the idle lock.

use super::SIDEBAR_W;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Glyph, NameSuffix, Ui};
use crate::{Action, Screen};
use faraday_draw::tokens::{FAB_H, FAB_PAD, ICON_GAP, ICON_GLYPH, LABEL_L};
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
    let lit = crate::nav::lit_with_flow(app);
    // Each tab is one of Material's destinations: icon and label in a
    // row, the current one on a pill (`docs/LAYOUT.md` §3).
    // The flows in progress under a tab are its second badge's number; a
    // press on the tab is `Action::Nav` as for every tab.
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
        let n = match action {
            Action::Nav(s) => flows.iter().filter(|r| r.tab == s).count(),
            _ => 0,
        };
        faraday_ui::nav::destination_row(ui, (x, y, w), &d, None, n);
    };
    for (icon, label, screen) in items.into_iter().chain(tail) {
        // A flow lights the tab it belongs to, Spend from a backup the one
        // it was opened from (Home or Wallets).
        row(ui, y, icon, label, lit == screen, Action::Nav(screen));
        y += TOUCH;
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
        lock_held_tag(ui, 16.0, fab_y - LOCK_TAG_H - GRID, SIDEBAR_W - 32.0);
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

/// The tag's height.
pub(crate) const LOCK_TAG_H: f32 = faraday_draw::tokens::TAG_H;

/// "Auto-lock off until backup complete" as a plain neutral tag at (x, y),
/// `w` wide at most: a fact, so no outline and no accent (F332). Returns its
/// height.
pub(crate) fn lock_held_tag(ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let room = w - ui.chip_width("");
    let text = ui.fit_as(faraday_draw::tokens::BODY_S, EN.lock_held_pill, room);
    ui.tag(x, y, &text, faraday_ui::Status::Neutral);
    LOCK_TAG_H
}
