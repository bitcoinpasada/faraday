//! `Screen::Message`: signing a message.

use super::parts::{file_row, made_state, next_button, pin_button};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, short};
use crate::visible::first_line;
use crate::{Action, Code, Screen, flow};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, GAP, MESSAGE_FIELD_MIN_H, MESSAGE_FIELD_PAD_X, MESSAGE_FIELD_PAD_Y,
    MESSAGE_SHOWN_GAP, TITLE_S,
};
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
            String::from(&*app.address_shown(w, false, m.index))
        }
    });
    let cards: Vec<flow::Card> = (0..4u8)
        .map(|k| flow::Card {
            title: MSTEPS[k as usize].to_string(),
            summary: match k {
                mstep::ADDRESS => address
                    .as_deref()
                    .map(|a| String::from(&*short(a)))
                    .unwrap_or_else(|| EN.message_none_chosen.into()),
                mstep::TEXT => {
                    let t = first_line(&m.text);
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
            lead: None,
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
        wrap_closed: false,
        total_fixed: true,
        note: None,
        chip: None,
        chip_tap: None,
        moves: crate::ui::Moves::No,
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

fn message_body(app: &View, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    use crate::mstep;
    let m = app.message.state().expect("message");
    let mut cy = y;
    match n {
        mstep::ADDRESS => {
            let mut bx = x;
            for wi in app.session.message_wallets() {
                let wl = &app.session.wallets[wi];
                let style = Style::chosen(m.wallet == Some(wi));
                bx += ui.button(bx, cy, None, BUTTON_H, &wl.name, style, Action::MWallet(wi)) + 8.0;
            }
            cy += 48.0;
            if let Some(wl) = m.wallet.and_then(|wi| app.session.wallets.get(wi)) {
                ui.text_mid_as(
                    (x, cy, 36.0),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    &fill(EN.message_receive_address, &[&m.index.to_string()]),
                );
                ui.button(
                    x + 170.0,
                    cy,
                    Some(40.0),
                    BUTTON_H,
                    "-",
                    Style::Secondary,
                    Action::MIndex(-1),
                );
                ui.button(
                    x + 216.0,
                    cy,
                    Some(40.0),
                    BUTTON_H,
                    "+",
                    Style::Secondary,
                    Action::MIndex(1),
                );
                cy += 46.0;
                let (a, held) = app.receive_line(wl, false, m.index);
                cy += ui.wrap_as(
                    (x, cy, w),
                    BODY_M.mono(),
                    if held { Role::Warning } else { Role::OnSurface },
                    &a,
                ) + 12.0;
            }
            if next_button(ui, x, cy, w, EN.message_continue, Action::MNext(n)) {
                cy += 48.0;
            }
        }
        mstep::TEXT => {
            // The field is as high as the message, its hidden characters
            // drawn as marks, so no line of it is out of sight.
            let shown = app.message.shown().text(&m.text);
            let room_w = w - 2.0 * MESSAGE_FIELD_PAD_X;
            let laid = kept_lines(app, ui, &m.text, &shown, room_w);
            let box_h = if m.text.is_empty() {
                MESSAGE_FIELD_MIN_H
            } else {
                (laid.h + 2.0 * MESSAGE_FIELD_PAD_Y).max(MESSAGE_FIELD_MIN_H)
            };
            // Material's outlined text field: its edge in outline, in
            // primary and thicker while typing goes to it.
            use faraday_draw::tokens::{FIELD_EDGE, FIELD_EDGE_ACTIVE, FIELD_RADIUS};
            let (edge, width) = if m.typing {
                (Role::Primary, FIELD_EDGE_ACTIVE)
            } else {
                (Role::Outline, FIELD_EDGE)
            };
            let field = (x, cy, w, box_h);
            crate::menu::outline(ui, field, FIELD_RADIUS, width, edge.paint());
            // A press on the field places the caret, or starts typing
            // (`docs/TEXT-CURSOR.md` §3).
            let here = Action::TypeHere(crate::edit::Here::Message);
            ui.press_box(x, cy, w, box_h, here);
            ui.c.push_clip(ui.rect(x, cy, w, box_h));
            let room = (x + MESSAGE_FIELD_PAD_X, cy + MESSAGE_FIELD_PAD_Y, room_w);
            if m.text.is_empty() {
                let hint = if m.typing {
                    EN.message_type_the_message
                } else {
                    EN.message_press_type
                };
                ui.wrap_as(room, BODY_M, Role::OnSurfaceVariant, hint);
                if m.typing {
                    let h = BODY_M.size * ui.text_factor();
                    ui.caret(room.0, room.1 + (ui.line_as(BODY_M) - h) / 2.0, h);
                }
            } else {
                ui.draw_typed((room.0, room.1), Role::OnSurface, &laid, (m.typing, here));
            }
            ui.c.pop_clip();
            cy += box_h + 12.0;
            let type_w = ui.button(
                x,
                cy,
                None,
                BUTTON_H,
                if m.typing {
                    EN.message_stop_typing
                } else {
                    EN.message_type
                },
                Style::Secondary,
                Action::MType,
            );
            let scan_w = ui.button(
                x + type_w + GAP,
                cy,
                None,
                BUTTON_H,
                EN.message_scan,
                Style::Secondary,
                Action::MScan,
            );
            let used = type_w + GAP + scan_w + 12.0;
            let count = fill(
                EN.message_characters,
                &[&m.text.chars().count().to_string()],
            );
            ui.text_mid_as((x + used, cy, 38.0), BODY_S, Role::OnSurfaceVariant, &count);
            // Continue goes on a row of its own where the count and it
            // do not share the line.
            let next_w = ui.measure_as(TITLE_S, EN.message_continue) + 36.0;
            let shared = used + ui.measure_as(BODY_S, &count) + 12.0 + next_w <= w;
            if !shared || ui.pinning {
                cy += 48.0;
            }
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
                } else {
                    Style::chosen(m.bip322 == is322)
                };
                bx += ui.button(
                    bx,
                    cy,
                    Some(110.0),
                    BUTTON_H,
                    label,
                    style,
                    Action::MFormat(is322),
                ) + 8.0;
            }
            // A format pressed moves on (F103).
            cy += 52.0;
        }
        _ => {
            // The whole message that is signed, before and after.
            ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, EN.app_message);
            cy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
            let shown = app.message.shown().text(&m.text);
            let laid = kept_lines(app, ui, &m.text, &shown, w);
            ui.draw_wrapped((x, cy), Role::OnSurface, &laid);
            cy += laid.h + MESSAGE_SHOWN_GAP;
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
                    ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, EN.message_signature);
                    cy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
                    cy += ui.wrap_as((x, cy, w), BODY_M.mono(), Role::OnSurface, &sig.signature)
                        + 14.0;
                    // Made For the stick as it was signed.
                    let file = format!(
                        "message-{}.txt",
                        &sig.address[sig.address.len().saturating_sub(6)..]
                    );
                    // The file this message put, under the name it went out as.
                    let (state, (label, style, action)) = made_state(
                        app,
                        m.out_name.as_deref().unwrap_or(""),
                        None,
                        (EN.message_make_the_file, Action::MOut),
                    );
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
                        BUTTON_H,
                        &[
                            (EN.message_show_as_qr, Style::Secondary, Action::MQr),
                            (&png, Style::Secondary, Action::CodePng(Code::Message)),
                        ],
                    ) + 4.0;
                }
            }
            if let Some(e) = &m.error {
                cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, e) + 6.0;
            }
        }
    }
    cy - y
}

/// `shown`, the visible form of the message `text`, laid out at `width`
/// in the body face, its lines kept while the message is the same.
fn kept_lines<'s>(
    app: &View,
    ui: &Ui,
    text: &str,
    shown: &'s str,
    width: f32,
) -> faraday_ui::roles::Wrapped<'s> {
    let basis = ui.wrap_basis(BODY_M, width);
    let lines = app
        .message
        .shown()
        .lines(text, basis, |v| ui.wrap_ranges(BODY_M, v, width));
    ui.wrapped_with(BODY_M, shown, width, lines)
}
