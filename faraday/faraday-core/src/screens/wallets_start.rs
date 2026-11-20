//! `Screen::WalletsStart`: the Wallets tab's first page, every flow by what the person has.

use super::catalog::Face;
use super::parts::FROM_KEY;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::{Session, key_line};
use crate::{Action, Screen};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, LABEL_M};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::wrap_buttons;

/// The five ways in: Create, Load or restore, Spend from a backup, Add a
/// key and Make a new key.
fn ways() -> [(Icon, &'static str, &'static str, Action); 5] {
    [
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
            Action::Nav(Screen::SpendBackup),
        ),
        (
            Icon::Keys,
            EN.start_add_a_key,
            EN.start_keys_first_wallets,
            Action::Entry(None),
        ),
        (
            Icon::Keys,
            EN.catalog_make_a_new_key,
            EN.catalog_make_key_start_line,
            Action::KeyGen(None),
        ),
    ]
}

/// The ways in as rows, one under the other. Returns their height.
fn ways_rows(ui: &mut Ui, (x, y): (f32, f32), width: f32) -> f32 {
    let mut cy = y;
    for (icon, label, sub, action) in ways() {
        cy += crate::compact::screen::row(
            ui,
            x,
            cy,
            width,
            Some(icon),
            label,
            sub,
            Role::OnSurfaceVariant.paint(),
            Some(action),
        );
    }
    cy - y
}

/// The ways in as buttons. Returns their height.
fn ways_buttons(ui: &mut Ui, (x, y): (f32, f32), width: f32) -> f32 {
    let buttons: Vec<_> = ways()
        .into_iter()
        .map(|(_, label, _, action)| (label, Style::Secondary, action))
        .collect();
    wrap_buttons(ui, x, y, width, BUTTON_H, &buttons)
}

/// Keys in no wallet, each with its way on, as on the desktop's Wallets
/// column. Returns their height.
fn loose_key_rows(app: &View, ui: &mut Ui, (x, y): (f32, f32), width: f32) -> f32 {
    let (loose, _) = app.loose_keys();
    let mut cy = y;
    for fp in &loose {
        let line = key_line(*fp, app.session.keys.key_label(*fp).unwrap_or(""));
        let line = ui.fit_as(BODY_M.mono(), &line, width);
        ui.text_mid_as((x, cy, 30.0), BODY_M.mono(), Role::OnSurface, &line);
        cy += 34.0;
        ui.button(
            x,
            cy,
            Some(width),
            BUTTON_H,
            FROM_KEY,
            Style::Secondary,
            Action::KeyWallet(fp.0, 1),
        );
        cy += 56.0;
    }
    cy - y
}

/// A vault file left locked, with the way to unlock it, as a row on the
/// small panel and as a line and a button beside it on the wide layout.
/// Returns its height; none when no file is locked.
fn unlock_line(app: &View, ui: &mut Ui, (x, y): (f32, f32), width: f32) -> f32 {
    let files = app.vault_files();
    let Some(i) = files.iter().position(|f| f.open.is_none()) else {
        return 0.0;
    };
    let line = fill(EN.start_is_in_files_locked, &[&files[i].name]);
    let open = Action::Vault(crate::vault::VaultAction::Open(i));
    if ui.compact {
        return crate::compact::screen::row(
            ui,
            x,
            y,
            width,
            Some(Icon::Lock),
            EN.start_unlock,
            &line,
            Role::OnSurfaceVariant.paint(),
            Some(open),
        );
    }
    ui.text_mid_as((x, y, 40.0), BODY_M, Role::OnSurfaceVariant, &line);
    let lw = ui.measure_as(BODY_M, &line);
    ui.button(
        x + lw + 16.0,
        y,
        None,
        BUTTON_H,
        EN.start_unlock,
        Style::Secondary,
        open,
    );
    BUTTON_H
}

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use super::menu_page::{self, Page, Section};
    let compact = ui.compact;
    // No wallet and no vault yet: the ways in are the page.
    let empty = app.session.wallets.is_empty()
        && app.vaults.open.is_empty()
        && !app.create_unfinished()
        && app.wallets.spend().is_none();
    let mut sections = vec![Section::block(
        Some(EN.start_your_wallets),
        |ui, (x, y), width| wallet_rows(app, ui, x, y, width, 2, &Action::OpenWallet),
    )];
    // On the small panel this is the wallet list: a key in no wallet is
    // listed after the wallets with its way on, as on the desktop's
    // Wallets column. With no wallet loaded the wallet page says it.
    if compact && !app.session.wallets.is_empty() && !app.loose_keys().0.is_empty() {
        sections.push(Section::block(
            Some(EN.start_keys_without),
            |ui, at, width| loose_key_rows(app, ui, at, width),
        ));
    }
    // Wallets loaded: the list and the wallet card carry every job
    // (Sign a transaction, Back up, Show wallet QR, Sign a message) and
    // Tools has the rest; Add a key stays under the list.
    // Spend from a backup stays offered with wallets loaded: it is the
    // way back into a spend left for another tab, which lights Wallets.
    if !empty {
        sections.push(if compact {
            Section::block(None, ways_rows)
        } else {
            Section::block(None, ways_buttons)
        });
    } else if compact {
        sections.push(Section::block(None, ways_rows));
        sections.push(Section::block(None, |ui, at, width| {
            unlock_line(app, ui, at, width)
        }));
    } else {
        let faces = ways()
            .into_iter()
            .map(|(_, name, line, action)| Face {
                mark: crate::screens::catalog::Mark::None,
                name,
                line,
                warn: false,
                tags: &[],
                opens: true,
                action,
            })
            .collect();
        sections.push(Section::tiles(None, faces));
        sections.push(Section::block(None, |ui, at, width| {
            unlock_line(app, ui, at, width)
        }));
    }
    let page = Page {
        name: EN.start_wallets,
        back: false,
        search: None,
        most_columns: usize::MAX,
        fixed_columns: None,
        above: None,
        bar_still: false,
    };
    menu_page::draw(app, ui, (x0, cw, h), &page, &sections);
}

/// The loaded wallets, two to a row, at most `rows` rows: each with its
/// shape and whether the seeds here sign for it. Returns its height.
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
            let here = app.session.held_toward_quorum(w);
            let (state, tone) = match ready {
                _ if app.silent_differs(i) => {
                    (EN.wallets_does_not_match.to_string(), Role::Error.paint())
                }
                _ if crate::wallet::another_key_spends(w) => (
                    fill(
                        EN.wallet_rows_another_can_spend,
                        &[&here.to_string(), &m.to_string()],
                    ),
                    Role::Error.paint(),
                ),
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
            let sub = format!("{} · {state}", &*Session::shape(w));
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
        // Its edge outline-variant as a card's: its marker says how it
        // stands (`docs/DESIGN.md` §3).
        let r = faraday_draw::tokens::SHAPE_M;
        ui.fill(rx, ry, rw, 44.0, r, Role::Surface);
        ui.stroke(rx, ry, rw, 44.0, r, Role::OutlineVariant);
        ui.press(
            Shape::new(rx, ry, rw, 44.0, r),
            Role::OnSurface.paint(),
            action,
        );
        let (m, _) = Session::quorum(w);
        let here = app.session.held_toward_quorum(w);
        // The marker on the right: ready, how many here, or none.
        let differs = app.silent_differs(i);
        let marker = match ready {
            _ if differs => EN.wallets_does_not_match.to_string(),
            _ if crate::wallet::another_key_spends(w) => fill(
                EN.wallet_rows_another_can_spend,
                &[&here.to_string(), &m.to_string()],
            ),
            2 => fill(
                EN.wallet_rows_can_spend,
                &[&here.to_string(), &m.to_string()],
            ),
            1 => fill(EN.wallet_rows_of_here, &[&here.to_string(), &m.to_string()]),
            _ => EN.wallet_rows_watch_only.to_string(),
        };
        let tone = match ready {
            _ if differs => Role::Error,
            _ if crate::wallet::another_key_spends(w) => Role::Error,
            2 => Role::OnOkContainer,
            1 => Role::OnSurface,
            _ => Role::OnSurfaceVariant,
        };
        let mw = ui.measure_as(LABEL_M, &marker) + 20.0;
        if ready == 2 && !crate::wallet::another_key_spends(w) {
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
    }
    let mut used = shown.div_ceil(cols) as f32 * 52.0;
    if shown < total {
        let more = fill(EN.wallet_rows_more, &[&(total - shown).to_string()]);
        let mw = ui.measure_as(LABEL_M, &more) + 8.0;
        ui.press_box(x, y + used - 4.0, mw, 24.0, open(order[shown]));
        ui.text_mid_as((x, y + used - 4.0, 24.0), LABEL_M, Role::Primary, &more);
        used += 24.0;
    }
    used + 6.0
}
