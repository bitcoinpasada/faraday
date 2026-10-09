//! The vanity screen: the step-card column over [`crate::vanity`].

use opensigner_core::vanity::Dial;
use osk_bip::keys::ScriptType;

use crate::screens::next_button;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, grouped, thousands};
use crate::vanity::{VanityAction as V, VanityState, dial_name, fp, script_name, vstep};
use crate::{Action, Faraday, flow};

const TITLES: [&str; vstep::COUNT] = [
    "Key",
    "What turns",
    "Kind of address",
    "Characters",
    "Search",
];

fn va(a: V) -> Action {
    Action::Vanity(a)
}

pub(crate) fn draw(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(v) = app.vanity.as_ref() else {
        return;
    };
    let cards: Vec<flow::Card> = (0..vstep::COUNT as u8)
        .map(|s| flow::Card {
            title: TITLES[usize::from(s)].to_string(),
            summary: summary(app, v, s),
            mono: s == vstep::KEY || s == vstep::PREFIX,
            done: v.done[usize::from(s)],
            open: v.open == Some(s),
            toggle: va(V::Step(s)),
            guide: Some(guide(s).to_string()),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + 40.0,
        w: (cw - 80.0).min(900.0),
        h,
        back: Some(("Wallets", Action::Nav(v.back))),
        heading: "Vanity address",
        guided: app.guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
    };
    let scroll = v.scroll;
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.vanity.as_ref() {
                Some(v) => card(app_ref, v, ui, i as u8, x, y, w),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    if let Some(v) = app.vanity.as_mut() {
        v.scroll = next;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
}

fn guide(s: u8) -> &'static str {
    match s {
        vstep::KEY => "The key whose address is searched. Nothing about the key changes.",
        vstep::DIAL => {
            "A search tries one account number, or one passphrase, after another until the first \
             receive address begins with the characters chosen. The same key and the same number \
             give the same address on any device."
        }
        vstep::SCRIPT => "The kind of address decides the characters it starts with.",
        vstep::PREFIX => {
            "Type the characters the address should begin with, after the part every address of \
             this kind shares. Each character more takes 32 times as long to find for a bech32 \
             address, and 58 times for a legacy one."
        }
        _ => {
            "The search runs while this screen is open and stops at the first match. The time is \
             an expectation, not a promise: a find can come much sooner or later."
        }
    }
}

fn summary(app: &Faraday, v: &VanityState, s: u8) -> String {
    let net = app.session.network();
    match s {
        vstep::KEY => v.key.map(fp).unwrap_or_else(|| "Not chosen".into()),
        vstep::DIAL => dial_name(v.grind.dial).0.to_string(),
        vstep::SCRIPT => script_name(v.grind.script, net),
        vstep::PREFIX if v.grind.prefix.has_free(v.grind.script, net) => {
            v.grind.prefix.as_str().to_string()
        }
        vstep::RUN => match (&v.grind.find, v.running) {
            (Some(f), _) => f.address.as_str().to_string(),
            (None, true) => format!("{} tried", thousands(v.grind.tested)),
            (None, false) => String::new(),
        },
        _ => String::new(),
    }
}

fn card(app: &Faraday, v: &VanityState, ui: &mut Ui, s: u8, x: f32, y: f32, w: f32) -> f32 {
    let net = app.session.network();
    let mut cy = y;
    match s {
        vstep::KEY => {
            for k in &app.session.keys {
                let id = k.master.fingerprint().0;
                let on = v.key == Some(id);
                ui.checkbox(x, cy + 11.0, on, true);
                ui.text_mid(x + 30.0, cy, 40.0, 14.0, W::M, TEXT, &fp(id));
                ui.text_mid(x + 130.0, cy, 40.0, 13.0, W::R, MUTED, &k.label);
                ui.hit(x, cy, w, 40.0, va(V::Key(id)));
                cy += 42.0;
            }
        }
        vstep::DIAL => {
            let words = app.vanity_has_words();
            for (i, d) in Dial::ALL.iter().enumerate() {
                let (name, line) = dial_name(*d);
                let on = v.grind.dial == *d;
                let usable = *d == Dial::Account || words;
                ui.checkbox(x, cy + 3.0, on, usable);
                ui.text(
                    x + 30.0,
                    cy,
                    14.0,
                    W::S,
                    if usable { TEXT } else { DIM },
                    name,
                );
                let used = ui.wrap(x + 30.0, cy + 22.0, w - 30.0, 12.0, W::R, MUTED, line);
                if usable {
                    ui.hit(x, cy, w, used + 26.0, va(V::Dial(i as u8)));
                }
                cy += used + 34.0;
            }
            if next_button(ui, x, cy, w, "Continue", va(V::Next(vstep::DIAL))) {
                cy += 52.0;
            }
        }
        vstep::SCRIPT => {
            for (i, sc) in ScriptType::ALL.iter().enumerate() {
                let on = v.grind.script == *sc;
                ui.checkbox(x, cy + 11.0, on, true);
                ui.text_mid(x + 30.0, cy, 40.0, 14.0, W::S, TEXT, &script_name(*sc, net));
                ui.hit(x, cy, w, 40.0, va(V::Script(i as u8)));
                cy += 42.0;
            }
            cy += 8.0;
            if next_button(ui, x, cy, w, "Continue", va(V::Next(vstep::SCRIPT))) {
                cy += 52.0;
            }
        }
        vstep::PREFIX => {
            let fixed = v.grind.prefix.fixed_len(v.grind.script, net);
            let text = v.grind.prefix.as_str();
            ui.fill(x, cy, w, 46.0, 8.0, BG);
            ui.stroke(x, cy, w, 46.0, 8.0, ACCENT);
            let fw = ui.text_mid(x + 14.0, cy, 46.0, 18.0, W::M, DIM, &text[..fixed]);
            let tw = ui.text_mid(x + 14.0 + fw, cy, 46.0, 18.0, W::M, TEXT, &text[fixed..]);
            ui.caret(x + 16.0 + fw + tw, cy + 12.0, 22.0);
            cy += 56.0;
            let expected = v.grind.expected(net);
            if v.grind.prefix.has_free(v.grind.script, net) {
                ui.text(
                    x,
                    cy,
                    13.0,
                    W::R,
                    MUTED,
                    &format!("About {} tries", thousands(expected)),
                );
                cy += 26.0;
            }
            if let Some(e) = &v.error {
                cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 8.0;
            }
            if next_button(ui, x, cy, w, "Continue", va(V::Next(vstep::PREFIX))) {
                cy += 52.0;
            }
        }
        _ => match (&v.grind.find, v.running) {
            (Some(f), _) => {
                ui.text(x, cy, 12.0, W::R, MUTED, "Found");
                cy += 22.0;
                cy += ui.wrap(x, cy, w, 15.0, W::M, OK, &grouped(f.address.as_str())) + 12.0;
                match v.grind.dial {
                    Dial::Account => {
                        ui.text(
                            x,
                            cy,
                            13.0,
                            W::R,
                            TEXT,
                            &format!(
                                "Account {} of this key, after {} tries",
                                f.account,
                                thousands(v.grind.tested)
                            ),
                        );
                        cy += 30.0;
                    }
                    Dial::Passphrase => {
                        let shown = if v.shown {
                            f.suffix.as_str().to_string()
                        } else {
                            "•".repeat(f.suffix.as_str().len())
                        };
                        ui.text_mid(x, cy, 36.0, 13.0, W::R, TEXT, "Added to the passphrase:");
                        let lw = ui.measure(13.0, W::R, "Added to the passphrase:");
                        ui.text_mid(x + lw + 10.0, cy, 36.0, 15.0, W::M, TEXT, &shown);
                        ui.button(
                            x + w - 90.0,
                            cy,
                            Some(90.0),
                            36.0,
                            if v.shown { "Hide" } else { "Show" },
                            Style::Secondary,
                            va(V::Show),
                        );
                        cy += 44.0;
                        cy += ui.wrap(
                            x,
                            cy,
                            w,
                            13.0,
                            W::R,
                            WARN,
                            "These characters are part of the passphrase: write them down \
                                 with it. The words with any other passphrase open another \
                                 wallet.",
                        ) + 12.0;
                    }
                }
                let bw = ui.button(
                    x,
                    cy,
                    None,
                    40.0,
                    "Open this wallet",
                    Style::Primary,
                    va(V::Use),
                );
                ui.button(
                    x + bw + 8.0,
                    cy,
                    None,
                    40.0,
                    "Search again",
                    Style::Secondary,
                    va(V::Start),
                );
                cy += 52.0;
            }
            (None, true) => {
                let rate = v
                    .grind
                    .per_second()
                    .map(|r| format!("{} a second", thousands(r as u64)))
                    .unwrap_or_else(|| "Measuring".into());
                let left = v
                    .grind
                    .expected_seconds(net)
                    .map(|s| format!(" · about {} expected", seconds_text(s)))
                    .unwrap_or_default();
                ui.text(
                    x,
                    cy,
                    14.0,
                    W::S,
                    ACCENT,
                    &format!("Searching for {}", v.grind.prefix.as_str()),
                );
                cy += 26.0;
                ui.text(
                    x,
                    cy,
                    13.0,
                    W::R,
                    MUTED,
                    &format!("{} tried · {rate}{left}", thousands(v.grind.tested)),
                );
                cy += 30.0;
                ui.button(x, cy, None, 40.0, "Stop", Style::Secondary, va(V::Stop));
                cy += 52.0;
            }
            (None, false) => {
                if let Some(e) = &v.error {
                    cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 8.0;
                }
                ui.button(
                    x,
                    cy,
                    None,
                    40.0,
                    &format!("Search for {}", v.grind.prefix.as_str()),
                    Style::Primary,
                    va(V::Start),
                );
                cy += 52.0;
            }
        },
    }
    cy - y
}

fn seconds_text(s: u64) -> String {
    match s {
        0..=59 => format!("{s} s"),
        60..=3599 => format!("{} min", s / 60),
        3600..=86_399 => format!("{} h", s / 3600),
        _ => format!("{} days", s / 86_400),
    }
}
