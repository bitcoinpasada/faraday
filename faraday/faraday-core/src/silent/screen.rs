//! The Silent payments screen: the step-card column over
//! [`crate::silent`].

use crate::drawing::{Scrolled, View};
use crate::screens::button_rows;
use crate::silent::{RecordCheck, SilentState, sstep};
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::fp_text;
use crate::{Action, Code, flow};
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, BUTTON_H, FIELD_PAD, GAP, GAP_SMALL, LABEL_L, PAD, TITLE_M,
};
use faraday_ui::Status;

pub(crate) const TITLES: [&str; sstep::COUNT] = [
    EN.silent_key,
    EN.silent_address,
    EN.silent_scan_key,
    EN.silent_check_a_payment,
];

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.silent.state() else {
        return;
    };
    let page = flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        pin: None,
        back: Some(app.named_back((EN.silent_wallets, Action::Nav(s.back)))),
        heading: EN.silent_silent_payments,
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::Yes,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&super::SILENT_STEPS) else {
        return;
    };
    let scroll = s.scroll;
    let (next, again, _) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, k: u8, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.silent.state() {
                Some(s) => card(app_ref, s, ui, k, x, y, w),
                None => 0.0,
            }
        };
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        flow::contained(ui, app_ref, contained, &page, scroll, &mut body, &mut under)
    };
    app.scroll(Scrolled::Silent, next, again);
}

/// The rows of Your choices: each step's answer as the page stands, from
/// what is kept (no work is done here).
pub(super) fn rows(app: &crate::Faraday) -> Vec<flow::Row> {
    let Some(s) = app.silent.state() else {
        return Vec::new();
    };
    let reached = s.key.is_some() || s.wallet.is_some();
    (0..sstep::COUNT as u8)
        .map(|k| {
            let value = match k {
                sstep::KEY => app
                    .silent
                    .named(&app.session)
                    .map(|fp| String::from(&*fp_text(faraday_bip::keys::Fingerprint(fp)))),
                sstep::ADDRESS => app
                    .silent
                    .shown_kept(&app.session)
                    .and_then(|shown| shown.address)
                    .map(|a| {
                        // As the page: no part of an address a key waiting
                        // for its backup would receive on.
                        if app.silent_waits_kept() {
                            EN.glance_back_up_before.to_string()
                        } else {
                            String::from(&*crate::ui::short(&a))
                        }
                    }),
                sstep::SCAN => reached.then(|| EN.silent_a_secret_shows.to_string()),
                _ => s.check.as_ref().map(|(_, c)| match c.result() {
                    Some(r) => checked_text(r).0,
                    None => EN.silent_needs_a_previous.to_string(),
                }),
            };
            crate::visible::choice_row(
                TITLES[usize::from(k)],
                value.as_deref(),
                k == sstep::KEY || k == sstep::ADDRESS,
                (k, s.open == Some(k)),
            )
        })
        .collect()
}

fn card(app: &View, s: &SilentState, ui: &mut Ui, k: u8, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    match k {
        sstep::KEY => {
            let chosen = app.silent.named(&app.session);
            let mut row: Vec<(String, Style, Action)> = app
                .session
                .keys
                .all()
                .map(|key| {
                    let fp = key.fingerprint;
                    (
                        String::from(&*crate::wallet::key_line(fp, &key.label)),
                        Style::chosen(chosen == Some(fp.0)),
                        Action::SKey(fp.0),
                    )
                })
                .collect();
            // Returns here, with the key loaded.
            row.push((
                EN.convert_add_a_key.to_string(),
                Style::Secondary,
                Action::KeyGen(None),
            ));
            cy += button_rows(ui, x, cy, w, &row);
        }
        // A record a loaded key contradicts: no address, no labels, no scan
        // key and no check.
        _ if app.silent_shown().check == Some(RecordCheck::Differs) => {
            let line = crate::silent::differs_line(
                &app.session.keys,
                app.silent.named(&app.session).unwrap_or_default(),
            );
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, &line) + PAD;
        }
        sstep::ADDRESS if !app.silent_shown().key_here && s.wallet.is_some() => {
            // The wallet's own address needs no key, and is not checked
            // without it; labels need it.
            let (a, role) = address_on_show(app);
            cy += ui.wrap_as((x, cy, w), BODY_M.mono(), role, &a) + GAP;
            let fp = app
                .silent
                .named(&app.session)
                .map(|fp| fp_text(faraday_bip::keys::Fingerprint(fp)).to_string())
                .unwrap_or_default();
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::Warning,
                &fill(EN.silent_not_checked, &[&fp]),
            ) + GAP;
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::OnSurfaceVariant,
                EN.silent_its_key_is_not_loaded,
            ) + 12.0;
            let mut row = Vec::new();
            if !app.silent_held() {
                row.push((
                    EN.show_as_qr.to_string(),
                    Style::Secondary,
                    Action::SQr(false),
                ));
            }
            row.push((
                EN.silent_add_its_key.to_string(),
                Style::Secondary,
                Action::Entry(app.silent.named(&app.session)),
            ));
            cy += button_rows(ui, x, cy, w, &row);
            cy += dns_card(app, s, ui, x, cy, w);
        }
        sstep::ADDRESS => {
            let mw = ui.button(
                x,
                cy,
                Some(44.0),
                BUTTON_H,
                "-",
                Style::Secondary,
                Action::SLabel(-1),
            );
            let label = if s.label == 0 {
                EN.silent_no_label.to_string()
            } else {
                fill(EN.silent_label, &[&s.label.to_string()])
            };
            ui.text_mid_as((x + mw + 16.0, cy, 40.0), TITLE_M, Role::OnSurface, &label);
            ui.button(
                x + mw + 150.0,
                cy,
                Some(44.0),
                BUTTON_H,
                "+",
                Style::Secondary,
                Action::SLabel(1),
            );
            cy += 52.0;
            let (a, role) = address_on_show(app);
            cy += ui.wrap_as((x, cy, w), BODY_M.mono(), role, &a)
                + faraday_draw::tokens::SILENT_LINE_GAP;
            // As a receive address's code is not offered while it is held.
            let held = app.silent_held();
            let mut row = Vec::new();
            if !held {
                row.push((
                    EN.show_as_qr.to_string(),
                    Style::Secondary,
                    Action::SQr(false),
                ));
                row.push((
                    EN.silent_as_a_bitcoin.to_string(),
                    Style::Secondary,
                    Action::SQr(true),
                ));
            }
            // The key's own backup releases the address: the wallet that
            // places its seed, as a vault item's Back it up opens.
            if held && let Some(w) = app.silent_hold_backup() {
                let label = if app.wallet_backup_unfinished(w) {
                    EN.backup_flow_continue
                } else {
                    EN.wallets_back_up
                };
                row.push((label.to_string(), Style::Secondary, Action::Backup(w)));
            }
            row.extend([
                (
                    fill(EN.silent_record, &[app.medium.for_the()]),
                    Style::Secondary,
                    Action::SRecord,
                ),
                (
                    EN.silent_record_as_qr.to_string(),
                    Style::Secondary,
                    Action::ShowCode(Code::SilentRecord),
                ),
                (
                    EN.silent_record_as_png.to_string(),
                    Style::Secondary,
                    Action::CodePng(Code::SilentRecord),
                ),
            ]);
            if s.wallet.is_none() {
                row.push((
                    EN.silent_add_as_a_wallet.to_string(),
                    Style::Primary,
                    Action::SAddWallet,
                ));
            }
            cy += button_rows(ui, x, cy, w, &row);
            cy += dns_card(app, s, ui, x, cy, w);
            cy += ending(ui, (x, cy, w), s.back, s.wallet.is_some());
        }
        sstep::CHECK => {
            cy += check_card(app, s, ui, x, cy, w);
            if s.check.is_some() {
                cy += ending(ui, (x, cy, w), s.back, true);
            }
        }
        _ => {
            ui.tag(x, cy, EN.silent_secret_shows, Status::Warning);
            cy += 38.0;
            let mut row: Vec<(String, Style, Action)> = Vec::new();
            match app.vaults.open.get(app.vaults.current) {
                Some(v) => row.push((
                    fill(EN.silent_save_into, &[&v.name]),
                    Style::Primary,
                    Action::SScanVault,
                )),
                // Make or unlock one, and back to this step.
                None => row.extend(
                    app.vault_way(crate::Screen::Silent)
                        .map(|(label, a)| (String::from(label.as_str()), Style::Primary, a)),
                ),
            }
            row.push((
                EN.silent_out_unprotected.to_string(),
                Style::Ghost,
                Action::SScanOut,
            ));
            cy += button_rows(ui, x, cy, w, &row);
        }
    }
    cy - y
}

/// The address's DNS record: a name and a domain, and once both are
/// filled and valid the name as it reads, the record line and its QR; a
/// plain line under a field that is not a valid DNS name.
/// The end of a result: Another starts the tool again, and Done goes back
/// to where it was opened from. Done is the filled button when `last`:
/// nothing else is left to do with the result.
fn ending(ui: &mut Ui, (x, y, w): (f32, f32, f32), back: crate::Screen, last: bool) -> f32 {
    let row = vec![
        (
            EN.silent_another.to_string(),
            Style::Ghost,
            Action::SAnother,
        ),
        (
            EN.app_done.to_string(),
            if last {
                Style::Primary
            } else {
                Style::Secondary
            },
            Action::Nav(back),
        ),
    ];
    button_rows(ui, x, y, w, &row)
}

/// The address on show and the role it is drawn in: while its key made
/// here waits for its backup, the warning a receive address is held back
/// with, in its place.
fn address_on_show(app: &View) -> (String, Role) {
    if app.silent_held() {
        return (EN.glance_back_up_before.to_string(), Role::Warning);
    }
    (
        app.silent_shown()
            .address
            .map(|a| a.to_string())
            .unwrap_or_default(),
        Role::OnSurface,
    )
}

fn dns_card(app: &View, s: &SilentState, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::edit::Here;
    let Some(address) = app.silent_shown().address else {
        return 0.0;
    };
    let mut cy = y + GAP;
    let dns = s.dns(&address);
    ui.text_as(x, cy, LABEL_L, Role::OnSurface, EN.silent_dns_record);
    cy += ui.line_as(LABEL_L) + GAP_SMALL;
    for (label, text, here, bad, focus) in [
        (
            EN.silent_dns_name,
            &s.dns_name,
            Here::SilentName,
            dns.name_bad,
            s.dns_focus == Some(false),
        ),
        (
            EN.silent_dns_domain,
            &s.dns_domain,
            Here::SilentDomain,
            dns.domain_bad,
            s.dns_focus == Some(true),
        ),
    ] {
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, label);
        cy += ui.line_as(BODY_S) + GAP_SMALL;
        let error = bad.then_some(EN.silent_dns_invalid);
        let fh = ui.field_h();
        let used = ui.field_box((x, cy, w), focus, error);
        let press = Action::TypeHere(here);
        ui.press_box(x, cy, w, fh, press);
        ui.typed(
            x + FIELD_PAD,
            cy,
            fh,
            BODY_L.size,
            crate::ui::W::M,
            Role::OnSurface,
            text,
            w - FIELD_PAD * 2.0,
            focus,
            press,
        );
        cy += used + GAP;
    }
    if app.silent_held() {
        // The record holds the address: while its key waits for its
        // backup it says so, as a receive address does.
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::Warning, EN.glance_back_up_before);
    } else if let Some((human, record)) = dns.record {
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, &human) + GAP;
        cy += ui.wrap_as((x, cy, w), BODY_M.mono(), Role::OnSurface, &record)
            + faraday_draw::tokens::SILENT_LINE_GAP;
        let row = vec![(
            EN.silent_dns_record_as_qr.to_string(),
            Style::Secondary,
            Action::ShowCode(Code::SilentDns),
        )];
        cy += button_rows(ui, x, cy, w, &row);
    }
    cy - y
}

/// What a check came to, as a line and its tone.
fn checked_text(r: &crate::silent::check::Checked) -> (String, crate::ui::Paint) {
    use crate::silent::check::Checked;
    match r {
        Checked::Paid(paid) => {
            let n = paid.len();
            (
                fill(
                    EN.silent_pays_this_wallet,
                    &[
                        &n.to_string(),
                        (if n == 1 {
                            EN.silent_output
                        } else {
                            EN.silent_outputs
                        }),
                    ],
                ),
                Role::Ok.paint(),
            )
        }
        Checked::NotPaid => (
            EN.silent_pays_nothing.to_string(),
            Role::OnSurfaceVariant.paint(),
        ),
        Checked::NoInputs => (
            EN.silent_spends_nothing.to_string(),
            Role::OnSurfaceVariant.paint(),
        ),
        Checked::Unsigned => (
            EN.silent_input_key_missing.to_string(),
            Role::Warning.paint(),
        ),
        Checked::KeyNotLoaded => (EN.silent_key_not_loaded.to_string(), Role::Warning.paint()),
    }
}

/// Check a payment: the transactions in Files to check, and what the one
/// checked came to, output by output.
fn check_card(app: &View, s: &SilentState, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::silent::check::Checked;
    let mut cy = y;
    if let Some((name, check)) = s.check.as_ref() {
        ui.text_as(x, cy, LABEL_L, Role::OnSurface, name);
        cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GAP_SMALL;
        match check.result() {
            Some(r) => {
                let (line, tone) = checked_text(r);
                ui.text_as(x, cy, LABEL_L, tone, &line);
                cy += ui.advance_as(LABEL_L, 26.0);
                if let Checked::Paid(paid) = r {
                    for p in paid {
                        let to = match p.label {
                            None => EN.silent_the_address.to_string(),
                            Some(0) => EN.silent_change.to_string(),
                            Some(m) => fill(EN.silent_label_inline, &[&m.to_string()]),
                        };
                        let row = fill(
                            EN.silent_output_row,
                            &[
                                &p.vout.to_string(),
                                &crate::ui::btc(p.amount.to_sat()),
                                &to.to_string(),
                            ],
                        );
                        ui.text_as(x + 12.0, cy, BODY_M.mono(), Role::OnSurface, &row);
                        cy += ui.advance_as(BODY_M.mono(), 22.0);
                    }
                }
            }
            None => {
                let txid = check
                    .waiting_for()
                    .map(|t| t.to_string())
                    .unwrap_or_default();
                cy += ui.wrap_as(
                    (x, cy, w),
                    BODY_S,
                    Role::Warning,
                    &fill(EN.silent_needs_the_transaction, &[&txid.to_string()]),
                ) + 8.0;
            }
        }
        cy += 10.0;
    }
    // Every file in Files that reads as a transaction.
    let txs: Vec<(String, Style, Action)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, it)| crate::silent::check::Check::read(&it.bytes).is_some())
        .map(|(k, it)| {
            (
                fill(EN.silent_check, &[&it.name]),
                Style::Secondary,
                Action::SCheck(k),
            )
        })
        .collect();
    if txs.is_empty() {
        ui.text_as(
            x,
            cy,
            BODY_S,
            Role::OnSurfaceVariant,
            EN.silent_no_transaction,
        );
        cy += ui.advance_as(BODY_S, 28.0);
    } else {
        cy += button_rows(ui, x, cy, w, &txs);
    }
    cy - y
}

/// The account dialog (Material 3's basic dialog): a Coldcard export
/// holding a single-sig account and a silent payments account, and the
/// two to choose between, alike, so neither is the default; Cancel adds
/// neither.
pub(crate) fn account_dialog(app: &View, ui: &mut Ui, w: f32, h: f32) {
    use crate::wallet::Account;
    use faraday_draw::tokens::DIALOG_MAX;
    use faraday_draw::widgets::Icon;
    let Some(ask) = app.session.account_asked() else {
        return;
    };
    let line = fill(EN.wallet_account_line, &[&ask.name]);
    let single = fill(EN.wallet_account_single, &[&ask.single_bip]);
    let actions = [
        (
            EN.wallet_account_cancel,
            Style::Secondary,
            Action::LoadAccount(None),
        ),
        (
            single.as_str(),
            Style::Secondary,
            Action::LoadAccount(Some(Account::SingleSig)),
        ),
        (
            EN.wallet_account_silent,
            Style::Secondary,
            Action::LoadAccount(Some(Account::Silent)),
        ),
    ];
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        (
            Icon::Wallet.into(),
            Role::Primary.paint(),
            EN.wallet_account_head,
        ),
        &mut |ui, x, y, iw| ui.wrap_as((x, y, iw), BODY_M, Role::OnSurface, &line) + GAP,
        &actions,
    );
}
