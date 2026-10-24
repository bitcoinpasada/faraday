//! `Screen::Restore`: restoring a wallet.

use super::parts::{
    buttons_and_next, descriptor_row, first_addresses, kind_rows, vaults_answer_line,
};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, short};
use crate::wallet::{FileKind, Session, fp_text};
use crate::{Action, Screen, flow, guide};
use faraday_ui::parts::wrap_buttons;

/// The closed line of Restore's card of vaults: the answer.
fn create_flow_summary_restore(app: &View, k: u8) -> String {
    let Some(r) = app.restore.state() else {
        return String::new();
    };
    if k == crate::rstep::HOW_MANY {
        r.vaults.count.to_string()
    } else {
        vaults_answer_line(r.vaults)
    }
}

pub(crate) const RSTEPS: [&str; crate::rstep::COUNT as usize] = [
    EN.app_kind,
    EN.app_quorum,
    EN.app_description,
    EN.app_keys,
    EN.app_check,
    EN.app_done,
    EN.backup_flow_card_use_vaults,
    EN.backup_flow_card_vault_count,
];

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::rstep;
    let col_x = x0 + 40.0;
    let col_w = (cw - 80.0).min(900.0);
    let Some(r) = app.restore.state() else {
        return;
    };
    let wl = r.wallet.and_then(|i| app.session.wallets.get(i));
    let steps = r.steps();
    let s = r.seeds.as_ref();
    let cards: Vec<flow::Card> = steps
        .iter()
        .map(|&k| flow::Card {
            title: RSTEPS[k as usize].to_string(),
            summary: match k {
                rstep::KIND => match wl.filter(|_| r.described) {
                    Some(w) => Session::shape(w),
                    None => r.kind().name().to_string(),
                },
                rstep::QUORUM => s
                    .map(|s| fill(EN.restore_threshold, &[&s.m.to_string(), &s.n.to_string()]))
                    .unwrap_or_default(),
                rstep::DESCRIPTION => match wl {
                    Some(w) if r.described => w.name.clone(),
                    _ if r.done[k as usize] => EN.restore_none.to_string(),
                    _ => String::new(),
                },
                rstep::WHERE | rstep::HOW_MANY => create_flow_summary_restore(app, k),
                rstep::SEEDS => {
                    let (here, all) = restore_filled(app);
                    fill(EN.restore_of_filled, &[&here.to_string(), &all.to_string()])
                }
                rstep::CHECK => match wl.filter(|_| r.done[rstep::SEEDS as usize]) {
                    Some(w) if app.receive_held(w) => crate::glance::RECEIVE_WARNING.to_string(),
                    Some(w) => short(&app.address_shown(w, false, 0)),
                    None => String::new(),
                },
                _ => String::new(),
            },
            mono: k == rstep::CHECK,
            done: r.done[k as usize],
            open: r.open == Some(k),
            // Read from a description, they are not to be changed.
            default: (!r.described && (k == rstep::KIND || k == rstep::QUORUM))
                // Unanswered, it has no value to change.
                || (k == rstep::WHERE && r.vaults.answer.is_some()),
            toggle: Action::RStep(k),
            guide: (r.open == Some(k)).then(|| guide::restore(k, app.medium)),
            learn: guide::restore_topic(k),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: col_x,
        w: col_w,
        h,
        // Back where it was opened from.
        back: Some(app.named_back(if r.from_home {
            (EN.restore_home, Action::Nav(Screen::Home))
        } else if r.from_key {
            (EN.restore_wallets, Action::Nav(Screen::Wallets))
        } else {
            (EN.restore_wallets, Action::Nav(Screen::WalletsStart))
        })),
        heading: EN.restore_title,
        guided: app.guided(),
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
        cancel: app.cancel_offered().is_some(),
    };
    let scroll = r.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            restore_body(app_ref, ui, steps[i], x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::Restore, next, again);
}

/// Restore's slots: how many are filled (a seed here, a cosigner's
/// xpub, or Not here), of how many.
fn restore_filled(app: &View) -> (usize, usize) {
    let Some(r) = app.restore.state() else {
        return (0, 0);
    };
    if r.described
        && let Some(w) = r.wallet.and_then(|i| app.session.wallets.get(i))
    {
        let slots = app.session.slots(w);
        let filled = slots
            .iter()
            .enumerate()
            .filter(|(k, s)| s.held_by.is_some() || r.absent.contains(&(*k as u8)))
            .count();
        return (filled, slots.len());
    }
    let Some(s) = r.seeds.as_ref() else {
        return (0, 0);
    };
    let xpubs = s
        .cosigners
        .iter()
        .filter(|c| crate::seeds::cosigner_key(c).is_some())
        .count();
    (app.seeds_here(s).len() + xpubs, s.n.max(1))
}

/// The wallet description's file names in Files, by place.
fn description_files<'a>(app: &'a View<'_>) -> Vec<(usize, &'a str)> {
    app.inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == FileKind::Wallet)
        .map(|(k, i)| (k, i.name.as_str()))
        .collect()
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn restore_body(app: &View, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    use crate::rstep;
    let r = app.restore.state().expect("restore");
    let mut cy = y;
    match n {
        rstep::KIND => {
            // The shortcut: a description fills Kind, Quorum and the
            // cosigners' keys, and the flow goes to Seeds.
            if !r.described {
                let label = EN.restore_have_description;
                if ui.compact {
                    cy += ui.wrap(x, cy, w, 13.0, W::S, TEXT, label) + 8.0;
                } else {
                    ui.text_mid(x, cy, 40.0, 13.0, W::S, TEXT, label);
                }
                let mut items: Vec<(&str, Style, Action)> =
                    vec![(EN.restore_scan_button, Style::Secondary, Action::Scan)];
                if !description_files(app).is_empty() {
                    items.push((EN.restore_from_files, Style::Secondary, Action::RFromFiles));
                }
                if ui.compact {
                    cy += wrap_buttons(ui, x, cy, w, 40.0, &items);
                } else {
                    let lw = ui.measure(13.0, W::S, label) + 16.0;
                    cy += wrap_buttons(ui, x + lw, cy, w - lw, 40.0, &items);
                }
                cy += 8.0;
                ui.rule(x, cy, w, BORDER);
                cy += 14.0;
            }
            cy += kind_rows(
                ui,
                x,
                cy,
                w,
                r.kind(),
                r.more_kinds,
                !r.described,
                Action::RKind,
                Action::RMoreKinds,
            );
            cy += 6.0;
            // A kind pressed moves on (F103); one read from a description
            // is not chosen here, and Continue goes on from it.
            let next = r
                .described
                .then_some((EN.restore_continue, Action::RNext(n)));
            cy += buttons_and_next(ui, x, cy, w, &[], next);
        }
        rstep::QUORUM => {
            if let Some(s) = r.seeds.as_ref() {
                cy += crate::seeds::screen::quorum(ui, s, x, cy, w);
            }
            cy += 6.0;
            cy += buttons_and_next(
                ui,
                x,
                cy,
                w,
                &[],
                Some((EN.restore_continue, Action::RNext(n))),
            );
        }
        rstep::DESCRIPTION => cy += restore_description(app, ui, x, cy, w),
        rstep::WHERE => {
            if let Some(h) = crate::backup_flow::wallet_screen::restore_where(app, ui, x, cy, w) {
                cy += h;
            }
        }
        rstep::HOW_MANY => {
            if let Some(h) = crate::backup_flow::wallet_screen::restore_count(app, ui, x, cy, w) {
                cy += h;
            }
        }
        rstep::SEEDS => cy += restore_seeds(app, ui, x, cy, w),
        rstep::CHECK => {
            if let Some(i) = r.wallet.filter(|_| r.done[rstep::SEEDS as usize])
                && let Some(wl) = app.session.wallets.get(i)
            {
                cy += descriptor_row(ui, x, cy, w, wl, Action::QrWallet(i));
                cy += first_addresses(app, ui, i, x, cy, w);
                cy += 6.0;
                cy += buttons_and_next(
                    ui,
                    x,
                    cy,
                    w,
                    &[],
                    Some((EN.restore_it_matches, Action::RNext(n))),
                );
            } else {
                ui.text(x, cy, 13.0, W::R, DIM, EN.restore_make_the_wallet);
                cy += 30.0;
            }
        }
        _ => {
            // Restore ends on a choice, none of them the one to take
            // (`docs/history/NEW-WALLET.md` §12.3).
            if let Some(i) = r.wallet.filter(|_| r.done[rstep::SEEDS as usize]) {
                let (spend, spend_action) = app.spend_button(i);
                let mut items = vec![
                    (spend.as_str(), Style::Secondary, spend_action),
                    (
                        EN.restore_back_up_button,
                        Style::Secondary,
                        Action::Backup(i),
                    ),
                ];
                let from = app
                    .session
                    .wallets
                    .get(i)
                    .and_then(|wl| crate::create::NewKind::of(wl.policy()))
                    .is_some_and(|k| !k.threshold());
                if from {
                    items.push((
                        EN.restore_another_wallet,
                        Style::Secondary,
                        Action::NewWalletFrom(i),
                    ));
                }
                items.push((EN.restore_wallets, Style::Secondary, Action::OpenWallet(i)));
                cy += wrap_buttons(ui, x, cy, w, 40.0, &items) + 4.0;
            }
        }
    }
    cy - y
}

/// Restore's Description card: Scan descriptor, From Files (each
/// description, the shares of a split one), the stick and the vaults,
/// or the wallet from its seeds alone: **I have the seed** for a single
/// key, which most backups of one are, **I do not have it** for several.
fn restore_description(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let r = app.restore.state().expect("restore");
    let mut cy = y;
    let files = description_files(app);
    let labels: Vec<String> = files
        .iter()
        .map(|(_, n)| fill(EN.restore_description_source, &[n]))
        .collect();
    let mut items: Vec<(&str, Style, Action)> = Vec::new();
    if !r.kind().multi() {
        items.push((
            EN.restore_description_i_have,
            Style::Primary,
            Action::RSeeds,
        ));
    }
    items.push((EN.restore_description_scan, Style::Secondary, Action::Scan));
    for ((k, _), l) in files.iter().zip(&labels) {
        items.push((l.as_str(), Style::Secondary, Action::RUse(*k)));
    }
    if r.kind().multi() && !r.kind().threshold() {
        items.push((EN.restore_description_i, Style::Secondary, Action::RSeeds));
    }
    cy += wrap_buttons(ui, x, cy, w, 40.0, &items) + 8.0;
    cy += restore_sources(app, ui, x, cy, w);
    let shares: Vec<(usize, &str)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == FileKind::Share)
        .map(|(k, i)| (k, i.name.as_str()))
        .collect();
    if !shares.is_empty() {
        ui.text(x, cy, 12.0, W::R, MUTED, EN.restore_description_shares);
        cy += 22.0;
        for (k, name) in &shares {
            let on = r.shares.iter().any(|s| s == name);
            ui.checkbox(x, cy + 8.0, on, true);
            let name = ui.fit(13.0, W::M, name, w - 30.0);
            ui.text_mid(x + 30.0, cy, 34.0, 13.0, W::M, TEXT, &name);
            ui.hit(x, cy, w, 34.0, Action::RShare(*k));
            cy += 38.0;
        }
        let texts = app.restore_share_texts();
        if !texts.is_empty() {
            match crate::restore::merge(&texts) {
                Ok(m) => {
                    let line = fill(
                        EN.restore_description_progress,
                        &[
                            &m.have.len().to_string(),
                            &m.n.to_string(),
                            &m.have.join(" ").to_string(),
                        ],
                    );
                    cy += ui.wrap(
                        x,
                        cy,
                        w,
                        13.0,
                        W::M,
                        if m.whole.is_some() { OK } else { WARN },
                        &line,
                    ) + 10.0;
                    let style = if m.whole.is_some() {
                        Style::Primary
                    } else {
                        Style::Disabled
                    };
                    ui.button(
                        x,
                        cy,
                        None,
                        38.0,
                        EN.restore_description_rebuild,
                        style,
                        Action::RRebuild,
                    );
                    cy += 48.0;
                }
                Err(e) => {
                    cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, &e) + 10.0;
                }
            }
        }
    }
    if let Some(e) = &r.error {
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 6.0;
    }
    cy - y
}

/// Restore's Seeds card: a slot per key. Each filled slot says what
/// fills it; the first empty one takes a passphrase and its seed, typed,
/// scanned, from an open vault or already loaded, or a cosigner's xpub;
/// a key of the description can be Not here. **Make the wallet** once
/// every slot is filled.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn restore_seeds(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::seeds::{Focus, SeedsAction as S};
    let r = app.restore.state().expect("restore");
    let Some(s) = r.seeds.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    let multi = r.kind().multi();
    let mut open_drawn = false;
    // One row: the slot's number, its fingerprint, what fills it.
    let row = |ui: &mut Ui, cy: f32, k: usize, fp: &str, line: &str, color| {
        ui.text_mid(x, cy, 40.0, 12.0, W::R, DIM, &(k + 1).to_string());
        ui.text_mid(x + 22.0, cy, 40.0, 14.0, W::M, TEXT, fp);
        let line = ui.fit(13.0, W::R, line, w - 124.0);
        ui.text_mid(x + 124.0, cy, 40.0, 13.0, W::R, color, &line);
    };
    if r.described
        && let Some(wl) = r.wallet.and_then(|i| app.session.wallets.get(i))
    {
        let here: Vec<[u8; 4]> = app.session.keys.all().map(|k| k.fingerprint.0).collect();
        for (k, slot) in app.session.slots(wl).iter().enumerate() {
            let fp = slot.fingerprint.map(fp_text).unwrap_or_default();
            let absent = r.absent.contains(&(k as u8));
            if slot.held_by.is_some() {
                row(
                    ui,
                    cy,
                    k,
                    &fp,
                    &fill(EN.restore_seeds_key_number, &[&(k + 1).to_string()]),
                    OK,
                );
                cy += 42.0;
            } else if absent {
                row(ui, cy, k, &fp, EN.restore_seeds_not_here, DIM);
                let cw = ui.measure(13.0, W::S, EN.restore_seeds_change) + 32.0;
                ui.button(
                    x + w - cw,
                    cy + 2.0,
                    Some(cw),
                    36.0,
                    EN.restore_seeds_change,
                    Style::Ghost,
                    Action::RAbsent(k as u8),
                );
                cy += 42.0;
            } else if !open_drawn {
                // The ways its seed comes in, over the slot they fill.
                open_drawn = true;
                let extra = multi.then_some((EN.restore_seeds_not_here, Action::RAbsent(k as u8)));
                cy += restore_open_slot(app, ui, r, &here, extra, x, cy, w);
                row(ui, cy, k, &fp, EN.restore_seeds_empty, DIM);
                cy += 42.0;
            } else {
                row(ui, cy, k, &fp, EN.restore_seeds_empty, DIM);
                if multi {
                    let nw = ui.measure(13.0, W::S, EN.restore_seeds_not_here) + 32.0;
                    ui.button(
                        x + w - nw,
                        cy + 2.0,
                        Some(nw),
                        36.0,
                        EN.restore_seeds_not_here,
                        Style::Ghost,
                        Action::RAbsent(k as u8),
                    );
                }
                cy += 42.0;
            }
            ui.rule(x, cy, w, BORDER);
            cy += 8.0;
        }
    } else {
        let here = app.seeds_here(s);
        for (k, fp) in here.iter().enumerate() {
            let f = faraday_bip::keys::Fingerprint(*fp);
            let label = app.session.keys.key_label(f).unwrap_or("");
            let line = if label.is_empty() || label.eq_ignore_ascii_case(&fp_text(f)) {
                EN.restore_seeds_here.to_string()
            } else {
                fill(EN.restore_seeds_here_n, &[label])
            };
            row(ui, cy, k, &fp_text(f), &line, OK);
            cy += 42.0;
            ui.rule(x, cy, w, BORDER);
            cy += 8.0;
        }
        for (k, text) in s.cosigners.iter().enumerate() {
            let slot = here.len() + k;
            let focused = s.focus == Some(Focus::Cosigner(k as u8));
            if focused || !text.trim().is_empty() {
                cy += crate::seeds::screen::cosigner(app, ui, s, k, slot + 1, x, cy, w);
            } else if !open_drawn {
                open_drawn = true;
                let extra = multi.then_some((
                    EN.restore_seeds_its_xpub,
                    Action::Seeds(S::Open(Focus::Cosigner(k as u8))),
                ));
                cy += restore_open_slot(app, ui, r, &here, extra, x, cy, w);
                row(ui, cy, slot, "", EN.restore_seeds_empty, DIM);
                cy += 42.0;
            } else {
                row(ui, cy, slot, "", EN.restore_seeds_empty, DIM);
                if multi {
                    let label = EN.restore_seeds_its_xpub;
                    let nw = ui.measure(13.0, W::S, label) + 32.0;
                    ui.button(
                        x + w - nw,
                        cy + 2.0,
                        Some(nw),
                        36.0,
                        label,
                        Style::Ghost,
                        Action::Seeds(S::Open(Focus::Cosigner(k as u8))),
                    );
                }
                cy += 42.0;
            }
            ui.rule(x, cy, w, BORDER);
            cy += 8.0;
        }
    }
    // The open slot says why its seed was refused; with none open, here.
    let error = if open_drawn { None } else { r.error.as_ref() };
    if let Some(e) = error.or(s.error.as_ref()) {
        cy += ui.wrap(x, cy, w, 13.0, W::S, ERR, e) + 8.0;
    }
    let (filled, all) = restore_filled(app);
    let ready = filled >= all && (r.described || app.seeds_descriptor(s).is_ok());
    if ready {
        cy += 6.0;
        cy += buttons_and_next(
            ui,
            x,
            cy,
            w,
            &[],
            Some((EN.restore_seeds_make, Action::Seeds(S::Make))),
        );
    }
    cy - y
}

/// The open slot's ways in, drawn over it: why its last seed was
/// refused, then **Type the words**, **Scan a SeedQR**, a seed an open
/// vault holds or one already loaded, and `extra` (Its xpub, Not here).
/// The passphrase is asked after the seed: on Add a key for typed words,
/// on the scan's sheet for a SeedQR (`seed_read`). Returns the height
/// used.
#[allow(clippy::too_many_arguments)]
fn restore_open_slot(
    app: &View,
    ui: &mut Ui,
    r: &crate::RestoreState,
    here: &[[u8; 4]],
    extra: Option<(&str, Action)>,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut cy = y;
    if let Some(e) = &r.error {
        cy += ui.wrap(x, cy, w, 13.0, W::S, ERR, e) + 10.0;
    }
    let mut labels: Vec<(String, Style, Action)> = vec![
        (
            EN.restore_open_slot_type.to_string(),
            Style::Primary,
            Action::RSlotWords,
        ),
        (
            EN.restore_open_slot_scan.to_string(),
            Style::Secondary,
            Action::RSlotScan,
        ),
    ];
    crate::seeds::screen::more_ways(app, here, true, &mut labels);
    if let Some((label, action)) = extra {
        labels.push((label.to_string(), Style::Secondary, action));
    }
    let items: Vec<(&str, Style, Action)> = labels
        .iter()
        .map(|(l, st, a)| (l.as_str(), *st, *a))
        .collect();
    cy += wrap_buttons(ui, x, cy, w, 40.0, &items) + 6.0;
    cy - y
}

/// Where a wallet's description comes back from besides the camera and
/// Files: the files of a stick, or a vault.
fn restore_sources(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::vault::VaultAction as V;
    use faraday_vault::records::{field, kind};
    if ui.compact {
        return restore_sources_compact(app, ui, x, y, w);
    }
    let mut cy = y;
    // A stick.
    let stick = !app.stick.is_empty() && !app.holds_secret();
    ui.text_mid(x, cy, 40.0, 13.0, W::S, MUTED, app.medium.cap());
    let bw = ui.button(
        x + 90.0,
        cy,
        None,
        40.0,
        EN.restore_sources_copy_files,
        if stick {
            Style::Secondary
        } else {
            Style::Disabled
        },
        Action::VisitFrom(Screen::Restore),
    );
    if !stick {
        ui.text_mid(
            x + 90.0 + bw + 12.0,
            cy,
            40.0,
            13.0,
            W::R,
            DIM,
            &if app.holds_secret() {
                EN.restore_sources_key_loaded.to_string()
            } else {
                fill(EN.restore_sources_insert, &[app.medium.noun()])
            },
        );
    }
    cy += 48.0;
    // A vault: locked in Files, or open.
    ui.text_mid(x, cy, 40.0, 13.0, W::S, MUTED, EN.restore_sources_vault);
    let files = app.vault_files();
    let mut bx = x + 90.0;
    let mut any = false;
    for (k, f) in files.iter().enumerate().filter(|(_, f)| f.open.is_none()) {
        bx += ui.button(
            bx,
            cy,
            None,
            40.0,
            &fill(EN.restore_sources_unlock, &[&f.name.to_string()]),
            Style::Secondary,
            Action::Vault(V::OpenFrom(k, Screen::Restore)),
        ) + 8.0;
        any = true;
    }
    for (v, open) in app.vaults.open.iter().enumerate() {
        for (r, rec) in app.vaults.records(open.id).of(kind::WALLET) {
            let name = rec
                .public_text(field::WALLET_NAME)
                .unwrap_or(EN.restore_sources_wallet);
            bx += ui.button(
                bx,
                cy,
                None,
                40.0,
                &format!("{name} · {}", open.name),
                Style::Secondary,
                Action::RFromVault(v, r),
            ) + 8.0;
            any = true;
            if bx > x + w - 160.0 {
                bx = x + 90.0;
                cy += 48.0;
            }
        }
    }
    if !any {
        ui.text_mid(
            bx,
            cy,
            40.0,
            13.0,
            W::R,
            DIM,
            &fill(EN.restore_sources_copy, &[app.medium.noun()]),
        );
    }
    cy += 48.0;
    ui.rule(x, cy - 6.0, w, BORDER);
    cy += 10.0;
    cy - y
}

/// [`restore_sources`] on a small panel: each source's name over its
/// buttons, and what stands in the way under them.
fn restore_sources_compact(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::vault::VaultAction as V;
    use faraday_vault::records::{field, kind};
    let mut cy = y;
    // A stick.
    let stick = !app.stick.is_empty() && !app.holds_secret();
    ui.text(x, cy, 13.0, W::S, MUTED, app.medium.cap());
    cy += 22.0;
    cy += wrap_buttons(
        ui,
        x,
        cy,
        w,
        40.0,
        &[(
            EN.restore_sources_copy_files,
            if stick {
                Style::Secondary
            } else {
                Style::Disabled
            },
            Action::VisitFrom(Screen::Restore),
        )],
    );
    if !stick {
        let why = if app.holds_secret() {
            EN.restore_sources_key_loaded.to_string()
        } else {
            fill(EN.restore_sources_insert, &[app.medium.noun()])
        };
        cy += ui.wrap(x, cy, w, 13.0, W::R, DIM, &why) + 8.0;
    }
    cy += 8.0;
    // A vault: locked in Files, or open.
    ui.text(x, cy, 13.0, W::S, MUTED, EN.restore_sources_vault);
    cy += 22.0;
    let files = app.vault_files();
    let mut labels: Vec<(String, Action)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .map(|(k, f)| {
            (
                fill(EN.restore_sources_unlock, &[&f.name.to_string()]),
                Action::Vault(V::OpenFrom(k, Screen::Restore)),
            )
        })
        .collect();
    for (v, open) in app.vaults.open.iter().enumerate() {
        for (r, rec) in app.vaults.records(open.id).of(kind::WALLET) {
            let name = rec
                .public_text(field::WALLET_NAME)
                .unwrap_or(EN.restore_sources_wallet);
            labels.push((format!("{name} · {}", open.name), Action::RFromVault(v, r)));
        }
    }
    if labels.is_empty() {
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::R,
            DIM,
            &fill(EN.restore_sources_copy, &[app.medium.noun()]),
        ) + 8.0;
    } else {
        let items: Vec<(&str, Style, Action)> = labels
            .iter()
            .map(|(l, a)| (l.as_str(), Style::Secondary, *a))
            .collect();
        cy += wrap_buttons(ui, x, cy, w, 40.0, &items);
    }
    cy += 8.0;
    ui.rule(x, cy - 6.0, w, BORDER);
    cy += 10.0;
    cy - y
}
