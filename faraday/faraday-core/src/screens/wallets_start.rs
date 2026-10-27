//! `Screen::WalletsStart`: the Wallets tab's first page, every flow by what the person has.

use super::parts::FROM_KEY;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::{Session, key_line};
use crate::{Action, Screen};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, LABEL_M, TITLE_M};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{section_label, title, wrap_buttons};

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, cw - 96.0)
    };
    let mut y = if compact { 14.0 } else { 36.0 } - app.list_offset;
    if !compact {
        title(ui, x, y, EN.start_wallets);
        y += 60.0;
    }
    // No wallet and no vault yet: two ways in.
    let empty = app.session.wallets.is_empty()
        && app.vaults.open.is_empty()
        && !app.create_unfinished()
        && app.wallets.spend().is_none();
    if app.guided() && !compact {
        let text = if empty {
            fill(EN.start_no_wallet_loaded, &[app.medium.a()])
        } else {
            EN.start_start_from_what.to_string()
        };
        let lead_w = width.min(820.0);
        let used = ui.wrap_as((x, y, lead_w), BODY_M, Role::OnSurfaceVariant, &text);
        let link = crate::flow::learn_link(ui, x, y + used, lead_w, crate::learn::Topic::StartHere);
        y += used + link + 20.0;
    }
    section_label(ui, x, y, EN.start_your_wallets);
    y += if compact { 24.0 } else { 30.0 };
    y += wallet_rows(app, ui, x, y, width, 2, &Action::OpenWallet) + 18.0;
    // On the small panel this is the wallet list: a key in no wallet is
    // listed after the wallets with its way on, as on the desktop's
    // Wallets column. With no wallet loaded the wallet page says it.
    let (loose, _) = app.loose_keys();
    if compact && !app.session.wallets.is_empty() && !loose.is_empty() {
        section_label(ui, x, y, EN.start_keys_without);
        y += 24.0;
        for fp in &loose {
            let line = key_line(*fp, app.session.keys.key_label(*fp).unwrap_or(""));
            let line = ui.fit_as(BODY_M.mono(), &line, width);
            ui.text_mid_as((x, y, 30.0), BODY_M.mono(), Role::OnSurface, &line);
            y += 34.0;
            ui.button(
                x,
                y,
                Some(width),
                BUTTON_H,
                FROM_KEY,
                Style::Secondary,
                Action::KeyWallet(fp.0, 1),
            );
            y += 56.0;
        }
        y += 6.0;
    }
    // Wallets loaded: the list and the wallet card carry every job
    // (Sign a transaction, Back up, Show wallet QR, Sign a message) and
    // Tools has the rest; Add a key stays under the list.
    // `docs/history/SIMPLIFY.md` §1.3.
    // Spend from a backup stays offered with wallets loaded: it is the
    // way back into a spend left for another tab, which lights Wallets.
    if !empty {
        if compact {
            y += crate::compact::screen::row(
                ui,
                x,
                y,
                width,
                Some(Icon::Sign),
                EN.start_spend_backup,
                EN.start_a_stick_words,
                Role::OnSurfaceVariant.paint(),
                Some(Action::Nav(Screen::Family)),
            );
            y += crate::compact::screen::row(
                ui,
                x,
                y,
                width,
                Some(Icon::Keys),
                EN.start_add_a_key,
                EN.start_keys_first_wallets,
                Role::OnSurfaceVariant.paint(),
                Some(Action::KeyGen(None)),
            );
        } else {
            y += wrap_buttons(
                ui,
                x,
                y,
                width,
                BUTTON_H,
                &[
                    (
                        EN.start_spend_backup,
                        Style::Secondary,
                        Action::Nav(Screen::Family),
                    ),
                    (EN.start_add_a_key, Style::Secondary, Action::KeyGen(None)),
                ],
            ) + 10.0;
        }
        if compact {
            crate::compact::screen::finish(app, ui, x0, cw, h, y + 16.0 + app.list_offset);
        } else {
            app.scroll.content_h().set(y + app.list_offset + 24.0);
            let view = ui.rect(x0, 0.0, cw, h);
            ui.report_scroll(view, app.scroll.content_h().get() - h);
        }
        return;
    }
    if empty && compact {
        y += crate::compact::screen::row(
            ui,
            x,
            y,
            width,
            Some(Icon::Flag),
            EN.start_make_a_new_wallet,
            EN.start_create_line,
            Role::OnSurfaceVariant.paint(),
            Some(Action::CreateWallet),
        );
        y += crate::compact::screen::row(
            ui,
            x,
            y,
            width,
            Some(Icon::Wallet),
            EN.start_restore_an_existing,
            EN.start_from_its_backups,
            Role::OnSurfaceVariant.paint(),
            Some(Action::RestoreWallet),
        );
        y += crate::compact::screen::row(
            ui,
            x,
            y,
            width,
            Some(Icon::Sign),
            EN.start_spend_backup,
            EN.start_a_stick_words,
            Role::OnSurfaceVariant.paint(),
            Some(Action::Nav(Screen::Family)),
        );
        y += crate::compact::screen::row(
            ui,
            x,
            y,
            width,
            Some(Icon::Keys),
            EN.start_add_a_key,
            EN.start_keys_first_wallets,
            Role::OnSurfaceVariant.paint(),
            Some(Action::KeyGen(None)),
        );
        y += 8.0;
        let files = app.vault_files();
        if let Some(i) = files.iter().position(|f| f.open.is_none()) {
            let line = fill(EN.start_is_in_files_locked, &[&files[i].name.to_string()]);
            y += crate::compact::screen::row(
                ui,
                x,
                y,
                width,
                Some(Icon::Lock),
                EN.start_unlock,
                &line,
                Role::OnSurfaceVariant.paint(),
                Some(Action::Vault(crate::vault::VaultAction::Open(i))),
            );
        }
        crate::compact::screen::finish(app, ui, x0, cw, h, y + app.list_offset);
        return;
    }
    if empty {
        let gap = 16.0;
        // Four ways in (`docs/history/SIMPLIFY.md` §6.2): Create, Load or
        // restore, Spend from a backup, and Add a key.
        let tw = (width - 3.0 * gap) / 4.0;
        let ways = [
            (
                Icon::Flag,
                EN.start_make_a_new_wallet,
                EN.start_create_line,
                Action::CreateWallet,
            ),
            (
                Icon::Wallet,
                EN.start_restore_an_existing,
                EN.start_from_its_backups,
                Action::RestoreWallet,
            ),
            (
                Icon::Sign,
                EN.start_spend_backup,
                EN.start_a_stick_words,
                Action::Nav(Screen::Family),
            ),
            (
                Icon::Keys,
                EN.start_add_a_key,
                EN.start_keys_first_wallets,
                Action::KeyGen(None),
            ),
        ];
        // A label or sub-line that does not fit at its size breaks onto
        // further lines (`docs/DESIGN.md` "Menu row"): every card in the
        // row grows to the tallest label and sub among them, so the row
        // stays even and nothing is cut off.
        let label_room = tw - 100.0;
        let label_line_h = ui.line_as(TITLE_M);
        let sub_line_h = ui.line_as(BODY_S);
        let max_label_lines = ways
            .iter()
            .map(|(_, label, _, _)| ui.wrap_lines_as(TITLE_M, label, label_room))
            .max()
            .unwrap_or(1);
        let max_sub_lines = ways
            .iter()
            .map(|(_, _, sub, _)| ui.wrap_lines_as(BODY_S, sub, label_room))
            .max()
            .unwrap_or(1);
        let card_h = 92.0
            + (max_label_lines - 1) as f32 * label_line_h
            + (max_sub_lines - 1) as f32 * sub_line_h;
        for (k, (icon, label, sub, action)) in ways.iter().enumerate() {
            let tx = x + k as f32 * (tw + gap);
            let pressed = ui.is_pressed(*action);
            crate::ui::fill_pressed(
                ui,
                (tx, y, tw, card_h, 12.0),
                Role::SurfaceContainerLow,
                pressed,
            );
            ui.stroke(tx, y, tw, card_h, 12.0, Role::OutlineVariant);
            ui.fill(
                tx + 20.0,
                y + 24.0,
                44.0,
                44.0,
                10.0,
                Role::SecondaryContainer,
            );
            let on_tile = Role::OnSecondaryContainer;
            ui.icon(tx + 20.0, y + 24.0, 44.0, *icon, 18.0, on_tile);
            if ui.wrap_lines_as(TITLE_M, label, label_room) > 1 {
                ui.wrap_as(
                    (tx + 82.0, y + 24.0, label_room),
                    TITLE_M,
                    Role::OnSurface,
                    label,
                );
            } else {
                let label = ui.fit_as(TITLE_M, label, label_room);
                ui.text_as(tx + 82.0, y + 24.0, TITLE_M, Role::OnSurface, &label);
            }
            let sub_y = y + 24.0 + max_label_lines as f32 * label_line_h + 2.0;
            if ui.wrap_lines_as(BODY_S, sub, label_room) > 1 {
                ui.wrap_as(
                    (tx + 82.0, sub_y, label_room),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    sub,
                );
            } else {
                let sub = ui.fit_as(BODY_S, sub, label_room);
                ui.text_as(tx + 82.0, sub_y, BODY_S, Role::OnSurfaceVariant, &sub);
            }
            ui.hit(tx, y, tw, card_h, *action);
        }
        y += card_h + 28.0;
        let files = app.vault_files();
        if let Some(i) = files.iter().position(|f| f.open.is_none()) {
            let line = fill(EN.start_is_in_files_locked, &[&files[i].name.to_string()]);
            ui.text_mid_as((x, y, 40.0), BODY_M, Role::OnSurfaceVariant, &line);
            let lw = ui.measure_as(BODY_M, &line);
            ui.button(
                x + lw + 16.0,
                y,
                None,
                BUTTON_H,
                EN.start_unlock,
                Style::Secondary,
                Action::Vault(crate::vault::VaultAction::Open(i)),
            );
        }
    }
}

/// The loaded wallets, two to a row, at most `rows` rows: each with its
/// shape and whether the seeds here sign for it. Returns its height.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn wallet_rows(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    width: f32,
    rows: usize,
    open: &dyn Fn(usize) -> Action,
) -> f32 {
    if app.session.wallets.is_empty() {
        ui.text_mid_as(
            (x, y, 30.0),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.wallet_rows_none_loaded,
        );
        return 38.0;
    }
    let total = app.session.wallets.len();
    if ui.compact {
        let mut order: Vec<usize> = (0..total).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(app.readiness(i)));
        let mut cy = y;
        for &i in &order {
            let w = &app.session.wallets[i];
            let ready = app.readiness(i);
            let (m, _) = Session::quorum(w);
            let here = app
                .session
                .slots(w)
                .iter()
                .filter(|s| s.held_by.is_some())
                .count()
                .min(m);
            let (state, tone) = match ready {
                _ if app.silent_differs(i) => {
                    (EN.wallets_does_not_match.to_string(), Role::Error.paint())
                }
                2 => (
                    fill(
                        EN.wallet_rows_can_spend,
                        &[&here.to_string(), &m.to_string()],
                    ),
                    Role::Ok.paint(),
                ),
                1 => (
                    fill(EN.wallet_rows_of_here, &[&here.to_string(), &m.to_string()]),
                    Role::OnSurface.paint(),
                ),
                _ => (
                    EN.wallet_rows_watch_only.to_string(),
                    Role::OnSurfaceVariant.paint(),
                ),
            };
            let sub = format!("{} · {state}", Session::shape(w));
            cy += crate::compact::screen::row(
                ui,
                x,
                cy,
                width,
                None,
                &w.name,
                &sub,
                tone,
                Some(open(i)),
            );
        }
        return cy - y;
    }
    let gap = 16.0;
    let cols = 2;
    let rw = (width - gap) / 2.0;
    // Ready wallets first, as the corner lists them.
    let mut order: Vec<usize> = (0..total).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(app.readiness(i)));
    let shown = (rows * cols).min(total);
    for (k, &i) in order.iter().enumerate().take(shown) {
        let w = &app.session.wallets[i];
        let rx = x + (k % cols) as f32 * (rw + gap);
        let ry = y + (k / cols) as f32 * 52.0;
        let action = open(i);
        let ready = app.readiness(i);
        ui.fill(
            rx,
            ry,
            rw,
            44.0,
            10.0,
            if ui.is_pressed(action) {
                Role::SurfaceContainerHighest
            } else {
                Role::Surface
            },
        );
        ui.stroke(
            rx,
            ry,
            rw,
            44.0,
            10.0,
            if ready == 2 {
                Role::Ok.with_alpha(90)
            } else {
                Role::OutlineVariant.paint()
            },
        );
        let (m, _) = Session::quorum(w);
        let here = app
            .session
            .slots(w)
            .iter()
            .filter(|s| s.held_by.is_some())
            .count()
            .min(m);
        // The marker on the right: ready, how many here, or none.
        let differs = app.silent_differs(i);
        let marker = match ready {
            _ if differs => EN.wallets_does_not_match.to_string(),
            2 => fill(
                EN.wallet_rows_can_spend,
                &[&here.to_string(), &m.to_string()],
            ),
            1 => fill(EN.wallet_rows_of_here, &[&here.to_string(), &m.to_string()]),
            _ => EN.wallet_rows_watch_only.to_string(),
        };
        let tone = match ready {
            _ if differs => Role::Error,
            2 => Role::OnOkContainer,
            1 => Role::OnSurface,
            _ => Role::OnSurfaceVariant,
        };
        let mw = ui.measure_as(LABEL_M, &marker) + 20.0;
        if ready == 2 {
            ui.fill(
                rx + rw - 10.0 - mw,
                ry + 10.0,
                mw,
                24.0,
                12.0,
                Role::OkContainer,
            );
        }
        ui.text_mid_as((rx + rw - mw, ry, 44.0), LABEL_M, tone, &marker);
        let name = ui.fit_as(LABEL_L, &w.name, rw * 0.4);
        let nw = ui.text_mid_as((rx + 14.0, ry, 44.0), LABEL_L, Role::OnSurface, &name);
        let shape = ui.fit_as(BODY_S, &Session::shape(w), rw - nw - mw - 50.0);
        ui.text_mid_as(
            (rx + 26.0 + nw, ry, 44.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &shape,
        );
        ui.hit(rx, ry, rw, 44.0, action);
    }
    let mut used = shown.div_ceil(cols) as f32 * 52.0;
    if shown < total {
        let more = fill(EN.wallet_rows_more, &[&(total - shown).to_string()]);
        let mw = ui.measure_as(LABEL_M, &more) + 8.0;
        ui.text_mid_as((x, y + used - 4.0, 24.0), LABEL_M, Role::Primary, &more);
        ui.hit(x, y + used - 4.0, mw, 24.0, open(order[shown]));
        used += 24.0;
    }
    used + 6.0
}
