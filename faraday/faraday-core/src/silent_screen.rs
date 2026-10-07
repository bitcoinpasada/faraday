//! The Silent payments screen: the step-card column over
//! [`crate::silent`].

use crate::screens::{button_rows, next_button};
use crate::silent::{SilentState, sstep};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Faraday, flow};

const TITLES: [&str; sstep::COUNT] = ["Key", "Address", "Scan key"];

pub(crate) fn draw(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.silent.as_ref() else {
        return;
    };
    let cards: Vec<flow::Card> = (0..sstep::COUNT as u8)
        .map(|k| flow::Card {
            title: TITLES[usize::from(k)].to_string(),
            summary: summary(app, s, k),
            mono: k != sstep::SCAN,
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
        _ => "A secret: shows every payment".to_string(),
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
        _ => "Finding payments needs the scan key: it shows every payment to this address, though it \
            cannot spend them. It goes into the vault, or to a scanning wallet you run yourself."
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
            cy += button_rows(ui, x, cy, w, &row);
            next_button(ui, x, cy, w, "Continue", Action::SNext);
            cy += 48.0;
        }
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
