//! `Screen::Files`: Inbox and Outbox.

use super::parts::{guide_text_learn, kind_line, pull_line};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::{FileKind, fp_text};
use crate::{Action, Screen};
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
    ui.card(x, y, w, h, LINE);
    ui.icon(x + 20.0, y + 18.0, 20.0, Icon::Download, 13.0, ACCENT);
    ui.text_mid(
        x + 48.0,
        y + 14.0,
        28.0,
        15.0,
        W::S,
        TEXT,
        app.medium.from_box(),
    );
    let mut ty = y + 52.0;
    let may = app.may_load_keys();
    if !wallets.is_empty() {
        ui.text(x + 20.0, ty, 13.0, W::S, MUTED, EN.inbox_panel_wallets);
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
            ui.icon(rx, ty + 7.0, 20.0, Icon::Wallet, 11.0, MUTED);
            let nw = ui.text_mid(rx + 26.0, ty, 34.0, 14.0, W::S, TEXT, &fw.name);
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
            let line = ui.fit(12.0, W::R, &line, w - nw - 260.0);
            ui.text_mid(rx + 40.0 + nw, ty, 34.0, 12.0, W::R, MUTED, &line);
            if fw.seeds_here >= fw.needed {
                let tag = EN.inbox_panel_ready_to_sign;
                let tw = ui.measure(12.0, W::S, tag) + 20.0;
                ui.fill(
                    x + w - 20.0 - tw,
                    ty + 5.0,
                    tw,
                    24.0,
                    12.0,
                    OK.with_alpha(30),
                );
                ui.text_mid(x + w - 10.0 - tw, ty, 34.0, 12.0, W::S, OK, tag);
            }
            ty += 38.0;
        }
        if wallets.len() > ROWS {
            ui.text_mid(
                x + 20.0,
                ty - 6.0,
                22.0,
                12.0,
                W::R,
                MUTED,
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
            40.0,
            &label,
            Style::Primary.when(chosen > 0),
            Action::InboxLoad,
        );
        if !may {
            ui.text_mid(
                x + 20.0 + ui.measure(14.0, W::S, &label) + 52.0,
                ty + 4.0,
                40.0,
                12.0,
                W::R,
                WARN,
                &fill(EN.inbox_panel_keys_load, &[app.medium.noun()]),
            );
        }
        ty += 54.0;
    }
    if !potential.is_empty() {
        ui.text(x + 20.0, ty, 13.0, W::S, MUTED, EN.inbox_panel_loose_keys);
        ty += 26.0;
        for s in potential.iter().take(ROWS) {
            ui.icon(x + 20.0, ty + 9.0, 20.0, Icon::Keys, 11.0, MUTED);
            ui.text_mid(
                x + 46.0,
                ty,
                38.0,
                14.0,
                W::M,
                TEXT,
                &fp_text(s.fingerprint),
            );
            let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
            let line = ui.fit(12.0, W::R, &from.join(" · "), w - 420.0);
            ui.text_mid(x + 140.0, ty, 38.0, 12.0, W::R, MUTED, &line);
            let label = EN.inbox_panel_make_wallet;
            let bw = ui.measure(13.0, W::S, label) + 28.0;
            ui.button(
                x + w - 20.0 - bw,
                ty + 3.0,
                Some(bw),
                32.0,
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
            ui.text_mid(
                x + 20.0,
                ty - 6.0,
                22.0,
                12.0,
                W::R,
                MUTED,
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
        ui.text(x + 20.0, ty, 13.0, W::S, MUTED, EN.inbox_panel_loose_xpubs);
        ty += 26.0;
        for (i, xp) in found.xpubs.iter().enumerate().take(ROWS) {
            ui.icon(x + 20.0, ty + 9.0, 20.0, Icon::Wallet, 11.0, MUTED);
            let fp = xp
                .fingerprint
                .map(fp_text)
                .unwrap_or_else(|| EN.inbox_panel_no_origin.to_string());
            ui.text_mid(x + 46.0, ty, 38.0, 14.0, W::M, TEXT, &fp);
            let line = format!(
                "{} · {}",
                xp.path.clone().unwrap_or_default(),
                xp.files
                    .first()
                    .and_then(|k| app.inbox.get(*k))
                    .map_or(String::new(), |it| it.name.clone())
            );
            let line = ui.fit(12.0, W::R, &line, w - 460.0);
            ui.text_mid(x + 140.0, ty, 38.0, 12.0, W::R, MUTED, &line);
            let label = EN.inbox_panel_make_watch_only;
            let bw = ui.measure(13.0, W::S, label) + 28.0;
            ui.button(
                x + w - 20.0 - bw,
                ty + 3.0,
                Some(bw),
                32.0,
                label,
                Style::Secondary,
                Action::XpubOpen(i),
            );
            ty += 42.0;
        }
        ty += 10.0;
    }
    if !found.waiting.is_empty() {
        ui.text(x + 20.0, ty, 13.0, W::S, MUTED, EN.inbox_panel_not_complete);
        ty += 26.0;
        for q in found.waiting.iter().take(ROWS) {
            let line = ui.fit(13.0, W::R, &q.line, w - 200.0);
            ui.text_mid(x + 20.0, ty, 36.0, 13.0, W::R, MUTED, &line);
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
    ui.button(x, y, None, 36.0, EN.add_to_vault_button, style, action) + 8.0
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
                36.0,
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
            ui.text_mid(x, y, 36.0, 13.0, W::R, WARN, &line);
            ui.measure(13.0, W::R, &line) + 16.0
        }
        Err(e) => {
            ui.text_mid(x, y, 36.0, 13.0, W::R, ERR, &e);
            ui.measure(13.0, W::R, &e) + 16.0
        }
    }
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return files_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 48.0;
    let width = cw - 96.0;
    let mut y = 36.0;
    title(ui, x, y, EN.files_files);
    let label = EN.files_in_memory_emptied;
    let pw = ui.measure(12.0, W::R, label) + 24.0;
    ui.pill(x + width - pw, y + 2.0, 30.0, label);
    let sw = ui.measure(13.0, W::S, EN.files_scan_a_qr_code) + 32.0;
    ui.button(
        x + width - pw - 12.0 - sw,
        y,
        Some(sw),
        36.0,
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
        width.min(900.0),
        &fill(
            EN.files_holds_what_was,
            &[app.medium.from_box(), app.medium.for_box()],
        ),
        crate::learn::Topic::AirGap,
    );
    // Arrived from Import and load: pulling the stick is the next step.
    if !app.stick.is_empty() && !app.visit.load_after.is_empty() {
        ui.fill(x, y, width, 48.0, 10.0, WARN.with_alpha(22));
        ui.stroke(x, y, width, 48.0, 10.0, WARN.with_alpha(90));
        ui.icon(x + 16.0, y + 14.0, 20.0, app.medium.icon(), 13.0, WARN);
        ui.text_mid(x + 44.0, y, 48.0, 14.0, W::S, WARN, &pull_line(app));
        y += 60.0;
    }
    for v in 0..app.vaults.open.len() {
        y += vault_panel(app, ui, v, x, y, width) + 20.0;
    }
    let ip = inbox_panel(app, ui, x, y, width);
    if ip > 0.0 {
        y += ip + 20.0;
    }
    let colw = (width - 20.0) / 2.0;
    // Inbox.
    let lw = ui.text(x, y, 15.0, W::S, MUTED, app.medium.from_box());
    ui.text(
        x + lw + 10.0,
        y + 2.0,
        13.0,
        W::R,
        DIM,
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
    let split = x + colw + 10.0;
    let clip = ui.rect(x0, top, split - x0, view);
    let clip_out = ui.rect(split, top, x0 + cw - split, view);
    ui.c.push_clip(clip);
    let mut iy = y + 32.0 - shift;
    for (k, item) in app.inbox.iter().enumerate() {
        // A row scrolled out of the list's sight is passed over.
        if !ui.shows(iy, 112.0) {
            iy += 124.0;
            continue;
        }
        ui.card(x, iy, colw, 112.0, LINE);
        ui.fill(x + 16.0, iy + 14.0, 36.0, 36.0, 10.0, ACCENT.with_alpha(26));
        ui.icon(x + 16.0, iy + 14.0, 36.0, Icon::Download, 14.0, ACCENT);
        let name = ui.fit(14.0, W::M, &item.name, colw - 90.0);
        ui.text(x + 66.0, iy + 14.0, 14.0, W::M, TEXT, &name);
        let line = if item.kind == FileKind::Vault
            && app
                .vault_files()
                .iter()
                .any(|f| f.name == item.name && f.open.is_some())
        {
            kind_line(item.kind, item.bytes.len()).replace(EN.files_locked, EN.files_open)
        } else {
            kind_line(item.kind, item.bytes.len())
        };
        ui.text(x + 66.0, iy + 36.0, 12.0, W::R, MUTED, &line);
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
                    36.0,
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
                        36.0,
                        EN.files_import_into_vault,
                        Style::Primary,
                        Action::Vault(action),
                    ) + 8.0;
                }
            }
            FileKind::Vault => {
                let files = app.vault_files();
                let at = files.iter().position(|f| f.name == item.name).unwrap_or(0);
                let open = files.get(at).is_some_and(|f| f.open.is_some());
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
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
            #[cfg(feature = "unreleased")]
            FileKind::Carry => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    EN.files_sign,
                    Style::Primary,
                    Action::StartSpend(k),
                ) + 8.0;
            }
            FileKind::Psbt => {
                let style = if app.session.wallets.is_empty() && app.session.keys.is_empty() {
                    Style::Secondary
                } else {
                    Style::Primary
                };
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    EN.files_sign,
                    style,
                    Action::StartSpend(k),
                ) + 8.0;
            }
            FileKind::Wallet | FileKind::SecretWallet => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
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
                    36.0,
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
                    36.0,
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
                    36.0,
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
                    36.0,
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
        ui.stroke(x, iy, colw, 72.0, 12.0, BORDER);
        let t = &fill(EN.files_empty_files_are, &[app.medium.a()]);
        let tw = ui.measure(13.0, W::R, t);
        ui.text_mid(x + (colw - tw) / 2.0, iy, 72.0, 13.0, W::R, DIM, t);
    }
    // Outbox, its title above the scrolled lists.
    let ox = x + colw + 20.0;
    ui.c.pop_clip();
    let lw = ui.text(ox, y, 15.0, W::S, MUTED, app.medium.for_box());
    ui.text(
        ox + lw + 10.0,
        y + 2.0,
        13.0,
        W::R,
        DIM,
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
            MUTED,
        ),
        (
            Exposure::Sealed,
            EN.files_written_sealed,
            fill(EN.files_sealed_under, &[app.medium.a()]),
            OK,
        ),
        (
            Exposure::Secret,
            EN.files_unprotected_secrets,
            fill(EN.files_anyone_who_has, &[app.medium.noun()]),
            ERR,
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
        let lw = ui.text(ox, oy, 12.0, W::S, tone, label);
        ui.text(ox + lw + 10.0, oy, 12.0, W::R, DIM, &line);
        oy += 24.0;
        for (k, item) in group {
            if !ui.shows(oy, 66.0) {
                oy += 78.0;
                continue;
            }
            let (edge, icon, ink) = match exposure {
                Exposure::Public => (LINE, Icon::Export, ACCENT),
                Exposure::Sealed => (OK.with_alpha(70), Icon::Lock, OK),
                Exposure::Secret => (ERR.with_alpha(110), Icon::Lock, ERR),
            };
            ui.card(ox, oy, colw, 66.0, edge);
            ui.fill(ox + 16.0, oy + 14.0, 36.0, 36.0, 10.0, ink.with_alpha(26));
            ui.icon(ox + 16.0, oy + 14.0, 36.0, icon, 14.0, ink);
            // A vault open in this session: the file here is the copy
            // sealed before it was opened, and Lock seals it again.
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
                Exposure::Secret => (EN.files_unprotected, ERR),
                _ if open_here.is_some() => (EN.files_written_after, ACCENT),
                _ => (EN.files_ready, OK),
            };
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
            let state_w = ui.measure(12.0, W::R, state.0);
            ui.text_right(dots_x - 12.0, oy + 14.0, 20.0, 12.0, W::R, state.1, state.0);
            let room = (dots_x - 12.0 - state_w - 12.0 - (ox + 66.0)).max(0.0);
            let name = ui.fit(14.0, W::M, &item.name, room);
            ui.text(ox + 66.0, oy + 14.0, 14.0, W::M, TEXT, &name);
            let line = ui.fit(12.0, W::R, &line, (dots_x - 12.0 - (ox + 66.0)).max(0.0));
            ui.text(ox + 66.0, oy + 36.0, 12.0, W::R, MUTED, &line);
            oy += 78.0;
        }
        oy += 6.0;
    }
    if app.outbox.is_empty() {
        ui.stroke(ox, oy, colw, 72.0, 12.0, BORDER);
        let t = EN.files_empty;
        let tw = ui.measure(13.0, W::R, t);
        ui.text_mid(ox + (colw - tw) / 2.0, oy, 72.0, 13.0, W::R, DIM, t);
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
        ui.icon(x, fy + 14.0, 20.0, Icon::Power, 11.0, MUTED);
        ui.text_mid(
            x + 26.0,
            fy,
            48.0,
            12.0,
            W::R,
            MUTED,
            &fill(EN.files_power_off_asks, &[app.medium.for_the()]),
        );
    }
    let (label, action) = if app.stick.is_empty() {
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
    };
    let bw = ui.measure(14.0, W::R, &label) + 52.0;
    let bx = x + width - bw;
    ui.fill(bx, fy, bw, 48.0, 12.0, ACCENT.with_alpha(16));
    ui.stroke(bx, fy, bw, 48.0, 12.0, ACCENT.with_alpha(110));
    ui.dot(bx + 22.0, fy + 24.0, 4.0, ACCENT);
    ui.text_mid(bx + 36.0, fy, 48.0, 14.0, W::R, TEXT, &label);
    if let Some(a) = action {
        ui.hit(bx, fy, bw, 48.0, a);
    }
}

/// The last visit's receipt under For the stick: "Written to STICK at
/// 14:02", then a row per file, "verified" or why it was not. Nothing on
/// it is written again. Returns its height.
fn receipt_rows(ui: &mut Ui, r: &crate::Receipt, tz_min: i16, x: f32, y: f32, w: f32) -> f32 {
    let head = ui.fit(12.0, W::S, &r.heading(tz_min), w);
    ui.text(x, y, 12.0, W::S, OK, &head);
    let mut ry = y + 30.0;
    for f in &r.files {
        let (state, tone) = match &f.failure {
            None => (EN.receipt_rows_verified.to_string(), OK),
            Some(why) => (why.clone(), ERR),
        };
        let state = ui.fit(12.0, W::R, &state, w / 2.0);
        let sw = ui.measure(12.0, W::R, &state);
        ui.text_right(x + w, ry - 4.0, 20.0, 12.0, W::R, tone, &state);
        let name = ui.fit(13.0, W::M, &f.name, (w - sw - 12.0).max(0.0));
        ui.text(x, ry, 13.0, W::M, TEXT, &name);
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
    ui.card(x, y, w, h, LINE);
    ui.icon(x + 20.0, y + 18.0, 20.0, Icon::Lock, 13.0, ACCENT);
    ui.text_mid(
        x + 48.0,
        y + 14.0,
        28.0,
        15.0,
        W::S,
        TEXT,
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
        let lw = ui.measure(13.0, W::S, label);
        let sx = x + w - 24.0 - lw - 28.0;
        ui.checkbox(sx, y + 19.0, all, true);
        ui.text_mid(sx + 28.0, y + 14.0, 28.0, 13.0, W::S, MUTED, label);
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
        ui.text(
            x + 20.0,
            ty,
            13.0,
            W::S,
            MUTED,
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
            ui.icon(rx, ty + 8.0, 20.0, Icon::Wallet, 11.0, MUTED);
            let nw = ui.text_mid(rx + 26.0, ty, 36.0, 14.0, W::S, TEXT, &wt.name);
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
            ui.text_mid(rx + 40.0 + nw, ty, 36.0, 12.0, W::R, MUTED, &line);
            if here >= wt.needed {
                let tag = EN.vault_panel_ready_to_sign;
                let tw = ui.measure(12.0, W::S, tag) + 20.0;
                ui.fill(
                    x + w - 20.0 - tw,
                    ty + 6.0,
                    tw,
                    24.0,
                    12.0,
                    OK.with_alpha(30),
                );
                ui.text_mid(x + w - 10.0 - tw, ty, 36.0, 12.0, W::S, OK, tag);
            }
            if many {
                ui.hit(x + 16.0, ty, w - 32.0, 36.0, action);
            }
            ty += 40.0;
        }
        if more > 0 {
            ui.text_mid(
                x + 20.0,
                ty - 4.0,
                26.0,
                12.0,
                W::R,
                MUTED,
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
            40.0,
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
            40.0,
            &each,
            Style::Ghost,
            Action::Vault(V::EachShown(v)),
        );
        if folded && rows.iter().any(|r| r.loaded) && app.screen != Screen::Family {
            ui.button(
                x + 20.0 + lw + ew + 24.0,
                ty + 4.0,
                None,
                40.0,
                EN.vault_panel_go_to_wallets,
                Style::Secondary,
                Action::Nav(Screen::WalletsStart),
            );
        }
        if !folded {
            ui.rule(x + 20.0, ty + 56.0, w - 40.0, INNER);
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
            ui.icon(rx - 2.0, ry + 6.0, 22.0, Icon::Done, 12.0, OK);
        } else {
            ui.checkbox(rx, ry + 8.0, row.chosen, true);
        }
        let icon = if row.seed { Icon::Keys } else { Icon::Wallet };
        ui.icon(rx + 26.0, ry + 7.0, 20.0, icon, 11.0, MUTED);
        let name = ui.fit(14.0, W::S, &row.name, colw * 0.5 - 24.0);
        ui.text_mid(rx + 52.0, ry, 34.0, 14.0, W::S, TEXT, &name);
        let detail = if row.loaded {
            EN.vault_panel_loaded.to_string()
        } else {
            row.detail.clone()
        };
        let face = if row.seed && !row.loaded { W::M } else { W::R };
        let detail = ui.fit(12.0, face, &detail, colw * 0.45);
        ui.text_right(
            rx + colw,
            ry,
            34.0,
            12.0,
            face,
            if row.loaded { OK } else { MUTED },
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
        40.0,
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
            40.0,
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
        ui.icon(ix, ty, 20.0, Icon::Download, 13.0, ACCENT);
        ui.text_mid(
            ix + 28.0,
            ty - 4.0,
            28.0,
            15.0,
            W::S,
            TEXT,
            app.medium.from_box(),
        );
        ty += 36.0;
        if !wallets.is_empty() {
            ui.text(ix, ty, 13.0, W::S, MUTED, EN.inbox_wallets);
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
                ui.icon(rx, ty + 2.0, 20.0, Icon::Wallet, 11.0, MUTED);
                let name = ui.fit(14.0, W::S, &fw.name, ix + iw - rx - 26.0);
                ui.text(rx + 26.0, ty + 2.0, 14.0, W::S, TEXT, &name);
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
                let line = ui.fit(12.0, W::R, &line, ix + iw - rx - 26.0);
                ui.text(rx + 26.0, ty, 12.0, W::R, MUTED, &line);
                ty += 20.0;
                if fw.seeds_here >= fw.needed {
                    ui.text(rx + 26.0, ty, 12.0, W::S, OK, EN.inbox_ready_to_sign);
                    ty += 20.0;
                }
                if many {
                    ui.hit(ix - 4.0, top, iw + 8.0, ty - top, Action::InboxChoose(i));
                }
                ty += 8.0;
            }
            if wallets.len() > ROWS {
                let more = fill(EN.inbox_more_loaded, &[&(wallets.len() - ROWS).to_string()]);
                ty += ui.wrap(ix, ty, iw, 12.0, W::R, MUTED, &more) + 8.0;
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
            ty += wrap_buttons(ui, ix, ty, iw, 40.0, &[(&label, style, Action::InboxLoad)]);
            if !may {
                ty += ui.wrap(
                    ix,
                    ty,
                    iw,
                    12.0,
                    W::R,
                    WARN,
                    &fill(EN.inbox_keys_load_once, &[app.medium.noun()]),
                ) + 8.0;
            }
            ty += 6.0;
        }
        if !potential.is_empty() {
            ui.text(ix, ty, 13.0, W::S, MUTED, EN.inbox_loose_keys);
            ty += 24.0;
            for s in potential.iter().take(ROWS) {
                ui.icon(ix, ty + 2.0, 20.0, Icon::Keys, 11.0, MUTED);
                ui.text(
                    ix + 26.0,
                    ty + 2.0,
                    14.0,
                    W::M,
                    TEXT,
                    &fp_text(s.fingerprint),
                );
                ty += 24.0;
                let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
                let line = ui.fit(12.0, W::R, &from.join(" · "), iw - 26.0);
                ui.text(ix + 26.0, ty, 12.0, W::R, MUTED, &line);
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
                    32.0,
                    &[(
                        EN.inbox_make_it_a_wallet,
                        style,
                        Action::PotentialOpen(s.fingerprint.0),
                    )],
                );
            }
            if potential.len() > ROWS {
                let more = fill(EN.inbox_more, &[&(potential.len() - ROWS).to_string()]);
                ty += ui.wrap(ix, ty, iw, 12.0, W::R, MUTED, &more) + 8.0;
            }
            ty += 4.0;
        }
        if !found.xpubs.is_empty() {
            ui.text(ix, ty, 13.0, W::S, MUTED, EN.inbox_loose_xpubs);
            ty += 24.0;
            for (i, xp) in found.xpubs.iter().enumerate().take(ROWS) {
                ui.icon(ix, ty + 2.0, 20.0, Icon::Wallet, 11.0, MUTED);
                let fp = xp
                    .fingerprint
                    .map(fp_text)
                    .unwrap_or_else(|| EN.inbox_no_origin.to_string());
                ui.text(ix + 26.0, ty + 2.0, 14.0, W::M, TEXT, &fp);
                ty += 24.0;
                let line = format!(
                    "{} · {}",
                    xp.path.clone().unwrap_or_default(),
                    xp.files
                        .first()
                        .and_then(|k| app.inbox.get(*k))
                        .map_or(String::new(), |it| it.name.clone())
                );
                let line = ui.fit(12.0, W::R, &line, iw - 26.0);
                ui.text(ix + 26.0, ty, 12.0, W::R, MUTED, &line);
                ty += 22.0;
                ty += wrap_buttons(
                    ui,
                    ix + 26.0,
                    ty,
                    iw - 26.0,
                    32.0,
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
            ui.text(ix, ty, 13.0, W::S, MUTED, EN.inbox_not_complete);
            ty += 24.0;
            for q in found.waiting.iter().take(ROWS) {
                ty += ui.wrap(ix, ty, iw, 13.0, W::R, MUTED, &q.line) + 8.0;
            }
        }
        ty - y + 6.0
    };
    let h = measured(ui, |ui| draw(ui));
    ui.card(x, y, w, h, LINE);
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
        ui.icon(ix, ty, 20.0, Icon::Lock, 13.0, ACCENT);
        let title = fill(
            EN.vault_open,
            &[
                &open.name.to_string(),
                &open.label(app.vaults.store()).to_string(),
            ],
        );
        let title = ui.fit(15.0, W::S, &title, iw - 28.0);
        ui.text_mid(ix + 28.0, ty - 4.0, 28.0, 15.0, W::S, TEXT, &title);
        ty += 36.0;
        if !together.is_empty() {
            ui.text(ix, ty, 13.0, W::S, MUTED, EN.vault_seeded_wallets);
            ty += 24.0;
            let many = together.len() > 1;
            for wt in together.iter().take(ROWS) {
                let top = ty;
                let mut rx = ix;
                if many {
                    ui.checkbox(rx, ty + 4.0, wt.chosen, true);
                    rx += 28.0;
                }
                ui.icon(rx, ty + 2.0, 20.0, Icon::Wallet, 11.0, MUTED);
                let name = ui.fit(14.0, W::S, &wt.name, ix + iw - rx - 26.0);
                ui.text(rx + 26.0, ty + 2.0, 14.0, W::S, TEXT, &name);
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
                ui.text(rx + 26.0, ty, 12.0, W::R, MUTED, &line);
                ty += 20.0;
                if here >= wt.needed {
                    ui.text(rx + 26.0, ty, 12.0, W::S, OK, EN.vault_ready_to_sign);
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
                ty += ui.wrap(ix, ty, iw, 12.0, W::R, MUTED, &line) + 8.0;
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
            ty += wrap_buttons(ui, ix, ty, iw, 40.0, &items);
            if !folded {
                ui.rule(ix, ty, iw, INNER);
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
            ui.text(ix + 28.0, ty + 2.0, 13.0, W::S, MUTED, EN.vault_select_all);
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
                ui.icon(ix - 2.0, ty + 2.0, 22.0, Icon::Done, 12.0, OK);
            } else {
                ui.checkbox(ix, ty + 4.0, row.chosen, true);
            }
            let icon = if row.seed { Icon::Keys } else { Icon::Wallet };
            ui.icon(ix + 28.0, ty + 2.0, 20.0, icon, 11.0, MUTED);
            let detail = if row.loaded {
                EN.vault_loaded.to_string()
            } else {
                row.detail.clone()
            };
            let face = if row.seed && !row.loaded { W::M } else { W::R };
            let dw = ui.measure(12.0, face, &detail).min(iw * 0.4);
            let detail = ui.fit(12.0, face, &detail, dw);
            ui.text_right(
                ix + iw,
                ty,
                26.0,
                12.0,
                face,
                if row.loaded { OK } else { MUTED },
                &detail,
            );
            let name = ui.fit(14.0, W::S, &row.name, iw - 54.0 - dw - 8.0);
            ui.text(ix + 54.0, ty + 3.0, 14.0, W::S, TEXT, &name);
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
        ty += wrap_buttons(ui, ix, ty, iw, 40.0, &items);
        ty - y + 4.0
    };
    let h = measured(ui, |ui| draw(ui));
    ui.card(x, y, w, h, LINE);
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
            let at = files.iter().position(|f| f.name == item.name).unwrap_or(0);
            let open = files.get(at).is_some_and(|f| f.open.is_some());
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
        #[cfg(feature = "unreleased")]
        FileKind::Carry => out.push((
            EN.inbox_actions_sign.into(),
            Style::Primary,
            Action::StartSpend(k),
        )),
        FileKind::Psbt => {
            let style = if app.session.wallets.is_empty() && app.session.keys.is_empty() {
                Style::Secondary
            } else {
                Style::Primary
            };
            out.push((EN.inbox_actions_sign.into(), style, Action::StartSpend(k)));
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
                        WARN,
                    ))
                }
                Err(e) => note = Some((e, ERR)),
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
        let n = ui.fit(14.0, W::M, name, room);
        ui.text(x + 56.0, cy, 14.0, W::M, TEXT, &n);
        crate::file_menu::dots(app, ui, x + w - 12.0 - dw, cy, dw, row);
        let l = ui.fit(12.0, W::R, line, room);
        ui.text(x + 56.0, cy + 20.0, 12.0, W::R, MUTED, &l);
        cy += 46.0;
        if let Some((t, c)) = note {
            cy += ui.wrap(x + 12.0, cy, w - 24.0, 13.0, W::R, *c, t) + 8.0;
        }
        let items: Vec<(&str, Style, Action)> = buttons
            .iter()
            .map(|(l, s, a)| (l.as_str(), *s, *a))
            .collect();
        cy += wrap_buttons(ui, x + 12.0, cy, w - 24.0, 36.0, &items);
        cy - y + 2.0
    };
    // Its height worked out from what it holds, so a card scrolled out of
    // sight is passed over, not drawn.
    let labels: Vec<&str> = buttons.iter().map(|(l, ..)| l.as_str()).collect();
    let note_h = note.map_or(0.0, |(t, _)| {
        faraday_ui::parts::measure_wrap(ui, w - 24.0, 13.0, W::R, t) + 8.0
    });
    let h = 60.0 + note_h + faraday_ui::parts::wrap_buttons_height(ui, w - 24.0, 36.0, &labels);
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

/// [`files`] on a small panel: one scrolled column of what is loaded from
/// open vaults and the Inbox, then the Inbox, then the Outbox by who may
/// read it, then the stick.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn files_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact::screen::{about_learn, finish};
    use crate::secrets::Exposure;
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    // The stick, or what waits for one.
    let (label, action) = if app.stick.is_empty() {
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
    };
    ui.fill(x, y, w, 44.0, 10.0, ACCENT.with_alpha(16));
    ui.stroke(x, y, w, 44.0, 10.0, ACCENT.with_alpha(110));
    ui.dot(x + 18.0, y + 22.0, 4.0, ACCENT);
    let label = ui.fit(14.0, W::R, &label, w - 44.0);
    ui.text_mid(x + 32.0, y, 44.0, 14.0, W::R, TEXT, &label);
    if let Some(a) = action {
        ui.hit(x, y, w, 44.0, a);
    }
    y += 52.0;
    ui.button(
        x,
        y,
        Some(w),
        40.0,
        EN.files_scan_a_qr_code,
        Style::Secondary,
        Action::Scan,
    );
    y += 48.0;
    ui.text(x, y, 12.0, W::R, DIM, EN.files_in_memory_emptied);
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
    // Inbox.
    let lw = ui.text(x, y, 15.0, W::S, MUTED, app.medium.from_box());
    ui.text(
        x + lw + 10.0,
        y + 2.0,
        13.0,
        W::R,
        DIM,
        &app.inbox.len().to_string(),
    );
    y += 28.0;
    for (k, item) in app.inbox.iter().enumerate() {
        let line = if item.kind == FileKind::Vault
            && app
                .vault_files()
                .iter()
                .any(|f| f.name == item.name && f.open.is_some())
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
            (Icon::Download, ACCENT, LINE),
            &item.name,
            &line,
            crate::file_menu::Row::In(k),
            note.as_ref(),
            &buttons,
        ) + 10.0;
    }
    if app.inbox.is_empty() {
        y += ui.wrap(
            x,
            y,
            w,
            13.0,
            W::R,
            DIM,
            &fill(EN.files_empty_files_are, &[app.medium.a()]),
        ) + 14.0;
    }
    // Outbox, grouped by who may read it.
    y += 8.0;
    let lw = ui.text(x, y, 15.0, W::S, MUTED, app.medium.for_box());
    ui.text(
        x + lw + 10.0,
        y + 2.0,
        13.0,
        W::R,
        DIM,
        &app.outbox.len().to_string(),
    );
    y += 28.0;
    for (exposure, label, line, tone) in [
        (
            Exposure::Public,
            EN.files_public,
            EN.files_anyone_may_read.to_string(),
            MUTED,
        ),
        (
            Exposure::Sealed,
            EN.files_written_sealed,
            fill(EN.files_sealed_under, &[app.medium.a()]),
            OK,
        ),
        (
            Exposure::Secret,
            EN.files_unprotected_secrets,
            fill(EN.files_anyone_who_has, &[app.medium.noun()]),
            ERR,
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
        ui.text(x, y, 12.0, W::S, tone, label);
        y += 18.0;
        y += ui.wrap(x, y, w, 12.0, W::R, DIM, &line) + 8.0;
        for (k, item) in group {
            let (edge, icon, ink) = match exposure {
                Exposure::Public => (LINE, Icon::Export, ACCENT),
                Exposure::Sealed => (OK.with_alpha(70), Icon::Lock, OK),
                Exposure::Secret => (ERR.with_alpha(110), Icon::Lock, ERR),
            };
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
                Exposure::Secret => (EN.files_unprotected, ERR),
                _ if open_here.is_some() => (EN.files_written_after, ACCENT),
                _ => (EN.files_ready, OK),
            };
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
        ui.text(x, y, 13.0, W::R, DIM, EN.files_empty);
        y += 26.0;
    }
    if let Some(r) = app.visit.receipt().as_ref() {
        y += receipt_rows(ui, r, app.tz_min(), x, y, w) + 8.0;
    }
    if !app.outbox.is_empty() {
        y += 6.0;
        ui.icon(x, y, 18.0, Icon::Power, 10.0, MUTED);
        y += ui.wrap(
            x + 24.0,
            y + 1.0,
            w - 24.0,
            12.0,
            W::R,
            MUTED,
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
    finish(app, ui, x0, cw, h, y - top + 16.0);
}
