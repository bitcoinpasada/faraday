//! The vanity screen: the step-card column over [`crate::vanity`].

use crate::grind::Dial;
use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::keys::ScriptType;
use faraday_draw::tokens::{BODY_L, BODY_M, BODY_S, BUTTON_H, LABEL_L};

use crate::drawing::{Scrolled, View};
use crate::screens::next_button;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, grouped, thousands};
use crate::vanity::{VanityAction as V, VanityState, dial_name, fp, script_name, vstep};
use crate::{Action, flow};

pub(crate) const TITLES: [&str; vstep::COUNT] = [
    EN.vanity_key,
    EN.vanity_what_turns,
    EN.vanity_kind_of_address,
    EN.vanity_characters,
    EN.vanity_search,
];

fn va(a: V) -> Action {
    Action::Vanity(a)
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(v) = app.vanity.state() else {
        return;
    };
    let cards: Vec<flow::Card> = (0..vstep::COUNT as u8)
        .map(|s| flow::Card {
            title: TITLES[usize::from(s)].to_string(),
            summary: summary(app, v, s),
            mono: s == vstep::KEY || s == vstep::PREFIX,
            done: v.done[usize::from(s)],
            open: v.open == Some(s),
            default: false,
            toggle: va(V::Step(s)),
            guide: Some(guide(s).to_string()),
            learn: matches!(s, vstep::KEY | vstep::DIAL).then_some(crate::learn::Topic::Xpubs),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + 40.0,
        w: (cw - 80.0).min(900.0),
        h,
        back: Some(app.named_back((EN.vanity_wallets, Action::Nav(v.back)))),
        heading: EN.vanity_vanity_address,
        guided: app.guided(),
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
        cancel: false,
    };
    let scroll = v.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.vanity.state() {
                Some(v) => card(app_ref, v, ui, i as u8, x, y, w),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::Vanity, next, again);
}

fn guide(s: u8) -> &'static str {
    match s {
        vstep::KEY => EN.vanity_the_key_whose,
        vstep::DIAL => EN.vanity_a_search_tries,
        vstep::SCRIPT => EN.vanity_guide_kind,
        vstep::PREFIX => EN.vanity_type_the_characters,
        _ => EN.vanity_the_search_runs,
    }
}

fn summary(app: &View, v: &VanityState, s: u8) -> String {
    let net = app.session.network();
    match s {
        vstep::KEY => v.key.map(fp).unwrap_or_else(|| EN.vanity_not_chosen.into()),
        vstep::DIAL => dial_name(v.grind.dial).0.to_string(),
        vstep::SCRIPT => script_name(v.grind.script, net),
        vstep::PREFIX if v.grind.prefix.has_free(v.grind.script, net) => {
            v.grind.prefix.as_str().to_string()
        }
        vstep::RUN => match (&v.grind.find, v.running) {
            (Some(f), _) => f.address.as_str().to_string(),
            (None, true) => fill(
                EN.vanity_tried_count,
                &[&thousands(v.grind.tested).to_string()],
            ),
            (None, false) => String::new(),
        },
        _ => String::new(),
    }
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn card(app: &View, v: &VanityState, ui: &mut Ui, s: u8, x: f32, y: f32, w: f32) -> f32 {
    let net = app.session.network();
    let mut cy = y;
    match s {
        vstep::KEY => {
            for k in &app.session.keys {
                let id = k.fingerprint.0;
                let on = v.key == Some(id);
                ui.checkbox(x, cy + 11.0, on, true);
                ui.text_mid_as(
                    (x + 30.0, cy, 40.0),
                    BODY_M.mono(),
                    Role::OnSurface,
                    &fp(id),
                );
                ui.text_mid_as(
                    (x + 130.0, cy, 40.0),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    &k.label,
                );
                ui.hit(x, cy, w, 40.0, va(V::Key(id)));
                cy += 42.0;
            }
        }
        vstep::DIAL => {
            // The demo dims the passphrase dial (`docs/DEMO.md` §4).
            let words = app.vanity_has_words() && app.makes_secrets();
            for (i, d) in Dial::ALL.iter().enumerate() {
                let (name, line) = dial_name(*d);
                let on = v.grind.dial == *d;
                let usable = *d == Dial::Account || words;
                ui.checkbox(x, cy + 3.0, on, usable);
                ui.text_as(
                    x + 30.0,
                    cy,
                    LABEL_L,
                    if usable {
                        Role::OnSurface.paint()
                    } else {
                        crate::ui::off()
                    },
                    name,
                );
                let used = ui.wrap_as(
                    (x + 30.0, cy + 22.0, w - 30.0),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    line,
                );
                if usable {
                    ui.hit(x, cy, w, used + 26.0, va(V::Dial(i as u8)));
                }
                cy += used + 34.0;
            }
            // A dial pressed moves on (F103).
        }
        vstep::SCRIPT => {
            for (i, sc) in ScriptType::ALL.iter().enumerate() {
                let on = v.grind.script == *sc;
                ui.checkbox(x, cy + 11.0, on, true);
                ui.text_mid_as(
                    (x + 30.0, cy, 40.0),
                    LABEL_L,
                    Role::OnSurface,
                    &script_name(*sc, net),
                );
                ui.hit(x, cy, w, 40.0, va(V::Script(i as u8)));
                cy += 42.0;
            }
            // A kind of address pressed moves on (F103).
            cy += 8.0;
        }
        vstep::PREFIX => {
            let fixed = v.grind.prefix.fixed_len(v.grind.script, net);
            let text = v.grind.prefix.as_str();
            ui.fill(x, cy, w, 46.0, 8.0, Role::SurfaceContainerHighest);
            ui.stroke(x, cy, w, 46.0, 8.0, Role::Primary);
            let fw = ui.text_mid_as(
                (x + 14.0, cy, 46.0),
                BODY_L.mono(),
                Role::OnSurfaceVariant,
                &text[..fixed],
            );
            let tw = ui.text_mid_as(
                (x + 14.0 + fw, cy, 46.0),
                BODY_L.mono(),
                Role::OnSurface,
                &text[fixed..],
            );
            ui.caret(x + 16.0 + fw + tw, cy + 12.0, 22.0);
            cy += 56.0;
            let expected = v.grind.expected(net);
            if v.grind.prefix.has_free(v.grind.script, net) {
                ui.text_as(
                    x,
                    cy,
                    BODY_S,
                    Role::OnSurfaceVariant,
                    &fill(EN.vanity_about_tries, &[&thousands(expected).to_string()]),
                );
                cy += 26.0;
            }
            if let Some(e) = &v.error {
                cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, e) + 8.0;
            }
            if next_button(ui, x, cy, w, EN.vanity_continue, va(V::Next(vstep::PREFIX))) {
                cy += 52.0;
            }
        }
        _ => match (&v.grind.find, v.running) {
            (Some(f), _) => {
                ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, EN.vanity_found);
                cy += 22.0;
                cy += ui.wrap_as(
                    (x, cy, w),
                    BODY_L.mono(),
                    Role::Ok,
                    &grouped(f.address.as_str()),
                ) + 12.0;
                match v.grind.dial {
                    Dial::Account => {
                        ui.text_as(
                            x,
                            cy,
                            BODY_S,
                            Role::OnSurface,
                            &fill(
                                EN.vanity_found_account,
                                &[
                                    &f.account.to_string(),
                                    &thousands(v.grind.tested).to_string(),
                                ],
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
                        ui.text_mid_as(
                            (x, cy, 36.0),
                            BODY_S,
                            Role::OnSurface,
                            EN.vanity_added_to_passphrase,
                        );
                        let lw = ui.measure_as(BODY_S, EN.vanity_added_to_passphrase);
                        ui.text_mid_as(
                            (x + lw + 10.0, cy, 36.0),
                            BODY_L.mono(),
                            Role::OnSurface,
                            &shown,
                        );
                        ui.button(
                            x + w - 90.0,
                            cy,
                            Some(90.0),
                            BUTTON_H,
                            if v.shown {
                                EN.vanity_hide
                            } else {
                                EN.vanity_show
                            },
                            Style::Secondary,
                            va(V::Show),
                        );
                        cy += 44.0;
                        cy += ui.wrap_as(
                            (x, cy, w),
                            BODY_S,
                            Role::Warning,
                            EN.vanity_these_characters,
                        ) + 12.0;
                    }
                }
                let bw = ui.button(
                    x,
                    cy,
                    None,
                    BUTTON_H,
                    EN.vanity_open_this_wallet,
                    Style::Primary,
                    va(V::Use),
                );
                ui.button(
                    x + bw + 8.0,
                    cy,
                    None,
                    BUTTON_H,
                    EN.vanity_search_again,
                    Style::Secondary,
                    va(V::Start),
                );
                cy += 52.0;
            }
            (None, true) => {
                let rate = v
                    .grind
                    .per_second()
                    .map(|r| fill(EN.vanity_a_second, &[&thousands(r as u64).to_string()]))
                    .unwrap_or_else(|| EN.vanity_measuring.into());
                let left = v
                    .grind
                    .expected_seconds(net)
                    .map(|s| format!(" {}", fill(EN.vanity_about_expected, &[&seconds_text(s)])))
                    .unwrap_or_default();
                ui.text_as(
                    x,
                    cy,
                    LABEL_L,
                    Role::Primary,
                    &fill(EN.vanity_searching_prefix, &[v.grind.prefix.as_str()]),
                );
                cy += 26.0;
                ui.text_as(
                    x,
                    cy,
                    BODY_S,
                    Role::OnSurfaceVariant,
                    &fill(
                        EN.vanity_tried,
                        &[
                            &thousands(v.grind.tested).to_string(),
                            &rate.to_string(),
                            &left.to_string(),
                        ],
                    ),
                );
                cy += 30.0;
                ui.button(
                    x,
                    cy,
                    None,
                    BUTTON_H,
                    EN.vanity_stop,
                    Style::Secondary,
                    va(V::Stop),
                );
                cy += 52.0;
            }
            (None, false) => {
                if let Some(e) = &v.error {
                    cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, e) + 8.0;
                }
                ui.button(
                    x,
                    cy,
                    None,
                    BUTTON_H,
                    &fill(EN.vanity_search_prefix, &[v.grind.prefix.as_str()]),
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
        60..=3599 => fill(EN.vanity_min, &[&(s / 60).to_string()]),
        3600..=86_399 => format!("{} h", s / 3600),
        _ => fill(EN.vanity_days, &[&(s / 86_400).to_string()]),
    }
}
