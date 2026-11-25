//! `Screen::Decode`: a transaction taken apart.

use crate::drawing::View;
use crate::screens::parts::{ending, file_choices, nothing_to_work_on, spaced};
use crate::strings::{EN, fill};
use crate::ui::Region;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, grouped};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, DECODE_INDEX_W, DECODE_KIND_GAP, DECODE_ROW_H, DECODE_VALUE_ROOM,
    GAP, GAP_SMALL, LABEL_L, LABEL_M,
};
use faraday_ui::parts::{section_label, wrap_buttons};

/// The widest the transaction's column stands.
const COLUMN_MAX: f32 = 960.0;

/// A transaction taken apart: what it is, what goes in, what comes out
/// and to whom, and its bytes.
/// What the way back from Decode a transaction is called.
pub(crate) fn decode_back(back: Screen) -> &'static str {
    match back {
        Screen::Spend => EN.decode_back_sign_transaction,
        Screen::SpendBackup => EN.decode_back_spend,
        Screen::Files => EN.decode_back_files,
        Screen::WalletsStart => EN.decode_back_wallets,
        Screen::Catalog => EN.decode_back_tools,
        Screen::Check => EN.compact_check,
        Screen::NonceCheck => EN.decode_back_nonce_check,
        _ => EN.decode_back_back,
    }
}

/// What the way back from Decode a transaction does: back to the screen
/// it was opened from, the nonce check checked again when the keys or
/// wallets changed while Decode was open.
pub(crate) fn decode_back_to(back: Screen) -> Action {
    match back {
        Screen::NonceCheck => Action::NonceCheck(crate::decode::CheckPress::Back),
        s => Action::Nav(s),
    }
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if crate::decode::flow::contained_decode(app) {
        return card(app, ui, x0, cw, h);
    }
    let d = app.decode();
    let back = d.map_or_else(
        || app.decode.landing().unwrap_or(Screen::Catalog),
        |d| d.back,
    );
    // A small panel has the way back in its bar, and one column: each
    // value under its name.
    let compact = ui.compact;
    let (x, w) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, (cw - 96.0).min(COLUMN_MAX))
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    // Scrolled as far as the end it last drew to (`content_h`).
    let top = if compact {
        14.0
    } else {
        let link = app.named_back((decode_back(back), decode_back_to(back)));
        crate::screens::bar_head(ui, Some(link), EN.decode_decode_a_transaction)
    };
    let Some(d) = d else {
        // As far as the page reaches: Type it, and the field's Next, are
        // scrolled to.
        let used = landing(app, ui, (x, top, w));
        app.scroll
            .content_h()
            .set(used + faraday_draw::tokens::INVOICE_TAIL);
        ui.region_close(open, app.scroll.content_h().get() - h);
        return;
    };
    let y = body(app, ui, d, (x, top, w));
    app.scroll.content_h().set(y - top + 40.0);
    ui.region_close(open, app.scroll.content_h().get() - h);
}

/// The transaction taken apart, from (x, top) in a column `w` wide: the
/// facts, the inputs and outputs as a table, the buttons and the hex.
/// Returns where it ends.
fn body(
    app: &View,
    ui: &mut Ui,
    d: &crate::decode::DecodeState,
    (x, top, w): (f32, f32, f32),
) -> f32 {
    use crate::decode::btc;
    let compact = ui.compact;
    let t = &d.decoded;
    let mut y = top;
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
        (EN.decode_transaction, String::from(&*t.txid), true),
        (EN.decode_witness_id, String::from(&*t.wtxid), true),
        (EN.decode_version, t.version.to_string(), false),
        (EN.decode_locktime, String::from(&*t.locktime), false),
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
        (
            EN.decode_amounts_in,
            String::from(&*t.amounts_from.text()),
            false,
        ),
    ];
    for (k, v, mono) in facts.iter() {
        let face = if *mono { BODY_M.mono() } else { BODY_M };
        if compact {
            ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, k);
            let v = if *mono {
                String::from(&*grouped(v))
            } else {
                v.clone()
            };
            let vh = ui.wrap_as((x, y + 18.0, w), face, Role::OnSurface, &v);
            ui.rule(x, y + vh + 26.0, w, Role::OutlineVariant);
            y += vh + 34.0;
            continue;
        }
        let (room, row) = (w - 170.0, 34.0);
        let fitted = ui.fit_as(face, v, room);
        ui.text_mid_as((x, y, row), BODY_S, Role::OnSurfaceVariant, k);
        // A transaction id wider than the card's column is wrapped in
        // groups, never cut.
        let h = if *mono && *fitted != **v {
            ui.wrap_as((x + 160.0, y, room), face, Role::OnSurface, &grouped(v))
                .max(row)
        } else {
            ui.text_mid_as((x + 160.0, y, row), face, Role::OnSurface, &fitted);
            row
        };
        ui.rule(x, y + h, w, Role::OutlineVariant);
        y += h + 2.0;
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
            // The outpoint whole: wrapped in the column when it does not fit.
            let room = tw - DECODE_VALUE_ROOM;
            if ui.measure_as(BODY_M.mono(), &input.outpoint) > room {
                y += ui.wrap_as(
                    (x + DECODE_INDEX_W, y, room),
                    BODY_M.mono(),
                    Role::OnSurface,
                    &input.outpoint,
                ) + GAP_SMALL;
            } else {
                ui.text_mid_as(
                    (x + DECODE_INDEX_W, y, DECODE_INDEX_W),
                    BODY_M.mono(),
                    Role::OnSurface,
                    &input.outpoint,
                );
                y += 26.0;
            }
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
        y = address_lines(
            ui,
            (x + DECODE_INDEX_W, y),
            tw - DECODE_VALUE_ROOM,
            &grouped(&out.address),
        );
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

    // Opened from the Nonce check page: when the check now refuses the
    // transaction, the hex and its code are not offered.
    let refused = (d.back == Screen::NonceCheck
        && app
            .outside_check()
            .is_some_and(|o| o.check.refused(o.hex.is_some())))
        || app.decode_from_spend_held();
    let mut buttons = Vec::new();
    if !refused {
        buttons.push((
            if d.show_hex {
                EN.decode_hide_the_hex
            } else {
                EN.decode_show_the_hex
            },
            Style::Secondary,
            Action::DecodeHex,
        ));
        buttons.push((EN.show_as_qr, Style::Secondary, Action::QrDecoded));
    }
    // With no wallet loaded no output can be checked as change: the way
    // to load one, which comes back here.
    if app.session.wallets.is_empty() {
        buttons.push((
            EN.decode_load_a_wallet,
            Style::Secondary,
            Action::RestoreFor(crate::restore::Check::Decode),
        ));
    }
    if !buttons.is_empty() {
        y += wrap_buttons(ui, x, y, w, BUTTON_H, &buttons) + 4.0;
    }
    y += ending(
        ui,
        (x, y, w),
        None,
        (Action::DecodeAnother, decode_back_to(d.back)),
    );
    if d.show_hex && !refused {
        let th = ui.wrap_as(
            (x + 12.0, y + 10.0, w - 24.0),
            BODY_S.mono(),
            Role::OnSurface,
            &spaced(&t.hex),
        );
        ui.stroke(x, y, w, th + 20.0, 8.0, Role::OutlineVariant);
        y += th + 32.0;
    }
    y
}

/// The transactions held in Files, then the ways in: Scan and Open Files.
/// The transaction on show, when it came from a held file, is marked.
pub(crate) fn pick_groups(app: &crate::Faraday) -> Vec<crate::flow::PickGroup> {
    let shown = app.decode().map(|d| &*d.decoded.source);
    let held: Vec<crate::pick::PickRow> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == crate::wallet::FileKind::Transaction)
        .map(|(k, i)| {
            crate::pick::PickRow::pick(
                faraday_draw::widgets::Icon::File,
                i.name.to_string(),
                String::new(),
                shown.is_some_and(|s| s == &*i.name),
                Action::DecodeInbox(k),
            )
        })
        .collect();
    let mut groups = Vec::new();
    let loaded = !held.is_empty();
    if loaded {
        groups.push(crate::flow::PickGroup {
            head: EN.session_group_files,
            rows: held,
        });
    }
    groups.push(crate::flow::PickGroup {
        head: crate::pick::ways_in_head(loaded),
        rows: vec![
            crate::pick::PickRow::way_in(
                faraday_draw::widgets::Icon::Keyboard,
                EN.decode_way_type,
                Action::DecodeTyped(crate::decode::TypedPress::Open),
            ),
            crate::pick::PickRow::way_in(
                faraday_draw::widgets::Icon::Scan,
                EN.compact_scan,
                Action::ScanFor(crate::Ask::Decode),
            ),
            crate::pick::PickRow::way_in(
                faraday_draw::widgets::Icon::File,
                EN.parts_open_files,
                Action::Nav(Screen::Files),
            ),
        ],
    });
    groups
}

/// Decode a transaction as the flow's one card: the pick at the left, the
/// transaction taken apart as the card (`docs/FLOW-CONTAINER.md` §2).
fn card(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::decode::flow;
    let back = app.decode().map_or_else(
        || app.decode.landing().unwrap_or(Screen::Catalog),
        |d| d.back,
    );
    let link = app.named_back((decode_back(back), decode_back_to(back)));
    let page = crate::flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        back: Some(link),
        heading: EN.decode_decode_a_transaction,
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::No,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&flow::DECODE_STEPS) else {
        return;
    };
    let scroll = app.decode.scroll();
    let (next, again, _) = {
        let app_ref: &View = app;
        let mut body_of = |ui: &mut Ui, step: u8, x: f32, y: f32, w: f32| -> f32 {
            if step == flow::PICK {
                let mut cy = y;
                let none = !app_ref
                    .inbox
                    .iter()
                    .any(|i| i.kind == crate::wallet::FileKind::Transaction);
                let typing = app_ref.decode.typing();
                if none && !typing {
                    cy += ui.wrap_as(
                        (x, cy, w),
                        BODY_M,
                        Role::OnSurfaceVariant,
                        EN.decode_no_transaction_waits,
                    ) + GAP;
                }
                // Where the pair stands the pane holds the rows.
                if !crate::flow::pair_left(app_ref, &flow::DECODE_STEPS, ui.compact, cw) {
                    for g in pick_groups(app_ref) {
                        cy += crate::pick::head(ui, x, cy, w, g.head);
                        cy += crate::pick::rows(ui, x, cy, w, &g.rows);
                    }
                }
                if typing {
                    cy += typed_field(app_ref, ui, (x, cy, w), true);
                }
                return cy - y;
            }
            let Some(d) = app_ref.decode() else {
                return 0.0;
            };
            // The table's width rule: the card's column, at most 960.
            let w = ui.flow_work.map_or(w, |end| end - x).min(COLUMN_MAX);
            body(app_ref, ui, d, (x, y, w)) - y
        };
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        crate::flow::contained(
            ui,
            app_ref,
            contained,
            &page,
            scroll,
            &mut body_of,
            &mut under,
        )
    };
    app.scroll(crate::drawing::Scrolled::Decode, next, again);
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
            &input.sequence,
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
            fill(EN.decode_change_back, &[name, &n.to_string()]),
            Role::Ok,
        ),
        Some((name, false, n)) => (
            fill(EN.decode_back_to_receive, &[name, &n.to_string()]),
            Role::Ok,
        ),
        None => (EN.decode_pays_from_loaded.to_string(), Role::Warning),
    }
}

/// An output's address whole at `(x, y)`: on one line where `room` holds
/// it, else wrapped between its groups in `room`, the first line level with
/// the amount. Returns where the row under it starts.
fn address_lines(ui: &mut Ui, (x, y): (f32, f32), room: f32, addr: &str) -> f32 {
    if ui.measure_as(BODY_M.mono(), addr) <= room {
        ui.text_mid_as((x, y, DECODE_ROW_H), BODY_M.mono(), Role::OnSurface, addr);
        y + DECODE_ROW_H
    } else {
        let laid = ui.wrapped_as(BODY_M.mono(), addr, room);
        let lead = ((DECODE_ROW_H - ui.line_as(BODY_M.mono())) / 2.0).max(0.0);
        ui.draw_wrapped((x, y + lead), Role::OnSurface, &laid);
        y + laid.h.max(DECODE_ROW_H) + lead
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

/// No transaction chosen: the files to choose from, or the empty part.
fn landing(app: &View, ui: &mut Ui, (x, top, w): (f32, f32, f32)) -> f32 {
    let waiting: Vec<(&str, Action)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == crate::wallet::FileKind::Transaction)
        .map(|(k, i)| (&*i.name, Action::DecodeInbox(k)))
        .collect();
    if app.decode.typing() {
        return typed_field(app, ui, (x, top, w), false);
    }
    let mut y = top;
    y += if waiting.is_empty() {
        nothing_to_work_on(
            app,
            ui,
            (x, y, w),
            EN.decode_no_transaction_waits,
            (Some(Action::ScanFor(crate::Ask::Decode)), Screen::Decode),
        )
    } else {
        file_choices(ui, (x, y, w), EN.decode_choose_transaction, &waiting)
    };
    // Among the ways in: Type it, beside Scan and Open Files above.
    y += GAP;
    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        BUTTON_H,
        &[(
            EN.decode_way_type,
            Style::Secondary,
            Action::DecodeTyped(crate::decode::TypedPress::Open),
        )],
    );
    y - top
}

/// Type it's field with Next under it and the words of a refusal, from
/// `y`; returns the height. On the card Clear stands in the field.
fn typed_field(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), card: bool) -> f32 {
    use crate::decode::TypedPress as P;
    use faraday_draw::tokens::INVOICE_REFUSAL_GAP;
    let text = app.decode.typed_text().unwrap_or("");
    let mut used = crate::invoice::screen::field(
        ui,
        text,
        (
            EN.decode_field,
            Action::TypeHere(crate::edit::Here::DecodeText),
            (EN.decode_clear, Action::DecodeTyped(P::Clear)),
        ),
        (x, y, w),
        card,
        Some(&|ui: &Ui, room: f32| {
            let basis = ui.wrap_basis(BODY_M.mono(), room);
            app.decode
                .typed_lines(basis, |v| ui.wrap_ranges(BODY_M.mono(), v, room))
        }),
    );
    let next = (EN.decode_next, Style::Primary, Action::DecodeTyped(P::Next));
    used += if card {
        wrap_buttons(ui, x, y + used, w, BUTTON_H, &[next])
    } else {
        let clear = (
            EN.decode_clear,
            Style::Secondary,
            Action::DecodeTyped(P::Clear),
        );
        wrap_buttons(ui, x, y + used, w, BUTTON_H, &[next, clear])
    };
    if let Some(why) = app.decode.typed_refusal() {
        used += GAP;
        used += ui.wrap_as((x, y + used, w), BODY_M, Role::Error, why) + INVOICE_REFUSAL_GAP;
    }
    used
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

    /// Where the row under an address starts, the address drawn in `room`.
    fn below(room: f32, addr: &str) -> f32 {
        let d = DisplayInfo {
            width: 1200,
            height: 900,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        };
        let mut canvas = faraday_draw::Canvas::new(&d);
        let f = canvas.scale().factor();
        let mut hits: Vec<(faraday_draw::Rect, Action)> = Vec::new();
        let mut ui = Ui::new(&mut canvas, f, &mut hits, None);
        address_lines(&mut ui, (0.0, 0.0), room, addr)
    }

    #[test]
    fn an_address_wider_than_its_room_wraps_and_pushes_the_row_under_it_down() {
        let addr = grouped("tb1pqu8p28pr9gcns06xf429kcnfwpmhapvvjwd2r290k67ufj7jm8sq9fgqn8");
        assert_eq!(below(2000.0, &addr), DECODE_ROW_H);
        assert!(below(200.0, &addr) > 2.0 * DECODE_ROW_H);
    }
}
