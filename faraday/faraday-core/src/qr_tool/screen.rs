//! The QR vault tool's screen: the step-card column over
//! [`crate::qr_tool`].

use super::{Field, Pick, QrToolAction as Q, QrToolState, qstep, state};
use crate::Action;
use crate::demo::{PassField, pass_box};
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::screens::pass_pair::pass_pair;
use crate::screens::randomness::row as own_row;
use crate::strings::{EN, fill};
use crate::ui::pal::Role;
use crate::ui::{Style, Ui};
use crate::vault::screen::secret::secret_with;
use crate::vault::screen::{secret_box, text_box};
use faraday_draw::tokens::{self, BODY_M, BODY_S, BUTTON_H, FIELD_GAP, FIELD_H, GAP};
use faraday_keys::{QR_SEAL_EXPONENTS, QrHolds, QrPassOffer, Reveal};
use zeroize::Zeroizing;

pub(crate) const TITLES: [&str; qstep::COUNT] =
    [EN.qr_tool_contents, EN.qr_tool_passphrase, EN.qr_tool_code];

fn act(a: Q) -> Action {
    Action::QrTool(a)
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = state(app) else { return };
    let opening = s.code_to_open().is_some();
    let steps = if opening { 1 } else { qstep::COUNT as u8 };
    let cards: Vec<flow::Card> = (0..steps)
        .map(|step| flow::Card {
            title: if opening {
                EN.qr_tool_open.to_string()
            } else {
                TITLES[usize::from(step)].to_string()
            },
            summary: summary(app, s, step),
            mono: false,
            done: if opening {
                s.holds().is_some()
            } else {
                s.done[usize::from(step)]
            },
            open: s.open == Some(step),
            default: false,
            toggle: act(Q::Step(step)),
            walkthrough: None,
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
        total_fixed: true,
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
    if s.code_to_open().is_some() {
        return String::new();
    }
    match step {
        qstep::CONTENTS if s.done[usize::from(step)] => match (s.pick(), s.key()) {
            (Some(Pick::Item), _) => s.item().map(|i| i.line()).unwrap_or_default(),
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
    if let Some(code) = s.code_to_open() {
        return open_card(app, s, ui, code.len(), (x, y, w));
    }
    match step {
        qstep::CONTENTS => contents(app, s, ui, (x, y, w)),
        qstep::PASSPHRASE => passphrase(app, s, ui, (x, y, w)),
        _ => code(s, ui, (x, y, w)),
    }
}

fn contents(app: &View, s: &QrToolState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    if let Some(item) = s.item() {
        return item_card(ui, item, (x, y, w));
    }
    if s.pick().is_none()
        && app
            .put_on()
            .is_some_and(|set| !set.inputs.qr_vault.is_empty())
    {
        crate::samples::put::offer(ui, act(Q::UseSample));
    }
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
            cy += FIELD_H + FIELD_GAP;
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
                    cy += FIELD_GAP;
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
        Some(Pick::Item) | None => cy -= GAP,
    }
    cy - y
}

/// The Contents card of a tool opened on a vault item: what it is and
/// where from, and that a seed's kept passphrase is left out.
fn item_card(ui: &mut Ui, item: &super::item::Source, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, &item.line());
    if item.pass_kept() {
        cy += GAP;
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.qr_tool_item_pass_kept,
        );
    }
    cy - y
}

fn passphrase(app: &View, s: &QrToolState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    cy += pass_pair(
        ui,
        (x, cy, w),
        (s.pass.text.as_str(), s.again.text.as_str()),
        |ui, n, fy| {
            let (label, field, b) = if n == 0 {
                (EN.qr_tool_pass, Field::Pass, &s.pass)
            } else {
                (EN.qr_tool_pass_again, Field::Again, &s.again)
            };
            let mut cy = fy;
            cy += ui.field_label(x, cy, label);
            pass_box(
                app,
                ui,
                (x, cy, w),
                b,
                (s.shown, s.focus() == Some(field)),
                (act(Q::Focus(field)), act(Q::Eye)),
                PassField::QrMake(n),
            );
            cy + FIELD_H + GAP - fy
        },
    );
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
    let label = app.wait_label(act(Q::Make), EN.qr_tool_make);
    ui.button(x, cy, None, BUTTON_H, label, style, act(Q::Make));
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

/// The Open card: the code brought, then the passphrase and Open, or what
/// the code holds once it is opened. A held note or passphrase is read
/// from `faraday-keys` only while its eye shows it.
fn open_card(
    app: &View,
    s: &QrToolState,
    ui: &mut Ui,
    len: usize,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let mut cy = y;
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_M,
        Role::OnSurface,
        &fill(EN.qr_tool_open_code, &[&len.to_string()]),
    ) + GAP;
    let Some(holds) = s.holds() else {
        cy += ui.field_label(x, cy, EN.qr_tool_pass);
        pass_box(
            app,
            ui,
            (x, cy, w),
            &s.pass,
            (s.shown, s.focus() == Some(Field::Pass)),
            (act(Q::Focus(Field::Pass)), act(Q::Eye)),
            PassField::QrOpen,
        );
        cy += FIELD_H + GAP;
        if let Some(e) = &s.error {
            cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, e) + GAP;
        }
        let style = Style::Primary.when(!s.pass.text.is_empty());
        ui.button(
            x,
            cy,
            None,
            BUTTON_H,
            EN.qr_tool_open,
            style,
            act(Q::OpenCode),
        );
        cy += BUTTON_H;
        return cy - y;
    };
    let keys = &app.session.keys;
    let line =
        |ui: &mut Ui, cy: f32, text: &str| ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, text);
    let eye = (act(Q::Eye), s.shown);
    match holds {
        QrHolds::Empty => cy += line(ui, cy, EN.qr_tool_open_empty),
        QrHolds::Seed { words } => {
            cy += line(ui, cy, &fill(EN.qr_tool_open_seed, &[&words.to_string()])) + GAP;
            cy += offer(ui, (x, cy, w), EN.qr_tool_load_seed, true, Q::Seed);
        }
        QrHolds::Share { key } => {
            cy += line(ui, cy, EN.qr_tool_open_share) + GAP;
            cy += line(ui, cy, &fill(EN.qr_tool_open_key, &[&key.to_string()])) + GAP;
            cy += offer(ui, (x, cy, w), EN.qr_tool_add_share, true, Q::Share);
        }
        QrHolds::Passphrase { key } => {
            cy += line(ui, cy, EN.qr_tool_open_passphrase) + GAP;
            cy += line(ui, cy, &fill(EN.qr_tool_open_key, &[&key.to_string()])) + GAP;
            let text = || Zeroizing::new(keys.qr_passphrase(Reveal::QrVault).to_string());
            cy += secret_with(ui, (x, cy, w), text, true, eye) + GAP;
            cy += pass_offer(app, s, ui, key, (x, cy, w));
        }
        QrHolds::Note => {
            cy += line(ui, cy, EN.qr_tool_open_note) + GAP;
            let text = || Zeroizing::new(keys.qr_note(Reveal::QrVault).to_string());
            cy += secret_with(ui, (x, cy, w), text, false, eye) + GAP;
            let current = app.vaults.open.get(app.vaults.current).map(|o| o.id);
            let vault = current.is_some();
            let saved = current.is_some() && s.note_saved == current;
            let label = if saved {
                EN.qr_tool_note_is_saved
            } else {
                EN.qr_tool_save_note
            };
            cy += offer(ui, (x, cy, w), label, vault && !saved, Q::SaveNote);
            if !vault {
                cy += GAP;
                cy += ui.wrap_as(
                    (x, cy, w),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.qr_tool_open_vault_first,
                );
            }
        }
    }
    if let Some(e) = &s.error {
        cy += GAP;
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, e);
    }
    cy - y
}

/// What an opened passphrase offers for the keys loaded: Add to key, already
/// here, or why not. Its height.
fn pass_offer(
    app: &View,
    s: &QrToolState,
    ui: &mut Ui,
    key: faraday_bip::keys::Fingerprint,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let fp = key.to_string();
    let mut cy = y;
    let keys = &app.session.keys;
    let line =
        |ui: &mut Ui, cy: f32, text: &str| ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, text);
    match s.pass_offer() {
        Some(QrPassOffer::Add(_)) => {
            let label = fill(EN.qr_tool_add_to_key, &[&fp]);
            cy += offer(ui, (x, cy, w), &label, true, Q::AddToKey);
        }
        Some(QrPassOffer::Loaded) => {
            cy += line(ui, cy, &fill(EN.scan_key_key_already, &[&fp]));
        }
        Some(QrPassOffer::NotLoaded) if keys.find(key).is_some() => {
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::OnSurfaceVariant,
                &fill(EN.qr_tool_pass_makes_no_loaded_key, &[&fp]),
            );
        }
        Some(QrPassOffer::NotLoaded) => {
            cy += line(ui, cy, &fill(EN.qr_tool_key_not_loaded, &[&fp])) + GAP;
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::OnSurfaceVariant,
                EN.qr_tool_load_words_first,
            );
        }
        None => {}
    }
    cy - y
}

/// An offer of the Open card: a button, dimmed when it cannot be pressed.
/// Its height.
fn offer(ui: &mut Ui, (x, y, w): (f32, f32, f32), label: &str, on: bool, a: Q) -> f32 {
    ui.button(
        x,
        y,
        Some(w),
        BUTTON_H,
        label,
        Style::Primary.when(on),
        act(a),
    );
    BUTTON_H
}
