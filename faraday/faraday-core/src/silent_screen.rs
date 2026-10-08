//! The Silent payments screen: the step-card column over
//! [`crate::silent`].

use crate::screens::{button_rows, next_button};
use crate::silent::{SilentState, sstep};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Faraday, flow};

const TITLES: [&str; sstep::COUNT] = ["Key", "Address", "Scan key", "Check a payment"];

pub(crate) fn draw(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.silent.as_ref() else {
        return;
    };
    let cards: Vec<flow::Card> = (0..sstep::COUNT as u8)
        .map(|k| flow::Card {
            title: TITLES[usize::from(k)].to_string(),
            summary: summary(app, s, k),
            mono: k == sstep::KEY || k == sstep::ADDRESS,
            done: s.done[usize::from(k)],
            open: s.open == Some(k),
            toggle: Action::SStep(k),
            guide: Some(guide(k)),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + 40.0,
        w: (cw - 80.0).min(900.0),
        h,
        back: Some(("Wallets", Action::Nav(s.back))),
        heading: "Silent payments",
        guided: app.guided,
        switch: true,
        note: None,
    };
    let scroll = s.scroll;
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.silent.as_ref() {
                Some(s) => card(app_ref, s, ui, i as u8, x, y, w),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    if let Some(s) = app.silent.as_mut() {
        s.scroll = next;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
}

fn summary(app: &Faraday, s: &SilentState, k: u8) -> String {
    match k {
        sstep::KEY => s
            .key
            .map(|fp| fp_text(osk_bip::keys::Fingerprint(fp)))
            .unwrap_or_else(|| "Not chosen".to_string()),
        sstep::ADDRESS => app
            .silent_address()
            .map(|a| crate::ui::short(&a))
            .unwrap_or_default(),
        sstep::SCAN => "A secret: shows every payment".to_string(),
        _ => match s.check.as_ref().map(|(_, c)| c.result()) {
            Some(Some(r)) => checked_text(r).0,
            Some(None) => "Needs a previous transaction".to_string(),
            None => "A transaction from Files".to_string(),
        },
    }
}

fn guide(k: u8) -> String {
    match k {
        sstep::KEY => "A silent payments address is one address you can give out many times: each payment \
            to it lands at an address nobody else can link to it or to the others."
            .to_string(),
        sstep::ADDRESS => "Give this address to whoever pays you. A label makes another address of the same \
            wallet, so you can tell who paid; the payer cannot tell the two apart. The record lists the \
            labels handed out, for the wallet software that watches for payments."
            .to_string(),
        sstep::SCAN => "Finding payments needs the scan key: it shows every payment to this address, \
            though it cannot spend them. It goes into the vault, or to a scanning wallet you run yourself."
            .to_string(),
        _ => "A payment to a silent payments address goes to an address only this key can find. Copy \
            the transaction into Files, signed, with the transactions it spends from; this works out \
            whether any of its outputs pay this wallet, and to which label."
            .to_string(),
    }
}

fn card(app: &Faraday, s: &SilentState, ui: &mut Ui, k: u8, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    match k {
        sstep::KEY => {
            let row: Vec<(String, Style, Action)> = app
                .session
                .keys
                .iter()
                .map(|key| {
                    let fp = key.master.fingerprint();
                    (
                        format!("{} · {}", fp_text(fp), key.label),
                        if s.key == Some(fp.0) {
                            Style::Primary
                        } else {
                            Style::Secondary
                        },
                        Action::SKey(fp.0),
                    )
                })
                .collect();
            cy += button_rows(ui, x, cy, w, &row);
        }
        sstep::ADDRESS if !app.silent_key_here() && s.wallet.is_some() => {
            // The wallet's own address needs no key; labels do.
            let a = app.silent_address().unwrap_or_default();
            cy += ui.wrap(x, cy, w, 14.0, W::M, TEXT, &a) + 14.0;
            cy += ui.wrap(
                x,
                cy,
                w,
                13.0,
                W::R,
                MUTED,
                "Its key is not loaded: labels, the scan key and a check need it",
            ) + 12.0;
            let row = vec![
                ("Show as QR".to_string(), Style::Primary, Action::SQr(false)),
                (
                    "Add its key".to_string(),
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
                40.0,
                "-",
                Style::Secondary,
                Action::SLabel(-1),
            );
            let label = if s.label == 0 {
                "No label".to_string()
            } else {
                format!("Label {}", s.label)
            };
            ui.text_mid(x + mw + 16.0, cy, 40.0, 15.0, W::S, TEXT, &label);
            ui.button(
                x + mw + 150.0,
                cy,
                Some(44.0),
                40.0,
                "+",
                Style::Secondary,
                Action::SLabel(1),
            );
            cy += 52.0;
            let a = app.silent_address().unwrap_or_default();
            cy += ui.wrap(x, cy, w, 14.0, W::M, TEXT, &a) + 14.0;
            let row = vec![
                ("Show as QR".to_string(), Style::Primary, Action::SQr(false)),
                (
                    "As a bitcoin: link".to_string(),
                    Style::Secondary,
                    Action::SQr(true),
                ),
                (
                    "Record to the Outbox".to_string(),
                    Style::Secondary,
                    Action::SRecord,
                ),
            ];
            let mut row = row;
            if s.wallet.is_none() {
                row.push((
                    "Add as a wallet".to_string(),
                    Style::Secondary,
                    Action::SAddWallet,
                ));
            }
            cy += button_rows(ui, x, cy, w, &row);
            if next_button(ui, x, cy, w, "Continue", Action::SNext) {
                cy += 48.0;
            }
        }
        sstep::CHECK => cy += check_card(app, s, ui, x, cy, w),
        _ => {
            ui.chip(
                x,
                cy,
                "Secret · shows every payment, cannot spend",
                WARN,
                WARN.with_alpha(30),
            );
            cy += 38.0;
            let mut row: Vec<(String, Style, Action)> = Vec::new();
            match app.vaults.open.get(app.vaults.current) {
                Some(v) => row.push((
                    format!("Save into {}", v.name),
                    Style::Primary,
                    Action::SScanVault,
                )),
                None => row.push((
                    "No vault open".to_string(),
                    Style::Disabled,
                    Action::SScanVault,
                )),
            }
            row.push((
                "Out unprotected…".to_string(),
                Style::Ghost,
                Action::SScanOut,
            ));
            cy += button_rows(ui, x, cy, w, &row);
        }
    }
    cy - y
}

/// What a check came to, as a line and its tone.
fn checked_text(r: &opensigner_core::silent::Checked) -> (String, osk_ui::Color) {
    use opensigner_core::silent::Checked;
    match r {
        Checked::Paid(paid) => {
            let n = paid.len();
            (
                format!(
                    "Pays this wallet: {n} {}",
                    if n == 1 { "output" } else { "outputs" }
                ),
                OK,
            )
        }
        Checked::NotPaid => ("Pays nothing to this wallet".to_string(), MUTED),
        Checked::NoInputs => (
            "Spends nothing a silent payment is made from".to_string(),
            MUTED,
        ),
        Checked::Unsigned => (
            "Not signed: an input's key is not in it yet".to_string(),
            WARN,
        ),
        Checked::KeyNotLoaded => ("Its key is not loaded".to_string(), WARN),
    }
}

/// Check a payment: the transactions in Files to check, and what the one
/// checked came to, output by output.
fn check_card(app: &Faraday, s: &SilentState, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use opensigner_core::silent::Checked;
    let mut cy = y;
    if let Some((name, check)) = s.check.as_ref() {
        ui.text(x, cy, 13.0, W::S, TEXT, name);
        cy += 24.0;
        match check.result() {
            Some(r) => {
                let (line, tone) = checked_text(r);
                ui.text(x, cy, 14.0, W::S, tone, &line);
                cy += 26.0;
                if let Checked::Paid(paid) = r {
                    for p in paid {
                        let to = match p.label {
                            None => "the address".to_string(),
                            Some(0) => "change".to_string(),
                            Some(m) => format!("label {m}"),
                        };
                        let row = format!(
                            "Output {} · {} · to {to}",
                            p.vout,
                            crate::ui::btc(p.amount.to_sat())
                        );
                        ui.text(x + 12.0, cy, 13.0, W::M, TEXT, &row);
                        cy += 22.0;
                    }
                }
            }
            None => {
                let txid = check
                    .waiting_for()
                    .map(|t| t.to_string())
                    .unwrap_or_default();
                cy += ui.wrap(
                    x,
                    cy,
                    w,
                    13.0,
                    W::R,
                    WARN,
                    &format!("Needs the transaction it spends from, {txid}: copy it into Files and check again"),
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
        .filter(|(_, it)| opensigner_core::silent::Check::read(&it.bytes).is_some())
        .map(|(k, it)| {
            (
                format!("Check {}", it.name),
                Style::Secondary,
                Action::SCheck(k),
            )
        })
        .collect();
    if txs.is_empty() {
        ui.text(x, cy, 13.0, W::R, DIM, "No transaction in Files");
        cy += 28.0;
    } else {
        cy += button_rows(ui, x, cy, w, &txs);
    }
    cy - y
}
