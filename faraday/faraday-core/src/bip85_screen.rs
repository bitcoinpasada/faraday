//! The BIP-85 screen: the step-card column over [`crate::bip85`].

use crate::strings::EN;
use opensigner_core::Bip85App;
use opensigner_core::strings::fill;

use crate::bip85::{APPS, Bip85State, app_name, lengths, pstep};
use crate::screens::{button_rows, next_button};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Faraday, flow};

const TITLES: [&str; pstep::COUNT] = [
    EN.bip85_key,
    EN.bip85_application,
    EN.bip85_index,
    EN.bip85_value,
];

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
            default: false,
            toggle: Action::PStep(s),
            inert: false,
            guide: Some(guide(s, b.app)),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + 40.0,
        w: (cw - 80.0).min(900.0),
        h,
        back: Some((EN.bip85_wallets, Action::Nav(b.back))),
        heading: EN.bip85_derive_with_bip,
        guided: app.guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
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
        Bip85App::Words => Some(fill(EN.bip85_words, &[&b.length.to_string()])),
        Bip85App::Hex => Some(fill(EN.bip85_bytes, &[&b.length.to_string()])),
        Bip85App::Base64 | Bip85App::Base85 => {
            Some(fill(EN.bip85_characters, &[&b.length.to_string()]))
        }
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
            None => EN.bip85_not_chosen.to_string(),
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
        pstep::KEY => EN.bip85_bip_85_turns.to_string(),
        pstep::APP => match app {
            Bip85App::Words => EN.bip85_a_child_seed_is.to_string(),
            Bip85App::Base64 | Bip85App::Base85 => EN.bip85_a_password_for.to_string(),
            _ => EN.bip85_the_value_is_made.to_string(),
        },
        pstep::INDEX => EN.bip85_index_0_is_the.to_string(),
        _ => EN.bip85_the_value_is.to_string(),
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
                    let fp = k.fingerprint();
                    (
                        crate::wallet::key_line(fp, &k.label),
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
                // On a small panel + sits at the row's end and the range
                // goes under the row.
                let px = if ui.compact {
                    x + w - 44.0
                } else {
                    x + mw + 170.0
                };
                ui.button(
                    px,
                    cy,
                    Some(44.0),
                    40.0,
                    "+",
                    Style::Secondary,
                    Action::PLength(1),
                );
                let range = fill(EN.bip85_to, &[&lo.to_string(), &hi.to_string()]);
                if ui.compact {
                    cy += 46.0;
                    ui.text(x, cy, 12.0, W::R, DIM, &range);
                    cy += 28.0;
                } else {
                    ui.text_mid(x + mw + 230.0, cy, 40.0, 12.0, W::R, DIM, &range);
                    cy += 52.0;
                }
            }
            if next_button(ui, x, cy, w, EN.bip85_continue, Action::PNext) {
                cy += 48.0;
            }
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
            let index = format!("{}{}", b.index, ui.caret_char());
            ui.text_mid(x + mw + 20.0, cy, 44.0, 16.0, W::M, TEXT, &index);
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
                ui.text(x, cy, 13.0, W::R, ERR, EN.bip85_an_index_is_0);
                cy += 24.0;
            }
            ui.text(x, cy, 12.0, W::R, DIM, EN.bip85_type_the_digits);
            cy += 24.0;
            if next_button(ui, x, cy, w, EN.bip85_continue, Action::PNext) {
                cy += 48.0;
            }
        }
        _ => {
            // On a small panel each value goes under its name.
            let (vx, below) = if ui.compact {
                (0.0, 18.0)
            } else {
                (120.0, 0.0)
            };
            ui.text(x, cy, 12.0, W::R, MUTED, EN.bip85_path);
            cy += below;
            cy += ui
                .wrap(x + vx, cy, w - vx, 13.0, W::M, TEXT, &b.path())
                .max(20.0)
                + 6.0;
            ui.text(x, cy, 12.0, W::R, MUTED, app_name(b.app));
            cy += below;
            // Derived for this frame only, and wiped with it.
            let value = if b.shown { app.bip85_value() } else { None };
            let shown =
                value.as_ref().map_or("••••••••••••", |v| v.as_str());
            cy += ui
                .wrap(x + vx, cy, w - vx, 14.0, W::M, TEXT, shown)
                .max(20.0)
                + 12.0;
            let mut row: Vec<(String, Style, Action)> = vec![(
                if b.shown {
                    EN.bip85_hide
                } else {
                    EN.bip85_show
                }
                .to_string(),
                Style::Secondary,
                Action::PShow,
            )];
            match app.vaults.open.get(app.vaults.current) {
                Some(v) => row.push((
                    fill(EN.bip85_save_into, &[&v.name.to_string()]),
                    Style::Primary,
                    Action::PVault,
                )),
                // Make or unlock one, and back to this value.
                None => row.extend(
                    app.vault_way(crate::Screen::Bip85)
                        .map(|(label, a)| (label, Style::Primary, a)),
                ),
            }
            if b.app == Bip85App::Words {
                row.push((
                    EN.bip85_load_as_a_key.to_string(),
                    Style::Secondary,
                    Action::PLoad,
                ));
            }
            row.push((
                EN.bip85_out_unprotected.to_string(),
                Style::Ghost,
                Action::POut,
            ));
            cy += button_rows(ui, x, cy, w, &row);
        }
    }
    cy - y
}
