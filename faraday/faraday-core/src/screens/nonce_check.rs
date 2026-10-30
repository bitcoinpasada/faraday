//! `Screen::NonceCheck`: the nonce check of a signed PSBT or a finished
//! transaction brought in, to be checked rather than signed
//! (`docs/FLOWS.md`, Nonce check).
//!
//! Everything shown is worked out once when the check opens and kept on
//! the Decode area (`crate::decode::OutsideCheck`); this only draws it.

use super::decode::decode_back;
use super::parts::{made_place, spaced};
use super::spend::nonce_card;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, compact};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, GAP, PAD, SHAPE_S, SPACER};
use faraday_ui::parts::wrap_buttons;

/// The check's file, its nonce check, whether it is finished, and what
/// can be done with it: Save to a stick, Decode it, the raw hex and the
/// code.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(o) = app.outside_check() else {
        return;
    };
    // A small panel has the way back in its bar, and one column.
    let (x, w) = if ui.compact {
        (x0 + compact::M, cw - 2.0 * compact::M)
    } else {
        (x0 + SPACER + SPACER, cw - 2.0 * (SPACER + SPACER))
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    let top = if ui.compact {
        PAD
    } else {
        let back = app.named_back((decode_back(o.back), Action::Nav(o.back)));
        super::bar_head(ui, Some(back), EN.nonce_screen_title)
    } - app.list_offset;
    let mut y = top;
    y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, &o.source) + PAD;
    y += nonce_card(ui, &o.check, x, y, w);
    let wrong = o.check.wrong();
    // Why there is no finished transaction, when there is none.
    if o.finished.is_none() {
        let (line, tone) = if wrong {
            (EN.nonce_screen_refused, Role::Error)
        } else {
            (EN.nonce_screen_more_wanted, Role::OnSurfaceVariant)
        };
        y += ui.wrap_as((x, y, w), BODY_M, tone, line) + PAD;
    }
    // What is done with it, as Spend's Send it card offers: For the
    // stick, or Remove while it waits there; Decode it, the hex and the
    // code of the finished transaction, or the PSBT's code.
    let mut items: Vec<(&str, Style, Action)> = Vec::new();
    if !wrong {
        match made_place(app, &o.out.0) {
            Some((_, Some(i))) => {
                items.push((EN.step_remove, Style::Ghost, Action::OutboxRemove(i)))
            }
            _ => items.push((
                EN.vault_save_to_a_stick,
                Style::Primary,
                Action::NonceCheck(crate::decode::CheckPress::Out),
            )),
        }
    }
    if o.finished.is_some() {
        items.push((
            EN.step_decode_button,
            Style::Secondary,
            Action::NonceCheck(crate::decode::CheckPress::Decode),
        ));
        items.push((
            if o.show_hex {
                EN.step_hide_the_raw_hex
            } else {
                EN.step_show_the_raw_hex
            },
            Style::Secondary,
            Action::NonceCheck(crate::decode::CheckPress::Hex),
        ));
    }
    if !wrong {
        items.push((
            EN.step_show_as_qr,
            Style::Secondary,
            Action::NonceCheck(crate::decode::CheckPress::Qr),
        ));
    }
    if let Some((place, _)) = made_place(app, &o.out.0) {
        y += ui.wrap_as((x, y, w), BODY_S, Role::Ok, &place) + GAP;
    }
    y += wrap_buttons(ui, x, y, w, BUTTON_H, &items) + GAP;
    if o.show_hex
        && let Some(hex) = o.hex.as_ref()
    {
        let th = ui.wrap_as(
            (x + PAD, y + PAD, w - 2.0 * PAD),
            BODY_S.mono(),
            Role::OnSurface,
            &spaced(hex),
        );
        ui.stroke(x, y, w, th + 2.0 * PAD, SHAPE_S, Role::OutlineVariant);
        y += th + 2.0 * PAD + PAD;
    }
    ui.c.pop_clip();
    app.scroll.content_h().set(y - top + SPACER);
    ui.report_scroll(clip, app.scroll.content_h().get() - h);
}
