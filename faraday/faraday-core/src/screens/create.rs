//! `Screen::Create`: creating a wallet.

use super::parts::{
    buttons_and_next, descriptor_row, first_addresses, kind_rows, locked_tag, never_real_seed_if,
    next_button, pass_field, stepper, vaults_answer_line,
};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::{FileKind, fp_text, key_line};
use crate::{Action, Code, Screen, flow, guide};
use faraday_ui::parts::button_rows;

pub(crate) const CSTEPS: [&str; crate::cstep::COUNT] = [
    EN.app_kind,
    EN.app_quorum,
    EN.app_keys,
    EN.app_check,
    EN.app_back_up,
    EN.backup_flow_card_use_vaults,
    // Bring in the sticks: named by the medium where it is drawn.
    "",
    EN.backup_flow_card_vaults,
    EN.backup_flow_card_vault_count,
];

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::cstep;
    let col_x = x0 + 40.0;
    let col_w = (cw - 80.0).min(900.0);
    let Some(c) = app.create.as_ref() else { return };
    let steps = app.create_cards();
    let filled = c
        .slots
        .iter()
        .filter(|s| **s != crate::create::Source::Empty)
        .count();
    let cards: Vec<flow::Card> = steps
        .iter()
        .map(|&k| flow::Card {
            title: if k == cstep::STICKS {
                fill(EN.backup_flow_card_sticks, &[app.medium.nouns()])
            } else {
                CSTEPS[k as usize].to_string()
            },
            summary: match k {
                cstep::WHERE | cstep::HOW_MANY | cstep::STICKS | cstep::VAULTS => {
                    create_flow_summary(app, k)
                }
                // A wallet made from another, or at another account than
                // 0, says its account (`docs/history/NEW-WALLET.md` §16.2, §17).
                cstep::KIND if (c.from.is_some() || c.account != 0) && c.kind.has_account() => {
                    fill(
                        EN.create_at_account,
                        &[c.kind.name(), &c.account.to_string()],
                    )
                }
                cstep::KIND => c.kind.name().to_string(),
                cstep::QUORUM => fill(EN.create_of, &[&c.m.to_string(), &c.n.to_string()]),
                cstep::KEYS if c.kind.threshold() => fill(
                    EN.create_of_chosen_computed,
                    &[
                        &filled.to_string(),
                        &c.slots.len().to_string(),
                        &(c.n - c.m).to_string(),
                    ],
                ),
                cstep::KEYS => match app.create_waiting() {
                    0 => fill(
                        EN.create_of_chosen,
                        &[&filled.to_string(), &c.slots.len().to_string()],
                    ),
                    k => fill(
                        EN.create_of_chosen_to,
                        &[
                            &(filled - k).to_string(),
                            &c.slots.len().to_string(),
                            &k.to_string(),
                        ],
                    ),
                },
                cstep::CHECK => match c.built.and_then(|i| app.session.wallets.get(i)) {
                    Some(w) => fill(EN.create_first_addresses, &[&w.name.to_string()]),
                    None => EN.create_not_made_yet.to_string(),
                },
                _ => EN.create_opens_on_standard.to_string(),
            },
            mono: false,
            done: app.create_card_done(k),
            open: c.open == Some(k),
            default: k == cstep::KIND || k == cstep::QUORUM || k == cstep::WHERE,
            toggle: Action::CStep(k),
            guide: Some(guide::create(k, c.kind, c.m, c.n, app.medium)).filter(|g| !g.is_empty()),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: col_x,
        w: col_w,
        h,
        // Back where it was opened from.
        back: Some(app.named_back(if c.from_home {
            (EN.create_home, Action::Nav(Screen::Home))
        } else {
            (EN.create_wallets, Action::Nav(Screen::WalletsStart))
        })),
        heading: EN.create_make_a_new_wallet,
        guided: app.guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
        cancel: app.cancel_offered().is_some(),
    };
    let scroll = c.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            create_body(app_ref, ui, steps[i], x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::Create, next, again);
}

/// The closed line of Make a new wallet's cards before Keys: what the
/// person answered and what the setup has done.
fn create_flow_summary(app: &View, k: u8) -> String {
    use crate::cstep;
    let Some(c) = app.create.as_ref() else {
        return String::new();
    };
    let flow = app
        .wallet_flow_of_owner(c.owner)
        .map(|i| (i, &app.backup_flows[i]));
    match (k, flow) {
        (cstep::HOW_MANY, _) => c.vaults.count.to_string(),
        (cstep::STICKS, Some((_, f))) => {
            let (read, of) = f.setup_sticks_read();
            fill(
                EN.backup_flow_read_of,
                &[&read.to_string(), &of.to_string()],
            )
        }
        (cstep::VAULTS, Some((i, f))) => {
            let used = f.setup_vaults();
            let open = used
                .iter()
                .filter(|&&v| app.flow_open_index(i, v).is_some())
                .count();
            fill(
                EN.backup_flow_unlocked_of,
                &[&open.to_string(), &used.len().to_string()],
            )
        }
        (cstep::WHERE, _) => vaults_answer_line(c.vaults),
        _ => String::new(),
    }
}

/// Under a Keys slot's session keys, each open vault's keys not loaded,
/// under the vault's name, and an Unlock button for each locked vault
/// (`docs/history/NEW-WALLET.md` §16.1). A threshold kind lists only shares; a
/// key in another slot is not offered (§2.2). Returns the height used.
#[allow(clippy::too_many_arguments)]
fn create_vault_rows(
    app: &View,
    ui: &mut Ui,
    vaults: &crate::create_from::CreateVaults,
    slot: usize,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    use crate::create::Source;
    let Some(c) = app.create.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    for group in &vaults.open {
        let row: Vec<(String, Style, Action)> = group
            .keys
            .iter()
            .filter(|k| !c.kind.threshold() || k.share)
            .filter(|k| {
                !c.slots
                    .iter()
                    .enumerate()
                    .any(|(i, s)| i != slot && *s == Source::Here(k.fp))
            })
            .map(|k| {
                (
                    k.line.clone(),
                    Style::Secondary,
                    Action::CSlotVault(slot as u8, group.vault, k.record),
                )
            })
            .collect();
        if row.is_empty() {
            continue;
        }
        let name = ui.fit(12.0, W::S, &group.name, w);
        ui.text(x, cy, 12.0, W::S, MUTED, &name);
        cy += 20.0;
        cy += button_rows(ui, x, cy, w, &row);
    }
    if !vaults.locked.is_empty() {
        let row: Vec<(String, Style, Action)> = vaults
            .locked
            .iter()
            .map(|(i, name)| {
                (
                    fill(EN.vaults_unlock, &[name]),
                    Style::Secondary,
                    Action::Vault(crate::vault::VaultAction::OpenFrom(*i, Screen::Create)),
                )
            })
            .collect();
        cy += button_rows(ui, x, cy, w, &row);
    }
    cy - y
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn create_body(app: &View, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    use crate::create::Source;
    use crate::cstep;
    let c = app.create.as_ref().expect("create");
    let locked = c.built.is_some();
    let mut cy = y;
    match n {
        cstep::KIND => {
            cy += kind_rows(
                ui,
                x,
                cy,
                w,
                c.kind,
                c.more_kinds,
                !locked,
                Action::CKind,
                Action::CMoreKinds,
            );
            cy += 6.0;
            // The account its keys are used at, for every kind whose
            // path has one (`docs/history/NEW-WALLET.md` §16.2, §17).
            if c.kind.has_account() {
                stepper(
                    ui,
                    x,
                    cy,
                    w,
                    EN.create_account,
                    c.account as usize,
                    Action::CAccount(-1),
                    Action::CAccount(1),
                    !locked,
                );
                cy += 50.0;
            }
            let over: &[(&str, Style, Action)] = if app.create_unfinished() {
                &[(EN.create_start_over, Style::Ghost, Action::CreateOver)]
            } else {
                &[]
            };
            cy += buttons_and_next(
                ui,
                x,
                cy,
                w,
                over,
                Some((EN.create_continue, Action::CNext(n))),
            );
        }
        cstep::QUORUM => {
            // MuSig2 is every key: only the count is chosen.
            let rows: &[(&str, usize, Action, Action)] = if c.kind.all_sign() {
                &[(EN.create_keys, c.n, Action::CN(-1), Action::CN(1))]
            } else {
                &[
                    (
                        EN.create_signatures_needed,
                        c.m,
                        Action::CM(-1),
                        Action::CM(1),
                    ),
                    (EN.create_keys, c.n, Action::CN(-1), Action::CN(1)),
                ]
            };
            for &(label, value, minus, plus) in rows {
                stepper(ui, x, cy, w, label, value, minus, plus, !locked);
                cy += 50.0;
            }
            let sentence = if c.kind.all_sign() {
                fill(EN.create_all_keys_sign, &[&c.n.to_string()])
            } else {
                fill(
                    EN.create_any_of_the_keys,
                    &[
                        &c.m.to_string(),
                        &c.n.to_string(),
                        &(c.n - c.m).to_string(),
                        (if c.n - c.m == 1 {
                            EN.create_key_inline
                        } else {
                            EN.create_keys_inline
                        }),
                        &c.m.to_string(),
                    ],
                )
            };
            cy += ui.wrap(x, cy, w, 13.0, W::R, TEXT, &sentence) + 14.0;
            if next_button(ui, x, cy, w, EN.create_continue, Action::CNext(n)) {
                cy += 48.0;
            }
        }
        cstep::KEYS => {
            let key_files: Vec<(usize, String, String)> = app
                .inbox
                .iter()
                .enumerate()
                // Files with a key for this kind: an account xpub, or a
                // wallet file that carries account keys (Coldcard's export).
                .filter_map(|(i, it)| {
                    if !matches!(it.kind, FileKind::Key | FileKind::Wallet) {
                        return None;
                    }
                    let text = crate::create::key_for(c.kind, &String::from_utf8_lossy(&it.bytes))?;
                    Some((i, it.name.clone(), text))
                })
                .collect();
            // The way on leads: New key on the first slot still empty,
            // Continue once every slot is filled.
            let first_empty = c.slots.iter().position(|s| *s == Source::Empty);
            // The open vaults' keys and the locked vaults, from the
            // drawing memo (`docs/history/NEW-WALLET.md` §16.1).
            let vaults = app.create_vaults();
            for (slot, src) in c.slots.iter().enumerate() {
                let label = format!(
                    "{} {}",
                    if c.kind.threshold() {
                        EN.create_share
                    } else {
                        EN.create_key
                    },
                    slot + 1
                );
                ui.text(x, cy + 4.0, 13.0, W::S, TEXT, &label);
                // A key locked in with a passphrase here says so
                // (`docs/history/NEW-WALLET.md` §3.6).
                if let Source::Here(fp) = src
                    && c.pass_locked.contains(fp)
                {
                    let tw = ui.measure(12.0, W::S, EN.create_locked_in) + 40.0;
                    locked_tag(ui, x + w - tw, cy);
                }
                let (state, color) = match src {
                    Source::Empty => (EN.create_not_chosen.to_string(), DIM),
                    Source::Here(fp) => {
                        let fpt = fp_text(faraday_bip::keys::Fingerprint(*fp));
                        let name = app
                            .session
                            .keys
                            .key_label(faraday_bip::keys::Fingerprint(*fp))
                            .unwrap_or("");
                        (fill(EN.create_loaded_here, &[&fpt.to_string(), name]), OK)
                    }
                    Source::Cosigner(t) => (
                        fill(EN.create_a_cosigner_s, &[&t[1..9.min(t.len())]]),
                        MUTED,
                    ),
                    Source::Later => (
                        app.create_slot_needs(slot)
                            .unwrap_or_else(|| EN.create_a_cosigner_adds.to_string()),
                        WARN,
                    ),
                };
                let indent = if ui.compact {
                    cy += 22.0;
                    let state = ui.fit(13.0, W::R, &state, w);
                    ui.text(x, cy + 4.0, 13.0, W::R, color, &state);
                    0.0
                } else {
                    ui.text(x + 70.0, cy + 4.0, 13.0, W::R, color, &state);
                    70.0
                };
                cy += 28.0;
                if !locked {
                    // A key chosen for one slot is offered in no other
                    // (`docs/history/NEW-WALLET.md` §2.2): clearing the slot
                    // offers it again.
                    let used_elsewhere_here = |fp: [u8; 4]| {
                        c.slots
                            .iter()
                            .enumerate()
                            .any(|(i, s)| i != slot && *s == Source::Here(fp))
                    };
                    let used_elsewhere_cosigner = |text: &str| {
                        c.slots.iter().enumerate().any(|(i, s)| {
                            i != slot && matches!(s, Source::Cosigner(t) if t == text)
                        })
                    };
                    // The choices for the slot, wrapped to the card.
                    let mut row: Vec<(String, Style, Action)> = Vec::new();
                    for k in &app.session.keys {
                        if c.kind.threshold() && k.share.is_none() {
                            continue;
                        }
                        let fp = k.fingerprint;
                        if used_elsewhere_here(fp.0) {
                            continue;
                        }
                        let on = *src == Source::Here(fp.0);
                        row.push((
                            key_line(fp, &k.label),
                            if on { Style::Primary } else { Style::Secondary },
                            Action::CSlotHere(slot as u8, fp.0),
                        ));
                    }
                    row.push((
                        EN.create_new_key.to_string(),
                        if first_empty == Some(slot) {
                            Style::Primary
                        } else {
                            Style::Secondary
                        },
                        Action::KeyGen(Some(slot as u8)),
                    ));
                    for (i, name, text) in key_files.iter().filter(|_| !c.kind.threshold()) {
                        if used_elsewhere_cosigner(text) {
                            continue;
                        }
                        row.push((
                            fill(EN.create_from, &[&name.to_string()]),
                            Style::Secondary,
                            Action::CSlotFile(slot as u8, *i),
                        ));
                    }
                    if c.kind.multi()
                        && !c.kind.threshold()
                        && matches!(src, Source::Empty | Source::Later)
                    {
                        row.push((
                            EN.create_scan_its_xpub.to_string(),
                            Style::Secondary,
                            Action::Scan,
                        ));
                    }
                    if c.kind.multi() && !c.kind.threshold() && *src == Source::Empty {
                        row.push((
                            EN.create_someone_else.to_string(),
                            Style::Secondary,
                            Action::CSlotLater(slot as u8),
                        ));
                    }
                    // A key from BIP-39 words with no passphrase can take
                    // one here: on the slot's own row on a single-key
                    // wallet, beside Show xpub QR on a multisig.
                    let pass_offered = app.create_pass_offered(slot);
                    if pass_offered && !c.kind.multi() {
                        row.push((
                            EN.create_add_a_passphrase.to_string(),
                            if c.pass_slot == Some(slot as u8) {
                                Style::Primary
                            } else {
                                Style::Secondary
                            },
                            Action::CPassOpen(slot as u8),
                        ));
                    }
                    if *src != Source::Empty {
                        row.push((
                            EN.create_clear.to_string(),
                            Style::Ghost,
                            Action::CSlotClear(slot as u8),
                        ));
                    }
                    cy += button_rows(ui, x + indent, cy, w - indent, &row) - 44.0;
                    cy += 44.0;
                    cy += create_vault_rows(app, ui, &vaults, slot, x + indent, cy, w - indent);
                    // A key held here goes out to the cosigners, who need
                    // it to make the same wallet.
                    if c.kind.multi() && !c.kind.threshold() && matches!(src, Source::Here(_)) {
                        let k = slot as u8;
                        let mut row = vec![
                            (
                                EN.create_show_xpub_qr.to_string(),
                                Style::Secondary,
                                Action::CKeyQr(k),
                            ),
                            (
                                fill(EN.create_xpub_png, &[app.medium.for_the()]),
                                Style::Secondary,
                                Action::CodePng(Code::Key(k)),
                            ),
                            (
                                fill(EN.create_xpub_file, &[app.medium.for_the()]),
                                Style::Secondary,
                                Action::CKeyOut(k),
                            ),
                        ];
                        if pass_offered {
                            row.insert(
                                1,
                                (
                                    EN.create_add_a_passphrase.to_string(),
                                    if c.pass_slot == Some(k) {
                                        Style::Primary
                                    } else {
                                        Style::Secondary
                                    },
                                    Action::CPassOpen(k),
                                ),
                            );
                        }
                        cy += button_rows(ui, x + indent, cy, w - indent, &row);
                    }
                    // The passphrase fields, under the slot they are for.
                    if c.pass_slot == Some(slot as u8) && pass_offered {
                        let (px, pw) = (x + indent, w - indent);
                        cy += pass_field(
                            ui,
                            px,
                            cy,
                            pw,
                            EN.create_passphrase,
                            &c.pass,
                            (c.pass_focus == Some(0), app.demo_passphrase()),
                            Action::CPassField(0),
                        );
                        cy += pass_field(
                            ui,
                            px,
                            cy,
                            pw,
                            EN.create_passphrase_again,
                            &c.pass2,
                            (c.pass_focus == Some(1), app.demo_passphrase()),
                            Action::CPassField(1),
                        );
                        cy += never_real_seed_if(!app.demo_passphrase(), app, ui, px, cy, pw);
                        if let Some(n) = &c.pass_note {
                            cy += ui.wrap(px, cy, pw, 13.0, W::R, ERR, n) + 6.0;
                        }
                        let bw = ui.measure(14.0, W::S, EN.create_lock_in) + 36.0;
                        ui.button(
                            px + pw - bw,
                            cy,
                            Some(bw),
                            40.0,
                            EN.create_lock_in,
                            Style::Primary,
                            Action::CPassLock,
                        );
                        cy += 52.0;
                    }
                }
                ui.rule(x, cy, w, INNER);
                cy += 10.0;
            }
            if c.kind.threshold() {
                for k in c.m..c.n {
                    ui.text(
                        x,
                        cy + 4.0,
                        13.0,
                        W::S,
                        TEXT,
                        &fill(EN.create_share_n, &[&(k + 1).to_string()]),
                    );
                    ui.text(x + 70.0, cy + 4.0, 13.0, W::R, MUTED, EN.create_computed_by);
                    cy += 34.0;
                    ui.rule(x, cy, w, INNER);
                    cy += 10.0;
                }
            }
            if let Some(e) = &c.error {
                cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 6.0;
            }
            // While a slot is empty, the foot button opens New key for
            // it; once every slot is filled it reads Continue
            // (`docs/history/NEW-WALLET.md` §2.3).
            let (label, action) = match first_empty {
                Some(slot) if !locked => (
                    fill(
                        EN.create_new_key_for,
                        &[
                            (if c.kind.threshold() {
                                EN.create_share
                            } else {
                                EN.create_key
                            }),
                            &(slot + 1).to_string(),
                        ],
                    ),
                    Action::KeyGen(Some(slot as u8)),
                ),
                _ => (EN.create_continue.to_string(), Action::CNext(n)),
            };
            let bw = ui.measure(14.0, W::S, &label) + 36.0;
            ui.button(
                x + w - bw,
                cy,
                Some(bw),
                40.0,
                &label,
                Style::Primary,
                action,
            );
            cy += 48.0;
        }
        cstep::WHERE => {
            if let Some(h) = crate::backup_flow::wallet_screen::create_where(app, ui, x, cy, w) {
                cy += h;
            }
        }
        cstep::HOW_MANY => {
            if let Some(h) = crate::backup_flow::wallet_screen::create_count(app, ui, x, cy, w) {
                cy += h;
            }
        }
        cstep::STICKS | cstep::VAULTS => {
            let step = if n == cstep::STICKS {
                crate::backup_flow::Step::Sticks
            } else {
                crate::backup_flow::Step::Vaults
            };
            if let Some(h) =
                crate::backup_flow::wallet_screen::create_flow_card(app, ui, step, x, cy, w)
            {
                cy += h;
            }
        }
        cstep::CHECK => {
            if let Some(wl) = c.built.and_then(|i| app.session.wallets.get(i)) {
                // The descriptor as a summary row: tapping it opens the
                // wallet's code with the descriptor's full text under it.
                cy += descriptor_row(ui, x, cy, w, wl, Action::QrWallet(c.built.unwrap_or(0)));
                cy += path_row(ui, x, cy, w, &c.path, app.create_can_take_back());
                cy += first_addresses(app, ui, c.built.unwrap_or(0), x, cy, w);
                cy += 6.0;
                cy += buttons_and_next(
                    ui,
                    x,
                    cy,
                    w,
                    &[],
                    Some((EN.create_continue, Action::CNext(n))),
                );
            } else {
                ui.text(x, cy, 13.0, W::R, DIM, EN.create_make_the_wallet);
                cy += 30.0;
            }
        }
        _ => {
            // Back up this wallet: Back up on the wallet just made, the
            // board open on The standard way (`docs/BACKUP-FLOW.md`
            // §17.21 item 2).
            if c.built.is_some() {
                ui.button(
                    x,
                    cy,
                    Some(w),
                    40.0,
                    EN.create_back_up_this,
                    Style::Primary,
                    Action::CBackup,
                );
                cy += 48.0;
                // Made to move a wallet's money to (`docs/history/NEW-WALLET.md`
                // §9.6): the spend of every coin from the old wallet.
                if let Some(old) = app.moving_from() {
                    let label = fill(
                        EN.create_move_the_money,
                        &[&app.session.wallets[old].name.to_string()],
                    );
                    let label = ui.fit(14.0, W::S, &label, w - 32.0);
                    ui.button(
                        x,
                        cy,
                        Some(w),
                        40.0,
                        &label,
                        Style::Primary,
                        Action::Family(crate::family::FamilyAction::SpendFrom(old)),
                    );
                    cy += 48.0;
                }
            } else {
                ui.text(x, cy, 13.0, W::R, DIM, EN.create_make_the_wallet);
                cy += 30.0;
            }
        }
    }
    cy - y
}

/// Check's Path row: the made wallet's derivation paths, worked out when
/// it was made, with **Change** back to Kind's Account while the wallet
/// can be taken back (`docs/history/NEW-WALLET.md` §17). Nothing when there is no
/// path. Returns its height.
fn path_row(ui: &mut Ui, x: f32, y: f32, w: f32, path: &str, change: bool) -> f32 {
    if path.is_empty() {
        return 0.0;
    }
    let bw = ui.measure(13.0, W::S, EN.create_path_change) + 32.0;
    let vw = if change { w - bw - 10.0 } else { w };
    ui.text(x, y, 12.0, W::R, MUTED, EN.create_path);
    let vh = ui.wrap(x, y + 20.0, vw, 13.0, W::M, TEXT, path).max(18.0);
    let h = 20.0 + vh + 8.0;
    if change {
        ui.button(
            x + w - bw,
            y + (h - 34.0) / 2.0,
            Some(bw),
            34.0,
            EN.create_path_change,
            Style::Ghost,
            Action::CPathChange,
        );
    }
    ui.rule(x, y + h, w, INNER);
    h + 12.0
}
