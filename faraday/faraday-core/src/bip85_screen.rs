//! The BIP-85 screen: the step-card column over [`crate::bip85`].

use opensigner_core::Bip85App;

use crate::bip85::{APPS, Bip85State, app_name, lengths, pstep};
use crate::screens::{button_rows, next_button};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Faraday, flow};

const TITLES: [&str; pstep::COUNT] = ["Key", "Application", "Index", "Value"];

pub(crate) fn draw(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(b) = app.bip85.as_ref() else {
        return;
    };
    let cards: Vec<flow::Card> = (0..pstep::COUNT as u8)
        .map(|s| flow::Card {
            title: TITLES[usize::from(s)].to_string(),
            summary: summary(app, b, s),
            mono: s == pstep::KEY || s == pstep::INDEX,
            done: b.done[usize::from(s)],
            open: b.open == Some(s),
            toggle: Action::PStep(s),
            guide: Some(guide(s, b.app)),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + 40.0,
        w: (cw - 80.0).min(900.0),
        h,
        back: Some(("Wallets", Action::Nav(b.back))),
        heading: "Derive with BIP-85",
        guided: app.guided,
        switch: true,
        note: None,
    };
    let scroll = b.scroll;
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.bip85.as_ref() {
                Some(b) => card(app_ref, b, ui, i as u8, x, y, w),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    if let Some(b) = app.bip85.as_mut() {
        b.scroll = next;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
}

fn length_label(b: &Bip85State) -> Option<String> {
    match b.app {
        Bip85App::Words => Some(format!("{} words", b.length)),
        Bip85App::Hex => Some(format!("{} bytes", b.length)),
        Bip85App::Base64 | Bip85App::Base85 => Some(format!("{} characters", b.length)),
        Bip85App::Wif | Bip85App::Xprv => None,
    }
}

fn summary(app: &Faraday, b: &Bip85State, s: u8) -> String {
    match s {
        pstep::KEY => match b.key {
            Some(fp) => {
                let label = app
                    .session
                    .key_label(osk_bip::keys::Fingerprint(fp))
                    .unwrap_or("");
                format!("{} · {label}", fp_text(osk_bip::keys::Fingerprint(fp)))
            }
            None => "Not chosen".to_string(),
        },
        pstep::APP => match length_label(b) {
            Some(l) => format!("{} · {l}", app_name(b.app)),
            None => app_name(b.app).to_string(),
        },
        pstep::INDEX => b.index.clone(),
        _ => b.path(),
    }
}

fn guide(s: u8, app: Bip85App) -> String {
    match s {
        pstep::KEY => "BIP-85 turns one key you already back up into many others: child seeds for other \
            wallets, passwords, keys. Each comes back from the same key, application and index, so the \
            key's backup is theirs too."
            .to_string(),
        pstep::APP => match app {
            Bip85App::Words => "A child seed is a key of its own, for another wallet or another device. \
                It reveals nothing about this key."
                .to_string(),
            Bip85App::Base64 | Bip85App::Base85 => "A password for a website or a program, as long as \
                it allows. Keep it in a vault entry; the index makes the next one."
                .to_string(),
            _ => "The value is made the way BIP-85 specifies, so any implementation gives the same one \
                from the same key."
                .to_string(),
        },
        pstep::INDEX => "Index 0 is the first of its kind; each number gives another. Write down the index \
            with what you use it for: it is all that is needed to make the value again."
            .to_string(),
        _ => "The value is a secret. It goes into the open vault, sealed under its passphrase; out of \
            the vault it goes only after the warning."
            .to_string(),
    }
}

fn card(app: &Faraday, b: &Bip85State, ui: &mut Ui, s: u8, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    match s {
        pstep::KEY => {
            let row: Vec<(String, Style, Action)> = app
                .session
                .keys
                .iter()
                .map(|k| {
                    let fp = k.master.fingerprint();
                    (
                        format!("{} · {}", fp_text(fp), k.label),
                        if b.key == Some(fp.0) {
                            Style::Primary
                        } else {
                            Style::Secondary
                        },
                        Action::PKey(fp.0),
                    )
                })
                .collect();
            cy += button_rows(ui, x, cy, w, &row);
        }
        pstep::APP => {
            let row: Vec<(String, Style, Action)> = APPS
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    (
                        app_name(*a).to_string(),
                        if *a == b.app {
                            Style::Primary
                        } else {
                            Style::Secondary
                        },
                        Action::PApp(i as u8),
                    )
                })
                .collect();
            cy += button_rows(ui, x, cy, w, &row) + 4.0;
            if let Some(l) = length_label(b) {
                let (lo, hi, _) = lengths(b.app);
                let mw = ui.button(
                    x,
                    cy,
                    Some(44.0),
                    40.0,
                    "-",
                    Style::Secondary,
                    Action::PLength(-1),
                );
                ui.text_mid(x + mw + 16.0, cy, 40.0, 15.0, W::S, TEXT, &l);
                ui.button(
                    x + mw + 170.0,
                    cy,
                    Some(44.0),
                    40.0,
                    "+",
                    Style::Secondary,
                    Action::PLength(1),
                );
                ui.text_mid(
                    x + mw + 230.0,
                    cy,
                    40.0,
                    12.0,
                    W::R,
                    DIM,
                    &format!("{lo} to {hi}"),
                );
                cy += 52.0;
            }
            next_button(ui, x, cy, w, "Continue", Action::PNext);
            cy += 48.0;
        }
        pstep::INDEX => {
            let mw = ui.button(
                x,
                cy,
                Some(44.0),
                44.0,
                "-",
                Style::Secondary,
                Action::PIndex(-1),
            );
            ui.fill(x + mw + 8.0, cy, 180.0, 44.0, 8.0, BG);
            ui.stroke(x + mw + 8.0, cy, 180.0, 44.0, 8.0, ACCENT);
            ui.text_mid(
                x + mw + 20.0,
                cy,
                44.0,
                16.0,
                W::M,
                TEXT,
                &format!("{}|", b.index),
            );
            ui.button(
                x + mw + 196.0,
                cy,
                Some(44.0),
                44.0,
                "+",
                Style::Secondary,
                Action::PIndex(1),
            );
            cy += 56.0;
            if b.index().is_none() {
                ui.text(x, cy, 13.0, W::R, ERR, "An index is 0 to 2147483647");
                cy += 24.0;
            }
            ui.text(x, cy, 12.0, W::R, DIM, "Type the digits · Enter continues");
            cy += 24.0;
            next_button(ui, x, cy, w, "Continue", Action::PNext);
            cy += 48.0;
        }
        _ => {
            ui.text(x, cy, 12.0, W::R, MUTED, "Path");
            ui.text(x + 120.0, cy, 13.0, W::M, TEXT, &b.path());
            cy += 26.0;
            ui.text(x, cy, 12.0, W::R, MUTED, app_name(b.app));
            // Derived for this frame only, and wiped with it.
            let value = if b.shown { app.bip85_value() } else { None };
            let shown =
                value.as_ref().map_or("••••••••••••", |v| v.as_str());
            cy += ui
                .wrap(x + 120.0, cy, w - 120.0, 14.0, W::M, TEXT, shown)
                .max(20.0)
                + 12.0;
            let mut row: Vec<(String, Style, Action)> = vec![(
                if b.shown { "Hide" } else { "Show" }.to_string(),
                Style::Secondary,
                Action::PShow,
            )];
            match app.vaults.open.get(app.vaults.current) {
                Some(v) => row.push((
                    format!("Save into {}", v.name),
                    Style::Primary,
                    Action::PVault,
                )),
                None => row.push(("No vault open".to_string(), Style::Disabled, Action::PVault)),
            }
            if b.app == Bip85App::Words {
                row.push(("Load as a key".to_string(), Style::Secondary, Action::PLoad));
            }
            row.push(("Out unprotected…".to_string(), Style::Ghost, Action::POut));
            cy += button_rows(ui, x, cy, w, &row);
        }
    }
    cy - y
}
