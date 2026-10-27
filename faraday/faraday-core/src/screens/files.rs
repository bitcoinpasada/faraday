//! `Screen::Files`: Inbox and Outbox.

use super::parts::{guide_text_learn, kind_line, pull_line};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::{FileKind, fp_text};
use crate::{Action, Screen};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, LABEL_M, TITLE_M, TITLE_S};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{title, wrap_buttons};

/// What the Inbox gives, put together: its wallets, once each, with the
/// seeds here that are their keys; seeds no wallet names, each a potential
/// wallet; and backups short of parts or sealed. Nothing when it gives
/// nothing to load. Returns its height.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn inbox_panel(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if ui.compact {
        return inbox_panel_compact(app, ui, x, y, w);
    }
    let found = app.inbox_found();
    let wallets = found.to_load();
    let potential: Vec<&crate::inbox::FoundSeed> = found.potential();
    if wallets.is_empty()
        && potential.is_empty()
        && found.waiting.is_empty()
        && found.xpubs.is_empty()
    {
        return 0.0;
    }
    // Three rows a group at most, so the Inbox and Outbox keep their room.
    const ROWS: usize = 3;
    let wrows = wallets.len().min(ROWS);
    let prows = potential.len().min(ROWS);
    let qrows = found.waiting.len().min(ROWS);
    let xrows = found.xpubs.len().min(ROWS);
    let mut h = 52.0;
    if !wallets.is_empty() {
        h += 26.0 + wrows as f32 * 38.0 + if wallets.len() > ROWS { 22.0 } else { 0.0 } + 54.0;
    }
    if !potential.is_empty() {
        h += 26.0 + prows as f32 * 42.0 + if potential.len() > ROWS { 22.0 } else { 0.0 } + 10.0;
    }
    if !found.xpubs.is_empty() {
        h += 26.0 + xrows as f32 * 42.0 + 10.0;
    }
    if !found.waiting.is_empty() {
        h += 26.0 + qrows as f32 * 40.0 + 6.0;
    }
    ui.card(x, y, w, h, Role::OutlineVariant);
    ui.icon(
        x + 20.0,
        y + 18.0,
        20.0,
        Icon::Download,
        13.0,
        Role::Primary,
    );
    ui.text_mid_as(
        (x + 48.0, y + 14.0, 28.0),
        TITLE_M,
        Role::OnSurface,
        app.medium.from_box(),
    );
    let mut ty = y + 52.0;
    let may = app.may_load_keys();
    if !wallets.is_empty() {
        ui.text_as(
            x + 20.0,
            ty,
            LABEL_L,
            Role::OnSurfaceVariant,
            EN.inbox_panel_wallets,
        );
        ty += 26.0;
        let many = wallets.len() > 1;
        for (i, fw) in wallets.iter().enumerate().take(ROWS) {
            let chosen = !app.inbox.skip().contains(&fw.descriptor);
            let mut rx = x + 20.0;
            if many {
                ui.checkbox(rx, ty + 8.0, chosen, true);
                ui.hit(x + 16.0, ty, w - 32.0, 34.0, Action::InboxChoose(i));
                rx += 28.0;
            }
            ui.icon(
                rx,
                ty + 7.0,
                20.0,
                Icon::Wallet,
                11.0,
                Role::OnSurfaceVariant,
            );
            let nw = ui.text_mid_as((rx + 26.0, ty, 34.0), TITLE_S, Role::OnSurface, &fw.name);
            let n = fw.keys.len().max(1);
            let line = fill(
                EN.inbox_panel_of_here,
                &[
                    &fw.shape.to_string(),
                    &fw.seeds_here.to_string(),
                    &n.to_string(),
                    (if n == 1 {
                        EN.inbox_panel_key
                    } else {
                        EN.inbox_panel_keys
                    }),
                    &(if fw.files.len() > 1 {
                        format!(
                            " {}",
                            fill(EN.inbox_panel_files_agree, &[&fw.files.len().to_string()])
                        )
                    } else if fw.from_shares {
                        format!(" {}", EN.inbox_panel_from_split_sheets)
                    } else {
                        String::new()
                    })
                    .to_string(),
                ],
            );
            let line = ui.fit_as(BODY_S, &line, w - nw - 260.0);
            ui.text_mid_as(
                (rx + 40.0 + nw, ty, 34.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &line,
            );
            if fw.seeds_here >= fw.needed {
                let tag = EN.inbox_panel_ready_to_sign;
                let tw = ui.measure_as(LABEL_M, tag) + 20.0;
                ui.fill(
                    x + w - 20.0 - tw,
                    ty + 5.0,
                    tw,
                    24.0,
                    12.0,
                    Role::OkContainer,
                );
                ui.text_mid_as(
                    (x + w - 10.0 - tw, ty, 34.0),
                    LABEL_M,
                    Role::OnOkContainer,
                    tag,
                );
            }
            ty += 38.0;
        }
        if wallets.len() > ROWS {
            ui.text_mid_as(
                (x + 20.0, ty - 6.0, 22.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &fill(
                    EN.inbox_panel_more_loaded,
                    &[&(wallets.len() - ROWS).to_string()],
                ),
            );
            ty += 22.0;
        }
        let chosen = wallets
            .iter()
            .filter(|w| !many || !app.inbox.skip().contains(&w.descriptor))
            .count();
        let with_seeds = wallets
            .iter()
            .filter(|w| !many || !app.inbox.skip().contains(&w.descriptor))
            .any(|w| w.seeds_here > 0);
        let label = match (chosen, with_seeds) {
            (0, _) => EN.inbox_panel_nothing.to_string(),
            (1, true) => EN.inbox_panel_load_the_wallet.to_string(),
            (1, false) => EN.inbox_panel_load_wallet.to_string(),
            (n, true) => fill(EN.inbox_panel_load_wallets_seeds, &[&n.to_string()]),
            (n, false) => fill(EN.inbox_panel_load_wallets, &[&n.to_string()]),
        };
        ui.button(
            x + 20.0,
            ty + 4.0,
            None,
            BUTTON_H,
            &label,
            Style::Primary.when(chosen > 0),
            Action::InboxLoad,
        );
        if !may {
            ui.text_mid_as(
                (
                    x + 20.0 + ui.measure_as(TITLE_S, &label) + 52.0,
                    ty + 4.0,
                    40.0,
                ),
                BODY_S,
                Role::Warning,
                &fill(EN.inbox_panel_keys_load, &[app.medium.noun()]),
            );
        }
        ty += 54.0;
    }
    if !potential.is_empty() {
        ui.text_as(
            x + 20.0,
            ty,
            LABEL_L,
            Role::OnSurfaceVariant,
            EN.inbox_panel_loose_keys,
        );
        ty += 26.0;
        for s in potential.iter().take(ROWS) {
            ui.icon(
                x + 20.0,
                ty + 9.0,
                20.0,
                Icon::Keys,
                11.0,
                Role::OnSurfaceVariant,
            );
            ui.text_mid_as(
                (x + 46.0, ty, 38.0),
                BODY_M.mono(),
                Role::OnSurface,
                &fp_text(s.fingerprint),
            );
            let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
            let line = ui.fit_as(BODY_S, &from.join(" · "), w - 420.0);
            ui.text_mid_as((x + 140.0, ty, 38.0), BODY_S, Role::OnSurfaceVariant, &line);
            let label = EN.inbox_panel_make_wallet;
            let bw = ui.measure_as(LABEL_L, label) + 28.0;
            ui.button(
                x + w - 20.0 - bw,
                ty + 3.0,
                Some(bw),
                BUTTON_H,
                label,
                if may {
                    Style::Secondary
                } else {
                    Style::Disabled
                },
                Action::PotentialOpen(s.fingerprint.0),
            );
            ty += 42.0;
        }
        if potential.len() > ROWS {
            ui.text_mid_as(
                (x + 20.0, ty - 6.0, 22.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &fill(
                    EN.inbox_panel_more,
                    &[&(potential.len() - ROWS).to_string()],
                ),
            );
            ty += 22.0;
        }
        ty += 10.0;
    }
    if !found.xpubs.is_empty() {
        ui.text_as(
            x + 20.0,
            ty,
            LABEL_L,
            Role::OnSurfaceVariant,
            EN.inbox_panel_loose_xpubs,
        );
        ty += 26.0;
        for (i, xp) in found.xpubs.iter().enumerate().take(ROWS) {
            ui.icon(
                x + 20.0,
                ty + 9.0,
                20.0,
                Icon::Wallet,
                11.0,
                Role::OnSurfaceVariant,
            );
            let fp = xp
                .fingerprint
                .map(fp_text)
                .unwrap_or_else(|| EN.inbox_panel_no_origin.to_string());
            ui.text_mid_as((x + 46.0, ty, 38.0), BODY_M.mono(), Role::OnSurface, &fp);
            let line = format!(
                "{} · {}",
                xp.path.clone().unwrap_or_default(),
                xp.files
                    .first()
                    .and_then(|k| app.inbox.get(*k))
                    .map_or(String::new(), |it| it.name.clone())
            );
            let line = ui.fit_as(BODY_S, &line, w - 460.0);
            ui.text_mid_as((x + 140.0, ty, 38.0), BODY_S, Role::OnSurfaceVariant, &line);
            let label = EN.inbox_panel_make_watch_only;
            let bw = ui.measure_as(LABEL_L, label) + 28.0;
            ui.button(
                x + w - 20.0 - bw,
                ty + 3.0,
                Some(bw),
                BUTTON_H,
                label,
                Style::Secondary,
                Action::XpubOpen(i),
            );
            ty += 42.0;
        }
        ty += 10.0;
    }
    if !found.waiting.is_empty() {
        ui.text_as(
            x + 20.0,
            ty,
            LABEL_L,
            Role::OnSurfaceVariant,
            EN.inbox_panel_not_complete,
        );
        ty += 26.0;
        for q in found.waiting.iter().take(ROWS) {
            let line = ui.fit_as(BODY_M, &q.line, w - 200.0);
            ui.text_mid_as((x + 20.0, ty, 36.0), BODY_S, Role::OnSurfaceVariant, &line);
            ty += 40.0;
        }
    }
    h
}

/// Add to vault, for an Inbox file a vault keeps; nothing while no vault
/// is open (F60: the vault's own row unlocks it). Returns the width used.
fn add_to_vault(app: &View, ui: &mut Ui, x: f32, y: f32, k: usize, style: Style) -> f32 {
    if app.vault_way(Screen::Files).is_some() {
        return 0.0;
    }
    let action = Action::Vault(crate::vault::VaultAction::AddFile(k));
    ui.button(x, y, None, BUTTON_H, EN.add_to_vault_button, style, action) + 8.0
}

/// Restore the wallet, on a share of a split backup: the shares in the
/// Inbox put together, or how many keys are still missing.
fn restore_shares_button(app: &View, ui: &mut Ui, x: f32, y: f32) -> f32 {
    let texts: Vec<String> = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Share)
        .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
        .collect();
    match crate::restore::merge(&texts) {
        Ok(m) if m.whole.is_some() => {
            ui.button(
                x,
                y,
                None,
                BUTTON_H,
                &fill(
                    EN.restore_shares_button_restore,
                    &[&texts.len().to_string()],
                ),
                Style::Primary,
                Action::RestoreShares,
            ) + 8.0
        }
        Ok(m) => {
            let line = fill(
                EN.restore_shares_button_progress,
                &[&m.have.len().to_string(), &m.n.to_string()],
            );
            ui.text_mid_as((x, y, 36.0), BODY_M, Role::Warning, &line);
            ui.measure_as(BODY_M, &line) + 16.0
        }
        Err(e) => {
            ui.text_mid_as((x, y, 36.0), BODY_M, Role::Error, &e);
            ui.measure_as(BODY_M, &e) + 16.0
        }
    }
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use faraday_draw::tokens::{LIST_PANE, MARGIN_WIDE, SPACER};
    if ui.compact {
        return files_compact(app, ui, x0, cw, h);
    }
    // The two lists are two list panes side by side at the start, and
    // what is above them takes their width (`docs/LAYOUT.md` §17).
    let x = x0 + MARGIN_WIDE;
    let width = (2.0 * LIST_PANE + SPACER).min(cw - 2.0 * MARGIN_WIDE);
    let mut y = 36.0;
    title(ui, x, y, EN.files_files);
    stick_chip(app, ui, x0 + cw - MARGIN_WIDE);
    let label = EN.files_in_memory_emptied;
    let pw = ui.measure_as(BODY_S, label) + 24.0;
    ui.pill(x + width - pw, y + 2.0, 30.0, label);
    let sw = ui.measure_as(LABEL_L, EN.files_scan_a_qr_code) + 32.0;
    ui.button(
        x + width - pw - 12.0 - sw,
        y,
        Some(sw),
        BUTTON_H,
        EN.files_scan_a_qr_code,
        Style::Secondary,
        Action::Scan,
    );
    y += 64.0;
    y += guide_text_learn(
        app,
        ui,
        x,
        y,
        width,
        &fill(
            EN.files_holds_what_was,
            &[app.medium.from_box(), app.medium.for_box()],
        ),
        crate::learn::Topic::AirGap,
    );
    // Arrived from Import and load: pulling the stick is the next step.
    if !app.stick.is_empty() && !app.visit.load_after.is_empty() {
        ui.fill(x, y, width, 48.0, 10.0, Role::WarningContainer);
        ui.stroke(x, y, width, 48.0, 10.0, Role::Warning.with_alpha(90));
        ui.icon(
            x + 16.0,
            y + 14.0,
            20.0,
            app.medium.icon(),
            13.0,
            Role::OnWarningContainer,
        );
        ui.text_mid_as(
            (x + 44.0, y, 48.0),
            TITLE_S,
            Role::OnWarningContainer,
            &pull_line(app),
        );
        y += 60.0;
    }
    for v in 0..app.vaults.open.len() {
        y += vault_panel(app, ui, v, x, y, width) + 20.0;
    }
    let ip = inbox_panel(app, ui, x, y, width);
    if ip > 0.0 {
        y += ip + 20.0;
    }
    let colw = (width - SPACER) / 2.0;
    // Inbox.
    let lw = ui.text_as(x, y, TITLE_M, Role::OnSurfaceVariant, app.medium.from_box());
    ui.text_as(
        x + lw + 10.0,
        y + 2.0,
        BODY_S,
        Role::OnSurfaceVariant,
        &app.inbox.len().to_string(),
    );
    // The two lists scroll each on its own under their titles, above the
    // foot, each with its own scrollbar: From the stick by the page's
    // offset, For the stick by `files_out_offset`.
    let top = y + 28.0;
    let bottom = h - 32.0 - 48.0 - 12.0;
    let view = (bottom - top).max(0.0);
    let groups = 3.0;
    // The receipt under For the stick: its heading and a row per file.
    let receipt_h = app.visit.receipt().as_ref().map_or(0.0, |r| {
        30.0 + r.files.len() as f32 * 28.0 + if app.outbox.is_empty() { 84.0 } else { 0.0 }
    });
    let content_in = app.inbox.len().max(1) as f32 * 124.0 + 8.0;
    let content_out = app.outbox.len() as f32 * 78.0 + groups * 30.0 + receipt_h + 8.0;
    let shift = app.list_offset.min((content_in - view).max(0.0));
    let shift_out = app
        .visit
        .files_out_offset()
        .min((content_out - view).max(0.0));
    let split = x + colw + SPACER / 2.0;
    let clip = ui.rect(x0, top, split - x0, view);
    let clip_out = ui.rect(split, top, colw + SPACER, view);
    ui.c.push_clip(clip);
    let mut iy = y + 32.0 - shift;
    for (k, item) in app.inbox.iter().enumerate() {
        // A row scrolled out of the list's sight is passed over.
        if !ui.shows(iy, 112.0) {
            iy += 124.0;
            continue;
        }
        ui.card(x, iy, colw, 112.0, Role::OutlineVariant);
        ui.fill(
            x + 16.0,
            iy + 14.0,
            36.0,
            36.0,
            10.0,
            Role::SecondaryContainer,
        );
        ui.icon(
            x + 16.0,
            iy + 14.0,
            36.0,
            Icon::Download,
            14.0,
            Role::OnSecondaryContainer,
        );
        let name = ui.fit_as(BODY_M.mono(), &item.name, colw - 90.0);
        ui.text_as(x + 66.0, iy + 14.0, BODY_M.mono(), Role::OnSurface, &name);
        let line = if item.kind == FileKind::Vault
            && vault_of_row(&app.vault_files(), item).is_some_and(|f| f.open.is_some())
        {
            kind_line(item.kind, item.bytes.len()).replace(EN.files_locked, EN.files_open)
        } else {
            kind_line(item.kind, item.bytes.len())
        };
        ui.text_as(x + 66.0, iy + 36.0, BODY_S, Role::OnSurfaceVariant, &line);
        let mut bx = x + 66.0;
        let same_spend = app.wallets.spend().is_some_and(|s| {
            item.psbt()
                .is_some_and(|p| p.txid == s.spend.txid && item.name != s.spend.source)
        });
        match item.kind {
            FileKind::Psbt if same_spend => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    BUTTON_H,
                    EN.files_add_to_signatures,
                    Style::Primary,
                    Action::Collect(k),
                ) + 8.0;
            }
            FileKind::Entries => {
                let action = crate::vault::VaultAction::ImportEntries(k);
                // No vault open: no button (F60).
                if app.vault_way(Screen::Files).is_none() {
                    bx += ui.button(
                        bx,
                        iy + 64.0,
                        None,
                        BUTTON_H,
                        EN.files_import_into_vault,
                        Style::Primary,
                        Action::Vault(action),
                    ) + 8.0;
                }
            }
            FileKind::Vault => {
                let files = app.vault_files();
                // The row's own vault, or no button.
                if let Some(at) = vault_at(&files, item) {
                    let open = files[at].open.is_some();
                    bx += ui.button(
                        bx,
                        iy + 64.0,
                        None,
                        BUTTON_H,
                        if open {
                            EN.files_contents
                        } else {
                            EN.files_unlock
                        },
                        if open {
                            Style::Secondary
                        } else {
                            Style::Primary
                        },
                        Action::Vault(crate::vault::VaultAction::Open(at)),
                    ) + 8.0;
                }
            }
            FileKind::Psbt => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    BUTTON_H,
                    EN.files_sign,
                    Style::Tonal,
                    Action::StartSpend(k),
                ) + 8.0;
            }
            FileKind::Wallet | FileKind::SecretWallet => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    BUTTON_H,
                    EN.files_load_the_wallet,
                    Style::Primary,
                    Action::LoadWallet(k),
                ) + 8.0;
            }
            FileKind::Text => {
                bx += add_to_vault(app, ui, bx, iy + 64.0, k, Style::Primary);
            }
            FileKind::Transaction => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    BUTTON_H,
                    EN.files_decode,
                    Style::Primary,
                    Action::DecodeInbox(k),
                ) + 8.0;
            }
            FileKind::Share => {
                bx += restore_shares_button(app, ui, bx, iy + 64.0);
            }
            FileKind::Words => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    BUTTON_H,
                    EN.files_load_this_key,
                    Style::Primary,
                    Action::LoadKey(k),
                ) + 8.0;
            }
            FileKind::Message => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    BUTTON_H,
                    EN.files_check_the_signature,
                    Style::Primary,
                    Action::CheckMessage(k),
                ) + 8.0;
            }
            FileKind::Sheet if app.online => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    BUTTON_H,
                    EN.files_make_pdf,
                    Style::Primary,
                    Action::PdfInbox(k),
                ) + 8.0;
            }
            _ => {}
        }
        // The rest behind the ⋮ at the row's right end (decision 61).
        crate::file_menu::dots(
            app,
            ui,
            (x + colw - 16.0 - 36.0).max(bx),
            iy + 64.0,
            36.0,
            crate::file_menu::Row::In(k),
        );
        iy += 124.0;
    }
    if app.inbox.is_empty() {
        ui.stroke(x, iy, colw, 72.0, 12.0, Role::OutlineVariant);
        let t = &fill(EN.files_empty_files_are, &[app.medium.a()]);
        let tw = ui.measure_as(BODY_M, t);
        ui.text_mid_as(
            (x + (colw - tw) / 2.0, iy, 72.0),
            BODY_S,
            Role::OnSurfaceVariant,
            t,
        );
    }
    // Outbox, its title above the scrolled lists.
    let ox = x + colw + SPACER;
    ui.c.pop_clip();
    let lw = ui.text_as(ox, y, TITLE_M, Role::OnSurfaceVariant, app.medium.for_box());
    ui.text_as(
        ox + lw + 10.0,
        y + 2.0,
        BODY_S,
        Role::OnSurfaceVariant,
        &app.outbox.len().to_string(),
    );
    ui.c.push_clip(clip_out);
    let mut oy = y + 32.0 - shift_out;
    // Grouped by who may read them: public files, vaults sealed under a
    // passphrase, and any secret the person let out unprotected.
    use crate::secrets::Exposure;
    for (exposure, label, line, tone) in [
        (
            Exposure::Public,
            EN.files_public,
            EN.files_anyone_may_read.to_string(),
            Role::OnSurfaceVariant,
        ),
        (
            Exposure::Sealed,
            EN.files_written_sealed,
            fill(EN.files_sealed_under, &[app.medium.a()]),
            Role::Ok,
        ),
        (
            Exposure::Secret,
            EN.files_unprotected_secrets,
            fill(EN.files_anyone_who_has, &[app.medium.noun()]),
            Role::Error,
        ),
    ] {
        let group: Vec<(usize, &crate::Item)> = app
            .outbox
            .iter()
            .enumerate()
            .filter(|(_, i)| i.exposure() == exposure)
            .collect();
        if group.is_empty() {
            continue;
        }
        let lw = ui.text_as(ox, oy, LABEL_M, tone, label);
        ui.text_as(ox + lw + 10.0, oy, BODY_S, Role::OnSurfaceVariant, &line);
        oy += 24.0;
        for (k, item) in group {
            if !ui.shows(oy, 66.0) {
                oy += 78.0;
                continue;
            }
            let (edge, icon, ink) = match exposure {
                Exposure::Public => (
                    Role::OutlineVariant.paint(),
                    Icon::Export,
                    Role::Primary.paint(),
                ),
                Exposure::Sealed => (Role::Ok.with_alpha(70), Icon::Lock, Role::Ok.paint()),
                Exposure::Secret => (Role::Error.with_alpha(110), Icon::Lock, Role::Error.paint()),
            };
            ui.card(ox, oy, colw, 66.0, edge);
            ui.fill(ox + 16.0, oy + 14.0, 36.0, 36.0, 10.0, ink.with_alpha(26));
            ui.icon(ox + 16.0, oy + 14.0, 36.0, icon, 14.0, ink);
            let (line, state) = out_line(app, item, exposure);
            // No main action: the row's ⋮ alone, at its top right, holds
            // Show as QR or Make PDF, Rename, Details and Remove (decision
            // 61); the state stands left of it.
            let dots_x = ox + colw - 16.0 - 36.0;
            crate::file_menu::dots(
                app,
                ui,
                dots_x,
                oy + 14.0,
                36.0,
                crate::file_menu::Row::Out(k),
            );
            let state_w = ui.measure_as(BODY_S, state.0);
            ui.text_right_as((dots_x - 12.0, oy + 14.0, 20.0), BODY_S, state.1, state.0);
            let room = (dots_x - 12.0 - state_w - 12.0 - (ox + 66.0)).max(0.0);
            let name = ui.fit_as(BODY_M.mono(), &item.name, room);
            ui.text_as(ox + 66.0, oy + 14.0, BODY_M.mono(), Role::OnSurface, &name);
            let line = ui.fit_as(BODY_S, &line, (dots_x - 12.0 - (ox + 66.0)).max(0.0));
            ui.text_as(ox + 66.0, oy + 36.0, BODY_S, Role::OnSurfaceVariant, &line);
            oy += 78.0;
        }
        oy += 6.0;
    }
    if app.outbox.is_empty() {
        ui.stroke(ox, oy, colw, 72.0, 12.0, Role::OutlineVariant);
        let t = EN.files_empty;
        let tw = ui.measure_as(BODY_M, t);
        ui.text_mid_as(
            (ox + (colw - tw) / 2.0, oy, 72.0),
            BODY_S,
            Role::OnSurfaceVariant,
            t,
        );
        oy += 84.0;
    }
    if let Some(r) = app.visit.receipt().as_ref() {
        receipt_rows(ui, r, app.tz_min(), ox, oy, colw);
    }
    ui.c.pop_clip();
    ui.report_scroll(clip, content_in - view);
    ui.report_scroll_beside(
        crate::ui::Slot::FilesOut,
        clip_out,
        content_out - view,
        *app.visit.files_out_offset(),
    );
    // Foot.
    let fy = h - 32.0 - 48.0;
    if !app.outbox.is_empty() {
        ui.icon(
            x,
            fy + 14.0,
            20.0,
            Icon::Power,
            11.0,
            Role::OnSurfaceVariant,
        );
        ui.text_mid_as(
            (x + 26.0, fy, 48.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &fill(EN.files_power_off_asks, &[app.medium.for_the()]),
        );
    }
}

/// An Outbox file's line and state: what it is, or for a vault open in
/// this session (the file here is the copy sealed before it was opened,
/// and Lock seals it again) its changes; then ready, written after, or
/// unprotected, in its colour.
fn out_line(
    app: &View,
    item: &crate::Item,
    exposure: crate::secrets::Exposure,
) -> (String, (&'static str, Role)) {
    use crate::secrets::Exposure;
    let open_here = (item.kind == FileKind::Vault)
        .then(|| faraday_vault::read_header(&item.bytes).ok())
        .flatten()
        .and_then(|h| app.vaults.open.iter().find(|v| v.header().salt == h.salt));
    let line = match open_here {
        // With no seed to seal with, Lock does not seal it.
        Some(v) if v.changes > 0 => fill(
            if app.seeded() {
                EN.files_vault_open_unsaved
            } else {
                EN.vault_open_unsaved
            },
            &[
                &v.changes.to_string(),
                (if v.changes == 1 {
                    EN.files_change
                } else {
                    EN.files_changes
                }),
            ],
        ),
        Some(_) => EN.files_vault_open_sealed.to_string(),
        None => kind_line(item.kind, item.bytes.len()),
    };
    let state = match exposure {
        Exposure::Secret => (EN.files_unprotected, Role::Error),
        _ if open_here.is_some() => (EN.files_written_after, Role::Primary),
        _ => (EN.files_ready, Role::Ok),
    };
    (line, state)
}

/// The stick, or what waits for one: the line Files shows, and what a
/// press on it does.
fn stick_status(app: &View) -> (String, Option<Action>) {
    if app.stick.is_empty() {
        (fill(EN.files_waiting_medium, &[app.medium.a()]), None)
    } else if app.holds_secret() {
        (
            fill(EN.files_attached_lock, &[app.medium.cap()]),
            Some(Action::Lock),
        )
    } else {
        (
            fill(
                EN.files_visit,
                &[&app.stick[app.visit.stick.min(app.stick.len() - 1)]
                    .label
                    .to_string()],
            ),
            Some(Action::Nav(Screen::Visit)),
        )
    }
}

/// Files' stick status as a chip in the top strip's row, ending where
/// the row's mode pill and ? begin, or at `right` where none stands
/// (`docs/LAYOUT.md` §17).
fn stick_chip(app: &View, ui: &mut Ui, right: f32) {
    use faraday_draw::tokens::{GAP, STATUS_CHIP_H, STATUS_CHIP_PAD, TOP_ROW_MID};
    let (label, action) = stick_status(app);
    // A gap before the row, which covers from a gap left of it.
    let right = (ui.top_row_left - GAP).min(right);
    let dot = GAP;
    let bw = ui.measure_as(LABEL_L, &label) + 2.0 * STATUS_CHIP_PAD + dot + GAP;
    let (bx, by) = (right - bw, TOP_ROW_MID - STATUS_CHIP_H / 2.0);
    let r = STATUS_CHIP_H / 2.0;
    ui.fill(bx, by, bw, STATUS_CHIP_H, r, Role::Primary.with_alpha(16));
    ui.stroke(bx, by, bw, STATUS_CHIP_H, r, Role::Primary.with_alpha(110));
    let cx = bx + STATUS_CHIP_PAD + dot / 2.0;
    ui.dot(cx, TOP_ROW_MID, dot / 2.0, Role::Primary);
    let tx = bx + STATUS_CHIP_PAD + dot + GAP;
    let ink = Role::OnSurface;
    ui.text_mid_as((tx, by, STATUS_CHIP_H), LABEL_L, ink, &label);
    if let Some(a) = action {
        ui.hit(bx, by - GAP, bw, STATUS_CHIP_H + 2.0 * GAP, a);
    }
}

/// The last visit's receipt under For the stick: "Written to STICK at
/// 14:02", then a row per file, "verified" or why it was not. Nothing on
/// it is written again. Returns its height.
fn receipt_rows(ui: &mut Ui, r: &crate::Receipt, tz_min: i16, x: f32, y: f32, w: f32) -> f32 {
    let head = ui.fit_as(LABEL_M, &r.heading(tz_min), w);
    ui.text_as(x, y, LABEL_M, Role::Ok, &head);
    let mut ry = y + 30.0;
    for f in &r.files {
        let (state, tone) = match &f.failure {
            None => (EN.receipt_rows_verified.to_string(), Role::Ok),
            Some(why) => (why.clone(), Role::Error),
        };
        let state = ui.fit_as(BODY_S, &state, w / 2.0);
        let sw = ui.measure_as(BODY_S, &state);
        ui.text_right_as((x + w, ry - 4.0, 20.0), BODY_S, tone, &state);
        let name = ui.fit_as(BODY_M.mono(), &f.name, (w - sw - 12.0).max(0.0));
        ui.text_as(x, ry, BODY_M.mono(), Role::OnSurface, &name);
        ry += 28.0;
    }
    ry - y
}

/// An open vault's keys and wallets on Files, each chosen or not, with
/// loading every one or the chosen ones into Wallets. Returns its height.
/// An open vault's seeds and wallets, each ticked to load or not, with
/// Select all and a Load button that counts what it loads. What loaded
/// stays listed, marked Loaded.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn vault_panel(app: &View, ui: &mut Ui, v: usize, x: f32, y: f32, w: f32) -> f32 {
    if ui.compact {
        return vault_panel_compact(app, ui, v, x, y, w);
    }
    use crate::vault::VaultAction as V;
    let Some(open) = app.vaults.open.get(v) else {
        return 0.0;
    };
    let rows = app.vault_rows(v);
    let waiting: Vec<&crate::vault::VaultRow> = rows.iter().filter(|r| !r.loaded).collect();
    let per_col = rows.len().div_ceil(2).max(1);
    // Wallets this vault holds with seeds of theirs: one press loads each
    // with its seeds.
    let together: Vec<crate::vault::VaultWallet> = app
        .vault_wallets_with_keys(v)
        .into_iter()
        .filter(|w| !w.done)
        .collect();
    // Three rows at most: the rest are ticked in the full list below,
    // which shares the same choice.
    const ROWS: usize = 3;
    let more = together.len().saturating_sub(ROWS);
    let top = if together.is_empty() {
        0.0
    } else {
        30.0 + together.len().min(ROWS) as f32 * 40.0 + 64.0 + if more > 0 { 26.0 } else { 0.0 }
    };
    // With wallets and their seeds to load together, the list of each
    // seed and wallet folds away until asked for.
    let folded = !together.is_empty() && !open.each_shown;
    let h = if folded {
        52.0 + top + 8.0
    } else {
        132.0 + per_col as f32 * 38.0 + top
    };
    ui.card(x, y, w, h, Role::OutlineVariant);
    ui.icon(x + 20.0, y + 18.0, 20.0, Icon::Lock, 13.0, Role::Primary);
    ui.text_mid_as(
        (x + 48.0, y + 14.0, 28.0),
        TITLE_M,
        Role::OnSurface,
        &fill(
            EN.vault_panel_open,
            &[
                &open.name.to_string(),
                &open.label(app.vaults.store()).to_string(),
            ],
        ),
    );
    // Select all: every row not loaded yet, or none when all are chosen.
    if !waiting.is_empty() && !folded {
        let all = waiting.iter().all(|r| r.chosen);
        let label = EN.vault_panel_select;
        let lw = ui.measure_as(LABEL_L, label);
        let sx = x + w - 24.0 - lw - 28.0;
        ui.checkbox(sx, y + 19.0, all, true);
        ui.text_mid_as(
            (sx + 28.0, y + 14.0, 28.0),
            LABEL_L,
            Role::OnSurfaceVariant,
            label,
        );
        ui.hit(
            sx - 8.0,
            y + 8.0,
            lw + 44.0,
            40.0,
            Action::Vault(V::ChooseAll(v)),
        );
    }
    if !together.is_empty() {
        let mut ty = y + 52.0;
        ui.text_as(
            x + 20.0,
            ty,
            LABEL_L,
            Role::OnSurfaceVariant,
            EN.vault_panel_seeded_wallets,
        );
        ty += 26.0;
        let many = together.len() > 1;
        for wt in together.iter().take(ROWS) {
            let action = Action::Vault(V::Choose(v, wt.record));
            let mut rx = x + 20.0;
            if many {
                ui.checkbox(rx, ty + 9.0, wt.chosen, true);
                rx += 28.0;
            }
            ui.icon(
                rx,
                ty + 8.0,
                20.0,
                Icon::Wallet,
                11.0,
                Role::OnSurfaceVariant,
            );
            let nw = ui.text_mid_as((rx + 26.0, ty, 36.0), TITLE_S, Role::OnSurface, &wt.name);
            let here = wt.keys.len();
            let line = fill(
                EN.vault_panel_keys_here,
                &[
                    &here.to_string(),
                    &wt.total.to_string(),
                    (if wt.total == 1 {
                        EN.vault_panel_key
                    } else {
                        EN.vault_panel_keys
                    }),
                ],
            );
            ui.text_mid_as(
                (rx + 40.0 + nw, ty, 36.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &line,
            );
            if here >= wt.needed {
                let tag = EN.vault_panel_ready_to_sign;
                let tw = ui.measure_as(LABEL_M, tag) + 20.0;
                ui.fill(
                    x + w - 20.0 - tw,
                    ty + 6.0,
                    tw,
                    24.0,
                    12.0,
                    Role::OkContainer,
                );
                ui.text_mid_as(
                    (x + w - 10.0 - tw, ty, 36.0),
                    LABEL_M,
                    Role::OnOkContainer,
                    tag,
                );
            }
            if many {
                ui.hit(x + 16.0, ty, w - 32.0, 36.0, action);
            }
            ty += 40.0;
        }
        if more > 0 {
            ui.text_mid_as(
                (x + 20.0, ty - 4.0, 26.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &fill(EN.vault_panel_more_listed, &[&more.to_string()]),
            );
            ty += 26.0;
        }
        let chosen = together.iter().filter(|w| w.chosen || !many).count();
        let label = match chosen {
            1 if !many => EN.vault_panel_load_wallet.to_string(),
            0 => EN.vault_panel_nothing.to_string(),
            1 => EN.vault_panel_load.to_string(),
            n => fill(EN.vault_panel_load_wallets, &[&n.to_string()]),
        };
        let lw = ui.button(
            x + 20.0,
            ty + 4.0,
            None,
            BUTTON_H,
            &label,
            Style::Primary.when(chosen > 0),
            Action::Vault(V::LoadWithKeys(v)),
        );
        let each = if open.each_shown {
            EN.vault_panel_hide_each.to_string()
        } else {
            fill(EN.vault_panel_each_seed, &[&rows.len().to_string()])
        };
        let ew = ui.button(
            x + 20.0 + lw + 12.0,
            ty + 4.0,
            None,
            BUTTON_H,
            &each,
            Style::Ghost,
            Action::Vault(V::EachShown(v)),
        );
        if folded && rows.iter().any(|r| r.loaded) && app.screen != Screen::Family {
            ui.button(
                x + 20.0 + lw + ew + 24.0,
                ty + 4.0,
                None,
                BUTTON_H,
                EN.vault_panel_go_to_wallets,
                Style::Secondary,
                Action::Nav(Screen::WalletsStart),
            );
        }
        if !folded {
            ui.rule(x + 20.0, ty + 56.0, w - 40.0, Role::OutlineVariant);
        }
    }
    if folded {
        return h;
    }
    let colw = (w - 60.0) / 2.0;
    for (k, row) in rows.iter().enumerate() {
        let rx = x + 20.0 + (k / per_col) as f32 * (colw + 20.0);
        let ry = y + 56.0 + top + (k % per_col) as f32 * 38.0;
        if row.loaded {
            ui.icon(rx - 2.0, ry + 6.0, 22.0, Icon::Done, 12.0, Role::Ok);
        } else {
            ui.checkbox(rx, ry + 8.0, row.chosen, true);
        }
        let icon = if row.seed { Icon::Keys } else { Icon::Wallet };
        ui.icon(
            rx + 26.0,
            ry + 7.0,
            20.0,
            icon,
            11.0,
            Role::OnSurfaceVariant,
        );
        let name = ui.fit_as(TITLE_S, &row.name, colw * 0.5 - 24.0);
        ui.text_mid_as((rx + 52.0, ry, 34.0), TITLE_S, Role::OnSurface, &name);
        let detail = if row.loaded {
            EN.vault_panel_loaded.to_string()
        } else {
            row.detail.clone()
        };
        let face = if row.seed && !row.loaded {
            BODY_S.mono()
        } else {
            BODY_S
        };
        let detail = ui.fit_as(face, &detail, colw * 0.45);
        ui.text_right_as(
            (rx + colw, ry, 34.0),
            face,
            if row.loaded {
                Role::Ok
            } else {
                Role::OnSurfaceVariant
            },
            &detail,
        );
        if !row.loaded {
            ui.hit(rx, ry, colw, 34.0, Action::Vault(V::Choose(v, row.record)));
        }
    }
    let by = y + h - 20.0 - 40.0;
    let seeds = waiting.iter().filter(|r| r.chosen && r.seed).count();
    let wallets = waiting.iter().filter(|r| r.chosen && !r.seed).count();
    let count =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let label = match (seeds, wallets) {
        (0, 0) if waiting.is_empty() => EN.vault_panel_everything.to_string(),
        (0, 0) => EN.vault_panel_nothing.to_string(),
        (s, 0) => fill(
            EN.vault_panel_load_n,
            &[&count(s, EN.vault_panel_key, EN.vault_panel_keys).to_string()],
        ),
        (0, wl) => fill(
            EN.vault_panel_load_n,
            &[&count(wl, EN.vault_panel_wallet, EN.vault_panel_wallets).to_string()],
        ),
        (s, wl) => fill(
            EN.vault_panel_load_keys_wallets,
            &[
                &count(s, EN.vault_panel_key, EN.vault_panel_keys).to_string(),
                &count(wl, EN.vault_panel_wallet, EN.vault_panel_wallets).to_string(),
            ],
        ),
    };
    let bw = ui.button(
        x + 20.0,
        by,
        None,
        BUTTON_H,
        &label,
        Style::Primary.when(seeds + wallets > 0),
        Action::Vault(V::LoadChosen(v)),
    );
    // Once something has loaded, the way on: Files stays put.
    if rows.iter().any(|r| r.loaded) && app.screen != Screen::Family {
        ui.button(
            x + 20.0 + bw + 12.0,
            by,
            None,
            BUTTON_H,
            EN.vault_panel_go_to_wallets,
            Style::Secondary,
            Action::Nav(Screen::WalletsStart),
        );
    }
    h
}

/// [`inbox_panel`] on a small panel: one column, each row's line under
/// its name and its button under that, measured before its card is drawn.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn inbox_panel_compact(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::compact::screen::measured;
    let found = app.inbox_found();
    let wallets = found.to_load();
    let potential: Vec<&crate::inbox::FoundSeed> = found.potential();
    if wallets.is_empty()
        && potential.is_empty()
        && found.waiting.is_empty()
        && found.xpubs.is_empty()
    {
        return 0.0;
    }
    const ROWS: usize = 3;
    let may = app.may_load_keys();
    let (ix, iw) = (x + 12.0, w - 24.0);
    let draw = |ui: &mut Ui| -> f32 {
        let mut ty = y + 14.0;
        ui.icon(ix, ty, 20.0, Icon::Download, 13.0, Role::Primary);
        ui.text_mid_as(
            (ix + 28.0, ty - 4.0, 28.0),
            TITLE_M,
            Role::OnSurface,
            app.medium.from_box(),
        );
        ty += 36.0;
        if !wallets.is_empty() {
            ui.text_as(ix, ty, LABEL_L, Role::OnSurfaceVariant, EN.inbox_wallets);
            ty += 24.0;
            let many = wallets.len() > 1;
            for (i, fw) in wallets.iter().enumerate().take(ROWS) {
                let chosen = !app.inbox.skip().contains(&fw.descriptor);
                let mut rx = ix;
                let top = ty;
                if many {
                    ui.checkbox(rx, ty + 4.0, chosen, true);
                    rx += 28.0;
                }
                ui.icon(
                    rx,
                    ty + 2.0,
                    20.0,
                    Icon::Wallet,
                    11.0,
                    Role::OnSurfaceVariant,
                );
                let name = ui.fit_as(TITLE_S, &fw.name, ix + iw - rx - 26.0);
                ui.text_as(rx + 26.0, ty + 2.0, TITLE_S, Role::OnSurface, &name);
                ty += 24.0;
                let n = fw.keys.len().max(1);
                let line = fill(
                    EN.inbox_of_here,
                    &[
                        &fw.shape.to_string(),
                        &fw.seeds_here.to_string(),
                        &n.to_string(),
                        (if n == 1 { EN.inbox_key } else { EN.inbox_keys }),
                        &(if fw.files.len() > 1 {
                            format!(
                                " {}",
                                fill(EN.inbox_panel_files_agree, &[&fw.files.len().to_string()])
                            )
                        } else if fw.from_shares {
                            format!(" {}", EN.inbox_panel_from_split_sheets)
                        } else {
                            String::new()
                        })
                        .to_string(),
                    ],
                );
                let line = ui.fit_as(BODY_S, &line, ix + iw - rx - 26.0);
                ui.text_as(rx + 26.0, ty, BODY_S, Role::OnSurfaceVariant, &line);
                ty += 20.0;
                if fw.seeds_here >= fw.needed {
                    ui.text_as(rx + 26.0, ty, LABEL_M, Role::Ok, EN.inbox_ready_to_sign);
                    ty += 20.0;
                }
                if many {
                    ui.hit(ix - 4.0, top, iw + 8.0, ty - top, Action::InboxChoose(i));
                }
                ty += 8.0;
            }
            if wallets.len() > ROWS {
                let more = fill(EN.inbox_more_loaded, &[&(wallets.len() - ROWS).to_string()]);
                ty += ui.wrap_as((ix, ty, iw), BODY_S, Role::OnSurfaceVariant, &more) + 8.0;
            }
            let chosen = wallets
                .iter()
                .filter(|w| !many || !app.inbox.skip().contains(&w.descriptor))
                .count();
            let with_seeds = wallets
                .iter()
                .filter(|w| !many || !app.inbox.skip().contains(&w.descriptor))
                .any(|w| w.seeds_here > 0);
            let label = match (chosen, with_seeds) {
                (0, _) => EN.inbox_nothing_chosen.to_string(),
                (1, true) => EN.inbox_load_wallet_seeds.to_string(),
                (1, false) => EN.inbox_load_the_wallet.to_string(),
                (n, true) => fill(EN.inbox_load_wallets_seeds, &[&n.to_string()]),
                (n, false) => fill(EN.inbox_load_wallets, &[&n.to_string()]),
            };
            let style = Style::Primary.when(chosen > 0);
            ty += wrap_buttons(
                ui,
                ix,
                ty,
                iw,
                BUTTON_H,
                &[(&label, style, Action::InboxLoad)],
            );
            if !may {
                ty += ui.wrap_as(
                    (ix, ty, iw),
                    BODY_S,
                    Role::Warning,
                    &fill(EN.inbox_keys_load_once, &[app.medium.noun()]),
                ) + 8.0;
            }
            ty += 6.0;
        }
        if !potential.is_empty() {
            ui.text_as(ix, ty, LABEL_L, Role::OnSurfaceVariant, EN.inbox_loose_keys);
            ty += 24.0;
            for s in potential.iter().take(ROWS) {
                ui.icon(ix, ty + 2.0, 20.0, Icon::Keys, 11.0, Role::OnSurfaceVariant);
                ui.text_as(
                    ix + 26.0,
                    ty + 2.0,
                    BODY_M.mono(),
                    Role::OnSurface,
                    &fp_text(s.fingerprint),
                );
                ty += 24.0;
                let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
                let line = ui.fit_as(BODY_S, &from.join(" · "), iw - 26.0);
                ui.text_as(ix + 26.0, ty, BODY_S, Role::OnSurfaceVariant, &line);
                ty += 22.0;
                let style = if may {
                    Style::Secondary
                } else {
                    Style::Disabled
                };
                ty += wrap_buttons(
                    ui,
                    ix + 26.0,
                    ty,
                    iw - 26.0,
                    BUTTON_H,
                    &[(
                        EN.inbox_make_it_a_wallet,
                        style,
                        Action::PotentialOpen(s.fingerprint.0),
                    )],
                );
            }
            if potential.len() > ROWS {
                let more = fill(EN.inbox_more, &[&(potential.len() - ROWS).to_string()]);
                ty += ui.wrap_as((ix, ty, iw), BODY_S, Role::OnSurfaceVariant, &more) + 8.0;
            }
            ty += 4.0;
        }
        if !found.xpubs.is_empty() {
            ui.text_as(
                ix,
                ty,
                LABEL_L,
                Role::OnSurfaceVariant,
                EN.inbox_loose_xpubs,
            );
            ty += 24.0;
            for (i, xp) in found.xpubs.iter().enumerate().take(ROWS) {
                ui.icon(
                    ix,
                    ty + 2.0,
                    20.0,
                    Icon::Wallet,
                    11.0,
                    Role::OnSurfaceVariant,
                );
                let fp = xp
                    .fingerprint
                    .map(fp_text)
                    .unwrap_or_else(|| EN.inbox_no_origin.to_string());
                ui.text_as(ix + 26.0, ty + 2.0, BODY_M.mono(), Role::OnSurface, &fp);
                ty += 24.0;
                let line = format!(
                    "{} · {}",
                    xp.path.clone().unwrap_or_default(),
                    xp.files
                        .first()
                        .and_then(|k| app.inbox.get(*k))
                        .map_or(String::new(), |it| it.name.clone())
                );
                let line = ui.fit_as(BODY_S, &line, iw - 26.0);
                ui.text_as(ix + 26.0, ty, BODY_S, Role::OnSurfaceVariant, &line);
                ty += 22.0;
                ty += wrap_buttons(
                    ui,
                    ix + 26.0,
                    ty,
                    iw - 26.0,
                    BUTTON_H,
                    &[(
                        EN.inbox_make_it_a_watch,
                        Style::Secondary,
                        Action::XpubOpen(i),
                    )],
                );
            }
            ty += 4.0;
        }
        if !found.waiting.is_empty() {
            ui.text_as(
                ix,
                ty,
                LABEL_L,
                Role::OnSurfaceVariant,
                EN.inbox_not_complete,
            );
            ty += 24.0;
            for q in found.waiting.iter().take(ROWS) {
                ty += ui.wrap_as((ix, ty, iw), BODY_S, Role::OnSurfaceVariant, &q.line) + 8.0;
            }
        }
        ty - y + 6.0
    };
    let h = measured(ui, |ui| draw(ui));
    ui.card(x, y, w, h, Role::OutlineVariant);
    draw(ui);
    h
}

/// [`vault_panel`] on a small panel: one column of the vault's wallets
/// and seeds, each name over what it is, measured before its card.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn vault_panel_compact(app: &View, ui: &mut Ui, v: usize, x: f32, y: f32, w: f32) -> f32 {
    use crate::compact::screen::measured;
    use crate::vault::VaultAction as V;
    let Some(open) = app.vaults.open.get(v) else {
        return 0.0;
    };
    let rows = app.vault_rows(v);
    let waiting: Vec<&crate::vault::VaultRow> = rows.iter().filter(|r| !r.loaded).collect();
    let together: Vec<crate::vault::VaultWallet> = app
        .vault_wallets_with_keys(v)
        .into_iter()
        .filter(|w| !w.done)
        .collect();
    const ROWS: usize = 3;
    let more = together.len().saturating_sub(ROWS);
    let folded = !together.is_empty() && !open.each_shown;
    let (ix, iw) = (x + 12.0, w - 24.0);
    let go = rows.iter().any(|r| r.loaded) && app.screen != Screen::Family;
    let draw = |ui: &mut Ui| -> f32 {
        let mut ty = y + 14.0;
        ui.icon(ix, ty, 20.0, Icon::Lock, 13.0, Role::Primary);
        let title = fill(
            EN.vault_open,
            &[
                &open.name.to_string(),
                &open.label(app.vaults.store()).to_string(),
            ],
        );
        let title = ui.fit_as(TITLE_M, &title, iw - 28.0);
        ui.text_mid_as(
            (ix + 28.0, ty - 4.0, 28.0),
            TITLE_M,
            Role::OnSurface,
            &title,
        );
        ty += 36.0;
        if !together.is_empty() {
            ui.text_as(
                ix,
                ty,
                LABEL_L,
                Role::OnSurfaceVariant,
                EN.vault_seeded_wallets,
            );
            ty += 24.0;
            let many = together.len() > 1;
            for wt in together.iter().take(ROWS) {
                let top = ty;
                let mut rx = ix;
                if many {
                    ui.checkbox(rx, ty + 4.0, wt.chosen, true);
                    rx += 28.0;
                }
                ui.icon(
                    rx,
                    ty + 2.0,
                    20.0,
                    Icon::Wallet,
                    11.0,
                    Role::OnSurfaceVariant,
                );
                let name = ui.fit_as(TITLE_S, &wt.name, ix + iw - rx - 26.0);
                ui.text_as(rx + 26.0, ty + 2.0, TITLE_S, Role::OnSurface, &name);
                ty += 24.0;
                let here = wt.keys.len();
                let line = fill(
                    EN.vault_of_in_this_vault,
                    &[
                        &here.to_string(),
                        &wt.total.to_string(),
                        (if wt.total == 1 {
                            EN.vault_key_inline
                        } else {
                            EN.vault_keys
                        }),
                    ],
                );
                ui.text_as(rx + 26.0, ty, BODY_S, Role::OnSurfaceVariant, &line);
                ty += 20.0;
                if here >= wt.needed {
                    ui.text_as(rx + 26.0, ty, LABEL_M, Role::Ok, EN.vault_ready_to_sign);
                    ty += 20.0;
                }
                if many {
                    ui.hit(
                        ix - 4.0,
                        top,
                        iw + 8.0,
                        ty - top,
                        Action::Vault(V::Choose(v, wt.record)),
                    );
                }
                ty += 8.0;
            }
            if more > 0 {
                let line = fill(EN.vault_more_listed_under, &[&more.to_string()]);
                ty += ui.wrap_as((ix, ty, iw), BODY_S, Role::OnSurfaceVariant, &line) + 8.0;
            }
            let chosen = together.iter().filter(|w| w.chosen || !many).count();
            let label = match chosen {
                1 if !many => EN.vault_load_wallet_keys.to_string(),
                0 => EN.vault_nothing_chosen.to_string(),
                1 => EN.vault_load_1_wallet.to_string(),
                n => fill(EN.vault_load_wallets, &[&n.to_string()]),
            };
            let each = if open.each_shown {
                EN.vault_hide_each_seed.to_string()
            } else {
                fill(EN.vault_each_seed_wallet, &[&rows.len().to_string()])
            };
            let mut items: Vec<(&str, Style, Action)> = vec![
                (
                    &label,
                    Style::Primary.when(chosen > 0),
                    Action::Vault(V::LoadWithKeys(v)),
                ),
                (&each, Style::Ghost, Action::Vault(V::EachShown(v))),
            ];
            if folded && go {
                items.push((
                    EN.vault_go_to_wallets,
                    Style::Secondary,
                    Action::Nav(Screen::WalletsStart),
                ));
            }
            ty += wrap_buttons(ui, ix, ty, iw, BUTTON_H, &items);
            if !folded {
                ui.rule(ix, ty, iw, Role::OutlineVariant);
                ty += 12.0;
            }
        }
        if folded {
            return ty - y + 4.0;
        }
        // Select all: every row not loaded yet, or none when all are chosen.
        if !waiting.is_empty() {
            let all = waiting.iter().all(|r| r.chosen);
            ui.checkbox(ix, ty + 4.0, all, true);
            ui.text_as(
                ix + 28.0,
                ty + 2.0,
                LABEL_L,
                Role::OnSurfaceVariant,
                EN.vault_select_all,
            );
            ui.hit(
                ix - 4.0,
                ty - 4.0,
                iw + 8.0,
                32.0,
                Action::Vault(V::ChooseAll(v)),
            );
            ty += 32.0;
        }
        for row in &rows {
            if row.loaded {
                ui.icon(ix - 2.0, ty + 2.0, 22.0, Icon::Done, 12.0, Role::Ok);
            } else {
                ui.checkbox(ix, ty + 4.0, row.chosen, true);
            }
            let icon = if row.seed { Icon::Keys } else { Icon::Wallet };
            ui.icon(
                ix + 28.0,
                ty + 2.0,
                20.0,
                icon,
                11.0,
                Role::OnSurfaceVariant,
            );
            let detail = if row.loaded {
                EN.vault_loaded.to_string()
            } else {
                row.detail.clone()
            };
            let face = if row.seed && !row.loaded {
                BODY_S.mono()
            } else {
                BODY_S
            };
            let dw = ui.measure_as(face, &detail).min(iw * 0.4);
            let detail = ui.fit_as(face, &detail, dw);
            ui.text_right_as(
                (ix + iw, ty, 26.0),
                face,
                if row.loaded {
                    Role::Ok
                } else {
                    Role::OnSurfaceVariant
                },
                &detail,
            );
            let name = ui.fit_as(TITLE_S, &row.name, iw - 54.0 - dw - 8.0);
            ui.text_as(ix + 54.0, ty + 3.0, TITLE_S, Role::OnSurface, &name);
            if !row.loaded {
                ui.hit(
                    ix - 4.0,
                    ty - 4.0,
                    iw + 8.0,
                    34.0,
                    Action::Vault(V::Choose(v, row.record)),
                );
            }
            ty += 34.0;
        }
        ty += 6.0;
        let seeds = waiting.iter().filter(|r| r.chosen && r.seed).count();
        let wallets = waiting.iter().filter(|r| r.chosen && !r.seed).count();
        let count =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        let label = match (seeds, wallets) {
            (0, 0) if waiting.is_empty() => EN.vault_everything_loaded.to_string(),
            (0, 0) => EN.vault_nothing_chosen.to_string(),
            (s, 0) => fill(
                EN.vault_load_n,
                &[&count(s, EN.vault_key_inline, EN.vault_keys).to_string()],
            ),
            (0, wl) => fill(
                EN.vault_load_n,
                &[&count(wl, EN.vault_wallet_inline, EN.vault_wallets).to_string()],
            ),
            (s, wl) => fill(
                EN.vault_load_keys_wallets,
                &[
                    &count(s, EN.vault_key_inline, EN.vault_keys).to_string(),
                    &count(wl, EN.vault_wallet_inline, EN.vault_wallets).to_string(),
                ],
            ),
        };
        let mut items: Vec<(&str, Style, Action)> = vec![(
            &label,
            Style::Primary.when(seeds + wallets > 0),
            Action::Vault(V::LoadChosen(v)),
        )];
        if go {
            items.push((
                EN.vault_go_to_wallets,
                Style::Secondary,
                Action::Nav(Screen::WalletsStart),
            ));
        }
        ty += wrap_buttons(ui, ix, ty, iw, BUTTON_H, &items);
        ty - y + 4.0
    };
    let h = measured(ui, |ui| draw(ui));
    ui.card(x, y, w, h, Role::OutlineVariant);
    draw(ui);
    h
}

/// Buttons by label, style and action.
type Buttons = Vec<(String, Style, Action)>;

/// A line and its colour.
type Note = Option<(String, crate::ui::Paint)>;

/// What an Inbox file offers on its card: the way it is used; the rest
/// is behind its ⋮ (decision 61). A split sheet that does not add up yet says how
/// many keys are in hand instead of offering a button.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn inbox_actions(app: &View, k: usize, item: &crate::Item) -> (Buttons, Note) {
    let mut out: Vec<(String, Style, Action)> = Vec::new();
    let mut note = None;
    // What goes into a vault shows only while one is open (F60); the
    // vault's own row unlocks it.
    let vault_open = app.vault_way(Screen::Files).is_none();
    let into_vault =
        |label: &str, style, action| vault_open.then(|| (label.to_string(), style, action));
    let add_to_vault = |style| {
        into_vault(
            EN.inbox_actions_add_to_vault,
            style,
            Action::Vault(crate::vault::VaultAction::AddFile(k)),
        )
    };
    let same_spend = app.wallets.spend().is_some_and(|s| {
        item.psbt()
            .is_some_and(|p| p.txid == s.spend.txid && item.name != s.spend.source)
    });
    match item.kind {
        FileKind::Psbt if same_spend => {
            out.push((
                EN.inbox_actions_add_signatures.into(),
                Style::Primary,
                Action::Collect(k),
            ));
        }
        FileKind::Entries => {
            let action = crate::vault::VaultAction::ImportEntries(k);
            out.extend(into_vault(
                EN.inbox_actions_import,
                Style::Primary,
                Action::Vault(action),
            ));
        }
        FileKind::Vault => {
            let files = app.vault_files();
            // The row's own vault, or no button.
            if let Some(at) = vault_at(&files, item) {
                let open = files[at].open.is_some();
                out.push((
                    if open {
                        EN.inbox_actions_contents
                    } else {
                        EN.inbox_actions_unlock
                    }
                    .into(),
                    if open {
                        Style::Secondary
                    } else {
                        Style::Primary
                    },
                    Action::Vault(crate::vault::VaultAction::Open(at)),
                ));
            }
        }
        FileKind::Psbt => {
            out.push((
                EN.inbox_actions_sign.into(),
                Style::Tonal,
                Action::StartSpend(k),
            ));
        }
        FileKind::Wallet | FileKind::SecretWallet => {
            out.push((
                EN.inbox_actions_load_wallet.into(),
                Style::Primary,
                Action::LoadWallet(k),
            ));
        }
        FileKind::Text => out.extend(add_to_vault(Style::Primary)),
        FileKind::Transaction => {
            out.push((
                EN.inbox_actions_decode.into(),
                Style::Primary,
                Action::DecodeInbox(k),
            ));
        }
        FileKind::Share => {
            let texts: Vec<String> = app
                .inbox
                .iter()
                .filter(|i| i.kind == FileKind::Share)
                .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
                .collect();
            match crate::restore::merge(&texts) {
                Ok(m) if m.whole.is_some() => out.push((
                    fill(EN.inbox_actions_restore, &[&texts.len().to_string()]),
                    Style::Primary,
                    Action::RestoreShares,
                )),
                Ok(m) => {
                    note = Some((
                        fill(
                            EN.inbox_actions_of_keys,
                            &[&m.have.len().to_string(), &m.n.to_string()],
                        ),
                        Role::Warning.paint(),
                    ))
                }
                Err(e) => note = Some((e, Role::Error.paint())),
            }
        }
        FileKind::Words => {
            out.push((
                EN.inbox_actions_load.into(),
                Style::Primary,
                Action::LoadKey(k),
            ));
        }
        FileKind::Message => {
            out.push((
                EN.inbox_actions_check.into(),
                Style::Primary,
                Action::CheckMessage(k),
            ));
        }
        FileKind::Sheet if app.online => {
            out.push((
                EN.inbox_actions_make.into(),
                Style::Primary,
                Action::PdfInbox(k),
            ));
        }
        _ => {}
    }
    (out, note)
}

/// One file's card on a small panel: its icon, name and line, its ⋮ at
/// the right (decision 61), and its buttons wrapped under them. Returns
/// its height.
#[allow(clippy::too_many_arguments)]
fn file_card(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    (icon, ink, edge): (Icon, crate::ui::Paint, crate::ui::Paint),
    name: &str,
    line: &str,
    row: crate::file_menu::Row,
    note: Option<&(String, crate::ui::Paint)>,
    buttons: &[(String, Style, Action)],
) -> f32 {
    let draw = |ui: &mut Ui| -> f32 {
        let mut cy = y + 12.0;
        ui.fill(x + 12.0, cy, 32.0, 32.0, 8.0, ink.with_alpha(26));
        ui.icon(x + 12.0, cy, 32.0, icon, 13.0, ink);
        let dw = 32.0;
        let room = w - 56.0 - 12.0 - dw - 8.0;
        let n = ui.fit_as(BODY_M.mono(), name, room);
        ui.text_as(x + 56.0, cy, BODY_M.mono(), Role::OnSurface, &n);
        crate::file_menu::dots(app, ui, x + w - 12.0 - dw, cy, dw, row);
        let l = ui.fit_as(BODY_S, line, room);
        ui.text_as(x + 56.0, cy + 20.0, BODY_S, Role::OnSurfaceVariant, &l);
        cy += 46.0;
        if let Some((t, c)) = note {
            cy += ui.wrap_as((x + 12.0, cy, w - 24.0), BODY_M, *c, t) + 8.0;
        }
        let items: Vec<(&str, Style, Action)> = buttons
            .iter()
            .map(|(l, s, a)| (l.as_str(), *s, *a))
            .collect();
        cy += wrap_buttons(ui, x + 12.0, cy, w - 24.0, BUTTON_H, &items);
        cy - y + 2.0
    };
    // Its height worked out from what it holds, so a card scrolled out of
    // sight is passed over, not drawn.
    let labels: Vec<&str> = buttons.iter().map(|(l, ..)| l.as_str()).collect();
    let note_h = note.map_or(0.0, |(t, _)| ui.wrap_height_as(BODY_S, t, w - 24.0) + 8.0);
    let h = 60.0 + note_h + faraday_ui::parts::wrap_buttons_height(ui, w - 24.0, BUTTON_H, &labels);
    if !ui.shows(y, h) {
        return h;
    }
    ui.card(x, y, w, h, edge);
    let drawn = draw(ui);
    debug_assert!(
        (drawn - h).abs() < 0.01,
        "a file card is {drawn} high, worked out {h}"
    );
    h
}

/// [`files`] on compact and medium windows: one scrolled column of what
/// is loaded from open vaults and the Inbox, then the Inbox, then the
/// Outbox by who may read it, then the stick. From expanded windows up,
/// where the area holds two list panes, the Inbox and the Outbox stand
/// side by side under the rest (`docs/LAYOUT.md` §17).
fn files_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact::screen::finish;
    use faraday_draw::tokens::{LIST_PANE, MARGIN_WIDE, SPACER};
    let lists = 2.0 * LIST_PANE + SPACER;
    let beside = crate::panes::screen::shown(app, Screen::Files) && lists + 2.0 * MARGIN_WIDE <= cw;
    let (x, w) = if beside {
        (x0 + MARGIN_WIDE, lists)
    } else {
        (x0 + M, cw - 2.0 * M)
    };
    let top = 12.0 - app.list_offset;
    let mut y = top;
    y += compact_head(app, ui, (x, y, w));
    if beside {
        let inn = compact_inbox(app, ui, (x, y, LIST_PANE));
        let out = compact_outbox(app, ui, (x + LIST_PANE + SPACER, y, LIST_PANE));
        y += inn.max(out);
    } else {
        y += compact_inbox(app, ui, (x, y, w));
        y += compact_outbox(app, ui, (x, y, w));
    }
    y += compact_tail(app, ui, (x, y, w));
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

/// Files' head on a small panel: the stick, Scan a QR code, what is kept
/// in memory, and what open vaults and the Inbox give. Returns its height.
fn compact_head(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    // The stick, or what waits for one.
    let (label, action) = stick_status(app);
    ui.fill(x, y, w, 44.0, 10.0, Role::Primary.with_alpha(16));
    ui.stroke(x, y, w, 44.0, 10.0, Role::Primary.with_alpha(110));
    ui.dot(x + 18.0, y + 22.0, 4.0, Role::Primary);
    let label = ui.fit_as(BODY_M, &label, w - 44.0);
    ui.text_mid_as((x + 32.0, y, 44.0), BODY_M, Role::OnSurface, &label);
    if let Some(a) = action {
        ui.hit(x, y, w, 44.0, a);
    }
    y += 52.0;
    ui.button(
        x,
        y,
        Some(w),
        BUTTON_H,
        EN.files_scan_a_qr_code,
        Style::Secondary,
        Action::Scan,
    );
    y += 48.0;
    ui.text_as(
        x,
        y,
        BODY_S,
        Role::OnSurfaceVariant,
        EN.files_in_memory_emptied,
    );
    y += 26.0;
    // Arrived from Import and load: pulling the stick is the next step.
    if !app.stick.is_empty() && !app.visit.load_after.is_empty() {
        y += crate::vault::screen::stick_banner(ui, x, y, w, &pull_line(app));
    }
    for v in 0..app.vaults.open.len() {
        y += vault_panel(app, ui, v, x, y, w) + 14.0;
    }
    let ip = inbox_panel(app, ui, x, y, w);
    if ip > 0.0 {
        y += ip + 14.0;
    }
    y - y0
}

/// From the stick on a small panel: its title and a card per file.
/// Returns its height.
fn compact_inbox(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    // Inbox.
    let lw = ui.text_as(x, y, TITLE_M, Role::OnSurfaceVariant, app.medium.from_box());
    ui.text_as(
        x + lw + 10.0,
        y + 2.0,
        BODY_S,
        Role::OnSurfaceVariant,
        &app.inbox.len().to_string(),
    );
    y += 28.0;
    for (k, item) in app.inbox.iter().enumerate() {
        let line = if item.kind == FileKind::Vault
            && vault_of_row(&app.vault_files(), item).is_some_and(|f| f.open.is_some())
        {
            kind_line(item.kind, item.bytes.len()).replace(EN.files_locked, EN.files_open)
        } else {
            kind_line(item.kind, item.bytes.len())
        };
        let (buttons, note) = inbox_actions(app, k, item);
        y += file_card(
            app,
            ui,
            x,
            y,
            w,
            (
                Icon::Download,
                Role::Primary.paint(),
                Role::OutlineVariant.paint(),
            ),
            &item.name,
            &line,
            crate::file_menu::Row::In(k),
            note.as_ref(),
            &buttons,
        ) + 10.0;
    }
    if app.inbox.is_empty() {
        y += ui.wrap_as(
            (x, y, w),
            BODY_S,
            Role::OnSurfaceVariant,
            &fill(EN.files_empty_files_are, &[app.medium.a()]),
        ) + 14.0;
    }
    y - y0
}

/// For the stick on a small panel, grouped by who may read it, and the
/// last visit's receipt. Returns its height.
fn compact_outbox(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    use crate::secrets::Exposure;
    let mut y = y0;
    // Outbox, grouped by who may read it.
    y += 8.0;
    let lw = ui.text_as(x, y, TITLE_M, Role::OnSurfaceVariant, app.medium.for_box());
    ui.text_as(
        x + lw + 10.0,
        y + 2.0,
        BODY_S,
        Role::OnSurfaceVariant,
        &app.outbox.len().to_string(),
    );
    y += 28.0;
    for (exposure, label, line, tone) in [
        (
            Exposure::Public,
            EN.files_public,
            EN.files_anyone_may_read.to_string(),
            Role::OnSurfaceVariant,
        ),
        (
            Exposure::Sealed,
            EN.files_written_sealed,
            fill(EN.files_sealed_under, &[app.medium.a()]),
            Role::Ok,
        ),
        (
            Exposure::Secret,
            EN.files_unprotected_secrets,
            fill(EN.files_anyone_who_has, &[app.medium.noun()]),
            Role::Error,
        ),
    ] {
        let group: Vec<(usize, &crate::Item)> = app
            .outbox
            .iter()
            .enumerate()
            .filter(|(_, i)| i.exposure() == exposure)
            .collect();
        if group.is_empty() {
            continue;
        }
        ui.text_as(x, y, LABEL_M, tone, label);
        y += 18.0;
        y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, &line) + 8.0;
        for (k, item) in group {
            let (edge, icon, ink) = match exposure {
                Exposure::Public => (
                    Role::OutlineVariant.paint(),
                    Icon::Export,
                    Role::Primary.paint(),
                ),
                Exposure::Sealed => (Role::Ok.with_alpha(70), Icon::Lock, Role::Ok.paint()),
                Exposure::Secret => (Role::Error.with_alpha(110), Icon::Lock, Role::Error.paint()),
            };
            let (line, state) = out_line(app, item, exposure);
            // No main action: the ⋮ alone (decision 61).
            // The state goes on the line, where the name leaves no room.
            let line = format!("{} · {line}", state.0);
            y += file_card(
                app,
                ui,
                x,
                y,
                w,
                (icon, ink, edge),
                &item.name,
                &line,
                crate::file_menu::Row::Out(k),
                None,
                &[],
            ) + 10.0;
        }
        y += 4.0;
    }
    if app.outbox.is_empty() {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.files_empty);
        y += 26.0;
    }
    if let Some(r) = app.visit.receipt().as_ref() {
        y += receipt_rows(ui, r, app.tz_min(), x, y, w) + 8.0;
    }
    y - y0
}

/// Files' foot on a small panel: what power-off asks, and what Files
/// holds, with its Learn. Returns its height.
fn compact_tail(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    use crate::compact::screen::about_learn;
    let mut y = y0;
    if !app.outbox.is_empty() {
        y += 6.0;
        ui.icon(x, y, 18.0, Icon::Power, 10.0, Role::OnSurfaceVariant);
        y += ui.wrap_as(
            (x + 24.0, y + 1.0, w - 24.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &fill(EN.files_power_off_asks, &[app.medium.for_the()]),
        ) + 10.0;
    }
    y += about_learn(
        ui,
        x,
        y,
        w,
        &fill(
            EN.files_holds_what_was,
            &[app.medium.from_box(), app.medium.for_box()],
        ),
        Some(crate::learn::Topic::AirGap),
    );
    y - y0
}

/// The place among `files` ([`crate::Faraday::vault_files`]) of the vault
/// From the stick row `item` is: its own file by name in that list, or
/// the vault it is a copy of ([`crate::vault::VaultFile::copies`]), never
/// a For the stick file of its name that is another vault; `None` when it
/// is neither. A lookup.
fn vault_at(files: &[crate::vault::VaultFile], item: &crate::Item) -> Option<usize> {
    files
        .iter()
        .position(|f| (f.name == item.name && !f.in_outbox) || f.copies.contains(&item.name))
}

/// [`vault_at`]'s vault file.
fn vault_of_row<'a>(
    files: &'a [crate::vault::VaultFile],
    item: &crate::Item,
) -> Option<&'a crate::vault::VaultFile> {
    vault_at(files, item).map(|i| &files[i])
}
