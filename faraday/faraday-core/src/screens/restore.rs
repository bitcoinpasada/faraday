//! `Screen::Restore`: restoring a wallet.

use super::parts::{buttons_and_next, descriptor_row, first_addresses, kind_rows, next_button};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::{FileKind, fp_text};
use crate::{Action, Screen, flow};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, GAP, GAP_SMALL, LABEL_L, LABEL_M, TOUCH};
use faraday_ui::parts::wrap_buttons;

pub(crate) const RSTEPS: [&str; crate::rstep::COUNT as usize] = [
    EN.app_kind,
    EN.app_quorum,
    EN.app_description,
    EN.app_keys,
    EN.app_check,
    EN.app_done,
];

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(r) = app.restore.state() else {
        return;
    };
    let page = flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        pin: None,
        // Back where it was opened from.
        back: Some(app.named_back({
            let to = r.back_to();
            (
                match to {
                    Screen::Catalog => EN.restore_tools,
                    Screen::Decode => EN.restore_decode,
                    Screen::Home => EN.restore_home,
                    Screen::Backups => EN.backups_backups,
                    Screen::Wallets | Screen::WalletsStart => EN.restore_wallets,
                    _ => EN.restore_back,
                },
                Action::Nav(to),
            )
        })),
        heading: if r.from_key {
            EN.create_make_a_new_wallet
        } else {
            EN.restore_title
        },
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::Yes,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&crate::restore::RESTORE_STEPS) else {
        return;
    };
    let (next, again, _room) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, k: u8, x: f32, y: f32, w: f32| -> f32 {
            restore_body(app_ref, ui, k, x, y, w)
        };
        // Nothing stands under Your choices' rows.
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        flow::contained(
            ui, app_ref, contained, &page, r.scroll, &mut body, &mut under,
        )
    };
    app.scroll(Scrolled::Restore, next, again);
}

/// The wallet description's file names in Files, by place.
fn description_files<'a>(app: &'a View<'_>) -> Vec<(usize, &'a str)> {
    app.inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| crate::create::wallet_flow_kind(i) == FileKind::Wallet)
        .map(|(k, i)| (k, i.name.as_str()))
        .collect()
}

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
                let mut labels: Vec<(usize, String)> = Vec::new();
                if ui.compact {
                    cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, label) + 8.0;
                } else {
                    ui.text_mid_as((x, cy, 40.0), LABEL_L, Role::OnSurface, label);
                }
                let mut items: Vec<(&str, Style, Action)> = vec![(
                    EN.restore_scan_button,
                    Style::Secondary,
                    Action::ScanFor(crate::Ask::Description),
                )];
                let files = description_files(app);
                match files.as_slice() {
                    [] => {}
                    [_] => {
                        items.push((EN.restore_from_files, Style::Secondary, Action::RFromFiles))
                    }
                    _ => {
                        for (k, name) in &files {
                            labels.push((*k, fill(EN.restore_description_source, &[name])));
                        }
                    }
                }
                for (k, l) in &labels {
                    items.push((l.as_str(), Style::Secondary, Action::RUse(*k)));
                }
                if ui.compact {
                    cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &items);
                } else {
                    let lw = ui.measure_as(LABEL_L, label) + 16.0;
                    cy += wrap_buttons(ui, x + lw, cy, w - lw, BUTTON_H, &items);
                }
                cy += 8.0;
                ui.rule(x, cy, w, Role::OutlineVariant);
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
        rstep::SEEDS => cy += restore_seeds(app, ui, x, cy, w),
        rstep::CHECK => {
            if let Some(i) = r.wallet.filter(|_| r.done[rstep::SEEDS as usize])
                && let Some(wl) = app.session.wallets.get(i)
            {
                cy += descriptor_row(ui, x, cy, w, wl, Action::QrWallet(i));
                cy += first_addresses(app, ui, i, x, cy, w);
                cy += 6.0;
                // A wallet made from the seeds alone says what it assumed.
                // It reads the wallet made, not the seeds card: the card can
                // be changed after Make.
                let alone = (!r.described && r.seeds.is_some())
                    .then(|| crate::wallet_backup::made_from_seed_line(wl))
                    .flatten();
                if let Some(line) = &alone {
                    cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, line) + GAP;
                }
                let (matches, left): (&str, &[(&str, Style, Action)]) = if alone.is_some() {
                    (
                        EN.restore_it_matches_record,
                        &[(
                            EN.restore_nothing_to_compare,
                            Style::Secondary,
                            Action::RNext(n),
                        )],
                    )
                } else {
                    (EN.restore_it_matches, &[])
                };
                cy += buttons_and_next(ui, x, cy, w, left, Some((matches, Action::RNext(n))));
            } else {
                ui.text_as(
                    x,
                    cy,
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.restore_make_the_wallet,
                );
                cy += ui.advance_as(BODY_S, 30.0);
            }
        }
        _ => {
            // Restore ends on the one end card.
            if let Some(i) = r.wallet.filter(|_| r.done[rstep::SEEDS as usize]) {
                cy += crate::screens::backup::wallet_done(
                    app,
                    ui,
                    i,
                    crate::screens::backup::From::Restore,
                    None,
                    (x, cy, w),
                ) + 4.0;
            }
        }
    }
    cy - y
}

/// Restore's Description card: the shares of a split description, the
/// stick and the vaults, or the wallet from its seeds alone: **I have the seed** for a single
/// key, which most backups of one are, **I do not have it** for several.
fn restore_description(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let r = app.restore.state().expect("restore");
    let mut cy = y;
    let mut items: Vec<(&str, Style, Action)> = Vec::new();
    if !r.kind().multi() {
        items.push((
            EN.restore_description_i_have,
            Style::Primary,
            Action::RSeeds,
        ));
    }
    if r.kind().multi() {
        items.push((EN.restore_description_i, Style::Secondary, Action::RSeeds));
    }
    cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &items) + 8.0;
    cy += restore_sources(app, ui, x, cy, w);
    // The share files held, one checkbox row each (the session pane's row
    // kind for a step that takes several): ticked as `RShare` toggles them.
    let shares = shares_rows(app, r);
    if !shares.is_empty() {
        ui.text_as(
            x,
            cy,
            BODY_S,
            Role::OnSurfaceVariant,
            EN.restore_description_shares,
        );
        cy += ui.advance_as(BODY_S, 22.0);
        cy += crate::pick::rows(ui, x, cy, w, &shares);
        if let Some(pool) = app.restore_pool() {
            match &pool.merged {
                Ok(m) => {
                    let line = fill(
                        EN.restore_description_progress,
                        &[
                            &m.have.len().to_string(),
                            &m.n.to_string(),
                            &m.have.join(" ").to_string(),
                        ],
                    );
                    cy += ui.wrap_as(
                        (x, cy, w),
                        BODY_M.mono(),
                        if m.whole.is_some() {
                            Role::Ok
                        } else {
                            Role::Warning
                        },
                        &line,
                    ) + 10.0;
                    let style = Style::Primary.when(m.whole.is_some());
                    ui.button(
                        x,
                        cy,
                        None,
                        BUTTON_H,
                        EN.restore_description_rebuild,
                        style,
                        Action::RRebuild,
                    );
                    cy += 48.0;
                }
                Err(e) => {
                    cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, e) + 10.0;
                }
            }
        }
    }
    if let Some(e) = &r.error {
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, e) + 6.0;
    }
    cy - y
}

/// The held share files as the pane's checkbox rows: each ticked when the
/// restore has it, a press toggling it. Only share files are offered.
pub(crate) fn shares_rows(
    app: &crate::Faraday,
    r: &crate::RestoreState,
) -> Vec<crate::pick::PickRow> {
    use crate::pick::{Decl, Take};
    Decl::new("")
        .files(app, |k, i| {
            (i.kind == FileKind::Share).then(|| {
                let on = r.shares.iter().any(|s| s == i.name.as_str());
                Take::of(on, Action::RShare(k)).several()
            })
        })
        .groups()
        .into_iter()
        .flat_map(|g| g.rows)
        .collect()
}

/// Restore's Seeds card: a slot per key. Each filled slot says what
/// fills it; the first empty one takes a passphrase and its seed, typed,
/// scanned, from an open vault or already loaded, or a cosigner's xpub;
/// a key of the description can be Not here. **Make the wallet** once
/// every slot is filled.
fn restore_seeds(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::seeds::{Focus, SeedsAction as S};
    let r = app.restore.state().expect("restore");
    let Some(s) = r.seeds.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    // A vault already on a stick comes in before any seed is typed: off
    // until pressed, and a stick is never read while a seed is in memory.
    let on = r.uses_existing_vault();
    if !r.done[crate::rstep::SEEDS as usize] && (on || !r.seeds_in(&app.session)) {
        cy += ui.switch_row(
            (x, cy, w),
            EN.create_use_existing_vault,
            on,
            Action::RExistingVault(!on),
        );
    }
    if let (true, Some(i)) = (
        app.restore_setup_pending(),
        app.wallet_flow_of_owner(r.owner),
    ) {
        cy += ui.wrap_as(
            (x, cy + GAP_SMALL, w),
            BODY_M,
            Role::OnSurfaceVariant,
            EN.create_vault_to_unlock,
        ) + GAP_SMALL;
        let show = Action::Flow(crate::backup_flow::steps::FlowAction::Show(i));
        if next_button(ui, x, cy, w, app.medium.bring_in(), show) {
            cy += TOUCH;
        }
        return cy - y;
    }
    if r.described {
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.restore_wrong_passphrase,
        ) + GAP;
    }
    let multi = r.kind().multi();
    let mut open_drawn = false;
    // What fills a slot starts at 124 dp; on a panel too narrow for "Not
    // here" between it and Change, it starts as far left as the
    // fingerprint allows.
    let change_w = ui.measure_as(LABEL_L, EN.restore_seeds_change) + 32.0;
    let not_here_w = ui.measure_as(BODY_S, EN.restore_seeds_not_here);
    let fp_end = 22.0 + ui.measure_as(BODY_M.mono(), "00000000") + 8.0;
    let col = 124.0_f32
        .min(w - change_w - not_here_w - GAP_SMALL)
        .max(fp_end);
    // One row: the slot's number, its fingerprint, what fills it.
    let row = |ui: &mut Ui, cy: f32, k: usize, fp: &str, line: &str, color| {
        ui.text_mid_as(
            (x, cy, 40.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &(k + 1).to_string(),
        );
        ui.text_mid_as((x + 22.0, cy, 40.0), BODY_M.mono(), Role::OnSurface, fp);
        let line = ui.fit_as(BODY_S, line, w - col);
        ui.text_mid_as((x + col, cy, 40.0), BODY_S, color, &line);
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
                    Role::Ok,
                );
                cy += 42.0;
            } else if absent {
                let cw = ui.measure_as(LABEL_L, EN.restore_seeds_change) + 32.0;
                // Its line stops short of Change beside it.
                let line = ui.fit_as(BODY_S, EN.restore_seeds_not_here, w - col - cw);
                row(ui, cy, k, &fp, &line, Role::OnSurfaceVariant);
                ui.button(
                    x + w - cw,
                    cy + 2.0,
                    Some(cw),
                    BUTTON_H,
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
                row(
                    ui,
                    cy,
                    k,
                    &fp,
                    EN.restore_seeds_empty,
                    Role::OnSurfaceVariant,
                );
                cy += 42.0;
            } else {
                row(
                    ui,
                    cy,
                    k,
                    &fp,
                    EN.restore_seeds_empty,
                    Role::OnSurfaceVariant,
                );
                if multi {
                    let nw = ui.measure_as(LABEL_L, EN.restore_seeds_not_here) + 32.0;
                    ui.button(
                        x + w - nw,
                        cy + 2.0,
                        Some(nw),
                        BUTTON_H,
                        EN.restore_seeds_not_here,
                        Style::Ghost,
                        Action::RAbsent(k as u8),
                    );
                }
                cy += 42.0;
            }
            ui.rule(x, cy, w, Role::OutlineVariant);
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
            row(ui, cy, k, &fp_text(f), &line, Role::Ok);
            cy += 42.0;
            ui.rule(x, cy, w, Role::OutlineVariant);
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
                row(
                    ui,
                    cy,
                    slot,
                    "",
                    EN.restore_seeds_empty,
                    Role::OnSurfaceVariant,
                );
                cy += 42.0;
            } else {
                row(
                    ui,
                    cy,
                    slot,
                    "",
                    EN.restore_seeds_empty,
                    Role::OnSurfaceVariant,
                );
                if multi {
                    let label = EN.restore_seeds_its_xpub;
                    let nw = ui.measure_as(LABEL_L, label) + 32.0;
                    ui.button(
                        x + w - nw,
                        cy + 2.0,
                        Some(nw),
                        BUTTON_H,
                        label,
                        Style::Ghost,
                        Action::Seeds(S::Open(Focus::Cosigner(k as u8))),
                    );
                }
                cy += 42.0;
            }
            ui.rule(x, cy, w, Role::OutlineVariant);
            cy += 8.0;
        }
    }
    // The open slot says why its seed was refused; with none open, here.
    let error = if open_drawn { None } else { r.error.as_ref() };
    if let Some(e) = error
        .map(|e| e.as_str())
        .or(s.error.as_ref().map(|e| e.as_str()))
    {
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, e) + 8.0;
    }
    let (filled, all) = crate::restore::restore_filled(app);
    let made = (filled >= all && !r.described).then(|| app.seeds_descriptor(s));
    let ready = filled >= all && (r.described || matches!(made, Some(Ok(_))));
    // With no description the seeds alone say neither the account nor a
    // path other than the standard one: the person says them.
    if !r.described && filled > 0 {
        cy += crate::seeds::screen::path_rows(ui, s, app.session.network(), x, cy, w);
        // A path that gives no wallet says why, where Make would be.
        if s.custom.is_some()
            && let Some(Err(e)) = &made
        {
            cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, e) + GAP;
        }
    }
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
/// refused, then **Type the words**, **Scan a SeedQR**, codex32,
/// **Other forms** (its rows in place), a seed an open
/// vault holds or one already loaded, and `extra` (Its xpub, Not here).
/// The passphrase is asked after the seed: on Add a key for typed words,
/// on the scan's sheet for a SeedQR (`seed_read`). Returns the height
/// used.
#[allow(
    clippy::too_many_arguments,
    reason = "the restore slot's view, state, keys held here, extra action, position and width"
)]
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
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, e) + 10.0;
    }
    let opener = &crate::ways_in::RESTORE_SLOT;
    let open = r.slot_forms;
    let form_row = |f: crate::forms::Form| {
        (
            f.name().to_string(),
            Style::Secondary,
            Action::RSlotForm(f.code()),
        )
    };
    let mut labels: Vec<(String, Style, Action)> = Vec::new();
    for way in opener.up_front {
        labels.push(match way {
            crate::ways_in::Way::Scan | crate::ways_in::Way::SeedQr => (
                EN.restore_open_slot_scan.to_string(),
                Style::Secondary,
                Action::RSlotScan,
            ),
            crate::ways_in::Way::Form(crate::forms::Form::Words) => (
                EN.entry_type_the_words.to_string(),
                Style::Primary,
                Action::RSlotWords,
            ),
            crate::ways_in::Way::Form(f) => form_row(*f),
            // Restore's slot has no such way.
            crate::ways_in::Way::Words(_)
            | crate::ways_in::Way::Aezeed
            | crate::ways_in::Way::Holder(_)
            | crate::ways_in::Way::AddAKey => continue,
        });
    }
    // A sheet on a small panel, rows in place on the wide layout.
    let other = if app.one_column() {
        Action::EntryOtherForms(crate::Opening::RestoreSlot)
    } else {
        Action::RSlotForms
    };
    labels.push((EN.entry_other_forms.to_string(), Style::chosen(open), other));
    if open && !app.one_column() {
        labels.extend(opener.other_forms().into_iter().map(form_row));
    }
    crate::seeds::screen::more_ways(app, true, &mut labels);
    if let Some((label, action)) = extra {
        labels.push((label.to_string(), Style::Secondary, action));
    }
    let items: Vec<(&str, Style, Action)> = labels
        .iter()
        .map(|(l, st, a)| (l.as_str(), *st, *a))
        .collect();
    cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &items) + 6.0;
    // A described wallet's slots match the loaded keys by themselves.
    if !r.described {
        cy += crate::seeds::screen::loaded_keys(app, ui, r.seeds.as_ref(), here, x, cy, w);
    }
    cy - y
}

/// What holds a secret, said for the stick's Copy files: a key, a vault
/// or something typed, with what Lock does to this page.
fn held_why(app: &View) -> &'static str {
    if app.session.keys.all().next().is_some() {
        EN.restore_sources_key_loaded
    } else if !app.vaults.open.is_empty() {
        EN.restore_sources_vault_open
    } else {
        EN.restore_sources_secret_typed
    }
}

/// Where a wallet's description comes back from besides the camera and
/// Files: the files of a stick, or a vault.
fn restore_sources(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let muted = Role::OnSurfaceVariant;
    use crate::vault::VaultAction as V;
    use faraday_vault::records::{field, kind};
    if ui.compact {
        return restore_sources_compact(app, ui, x, y, w);
    }
    let mut cy = y;
    // A stick.
    let stick = !app.stick.is_empty() && !app.holds_secret();
    ui.text_mid_as((x, cy, 40.0), LABEL_M, muted, app.medium.cap());
    let bw = ui.button(
        x + 90.0,
        cy,
        None,
        BUTTON_H,
        EN.restore_sources_copy_files,
        if stick {
            Style::Secondary
        } else {
            Style::Disabled
        },
        Action::VisitFrom(Screen::Restore),
    );
    if !stick {
        let tx = x + 90.0 + bw + 12.0;
        let tw = ui.text_mid_as(
            (tx, cy, 40.0),
            BODY_S,
            muted,
            &if app.holds_secret() {
                held_why(app).to_string()
            } else {
                fill(EN.restore_sources_insert, &[app.medium.noun()])
            },
        );
        if app.holds_secret() && !app.stick.is_empty() {
            ui.button(
                tx + tw + GAP,
                cy,
                None,
                BUTTON_H,
                EN.restore_sources_lock,
                Style::Secondary,
                Action::Lock,
            );
        }
    }
    cy += 48.0;
    // A vault: locked in Files, or open.
    ui.text_mid_as((x, cy, 40.0), LABEL_M, muted, EN.restore_sources_vault);
    let files = app.vault_files();
    let mut bx = x + 90.0;
    let mut any = false;
    for (k, f) in files.iter().enumerate().filter(|(_, f)| f.open.is_none()) {
        bx += ui.button(
            bx,
            cy,
            None,
            BUTTON_H,
            &fill(EN.restore_sources_unlock, &[&f.name]),
            Style::Secondary,
            Action::Vault(V::OpenFrom(k, Screen::Restore)),
        ) + 8.0;
        any = true;
    }
    for (v, open) in app.vaults.open.iter().enumerate() {
        for (r, rec) in app.vaults.records(open.id).of(kind::WALLET) {
            let name =
                crate::vault::wallet_name_in(rec, field::WALLET_NAME, EN.restore_sources_wallet);
            bx += ui.button(
                bx,
                cy,
                None,
                BUTTON_H,
                &format!("{name} · {}", open.name.as_str()),
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
        ui.text_mid_as(
            (bx, cy, 40.0),
            BODY_S,
            muted,
            &fill(EN.restore_sources_copy, &[app.medium.noun()]),
        );
    }
    cy += 48.0;
    ui.rule(x, cy - 6.0, w, Role::OutlineVariant);
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
    ui.text_as(x, cy, LABEL_M, Role::OnSurfaceVariant, app.medium.cap());
    cy += ui.advance_as(LABEL_M, 22.0);
    cy += wrap_buttons(
        ui,
        x,
        cy,
        w,
        BUTTON_H,
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
            held_why(app).to_string()
        } else {
            fill(EN.restore_sources_insert, &[app.medium.noun()])
        };
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &why) + 8.0;
        if app.holds_secret() && !app.stick.is_empty() {
            cy += wrap_buttons(
                ui,
                x,
                cy,
                w,
                BUTTON_H,
                &[(EN.restore_sources_lock, Style::Secondary, Action::Lock)],
            );
        }
    }
    cy += 8.0;
    // A vault: locked in Files, or open.
    ui.text_as(
        x,
        cy,
        LABEL_M,
        Role::OnSurfaceVariant,
        EN.restore_sources_vault,
    );
    cy += ui.advance_as(LABEL_M, 22.0);
    let files = app.vault_files();
    let mut labels: Vec<(String, Action)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .map(|(k, f)| {
            (
                fill(EN.restore_sources_unlock, &[&f.name]),
                Action::Vault(V::OpenFrom(k, Screen::Restore)),
            )
        })
        .collect();
    for (v, open) in app.vaults.open.iter().enumerate() {
        for (r, rec) in app.vaults.records(open.id).of(kind::WALLET) {
            let name =
                crate::vault::wallet_name_in(rec, field::WALLET_NAME, EN.restore_sources_wallet);
            labels.push((
                format!("{name} · {}", open.name.as_str()),
                Action::RFromVault(v, r),
            ));
        }
    }
    if labels.is_empty() {
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            &fill(EN.restore_sources_copy, &[app.medium.noun()]),
        ) + 8.0;
    } else {
        let items: Vec<(&str, Style, Action)> = labels
            .iter()
            .map(|(l, a)| (l.as_str(), Style::Secondary, *a))
            .collect();
        cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &items);
    }
    cy += 8.0;
    ui.rule(x, cy - 6.0, w, Role::OutlineVariant);
    cy += 10.0;
    cy - y
}

/// The key Add a key is for when Restore's open slot opened it: the
/// fingerprint of the description's first key not here, or the number of
/// the slot a wallet made from the seeds fills next. `None` elsewhere.
pub(crate) fn open_slot_key(app: &View) -> Option<String> {
    if app.entry.state().back != Some(Screen::Restore) {
        return None;
    }
    let r = app.restore.state()?;
    if r.described {
        let wl = r.wallet.and_then(|i| app.session.wallets.get(i))?;
        let slots = app.session.slots(wl);
        let (k, slot) = slots
            .iter()
            .enumerate()
            .find(|(k, s)| s.held_by.is_none() && !r.absent.contains(&(*k as u8)))?;
        return Some(
            slot.fingerprint
                .map_or_else(|| (k + 1).to_string(), |f| fp_text(f).to_string()),
        );
    }
    let s = r.seeds.as_ref()?;
    let first = s
        .cosigners
        .iter()
        .position(|t| t.trim().is_empty())
        .unwrap_or(0);
    Some((app.seeds_here(s).len() + first + 1).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::session_pane::pick::Kind;

    #[test]
    fn only_share_files_are_offered_and_ticked_as_held() {
        let mut app = crate::Faraday::new();
        for (name, kind) in [
            ("a.txt", FileKind::Share),
            ("b.txt", FileKind::Other),
            ("c.txt", FileKind::Share),
        ] {
            let mut item = crate::Item::new(&crate::testkit::compute(), name, b"x".to_vec());
            item.kind = kind;
            crate::testkit::inbox_push(&mut app, item);
        }
        let mut r = crate::RestoreState::default();
        r.shares.push(app.inbox[2].name.clone());
        let rows = shares_rows(&app, &r);
        let got: Vec<_> = rows
            .iter()
            .map(|x| (x.head.as_str(), x.kind, x.action))
            .collect();
        assert_eq!(
            got,
            [
                ("a.txt", Kind::Tick { ticked: false }, Action::RShare(0)),
                ("c.txt", Kind::Tick { ticked: true }, Action::RShare(2)),
            ]
        );
    }
}
