//! `Screen::Create`: creating a wallet.

use super::parts::{
    buttons_and_next, descriptor_row, first_addresses, kind_list, locked_tag, never_real_seed,
    next_button, pass_field, stepper, vaults_answer_line,
};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::{FileKind, fp_text, key_line};
use crate::{Action, Code, Screen, flow, guide};
use faraday_draw::tokens;
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
    let Some(c) = app.create.state() else { return };
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
            summary: card_summary(app, k, filled),
            mono: false,
            done: app.create_card_done(k),
            open: c.open == Some(k),
            default: k == cstep::KIND || k == cstep::QUORUM || k == cstep::WHERE,
            toggle: Action::CStep(k),
            // Only the open card shows its walk-through: the others make none.
            guide: (c.open == Some(k))
                .then(|| create_guide(k, c, app.medium))
                .filter(|g| !g.is_empty()),
            learn: create_topic(k, c.kind),
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
        guided: app.guided(),
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

/// The walk-through of Make a new wallet's card `k`: Single key, backup
/// in shares has its own on Quorum and Keys (`docs/CODEX32.md` §6.9).
fn create_guide(k: u8, c: &crate::CreateState, medium: crate::medium::Medium) -> String {
    use crate::cstep;
    match k {
        cstep::QUORUM if c.kind.shares() => fill(
            EN.create_guide_shares_quorum,
            &[&c.share_k.to_string(), &c.share_n.to_string()],
        ),
        cstep::KEYS if c.kind.shares() => EN.create_guide_shares_keys.to_string(),
        _ => guide::create(k, c.kind, c.m, c.n, medium),
    }
}

/// The Learn page under card `k`'s walk-through: codex32 is on Other
/// backups.
fn create_topic(k: u8, kind: crate::create::NewKind) -> Option<crate::learn::Topic> {
    use crate::cstep;
    match k {
        cstep::QUORUM | cstep::KEYS if kind.shares() => Some(crate::learn::Topic::OtherBackups),
        _ => guide::create_topic(k, kind),
    }
}

/// The closed line of Make a new wallet's card `k`, with the account on
/// the card that carries the Account row (F98); `filled`, the slots with
/// a key.
fn card_summary(app: &View, k: u8, filled: usize) -> String {
    use crate::cstep;
    let Some(c) = app.create.state() else {
        return String::new();
    };
    let line = match k {
        cstep::WHERE | cstep::HOW_MANY | cstep::STICKS | cstep::VAULTS => {
            create_flow_summary(app, k)
        }
        cstep::KIND if c.kind_unchosen => EN.create_not_answered.to_string(),
        cstep::KIND => c.kind.name().to_string(),
        cstep::QUORUM if c.kind.shares() => fill(
            EN.create_threshold,
            &[&c.share_k.to_string(), &c.share_n.to_string()],
        ),
        cstep::QUORUM => fill(EN.create_threshold, &[&c.m.to_string(), &c.n.to_string()]),
        cstep::KEYS => match app.create_waiting() {
            0 => fill(
                EN.create_of_chosen,
                &[&filled.to_string(), &c.slots.len().to_string()],
            ),
            k => fill(
                EN.create_slots_progress,
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
        _ => String::new(),
    };
    // A wallet made from another, or at an account other than 0, says
    // its account (`docs/history/NEW-WALLET.md` §16.2, §17).
    if c.kind.account_card() == Some(k) && (c.from.is_some() || c.account != 0) {
        return fill(EN.create_at_account, &[&line, &c.account.to_string()]);
    }
    line
}

/// The closed line of Make a new wallet's cards before Keys: what the
/// person answered and what the setup has done.
fn create_flow_summary(app: &View, k: u8) -> String {
    use crate::cstep;
    let Some(c) = app.create.state() else {
        return String::new();
    };
    let flow = app
        .wallet_flow_of_owner(c.owner)
        .map(|i| (i, &app.flows[i]));
    match (k, flow) {
        (cstep::HOW_MANY, _) => c.vaults.count.to_string(),
        (cstep::STICKS, Some((_, f))) => {
            let (read, of) = f.setup_sticks_read();
            fill(
                EN.backup_flow_read_count,
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
                EN.backup_flow_unlocked_count,
                &[&open.to_string(), &used.len().to_string()],
            )
        }
        (cstep::WHERE, _) if c.vaults.answer.is_none() => EN.create_not_answered.to_string(),
        (cstep::WHERE, _) => vaults_answer_line(c.vaults),
        _ => String::new(),
    }
}

/// Under a Keys slot's session keys, each open vault's keys not loaded,
/// under the vault's name, and an Unlock button for each locked vault
/// (`docs/history/NEW-WALLET.md` §16.1). A key in another slot is not
/// offered (§2.2). Returns the height used.
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
    let Some(c) = app.create.state() else {
        return 0.0;
    };
    let mut cy = y;
    for group in &vaults.open {
        let row: Vec<(String, Style, Action)> = group
            .keys
            .iter()
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
        ui.text(x, cy, 12.0, W::S, Role::OnSurfaceVariant, &name);
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
    let c = app.create.state().expect("create");
    let locked = c.built.is_some();
    let mut cy = y;
    match n {
        cstep::KIND => {
            // A row pressed moves on, the ticked one too: no Continue.
            let shown =
                crate::create::NewKind::create_rows(c.more_kinds, crate::create::MAKE_OFFERED);
            let rows: Vec<(usize, String)> = shown
                .into_iter()
                .map(|i| {
                    let line = crate::create::NewKind::ALL[i]
                        .create_line((c.m, c.n), (c.share_k, c.share_n));
                    (i, line)
                })
                .collect();
            let more = (!c.more_kinds).then_some(Action::CMoreKinds);
            // From Move money no kind is ticked: the person chooses
            // (owner, 2026-10-24).
            let chosen = (!c.kind_unchosen).then_some(c.kind);
            cy += kind_list(ui, (x, cy, w), &rows, chosen, more, !locked, Action::CKind);
            if app.create_unfinished() {
                cy += tokens::GAP_SMALL;
                let over = [(EN.create_start_over, Style::Ghost, Action::CreateOver)];
                cy += buttons_and_next(ui, x, cy, w, &over, None);
            }
        }
        cstep::QUORUM if c.kind.shares() => cy += shares_quorum(ui, x, cy, w, c),
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
                cy += tokens::STEPPER_ROW;
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
            cy += ui.wrap(x, cy, w, 13.0, W::R, Role::OnSurface, &sentence) + 14.0;
            // The account, below the quorum (F98).
            if c.kind.account_card() == Some(n) {
                cy += account_row(ui, x, cy, w, c);
            }
            if next_button(ui, x, cy, w, EN.create_continue, Action::CNext(n)) {
                cy += 48.0;
            }
        }
        cstep::KEYS => {
            // The account, at the head of Keys for a kind with no Quorum
            // card (F98).
            if c.kind.account_card() == Some(n) {
                cy += account_row(ui, x, cy, w, c);
            }
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
                let label = format!("{} {}", EN.create_key, slot + 1);
                ui.text(x, cy + 4.0, 13.0, W::S, Role::OnSurface, &label);
                // A key locked in with a passphrase here says so
                // (`docs/history/NEW-WALLET.md` §3.6).
                if let Source::Here(fp) = src
                    && c.pass_locked.contains(fp)
                {
                    let tw = ui.measure(12.0, W::S, EN.create_locked_line) + 40.0;
                    locked_tag(ui, x + w - tw, cy);
                }
                let (state, color) = match src {
                    Source::Empty => (EN.create_not_chosen.to_string(), Role::OnSurfaceVariant),
                    Source::Here(fp) => {
                        let fpt = fp_text(faraday_bip::keys::Fingerprint(*fp));
                        let name = app
                            .session
                            .keys
                            .key_label(faraday_bip::keys::Fingerprint(*fp))
                            .unwrap_or("");
                        (
                            fill(EN.create_loaded_here, &[&fpt.to_string(), name]),
                            Role::Ok,
                        )
                    }
                    Source::Cosigner(t) => (
                        fill(EN.create_a_cosigner_s, &[&t[1..9.min(t.len())]]),
                        Role::OnSurfaceVariant,
                    ),
                    Source::Later => (
                        app.create_slot_needs(slot)
                            .unwrap_or_else(|| EN.create_a_cosigner_adds.to_string()),
                        Role::Warning,
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
                    // The choices for the slot, wrapped to the card. A
                    // backup in shares has one way in: its key is made
                    // as the scheme's codex32 shares (`docs/CODEX32.md`
                    // §6.9).
                    let mut row: Vec<(String, Style, Action)> = Vec::new();
                    for k in &app.session.keys {
                        if c.kind.shares() {
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
                        (if c.kind.shares() {
                            EN.create_make_codex32_shares
                        } else {
                            EN.create_new_key
                        })
                        .to_string(),
                        if first_empty == Some(slot) {
                            Style::Primary
                        } else {
                            Style::Secondary
                        },
                        Action::KeyGen(Some(slot as u8)),
                    ));
                    let files = !c.kind.shares();
                    for (i, name, text) in key_files.iter().filter(|_| files) {
                        if used_elsewhere_cosigner(text) {
                            continue;
                        }
                        row.push((
                            fill(EN.create_slot_source, &[&name.to_string()]),
                            Style::Secondary,
                            Action::CSlotFile(slot as u8, *i),
                        ));
                    }
                    if c.kind.multi() && matches!(src, Source::Empty | Source::Later) {
                        row.push((
                            EN.create_scan_its_xpub.to_string(),
                            Style::Secondary,
                            Action::Scan,
                        ));
                    }
                    if c.kind.multi() && *src == Source::Empty {
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
                    if !c.kind.shares() {
                        cy += create_vault_rows(app, ui, &vaults, slot, x + indent, cy, w - indent);
                    }
                    // A key held here goes out to the cosigners, who need
                    // it to make the same wallet.
                    if c.kind.multi() && matches!(src, Source::Here(_)) {
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
                            (
                                c.pass_focus == Some(0),
                                app.pass_chip(crate::demo::PassField::Slot(0)),
                            ),
                            Action::CPassField(0),
                        );
                        cy += pass_field(
                            ui,
                            px,
                            cy,
                            pw,
                            EN.create_passphrase_again,
                            &c.pass2,
                            (
                                c.pass_focus == Some(1),
                                app.pass_chip(crate::demo::PassField::Slot(1)),
                            ),
                            Action::CPassField(1),
                        );
                        cy += never_real_seed(app, ui, px, cy, pw);
                        if let Some(n) = &c.pass_note {
                            cy += ui.wrap(px, cy, pw, 13.0, W::R, Role::Error, n) + 6.0;
                        }
                        let bw = ui.measure(14.0, W::S, EN.create_lock_button) + 36.0;
                        ui.button(
                            px + pw - bw,
                            cy,
                            Some(bw),
                            40.0,
                            EN.create_lock_button,
                            Style::Primary,
                            Action::CPassLock,
                        );
                        cy += 52.0;
                    }
                }
                ui.rule(x, cy, w, Role::OutlineVariant);
                cy += 10.0;
            }
            if let Some(e) = &c.error {
                cy += ui.wrap(x, cy, w, 13.0, W::R, Role::Error, e) + 6.0;
            }
            // The foot shows once every slot is filled: Make the wallet,
            // which makes it; the slot's own New key leads until then.
            // Once made, Continue goes on to Check.
            let label = match first_empty {
                Some(_) if !locked => None,
                _ if locked => Some(EN.create_continue),
                _ => Some(EN.create_make_the_wallet_button),
            };
            if let Some(label) = label
                && next_button(ui, x, cy, w, label, Action::CNext(n))
            {
                cy += tokens::TOUCH;
            }
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
                // Single key, backup in shares: the set its key was made
                // as (`docs/CODEX32.md` §6.9).
                if let Some(set) = c.shares_set.as_ref().filter(|_| c.kind.shares()) {
                    cy += change_row(ui, x, cy, w, EN.create_backup, &set.line(), None);
                }
                cy += first_addresses(app, ui, c.built.unwrap_or(0), x, cy, w);
                cy += 6.0;
                cy += check_foot(app, ui, x, cy, w, c.done[n as usize]);
            } else {
                ui.text(
                    x,
                    cy,
                    13.0,
                    W::R,
                    Role::OnSurfaceVariant,
                    EN.create_make_the_wallet,
                );
                cy += 30.0;
            }
        }
        _ => {}
    }
    cy - y
}

/// Check's foot: **It matches** until the wallet is checked, then
/// **Back up wallet**, which opens Back up on the wallet just made, the
/// board open on The standard way (`docs/BACKUP-FLOW.md` §17.21 item 2),
/// and beside it, for a wallet made to move another's money to
/// (`docs/history/NEW-WALLET.md` §9.6), the spend of every coin from the
/// old one. Returns its height.
fn check_foot(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, checked: bool) -> f32 {
    if !checked {
        let next = (EN.create_it_matches, Action::CNext(crate::cstep::CHECK));
        return buttons_and_next(ui, x, y, w, &[], Some(next));
    }
    let moving = app.moving_from().map(|old| {
        let label = fill(
            EN.create_move_the_money,
            &[&app.session.wallets[old].name.to_string()],
        );
        let label = ui.fit(14.0, W::S, &label, w - 32.0);
        (
            label,
            Style::Secondary,
            Action::Family(crate::family::FamilyAction::SpendFrom(old)),
        )
    });
    let left: Vec<(&str, Style, Action)> = moving
        .iter()
        .map(|(l, s, a)| (l.as_str(), *s, *a))
        .collect();
    buttons_and_next(
        ui,
        x,
        y,
        w,
        &left,
        Some((EN.create_back_up_button, Action::CBackup)),
    )
}

/// Single key, backup in shares' Quorum: the codex32 scheme, How many to
/// rebuild it (2 to 9) and How many shares (k to 31), as Make codex32
/// shares' Other asks them; the line of fact under them, the Account row
/// at the foot (F98) and Continue (`docs/CODEX32.md` §6.9). Returns its
/// height.
fn shares_quorum(ui: &mut Ui, x: f32, y: f32, w: f32, c: &crate::CreateState) -> f32 {
    let locked = c.built.is_some();
    let mut cy = y;
    let rows = [
        (
            EN.create_shares_rebuild,
            c.share_k,
            Action::CM(-1),
            Action::CM(1),
        ),
        (
            EN.create_shares_count,
            c.share_n,
            Action::CN(-1),
            Action::CN(1),
        ),
    ];
    for (label, value, minus, plus) in rows {
        stepper(
            ui,
            x,
            cy,
            w,
            label,
            usize::from(value),
            minus,
            plus,
            !locked,
        );
        cy += tokens::STEPPER_ROW;
    }
    let line = fill(
        EN.create_shares_any,
        &[&c.share_k.to_string(), &c.share_n.to_string()],
    );
    cy += ui.wrap(
        x,
        cy,
        w,
        tokens::CAPTION + 1.0,
        W::R,
        Role::OnSurface,
        &line,
    ) + tokens::PAD;
    if c.kind.account_card() == Some(crate::cstep::QUORUM) {
        cy += account_row(ui, x, cy, w, c);
    }
    if next_button(
        ui,
        x,
        cy,
        w,
        EN.create_continue,
        Action::CNext(crate::cstep::QUORUM),
    ) {
        cy += tokens::TOUCH;
    }
    cy - y
}

/// Check's Path row: the made wallet's derivation paths, worked out when
/// it was made, with **Change** back to the Account row while the wallet
/// can be taken back (F98, `docs/history/NEW-WALLET.md` §17). Nothing when
/// there is no path. Returns its height.
fn path_row(ui: &mut Ui, x: f32, y: f32, w: f32, path: &str, change: bool) -> f32 {
    if path.is_empty() {
        return 0.0;
    }
    let change = change.then_some((EN.create_path_change, Action::CPathChange));
    change_row(ui, x, y, w, EN.create_path, path, change)
}

/// The Account row (F98), for a kind whose path has an account: its
/// value with **Change**, or, once Change is pressed, its − n + stepper.
/// The made wallet's row has no Change. Returns its height.
fn account_row(ui: &mut Ui, x: f32, y: f32, w: f32, c: &crate::CreateState) -> f32 {
    let locked = c.built.is_some();
    if c.account_open && !locked {
        stepper(
            ui,
            x,
            y,
            w,
            EN.create_account,
            c.account as usize,
            Action::CAccount(-1),
            Action::CAccount(1),
            true,
        );
        return tokens::STEPPER_ROW;
    }
    let change = (!locked).then_some((EN.create_account_change, Action::CAccount(0)));
    change_row(
        ui,
        x,
        y,
        w,
        EN.create_account,
        &c.account.to_string(),
        change,
    )
}

/// A value row: `label` above, `value` mono and wrapped below, and a
/// **Change** button at the right when `change` is given. Returns its
/// height.
fn change_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: &str,
    change: Option<(&str, Action)>,
) -> f32 {
    let bw = change.map_or(0.0, |(l, _)| ui.measure(13.0, W::S, l) + 32.0);
    let vw = if change.is_some() { w - bw - 10.0 } else { w };
    ui.text(x, y, 12.0, W::R, Role::OnSurfaceVariant, label);
    let vh = ui
        .wrap(x, y + 20.0, vw, 13.0, W::M, Role::OnSurface, value)
        .max(18.0);
    let h = 20.0 + vh + 8.0;
    if let Some((l, action)) = change {
        ui.button(
            x + w - bw,
            y + (h - 34.0) / 2.0,
            Some(bw),
            34.0,
            l,
            Style::Ghost,
            action,
        );
    }
    ui.rule(x, y + h, w, Role::OutlineVariant);
    h + 12.0
}
