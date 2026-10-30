//! `Screen::Wallets`: the Wallets tab, the wallets, the wallet card and every flow (`docs/WALLETS.md`).

use super::parts::FROM_KEY;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::{Session, fp_text, key_line};
use crate::{Action, Screen};
use faraday_draw::tokens::{self};
use faraday_draw::tokens::{BODY_L, BODY_M, BODY_S, BUTTON_H, LABEL_L, LABEL_M, TITLE_L};
use faraday_draw::widgets::Icon;
use faraday_ui::Status;
use faraday_ui::parts::{section_label, wrap_buttons};

/// The line beside key `fp`'s fingerprint where the codex32 strings it
/// was loaded from went in by a correction (`docs/CODEX32.md` §8.4):
/// "9a6a2580 · Share entry corrected", or "… · Share repairs unchecked"
/// in the caution tone. Nothing for any other key. Returns its height.
pub(crate) fn mark_line(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    fp: faraday_bip::keys::Fingerprint,
) -> f32 {
    use crate::codex32_correct::Mark;
    let Some(mark) = app.session.key_marks.of(fp.0) else {
        return 0.0;
    };
    let (icon, tone) = match mark {
        Mark::Corrected => (Icon::Info, Role::OnSurfaceVariant.paint()),
        Mark::Repaired => (Icon::Warning, Role::Warning.paint()),
    };
    let text = fill(EN.c32r_key_mark, &[&fp_text(fp), mark.fingerprint_line()]);
    crate::codex32_correct::screen::line(ui, x, y, w, icon, tone, &text)
        + faraday_draw::tokens::CORRECT_GAP
}

/// The line under each key held by wallet `slots`' keys that carries a
/// codex32 mark (§8.4). Returns the height.
pub(crate) fn slot_marks(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    slots: &[crate::wallet::Slot],
) -> f32 {
    let mut h = 0.0;
    for fp in slots
        .iter()
        .filter(|s| s.held_by.is_some())
        .filter_map(|s| s.fingerprint)
    {
        h += mark_line(app, ui, x, y + h, w, fp);
    }
    h
}

/// The wallet card's backup line (`docs/history/SIMPLIFY.md` §5.2) for loaded
/// wallet `i`, which opens Backups on it. Returns the height it took.
pub(crate) fn backup_line(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, i: usize) -> f32 {
    let line = app.backup_line(i);
    let action = Action::BackupsOf(i);
    let tone = if line.starts_with(
        EN.backups_backup
            .split("{}")
            .next()
            .unwrap_or_default()
            .trim_end(),
    ) {
        Role::OnSurfaceVariant
    } else {
        Role::Warning
    };
    let h = ui.wrap_as((x, y, w), BODY_M, tone, &line);
    ui.press_box(x, y - 6.0, w, h + 12.0, action);
    h + 16.0
}

/// Where a wallet's missing keys may be, and the way to look: a locked
/// vault to unlock (coming back to `back`), the open vaults that do not
/// hold them, and the other places a seed is kept. Nothing when an open
/// vault holds one, since its row loads it. Returns its height.
#[allow(clippy::too_many_arguments)]
pub(crate) fn missing_keys_line(
    app: &View,
    ui: &mut Ui,
    slots: &[crate::wallet::Slot],
    back: Screen,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    if slots
        .iter()
        .filter(|s| s.held_by.is_none() && s.fingerprint.is_some())
        .any(|s| app.vault_key_for(s).is_some())
    {
        return 0.0;
    }
    let files = app.vault_files();
    let locked = files.iter().position(|f| f.open.is_none());
    let open_names: Vec<String> = app.vaults.open.iter().map(|v| v.name.clone()).collect();
    let line = match (locked, open_names.is_empty()) {
        (Some(i), _) => fill(EN.missing_keys_line_locked, &[&files[i].name.to_string()]),
        (None, false) => fill(
            EN.missing_keys_line_not_open,
            &[&open_names.join(", ").to_string()],
        ),
        (None, true) => fill(EN.missing_keys_line_elsewhere, &[app.medium.a()]),
    };
    if ui.compact {
        ui.icon(
            x - 4.0,
            y - 2.0,
            20.0,
            Icon::Info,
            11.0,
            Role::OnSurfaceVariant,
        );
        let mut cy =
            y + ui.wrap_as(
                (x + 20.0, y, w - 20.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &line,
            ) + 8.0;
        if let Some(i) = locked {
            ui.button(
                x + 20.0,
                cy,
                Some(w - 20.0),
                BUTTON_H,
                EN.missing_keys_line_unlock,
                if app.may_load_keys() {
                    Style::Secondary
                } else {
                    Style::Disabled
                },
                Action::Vault(crate::vault::VaultAction::OpenFrom(i, back)),
            );
            cy += 44.0;
        }
        return cy - y;
    }
    ui.icon(x, y + 10.0, 20.0, Icon::Info, 11.0, Role::OnSurfaceVariant);
    let line = ui.fit_as(BODY_S, &line, w - 140.0);
    let lw = ui.text_mid_as((x + 26.0, y, 40.0), BODY_S, Role::OnSurfaceVariant, &line);
    if let Some(i) = locked {
        ui.button(
            x + 40.0 + lw,
            y + 4.0,
            None,
            BUTTON_H,
            EN.missing_keys_line_unlock,
            if app.may_load_keys() {
                Style::Secondary
            } else {
                Style::Disabled
            },
            Action::Vault(crate::vault::VaultAction::OpenFrom(i, back)),
        );
    }
    44.0
}

/// The Wallets card with keys loaded and no wallet: each key, the one
/// picked when there are several, and the ways on to a wallet. Returns
/// the height used.
pub(crate) fn loose_card(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let (loose, picked) = app.loose_keys();
    let Some(picked) = picked else {
        return 0.0;
    };
    let mut cy = y;
    if let [only] = loose.as_slice() {
        let line = fill(EN.loose_card_key_is_loaded, &[&fp_text(*only).to_string()]);
        cy += ui.wrap_as((x, cy, w), BODY_L, Role::OnSurface, &line) + 16.0;
        cy += mark_line(app, ui, x, cy, w, *only);
    } else {
        let line = fill(EN.loose_card_keys_are, &[&loose.len().to_string()]);
        cy += ui.wrap_as((x, cy, w), BODY_L, Role::OnSurface, &line) + 10.0;
        for fp in &loose {
            let on = fp.0 == picked;
            ui.checkbox(x, cy + 11.0, on, true);
            let line = key_line(*fp, app.session.keys.key_label(*fp).unwrap_or(""));
            let line = ui.fit_as(BODY_M.mono(), &line, w - 30.0);
            ui.text_mid_as(
                (x + 30.0, cy, 40.0),
                BODY_M.mono(),
                if on {
                    Role::OnSurface
                } else {
                    Role::OnSurfaceVariant
                },
                &line,
            );
            ui.press_box(x, cy, w, 40.0, Action::PickKey(fp.0));
            ui.rule(x, cy + 40.0, w, Role::OutlineVariant);
            cy += 42.0;
            cy += mark_line(app, ui, x, cy, w, *fp);
        }
        cy += 14.0;
    }
    let items = [
        (FROM_KEY, Style::Primary, Action::KeyWallet(picked, 1)),
        (
            EN.loose_card_add_another,
            Style::Secondary,
            Action::KeyWallet(picked, 2),
        ),
        (
            EN.loose_card_restore,
            Style::Secondary,
            Action::RestoreWallet,
        ),
    ];
    cy += if ui.compact {
        crate::compact::screen::stack(ui, x, cy, w, &items)
    } else {
        wrap_buttons(ui, x, cy, w, BUTTON_H, &items)
    };
    cy - y
}

/// Wallets' list pane (`docs/LAYOUT.md` §5): each wallet, the one the
/// detail shows chosen, then the keys in no wallet and the ways to more,
/// in `(x, top, w, h)`, scrolled on its own.
fn wallet_list(app: &View, ui: &mut Ui, rect: (f32, f32, f32, f32)) {
    use faraday_draw::tokens::{BUTTON_H, GAP, LIST_PAD, TOUCH};
    let mut pane = crate::panes::screen::ListPane::begin(app, ui, rect);
    let (x, listw, _) = pane.at();
    let total = app.session.wallets.len();
    let none = total == 0;
    let (loose, _) = app.loose_keys();
    // The card offers the loose keys' next steps, and Load or restore,
    // when no wallet is loaded.
    let card_offers = none && !loose.is_empty();
    // The wallets lead the list unlabelled: the top app bar names the
    // page Wallets.
    for (i, wlt) in app.session.wallets.iter().enumerate() {
        // Can spend at its end (`docs/history/SCAN-TO-SIGN.md` §2.4).
        let can = app.readiness(i) == 2;
        let differs = app.silent_differs(i);
        let (m, _) = Session::quorum(wlt);
        let here = app
            .session
            .slots(wlt)
            .iter()
            .filter(|s| s.held_by.is_some())
            .count()
            .min(m);
        let shape = Session::shape(wlt);
        let line = if can {
            shape
        } else if here == 0 {
            fill(EN.wallets_watch_only, &[&shape.to_string()])
        } else {
            fill(
                EN.wallets_of_here,
                &[&shape.to_string(), &here.to_string(), &m.to_string()],
            )
        };
        let item = crate::panes::screen::Item {
            headline: &wlt.name,
            supporting: &line,
            trailing: if differs {
                Some((EN.wallets_does_not_match, Role::Error.paint()))
            } else if crate::wallet::another_key_spends(wlt) {
                Some((EN.wallets_another_can_spend, Role::Error.paint()))
            } else {
                can.then(|| (EN.wallets_can_spend, Role::Ok.paint()))
            },
            action: Action::PickWallet(i),
        };
        pane.item(ui, &item, i == app.wallets.wallet());
    }
    // Keys not in any wallet.
    if !loose.is_empty() {
        pane.advance(GAP);
        section_label(ui, x + LIST_PAD, pane.at().2, EN.wallets_keys_without);
        pane.advance(TOUCH / 2.0 + GAP);
        for fp in &loose {
            let line = key_line(*fp, app.session.keys.key_label(*fp).unwrap_or(""));
            let line = ui.fit_as(BODY_S, &line, listw - 2.0 * LIST_PAD);
            ui.text_as(x + LIST_PAD, pane.at().2, BODY_S, Role::OnSurface, &line);
            pane.advance(22.0);
            let h = mark_line(app, ui, x + LIST_PAD, pane.at().2, listw, *fp);
            pane.advance(h);
            // With wallets loaded the card shows one of them: the key's
            // own row offers the wallet from it.
            if !none {
                let y = pane.at().2;
                ui.button(
                    x + LIST_PAD,
                    y,
                    None,
                    BUTTON_H,
                    FROM_KEY,
                    Style::Secondary,
                    Action::KeyWallet(fp.0, 1),
                );
                pane.advance(BUTTON_H + GAP);
            }
        }
    }
    pane.advance(GAP);
    if none && !card_offers {
        let y = pane.at().2;
        ui.button(
            x + LIST_PAD,
            y,
            None,
            BUTTON_H,
            EN.wallets_restore_wallet,
            Style::Secondary,
            Action::RestoreWallet,
        );
        pane.advance(BUTTON_H + GAP);
    }
    add_key_button(app, ui, &mut pane);
    pane.end(ui);
}

/// The list pane's last button: Scan a key for the wallet shown when it
/// is short of keys (`docs/history/SCAN-TO-SIGN.md` §2.4), else Add a key.
fn add_key_button(app: &View, ui: &mut Ui, pane: &mut crate::panes::screen::ListPane) {
    use faraday_draw::tokens::{BUTTON_H, GAP, LIST_PAD};
    let (x, listw, _) = pane.at();
    let short = app
        .session
        .wallets
        .get(app.wallets.wallet())
        .filter(|wl| wl.policy().silent().is_none() && !app.can_spend(app.wallets.wallet()));
    let (add_label, add_action) = match short {
        Some(wl) => (
            fill(EN.wallets_scan_key_title, &[&wl.name.to_string()]),
            Action::ScanKey(app.wallets.wallet()),
        ),
        None => (EN.wallets_add_a_key.to_string(), Action::KeyGen(None)),
    };
    let add_label = ui.fit_as(LABEL_L, &add_label, listw - 2.0 * LIST_PAD - 32.0);
    let y = pane.at().2;
    let style = if app.may_load_keys() {
        Style::Secondary
    } else {
        Style::Disabled
    };
    ui.button(
        x + LIST_PAD,
        y,
        None,
        BUTTON_H,
        &add_label,
        style,
        add_action,
    );
    pane.advance(BUTTON_H + GAP);
}

/// Wallets on the rail's windows from expanded up: the list pane and the
/// chosen wallet's page beside it (`docs/LAYOUT.md` §5).
fn panes_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    let p = crate::panes::screen::split(x0, cw);
    wallet_list(app, ui, (p.list.0, p.top, p.list.1, h - p.top));
    let (dx, _) = p.detail;
    crate::compact::screen::wallet_card(app, ui, dx - M, p.reading() + 2.0 * M, h);
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use faraday_draw::tokens::MARGIN_WIDE;
    if ui.compact && crate::panes::screen::shown(app, Screen::Wallets) {
        return panes_compact(app, ui, x0, cw, h);
    }
    if ui.compact {
        crate::compact::screen::wallet_card(app, ui, x0, cw, h);
        return;
    }
    let x = x0 + MARGIN_WIDE;
    let width = cw - 2.0 * MARGIN_WIDE;
    let mut y = super::bar_head(ui, None, EN.wallets_wallets);

    // An Inbox PSBT leads.
    let lead = match app.wallets.spend() {
        Some(s) => Some((
            s.spend.source.clone(),
            EN.wallets_signing,
            Action::Nav(Screen::Spend),
            EN.wallets_continue,
        )),
        None => app.lead_psbt().map(|i| {
            (
                app.inbox[i].name.clone(),
                EN.wallets_in_files,
                Action::StartSpend(i),
                EN.wallets_review,
            )
        }),
    };
    if let Some((name, where_, action, verb)) = lead
        && !app.session.wallets.is_empty()
    {
        // A filled card: what leads stands in secondary-container, never
        // a tint (`docs/DESIGN.md` §3).
        let ink = Role::OnSecondaryContainer;
        let card = Shape::new(x, y, width, 48.0, tokens::SHAPE_M);
        ui.fill(x, y, width, 48.0, tokens::SHAPE_M, Role::SecondaryContainer);
        ui.icon(x + 14.0, y + 14.0, 20.0, Icon::File, 13.0, ink);
        let nw = ui.text_mid_as((x + 44.0, y, 48.0), BODY_M.mono(), ink, &name);
        ui.text_mid_as((x + 56.0 + nw, y, 48.0), BODY_M, ink, where_);
        ui.text_right_as((x + width - 18.0, y, 48.0), LABEL_L, ink, verb);
        ui.press(card, ink.paint(), action);
        y += 64.0;
    }

    let bottom = h - 28.0;
    let none = app.session.wallets.is_empty();
    let (loose, _) = app.loose_keys();
    let card_offers = none && !loose.is_empty();
    // The list pane, and the chosen wallet's card beside it; the card
    // alone where a chart's side sheet leaves no room for both.
    let p = if crate::panes::screen::fits(cw) {
        let p = crate::panes::screen::split(x0, cw);
        wallet_list(app, ui, (p.list.0, y, p.list.1, bottom - y));
        p
    } else {
        crate::panes::screen::alone(x0, cw)
    };

    // The wallet card: its reading parts at most `COLUMN_MAX`, the
    // chart the card's width.
    let (cx, cwid) = p.detail;
    let ch = bottom - y;
    ui.card(cx, y, cwid, ch, Role::OutlineVariant);
    let Some(wlt) = app.session.wallets.get(app.wallets.wallet()) else {
        if card_offers {
            loose_card(app, ui, cx + 24.0, y + 24.0, cwid - 48.0);
        } else {
            let msg = EN.wallets_no_wallet_loaded;
            ui.wrap_as(
                (cx + 24.0, y + 24.0, cwid - 48.0),
                BODY_L,
                Role::OnSurfaceVariant,
                msg,
            );
        }
        return;
    };
    let ix = cx + 24.0;
    let chart_w = cwid - 48.0;
    let iw = chart_w.min(faraday_draw::tokens::COLUMN_MAX);
    let mut cy = y + 22.0;
    // New wallet from this one, while it is still to be drawn: beside
    // Rename, or in the foot.
    let mut new_from = crate::create::NewKind::of(wlt.policy()).is_some();
    match app.wallets.renaming() {
        Some(name) => {
            ui.fill(
                ix - 6.0,
                cy - 4.0,
                360.0,
                38.0,
                8.0,
                Role::SurfaceContainerHighest,
            );
            // Typing goes to it: Material's focused field indicator.
            let field = (ix - 6.0, cy - 4.0, 360.0, 38.0);
            let width = faraday_draw::tokens::FIELD_EDGE_ACTIVE;
            crate::menu::outline(ui, field, 8.0, width, Role::Primary.paint());
            ui.typed(
                ix,
                cy - 4.0,
                38.0,
                (TITLE_L.medium()).size,
                W::from((TITLE_L.medium()).face),
                Role::OnSurface,
                name,
                348.0,
                true,
                Action::Rename,
            );
            ui.press_box(ix - 6.0, cy - 4.0, 360.0, 38.0, Action::Rename);
            ui.text_as(
                ix + 370.0,
                cy + 8.0,
                BODY_S,
                Role::OnSurfaceVariant,
                EN.wallets_enter_keeps,
            );
        }
        None => {
            let nw = ui.text_as(ix, cy, TITLE_L.medium(), Role::OnSurface, &wlt.name);
            let rw = ui.button(
                ix + nw + 14.0,
                cy,
                None,
                BUTTON_H,
                EN.wallets_rename,
                Style::Ghost,
                Action::Rename,
            );
            // Create filled from this wallet (`docs/history/NEW-WALLET.md`
            // §16.2), beside Rename where it fits before the source's
            // pill, so the foot keeps its one row; else in the foot.
            let bx = ix + nw + 14.0 + rw + 6.0;
            let bw = ui.measure_as(LABEL_L, EN.wallets_new_wallet_button) + 32.0;
            let pill = ui.measure_as(
                BODY_S,
                &crate::wallet::source_tag(&wlt.source, wlt.origin, true),
            ) + 24.0;
            if new_from && bx + bw + 12.0 <= ix + iw - pill {
                ui.button(
                    bx,
                    cy,
                    None,
                    BUTTON_H,
                    EN.wallets_new_wallet_button,
                    Style::Ghost,
                    Action::NewWalletFrom(app.wallets.wallet()),
                );
                new_from = false;
            }
        }
    }
    let src = crate::wallet::source_tag(&wlt.source, wlt.origin, true);
    let sw = ui.measure_as(BODY_S, &src) + 24.0;
    ui.pill(ix + iw - sw, cy, 28.0, &src);
    // Under the name's line, as tall as the text size makes it.
    cy += ui.advance_as(TITLE_L, 34.0);
    ui.text_as(ix, cy, BODY_M, Role::OnSurfaceVariant, &Session::shape(wlt));
    cy += ui.advance_as(BODY_M, 26.0);
    cy += super::parts::key_path_line(app, ui, (ix, cy, iw), wlt.policy());
    if let Some(r) = wlt.policy().silent() {
        // Where its backup is (§5.2): a press opens Backups on it.
        cy += backup_line(app, ui, ix, cy, iw, app.wallets.wallet());
        silent_wallet_card(app, ui, r, ix, iw, cy, y + ch);
        return;
    }

    let (m, _) = Session::quorum(wlt);
    let slots = app.session.slots(wlt);
    let here = slots.iter().filter(|s| s.held_by.is_some()).count().min(m);

    // Actions: the first along the foot, Remove from session at the
    // end; a row more above when they do not fit the card's width.
    // The first: from the vault to a spend (`docs/history/NEW-WALLET.md` §11.2).
    let (spend_label, spend_action) = app.spend_button(app.wallets.wallet());
    let mut items: Vec<(&str, Style, Action)> = vec![(&spend_label, Style::Primary, spend_action)];
    items.push((
        EN.wallets_show_wallet,
        Style::Secondary,
        Action::QrWallet(app.wallets.wallet()),
    ));
    // A backup left part way is offered as such (`docs/history/NEW-WALLET.md`
    // §12.3).
    let back_up = if app.wallet_backup_unfinished(app.wallets.wallet()) {
        EN.backup_flow_continue
    } else {
        EN.wallets_back_up
    };
    items.push((
        back_up,
        Style::Secondary,
        Action::Backup(app.wallets.wallet()),
    ));
    if new_from {
        items.push((
            EN.wallets_new_wallet_button,
            Style::Secondary,
            Action::NewWalletFrom(app.wallets.wallet()),
        ));
    }
    if app
        .session
        .message_wallets()
        .contains(&app.wallets.wallet())
    {
        items.push((
            EN.wallets_sign_a_message,
            Style::Secondary,
            Action::SignMessage,
        ));
    }
    let end = (
        EN.wallets_remove_from_session,
        Style::Ghost,
        Action::RemoveWallet(app.wallets.wallet()),
    );
    // The actions stand under the body (`docs/LAYOUT.md` §17): right
    // after the chart and the address where the card has room, and at the
    // card's foot, the body scrolling above them, where it has not.
    let placed = place_actions(ui, iw, &items, end);
    let foot = y + ch - 22.0 - actions_height(placed.2);

    // The body, between the head and the actions: scrolled when the
    // chart makes it taller than the card.
    let view_top = cy - 6.0;
    let view_h = (foot - tokens::GAP - view_top).max(0.0);
    let clip = ui.rect(cx + 1.0, view_top, cwid - 2.0, view_h);
    ui.c.push_clip(clip);
    let top = cy;
    cy -= app.wallets.card_offset();
    // Where its backup is (§5.2): a press opens Backups on it.
    cy += backup_line(app, ui, ix, cy, iw, app.wallets.wallet());
    ui.fill(
        ix,
        cy,
        iw,
        50.0,
        faraday_draw::tokens::SHAPE_M,
        Role::SurfaceContainerHighest,
    );
    ui.stroke(
        ix,
        cy,
        iw,
        50.0,
        faraday_draw::tokens::SHAPE_M,
        Role::OutlineVariant,
    );
    let pw = ui.pips(ix + 16.0, cy + 20.0, m, here);
    ui.text_mid_as(
        (ix + 30.0 + pw, cy, 50.0),
        LABEL_L,
        Role::OnSurface,
        &fill(
            EN.wallets_of_signatures,
            &[&here.to_string(), &m.to_string()],
        ),
    );
    // Short: one Scan a key for any missing key; enough: Can spend
    // (`docs/history/SCAN-TO-SIGN.md` §2.2).
    if app.can_spend(app.wallets.wallet()) {
        // A key not of the wallet spends it alone: never the plain green.
        let (t, status) = if crate::wallet::another_key_spends(wlt) {
            (EN.wallets_another_can_spend, Status::Error)
        } else {
            (EN.wallets_can_spend, Status::Ok)
        };
        let chipw = ui.measure_as(BODY_S, t) + 34.0;
        ui.tag(ix + iw - 16.0 - chipw, cy + 12.0, t, status);
    } else {
        let label = EN.wallets_scan_a_key;
        let bw = ui.measure_as(LABEL_L, label) + 32.0;
        ui.button(
            ix + iw - 9.0 - bw,
            cy + 7.0,
            Some(bw),
            BUTTON_H,
            label,
            Style::Primary.when(app.may_load_keys()),
            Action::ScanKey(app.wallets.wallet()),
        );
    }
    cy += 66.0;

    // The wallet at a glance (`docs/history/NEW-WALLET.md` §6): its keys and
    // its backup, where each thing is and what each place gives.
    ui.text_as(
        ix,
        cy,
        LABEL_M,
        Role::OnSurfaceVariant,
        EN.wallets_at_a_glance,
    );
    cy += ui.line_as(LABEL_M) + faraday_draw::tokens::GRID;
    if let Some(g) = app.glance_of(app.wallets.wallet()) {
        // Wide content in the detail's width (`docs/LAYOUT.md` §17): a
        // chart no wider than the column stands in it, over the reading
        // parts; a wider one runs on towards the card's end.
        let dir = crate::glance::Direction::WalletFirst;
        let gw = crate::glance::natural_width(&g).clamp(iw, chart_w);
        cy += crate::glance::draw(ui, &g, dir, ix, cy, gw);
    }
    cy += slot_marks(app, ui, (ix, cy, iw), &slots);
    // Where the keys still missing may be.
    let quorum_here =
        slots.iter().filter(|s| s.held_by.is_some()).count() >= crate::wallet::needed(wlt);
    if !quorum_here {
        cy += 4.0;
        cy += missing_keys_line(app, ui, &slots, Screen::Wallets, ix, cy, iw);
    }
    cy += 16.0;
    ui.text_as(
        ix,
        cy,
        LABEL_M,
        Role::OnSurfaceVariant,
        EN.wallets_first_receive,
    );
    cy += ui.line_as(LABEL_M) + faraday_draw::tokens::GRID;
    let (addr, held) = app.receive_line(wlt, false, 0);
    // The row opens the address's code, unless it is held back.
    let open = (!held).then_some(Action::ShowCode(crate::Code::Receive(
        app.wallets.wallet(),
        0,
    )));
    let room = if held { 0.0 } else { tokens::ROW_CHEVRON_ROOM };
    // On one line where the smaller size fits it: an address split over
    // two lines can be read as whole from its first.
    let size = if ui.measure_as(BODY_L.mono(), &addr) <= iw - room {
        BODY_L.mono()
    } else {
        BODY_M.mono()
    };
    let role = if held { Role::Warning } else { Role::OnSurface };
    cy += crate::screens::address_press(ui, (ix, cy, iw), (size, role), &addr, open) + 8.0;
    ui.c.pop_clip();
    card_actions(
        ui,
        (ix, (cy + tokens::GAP).min(foot), iw),
        &items,
        end,
        &placed,
    );
    let content = cy + app.wallets.card_offset() - top;
    let max = (content - (view_h - (top - view_top))).max(0.0);
    ui.report_scroll_in(crate::ui::Slot::Card, clip, max, app.wallets.card_offset());
}

/// Where a wallet card's actions go in a row `w` wide: each item's row,
/// x and width, then the end's width and how many rows they take. Items
/// run from the left, a row more under where they do not fit; `end`
/// stands at the right of the last row. A lookup and text measures.
fn place_actions(
    ui: &Ui,
    w: f32,
    items: &[(&str, Style, Action)],
    end: (&str, Style, Action),
) -> (Vec<(usize, f32, f32)>, f32, usize) {
    let width = |ui: &Ui, label: &str| (ui.measure_as(LABEL_L, label) + 32.0).min(w);
    let mut placed: Vec<(usize, f32, f32)> = Vec::new();
    let (mut row, mut bx) = (0usize, 0.0f32);
    for &(label, ..) in items {
        let bw = width(ui, label);
        if bx > 0.0 && bx + bw > w {
            row += 1;
            bx = 0.0;
        }
        placed.push((row, bx, bw));
        bx += bw + tokens::GAP;
    }
    let ew = width(ui, end.0);
    if bx > 0.0 && bx + ew > w {
        row += 1;
    }
    (placed, ew, row + 1)
}

/// The space between the rule above a card's actions and their first row.
const ACTIONS_RULE_GAP: f32 = 14.0;

/// How tall `rows` rows of a card's actions stand, the rule above them
/// included.
fn actions_height(rows: usize) -> f32 {
    ACTIONS_RULE_GAP + rows as f32 * (BUTTON_H + tokens::GAP) - tokens::GAP
}

/// A wallet card's actions as `placed` puts them in a row `w` wide from
/// `x`, under a rule at `rule`: `items` from the left, `end` at the right
/// of the last row.
fn card_actions(
    ui: &mut Ui,
    (x, rule, w): (f32, f32, f32),
    items: &[(&str, Style, Action)],
    end: (&str, Style, Action),
    (placed, ew, rows): &(Vec<(usize, f32, f32)>, f32, usize),
) {
    ui.rule(x, rule, w, Role::OutlineVariant);
    let row_y = |r: usize| rule + ACTIONS_RULE_GAP + r as f32 * (BUTTON_H + tokens::GAP);
    for (&(label, style, action), &(r, px, bw)) in items.iter().zip(placed) {
        let label = ui.fit_as(LABEL_L, label, bw - 24.0);
        ui.button(x + px, row_y(r), Some(bw), BUTTON_H, &label, style, action);
    }
    let label = ui.fit_as(LABEL_L, end.0, ew - 24.0);
    ui.button(
        x + w - ew,
        row_y(rows - 1),
        Some(*ew),
        BUTTON_H,
        &label,
        end.1,
        end.2,
    );
}

/// A silent payments wallet's address as its card shows it, by how its
/// record stands against the loaded keys: the address where a loaded key
/// gives both of its points; the address and a caution while no key with
/// its fingerprint is loaded; no address and a danger line where a loaded
/// key with its fingerprint gives other points. Returns its height.
pub(crate) fn silent_address_lines(
    app: &View,
    ui: &mut Ui,
    r: &faraday_bip::silent_wallet::SilentWallet,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    use crate::silent::RecordCheck;
    let fp = fp_text(r.fingerprint).to_string();
    match app.silent_wallet_check(app.wallets.wallet()) {
        Some(RecordCheck::Differs) => ui.wrap_as(
            (x, y, w),
            BODY_M,
            Role::Error,
            &fill(EN.silent_record_differs, &[&fp]),
        ),
        Some(RecordCheck::Matches) => {
            ui.wrap_as((x, y, w), BODY_M.mono(), Role::OnSurface, &r.address())
        }
        _ => {
            let h =
                ui.wrap_as((x, y, w), BODY_M.mono(), Role::OnSurface, &r.address()) + tokens::GAP;
            h + ui.wrap_as(
                (x, y + h, w),
                BODY_S,
                Role::Warning,
                &fill(EN.silent_not_checked, &[&fp]),
            )
        }
    }
}

/// A silent payments wallet's card: its key, its address and the labels
/// handed out, then its page. It has nothing to sign: sending to and from
/// silent payments waits on upstream.
fn silent_wallet_card(
    app: &View,
    ui: &mut Ui,
    r: &faraday_bip::silent_wallet::SilentWallet,
    ix: f32,
    iw: f32,
    y: f32,
    bottom: f32,
) {
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    let may = app.may_load_keys();
    let mut cy = y;
    ui.text_as(ix, cy, LABEL_M, muted, EN.silent_wallet_card_key);
    cy += ui.line_as(LABEL_M) + faraday_draw::tokens::GRID;
    let held = app.session.keys.key_label(r.fingerprint);
    ui.text_mid_as((ix, cy, 46.0), BODY_M.mono(), ink, &fp_text(r.fingerprint));
    if let Some(label) = held {
        ui.text_mid_as((ix + 104.0, cy, 46.0), BODY_S, muted, label);
    }
    let (state, status) = if app.silent_differs(app.wallets.wallet()) {
        (EN.silent_wallet_card_differs, Status::Error)
    } else if held.is_some() {
        (EN.silent_wallet_card_here, Status::Ok)
    } else {
        (EN.silent_wallet_card_not_here, Status::Neutral)
    };
    let button = held.is_none().then(|| {
        let slot = crate::wallet::Slot::bare(r.fingerprint);
        if app.vault_key_for(&slot).is_some() {
            (
                EN.silent_wallet_card_load,
                Style::Primary,
                Action::Vault(crate::vault::VaultAction::LoadKeyOf(
                    r.fingerprint.0,
                    crate::vault::SlotOf::Bare,
                )),
            )
        } else {
            (
                EN.silent_wallet_card_add,
                Style::Secondary,
                Action::Entry(Some(r.fingerprint.0)),
            )
        }
    });
    let bw = button.map_or(0.0, |(l, _, _)| ui.measure_as(LABEL_L, l) + 28.0);
    let chipw = ui.measure_as(BODY_S, state) + 34.0;
    ui.tag(ix + iw - bw - 10.0 - chipw, cy + 10.0, state, status);
    if let Some((label, style, action)) = button {
        let style = if may { style } else { Style::Disabled };
        ui.button(
            ix + iw - bw,
            cy + 7.0,
            Some(bw),
            BUTTON_H,
            label,
            style,
            action,
        );
    }
    ui.rule(ix, cy + 46.0, iw, Role::OutlineVariant);
    cy += 64.0;
    ui.text_as(ix, cy, LABEL_M, muted, EN.silent_wallet_card_address_label);
    cy += ui.line_as(LABEL_M) + faraday_draw::tokens::GRID;
    cy += silent_address_lines(app, ui, r, (ix, cy, iw)) + 18.0;
    ui.text_as(ix, cy, LABEL_M, muted, EN.silent_wallet_card_labels);
    ui.text_as(ix + 180.0, cy, BODY_M.mono(), ink, &r.labels.to_string());
    let items = [
        (
            EN.silent_wallet_card_address,
            Style::Primary,
            Action::SWallet(app.wallets.wallet()),
        ),
        (
            EN.silent_wallet_card_show,
            Style::Secondary,
            Action::QrWallet(app.wallets.wallet()),
        ),
    ];
    let end = (
        EN.silent_wallet_card_remove,
        Style::Ghost,
        Action::RemoveWallet(app.wallets.wallet()),
    );
    // Under the labels line, or at the card's foot where that is higher.
    let placed = place_actions(ui, iw, &items, end);
    let foot = bottom - 22.0 - actions_height(placed.2);
    let rule = (cy + tokens::PAD + tokens::GAP).min(foot);
    card_actions(ui, (ix, rule, iw), &items, end, &placed);
}
