//! `Screen::Message`: signing a message.

use super::parts::{file_row, made_state, next_button, pin_button};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, short};
use crate::{Action, Code, Screen, flow, guide};
use faraday_ui::parts::wrap_buttons;

pub(crate) const MSTEPS: [&str; 4] = [
    EN.app_address,
    EN.app_message,
    EN.app_format,
    EN.app_signature,
];

/// Sign a message's steps, plugged into Back, Forward and the choices
/// that move on (F103, F104). The flow's state is `state.rs`'s.
pub(crate) struct MessageSteps;

/// The one [`MessageSteps`].
pub(crate) static MESSAGE_STEPS: MessageSteps = MessageSteps;

impl crate::trail::StepFlow for MessageSteps {
    fn steps(&self, app: &crate::Faraday) -> Option<crate::trail::Steps> {
        let m = app.message.state()?;
        Some(crate::trail::Steps::new((0..4u8).collect(), m.open))
    }

    fn open(&self, _: &crate::Faraday, k: u8) -> Action {
        Action::MStep(k)
    }

    fn name(&self, _: &crate::Faraday, k: u8) -> &'static str {
        MSTEPS[usize::from(k)]
    }

    fn moves_on(&self, _: &crate::Faraday, chosen: Action) -> Option<Action> {
        matches!(chosen, Action::MFormat(_)).then_some(Action::MNext(crate::mstep::FORMAT))
    }
}

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::mstep;
    let col_x = x0 + 40.0;
    let col_w = (cw - 80.0).min(880.0);
    let Some(m) = app.message.state() else {
        return;
    };
    let address = m.wallet.and_then(|w| app.session.wallets.get(w)).map(|w| {
        if app.receive_held(w) {
            crate::glance::RECEIVE_WARNING.to_string()
        } else {
            app.address_shown(w, false, m.index)
        }
    });
    let cards: Vec<flow::Card> = (0..4u8)
        .map(|k| flow::Card {
            title: MSTEPS[k as usize].to_string(),
            summary: match k {
                mstep::ADDRESS => address
                    .as_deref()
                    .map(short)
                    .unwrap_or_else(|| EN.message_none_chosen.into()),
                mstep::TEXT => {
                    let t = m.text.lines().next().unwrap_or("");
                    if t.is_empty() {
                        EN.message_not_written.to_string()
                    } else {
                        format!("\"{t}\"")
                    }
                }
                mstep::FORMAT => (if m.bip322 { "BIP-322" } else { "BIP-137" }).to_string(),
                _ => {
                    if m.signed.is_some() {
                        EN.message_signed.to_string()
                    } else {
                        EN.message_not_signed.to_string()
                    }
                }
            },
            mono: k == mstep::ADDRESS && address.is_some(),
            done: m.done[k as usize],
            open: m.open == Some(k),
            default: false,
            toggle: Action::MStep(k),
            guide: Some(guide::message(k)),
            learn: guide::message_topic(k),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: col_x,
        w: col_w,
        h,
        back: Some(app.named_back((EN.message_wallets, Action::Nav(Screen::WalletsStart)))),
        heading: EN.message_sign_a_message,
        guided: app.guided(),
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
        cancel: app.cancel_offered().is_some(),
    };
    let scroll = m.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            message_body(app_ref, ui, i as u8, x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::Message, next, again);
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn message_body(app: &View, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    use crate::mstep;
    let m = app.message.state().expect("message");
    let mut cy = y;
    match n {
        mstep::ADDRESS => {
            let mut bx = x;
            for wi in app.session.message_wallets() {
                let wl = &app.session.wallets[wi];
                let style = if m.wallet == Some(wi) {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                bx += ui.button(bx, cy, None, 36.0, &wl.name, style, Action::MWallet(wi)) + 8.0;
            }
            cy += 48.0;
            if let Some(wl) = m.wallet.and_then(|wi| app.session.wallets.get(wi)) {
                ui.text_mid(
                    x,
                    cy,
                    36.0,
                    12.0,
                    W::R,
                    Role::OnSurfaceVariant,
                    &fill(EN.message_receive_address, &[&m.index.to_string()]),
                );
                ui.button(
                    x + 170.0,
                    cy,
                    Some(40.0),
                    36.0,
                    "-",
                    Style::Secondary,
                    Action::MIndex(-1),
                );
                ui.button(
                    x + 216.0,
                    cy,
                    Some(40.0),
                    36.0,
                    "+",
                    Style::Secondary,
                    Action::MIndex(1),
                );
                cy += 46.0;
                let (a, held) = app.receive_line(wl, false, m.index);
                cy += ui.wrap(
                    x,
                    cy,
                    w,
                    14.0,
                    W::M,
                    if held { Role::Warning } else { Role::OnSurface },
                    &a,
                ) + 12.0;
            }
            if next_button(ui, x, cy, w, EN.message_continue, Action::MNext(n)) {
                cy += 48.0;
            }
        }
        mstep::TEXT => {
            let box_h = 120.0;
            ui.fill(x, cy, w, box_h, 8.0, Role::SurfaceContainerHighest);
            ui.stroke(
                x,
                cy,
                w,
                box_h,
                8.0,
                if m.typing {
                    Role::Primary.with_alpha(110)
                } else {
                    Role::OutlineVariant.paint()
                },
            );
            let shown = if m.text.is_empty() {
                if m.typing {
                    EN.message_type_the_message.to_string()
                } else {
                    EN.message_press_type.to_string()
                }
            } else if m.typing && !ui.edit.all(m.text.chars().count()) {
                format!("{}{}", m.text, ui.caret_char())
            } else {
                m.text.clone()
            };
            ui.c.push_clip(ui.rect(x, cy, w, box_h));
            if m.typing && ui.edit.all(m.text.chars().count()) {
                ui.fill(
                    x + 4.0,
                    cy + 4.0,
                    w - 8.0,
                    box_h - 8.0,
                    6.0,
                    Role::Primary.with_alpha(40),
                );
            }
            ui.wrap(
                x + 14.0,
                cy + 12.0,
                w - 28.0,
                14.0,
                W::R,
                if m.text.is_empty() {
                    Role::OnSurfaceVariant
                } else {
                    Role::OnSurface
                },
                &shown,
            );
            ui.c.pop_clip();
            cy += box_h + 12.0;
            ui.button(
                x,
                cy,
                None,
                38.0,
                if m.typing {
                    EN.message_stop_typing
                } else {
                    EN.message_type
                },
                Style::Secondary,
                Action::MType,
            );
            ui.text_mid(
                x + 120.0,
                cy,
                38.0,
                12.0,
                W::R,
                Role::OnSurfaceVariant,
                &fill(
                    EN.message_characters,
                    &[&m.text.chars().count().to_string()],
                ),
            );
            if next_button(ui, x, cy, w, EN.message_continue, Action::MNext(n)) {
                cy += 48.0;
            }
        }
        mstep::FORMAT => {
            let script = m
                .wallet
                .and_then(|wi| app.session.wallets.get(wi))
                .and_then(|wl| match crate::wallet::Kind::of(wl.policy()) {
                    crate::wallet::Kind::Single(t) => Some(t),
                    _ => None,
                });
            let allowed = script
                .map(faraday_psbt::message::Format::for_script)
                .unwrap_or(&[]);
            let mut bx = x;
            for (label, is322) in [("BIP-322", true), ("BIP-137", false)] {
                let f = if is322 {
                    faraday_psbt::message::Format::Bip322
                } else {
                    faraday_psbt::message::Format::Bip137
                };
                let style = if !allowed.contains(&f) {
                    Style::Disabled
                } else if m.bip322 == is322 {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                bx += ui.button(
                    bx,
                    cy,
                    Some(110.0),
                    38.0,
                    label,
                    style,
                    Action::MFormat(is322),
                ) + 8.0;
            }
            // A format pressed moves on (F103).
            cy += 52.0;
        }
        _ => {
            match m.signed.as_ref() {
                None => {
                    if pin_button(
                        ui,
                        x,
                        cy,
                        None,
                        40.0,
                        EN.message_sign_the_message,
                        Style::Primary,
                        Action::MSign,
                    ) {
                        cy += 52.0;
                    }
                }
                Some(sig) => {
                    ui.text(
                        x,
                        cy,
                        12.0,
                        W::R,
                        Role::OnSurfaceVariant,
                        EN.message_signature,
                    );
                    cy += 20.0;
                    cy += ui.wrap(x, cy, w, 13.0, W::M, Role::OnSurface, &sig.signature) + 14.0;
                    // Made For the stick as it was signed.
                    let file = format!(
                        "message-{}.txt",
                        &sig.address[sig.address.len().saturating_sub(6)..]
                    );
                    let (state, (label, style, action)) =
                        made_state(app, &file, None, (EN.message_make_the_file, Action::MOut));
                    let png = fill(EN.message_png, &[app.medium.for_the()]);
                    cy += file_row(
                        ui,
                        x,
                        cy,
                        w,
                        &file,
                        EN.message_signed_message,
                        state.as_deref(),
                        (label, style, action),
                        &[],
                    ) + 6.0;
                    cy += wrap_buttons(
                        ui,
                        x,
                        cy,
                        w,
                        38.0,
                        &[
                            (EN.message_show_as_qr, Style::Secondary, Action::MQr),
                            (&png, Style::Secondary, Action::CodePng(Code::Message)),
                        ],
                    ) + 4.0;
                }
            }
            if let Some(e) = &m.error {
                cy += ui.wrap(x, cy, w, 13.0, W::R, Role::Error, e) + 6.0;
            }
        }
    }
    cy - y
}
