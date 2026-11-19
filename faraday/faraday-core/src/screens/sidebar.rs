//! The sidebar: the tabs, what is under way, the sticks and the foot.

use super::SIDEBAR_W;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Glyph, NameSuffix, Style, Ui};
use crate::{Action, Screen, Sheet};
use faraday_draw::tokens::{BODY_S, BUTTON_H, LABEL_M, LABEL_S};
use faraday_draw::tokens::{GRID, SHAPE_S, TOUCH};
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
        ui.fill(x, y, w, FLOW_ROW, SHAPE_S, Role::SecondaryContainer);
    }
    ui.press(
        Shape::new(x, y, w, FLOW_ROW, SHAPE_S),
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

pub(super) fn sidebar(app: &View, ui: &mut Ui, h: f32) {
    ui.fill(0.0, 0.0, SIDEBAR_W, h, 0.0, Role::SurfaceContainerLow);
    ui.fill(SIDEBAR_W - 1.0, 0.0, 1.0, h, 0.0, Role::OutlineVariant);
    ui.mark(22.0, 20.0, 28.0, 32.0);
    // The name with its word (DEMO, OFFLINE) centred on the mark, its
    // capitals 9 tall, which ends it within the rows' labels' edge.
    ui.wordmark_named(58.0, 31.5, 9.0, Role::OnSurface, name_suffix(app));

    // The places that are always offered, in order; the guide (not a place,
    // a sheet) is drawn between Tools and whatever comes after it.
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
    let flows = app.flows_in_progress();
    let on = app.flow_on_screen();
    let v = app.vaults.open.len();
    // Each tab is one of Material's destinations: icon and label in a
    // row, the current one on a pill (`docs/LAYOUT.md` §3).
    let row = |ui: &mut Ui, y: f32, icon: Glyph, label: &str, active: bool, action: Action| {
        // Vaults carries how many are open, at the row's right end.
        // Files carries how many wait to be written to the stick.
        let open = if icon == Glyph::Icon(Icon::Lock) && v > 0 {
            Some(fill(EN.sidebar_open_n, &[&v.to_string()]))
        } else if icon == Glyph::Icon(Icon::File) && !app.outbox.is_empty() {
            Some(fill(
                EN.sidebar_to_write_n,
                &[&app.outbox.len().to_string()],
            ))
        } else {
            None
        };
        let (x, w) = (12.0, SIDEBAR_W - 24.0);
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
        faraday_ui::nav::destination_row(ui, (x, y, w), &d, open.as_deref());
    };
    for (icon, label, screen) in items.into_iter().chain(tail) {
        // Spend, which has no row of its own, lights Wallets, where it is
        // opened from: Wallets in the sidebar leads back to it.
        let active = crate::nav::destination_of(app.screen) == screen;
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
        // The guide, after Tools: opens the guide sheet, not a screen.
        if screen == Screen::Catalog {
            row(
                ui,
                y,
                Glyph::Icon(Icon::Help),
                EN.sidebar_guide,
                app.sheet == Some(Sheet::Guide) || app.guide_pane_shown(),
                Action::Guide,
            );
            y += TOUCH;
        }
    }

    let held = app.lock_held();
    // The stick's row is shown when a stick is attached or an import
    // waits; with none, "none" is not worth the room.
    let sticks_row = sticks_row_shown(app);
    let foot_top = h - 124.0 + if sticks_row { 0.0 } else { TOUCH }
        - if held { LOCK_PILL_H + GRID } else { 0.0 };
    // The rule above the foot stands under the last tab at the highest.
    ui.rule(
        12.0,
        foot_top.max(y + GRID) - 2.0,
        SIDEBAR_W - 24.0,
        Role::OutlineVariant,
    );
    foot(app, ui, foot_top, held, sticks_row);
}

/// Whether the foot has the stick's row: a stick is
/// attached, or an import the boot stick brought waits behind it.
fn sticks_row_shown(app: &View) -> bool {
    !app.stick.is_empty() || app.import.state().is_some()
}

/// The foot of the sidebar from `fy`: the stick's row (`sticks_row`), Lock and Power off, and under them the pill
/// while the idle lock is `held`.
fn foot(app: &View, ui: &mut Ui, fy: f32, held: bool, sticks_row: bool) {
    let mut fy = fy;
    if sticks_row {
        stick_row(app, ui, fy);
        fy += TOUCH;
    }
    // Room above the buttons, for their targets to reach 48 dp.
    foot_buttons(app, ui, fy + 12.0, held);
}

/// The stick's row at `fy`: what is attached, or the import waiting.
fn stick_row(app: &View, ui: &mut Ui, fy: f32) {
    // With no stick attached, an import the boot stick brought waits
    // behind this row.
    let waiting = app.import.state().is_some() && app.stick.is_empty();
    let sticks = match app.stick.len() {
        0 if waiting => EN.sidebar_import_waiting.to_string(),
        0 => EN.sidebar_none.to_string(),
        1 => String::from(&*app.stick[0].label),
        k => fill(EN.sidebar_attached, &[&k.to_string()]),
    };
    let open = waiting && app.sheet == Some(Sheet::Import);
    if open {
        ui.fill(
            16.0,
            fy,
            SIDEBAR_W - 32.0,
            TOUCH,
            SHAPE_S,
            Role::SecondaryContainer,
        );
    }
    if waiting {
        ui.press_box(16.0, fy, SIDEBAR_W - 32.0, TOUCH, crate::boot_import::OPEN);
    }
    let muted = on_selected(open, Role::OnSurfaceVariant);
    ui.icon(22.0, fy + 14.0, 20.0, app.medium.icon(), 12.0, muted);
    ui.text_mid_as((46.0, fy, TOUCH), BODY_S, muted, app.medium.caps());
    let sticks = ui.fit_as(BODY_S, &sticks, 110.0);
    let tone = on_selected(
        open,
        if waiting {
            Role::Primary
        } else {
            Role::OnSurface
        },
    );
    ui.text_right_as((SIDEBAR_W - 24.0, fy, TOUCH), BODY_S, tone, &sticks);
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

/// Lock and Power off at `fy`, and under them the pill while the idle
/// lock is `held`.
fn foot_buttons(app: &View, ui: &mut Ui, fy: f32, held: bool) {
    // Lock and Power off side by side, each named; Power off alone has
    // the whole width.
    let (x, w) = (16.0, SIDEBAR_W - 32.0);
    if app.holds_secret() {
        let half = (w - GRID) / 2.0;
        ui.button(
            x,
            fy,
            Some(half),
            BUTTON_H,
            EN.sidebar_lock,
            Style::Secondary,
            Action::LockAsk,
        );
        ui.button(
            x + half + GRID,
            fy,
            Some(half),
            BUTTON_H,
            leave_label(app),
            Style::Secondary,
            Action::PowerAsk,
        );
    } else {
        ui.button(
            x,
            fy,
            Some(w),
            BUTTON_H,
            leave_label(app),
            Style::Secondary,
            Action::PowerAsk,
        );
    }
    if held {
        lock_held_pill(ui, 16.0, fy + 44.0, SIDEBAR_W - 32.0);
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
