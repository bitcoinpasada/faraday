//! `Screen::Decode`: a transaction taken apart.

use super::parts::spaced;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, grouped};
use crate::{Action, Screen};
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
        let lw = ui.text(x + 14.0, y, 12.0, W::R, Role::OnSurfaceVariant, label);
        ui.hit(x - 4.0, y - 4.0, lw + 30.0, 24.0, Action::Nav(d.back));
        ui.back(Action::Nav(d.back));
        y += 22.0;
        title(ui, x, y, EN.decode_decode_a_transaction);
        y += 44.0;
    }
    y += ui.wrap(x, y, w, 13.0, W::R, Role::OnSurfaceVariant, &t.source) + 14.0;

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
    let facts: [(&str, String, bool); 6] = [
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
    ];
    for (k, v, mono) in facts.iter() {
        let face = if *mono { W::M } else { W::R };
        if compact {
            ui.text(x, y, 12.0, W::R, Role::OnSurfaceVariant, k);
            let v = if *mono { grouped(v) } else { v.clone() };
            let vh = ui.wrap(x, y + 18.0, w, 13.0, face, Role::OnSurface, &v);
            ui.rule(x, y + vh + 26.0, w, Role::OutlineVariant);
            y += vh + 34.0;
            continue;
        }
        ui.text_mid(x, y, 34.0, 13.0, W::R, Role::OnSurfaceVariant, k);
        let v = ui.fit(13.0, face, v, w - 170.0);
        ui.text_mid(x + 160.0, y, 34.0, 13.0, face, Role::OnSurface, &v);
        ui.rule(x, y + 34.0, w, Role::OutlineVariant);
        y += 36.0;
    }
    y += 18.0;

    section_label(
        ui,
        x,
        y,
        &fill(EN.decode_inputs, &[&t.inputs.len().to_string()]),
    );
    y += 28.0;
    for (i, input) in t.inputs.iter().enumerate() {
        ui.text_mid(
            x,
            y,
            26.0,
            12.0,
            W::R,
            Role::OnSurfaceVariant,
            &i.to_string(),
        );
        if let Some(a) = input.amount {
            ui.text_right(x + w, y, 26.0, 13.0, W::S, Role::OnSurface, &btc(a));
        }
        if compact {
            // The outpoint whole, under its number and amount.
            y += 26.0;
            y += ui.wrap(
                x + 26.0,
                y,
                w - 26.0,
                12.0,
                W::M,
                Role::OnSurface,
                &input.outpoint,
            ) + 4.0;
        } else {
            let op = ui.fit(13.0, W::M, &input.outpoint, w - 220.0);
            ui.text_mid(x + 26.0, y, 26.0, 13.0, W::M, Role::OnSurface, &op);
            y += 26.0;
        }
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
        let line = fill(
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
        );
        if compact {
            let lh = ui.wrap(
                x + 26.0,
                y + 2.0,
                w - 26.0,
                12.0,
                W::R,
                Role::OnSurfaceVariant,
                &line,
            );
            ui.rule(x, y + lh + 8.0, w, Role::OutlineVariant);
            y += lh + 12.0;
            continue;
        }
        let line = ui.fit(12.0, W::R, &line, w - 26.0);
        ui.text_mid(x + 26.0, y, 22.0, 12.0, W::R, Role::OnSurfaceVariant, &line);
        ui.rule(x, y + 26.0, w, Role::OutlineVariant);
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
        ui.text_mid(
            x,
            y,
            26.0,
            12.0,
            W::R,
            Role::OnSurfaceVariant,
            &i.to_string(),
        );
        ui.text_right(
            x + w,
            y,
            26.0,
            13.0,
            W::S,
            Role::OnSurface,
            &btc(out.amount),
        );
        let (whose, tone) = match &out.ours {
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
        };
        if compact {
            // The address whole, then its kind, then whose it is.
            y += 26.0;
            let ah = ui.wrap(
                x + 26.0,
                y,
                w - 26.0,
                12.0,
                W::M,
                Role::OnSurface,
                &grouped(&out.address),
            );
            y += ah + 4.0;
            ui.text_mid(
                x + 26.0,
                y,
                22.0,
                12.0,
                W::R,
                Role::OnSurfaceVariant,
                out.kind,
            );
            y += 22.0;
            let wh = ui.wrap(x + 26.0, y + 2.0, w - 26.0, 12.0, W::S, tone, &whose);
            ui.rule(x, y + wh + 8.0, w, Role::OutlineVariant);
            y += wh + 12.0;
            continue;
        }
        let addr = ui.fit(13.0, W::M, &grouped(&out.address), w - 220.0);
        ui.text_mid(x + 26.0, y, 26.0, 13.0, W::M, Role::OnSurface, &addr);
        y += 26.0;
        let kw = ui.text_mid(
            x + 26.0,
            y,
            22.0,
            12.0,
            W::R,
            Role::OnSurfaceVariant,
            out.kind,
        );
        ui.text_mid(x + 40.0 + kw, y, 22.0, 12.0, W::S, tone, &whose);
        ui.rule(x, y + 26.0, w, Role::OutlineVariant);
        y += 30.0;
    }
    y += 18.0;

    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        38.0,
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
        let th = ui.wrap(
            x + 12.0,
            y + 10.0,
            w - 24.0,
            12.0,
            W::M,
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
