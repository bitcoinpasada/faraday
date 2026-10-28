//! `Screen::Decode`: a transaction taken apart.

use super::parts::spaced;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, grouped};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, DECODE_INDEX_W, DECODE_KIND_GAP, DECODE_VALUE_ROOM, LABEL_L, LABEL_M,
};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{section_label, title, wrap_buttons};

/// A transaction taken apart: what it is, what goes in, what comes out
/// and to whom, and its bytes.
/// What the way back from Decode a transaction is called.
pub(crate) fn decode_back(back: Screen) -> &'static str {
    match back {
        Screen::Spend => EN.decode_back_sign_transaction,
        Screen::Family => EN.decode_back_spend,
        Screen::Files => EN.decode_back_files,
        Screen::WalletsStart => EN.decode_back_wallets,
        Screen::Catalog => EN.decode_back_tools,
        _ => EN.decode_back_back,
    }
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::decode::btc;
    let Some(d) = app.decode() else {
        return;
    };
    let t = &d.decoded;
    // A small panel has the way back in its bar, and one column: each
    // value under its name.
    let compact = ui.compact;
    let (x, w) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, (cw - 96.0).min(960.0))
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    // Scrolled as far as the end it last drew to (`content_h`).
    let top = if compact { 14.0 } else { 28.0 } - app.list_offset;
    let mut y = top;
    if !compact {
        ui.icon(
            x - 4.0,
            y,
            16.0,
            Icon::ChevronLeft,
            10.0,
            Role::OnSurfaceVariant,
        );
        let (label, _) = app.named_back((decode_back(d.back), Action::Nav(d.back)));
        let lw = ui.text_as(x + 14.0, y, BODY_S, Role::OnSurfaceVariant, label);
        ui.hit(x - 4.0, y - 4.0, lw + 30.0, 24.0, Action::Nav(d.back));
        ui.back(Action::Nav(d.back));
        y += 22.0;
        title(ui, x, y, EN.decode_decode_a_transaction);
        y += 44.0;
    }
    y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, &t.source) + 14.0;

    let fee = match t.fee {
        Some(f) => fill(
            EN.decode_sat_sat_vb,
            &[
                &f.to_string(),
                &format!("{:.1}", f as f64 / t.size.1.max(1) as f64),
            ],
        ),
        None => EN.decode_fee_not_stated.to_string(),
    };
    let facts: [(&str, String, bool); 7] = [
        (EN.decode_transaction, t.txid.clone(), true),
        (EN.decode_witness_id, t.wtxid.clone(), true),
        (EN.decode_version, t.version.to_string(), false),
        (EN.decode_locktime, t.locktime.clone(), false),
        (
            EN.decode_size,
            fill(
                EN.decode_bytes_vb_wu,
                &[
                    &t.size.0.to_string(),
                    &t.size.1.to_string(),
                    &t.size.2.to_string(),
                ],
            ),
            false,
        ),
        (EN.decode_fee, fee, false),
        (EN.decode_amounts_in, t.amounts_from.text(), false),
    ];
    for (k, v, mono) in facts.iter() {
        let face = if *mono { BODY_M.mono() } else { BODY_M };
        if compact {
            ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, k);
            let v = if *mono { grouped(v) } else { v.clone() };
            let vh = ui.wrap_as((x, y + 18.0, w), face, Role::OnSurface, &v);
            ui.rule(x, y + vh + 26.0, w, Role::OutlineVariant);
            y += vh + 34.0;
            continue;
        }
        ui.text_mid_as((x, y, 34.0), BODY_S, Role::OnSurfaceVariant, k);
        let v = ui.fit_as(face, v, w - 170.0);
        ui.text_mid_as((x + 160.0, y, 34.0), face, Role::OnSurface, &v);
        ui.rule(x, y + 34.0, w, Role::OutlineVariant);
        y += 36.0;
    }
    y += 18.0;
    // Beside the navigation, the inputs and outputs are a table as wide as
    // its columns (`docs/LAYOUT.md` §17): the number, the longest
    // outpoint, address or line under one, the amount; no wider than the
    // page's column.
    let tw = if compact { w } else { table_w(ui, t, w) };

    section_label(
        ui,
        x,
        y,
        &fill(EN.decode_inputs, &[&t.inputs.len().to_string()]),
    );
    y += 28.0;
    for (i, input) in t.inputs.iter().enumerate() {
        ui.text_mid_as((x, y, 26.0), BODY_S, Role::OnSurfaceVariant, &i.to_string());
        if let Some(a) = input.amount {
            ui.text_right_as((x + tw, y, 26.0), LABEL_L, Role::OnSurface, &btc(a));
        }
        if compact {
            // The outpoint whole, under its number and amount.
            y += 26.0;
            y += ui.wrap_as(
                (x + 26.0, y, w - 26.0),
                BODY_S.mono(),
                Role::OnSurface,
                &input.outpoint,
            ) + 4.0;
        } else {
            let op = ui.fit_as(BODY_M.mono(), &input.outpoint, tw - DECODE_VALUE_ROOM);
            ui.text_mid_as((x + 26.0, y, 26.0), BODY_M.mono(), Role::OnSurface, &op);
            y += 26.0;
        }
        let line = input_line(input);
        if compact {
            let lh = ui.wrap_as(
                (x + 26.0, y + 2.0, w - 26.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &line,
            );
            ui.rule(x, y + lh + 8.0, w, Role::OutlineVariant);
            y += lh + 12.0;
            continue;
        }
        let line = ui.fit_as(BODY_S, &line, tw - DECODE_INDEX_W);
        ui.text_mid_as((x + 26.0, y, 22.0), BODY_S, Role::OnSurfaceVariant, &line);
        ui.rule(x, y + 26.0, tw, Role::OutlineVariant);
        y += 30.0;
    }
    y += 18.0;

    section_label(
        ui,
        x,
        y,
        &fill(EN.decode_outputs, &[&t.outputs.len().to_string()]),
    );
    y += 28.0;
    for (i, out) in t.outputs.iter().enumerate() {
        ui.text_mid_as((x, y, 26.0), BODY_S, Role::OnSurfaceVariant, &i.to_string());
        ui.text_right_as(
            (x + tw, y, 26.0),
            LABEL_L,
            Role::OnSurface,
            &btc(out.amount),
        );
        let (whose, tone) = whose(out);
        if compact {
            // The address whole, then its kind, then whose it is.
            y += 26.0;
            let ah = ui.wrap_as(
                (x + 26.0, y, w - 26.0),
                BODY_S.mono(),
                Role::OnSurface,
                &grouped(&out.address),
            );
            y += ah + 4.0;
            ui.text_mid_as(
                (x + 26.0, y, 22.0),
                BODY_S,
                Role::OnSurfaceVariant,
                out.kind,
            );
            y += 22.0;
            let wh = ui.wrap_as((x + 26.0, y + 2.0, w - 26.0), LABEL_M, tone, &whose);
            ui.rule(x, y + wh + 8.0, w, Role::OutlineVariant);
            y += wh + 12.0;
            continue;
        }
        let addr = ui.fit_as(
            BODY_M.mono(),
            &grouped(&out.address),
            tw - DECODE_VALUE_ROOM,
        );
        ui.text_mid_as((x + 26.0, y, 26.0), BODY_M.mono(), Role::OnSurface, &addr);
        y += 26.0;
        let kw = ui.text_mid_as(
            (x + 26.0, y, 22.0),
            BODY_S,
            Role::OnSurfaceVariant,
            out.kind,
        );
        ui.text_mid_as((x + 40.0 + kw, y, 22.0), LABEL_M, tone, &whose);
        ui.rule(x, y + 26.0, tw, Role::OutlineVariant);
        y += 30.0;
    }
    y += 18.0;

    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        BUTTON_H,
        &[
            (
                if d.show_hex {
                    EN.decode_hide_the_hex
                } else {
                    EN.decode_show_the_hex
                },
                Style::Secondary,
                Action::DecodeHex,
            ),
            (EN.decode_show_as_qr, Style::Secondary, Action::QrDecoded),
        ],
    ) + 4.0;
    if d.show_hex {
        let th = ui.wrap_as(
            (x + 12.0, y + 10.0, w - 24.0),
            BODY_S.mono(),
            Role::OnSurface,
            &spaced(&t.hex),
        );
        ui.stroke(x, y, w, th + 20.0, 8.0, Role::OutlineVariant);
        y += th + 32.0;
    }
    ui.c.pop_clip();
    app.scroll.content_h().set(y - top + 40.0);
    ui.report_scroll(clip, app.scroll.content_h().get() - h);
}

/// An input's line under its outpoint: its sequence, its witness and its
/// script sig's size.
fn input_line(input: &crate::decode::DecodedInput) -> String {
    let wit = if input.witness.is_empty() {
        EN.decode_no_witness.to_string()
    } else {
        let sizes: Vec<String> = input
            .witness
            .iter()
            .map(|h| (h.len() / 2).to_string())
            .collect();
        fill(
            EN.decode_witness_bytes,
            &[
                &input.witness.len().to_string(),
                (if input.witness.len() == 1 {
                    EN.decode_item
                } else {
                    EN.decode_items
                }),
                &sizes.join(", ").to_string(),
            ],
        )
    };
    fill(
        EN.decode_sequence,
        &[
            &input.sequence.to_string(),
            &wit.to_string(),
            &(if input.script_sig > 0 {
                format!(
                    " {}",
                    fill(EN.decode_script_sig_bytes, &[&input.script_sig.to_string()])
                )
            } else {
                String::new()
            })
            .to_string(),
        ],
    )
}

/// Whose an output is, and its colour: change or a receive address of a
/// wallet loaded here, or a payment away from them.
fn whose(out: &crate::decode::DecodedOutput) -> (String, Role) {
    match &out.ours {
        Some((name, true, n)) => (
            fill(EN.decode_change_back, &[&name.to_string(), &n.to_string()]),
            Role::Ok,
        ),
        Some((name, false, n)) => (
            fill(
                EN.decode_back_to_receive,
                &[&name.to_string(), &n.to_string()],
            ),
            Role::Ok,
        ),
        None => (EN.decode_pays_from_loaded.to_string(), Role::Warning),
    }
}

/// The inputs' and outputs' table on the wide layout: as wide as its
/// columns, the number, the longest outpoint or address with the room
/// for the amount after it, or the longest line under one, at most `w`.
fn table_w(ui: &mut Ui, t: &crate::decode::Decoded, w: f32) -> f32 {
    let mono = BODY_M.mono();
    let values = t
        .inputs
        .iter()
        .map(|i| ui.measure_as(mono, &i.outpoint))
        .chain(
            t.outputs
                .iter()
                .map(|o| ui.measure_as(mono, &grouped(&o.address))),
        )
        .fold(0.0, f32::max)
        + DECODE_VALUE_ROOM;
    let lines = t
        .inputs
        .iter()
        .map(|i| ui.measure_as(BODY_S, &input_line(i)))
        .chain(t.outputs.iter().map(|o| {
            ui.measure_as(BODY_S, o.kind) + DECODE_KIND_GAP + ui.measure_as(LABEL_M, &whose(o).0)
        }))
        .fold(0.0, f32::max)
        + DECODE_INDEX_W;
    values.max(lines).min(w)
}
