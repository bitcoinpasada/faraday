//! The QR vault tool's screen: the step-card column over
//! [`crate::qr_tool`].

use super::{Field, Pick, QrToolAction as Q, QrToolState, qstep, state};
use crate::Action;
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::screens::randomness::row as own_row;
use crate::strings::{EN, fill};
use crate::ui::pal::Role;
use crate::ui::{Style, Ui};
use crate::vault::screen::{secret_box, text_box};
use faraday_draw::tokens::{self, BODY_S, BUTTON_H, FIELD_H, GAP};
use faraday_keys::QR_SEAL_EXPONENTS;

pub(crate) const TITLES: [&str; qstep::COUNT] =
    [EN.qr_tool_contents, EN.qr_tool_passphrase, EN.qr_tool_code];

fn act(a: Q) -> Action {
    Action::QrTool(a)
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = state(app) else { return };
    let cards: Vec<flow::Card> = (0..qstep::COUNT as u8)
        .map(|step| flow::Card {
            title: TITLES[usize::from(step)].to_string(),
            summary: summary(app, s, step),
            mono: false,
            done: s.done[usize::from(step)],
            open: s.open == Some(step),
            default: false,
            toggle: act(Q::Step(step)),
            guide: None,
            learn: None,
        })
        .collect();
    let back_name = if s.back == crate::Screen::Catalog {
        EN.qr_tool_tools
    } else {
        app.back_name(s.back)
    };
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + tokens::FLOW_INSET,
        w: (cw - tokens::FLOW_INSET - tokens::FLOW_INSET).min(tokens::FLOW_MAX),
        h,
        back: Some(app.named_back((back_name, Action::Nav(s.back)))),
        heading: EN.qr_tool_title,
        guided: app.guided(),
        switch: true,
        wrap_closed: false,
        note: None,
        chip: None,
        chip_tap: None,
        moves: crate::ui::Moves::No,
    };
    let scroll = s.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match state(app_ref) {
                Some(s) => card(app_ref, s, ui, i as u8, (x, y, w)),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::QrTool, next, again);
}

fn summary(app: &View, s: &QrToolState, step: u8) -> String {
    match step {
        qstep::CONTENTS if s.done[usize::from(step)] => match (s.pick(), s.key()) {
            (Some(Pick::Key), Some(i)) => app
                .session
                .keys
                .info(i)
                .map(|k| fill(EN.qr_tool_key_sum, &[&k.label]))
                .unwrap_or_default(),
            _ => String::new(),
        },
        qstep::PASSPHRASE if s.done[usize::from(step)] => {
            fill(EN.qr_tool_pass_sum, &[&mib(s.exponent()).to_string()])
        }
        qstep::CODE if s.done[usize::from(step)] => EN.qr_tool_code_sum.to_string(),
        _ => String::new(),
    }
}

/// The MiB an exponent stands for.
fn mib(exponent: u8) -> u32 {
    1u32 << exponent.saturating_sub(10)
}

fn card(app: &View, s: &QrToolState, ui: &mut Ui, step: u8, (x, y, w): (f32, f32, f32)) -> f32 {
    match step {
        qstep::CONTENTS => contents(app, s, ui, (x, y, w)),
        qstep::PASSPHRASE => passphrase(app, s, ui, (x, y, w)),
        _ => code(s, ui, (x, y, w)),
    }
}

fn contents(app: &View, s: &QrToolState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    for (label, pick) in [
        (EN.qr_tool_type_note, Pick::Note),
        (EN.qr_tool_loaded_key, Pick::Key),
    ] {
        ui.button(
            x,
            cy,
            Some(w),
            BUTTON_H,
            label,
            Style::chosen(s.pick() == Some(pick)),
            act(Q::Pick(pick)),
        );
        cy += BUTTON_H + GAP;
    }
    match s.pick() {
        Some(Pick::Note) => {
            cy += ui.field_label(x, cy, EN.qr_tool_note);
            let show = app.pass_shows(s.shown, s.note.text.as_str());
            secret_box(
                ui,
                x,
                cy,
                w,
                &s.note,
                show,
                s.focus() == Some(Field::Note),
                act(Q::Focus(Field::Note)),
                act(Q::Eye),
            );
            cy += FIELD_H + 4.0;
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::OnSurfaceVariant,
                &fill(EN.qr_tool_room_left, &[&s.room_left().to_string()]),
            ) + GAP;
            let style = Style::Primary.when(!s.note.text.is_empty());
            ui.button(
                x,
                cy,
                None,
                BUTTON_H,
                EN.qr_tool_continue,
                style,
                act(Q::Next),
            );
            cy += BUTTON_H;
        }
        Some(Pick::Key) => {
            let keys = &app.session.keys;
            if keys.is_empty() {
                cy += ui.wrap_as(
                    (x, cy, w),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.qr_tool_no_key,
                ) + GAP;
            }
            for i in 0..keys.len() {
                let Some(info) = keys.info(i) else { continue };
                let label = format!("{} · {}", info.label, info.fingerprint);
                let offered = keys.qr_key_offered(i);
                let style = Style::chosen(s.key() == Some(i)).when(offered);
                ui.button(x, cy, Some(w), BUTTON_H, &label, style, act(Q::Key(i)));
                cy += BUTTON_H;
                if offered {
                    cy += GAP;
                } else {
                    cy += 4.0;
                    cy += ui.wrap_as(
                        (x, cy, w),
                        BODY_S,
                        Role::OnSurfaceVariant,
                        EN.qr_tool_key_not_offered,
                    ) + GAP;
                }
            }
            cy -= GAP;
        }
        None => cy -= GAP,
    }
    cy - y
}

fn passphrase(app: &View, s: &QrToolState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    for (label, field, b) in [
        (EN.qr_tool_pass, Field::Pass, &s.pass),
        (EN.qr_tool_pass_again, Field::Again, &s.again),
    ] {
        cy += ui.field_label(x, cy, label);
        let show = app.pass_shows(s.shown, b.text.as_str());
        secret_box(
            ui,
            x,
            cy,
            w,
            b,
            show,
            s.focus() == Some(field),
            act(Q::Focus(field)),
            act(Q::Eye),
        );
        cy += FIELD_H + GAP;
    }
    if !s.again.text.is_empty() && !s.pass.text.is_empty() && !s.passphrase_ready() {
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, EN.qr_tool_pass_differ) + GAP;
    }
    cy += ui.wrap_as((x, cy, w), BODY_S, Role::Warning, EN.qr_tool_pass_warning) + GAP;
    cy += ui.field_label(x, cy, EN.qr_tool_memory);
    for e in QR_SEAL_EXPONENTS {
        ui.button(
            x,
            cy,
            Some(w),
            BUTTON_H,
            &fill(EN.qr_tool_memory_mib, &[&mib(e).to_string()]),
            Style::chosen(s.exponent() == e),
            act(Q::Memory(e)),
        );
        cy += BUTTON_H + GAP;
    }
    cy += own_row(app, ui, (x, cy, w));
    cy += ui.field_label(x, cy, EN.qr_tool_name);
    text_box(
        ui,
        x,
        cy,
        w,
        &s.name,
        false,
        s.focus() == Some(Field::Name),
        act(Q::Focus(Field::Name)),
    );
    cy += FIELD_H + GAP;
    if let Some(e) = &s.error {
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, e) + GAP;
    }
    let style = Style::Primary.when(s.passphrase_ready());
    ui.button(x, cy, None, BUTTON_H, EN.qr_tool_make, style, act(Q::Make));
    cy += BUTTON_H;
    cy - y
}

fn code(s: &QrToolState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    if !s.is_made() {
        return 0.0;
    }
    let mut cy = y;
    for (label, style, a) in [
        (EN.qr_tool_show, Style::Primary, Q::Show),
        (EN.qr_tool_png, Style::Tonal, Q::Png),
        (EN.qr_tool_file, Style::Tonal, Q::File),
        (EN.qr_tool_another, Style::Secondary, Q::Another),
    ] {
        ui.button(x, cy, Some(w), BUTTON_H, label, style, act(a));
        cy += BUTTON_H + GAP;
    }
    cy - GAP - y
}
