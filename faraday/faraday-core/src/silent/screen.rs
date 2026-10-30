//! The Silent payments screen: the step-card column over
//! [`crate::silent`].

use crate::drawing::{Scrolled, View};
use crate::screens::{button_rows, next_button};
use crate::silent::{RecordCheck, SilentState, sstep};
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::fp_text;
use crate::{Action, Code, flow};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, GAP, LABEL_L, PAD, TITLE_M};
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
    let cards: Vec<flow::Card> = (0..sstep::COUNT as u8)
        .map(|k| flow::Card {
            title: TITLES[usize::from(k)].to_string(),
            summary: summary(app, s, k),
            mono: k == sstep::KEY || k == sstep::ADDRESS,
            done: s.done[usize::from(k)],
            open: s.open == Some(k),
            default: false,
            toggle: Action::SStep(k),
            guide: Some(guide(k)),
            learn: Some(crate::learn::Topic::SilentPayments),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + 40.0,
        w: (cw - 80.0).min(900.0),
        h,
        back: Some(app.named_back((EN.silent_wallets, Action::Nav(s.back)))),
        heading: EN.silent_silent_payments,
        guided: app.guided(),
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
    };
    let scroll = s.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.silent.state() {
                Some(s) => card(app_ref, s, ui, i as u8, x, y, w),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::Silent, next, again);
}

fn summary(app: &View, s: &SilentState, k: u8) -> String {
    match k {
        sstep::KEY => s
            .key
            .map(|fp| fp_text(faraday_bip::keys::Fingerprint(fp)))
            .unwrap_or_else(|| EN.silent_not_chosen.to_string()),
        sstep::ADDRESS => app
            .silent_shown()
            .address
            .map(|a| crate::ui::short(&a))
            .unwrap_or_default(),
        sstep::SCAN => EN.silent_a_secret_shows.to_string(),
        _ => match s.check.as_ref().map(|(_, c)| c.result()) {
            Some(Some(r)) => checked_text(r).0,
            Some(None) => EN.silent_needs_a_previous.to_string(),
            None => EN.silent_a_transaction.to_string(),
        },
    }
}

fn guide(k: u8) -> String {
    match k {
        sstep::KEY => EN.silent_a_silent_payments_address.to_string(),
        sstep::ADDRESS => EN.silent_give_this_address.to_string(),
        sstep::SCAN => EN.silent_finding_payments.to_string(),
        _ => EN.silent_guide_payment.to_string(),
    }
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn card(app: &View, s: &SilentState, ui: &mut Ui, k: u8, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    match k {
        sstep::KEY => {
            let row: Vec<(String, Style, Action)> = app
                .session
                .keys
                .all()
                .map(|key| {
                    let fp = key.fingerprint;
                    (
                        crate::wallet::key_line(fp, &key.label),
                        Style::chosen(s.key == Some(fp.0)),
                        Action::SKey(fp.0),
                    )
                })
                .collect();
            cy += button_rows(ui, x, cy, w, &row);
        }
        // A record a loaded key contradicts: no address, no labels, no scan
        // key and no check.
        _ if app.silent_shown().check == Some(RecordCheck::Differs) => {
            let fp = s
                .key
                .map(|fp| fp_text(faraday_bip::keys::Fingerprint(fp)).to_string())
                .unwrap_or_default();
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_M,
                Role::Error,
                &fill(EN.silent_record_differs, &[&fp]),
            ) + PAD;
        }
        sstep::ADDRESS if !app.silent_shown().key_here && s.wallet.is_some() => {
            // The wallet's own address needs no key, and is not checked
            // without it; labels need it.
            let a = app.silent_shown().address.unwrap_or_default();
            cy += ui.wrap_as((x, cy, w), BODY_M.mono(), Role::OnSurface, &a) + GAP;
            let fp = s
                .key
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
            let row = vec![
                (
                    EN.silent_show_as_qr.to_string(),
                    Style::Primary,
                    Action::SQr(false),
                ),
                (
                    EN.silent_add_its_key.to_string(),
                    Style::Secondary,
                    Action::Entry(s.key),
                ),
            ];
            cy += button_rows(ui, x, cy, w, &row);
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
            let a = app.silent_shown().address.unwrap_or_default();
            cy += ui.wrap_as((x, cy, w), BODY_M.mono(), Role::OnSurface, &a) + 14.0;
            let row = vec![
                (
                    EN.silent_show_as_qr.to_string(),
                    Style::Primary,
                    Action::SQr(false),
                ),
                (
                    EN.silent_as_a_bitcoin.to_string(),
                    Style::Secondary,
                    Action::SQr(true),
                ),
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
            ];
            let mut row = row;
            if s.wallet.is_none() {
                row.push((
                    EN.silent_add_as_a_wallet.to_string(),
                    Style::Secondary,
                    Action::SAddWallet,
                ));
            }
            cy += button_rows(ui, x, cy, w, &row);
            if next_button(ui, x, cy, w, EN.silent_continue, Action::SNext) {
                cy += 48.0;
            }
        }
        sstep::CHECK => cy += check_card(app, s, ui, x, cy, w),
        _ => {
            ui.tag(x, cy, EN.silent_secret_shows, Status::Warning);
            cy += 38.0;
            let mut row: Vec<(String, Style, Action)> = Vec::new();
            match app.vaults.open.get(app.vaults.current) {
                Some(v) => row.push((
                    fill(EN.silent_save_into, &[&v.name.to_string()]),
                    Style::Primary,
                    Action::SScanVault,
                )),
                // Make or unlock one, and back to this step.
                None => row.extend(
                    app.vault_way(crate::Screen::Silent)
                        .map(|(label, a)| (label, Style::Primary, a)),
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
                                &crate::ui::btc(p.amount.to_sat()).to_string(),
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
                fill(EN.silent_check, &[&it.name.to_string()]),
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
