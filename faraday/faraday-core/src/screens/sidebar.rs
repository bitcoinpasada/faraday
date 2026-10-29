//! The sidebar: the tabs, the wallets and seeds, and what is under way.

use super::SIDEBAR_W;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Glyph, NameSuffix, Style, Ui};
use crate::wallet::{Session, fp_text};
use crate::{Action, Screen, Sheet};
use faraday_draw::tokens::{BODY_S, BUTTON_H, LABEL_M, LABEL_S};
use faraday_draw::tokens::{SHAPE_S, TOUCH};
use faraday_draw::widgets::Icon;

/// How many rows of seeds and of wallets the sidebar shows, in `rows`:
/// all of both when they fit, else the room shared, the smaller group
/// keeping all it needs when that leaves the other at least half. A
/// group cut short gives its last row to "+ N more".
fn sidebar_caps(seeds: usize, wallets: usize, rows: usize) -> (usize, usize) {
    if seeds + wallets <= rows {
        return (seeds, wallets);
    }
    let half = rows / 2;
    let cap_s = seeds.min(half.max(rows.saturating_sub(wallets)));
    let cap_w = wallets.min(rows - cap_s);
    // What wallets leave unused goes back to seeds.
    let cap_s = seeds.min(rows - cap_w);
    (cap_s, cap_w)
}

/// The word after the wordmark: the desktop app's, none on the device.
pub(crate) fn name_suffix(app: &View) -> NameSuffix {
    if app.online {
        NameSuffix::Demo
    } else {
        NameSuffix::None
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
    ui.press(
        Shape::new(x, y, w, FLOW_ROW, SHAPE_S),
        Role::OnSurface.paint(),
        action,
    );
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn sidebar(app: &View, ui: &mut Ui, h: f32) {
    ui.fill(0.0, 0.0, SIDEBAR_W, h, 0.0, Role::SurfaceContainerLow);
    ui.fill(SIDEBAR_W - 1.0, 0.0, 1.0, h, 0.0, Role::OutlineVariant);
    ui.mark(22.0, 20.0, 28.0, 32.0);
    // The name centred on the mark, its capitals 10 tall; with its word
    // 9 tall, which keeps it inside the sidebar (at 10 it reaches the edge).
    let suffix = name_suffix(app);
    let cap = if suffix == NameSuffix::None {
        10.0
    } else {
        9.0
    };
    ui.wordmark_named(
        58.0,
        31.0 + (10.0 - cap) / 2.0,
        cap,
        Role::OnSurface,
        suffix,
    );

    // The places that are always offered, in order; Learn (not a place,
    // a sheet) is drawn between Tools and whatever comes after it.
    let items = [
        (Glyph::Icon(Icon::House), EN.sidebar_home, Screen::Home),
        (Glyph::Icon(Icon::Lock), EN.sidebar_vaults, Screen::Vaults),
        (Glyph::Icon(Icon::File), EN.sidebar_files, Screen::Files),
        (
            Glyph::Icon(Icon::Wallet),
            EN.sidebar_wallets,
            Screen::WalletsStart,
        ),
        (Glyph::Icon(Icon::Tools), EN.sidebar_tools, Screen::Catalog),
    ];
    let mut tail = Vec::new();
    if !app.stick.is_empty() && !app.holds_secret() {
        tail.push((app.medium.icon(), app.medium.visit(), Screen::Visit));
    }
    // The online app is the device's QR link.
    if app.online {
        tail.push((Glyph::Icon(Icon::Qr), EN.sidebar_transfer, Screen::Transfer));
    }
    tail.push((
        Glyph::Icon(Icon::Settings),
        EN.sidebar_settings,
        Screen::Settings,
    ));
    // The tabs are a touch target tall each, from just under the name.
    let mut y = 64.0;
    let flows = app.flows_in_progress();
    let on = app.flow_on_screen();
    let v = app.vaults.open.len();
    // Each tab is one of Material's destinations: icon and label in a
    // row, the current one on a pill (`docs/LAYOUT.md` §3).
    let row = |ui: &mut Ui, y: f32, icon: Glyph, label: &str, active: bool, action: Action| {
        // Vaults carries how many are open, at the row's right end.
        let open = (icon == Glyph::Icon(Icon::Lock) && v > 0)
            .then(|| fill(EN.sidebar_open_n, &[&v.to_string()]));
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
        // Learn, after Tools: opens the Learn sheet, not a screen.
        if screen == Screen::Catalog {
            row(
                ui,
                y,
                Glyph::Icon(Icon::Learn),
                EN.sidebar_learn,
                app.sheet == Some(Sheet::Learn) || app.learn_pane_shown(),
                Action::Learn,
            );
            y += TOUCH;
        }
    }

    // Status at the foot, and above it what is loaded in two groups,
    // seeds and wallets, as high as the last tab. A group that does not
    // fit shows its first rows and how many more, which opens the whole
    // list; each row opens what it names.
    // The pill under Lock while the idle lock waits for a new wallet's
    // backup (`docs/history/NEW-WALLET.md` §18) takes its room from the list.
    let held = app.lock_held();
    let foot = h - 172.0 - if held { LOCK_PILL_H + 8.0 } else { 0.0 };
    let seeds = app.session.keys.len();
    let wallets = app.session.wallets.len();
    const ROW: f32 = 22.0;
    const HEAD: f32 = 26.0;
    let groups = usize::from(seeds > 0) + usize::from(wallets > 0);
    let room = (foot - (y + 8.0) - 54.0 - groups as f32 * HEAD).max(0.0);
    let (cap_s, cap_w) = sidebar_caps(seeds, wallets, (room / ROW).floor() as usize);
    let gap = if groups > 0 { 8.0 } else { 0.0 };
    let block = groups as f32 * HEAD + (cap_s + cap_w) as f32 * ROW + gap;
    let mut fy = foot - block;
    ui.rule(12.0, fy, SIDEBAR_W - 24.0, Role::OutlineVariant);
    fy += 16.0;
    // The session strip, in the dot line's old place and height: the
    // four stages of the lock cycle, the one now current in the accent,
    // the rest dim. It explains the cycle without prose and opens Files,
    // costing the seed and wallet list below no room of its own.
    {
        let stage = app.session_stage();
        let open_files = Action::Nav(Screen::Files);
        let mut sx = 24.0;
        for (i, label) in [
            EN.sidebar_bring_in_button,
            EN.sidebar_open,
            EN.sidebar_work,
            EN.sidebar_write_out,
        ]
        .into_iter()
        .enumerate()
        {
            if i > 0 {
                sx += ui.text_mid_as((sx, fy, 20.0), BODY_S, Role::OnSurfaceVariant, " · ");
            }
            let fg = if label == stage {
                Role::Primary
            } else {
                Role::OnSurfaceVariant
            };
            sx += ui.text_mid_as((sx, fy, 20.0), LABEL_S, fg, label);
        }
        ui.press_box(16.0, fy - 4.0, SIDEBAR_W - 32.0, 28.0, open_files);
    }
    fy += 30.0;
    let rows_w = SIDEBAR_W - 56.0;
    // One group: its header with the count, its rows, and the rest as a
    // line that opens the whole list.
    let group = |ui: &mut Ui,
                 fy: &mut f32,
                 title: &str,
                 total: usize,
                 cap: usize,
                 rows: &mut dyn Iterator<
        Item = (String, String, bool, Action, Option<crate::ui::Paint>, bool),
    >,
                 more: Action| {
        if total == 0 {
            return;
        }
        ui.text_mid_as((28.0, *fy, HEAD), LABEL_M, Role::OnSurfaceVariant, title);
        ui.text_right_as(
            (SIDEBAR_W - 24.0, *fy, HEAD),
            LABEL_M,
            Role::OnSurfaceVariant,
            &total.to_string(),
        );
        *fy += HEAD;
        let shown = if total > cap {
            cap.saturating_sub(1)
        } else {
            total
        };
        for (head, tail, mono, action, mark, pass) in rows.take(shown) {
            // A wallet's dot: green when the seeds here sign for it alone,
            // grey when some of them are here.
            if let Some(c) = mark {
                ui.dot(18.0, *fy + ROW / 2.0, 3.5, c);
            }
            let role = if mono { BODY_S.mono() } else { LABEL_M };
            let head = ui.fit_as(role, &head, rows_w * 0.55);
            let hw = ui.text_mid_as((28.0, *fy, ROW), role, Role::OnSurface, &head);
            // A key loaded with a passphrase says so at the row's end:
            // the same words without it are another key.
            let tag_w = if pass {
                let t = EN.sidebar_passphrase;
                ui.text_right_as(
                    (SIDEBAR_W - 24.0, *fy, ROW),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    t,
                );
                ui.measure_as(BODY_S, t) + 8.0
            } else {
                0.0
            };
            if !tail.is_empty() {
                let tone = match mark {
                    Some(c) if c == Role::Ok.paint() => Role::Ok,
                    _ => Role::OnSurfaceVariant,
                };
                let tail = ui.fit_as(BODY_S, &tail, rows_w - hw - 8.0 - tag_w);
                ui.text_mid_as((36.0 + hw, *fy, ROW), BODY_S, tone, &tail);
            }
            ui.press_box(16.0, *fy, SIDEBAR_W - 32.0, ROW, action);
            *fy += ROW;
        }
        if total > cap && cap > 0 {
            let line = fill(EN.sidebar_more, &[&(total - shown).to_string()]);
            ui.text_mid_as((28.0, *fy, ROW), LABEL_M, Role::Primary, &line);
            ui.press_box(16.0, *fy, SIDEBAR_W - 32.0, ROW, more);
            *fy += ROW;
        }
    };
    let mut seed_rows = app.session.keys.all().map(|k| {
        let fp = fp_text(k.fingerprint);
        // A seed named by its fingerprint is not named twice.
        let label = if k.label.eq_ignore_ascii_case(&fp) {
            String::new()
        } else {
            k.label.clone()
        };
        (
            fp,
            label,
            true,
            Action::ExploreKey(k.fingerprint.0),
            None,
            k.has_passphrase,
        )
    });
    group(
        ui,
        &mut fy,
        EN.sidebar_keys,
        seeds,
        cap_s,
        &mut seed_rows,
        Action::Explore,
    );
    // Wallets the seeds here sign for alone first, then those with some
    // of their seeds here, then watch-only ones: a cut list keeps the
    // ready ones in view.
    let mut ranked: Vec<(usize, u8)> = app
        .session
        .wallets
        .iter()
        .enumerate()
        .map(|(i, _)| (i, app.readiness(i)))
        .collect();
    ranked.sort_by_key(|&(i, r)| (std::cmp::Reverse(r), i));
    let mut wallet_rows = ranked.into_iter().map(|(i, ready)| {
        let w = &app.session.wallets[i];
        let (m, n) = Session::quorum(w);
        // A key not of the wallet spends it alone: no "k of n" (F132).
        let shape = if crate::wallet::another_key_spends(w) {
            EN.sidebar_another_key.to_string()
        } else if n > 1 {
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
        (
            w.name.clone(),
            shape,
            false,
            Action::OpenWallet(i),
            mark,
            false,
        )
    });
    group(
        ui,
        &mut fy,
        EN.sidebar_wallets,
        wallets,
        cap_w,
        &mut wallet_rows,
        Action::Nav(Screen::Wallets),
    );
    fy += gap;
    let files = app.files_count();
    let here = app.screen == Screen::Files;
    if here {
        ui.fill(
            16.0,
            fy - 4.0,
            SIDEBAR_W - 32.0,
            28.0,
            SHAPE_S,
            Role::SecondaryContainer,
        );
    }
    let muted = on_selected(here, Role::OnSurfaceVariant);
    ui.icon(22.0, fy, 20.0, Glyph::Icon(Icon::File), 12.0, muted);
    ui.text_mid_as((46.0, fy, 20.0), BODY_S, muted, EN.sidebar_files);
    let ink = on_selected(here, Role::OnSurface);
    ui.text_right_as((SIDEBAR_W - 24.0, fy, 20.0), BODY_S, ink, &files);
    ui.press_box(
        16.0,
        fy - 4.0,
        SIDEBAR_W - 32.0,
        28.0,
        Action::Nav(Screen::Files),
    );
    fy += 30.0;
    // With no stick attached, an import the boot stick brought waits
    // behind this row.
    let waiting = app.import.state().is_some() && app.stick.is_empty();
    let sticks = match app.stick.len() {
        0 if waiting => EN.sidebar_import_waiting.to_string(),
        0 => EN.sidebar_none.to_string(),
        1 => app.stick[0].label.clone(),
        k => fill(EN.sidebar_attached, &[&k.to_string()]),
    };
    let open = waiting && app.sheet == Some(Sheet::Import);
    if open {
        ui.fill(
            16.0,
            fy - 4.0,
            SIDEBAR_W - 32.0,
            28.0,
            SHAPE_S,
            Role::SecondaryContainer,
        );
    }
    let muted = on_selected(open, Role::OnSurfaceVariant);
    ui.icon(22.0, fy, 20.0, app.medium.icon(), 12.0, muted);
    ui.text_mid_as((46.0, fy, 20.0), BODY_S, muted, app.medium.caps());
    let sticks = ui.fit_as(BODY_S, &sticks, 110.0);
    let tone = on_selected(
        open,
        if waiting {
            Role::Primary
        } else {
            Role::OnSurface
        },
    );
    ui.text_right_as((SIDEBAR_W - 24.0, fy, 20.0), BODY_S, tone, &sticks);
    if waiting {
        ui.press_box(
            16.0,
            fy - 4.0,
            SIDEBAR_W - 32.0,
            28.0,
            crate::boot_import::OPEN,
        );
    }
    fy += 34.0;
    if app.holds_secret() {
        ui.button(
            16.0,
            fy,
            Some(SIDEBAR_W - 32.0 - 44.0),
            BUTTON_H,
            EN.sidebar_lock,
            Style::Secondary,
            Action::LockAsk,
        );
    }
    let px = SIDEBAR_W - 16.0 - 36.0;
    // Material's outlined icon button, round (F108).
    let r = faraday_draw::tokens::shape_full(36.0);
    ui.stroke(px, fy, 36.0, 36.0, r, Role::OutlineVariant);
    ui.icon(
        px,
        fy,
        36.0,
        Glyph::Icon(Icon::Power),
        14.0,
        Role::OnSurfaceVariant,
    );
    ui.press(
        Shape::new(px, fy, 36.0, 36.0, r),
        Role::OnSurface.paint(),
        Action::PowerAsk,
    );
    ui.tip_plain((px, fy, 36.0, 36.0), Action::PowerAsk, EN.tip_power_off);
    if held {
        lock_held_pill(ui, 16.0, fy + 44.0, SIDEBAR_W - 32.0);
    }
}

/// The pill's height.
pub(crate) const LOCK_PILL_H: f32 = 24.0;

/// "Auto-lock off until backup complete" as an outlined pill at (x, y),
/// `w` wide at most (`docs/history/NEW-WALLET.md` §18). Returns its height.
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
