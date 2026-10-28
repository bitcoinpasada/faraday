//! The Spend tab, drawn: one step-card column (`flow.rs`) of the tab's
//! pages and the spend's own steps (`docs/SPEND-FROM-BACKUP.md`). Each card's lead
//! is always shown; its More about this is folded.

use crate::strings::EN;
use crate::strings::fill;
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, LABEL_M, TITLE_M};
use faraday_draw::tokens::{FIELD_H, GAP, GAP_SMALL, PROGRESS_H};
use faraday_draw::widgets::Icon;

use crate::drawing::{Scrolled, View};
use crate::family::text;
use crate::family::{Came, CardId, FamilyAction as F, Question, more_bit, page};
use crate::screens::{next_button, step_body, step_summary};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::{VaultAction, cost_text, file_text, mib_text};
use crate::wallet::{FileKind, Kind, Session, fp_text, step};
use crate::{Action, Faraday, flow};
use faraday_ui::Status;

fn fa(a: F) -> Action {
    Action::Family(a)
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if app.family_overview() && ui.compact {
        loaded_compact(app, ui, cw, h);
        return;
    }
    if app.family_overview() {
        let content = loaded(app, ui, x0, cw);
        // Scrolled no further than its end, in whole pixels: the frame
        // rounds an offset to whole pixels, and an end between two would
        // be passed again on every frame at a scale that is not whole.
        let max = ((content - h).max(0.0) * ui.f).floor() / ui.f;
        if crate::flow::past_end(app.family.state.scroll.y, max) {
            let to = flow::Scroll {
                y: max,
                ..app.family.state.scroll
            };
            app.scroll(Scrolled::Family, to, true);
        }
        return;
    }
    let col_x = x0 + 40.0;
    let col_w = (cw - 80.0).min(900.0);
    let ids = app.family_cards();
    let open = app.family_open_card();
    let cards: Vec<flow::Card> = ids
        .iter()
        .map(|&id| flow::Card {
            title: text::title(app, id),
            summary: summary(app, id),
            mono: matches!(id, CardId::Step(step::TXID)),
            done: done(app, id),
            open: open == Some(id),
            default: false,
            toggle: crate::family::toggle(app, id),
            guide: Some(text::lead(app, id)).filter(|s| !s.is_empty()),
            // Why the summary is read before signing is Learn's.
            learn: (id == CardId::Step(step::TRANSACTION))
                .then_some(crate::learn::Topic::Verifying),
        })
        .collect();
    // From a wallet's card: the wallet's name, and no word on the
    // walk-through.
    let from = app
        .family_wallet()
        .filter(|_| app.family.state.from_card)
        .map(|i| {
            fill(
                EN.family_spend_from_n,
                &[&app.session.wallets[i].name.to_string()],
            )
        });
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: col_x,
        w: col_w,
        h,
        // Opened from anywhere else, the way back is where it was opened
        // from, by the trail (F77).
        back: app
            .family_wallet()
            .filter(|_| app.family.state.from_card)
            .map(|i| (EN.family_wallets, Action::OpenWallet(i)))
            .or(match app.family.state.from {
                Some(crate::Screen::Home) => {
                    Some((EN.family_home, Action::Nav(crate::Screen::Home)))
                }
                Some(crate::Screen::WalletsStart) => {
                    Some((EN.family_wallets, Action::Nav(crate::Screen::WalletsStart)))
                }
                _ => app
                    .trail_name()
                    .map(|_| (EN.family_wallets, Action::Nav(crate::Screen::WalletsStart))),
            })
            .map(|b| app.named_back(b)),
        heading: from.as_deref().unwrap_or(EN.family_spend),
        guided: true,
        switch: false,
        note: from.is_none().then_some(EN.family_this_page_takes),
        chip: None,
        chip_tap: None,
        cancel: app.cancel_offered().is_some(),
    };
    let scroll = app.family.state.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut draw_body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            body(app_ref, ui, ids[i], x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut draw_body)
    };
    app.scroll(Scrolled::Family, next, again);
}

/// The tab with something loaded already: each wallet and what it needs
/// next, seeds no wallet uses, and the walk-through one press away.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn loaded(app: &View, ui: &mut Ui, x0: f32, cw: f32) -> f32 {
    let x = x0 + 48.0;
    let w = (cw - 96.0).min(980.0);
    let scrolled = app.family.state.scroll.y;
    let mut y = 36.0 - scrolled;
    // Opened from another screen: the way back to it, over the title, so
    // the page is the same height either way (F77).
    if let Some(name) = app.trail_name() {
        let at = y - faraday_draw::tokens::BACK_LINK_ROW;
        crate::vault::screen::back_link(app, ui, x, at, (name, Action::Back));
    }
    crate::screens::title(ui, x, y, EN.family_spend);
    let lw = ui.measure_as(LABEL_L, EN.family_walk_through) + 32.0;
    ui.button(
        x + w - lw,
        y,
        Some(lw),
        BUTTON_H,
        EN.family_walk_through,
        Style::Ghost,
        fa(F::Walkthrough),
    );
    y += 64.0;

    // A transaction being signed comes first.
    if let Some(s) = app.wallets.spend() {
        let name = s
            .wallet
            .and_then(|i| app.session.wallets.get(i))
            .map_or(EN.family_no_wallet_loaded.to_string(), |w| w.name.clone());
        y += next_row(
            ui,
            x,
            y,
            w,
            Icon::Sign,
            &fill(
                EN.family_signing_wallet,
                &[&s.spend.source.to_string(), &name.to_string()],
            ),
            Some((
                EN.family_continue_signing,
                s.wallet
                    .map_or(Action::Nav(crate::Screen::Spend), |i| fa(F::SpendFrom(i))),
            )),
        );
    }

    crate::screens::section_label(ui, x, y, EN.family_wallets_loaded);
    y += ui.advance_as(faraday_draw::tokens::TITLE_S, 30.0);
    if app.session.wallets.is_empty() {
        ui.text_mid_as(
            (x, y, 30.0),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.family_none_loaded,
        );
        y += 42.0;
    }
    for (i, wl) in app.session.wallets.iter().enumerate() {
        let slots = app.session.slots(wl);
        let here = slots.iter().filter(|s| s.held_by.is_some()).count();
        let need = crate::wallet::needed(wl);
        let ready = here >= need;
        let ch = if ready { 64.0 } else { 108.0 };
        ui.card(x, y, w, ch, Role::OutlineVariant);
        ui.icon(
            x + 18.0,
            y + 14.0,
            20.0,
            Icon::Wallet,
            12.0,
            Role::OnSurfaceVariant,
        );
        let nw = ui.text_mid_as(
            (x + 46.0, y + 4.0, 40.0),
            TITLE_M,
            Role::OnSurface,
            &wl.name,
        );
        ui.text_mid_as(
            (x + 60.0 + nw, y + 4.0, 40.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &Session::shape(wl),
        );
        let (marker, tone) = if ready {
            (
                fill(
                    EN.family_can_spend_count,
                    &[&here.min(need).to_string(), &need.to_string()],
                ),
                Role::Ok,
            )
        } else if here > 0 {
            (
                fill(
                    EN.family_here_needed,
                    &[
                        &here.to_string(),
                        (if here == 1 {
                            EN.family_key
                        } else {
                            EN.family_keys
                        }),
                        &need.to_string(),
                    ],
                ),
                Role::Warning,
            )
        } else {
            (EN.family_no_key_here.to_string(), Role::OnSurfaceVariant)
        };
        ui.text_mid_as((x + 46.0, y + 30.0, 28.0), LABEL_M, tone, &marker);
        let label = if ready {
            EN.family_spend_from_button
        } else {
            EN.family_open_button
        };
        let bw = ui.measure_as(LABEL_L, label) + 36.0;
        ui.button(
            x + w - 16.0 - bw,
            y + 12.0,
            Some(bw),
            BUTTON_H,
            label,
            if ready {
                Style::Primary
            } else {
                Style::Secondary
            },
            if ready {
                fa(F::SpendFrom(i))
            } else {
                Action::OpenWallet(i)
            },
        );
        if !ready {
            // The keys still missing: where they may be.
            let used = crate::screens::missing_keys_line(
                app,
                ui,
                &slots,
                crate::Screen::Family,
                x + 46.0,
                y + 60.0,
                w - 62.0,
            );
            if used == 0.0 {
                ui.text_mid_as(
                    (x + 46.0, y + 62.0, 40.0),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.family_an_open_vault,
                );
            }
        }
        y += ch + 10.0;
    }

    // Seeds that open no wallet loaded.
    let lonely: Vec<&faraday_keys::KeyInfo> = app
        .session
        .keys
        .all()
        .filter(|k| {
            let fp = k.fingerprint;
            !app.session.wallets.iter().any(|w| {
                app.session
                    .slots(w)
                    .iter()
                    .any(|s| s.fingerprint == Some(fp))
            })
        })
        .collect();
    if !lonely.is_empty() {
        y += 8.0;
        crate::screens::section_label(ui, x, y, EN.family_keys_without_wallet);
        y += ui.advance_as(faraday_draw::tokens::TITLE_S, 30.0);
        for k in &lonely {
            ui.text_mid_as(
                (x, y, 30.0),
                BODY_M.mono(),
                Role::OnSurface,
                &fp_text(k.fingerprint),
            );
            ui.text_mid_as(
                (x + 100.0, y, 30.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &k.label,
            );
            y += 32.0;
        }
        let file = app.inbox.iter().position(|i| i.kind == FileKind::Wallet);
        let mut bx = x;
        if let Some(k) = file {
            bx += ui.button(
                bx,
                y + 4.0,
                None,
                BUTTON_H,
                &fill(EN.family_open, &[&app.inbox[k].name.to_string()]),
                Style::Primary,
                fa(F::UseWallet(k)),
            ) + 8.0;
        }
        bx += ui.button(
            bx,
            y + 4.0,
            None,
            BUTTON_H,
            EN.family_restore_wallet,
            Style::Secondary,
            Action::RestoreWallet,
        ) + 8.0;
        ui.button(
            bx,
            y + 4.0,
            None,
            BUTTON_H,
            EN.family_spend_as_a_single,
            Style::Secondary,
            fa(F::SeedWallet),
        );
        y += 50.0;
    }
    y + scrolled + 32.0
}

/// [`loaded`] on a small panel: under the page's bar, one column, each
/// card's button under what it says.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn loaded_compact(app: &View, ui: &mut Ui, cw: f32, h: f32) {
    use crate::compact::screen::measured;
    use crate::compact::{BAR_H, M};
    use crate::screens::{section_label, wrap_buttons};
    let (x, w) = (M, cw - 2.0 * M);
    let clip = ui.rect(0.0, BAR_H, cw, h - BAR_H);
    ui.c.push_clip(clip);
    let top = BAR_H + 12.0 - app.family.state.scroll.y;
    let mut y = top;
    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        36.0,
        &[(EN.family_walk_through, Style::Ghost, fa(F::Walkthrough))],
    ) + 4.0;

    // A transaction being signed comes first.
    if let Some(s) = app.wallets.spend() {
        let name = s
            .wallet
            .and_then(|i| app.session.wallets.get(i))
            .map_or(EN.family_no_wallet_loaded.to_string(), |w| w.name.clone());
        let line = fill(
            EN.family_signing_wallet,
            &[&s.spend.source.to_string(), &name.to_string()],
        );
        let lh = measured(ui, |ui| {
            ui.wrap_as(
                (x + 46.0, y + 14.0, w - 60.0),
                LABEL_L,
                Role::OnSurface,
                &line,
            )
        });
        let ch = lh + 14.0 + 12.0 + 40.0 + 14.0;
        ui.card(x, y, w, ch, Role::OutlineVariant);
        ui.icon(x + 14.0, y + 14.0, 20.0, Icon::Sign, 12.0, Role::Primary);
        ui.wrap_as(
            (x + 46.0, y + 14.0, w - 60.0),
            LABEL_L,
            Role::OnSurface,
            &line,
        );
        ui.button(
            x + 14.0,
            y + lh + 26.0,
            Some(w - 28.0),
            BUTTON_H,
            EN.family_continue_signing,
            Style::Primary,
            s.wallet
                .map_or(Action::Nav(crate::Screen::Spend), |i| fa(F::SpendFrom(i))),
        );
        y += ch + 16.0;
    }

    section_label(ui, x, y, EN.family_wallets_loaded);
    y += ui.advance_as(faraday_draw::tokens::TITLE_S, 30.0);
    if app.session.wallets.is_empty() {
        ui.text_mid_as(
            (x, y, 30.0),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.family_none_loaded,
        );
        y += 42.0;
    }
    for (i, wl) in app.session.wallets.iter().enumerate() {
        y += wallet_card_compact(app, ui, (x, y, w), i, wl) + 10.0;
    }

    // Seeds that open no wallet loaded.
    let lonely: Vec<&faraday_keys::KeyInfo> = app
        .session
        .keys
        .all()
        .filter(|k| {
            let fp = k.fingerprint;
            !app.session.wallets.iter().any(|w| {
                app.session
                    .slots(w)
                    .iter()
                    .any(|s| s.fingerprint == Some(fp))
            })
        })
        .collect();
    if !lonely.is_empty() {
        y += 8.0;
        section_label(ui, x, y, EN.family_keys_without_wallet);
        y += ui.advance_as(faraday_draw::tokens::TITLE_S, 30.0);
        for k in &lonely {
            ui.text_mid_as(
                (x, y, 30.0),
                BODY_M.mono(),
                Role::OnSurface,
                &fp_text(k.fingerprint),
            );
            let label = ui.fit_as(BODY_S, &k.label, w - 100.0);
            ui.text_mid_as((x + 100.0, y, 30.0), BODY_S, Role::OnSurfaceVariant, &label);
            y += 32.0;
        }
        let file = app.inbox.iter().position(|i| i.kind == FileKind::Wallet);
        let open_label = file.map(|k| fill(EN.family_open, &[&app.inbox[k].name.to_string()]));
        let mut items: Vec<(&str, Style, Action)> = Vec::new();
        if let (Some(k), Some(l)) = (file, open_label.as_deref()) {
            items.push((l, Style::Primary, fa(F::UseWallet(k))));
        }
        items.push((
            EN.family_restore_wallet,
            Style::Secondary,
            Action::RestoreWallet,
        ));
        items.push((
            EN.family_spend_as_a_single,
            Style::Secondary,
            fa(F::SeedWallet),
        ));
        y += 4.0 + wrap_buttons(ui, x, y + 4.0, w, 38.0, &items);
    }
    ui.c.pop_clip();
    let content = y - top + 24.0;
    let max = (content - (h - BAR_H)).max(0.0);
    ui.report_scroll(clip, max);
    if crate::flow::past_end(app.family.state.scroll.y, max) {
        let to = flow::Scroll {
            y: max,
            ..app.family.state.scroll
        };
        app.scroll(Scrolled::Family, to, true);
    }
    crate::compact::bar(
        app,
        ui,
        cw,
        (EN.family_home, Action::Nav(crate::Screen::Home)),
        EN.family_spend,
    );
}

/// What a loaded wallet's card says of its keys here: can spend, how
/// many are here of how many needed, or none, and its tone.
fn keys_marker(here: usize, need: usize) -> (String, Role) {
    if here >= need {
        (
            fill(
                EN.family_can_spend_count,
                &[&here.min(need).to_string(), &need.to_string()],
            ),
            Role::Ok,
        )
    } else if here > 0 {
        let keys = if here == 1 {
            EN.family_key
        } else {
            EN.family_keys
        };
        (
            fill(
                EN.family_here_needed,
                &[&here.to_string(), keys, &need.to_string()],
            ),
            Role::Warning,
        )
    } else {
        (EN.family_no_key_here.to_string(), Role::OnSurfaceVariant)
    }
}

/// A loaded wallet's card on a one-column page, `i` the wallet `wl`,
/// from (x, y) `w` wide: its name, shape and keys' marker, where the keys
/// still missing may be, and Spend from it or Open. Its lines advance by
/// their roles, so the card grows with the text size
/// (`docs/TEXT-SIZE.md` §5). Returns its height.
fn wallet_card_compact(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    i: usize,
    wl: &crate::wallet::Wallet,
) -> f32 {
    use crate::compact::screen::measured;
    use crate::screens::missing_keys_line;
    let slots = app.session.slots(wl);
    let here = slots.iter().filter(|s| s.held_by.is_some()).count();
    let need = crate::wallet::needed(wl);
    let ready = here >= need;
    let (marker, tone) = keys_marker(here, need);
    // The keys still missing: where they may be. Measured first so
    // the card is drawn at its height.
    let missing = |ui: &mut Ui, y: f32| -> f32 {
        if ready {
            return 0.0;
        }
        let used = missing_keys_line(
            app,
            ui,
            &slots,
            crate::Screen::Family,
            x + 18.0,
            y,
            w - 32.0,
        );
        if used > 0.0 {
            used
        } else {
            ui.wrap_as(
                (x + 14.0, y, w - 28.0),
                BODY_S,
                Role::OnSurfaceVariant,
                EN.family_an_open_vault,
            ) + 8.0
        }
    };
    // What larger text adds to each line: nothing at Standard.
    let more = 1.0 - 1.0 / ui.text_factor();
    let (dn, ds, dm) = (
        ui.line_as(TITLE_M) * more,
        ui.line_as(BODY_S) * more,
        ui.line_as(LABEL_M) * more,
    );
    let head = 70.0 + dn + ds + dm;
    let bh = ui.button_height(BUTTON_H);
    let mh = measured(ui, |ui| missing(ui, y));
    let ch = head + mh + bh + 14.0;
    ui.card(x, y, w, ch, Role::OutlineVariant);
    let ib = ui.row_icon(20.0);
    let icon = ui.row_icon(12.0);
    ui.icon(
        x + 14.0,
        y + 12.0,
        ib,
        Icon::Wallet,
        icon,
        Role::OnSurfaceVariant,
    );
    let name = ui.fit_as(TITLE_M, &wl.name, w - 60.0);
    let band = (x + 42.0, y + 4.0, 36.0 + dn);
    ui.text_mid_as(band, TITLE_M, Role::OnSurface, &name);
    let shape = ui.fit_as(BODY_S, &Session::shape(wl), w - 56.0);
    let band = (x + 42.0, y + 28.0 + dn, 18.0 + ds);
    ui.text_mid_as(band, BODY_S, Role::OnSurfaceVariant, &shape);
    let marker = ui.fit_as(LABEL_M, &marker, w - 56.0);
    let band = (x + 42.0, y + 46.0 + dn + ds, 18.0 + dm);
    ui.text_mid_as(band, LABEL_M, tone, &marker);
    missing(ui, y + head);
    let label = if ready {
        EN.family_spend_from_button
    } else {
        EN.family_open_button
    };
    // The button's top where its grown height starts.
    let by = y + head + mh + (bh - BUTTON_H) / 2.0;
    let (style, action) = if ready {
        (Style::Primary, fa(F::SpendFrom(i)))
    } else {
        (Style::Secondary, Action::OpenWallet(i))
    };
    ui.button(x + 14.0, by, Some(w - 28.0), BUTTON_H, label, style, action);
    ch
}

/// One line of what to do next, with its button. Returns its height.
#[allow(clippy::too_many_arguments)]
fn next_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    icon: Icon,
    line: &str,
    button: Option<(&str, Action)>,
) -> f32 {
    ui.card(x, y, w, 60.0, Role::OutlineVariant);
    ui.icon(x + 18.0, y + 20.0, 20.0, icon, 12.0, Role::Primary);
    let line = ui.fit_as(LABEL_L, line, w - 260.0);
    ui.text_mid_as((x + 46.0, y, 60.0), LABEL_L, Role::OnSurface, &line);
    if let Some((label, action)) = button {
        let bw = ui.measure_as(LABEL_L, label) + 36.0;
        ui.button(
            x + w - 16.0 - bw,
            y + 10.0,
            Some(bw),
            BUTTON_H,
            label,
            Style::Primary,
            action,
        );
    }
    76.0
}

fn done(app: &View, id: CardId) -> bool {
    match id {
        CardId::Page(p) => app.family_page_done(p),
        CardId::Step(n) => app.wallets.spend().is_some_and(|s| s.done[n as usize]),
        CardId::Later(_) => false,
    }
}

fn summary(app: &View, id: CardId) -> String {
    let s = match id {
        CardId::Page(page::MAP) => EN.family_six_stops,
        CardId::Page(page::SAFE) if app.online => EN.family_desktop_copy,
        CardId::Page(page::SAFE) => {
            return fill(EN.family_running_from_medium, &[app.medium.noun()]);
        }
        CardId::Page(page::HOLDING | page::OPEN) => {
            return match app.family_wallet().map(|i| &app.session.wallets[i]) {
                Some(w) => wallet_once(w),
                None => match came_rows(app).len() {
                    0 => EN.family_nothing_in_yet.to_string(),
                    1 => EN.family_things_one.to_string(),
                    n => fill(EN.family_things_count, &[&n.to_string()]),
                },
            };
        }
        CardId::Page(page::CHECK)
            if app.family.state.from_card && app.family_page_done(page::CHECK) =>
        {
            EN.family_loaded_in_sparrow
        }
        CardId::Page(page::CHECK) if app.family_page_done(page::CHECK) => EN.family_money_shows,
        // From the wallet's card Write the payment brings the PSBT too.
        CardId::Page(page::WRITE) if app.family.state.from_card => {
            return match app.wallets.spend() {
                Some(s) => fill(EN.family_signing, &[&s.spend.source.to_string()]),
                None => String::new(),
            };
        }
        CardId::Page(page::WRITE) if app.family_page_done(page::WRITE) => EN.family_done_in_sparrow,
        CardId::Page(page::BRING) => {
            return match app.wallets.spend() {
                Some(s) => fill(EN.family_signing, &[&s.spend.source.to_string()]),
                None => EN.family_not_here_yet.to_string(),
            };
        }
        CardId::Page(page::AWAY) => {
            return match app.outbox.len() {
                0 => fill(EN.family_nothing_waits, &[app.medium.for_the()]),
                1 => fill(EN.family_1_file, &[app.medium.for_the()]),
                n => fill(EN.family_files, &[&n.to_string(), app.medium.for_the()]),
            };
        }
        CardId::Page(_) => "",
        CardId::Step(n) => {
            return match app.wallets.spend() {
                Some(s) => step_summary(app, n, s.wallet, app.spend_needed()),
                None => String::new(),
            };
        }
        CardId::Later(_) => EN.family_after_the_transaction,
    };
    s.to_string()
}

fn body(app: &View, ui: &mut Ui, id: CardId, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    cy += match id {
        CardId::Page(page::MAP) => map(app, ui, x, cy, w),
        // Starting from the stick heads What do you have? (F103).
        CardId::Page(page::SAFE | page::HOLDING | page::OPEN) => holding(app, ui, x, cy, w),
        CardId::Page(page::CHECK) if app.family.state.from_card => {
            loaded_in_sparrow(app, ui, x, cy, w)
        }
        CardId::Page(page::CHECK) => check(app, ui, x, cy, w),
        CardId::Page(page::WRITE) if app.family.state.from_card => bring(app, ui, x, cy, w),
        CardId::Page(page::WRITE) => {
            if next_button(
                ui,
                x,
                cy,
                w,
                EN.family_have_psbt_button,
                fa(F::Next(page::WRITE)),
            ) {
                52.0
            } else {
                0.0
            }
        }
        CardId::Page(page::BRING) => bring(app, ui, x, cy, w),
        CardId::Page(_) => away(app, ui, x, cy, w),
        CardId::Step(step::SIGNERS) => signers(app, ui, x, cy, w),
        CardId::Step(n) => {
            let Some(s) = app.wallets.spend() else {
                return 0.0;
            };
            let mut used = step_body(app, ui, n, x, cy, w, s.wallet, app.spend_needed());
            if n == step::FINISH && s.spend.finished.is_some() {
                used += crate::screens::wrap_buttons(
                    ui,
                    x,
                    cy + used,
                    w,
                    38.0,
                    &[(EN.family_show_the_finished, Style::Secondary, fa(F::QrTx))],
                ) + 4.0;
            }
            used
        }
        CardId::Later(_) => 0.0,
    };
    let paras = text::more(app, id);
    if !paras.is_empty() {
        cy += more(app, ui, id, &paras, x, cy, w);
    }
    cy - y
}

/// More about this: a quiet button, and the paragraphs while it is open.
fn more(app: &View, ui: &mut Ui, id: CardId, paras: &[String], x: f32, y: f32, w: f32) -> f32 {
    let bit = more_bit(id);
    let shown = app.family.state.more & (1 << u32::from(bit)) != 0;
    let mut cy = y + 4.0;
    ui.button(
        x - 12.0,
        cy,
        None,
        BUTTON_H,
        if shown {
            EN.family_less_about
        } else {
            EN.family_more_about
        },
        Style::Ghost,
        fa(F::More(bit)),
    );
    cy += 40.0;
    if shown {
        for p in paras {
            cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, p) + 10.0;
        }
    }
    cy - y
}

fn map(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let stops = text::map(app.medium);
    for (k, (place, head, line)) in stops.iter().enumerate() {
        let c = if *place == EN.family_offline {
            Role::Ok
        } else if *place == EN.family_online {
            Role::Warning
        } else {
            Role::OnSurfaceVariant
        };
        // The lane: one node a stop, joined.
        ui.dot(x + 9.0, cy + 11.0, 6.0, c);
        let used = 30.0
            + ui.wrap_as(
                (x + 112.0, cy + 26.0, w - 112.0),
                BODY_S,
                Role::OnSurfaceVariant,
                line,
            );
        if k + 1 < stops.len() {
            ui.fill(
                x + 8.0,
                cy + 20.0,
                2.0,
                used - 12.0,
                1.0,
                Role::OutlineVariant,
            );
        }
        ui.text_as(x + 28.0, cy + 2.0, LABEL_M, c, place);
        ui.text_as(x + 112.0, cy + 2.0, LABEL_L, Role::OnSurface, head);
        cy += used + 10.0;
    }
    cy += 6.0;
    ui.icon(x, cy, 16.0, Icon::Warning, 10.0, Role::Warning);
    cy += ui.wrap_as(
        (x + 22.0, cy, w - 22.0),
        LABEL_L,
        Role::Warning,
        text::WORDS_ARE_MONEY,
    ) + 16.0;
    let drawn = next_button(ui, x, cy, w, EN.family_start, fa(F::Next(page::MAP)));
    // The count's line holds its band whether Continue stands beside
    // it or at the foot.
    cy + (if drawn { 52.0 } else { 48.0 }) - y
}

/// What do you have? (`docs/history/SCAN-TO-SIGN.md` §4): a vault when one is
/// found, the two ways in, what came in, the next question, and the
/// wallet once it is worked out.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn holding(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    // A vault, only when one is found.
    let files = app.vault_files();
    let locked: Vec<&crate::vault::VaultFile> = files.iter().filter(|f| f.open.is_none()).collect();
    if !locked.is_empty() || !app.vaults.open.is_empty() {
        if let Some(f) = files
            .get(app.vaults.pick)
            .filter(|f| f.open.is_none())
            .or(locked.first().copied())
        {
            let head = fill(EN.family_unlock_n, &[&f.name.to_string()]);
            cy += ui.wrap_as((x, cy, w), TITLE_M, Role::OnSurface, &head) + 12.0;
        }
        cy += vault(app, ui, x, cy, w);
        cy += 4.0;
        ui.text_as(
            x,
            cy,
            LABEL_L,
            Role::OnSurfaceVariant,
            EN.family_bring_in_paper,
        );
        cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GRID;
    }
    let ready = app.family_ready();
    let may = app.may_load_keys();
    let ways = vec![
        (
            EN.family_scan_the_qr,
            if ready {
                Style::Secondary
            } else {
                Style::Primary
            },
            fa(F::Scan),
        ),
        (
            EN.family_type_the_words,
            if may {
                Style::Secondary
            } else {
                Style::Disabled
            },
            fa(F::TypeWords),
        ),
    ];
    // Load sample PSBT goes in the "Demo only" card (`docs/DEMO.md` §14).
    if app.sample_psbt_offered() {
        ui.demo_offer(EN.scan_sample_psbt, Action::SamplePsbt);
    }
    cy += crate::screens::wrap_buttons(ui, x, cy, w, 40.0, &ways);
    // A description already in Files, or on a stick to copy in.
    if !ready {
        let files: Vec<String> = app
            .inbox
            .iter()
            .filter(|i| i.kind == FileKind::Wallet)
            .map(|i| fill(EN.family_open, &[&i.name.to_string()]))
            .collect();
        let items: Vec<(&str, Style, Action)> = app
            .inbox
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == FileKind::Wallet)
            .zip(&files)
            .map(|((k, _), l)| (l.as_str(), Style::Secondary, fa(F::UseWallet(k))))
            .collect();
        if !items.is_empty() {
            cy += crate::screens::wrap_buttons(ui, x, cy, w, 36.0, &items);
        }
        let label = fill(EN.family_copy_files_button, &[app.medium.a()]);
        cy += stick_row(app, ui, x, cy, w, &label);
    }
    ui.button(
        x - 12.0,
        cy,
        None,
        BUTTON_H,
        text::UNSURE,
        Style::Ghost,
        fa(F::Unsure),
    );
    cy += 40.0;
    if app.family.state.help {
        for (look, what) in text::looks(app.medium) {
            cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, &look) + 4.0;
            cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &what) + 14.0;
        }
    }
    // What came in, newest last.
    let rows = came_rows(app);
    if !rows.is_empty() {
        cy += 6.0;
        ui.text_as(
            x,
            cy,
            LABEL_L,
            Role::OnSurfaceVariant,
            EN.family_came_in_heading,
        );
        cy += ui.advance_as(LABEL_L, 26.0);
        for row in &rows {
            let row = ui.fit_as(BODY_M.mono(), row, w);
            ui.text_mid_as((x, cy, 34.0), BODY_M.mono(), Role::OnSurface, &row);
            ui.rule(x, cy + 34.0, w, Role::OutlineVariant);
            cy += 36.0;
        }
        cy += 12.0;
    }
    // Split sheets in Files that add up.
    if !ready {
        let sheets: Vec<String> = app
            .inbox
            .iter()
            .filter(|i| i.kind == FileKind::Share)
            .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
            .collect();
        if !sheets.is_empty() && crate::restore::merge(&sheets).is_ok_and(|m| m.whole.is_some()) {
            cy += crate::screens::wrap_buttons(
                ui,
                x,
                cy,
                w,
                38.0,
                &[(EN.family_rebuild_wallet, Style::Primary, fa(F::Rebuild))],
            );
        }
    }
    // A seed that is none of the described wallet's keys.
    if let Some(wi) = app.family_described() {
        let name = &app.session.wallets[wi].name;
        for fp in app.family_strays() {
            let line = fill(
                EN.family_seed_not_wallets,
                &[&fp_text(fp).to_string(), &name.to_string()],
            );
            cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Warning, &line) + 8.0;
            cy += crate::screens::wrap_buttons(
                ui,
                x,
                cy,
                w,
                38.0,
                &[(
                    EN.family_add_its_passphrase,
                    if may {
                        Style::Secondary
                    } else {
                        Style::Disabled
                    },
                    fa(F::StrayPass(fp.0)),
                )],
            ) + 4.0;
        }
    }
    cy += question(app, ui, x, cy, w);
    if let Some(e) = &app.family.state.error {
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, e) + 10.0;
    }
    cy += opened(app, ui, x, cy, w);
    if !rows.is_empty() || app.family.state.route.is_some() {
        cy += crate::screens::wrap_buttons(
            ui,
            x - 12.0,
            cy,
            w,
            36.0,
            &[(EN.family_start_over, Style::Ghost, fa(F::StartOver))],
        );
    }
    cy - y
}

/// The wallet, said once: its name and its shape, or the shape alone
/// when the name already says it ("Words · native SegWit").
fn wallet_once(w: &crate::wallet::Wallet) -> String {
    let shape = Session::shape(w);
    let kind = shape.rsplit(" · ").next().unwrap_or(&shape);
    if w.name.to_lowercase().contains(&kind.to_lowercase()) {
        shape
    } else {
        format!("{} · {shape}", w.name)
    }
}

/// What came in, a line each, newest last: the seeds still loaded and
/// the files still in Files.
fn came_rows(app: &View) -> Vec<String> {
    let mut sheets = 0;
    let mut texts: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    // Split sheets already in Files, copied in from a stick say, are
    // listed after what came in on the card.
    let mut came = app.family.state.came.clone();
    for it in app.inbox.iter().filter(|i| i.kind == FileKind::Share) {
        let c = Came::File(it.name.clone());
        if !came.contains(&c) {
            came.push(c);
        }
    }
    for c in &came {
        match c {
            Came::Seed(fp) => {
                let f = faraday_bip::keys::Fingerprint(*fp);
                let Some(label) = app.session.keys.key_label(f) else {
                    continue;
                };
                let t = fp_text(f);
                let label = label.trim();
                rows.push(if label.is_empty() || label.eq_ignore_ascii_case(&t) {
                    fill(EN.family_seed_row, &[&t.to_string()])
                } else {
                    fill(EN.family_seed, &[&t.to_string(), label])
                });
            }
            Came::File(name) => {
                let Some(item) = app.inbox.iter().find(|i| i.name == *name) else {
                    continue;
                };
                rows.push(match item.kind {
                    FileKind::Wallet => {
                        match app.session.wallets.iter().find(|wl| wl.source == *name) {
                            Some(wl) => fill(
                                EN.family_description_row_shape,
                                &[&wl.name.to_string(), &Session::shape(wl).to_string()],
                            ),
                            None => fill(EN.family_description_row, &[&name.to_string()]),
                        }
                    }
                    FileKind::Share => {
                        sheets += 1;
                        texts.push(String::from_utf8_lossy(&item.bytes).into_owned());
                        match crate::restore::merge(&texts) {
                            Ok(m) => fill(
                                EN.family_part_of_keys,
                                &[
                                    &sheets.to_string(),
                                    &m.have.len().to_string(),
                                    &m.n.to_string(),
                                ],
                            ),
                            Err(_) => fill(EN.family_part, &[&sheets.to_string()]),
                        }
                    }
                    FileKind::Key => {
                        let t = String::from_utf8_lossy(&item.bytes);
                        let key = crate::create::read_key(&t).unwrap_or_default();
                        let fp = key
                            .strip_prefix('[')
                            .and_then(|r| r.split(['/', ']']).next())
                            .map(str::to_uppercase);
                        match fp {
                            Some(fp) => fill(EN.family_key_xpub_n, &[&fp.to_string()]),
                            None => EN.family_key_xpub.to_string(),
                        }
                    }
                    FileKind::Psbt => EN.family_transaction.to_string(),
                    _ => name.clone(),
                });
            }
        }
    }
    rows
}

/// The next question, by what came in (`docs/history/SCAN-TO-SIGN.md` §4.3).
fn question(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let may = app.may_load_keys();
    let typing = if may {
        Style::Secondary
    } else {
        Style::Disabled
    };
    match app.family_question() {
        Question::None => return 0.0,
        Question::Anything => {
            cy += ui.wrap_as(
                (x, cy, w),
                TITLE_M,
                Role::OnSurface,
                EN.family_anything_else,
            ) + 6.0;
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::OnSurfaceVariant,
                text::ANYTHING_HINT,
            ) + 12.0;
            cy += crate::screens::wrap_buttons(
                ui,
                x,
                cy,
                w,
                40.0,
                &[
                    (EN.family_scan_more, Style::Secondary, fa(F::Scan)),
                    (EN.family_type_more_words, typing, fa(F::TypeWords)),
                    (
                        EN.family_that_s_everything,
                        Style::Primary,
                        fa(F::Everything),
                    ),
                ],
            );
        }
        Question::Description => {
            cy += ui.wrap_as((x, cy, w), TITLE_M, Role::OnSurface, EN.family_these_seeds) + 6.0;
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::OnSurfaceVariant,
                text::DESCRIPTION_HINT,
            ) + 12.0;
            cy += crate::screens::wrap_buttons(
                ui,
                x,
                cy,
                w,
                40.0,
                &[
                    (EN.family_scan_the_description, Style::Primary, fa(F::Scan)),
                    (
                        EN.family_i_can_t_find,
                        Style::Secondary,
                        fa(F::NoDescription),
                    ),
                ],
            );
        }
        Question::Shape => {
            let s = &app.family.state.seeds;
            let here = app.seeds_here(s).len();
            if s.n > here {
                let line = fill(EN.family_without_description, &[&s.n.to_string()]);
                cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Warning, &line) + 12.0;
            }
            cy += crate::seeds::screen::shape(app, ui, s, false, x, cy, w);
        }
    }
    cy + 8.0 - y
}

/// The vault route: the vaults, the stick, the passphrase, Unlock.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn vault(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let files = app.vault_files();
    // Open already: its seeds and wallets, all ticked at first, load
    // from the same list as on Files.
    for v in 0..app.vaults.open.len() {
        cy += crate::screens::vault_panel(app, ui, v, x, cy, w) + 14.0;
    }
    let locked: Vec<(usize, &crate::vault::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() {
        return cy - y;
    }
    if !app.may_load_keys() {
        cy += crate::vault::screen::stick_banner(
            ui,
            x,
            cy,
            w,
            &fill(EN.family_remove_the_then, &[app.medium.noun()]),
        );
    }
    let compact = ui.compact;
    for (i, f) in &locked {
        let on = *i == app.vaults.pick;
        let ink = on_selected(on, Role::OnSurface);
        let muted = on_selected(on, Role::OnSurfaceVariant);
        // On a small panel the unlock time goes on a third line.
        let ch = if compact { 84.0 } else { 64.0 };
        ui.fill(
            x,
            cy,
            w,
            ch,
            10.0,
            selected_or(on, Role::SurfaceContainerHighest),
        );
        ui.stroke(x, cy, w, ch, 10.0, selected_or(on, Role::OutlineVariant));
        ui.icon(x + 14.0, cy + 14.0, 36.0, Icon::Lock, 15.0, muted);
        let where_ = if f.in_outbox {
            fill(EN.family_sealed_here, &[app.medium.for_the()])
        } else {
            fill(EN.family_copied_from_medium, &[app.medium.a()])
        };
        let where_ = format!("{where_} · {}", file_text(f.len));
        let mem = f.header.cost.memory_kib / 1024;
        let time = fill(
            EN.family_unlock_time,
            &[&app
                .vaults()
                .time_text(mem, f.header.cost.passes)
                .to_string()],
        );
        if compact {
            let room = w - 62.0 - 12.0;
            let name = ui.fit_as(BODY_M.mono(), &f.name, room);
            ui.text_as(x + 62.0, cy + 10.0, BODY_M.mono(), ink, &name);
            let where_ = ui.fit_as(BODY_S, &where_, room);
            ui.text_as(x + 62.0, cy + 34.0, BODY_S, muted, &where_);
            let time = ui.fit_as(LABEL_M, &time, room);
            ui.text_as(x + 62.0, cy + 56.0, LABEL_M, ink, &time);
        } else {
            ui.text_as(x + 62.0, cy + 12.0, BODY_M.mono(), ink, &f.name);
            ui.text_as(x + 62.0, cy + 36.0, BODY_S, muted, &where_);
            ui.text_right_as((x + w - 14.0, cy + 8.0, 24.0), LABEL_M, ink, &time);
            ui.text_right_as(
                (x + w - 14.0, cy + 32.0, 24.0),
                BODY_S,
                muted,
                &cost_text(&f.header.cost),
            );
        }
        ui.hit(x, cy, w, ch, fa(F::Pick(*i)));
        cy += ch + 8.0;
    }
    cy += 8.0;
    ui.text_as(x, cy, LABEL_L, Role::OnSurfaceVariant, EN.family_passphrase);
    cy += ui.advance_as(LABEL_L, 22.0);
    if app.may_load_keys() {
        let focused = app.vaults.focus == Some(crate::vault::Focus::Passphrase);
        crate::demo::pass_box(
            app,
            ui,
            (x, cy, w),
            &app.vaults.passphrase,
            (app.vaults.typed_shown, focused),
            (
                Action::Vault(VaultAction::FocusPassphrase),
                Action::Vault(VaultAction::ShowTyped),
            ),
            crate::demo::PassField::Unlock,
        );
    } else {
        crate::vault::screen::stick_field(app.medium, ui, x, cy, w);
    }
    cy += FIELD_H + 12.0 + crate::screens::never_under(app, ui, x, cy, w);
    if app.vaults.working == Some(crate::vault::Work::Unlock) {
        ui.text_mid_as((x, cy, 42.0), LABEL_L, Role::Primary, EN.family_unlocking);
        crate::screens::working_bar(ui, x, cy, w, 42.0);
    } else {
        let style = Style::Primary.when(app.may_load_keys());
        let free = match app.vaults.memory_free_mib {
            Some(m) => fill(EN.family_this_computer, &[&mib_text(m).to_string()]),
            None => String::new(),
        };
        if compact {
            // Unlock pinned at the foot; the memory line here.
            if crate::screens::pin_button(
                ui,
                x,
                cy,
                Some(w),
                42.0,
                EN.family_unlock,
                style,
                fa(F::Unlock),
            ) {
                cy += 50.0;
            }
            if !free.is_empty() {
                cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &free) + 8.0;
            }
            cy -= 54.0;
        } else {
            let bw = ui.button(
                x,
                cy,
                Some(140.0),
                BUTTON_H,
                EN.family_unlock,
                style,
                fa(F::Unlock),
            );
            ui.text_mid_as(
                (x + bw + 16.0, cy, 42.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &free,
            );
        }
    }
    cy += 54.0;
    if let Some(e) = &app.vaults.unlock_error {
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, e) + 10.0;
    }
    cy - y
}

/// A stick visit from this tab, or what stops one.
fn stick_row(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, label: &str) -> f32 {
    let open = app.holds_secret() || !app.vaults.open.is_empty();
    let (style, note) = match (app.stick.is_empty(), open) {
        (true, _) => (
            Style::Disabled,
            fill(EN.family_plug_medium, &[app.medium.noun()]),
        ),
        (false, true) => (
            Style::Disabled,
            fill(EN.family_faraday_asks, &[app.medium.a()]),
        ),
        (false, false) => (Style::Secondary, String::new()),
    };
    let bw = ui.button(
        x,
        y,
        None,
        BUTTON_H,
        label,
        style,
        Action::VisitFrom(crate::Screen::Family),
    );
    if ui.compact && !note.is_empty() {
        // Under the button, wrapped.
        return 46.0 + ui.wrap_as((x, y + 46.0, w), BODY_S, Role::OnSurfaceVariant, &note) + 12.0;
    }
    if !note.is_empty() {
        ui.text_mid_as(
            (x + bw + 14.0, y, 38.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &note,
        );
    }
    50.0
}

/// What opened: the wallets to spend from, each with its keys here. A
/// wallet opening moves on to Check the money, which shows its address;
/// pressing the one chosen moves on too (F103).
fn opened(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let chosen = app.family_wallet();
    if app.session.wallets.is_empty() {
        return 0.0;
    }
    ui.text_as(
        x,
        cy,
        LABEL_L,
        Role::OnSurfaceVariant,
        EN.family_spend_from_label,
    );
    cy += ui.advance_as(LABEL_L, 26.0);
    for (i, wl) in app.session.wallets.iter().enumerate() {
        let on = chosen == Some(i);
        ui.checkbox(x, cy + 11.0, on, true);
        let name = ui.fit_as(LABEL_L, &wl.name, w - 30.0);
        ui.text_mid_as((x + 30.0, cy, 40.0), LABEL_L, Role::OnSurface, &name);
        // How many of its keys are here: a vault may hold only some.
        let slots = app.session.slots(wl);
        let here = slots.iter().filter(|s| s.held_by.is_some()).count();
        let line = fill(
            EN.family_of_keys_here,
            &[
                &Session::shape(wl).to_string(),
                &here.to_string(),
                &slots.len().to_string(),
            ],
        );
        let tone = Role::OnSurfaceVariant;
        // Under the name on a small panel.
        let rh = if ui.compact {
            let line = ui.fit_as(BODY_S, &line, w - 30.0);
            ui.text_mid_as((x + 30.0, cy + 30.0, 20.0), BODY_S, tone, &line);
            56.0
        } else {
            ui.text_right_as((x + w, cy, 40.0), BODY_S, tone, &line);
            40.0
        };
        ui.hit(x, cy, w, rh, fa(F::Choose(i)));
        cy += rh + 2.0;
    }
    cy + 8.0 - y
}

/// The shortcut over a card's first page: the PSBT made in Sparrow
/// already, scanned at once, which goes to its Check.
fn shortcut(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if app.wallets.spend().is_some() {
        return 0.0;
    }
    // Load sample PSBT goes in the "Demo only" card (`docs/DEMO.md` §14).
    if app.sample_psbt_offered() {
        ui.demo_offer(EN.scan_sample_psbt, Action::SamplePsbt);
    }
    let with_sample = |scan: &'static str| vec![(scan, Style::Secondary, Action::Scan)];
    if ui.compact {
        // Too long for one button on a small panel: the line, then Scan it.
        let lh = ui.wrap_as(
            (x, y, w),
            LABEL_L,
            Role::OnSurface,
            EN.family_have_psbt_line,
        ) + 8.0;
        return lh
            + crate::screens::wrap_buttons(
                ui,
                x,
                y + lh,
                w,
                40.0,
                &with_sample(EN.family_scan_button),
            )
            + 8.0;
    }
    crate::screens::wrap_buttons(ui, x, y, w, 40.0, &with_sample(SHORTCUT)) + 8.0
}

/// The shortcut's label.
pub const SHORTCUT: &str = EN.family_i_have_the_psbt;

/// Load the wallet in Sparrow, from the wallet's card: the shortcut, the
/// wallet QR, its first address, It is loaded.
fn loaded_in_sparrow(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let Some(i) = app.family_wallet() else {
        return 0.0;
    };
    cy += shortcut(app, ui, x, cy, w);
    let wl = &app.session.wallets[i];
    cy += crate::screens::wrap_buttons(
        ui,
        x,
        cy,
        w,
        40.0,
        &[(
            EN.family_show_the_wallet_qr,
            Style::Primary,
            Action::QrWallet(i),
        )],
    ) + 6.0;
    ui.text_as(
        x,
        cy,
        LABEL_L,
        Role::OnSurfaceVariant,
        EN.family_receive_address,
    );
    cy += ui.advance_as(LABEL_L, 22.0);
    let (addr, held) = app.receive_line(wl, false, 0);
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_M.mono(),
        if held { Role::Warning } else { Role::OnSurface },
        &addr,
    ) + 14.0;
    let drawn = next_button(
        ui,
        x,
        cy,
        w,
        EN.family_it_is_loaded,
        fa(F::Next(page::CHECK)),
    );
    // The count's line holds its band whether Continue stands beside
    // it or at the foot.
    cy + (if drawn { 52.0 } else { 48.0 }) - y
}

/// Check the money: the wallet as a QR code, and its addresses. For a
/// wallet worked out from seeds alone, the ways tried, the way tried
/// now, and No money shows for the next (`docs/history/SCAN-TO-SIGN.md` §5).
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn check(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let Some(i) = app.family_wallet() else {
        return 0.0;
    };
    let guess = app.family_guess();
    if let Some(g) = guess {
        // The ways tried, struck.
        for t in &g.tried {
            let lh = ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, t);
            let tw = ui.measure_as(BODY_S, t).min(w);
            ui.rule(x, cy + ui.line_as(BODY_S) / 2.0, tw, Role::OnSurfaceVariant);
            cy += lh + 6.0;
        }
        if !g.tried.is_empty() {
            cy += 8.0;
        }
        if g.none {
            cy += ui.wrap_as(
                (x, cy, w),
                TITLE_M,
                Role::OnSurface,
                EN.family_none_of_these,
            ) + 6.0;
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::OnSurfaceVariant,
                EN.family_the_words_may,
            ) + 12.0;
            cy += crate::screens::wrap_buttons(
                ui,
                x,
                cy,
                w,
                40.0,
                &[
                    (EN.family_scan_more, Style::Primary, fa(F::Again(true))),
                    (
                        EN.family_type_a_passphrase,
                        Style::Secondary,
                        fa(F::Again(false)),
                    ),
                ],
            );
            return cy - y;
        }
        if g.asking {
            cy += ui.wrap_as(
                (x, cy, w),
                TITLE_M,
                Role::OnSurface,
                EN.family_is_there_a_word,
            ) + 12.0;
            cy += crate::screens::wrap_buttons(
                ui,
                x,
                cy,
                w,
                40.0,
                &[
                    (
                        EN.family_yes_type_button,
                        Style::Primary,
                        fa(F::Passphrase(true)),
                    ),
                    (
                        EN.family_no_button,
                        Style::Secondary,
                        fa(F::Passphrase(false)),
                    ),
                ],
            );
            return cy - y;
        }
        let (kind, pass) = Faraday::guess_way(g, g.at);
        let total = Faraday::guess_total(g);
        let line = fill(
            EN.family_trying_progress,
            &[
                crate::family::kind_label(kind),
                (if pass {
                    EN.family_with_a_passphrase
                } else {
                    EN.family_no_passphrase
                }),
                &(g.at + 1).to_string(),
                &total.to_string(),
            ],
        );
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, &line) + GAP;
        // The ways looked at so far, the one shown among them.
        let done = (g.at + 1) as f32 / total.max(1) as f32;
        ui.progress((x, cy, w), Some(done), None);
        cy += PROGRESS_H + GAP + GAP_SMALL;
    }
    let wl = &app.session.wallets[i];
    cy += crate::screens::wrap_buttons(
        ui,
        x,
        cy,
        w,
        40.0,
        &[(
            EN.family_show_the_wallet,
            Style::Primary,
            Action::QrWallet(i),
        )],
    ) + 6.0;
    if let Some(g) = guess {
        let open = if g.tried.is_empty() {
            EN.family_open_online_wallet
        } else {
            EN.family_open_this_one
        };
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, open) + 14.0;
    }
    let n = 3 + app.family.state.addresses;
    for (head, change) in [(EN.family_receive, false), (EN.family_change, true)] {
        ui.text_as(x, cy, LABEL_L, Role::OnSurfaceVariant, head);
        cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GAP_SMALL;
        for k in 0..n {
            ui.text_mid_as(
                (x, cy, 28.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &k.to_string(),
            );
            let (a, held) = app.receive_line(wl, change, k);
            let col = if held { Role::Warning } else { Role::OnSurface };
            if ui.compact {
                // Whole, to compare character for character.
                cy += ui.wrap_as((x + 24.0, cy + 6.0, w - 24.0), BODY_S.mono(), col, &a) + 14.0;
                continue;
            }
            let a = ui.fit_as(BODY_M.mono(), &a, w - 40.0);
            ui.text_mid_as((x + 40.0, cy, 28.0), BODY_M.mono(), col, &a);
            cy += 30.0;
        }
        cy += 8.0;
    }
    let mut left: Vec<(&str, Style, Action)> = Vec::new();
    if n < 50 {
        left.push((
            EN.family_show_more_addresses,
            Style::Secondary,
            fa(F::MoreAddresses),
        ));
    }
    if guess.is_some() {
        left.push((EN.family_no_money_shows, Style::Secondary, fa(F::NoMoney)));
    }
    cy += crate::screens::buttons_and_next(
        ui,
        x,
        cy,
        w,
        &left,
        Some((EN.family_yes_my_money, fa(F::Next(page::CHECK)))),
    );
    cy + 4.0 - y
}

/// Bring the transaction: scan it, copy it in, or one already in Files.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn bring(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    // From the wallet's card this is Write the payment's own: Scan the
    // PSBT is the shortcut here.
    let card = app.family.state.from_card;
    // The money moving to a new wallet (`docs/history/NEW-WALLET.md` §9.6): every
    // coin, to its first address.
    if card && let Some((name, address)) = app.family_wallet().and_then(|i| app.moving_to(i)) {
        let head = fill(EN.family_every_coin_to_wallet, &[&name.to_string()]);
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Warning, &head) + 4.0;
        cy += ui.wrap_as((x, cy, w), BODY_M.mono(), Role::OnSurface, &address) + 4.0;
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.family_amount_max_hint,
        ) + 12.0;
    }
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        if card {
            EN.family_scan_the_psbt
        } else {
            EN.family_scan
        },
        if app.wallets.spend().is_some() {
            Style::Secondary
        } else {
            Style::Primary
        },
        Action::Scan,
    );
    cy += 52.0;
    // Load sample PSBT goes in the "Demo only" card (`docs/DEMO.md` §14).
    if app.sample_psbt_offered() {
        ui.demo_offer(EN.scan_sample_psbt, Action::SamplePsbt);
    }
    let label = if card {
        fill(EN.family_on_medium, &[app.medium.a()])
    } else {
        fill(EN.family_copy_psbt_button, &[app.medium.a()])
    };
    cy += stick_row(app, ui, x, cy, w, &label);
    // From the wallet's card, only this wallet's.
    let wallet = app.family_wallet();
    let psbts: Vec<(usize, &str)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == FileKind::Psbt)
        .filter(|(_, i)| {
            !card
                || i.psbt()
                    .and_then(|p| app.spends_from(p))
                    .is_some_and(|w| Some(w) == wallet)
        })
        .map(|(k, i)| (k, i.name.as_str()))
        .collect();
    if !psbts.is_empty() {
        ui.text_as(x, cy, LABEL_L, Role::OnSurfaceVariant, EN.family_in_files);
        cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GAP_SMALL;
    }
    for (k, name) in &psbts {
        let current = app.wallets.spend().is_some_and(|s| s.spend.source == *name);
        // On a small panel the name has its own line.
        if ui.compact {
            let n = ui.fit_as(BODY_M.mono(), name, w);
            ui.text_as(x, cy, BODY_M.mono(), Role::OnSurface, &n);
            cy += ui.advance_as(BODY_M.mono(), 22.0);
        } else {
            ui.text_mid_as((x, cy, 38.0), BODY_M.mono(), Role::OnSurface, name);
        }
        if current {
            ui.text_right_as(
                (x + w, cy, 38.0),
                LABEL_L,
                Role::Ok,
                EN.family_signing_this_one,
            );
        } else {
            let label = EN.family_use_this_one;
            let bw = ui.measure_as(LABEL_L, label) + 32.0;
            ui.button(
                x + w - bw,
                cy + 1.0,
                Some(bw),
                BUTTON_H,
                label,
                Style::Secondary,
                fa(F::UsePsbt(*k)),
            );
        }
        cy += 44.0;
    }
    if let Some(s) = app.wallets.spend() {
        cy += 6.0;
        match s.wallet.and_then(|w| app.session.wallets.get(w)) {
            Some(wl) => {
                let line = fill(
                    EN.family_spends_from_wallet,
                    &[&wl.name.to_string(), &Session::shape(wl).to_string()],
                );
                cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Ok, &line) + 12.0;
            }
            None => {
                cy +=
                    ui.wrap_as((x, cy, w), BODY_S, Role::Warning, EN.family_no_wallet_open) + 12.0;
            }
        }
        if next_button(
            ui,
            x,
            cy,
            w,
            EN.family_read_button,
            Action::Step(first_step(app)),
        ) {
            cy += 52.0;
        }
    }
    cy - y
}

fn first_step(app: &View) -> u8 {
    app.wallets
        .spend()
        .and_then(|s| s.steps.iter().copied().find(|&n| !s.done[n as usize]))
        .unwrap_or(step::TRANSACTION)
}

/// Who signs: each of the wallet's keys, here or not, how many sign
/// here, and while that is short of what the spend needs, Scan a key,
/// the vaults to unlock and Collect a signature (`docs/history/SCAN-TO-SIGN.md`
/// §3).
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn signers(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let Some(s) = app.wallets.spend() else {
        return 0.0;
    };
    let needed = app.spend_needed();
    let may = app.may_load_keys();
    let wallet = s
        .wallet
        .and_then(|i| app.session.wallets.get(i).map(|wl| (i, wl)));
    match wallet {
        Some((wi, wl)) => {
            // On a small panel a key's name has its line, and what it
            // does goes under it.
            let compact = ui.compact;
            let passphrase_key = app.session.slots(wl).iter().any(|s| s.held_by.is_none())
                && !app.vault_passphrase_seeds().is_empty();
            for (k, slot) in app.session.slots(wl).iter().enumerate() {
                ui.text_mid_as(
                    (x, cy, 42.0),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    &(k + 1).to_string(),
                );
                let fp = slot.fingerprint.map(fp_text).unwrap_or_default();
                ui.text_mid_as((x + 22.0, cy, 42.0), BODY_M.mono(), Role::OnSurface, &fp);
                // A label that is its fingerprint is not drawn twice.
                let label = slot
                    .held_by
                    .as_deref()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && !l.eq_ignore_ascii_case(&fp));
                if compact {
                    if let Some(label) = label {
                        let label = ui.fit_as(BODY_S, label, w - 124.0);
                        ui.text_mid_as(
                            (x + 124.0, cy, 42.0),
                            BODY_S,
                            Role::OnSurfaceVariant,
                            &label,
                        );
                    }
                    cy += 36.0;
                }
                // Made here, its backup not done: it waits
                // (`docs/history/NEW-WALLET.md` §14.3).
                let waits =
                    slot.held_by.is_some() && slot.fingerprint.is_some_and(|f| app.key_held(f));
                let in_vault = slot
                    .fingerprint
                    .filter(|_| slot.held_by.is_none() && app.vault_key_for(slot).is_some());
                if waits {
                    let t = crate::glance::WAITING;
                    let label = EN.family_finish_backup_button;
                    let bw = ui.measure_as(LABEL_L, label) + 32.0;
                    let cw2 = ui.measure_as(BODY_S, t) + 34.0;
                    if compact {
                        ui.tag(x + 22.0, cy + 8.0, t, Status::Warning);
                    } else {
                        ui.tag(x + w - bw - 8.0 - cw2, cy + 8.0, t, Status::Warning);
                    }
                    ui.button(
                        x + w - bw,
                        cy + 3.0,
                        Some(bw),
                        BUTTON_H,
                        label,
                        Style::Secondary,
                        Action::BackupFirst(wi),
                    );
                } else if in_vault.is_none() && slot.held_by.is_none() && passphrase_key {
                    // A seed an open vault keeps without the passphrase
                    // this wallet may use it with (F62).
                    let label = EN.vault_load_with_a_passphrase;
                    let bw = ui.measure_as(LABEL_L, label) + 32.0;
                    ui.button(
                        x + w - bw,
                        cy + 3.0,
                        Some(bw),
                        BUTTON_H,
                        label,
                        if may {
                            Style::Secondary
                        } else {
                            Style::Disabled
                        },
                        Action::Vault(VaultAction::PassphraseKey(wi)),
                    );
                } else if let Some(f) = in_vault {
                    // A key an open vault holds loads from it.
                    let label = EN.family_load_from_vault;
                    let bw = ui.measure_as(LABEL_L, label) + 32.0;
                    ui.button(
                        x + w - bw,
                        cy + 3.0,
                        Some(bw),
                        BUTTON_H,
                        label,
                        if may {
                            Style::Secondary
                        } else {
                            Style::Disabled
                        },
                        Action::Vault(VaultAction::LoadKeyOf(
                            f.0,
                            crate::vault::SlotOf::Loaded(wi, k),
                        )),
                    );
                } else {
                    let (t, status) = if slot.held_by.is_some() {
                        (EN.family_signs_here, Status::Ok)
                    } else {
                        (EN.family_not_here, Status::Neutral)
                    };
                    let cw2 = ui.measure_as(BODY_S, t) + 34.0;
                    let chip_x = if compact { x + 22.0 } else { x + w - cw2 };
                    if !compact && let Some(label) = label {
                        let label = ui.fit_as(BODY_S, label, w - 124.0 - cw2 - 8.0);
                        ui.text_mid_as(
                            (x + 124.0, cy, 42.0),
                            BODY_S,
                            Role::OnSurfaceVariant,
                            &label,
                        );
                    }
                    ui.tag(chip_x, cy + 8.0, t, status);
                }
                ui.rule(x, cy + 42.0, w, Role::OutlineVariant);
                cy += 44.0;
            }
        }
        None => {
            for fp in &s.inspection.participating_keys {
                ui.text_mid_as((x, cy, 30.0), BODY_M.mono(), Role::OnSurface, &fp_text(*fp));
                cy += 30.0;
            }
        }
    }
    cy += 10.0;
    let here = s
        .inspection
        .participating_keys
        .iter()
        .filter(|fp| app.session.keys.key_label(**fp).is_some())
        .count();
    // Fewer keys here than sign: bring one here, or collect a signature
    // from whoever holds one (`docs/history/NEW-WALLET.md` §11.2). A wallet whose
    // paths differ keeps them until the spend is signed, since a count
    // of keys does not settle which path signs.
    let paths = wallet
        .is_some_and(|(_, wl)| matches!(Kind::of(wl.policy()), Kind::Miniscript | Kind::Tree));
    let short = here < needed;
    if short || (paths && !s.complete) {
        let style = if may {
            Style::Secondary
        } else {
            Style::Disabled
        };
        let unlocks: Vec<(String, Action)> = app
            .vault_files()
            .iter()
            .enumerate()
            .filter(|(_, f)| f.open.is_none())
            .map(|(k, f)| {
                (
                    fill(EN.family_unlock_n, &[&f.name.to_string()]),
                    Action::Vault(VaultAction::OpenFrom(k, crate::Screen::Family)),
                )
            })
            .collect();
        let mut items: Vec<(&str, Style, Action)> = Vec::new();
        if let Some((wi, _)) = wallet {
            let scan = Style::Primary.when(may);
            items.push((EN.family_scan_a_key, scan, Action::ScanKey(wi)));
        }
        items.extend(unlocks.iter().map(|(l, a)| (l.as_str(), style, *a)));
        if s.steps.contains(&step::COLLECT) {
            items.push((
                EN.family_collect_a_signature,
                Style::Secondary,
                Action::Step(step::COLLECT),
            ));
        }
        cy += crate::screens::wrap_buttons(ui, x, cy, w, 38.0, &items) + 2.0;
        if !may {
            let pull = fill(EN.family_remove_the_first, &[app.medium.noun()]);
            ui.text_as(x, cy, LABEL_L, Role::Warning, &pull);
            cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GRID;
        }
        cy += 6.0;
    }
    let cy = crate::screens::signers_count(ui, x, cy, w, here, needed);
    let drawn = next_button(
        ui,
        x,
        cy,
        w,
        EN.family_continue,
        Action::StepNext(step::SIGNERS),
    );
    // The count's line holds its band whether Continue stands beside
    // it or at the foot.
    cy + (if drawn { 52.0 } else { 48.0 }) - y
}

/// Put everything away: lock, write what waits, power off.
fn away(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    for it in &app.outbox {
        ui.icon(x, cy + 6.0, 20.0, Icon::File, 11.0, Role::OnSurfaceVariant);
        let name = ui.fit_as(BODY_M.mono(), &it.name, w - 28.0);
        ui.text_mid_as((x + 28.0, cy, 32.0), BODY_M.mono(), Role::OnSurface, &name);
        cy += 34.0;
    }
    if !app.outbox.is_empty() {
        cy += 8.0;
    }
    if ui.compact {
        cy += crate::screens::wrap_buttons(
            ui,
            x,
            cy,
            w,
            40.0,
            &[
                (EN.family_lock, Style::Primary, Action::LockAsk),
                (EN.family_power_off, Style::Secondary, Action::PowerAsk),
                (EN.family_start_over, Style::Ghost, fa(F::StartOver)),
            ],
        );
        return cy + 4.0 - y;
    }
    let bw = ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.family_lock,
        Style::Primary,
        Action::LockAsk,
    );
    ui.button(
        x + bw + 8.0,
        cy,
        None,
        BUTTON_H,
        EN.family_power_off,
        Style::Secondary,
        Action::PowerAsk,
    );
    let label = EN.family_start_over;
    let sw = ui.measure_as(LABEL_L, label) + 32.0;
    ui.button(
        x + w - sw,
        cy + 2.0,
        Some(sw),
        BUTTON_H,
        label,
        Style::Ghost,
        fa(F::StartOver),
    );
    cy + 52.0 - y
}
