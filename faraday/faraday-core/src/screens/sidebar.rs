//! The sidebar: the tabs, the wallets and seeds, and what is under way.

use super::SIDEBAR_W;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::Region;
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Glyph, NameSuffix, Style, Ui};
use crate::wallet::{Session, fp_text};
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
const FLOW_ROW: f32 = 30.0;

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
    // The pill under Lock while the idle lock waits for a new wallet's
    // backup takes its room from the lists.
    // The stick's row is shown when a stick is attached or an import
    // waits; with none, "none" is not worth the room the lists need.
    let sticks_row = sticks_row_shown(app);
    let foot_top = h - 124.0 + if sticks_row { 0.0 } else { TOUCH }
        - if held { LOCK_PILL_H + GRID } else { 0.0 };
    lists(app, ui, y + GRID, foot_top);
    foot(app, ui, foot_top, held, sticks_row);
}

/// Whether the foot has the stick's row: a stick is
/// attached, or an import the boot stick brought waits behind it.
fn sticks_row_shown(app: &View) -> bool {
    !app.stick.is_empty() || app.import.state().is_some()
}

/// A row of the lists, a touch target tall: stacked rows leave a target
/// no room to grow.
const ROW: f32 = TOUCH;
/// A group's header's height.
const HEAD: f32 = 26.0;

/// A row of the lists as drawn: its name, the words after it, whether
/// the name is a fingerprint, what it opens, its dot and whether it
/// carries the passphrase tag.
type ListRow = (String, String, bool, Action, Option<crate::ui::Paint>, bool);

/// What is loaded, in two groups, keys and wallets, from just under the
/// last tab (`from`) down to the Files row (`to`): as tall as they are,
/// standing on the Files row, and where they are taller than that room,
/// one region that scrolls within the sidebar by its own offset, the
/// wheel, a trackpad and a finger over it moving it (Material 3
/// Navigation drawer: its content scrolls where it is taller than the
/// drawer). Each row opens what it names. Only the rows in view are
/// worked out and drawn.
fn lists(app: &View, ui: &mut Ui, from: f32, to: f32) {
    let seeds = app.session.keys.len();
    let wallets = app.session.wallets.len();
    let groups = usize::from(seeds > 0) + usize::from(wallets > 0);
    let gap = if groups > 0 { GRID } else { 0.0 };
    let content = groups as f32 * HEAD + (seeds + wallets) as f32 * ROW;
    let bottom = to - gap;
    // The rule above the lists stands under the last tab at the highest.
    let view = content.min((bottom - from - 2.0).max(0.0));
    let top = bottom - view;
    ui.rule(12.0, top - 2.0, SIDEBAR_W - 24.0, Role::OutlineVariant);
    if groups == 0 {
        return;
    }
    let max = content - view;
    let offset = app.scroll.sidebar_offset().clamp(0.0, max);
    // Which of a group's `total` rows, the first drawn at `y0`, are in
    // view: the first and how many.
    let seen = |y0: f32, total: usize| -> (usize, usize) {
        let first = (((top + offset - y0) / ROW).floor().max(0.0) as usize).min(total);
        let end = (((top + offset + view - y0) / ROW).ceil().max(0.0) as usize).min(total);
        (first, end.saturating_sub(first))
    };
    let clip = ui.rect(0.0, top, SIDEBAR_W, view);
    let open = ui.region_open(
        Region::beside(crate::ui::Slot::Sidebar, clip, offset)
            .on(Role::SurfaceContainerLow)
            .from_zero(offset),
    );
    let mark = ui.presses_mark();
    let keys_at = top;
    let wallets_at = keys_at
        + if seeds > 0 {
            HEAD + seeds as f32 * ROW
        } else {
            0.0
        };
    let (first, n) = seen(keys_at + HEAD, seeds);
    let mut seed_rows = app.session.keys.all().skip(first).take(n).map(|k| {
        let fp = fp_text(k.fingerprint);
        // A seed named by its fingerprint is not named twice.
        let name = crate::bip85::key_name(k);
        let label = if name.eq_ignore_ascii_case(&fp) {
            String::new()
        } else {
            name.into_owned()
        };
        let open = Action::ExploreKey(k.fingerprint.0);
        (
            String::from(&*fp),
            label,
            true,
            open,
            None,
            k.has_passphrase,
        )
    });
    let keys = (EN.sidebar_keys, seeds, first);
    group(ui, keys_at, keys, &mut seed_rows);
    // Wallets the seeds here sign for alone first, then those with some
    // of their seeds here, then watch-only ones: the ready ones head the
    // list.
    let mut ranked: Vec<(usize, u8)> = app
        .session
        .wallets
        .iter()
        .enumerate()
        .map(|(i, _)| (i, app.readiness(i)))
        .collect();
    ranked.sort_by_key(|&(i, r)| (std::cmp::Reverse(r), i));
    let (first, n) = seen(wallets_at + HEAD, wallets);
    let mut wallet_rows = ranked
        .into_iter()
        .skip(first)
        .take(n)
        .map(|(i, ready)| wallet_row(app, i, ready));
    let group_w = (EN.sidebar_wallets, wallets, first);
    group(ui, wallets_at, group_w, &mut wallet_rows);
    // What is scrolled out of the lists takes no press.
    ui.clip_presses_since(mark, clip);
    ui.region_close(open, max);
}

/// Wallet `i`'s row, `ready` as `View::readiness` has it.
fn wallet_row(app: &View, i: usize, ready: u8) -> ListRow {
    let w = &app.session.wallets[i];
    let (m, n) = Session::quorum(w);
    // A key not of the wallet spends it alone: no "k of n" (F132).
    let shape = if crate::wallet::another_key_spends(w) {
        EN.sidebar_another_key.to_string()
    } else if n > 1 || w.policy().keys().len() > 1 {
        fill(EN.sidebar_count, &[&m.to_string(), &n.to_string()])
    } else {
        EN.sidebar_single_key.to_string()
    };
    let mark = match ready {
        _ if app.silent_differs(i) => Some(Role::Error.paint()),
        _ if crate::wallet::another_key_spends(w) => Some(Role::Error.paint()),
        2 => Some(Role::Ok.paint()),
        1 => Some(Role::OnSurfaceVariant.paint()),
        _ => None,
    };
    let open = Action::OpenWallet(i);
    (String::from(&*w.name), shape, false, open, mark, false)
}

/// One group at `at`: its header with the count (`title`, `total`),
/// then `rows`, the first of them its row `first`.
fn group(
    ui: &mut Ui,
    at: f32,
    (title, total, first): (&str, usize, usize),
    rows: &mut dyn Iterator<Item = ListRow>,
) {
    if total == 0 {
        return;
    }
    let end = SIDEBAR_W - 24.0;
    ui.text_mid_as((28.0, at, HEAD), LABEL_M, Role::OnSurfaceVariant, title);
    let count = total.to_string();
    ui.text_right_as((end, at, HEAD), LABEL_M, Role::OnSurfaceVariant, &count);
    let rows_w = SIDEBAR_W - 56.0;
    let mut fy = at + HEAD + first as f32 * ROW;
    for (head, tail, mono, action, mark, pass) in rows {
        ui.press_box(16.0, fy, SIDEBAR_W - 32.0, ROW, action);
        // A wallet's dot: green when the seeds here sign for it alone,
        // grey when some of them are here.
        if let Some(c) = mark {
            ui.dot(18.0, fy + ROW / 2.0, 3.5, c);
        }
        let role = if mono { BODY_S.mono() } else { LABEL_M };
        let head = ui.fit_as(role, &head, rows_w * 0.55);
        let hw = ui.text_mid_as((28.0, fy, ROW), role, Role::OnSurface, &head);
        // A key loaded with a passphrase says so at the row's end: the
        // same words without it are another key.
        let tag_w = if pass {
            let t = EN.sidebar_passphrase;
            ui.text_right_as((end, fy, ROW), BODY_S, Role::OnSurfaceVariant, t);
            ui.measure_as(BODY_S, t) + GRID
        } else {
            0.0
        };
        if !tail.is_empty() {
            let tone = match mark {
                Some(c) if c == Role::Ok.paint() => Role::Ok,
                _ => Role::OnSurfaceVariant,
            };
            let tail = ui.fit_as(BODY_S, &tail, rows_w - hw - GRID - tag_w);
            ui.text_mid_as((36.0 + hw, fy, ROW), BODY_S, tone, &tail);
        }
        fy += ROW;
    }
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
