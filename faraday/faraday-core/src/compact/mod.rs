//! The small panel (`Faraday::is_compact`): no sidebar. Home is the menu,
//! a button for each of the sidebar's entries, and every other screen has
//! a bar across its top that goes back to it. Flows are a page per step
//! (`flow::column`).

pub(crate) mod screen;

use crate::strings::EN;
use crate::strings::fill;
use faraday_draw::tokens::{
    GRID_TILE_INSET, GRID_TILE_TEXT_X, LEAD_MAX, TILE_H_COMPACT, TILE_H_GRID,
};
use faraday_draw::widgets::Icon;

use crate::drawing::View;
use crate::ui::pal::*;
use crate::ui::{Glyph, Style, Ui, W};
use crate::{Action, Faraday, Screen};
pub(crate) use faraday_ui::sheet::{buttons, kv, kv_sheet, scroll_sheet, sheet, sheet_head};

/// The side margin, design units.
pub(crate) const M: f32 = 12.0;
/// The bar across the top of every page but Home.
pub(crate) const BAR_H: f32 = 44.0;

/// The screens that draw their own top, as a step flow does.
pub(crate) fn draws_own_bar(s: Screen) -> bool {
    matches!(
        s,
        Screen::Home
            | Screen::Create
            | Screen::Restore
            | Screen::Backup
            | Screen::Message
            | Screen::Spend
            | Screen::KeyGen
            | Screen::Bip85
            | Screen::Silent
            | Screen::Vanity
            | Screen::CreateVault
            | Screen::BackupFlow
            | Screen::Family
    )
}

/// What a page's bar calls it.
pub(crate) fn screen_name(s: Screen, medium: crate::Medium) -> &'static str {
    match s {
        Screen::Home => EN.compact_home,
        Screen::WalletsStart | Screen::Wallets => EN.compact_wallets,
        Screen::Spend | Screen::Family => EN.compact_spend,
        Screen::Files => EN.compact_files,
        Screen::Visit => medium.visit(),
        Screen::Entry => EN.compact_add_a_key,
        Screen::Backup => EN.compact_back_up,
        Screen::Message => EN.compact_sign_a_message,
        Screen::CheckMessage => EN.compact_check_a_message,
        Screen::Create => EN.compact_make_a_new,
        Screen::Restore => EN.compact_restore_wallet,
        Screen::Settings => EN.compact_settings,
        Screen::SetTime => EN.compact_set_the_time,
        Screen::Randomness => EN.own_add_randomness,
        Screen::Codes => EN.compact_totp_codes,
        Screen::Vaults | Screen::Unlock | Screen::VaultContents => EN.compact_vaults,
        Screen::Backups => EN.compact_backups,
        Screen::CreateVault => EN.compact_create_a_vault,
        Screen::KeyGen => EN.compact_new_key,
        Screen::Bip85 => EN.compact_bip_85,
        Screen::Silent => EN.compact_silent_payments,
        Screen::Explore => EN.compact_explore_a_key,
        Screen::Lightning => EN.compact_lightning,
        Screen::Tools | Screen::Catalog => EN.compact_tools,
        Screen::Vanity => EN.compact_vanity_address,
        Screen::Decode => EN.compact_transaction,
        Screen::Transfer => EN.compact_transfer,
        Screen::Upgrade => medium.upgrade(),
        Screen::BackupFlow => EN.backup_flow_heading,
    }
}

/// The bar: a way back on the left, the page's name, and the ? when the
/// page has something in Learn. Returns its height.
pub(crate) fn bar(app: &View, ui: &mut Ui, w: f32, back: (&str, Action), name: &str) -> f32 {
    ui.fill(0.0, 0.0, w, BAR_H, 0.0, Role::SurfaceContainerLow);
    ui.fill(0.0, BAR_H - 1.0, w, 1.0, 0.0, Role::OutlineVariant);
    let (label, action) = app.named_back(back);
    if ui.is_pressed(action) {
        ui.wash(
            4.0,
            4.0,
            BAR_H - 8.0,
            BAR_H - 8.0,
            8.0,
            Role::OnSurface.at(faraday_draw::tokens::STATE_PRESS),
        );
    }
    ui.icon(
        4.0,
        4.0,
        BAR_H - 8.0,
        Icon::ChevronLeft,
        13.0,
        Role::OnSurface,
    );
    ui.hit_tip((0.0, 0.0, BAR_H + 8.0, BAR_H), action, EN.tip_back);
    ui.back(action);
    let right = tools(app, ui, w);
    // The page's own name, and above it, small, where Back goes.
    let x = BAR_H;
    let room = right - x;
    let label = ui.fit(11.0, W::R, label, room);
    ui.text(x, 5.0, 11.0, W::R, Role::OnSurfaceVariant, &label);
    let name = ui.fit(16.0, W::S, name, room);
    ui.text(x, 19.0, 16.0, W::S, Role::OnSurface, &name);
    BAR_H
}

/// Where the tools of a step flow's bar begin, measured once and kept on
/// the `Ui` (`bar_tools_x`) for the flow's title to end before; the same
/// measure `flow_bar_tools` draws from. Cheap: widths of the tools' labels.
pub(crate) fn flow_bar_tools_x(app: &View, ui: &mut Ui, w: f32) -> f32 {
    let right = tools(app, ui, -w);
    if right < w - M {
        right - faraday_draw::tokens::GAP
    } else {
        right
    }
}

/// A step flow's bar, which the flow draws: the ? and the keyboard at its
/// right end, over whatever of its name runs that far.
pub(crate) fn flow_bar_tools(app: &View, ui: &mut Ui, w: f32) {
    let right = tools(app, ui, -w);
    if right < w - M {
        ui.fill(
            right,
            0.0,
            w - right,
            BAR_H - 1.0,
            0.0,
            Role::SurfaceContainerLow,
        );
        tools(app, ui, w);
    }
}

/// The bar's keyboard toggle, where the page types, and to bring it back
/// once put away from a field. Draws it when `draw`; returns `right` moved
/// left past it.
fn osk_toggle(app: &View, ui: &mut Ui, draw: bool, right: f32) -> f32 {
    if !(app.osk_offered() || app.osk_shown() || app.osk_auto()) {
        return right;
    }
    let shown = app.osk_shown();
    let action = Action::Osk(if shown {
        OskPress::Hide
    } else {
        OskPress::Show
    });
    let kx = right - 34.0;
    let ky = (BAR_H - 34.0) / 2.0;
    if draw {
        if shown || ui.is_pressed(action) {
            let fill = crate::ui::pal::selected_or(shown, Role::SurfaceContainerHighest);
            ui.fill(kx, ky, 34.0, 34.0, 8.0, fill);
        }
        let ink = crate::ui::pal::on_selected(shown, Role::OnSurfaceVariant);
        ui.icon(kx, ky, 34.0, Icon::Keyboard, 14.0, ink);
        ui.hit_tip((kx - 4.0, 0.0, 42.0, BAR_H), action, EN.tip_keyboard);
    }
    kx - 6.0
}

/// The ? when the page has something in Learn, and the keyboard where
/// the page types, at the bar's right end. Returns where they begin.
/// A negative `w` measures without drawing.
fn tools(app: &View, ui: &mut Ui, w: f32) -> f32 {
    let draw = w > 0.0;
    let w = w.abs();
    let mut right = if app.learn_pages().is_empty() {
        w - M
    } else {
        if draw {
            help(ui, w - M - 30.0, (BAR_H - 30.0) / 2.0);
        }
        w - M - 38.0
    };
    // The mode, off mainnet, beside the ?: centred on the bar, a gap
    // to the ?'s left (the gap `right` already holds). Pressed, it opens
    // the mode sheet.
    if crate::modes::screen::shows_pill(app) {
        let pw = crate::modes::screen::pill_width(app, ui);
        let (px, py) = (
            right - pw,
            (BAR_H - faraday_draw::tokens::MODE_PILL_HEIGHT) / 2.0,
        );
        if draw {
            crate::modes::screen::pill(app, ui, px, py);
        }
        right = px - faraday_draw::tokens::GAP;
    }
    // The keyboard, where the page types, and to bring it back once put
    // away from a field.
    right = osk_toggle(app, ui, draw, right);
    // The eye, while Make codex32 shares has sheets to mask
    // (`docs/CODEX32.md` §6.4.6).
    if let Some(masked) = eye_state(app) {
        use faraday_draw::tokens::{ICON_BUTTON, TOUCH};
        // Material's icon button: 40 dp round in a 48 dp target.
        let edge = (TOUCH - ICON_BUTTON) / 2.0;
        let (kx, ky) = (right - edge - ICON_BUTTON, (BAR_H - ICON_BUTTON) / 2.0);
        right = kx - edge;
        if draw {
            eye_icon(ui, kx, ky, masked);
            let eye = Action::Codex32(crate::codex32::C32::Eye);
            eye_hit(ui, (kx - edge, 0.0, TOUCH, BAR_H), eye, !masked);
        }
    }
    // Cancel, on a flow that can be in progress: it leaves the flow
    // (`docs/FLOWS.md`, Cancel; F51).
    if app.cancel_offered().is_some() {
        let a = Action::CancelFlow;
        let cw = ui.measure(13.0, W::S, EN.flow_cancel) + 20.0;
        let cx = right - cw;
        if draw {
            if ui.is_pressed(a) {
                ui.wash(
                    cx,
                    8.0,
                    cw,
                    BAR_H - 16.0,
                    8.0,
                    Role::OnSurface.at(faraday_draw::tokens::STATE_PRESS),
                );
            }
            ui.text_mid(
                cx + 10.0,
                0.0,
                BAR_H,
                13.0,
                W::S,
                Role::OnSurfaceVariant,
                EN.flow_cancel,
            );
            ui.hit(cx, 0.0, cw, BAR_H, a);
        }
        right = cx - 2.0;
    }
    right
}

/// Whether the eye is offered on this page, and whether the sheets are
/// masked now: Make codex32 shares' sheets, on a device
/// (`docs/CODEX32.md` §6.4.6).
fn eye_state(app: &View) -> Option<bool> {
    if app.screen != Screen::KeyGen {
        return None;
    }
    let m = app.keygen()?.codex32.as_ref()?;
    m.eye_offered().then(|| m.masked())
}

/// The eye: open while the sheets are shown, struck while they are masked
/// (the icon shows what a press does, as a password field's eye does).
fn eye_icon(ui: &mut Ui, x: f32, y: f32, masked: bool) {
    use faraday_draw::tokens::{ICON_BUTTON, ICON_GLYPH, shape_full};
    let a = Action::Codex32(crate::codex32::C32::Eye);
    let r = shape_full(ICON_BUTTON);
    if !masked || ui.is_pressed(a) {
        ui.fill(
            x,
            y,
            ICON_BUTTON,
            ICON_BUTTON,
            r,
            crate::ui::pal::selected_or(!masked, Role::SurfaceContainerHighest),
        );
    }
    ui.icon(
        x,
        y,
        ICON_BUTTON,
        if masked { Icon::Eye } else { Icon::EyeOff },
        ICON_GLYPH,
        crate::ui::pal::on_selected(!masked, Role::OnSurfaceVariant),
    );
}

/// The wide layout's eye (`docs/CODEX32.md` §6.4.6): Material's icon
/// button in the page's right margin, `right` its right edge, under the
/// top row of the network and the ?, so it stands over no row of the
/// flow's column.
pub(crate) fn wide_eye_button(app: &View, ui: &mut Ui, right: f32) {
    use faraday_draw::tokens::{EYE_WIDE_INSET, ICON_BUTTON, TOUCH};
    let Some(masked) = eye_state(app) else {
        return;
    };
    let edge = (TOUCH - ICON_BUTTON) / 2.0;
    let x = right - EYE_WIDE_INSET - ICON_BUTTON;
    let y = TOUCH + edge;
    eye_icon(ui, x, y, masked);
    let eye = Action::Codex32(crate::codex32::C32::Eye);
    eye_hit(ui, (x - edge, y - edge, TOUCH, TOUCH), eye, !masked);
}

/// An eye's press target at `rect`, with the tooltip that names what a
/// press does: it hides what `shown` shows, and shows what is hidden.
pub(crate) fn eye_hit(ui: &mut Ui, rect: (f32, f32, f32, f32), action: Action, shown: bool) {
    ui.hit_tip(rect, action, if shown { EN.tip_hide } else { EN.tip_show });
}

/// The wide layout's keyboard button, in the top-right row its right
/// edge at `right`, beside the network and the ?: on a page where typing
/// goes somewhere, it brings the docked keyboard up and puts it away.
pub(crate) fn wide_keyboard_button(app: &View, ui: &mut Ui, right: f32) {
    if !app.osk_wanted() {
        return;
    }
    let shown = app.osk_shown();
    let action = Action::Osk(if shown {
        OskPress::Hide
    } else {
        OskPress::Show
    });
    let (s, y) = (28.0, 12.0);
    let x = right - s;
    let tint = if shown {
        Role::SecondaryContainer.paint()
    } else if ui.is_pressed(action) || ui.is_hovered(action) {
        Role::SurfaceContainerHighest.paint()
    } else {
        Role::Surface.paint()
    };
    ui.fill(x, y, s, s, s / 2.0, tint);
    ui.stroke(
        x,
        y,
        s,
        s,
        s / 2.0,
        crate::ui::pal::selected_or(shown, Role::OutlineVariant),
    );
    ui.icon(
        x,
        y,
        s,
        Icon::Keyboard,
        13.0,
        crate::ui::pal::on_selected(shown, Role::OnSurfaceVariant),
    );
    ui.hit_tip(
        (x - 6.0, y - 6.0, s + 12.0, s + 12.0),
        action,
        EN.tip_keyboard,
    );
}

/// The ? that opens what explains the page.
fn help(ui: &mut Ui, x: f32, y: f32) {
    let pressed = ui.is_pressed(Action::Learn);
    crate::ui::fill_pressed(ui, (x, y, 30.0, 30.0, 15.0), Role::Surface, pressed);
    ui.stroke(x, y, 30.0, 30.0, 15.0, Role::OutlineVariant);
    let tw = ui.measure(14.0, W::S, "?");
    ui.text_mid(
        x + 15.0 - tw / 2.0,
        y,
        30.0,
        14.0,
        W::S,
        Role::OnSurfaceVariant,
        "?",
    );
    ui.hit_tip((x - 6.0, y - 6.0, 42.0, 42.0), Action::Learn, EN.tip_learn);
}

/// A page that is not one of the step flows: its bar, then the page
/// under it as the desktop draws it, in the panel's width.
pub(crate) fn page(app: &View, ui: &mut Ui, w: f32, h: f32) -> (f32, f32) {
    let (back, name) = page_head(app);
    bar(app, ui, w, back, name);
    (BAR_H, h - BAR_H)
}

/// A page's way back with the name of where it lands, and the page's
/// own name: what its bar or its top app bar shows. A lookup.
pub(crate) fn page_head<'a>(app: &'a View) -> ((&'a str, Action), &'a str) {
    let explore = app.explore.page();
    let name = match app.session.wallets.get(app.wallets.wallet()) {
        _ if app.screen == Screen::Explore && explore == Some(crate::explore::Page::Addresses) => {
            EN.explore_addresses
        }
        _ if app.screen == Screen::Explore && explore == Some(crate::explore::Page::Find) => {
            EN.explore_find
        }
        Some(_) if app.screen == Screen::Wallets && app.glance.open() => EN.compact_at_a_glance,
        Some(w) if app.screen == Screen::Wallets => w.name.as_str(),
        _ => screen_name(app.screen, app.medium),
    };
    let back = match app.screen {
        // The chart's page goes back to the wallet's card.
        Screen::Wallets if app.glance.open() => (
            app.session
                .wallets
                .get(app.wallets.wallet())
                .map_or(EN.compact_wallets, |w| w.name.as_str()),
            Action::Glance(false),
        ),
        // Explore's address list and search go back to its path.
        Screen::Explore if explore.is_some_and(|p| p != crate::explore::Page::Path) => (
            EN.compact_explore_a_key,
            Action::X(crate::explore::XAction::Page(crate::explore::Page::Path)),
        ),
        Screen::Wallets | Screen::Explore | Screen::Entry | Screen::CheckMessage => {
            (EN.compact_wallets, Action::Nav(Screen::WalletsStart))
        }
        // An item of a vault goes back to the vault's list.
        Screen::VaultContents if crate::vault::screen::contents_detail(app) => (
            EN.compact_vault_contents,
            Action::Vault(crate::vault::VaultAction::ItemBack),
        ),
        Screen::Unlock => crate::vault::screen::unlock_back(app),
        Screen::VaultContents => (EN.compact_vaults, Action::Nav(Screen::Vaults)),
        // Made from another flow: the way back to it.
        Screen::Vaults if let Some(s) = app.vaults.back_to => (
            app.back_link_name(s),
            Action::Vault(crate::vault::VaultAction::Back),
        ),
        // Decode goes back where it was opened from.
        Screen::Decode => match app.decode() {
            Some(d) => (crate::screens::decode_back(d.back), Action::Nav(d.back)),
            None => (EN.compact_tools, Action::Nav(Screen::Catalog)),
        },
        Screen::Tools | Screen::Lightning | Screen::Codes => {
            (EN.compact_tools, Action::Nav(Screen::Catalog))
        }
        Screen::Upgrade => (EN.compact_settings, Action::Nav(Screen::Settings)),
        // Add randomness goes back where it was opened: Settings or New key.
        Screen::Randomness => (
            app.trail_name().unwrap_or(EN.compact_settings),
            Action::Back,
        ),
        _ => (EN.compact_home, Action::Nav(Screen::Home)),
    };
    (back, name)
}

/// The keyboard button's press where the page types, and whether the
/// keyboard is up: what the top app bar's button shows. A lookup.
pub(crate) fn keyboard_action(app: &View) -> Option<(Action, bool)> {
    let shown = app.osk_shown();
    let action = Action::Osk(if shown {
        OskPress::Hide
    } else {
        OskPress::Show
    });
    (app.osk_offered() || shown || app.osk_auto()).then_some((action, shown))
}

/// The height Scan's floating button takes at the foot of Home (its own
/// height, the margin under it and a gap above it), so the scrollable
/// list can be clipped above it.
const FAB_ROOM: f32 = M + 40.0 + 8.0;

/// Home: what is loaded, then a button for each place the sidebar goes.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn home(app: &View, ui: &mut Ui, w: f32, h: f32) {
    // Scan floats over the foot in a strip of its own: the scrollable
    // list is clipped above it, so no tile's hit area ever falls under
    // it, at any scroll position, not only at the bottom.
    let view_h = (h - FAB_ROOM).max(0.0);
    let clip = ui.rect(0.0, 0.0, w, view_h);
    ui.c.push_clip(clip);
    let top = -app.list_offset;
    let mut y = top + 12.0;
    let inner = w - 2.0 * M;

    // On the rail's and the bar's windows the top app bar carries the
    // name, and the sidebar's lists lead as chips (`docs/LAYOUT.md` §3).
    let beside = app.nav().beside();
    if beside {
        y += crate::nav::screen::status_strip(app, ui, M, y, inner);
    } else {
        home_head(app, ui, w, y);
        y += 38.0;
    }

    // The system gave no randomness: nothing seals until a restart.
    if app.seed_missing() {
        y += ui.wrap(M, y, inner, 13.0, W::S, Role::Error, EN.seed_missing) + 10.0;
    }

    // What this session holds.
    let keys = app.session.keys.len();
    let wallets = app.session.wallets.len();
    let vaults = app.vaults.open.len();
    let (dot, line) = match (keys + wallets, vaults) {
        (0, 0) => (
            Role::OnSurfaceVariant,
            EN.compact_nothing_loaded.to_string(),
        ),
        (_, 0) => (
            Role::Ok,
            format!(
                "{keys} {} · {wallets} {}",
                if keys == 1 {
                    EN.compact_key_inline
                } else {
                    EN.compact_keys
                },
                if wallets == 1 {
                    EN.compact_wallet
                } else {
                    EN.compact_wallets_inline
                }
            ),
        ),
        (_, v) => (
            Role::Ok,
            fill(
                EN.compact_vaults_open,
                &[
                    &v.to_string(),
                    (if v == 1 {
                        EN.compact_vault
                    } else {
                        EN.compact_vaults_inline
                    }),
                ],
            ),
        ),
    };
    let sub = app.home_files_line();
    let session = if keys + wallets == 0 {
        Screen::WalletsStart
    } else {
        Screen::Wallets
    };
    let action = Action::Nav(session);
    let pressed = ui.is_pressed(action);
    crate::ui::fill_pressed(
        ui,
        (M, y, inner, 52.0, 10.0),
        Role::SurfaceContainerLow,
        pressed,
    );
    ui.stroke(M, y, inner, 52.0, 10.0, Role::OutlineVariant);
    ui.dot(M + 16.0, y + 18.0, 4.0, dot);
    let line = ui.fit(14.0, W::S, &line, inner - 40.0);
    ui.text_mid(M + 28.0, y + 6.0, 24.0, 14.0, W::S, Role::OnSurface, &line);
    let sub = ui.fit(12.0, W::R, &sub, inner - 40.0);
    ui.text_mid(
        M + 28.0,
        y + 26.0,
        20.0,
        12.0,
        W::R,
        Role::OnSurfaceVariant,
        &sub,
    );
    ui.hit(M, y, inner, 52.0, action);
    y += 52.0 + 10.0;

    // The session strip: the four stages of the lock cycle, the one now
    // current in the accent, the rest dim. Tapping it opens Files.
    {
        let stage = app.session_stage();
        let open_files = Action::Nav(Screen::Files);
        if ui.is_pressed(open_files) {
            ui.wash(
                M,
                y,
                inner,
                22.0,
                6.0,
                Role::OnSurface.at(faraday_draw::tokens::STATE_PRESS),
            );
        }
        let mut sx = M + 4.0;
        for (i, label) in [
            EN.compact_bring_in_button,
            EN.compact_open_button,
            EN.compact_work,
            EN.compact_write_out,
        ]
        .into_iter()
        .enumerate()
        {
            if i > 0 {
                sx += ui.text_mid(sx, y, 22.0, 11.0, W::R, Role::OnSurfaceVariant, " · ");
            }
            let fg = if label == stage {
                Role::Primary
            } else {
                Role::OnSurfaceVariant
            };
            sx += ui.text_mid(sx, y, 22.0, 11.0, W::S, fg, label);
        }
        ui.hit(M, y, inner, 22.0, open_files);
        y += 22.0 + 10.0;
    }

    // A tile per flow in progress leads the tiles: its name and its open
    // step, opening it as it was left (`docs/FLOWS.md`, Flows in
    // progress).
    for r in app.flows_in_progress().iter().filter(|_| !beside) {
        let icon = match r.tab {
            Screen::Vaults => Icon::Lock,
            Screen::Catalog => Icon::Tools,
            _ => Icon::Wallet,
        };
        y += crate::compact::screen::row(
            ui,
            M,
            y,
            inner,
            Some(icon),
            &r.name,
            &r.step,
            Role::OnSurfaceVariant.paint(),
            Some(Action::Resume(r.going)),
        );
    }

    // The one thing waiting, when there is one.
    let lead_w = if app.nav() == crate::nav::Nav::Menu {
        inner
    } else {
        inner.min(LEAD_MAX)
    };
    for (icon, label, action, tone) in prompts(app) {
        let pressed = action.is_some_and(|a| ui.is_pressed(a));
        let edge = if tone == Role::Warning.paint() {
            Role::Warning.with_alpha(110)
        } else {
            Role::Primary.with_alpha(110)
        };
        ui.fill(
            M,
            y,
            lead_w,
            44.0,
            10.0,
            if pressed {
                Role::SurfaceContainerHighest.paint()
            } else {
                tone.with_alpha(26)
            },
        );
        ui.stroke(M, y, lead_w, 44.0, 10.0, edge);
        ui.icon(M + 8.0, y + 8.0, 28.0, icon, 14.0, tone);
        let label = ui.fit(14.0, W::S, &label, lead_w - 60.0);
        ui.text_mid(M + 44.0, y, 44.0, 14.0, W::S, Role::OnSurface, &label);
        if let Some(a) = action {
            ui.icon(
                M + lead_w - 26.0,
                y + 12.0,
                20.0,
                Icon::ChevronRight,
                9.0,
                Role::OnSurfaceVariant,
            );
            ui.hit(M, y, lead_w, 44.0, a);
        }
        y += 44.0 + 8.0;
    }
    y += 2.0;

    // The menu: the sidebar's places, two to a row.
    // On the panel's menu two to a row; elsewhere a grid of as many
    // columns as `TILE_MIN` leaves room for (`docs/LAYOUT.md` §4).
    let gap = 8.0;
    let (cols, tw) = if app.nav() == crate::nav::Nav::Menu {
        (2, (inner - gap) / 2.0)
    } else {
        crate::screens::tile_grid(inner, gap, 0.0)
    };
    let grid = app.nav() != crate::nav::Nav::Menu;
    let th = if grid { TILE_H_GRID } else { TILE_H_COMPACT };
    let tiles = tiles(app);
    for (i, (icon, label, sub, action, enabled)) in tiles.iter().enumerate() {
        let tx = M + (i % cols) as f32 * (tw + gap);
        let ty = y + (i / cols) as f32 * (th + gap);
        let pressed = ui.is_pressed(*action);
        crate::ui::fill_pressed(
            ui,
            (tx, ty, tw, th, 12.0),
            Role::SurfaceContainerLow,
            pressed,
        );
        ui.stroke(tx, ty, tw, th, 12.0, Role::OutlineVariant);
        let (ibg, ifg) = if *enabled {
            (
                Role::SecondaryContainer.paint(),
                Role::OnSecondaryContainer.paint(),
            )
        } else {
            (
                Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTAINER),
                crate::ui::off(),
            )
        };
        ui.fill(tx + 12.0, ty + 12.0, 30.0, 30.0, 8.0, ibg);
        ui.icon(tx + 12.0, ty + 12.0, 30.0, *icon, 14.0, ifg);
        let fg = if *enabled {
            Role::OnSurface.paint()
        } else {
            crate::ui::off()
        };
        if grid {
            // The icon beside the name, one line under it
            // (`docs/LAYOUT.md` §4).
            let room = tw - GRID_TILE_TEXT_X - GRID_TILE_INSET;
            let label = ui.fit(14.0, W::S, label, room);
            let tx = tx + GRID_TILE_TEXT_X;
            if sub.is_empty() {
                ui.text_mid(tx, ty, th, 14.0, W::S, fg, &label);
            } else {
                ui.text(tx, ty + 10.0, 14.0, W::S, fg, &label);
                let sub = ui.fit(12.0, W::R, sub, room);
                let c = if *enabled {
                    Role::OnSurfaceVariant
                } else {
                    Role::Warning
                };
                ui.text(tx, ty + 30.0, 12.0, W::R, c, &sub);
            }
        } else {
            let label = ui.fit(14.0, W::S, label, tw - 24.0);
            ui.text(tx + 12.0, ty + 48.0, 14.0, W::S, fg, &label);
            if !sub.is_empty() {
                let sub = ui.fit(11.0, W::R, sub, tw - 54.0);
                ui.text_right(
                    tx + tw - 10.0,
                    ty + 12.0,
                    30.0,
                    11.0,
                    W::R,
                    if *enabled {
                        Role::OnSurfaceVariant
                    } else {
                        Role::Warning
                    },
                    &sub,
                );
            }
        }
        if *enabled {
            ui.hit(tx, ty, tw, th, *action);
        }
    }
    y += tiles.len().div_ceil(cols) as f32 * (th + gap) + 4.0;

    // Lock and power, here on the small panel; the rail and the top app
    // bar carry them on wider windows.
    if beside {
        y += faraday_draw::tokens::GRID;
    } else {
        y += lock_row(app, ui, y, inner, gap);
    }
    // The idle lock waits for a new wallet's backup (`docs/history/NEW-WALLET.md`
    // §18).
    if app.lock_held() {
        y += crate::screens::lock_held_pill(ui, M, y - 8.0, inner) + 8.0;
    }

    let content = y - top;
    app.scroll.content_h().set(content);
    ui.report_scroll(clip, content - view_h);
    ui.c.pop_clip();

    ui.fab(w - M, h - M, Icon::Scan, EN.compact_scan, Action::Scan);
}

/// Home's foot on the small panel at `y`: Lock, when something is held,
/// and power. Returns the height it takes.
fn lock_row(app: &View, ui: &mut Ui, y: f32, inner: f32, gap: f32) -> f32 {
    let pw = 44.0;
    if app.holds_secret() {
        ui.button(
            M,
            y,
            Some(inner - pw - gap),
            44.0,
            EN.compact_lock,
            Style::Secondary,
            Action::LockAsk,
        );
    }
    let px = M + inner - pw;
    if ui.is_pressed(Action::PowerAsk) {
        ui.wash(
            px,
            y,
            pw,
            44.0,
            10.0,
            Role::OnSurface.at(faraday_draw::tokens::STATE_PRESS),
        );
    }
    ui.stroke(px, y, pw, 44.0, 10.0, Role::OutlineVariant);
    ui.icon(px, y, pw, Icon::Power, 15.0, Role::OnSurfaceVariant);
    ui.hit_tip((px, y, pw, 44.0), Action::PowerAsk, EN.tip_power_off);
    44.0 + 16.0
}

/// Home's head on the small panel: the mark and the name; off mainnet,
/// the mode pill on the right, which opens the mode sheet.
fn home_head(app: &View, ui: &mut Ui, w: f32, y: f32) {
    ui.mark(M, y, 20.0, 23.0);
    let mut suffix = crate::screens::name_suffix(app);
    let pill = crate::modes::screen::shows_pill(app);
    let pw = if pill {
        crate::modes::screen::pill_width(app, ui)
    } else {
        0.0
    };
    // The pill takes the name's end where the two do not fit: the name
    // goes without "Desktop".
    let (nx, ny) = (M + 28.0, y + 6.0);
    let room = w - M - pw - faraday_draw::tokens::GAP;
    let fits = |ui: &mut Ui, suffix| {
        nx + crate::compact::screen::measured(ui, |ui| {
            ui.wordmark_named(nx, ny, 11.0, Role::OnSurface, suffix)
        }) <= room
    };
    if pill && !fits(ui, suffix) {
        suffix = crate::ui::NameSuffix::None;
    }
    // With Load samples beside it the pair leaves no room for the name:
    // the mark stands alone.
    if fits(ui, suffix) {
        ui.wordmark_named(nx, ny, 11.0, Role::OnSurface, suffix);
    }
    if pill {
        crate::modes::screen::pill(app, ui, w - M - pw, y - 2.0);
    }
}

/// A tile: its icon, its name, a short count, where it goes, and
/// whether it can be pressed now.
type Tile = (crate::ui::Glyph, &'static str, String, Action, bool);

fn tiles(app: &View) -> Vec<Tile> {
    let wallets = app.session.wallets.len();
    let vault_files = app.vault_files().len();
    let open = app.vaults.open.len();
    // In two columns: wallets and keys first (every wallet flow,
    // including spending: `docs/history/SIMPLIFY.md` §6.2 moves Spend to the
    // Wallets empty state and the Learn sheet), then the ways in and
    // out, then what is used now and then.
    let mut t: Vec<Tile> = vec![
        (
            Icon::Wallet.into(),
            EN.compact_wallets,
            if wallets == 0 {
                String::new()
            } else {
                wallets.to_string()
            },
            Action::Nav(Screen::WalletsStart),
            true,
        ),
        (
            Icon::Lock.into(),
            EN.compact_vaults,
            match (open, vault_files) {
                (0, 0) => String::new(),
                (0, n) => fill(EN.compact_locked, &[&n.to_string()]),
                (o, _) => fill(EN.compact_open, &[&o.to_string()]),
            },
            Action::Nav(Screen::Vaults),
            true,
        ),
        (
            Icon::File.into(),
            EN.compact_files,
            format!("{} · {}", app.inbox.len(), app.outbox.len()),
            Action::Nav(Screen::Files),
            true,
        ),
        (
            Icon::Learn.into(),
            EN.compact_learn,
            String::new(),
            Action::Learn,
            true,
        ),
    ];
    // Backups once any wallet is known (§5), after Wallets.
    let known = app.backups();
    if !known.is_empty() {
        let without = known.iter().filter(|e| e.lines.is_none()).count();
        t.insert(
            2,
            (
                Icon::Shield.into(),
                EN.compact_backups,
                if without == 0 {
                    String::new()
                } else {
                    fill(EN.compact_not_backed, &[&without.to_string()])
                },
                Action::Nav(Screen::Backups),
                true,
            ),
        );
    }
    if !app.stick.is_empty() && !app.holds_secret() {
        t.push((
            app.medium.icon(),
            app.medium.visit(),
            String::new(),
            Action::Nav(Screen::Visit),
            true,
        ));
    }
    t.push((
        Icon::Tools.into(),
        EN.compact_tools,
        String::new(),
        Action::Nav(Screen::Catalog),
        true,
    ));
    // The online app is the device's QR link.
    if app.online {
        t.push((
            Icon::Qr.into(),
            EN.compact_transfer,
            String::new(),
            Action::Nav(Screen::Transfer),
            true,
        ));
    }
    t.push((
        Icon::Settings.into(),
        EN.compact_settings,
        String::new(),
        Action::Nav(Screen::Settings),
        true,
    ));
    t
}

/// What Home puts above the menu: the same one lead job §1.2 ranks for
/// the wide Home (`crate::screens::home_lead_beside_tiles`), as one prompt row; a lead
/// that would open or start over a flow whose tile is above it gives way
/// to the next, or to none (`docs/FLOWS.md`, Flows in progress).
fn prompts(app: &View) -> Vec<(Glyph, String, Option<Action>, crate::ui::Paint)> {
    crate::screens::home_lead_beside_tiles(app)
        .map(|(icon, label, _sub, action, enabled)| {
            let tone = if enabled {
                Role::Primary.paint()
            } else {
                Role::Warning.paint()
            };
            (icon, label, enabled.then_some(action), tone)
        })
        .into_iter()
        .collect()
}

pub use faraday_ui::keyboard::OskPress;

/// The docked keyboard, at the foot of a small panel
/// while something on the page takes typing. A press is the key a
/// physical keyboard would send.
#[derive(Debug, Default)]
pub(crate) struct Osk {
    /// Shift and the symbol layer.
    keys: faraday_ui::keyboard::Keys,
    /// Brought up from the bar on a page that types with no field.
    manual: bool,
    /// Put away while a field has focus.
    hidden: bool,
    /// Which field had focus on the last frame, when one did.
    was_field: Option<String>,
    /// Where the press that focused that field was, pixels down the
    /// window: what the wide layout keeps in view above the keyboard.
    field_at: i32,
    /// The foot of the focused field's caret as the page would draw it
    /// unmoved, units down the window, when one was drawn.
    caret: Option<f32>,
    /// How far the wide layout drew the page up, units, on the last
    /// frame.
    lift: f32,
}

/// A small panel's own state: the docked keyboard and the walk-through
/// opened behind a tap. What decides whether typing goes to a field is
/// worked out by the glue on [`Faraday`], which reaches every area that
/// has one, and is given here as plain values.
#[derive(Debug, Default)]
pub(crate) struct CompactArea {
    /// The docked keyboard.
    osk: Osk,
    /// On a small panel, the walk-through shown: the screen, and the
    /// step's place or [`ABOUT_PAGE`]. Behind a tap there, so the
    /// controls keep the panel.
    about_open: Option<(Screen, u8)>,
}

impl CompactArea {
    /// The walk-through shown: the screen, and the step's place.
    pub(crate) fn about_open(&self) -> Option<(Screen, u8)> {
        self.about_open
    }

    /// A tap on a step's About on `screen`: opens it, or puts it away
    /// when it was open.
    pub(crate) fn toggle_about(&mut self, screen: Screen, k: u8) {
        let this = (screen, k);
        self.about_open = (self.about_open != Some(this)).then_some(this);
    }

    /// The walk-through to draw on `screen`: none behind a sheet or on
    /// another screen.
    pub(crate) fn about_on(&self, screen: Screen, sheet_none: bool) -> Option<u8> {
        self.about_open
            .filter(|(s, _)| *s == screen && sheet_none)
            .map(|(_, k)| k)
    }

    /// The keyboard's Shift and symbol layer.
    pub(crate) fn keys(&self) -> faraday_ui::keyboard::Keys {
        self.osk.keys
    }

    /// Whether the keyboard is up. On a small panel it rises by itself
    /// where a field takes typing (`auto`); in the wide layout it is up
    /// only once brought up by hand, and only while typing goes
    /// somewhere (`wanted`). `offered`: the page takes typing with no
    /// field.
    pub(crate) fn shown(&self, compact: bool, auto: bool, offered: bool, wanted: bool) -> bool {
        if compact {
            (auto && !self.osk.hidden) || (self.osk.manual && offered)
        } else {
            self.osk.manual && wanted
        }
    }

    /// A field newly focused, or another field than before, brings the
    /// keyboard up again. `field` tells fields apart, `down_y` is where
    /// the last press was, pixels down the window. Returns whether it
    /// came up on this frame.
    pub(crate) fn track(
        &mut self,
        field: Option<String>,
        down_y: i32,
        compact: bool,
        wanted: bool,
    ) -> bool {
        let rose = field.is_some() && field != self.osk.was_field;
        if rose {
            self.osk.hidden = false;
            self.osk.field_at = down_y;
        }
        self.osk.was_field = field;
        // In the wide layout the keyboard goes once nothing takes typing,
        // and does not come back by itself with the next field.
        if !compact && !wanted {
            self.osk.manual = false;
        }
        rose
    }

    /// How far the wide layout draws the page up while its keyboard is
    /// up (`height` tall), so the focused field (`auto`) stays in view
    /// above it, given `visible` and `f` pixels a unit.
    pub(crate) fn lift_now(&self, auto: bool, height: f32, visible: f32, f: f32) -> f32 {
        let foot = if auto {
            self.osk
                .caret
                .or(Some(self.osk.field_at as f32 / f.max(0.01) + 24.0))
        } else {
            self.osk.caret
        };
        foot.map_or(0.0, |y| (y + 24.0 - visible).clamp(0.0, height))
    }

    /// Keeps how far the frame drew the page up.
    pub(crate) fn set_lift(&mut self, lift: f32) {
        self.osk.lift = lift;
    }

    /// The caret the frame drew, where it would be unmoved.
    pub(crate) fn caret(&mut self, foot: Option<f32>, compact: bool, shown: bool) {
        self.osk.caret = foot.map(|y| y + self.osk.lift);
        if compact || !shown {
            self.osk.lift = 0.0;
        }
    }

    /// Leaving a page puts away a keyboard brought up by hand.
    pub(crate) fn leave(&mut self) {
        self.osk.manual = false;
        self.osk.keys.reset();
    }

    /// Done was pressed: the keyboard is put away, so the page's pinned
    /// action shows again; another field brings it back.
    pub(crate) fn done(&mut self) {
        self.osk.hidden = true;
        self.osk.manual = false;
    }

    /// A press other than Done: the key it types, if any. `address`:
    /// the address keyboard, whose shift is base58, not a capital, and
    /// stays on after a letter.
    pub(crate) fn press(&mut self, p: OskPress, address: bool) -> Option<faraday_shell_api::Key> {
        use faraday_shell_api::Key as K;
        use faraday_ui::keyboard::Typed;
        match p {
            OskPress::Done => None,
            OskPress::Show => {
                self.osk.manual = true;
                self.osk.hidden = false;
                None
            }
            OskPress::Hide => {
                self.osk.manual = false;
                self.osk.hidden = true;
                None
            }
            OskPress::Char(ch) if address => Some(K::Char(ch)),
            _ => match self.osk.keys.press(p) {
                Some(Typed::Char(c)) => Some(K::Char(c)),
                Some(Typed::Backspace) => Some(K::Backspace),
                Some(Typed::Enter) => Some(K::Enter),
                None => None,
            },
        }
    }
}

impl Faraday {
    /// On a small panel, the walk-through shown: the screen, and the
    /// step's place. For tests that tap an About.
    pub fn about_open(&self) -> Option<(Screen, u8)> {
        self.panel.about_open()
    }

    /// A field with focus takes typing now.
    fn osk_auto(&self) -> bool {
        // An empty passphrase offering its chip is not covered by the keys.
        if self.pass_chip_waiting() {
            return false;
        }
        if self.sheet == Some(crate::Sheet::Potential) {
            return self.inbox.potential().is_some_and(|p| p.typing);
        }
        if self.sheet == Some(crate::Sheet::Import) {
            return self.vaults.focus == Some(crate::vault::Focus::Passphrase);
        }
        if self.sheet == Some(crate::Sheet::Rename) {
            return self.visit.files_menu().renaming.is_some();
        }
        if self.sheet.is_some() {
            return false;
        }
        // The codex32 form's correction or a clash stands in place of the
        // field and takes presses, not typing, but for the boxes typed
        // again from the paper (`docs/CODEX32.md` §8.4, §8.5).
        if self.entry_typing_here() && crate::forms::codex32_pressed_only(self.entry.state()) {
            return false;
        }
        match self.screen {
            Screen::Entry => {
                self.entry.state().on_passphrase
                    || self.entry.state().form != crate::forms::Form::Words
                    || self.entry.state().keys.is_none()
            }
            // The vault's list has no field; its item page may.
            Screen::VaultContents => {
                self.vaults.focus.is_some() && crate::vault::screen::contents_detail(self)
            }
            Screen::Vanity => self.vanity.typing_prefix(),
            // Find an address's field.
            Screen::Explore => self
                .explore
                .page()
                .is_some_and(|p| p == crate::explore::Page::Find),
            // The Key card's passphrase, and the Words card of a key typed
            // as Add a key's form docks it; the box of rolls has buttons
            // of its own on a small panel.
            Screen::KeyGen => self.keygen.state().is_some_and(|k| {
                !k.locked
                    && ((k.focus.is_some() && k.open == Some(crate::keygen::kstep::KEY))
                        || (k.open == Some(crate::keygen::kstep::TYPED)
                            && (self.entry.state().form != crate::forms::Form::Words
                                || self.entry.state().keys.is_none())))
            }),
            _ => self.typing_field(),
        }
    }

    /// A page that takes typing with no field to focus: its bar offers
    /// the keyboard.
    pub(crate) fn osk_offered(&self) -> bool {
        self.sheet.is_none()
            && matches!(
                self.screen,
                Screen::Explore
                    | Screen::Bip85
                    | Screen::Catalog
                    | Screen::Tools
                    | Screen::Lightning
            )
    }

    /// Whether the keyboard is up. On a small panel it rises by itself
    /// where a field takes typing; in the wide layout it is up only once
    /// brought up by hand, and only while typing goes somewhere.
    pub(crate) fn osk_shown(&self) -> bool {
        self.panel.shown(
            self.compact,
            self.osk_auto(),
            self.osk_offered(),
            self.osk_wanted(),
        )
    }

    /// Typing goes somewhere on this page: a field has focus, or the page
    /// takes typing with no field. Where the wide layout offers its
    /// keyboard button.
    pub(crate) fn osk_wanted(&self) -> bool {
        self.osk_auto() || self.osk_offered()
    }

    /// Which of the keyboards: the path pad for a derivation
    /// path, else the passphrase keyboard, which types any printable
    /// character.
    fn osk_kind(&self) -> faraday_draw::widgets::keyboard::KeyboardKind {
        use faraday_draw::widgets::keyboard::KeyboardKind as K;
        match self.screen {
            Screen::Explore
                if self.sheet.is_none()
                    && self
                        .explore
                        .page()
                        .is_some_and(|p| p == crate::explore::Page::Find) =>
            {
                K::Address
            }
            Screen::Explore if self.sheet.is_none() => K::Path,
            // The codex32 form types on the codex32 keyboard, with `?`
            // while corrections are suggested (`docs/CODEX32.md` §8.4).
            _ if self.sheet.is_none()
                && self.entry_typing_here()
                && self.entry.state().form == crate::forms::Form::Codex32
                && !self.entry.state().on_passphrase =>
            {
                if self.entry.state().correction.takes_unreadable() {
                    K::Codex32Unreadable
                } else {
                    K::Codex32
                }
            }
            _ => K::Passphrase,
        }
    }

    /// The keyboard's height when it is up, else nothing.
    pub(crate) fn osk_height(&self) -> f32 {
        if self.osk_shown() {
            faraday_ui::keyboard::height(self.osk_kind())
        } else {
            0.0
        }
    }

    /// Which field typing goes to, as far as telling one from another:
    /// none when no field takes typing.
    fn osk_field(&self) -> Option<String> {
        self.osk_auto().then(|| {
            format!(
                "{:?} {:?} {:?} {} {:?} {:?} {:?}",
                self.screen,
                self.sheet,
                self.vaults.focus,
                self.entry.state().on_passphrase,
                self.keygen.state().and_then(|k| k.focus),
                self.create.state().and_then(|c| c.pass_focus),
                self.seed_read.state().and_then(|s| s.focus)
            )
        })
    }

    /// A field newly focused, or another field than before, brings the
    /// keyboard up again. Returns whether it came up on this frame.
    pub(crate) fn osk_track(&mut self) -> bool {
        let field = self.osk_field();
        let wanted = self.osk_wanted();
        self.panel
            .track(field, self.pointer.down_at().1, self.compact, wanted)
    }

    /// The keyboard just came up over the panel's foot: when the press
    /// that focused the field was under where the keyboard now is, the
    /// page scrolls that far, so the field stays in view. `f` is pixels a
    /// design unit; `visible` the height left above the keyboard.
    pub(crate) fn osk_reveal(&mut self, f: f32, visible: f32) {
        self.osk_reveal_at(self.pointer.down_at().1, f, visible);
    }

    fn osk_reveal_at(&mut self, at: i32, f: f32, visible: f32) {
        let tapped = at as f32 / f.max(0.01);
        let over = tapped + 56.0 - visible;
        if over > 0.0
            && let Some(slot) = self.scroll_slot()
        {
            *slot += over;
            self.out.dirty = true;
            self.out
                .commands
                .push_back(faraday_shell_api::Command::Draw);
        }
    }

    /// How far the wide layout draws the page up while its keyboard is
    /// up, so the focused field stays in view above it: the page keeps
    /// its own height and moves up no further than the keyboard is tall.
    /// `visible` is the height left above the keyboard; `f` pixels a unit.
    /// Drawing asks it through [`crate::drawing::View::osk_lift`], which
    /// keeps it for the caret after the pass ([`Self::osk_set_lift`]).
    pub(crate) fn osk_lift_now(&self, visible: f32, f: f32) -> f32 {
        self.panel
            .lift_now(self.osk_auto(), self.osk_height(), visible, f)
    }

    /// Keeps how far the frame drew the page up ([`Self::osk_lift_now`]).
    pub(crate) fn osk_set_lift(&mut self, lift: f32) {
        self.panel.set_lift(lift);
    }

    /// The caret the frame drew, where it would be unmoved.
    pub(crate) fn osk_caret(&mut self, foot: Option<f32>) {
        let shown = self.osk_shown();
        self.panel.caret(foot, self.compact, shown);
    }

    /// Leaving a page puts away a keyboard brought up by hand.
    pub(crate) fn osk_leave(&mut self) {
        self.panel.leave();
    }

    pub(crate) fn osk_press(&mut self, p: OskPress) {
        // Done is Enter, and puts the keyboard away.
        if p == OskPress::Done {
            self.key(faraday_shell_api::Key::Enter);
            self.panel.done();
            return;
        }
        let address = self.osk_kind() == faraday_draw::widgets::keyboard::KeyboardKind::Address;
        if let Some(k) = self.panel.press(p, address) {
            self.key(k);
        }
    }
}

/// The keyboard, across the foot of the panel from `y`, or in the wide
/// layout across the page area from `x`, `w` wide (`faraday-ui`'s).
pub(crate) fn keyboard(app: &View, ui: &mut Ui, x: f32, w: f32, y: f32) {
    faraday_ui::keyboard::keyboard(
        ui,
        x,
        w,
        y,
        app.osk_kind(),
        app.panel.keys(),
        app.compact,
        Action::Osk,
    );
}

impl Faraday {
    /// The top app bar's ⋮ menu on the bar's windows (`docs/LAYOUT.md`
    /// §3): `None` opens it; a screen closes it and goes there.
    pub(crate) fn nav_menu(&mut self, to: Option<Screen>) {
        match to {
            None => self.sheet = Some(crate::Sheet::NavMenu),
            Some(s) => {
                self.sheet = None;
                self.act_inner(Action::Nav(s));
            }
        }
    }

    /// Files by direction, as the sidebar's Files row reads it: "3 for
    /// the stick" while any wait, else "20 from the stick", else "none".
    pub fn files_count(&self) -> String {
        if !self.outbox.is_empty() {
            format!("{} {}", self.outbox.len(), self.medium.for_the())
        } else if !self.inbox.is_empty() {
            format!("{} {}", self.inbox.len(), self.medium.from_the())
        } else {
            EN.compact_none.to_string()
        }
    }

    /// Home's status line on a small panel: Files by direction, and
    /// what is attached, by its label or by how many.
    pub fn home_files_line(&self) -> String {
        let sticks = match self.stick.len() {
            0 => fill(EN.compact_no_medium, &[self.medium.noun()]),
            1 => self.stick[0].label.clone(),
            k => format!("{k} {}", self.medium.nouns()),
        };
        format!("{} · {sticks}", self.files_count())
    }
}
