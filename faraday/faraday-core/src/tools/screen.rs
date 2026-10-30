//! The Tools screen: the calculators, one at a time.

use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use crate::tools::Reading;
use crate::tools::Tool;
use faraday_bip::compile::PolicyScript;
use faraday_bip::slip132::KeyReading;
use faraday_codec::encodings::ReadAs;

use crate::Action;
use crate::screens::{button_rows, guide_text_learn};
use crate::tools::{UNITS, tool_name};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

const GUIDE: &str = EN.tools_calculators_each;

/// What Encodings calls an encoding.
fn encoding_name(e: faraday_codec::encodings::Encoding) -> &'static str {
    use faraday_codec::encodings::Encoding as E;
    match e {
        E::Base58Check => EN.tools_encoding_base58check,
        E::Base58 => EN.tools_encoding_base58,
        E::Bech32 => EN.tools_encoding_bech32,
        E::Bech32m => EN.tools_encoding_bech32m,
        E::Hex => EN.tools_encoding_hex,
    }
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if app.tools.state().is_none() {
        return;
    }
    // A small panel has its bar for the way back and the name, and puts
    // each value under its name.
    if ui.compact {
        let (x, width) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
        let top = 12.0 - app.list_offset;
        let y = body(app, ui, x, top, width);
        let y = y + crate::compact::screen::about_learn(
            ui,
            x,
            y,
            width,
            GUIDE,
            Some(crate::learn::Topic::Tools),
        );
        crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 16.0);
        return;
    }
    use faraday_draw::tokens::{TOOLS_COLUMN_W, TOOLS_INSET};
    let x = x0 + TOOLS_INSET;
    let width = (cw - 2.0 * TOOLS_INSET).min(TOOLS_COLUMN_W);
    let y = header(app, ui, x, width);
    body(app, ui, x, y, width);
}

/// The way back and the name, handed to the top app bar, and what
/// explains the page, as the desktop draws them. Returns where the page
/// goes on.
fn header(app: &View, ui: &mut Ui, x: f32, width: f32) -> f32 {
    let Some(t) = app.tools.state() else {
        return 0.0;
    };
    let back = app.named_back((app.back_link_name(t.back), Action::Nav(t.back)));
    let y = crate::screens::bar_head(ui, Some(back), EN.tools_tools);
    y + guide_text_learn(app, ui, x, y, width, GUIDE, crate::learn::Topic::Tools)
}

/// The calculators, the field and what it works out. Returns where it
/// ends.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn body(app: &View, ui: &mut Ui, x: f32, mut y: f32, width: f32) -> f32 {
    let Some(t) = app.tools.state() else {
        return y;
    };
    let tools: Vec<(String, Style, Action)> = Tool::ALL
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            (
                tool_name(*tool).to_string(),
                Style::chosen(*tool == t.tool),
                Action::TTool(i as u8),
            )
        })
        .collect();
    y += button_rows(ui, x, y, width, &tools) + 4.0;
    // The mode row, where the tool has one.
    let modes: Vec<(String, bool)> = match t.tool {
        Tool::Hashes => ReadAs::ALL
            .iter()
            .map(|r| {
                let n = match r {
                    ReadAs::Auto => EN.tools_read_as_automatic,
                    ReadAs::Text => EN.tools_text,
                    ReadAs::Hex => EN.tools_hex,
                };
                (n.to_string(), *r == t.read_as)
            })
            .collect(),
        Tool::Units => UNITS
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (fill(EN.tools_unit_option, &[n]), usize::from(t.unit) == i))
            .collect(),
        Tool::Miniscript => PolicyScript::ALL
            .iter()
            .map(|s| {
                let n = match s {
                    PolicyScript::Segwit => "wsh",
                    PolicyScript::Taproot => "tr",
                };
                (n.to_string(), *s == t.script)
            })
            .collect(),
        _ => Vec::new(),
    };
    if !modes.is_empty() {
        let row: Vec<(String, Style, Action)> = modes
            .into_iter()
            .enumerate()
            .map(|(i, (n, on))| (n, Style::chosen(on), Action::TMode(i as u8)))
            .collect();
        y += button_rows(ui, x, y, width, &row);
    }
    // The field.
    let hint = match t.tool {
        Tool::Hashes => EN.tools_text_or_hex,
        Tool::Encodings => EN.tools_base58_bech32,
        Tool::Descriptor => EN.tools_a_descriptor,
        Tool::ConvertKey => EN.tools_an_xpub_ypub,
        Tool::Units => EN.tools_an_amount,
        Tool::Miniscript => EN.tools_a_policy_e_g,
    };
    ui.fill(x, y, width, 44.0, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(x, y, width, 44.0, 8.0, Role::Primary);
    // A press places the caret (`docs/TEXT-CURSOR.md` §4, C3).
    let here = Action::TypeHere(crate::edit::Here::Tools);
    let (tx, fh, room) = (
        x + faraday_draw::tokens::TYPED_INSET,
        faraday_draw::tokens::TEXT_FIELD_SHEET,
        width - 100.0,
    );
    ui.press_box(x, y, room + faraday_draw::tokens::TYPED_INSET, fh, here);
    if t.typed.is_empty() {
        let s = ui.fit_as(BODY_M.mono(), hint, room);
        ui.text_mid_as((tx, y, fh), BODY_M.mono(), Role::OnSurfaceVariant, &s);
    } else {
        ui.typed(
            tx,
            y,
            fh,
            BODY_M.size,
            W::M,
            Role::OnSurface,
            &t.typed,
            room,
            true,
            here,
        );
    }
    ui.button(
        x + width - 76.0,
        y + 6.0,
        Some(68.0),
        BUTTON_H,
        EN.tools_clear,
        Style::Ghost,
        Action::TClear,
    );
    y += 58.0;
    let Some(reading) = app.tools_reading() else {
        return y;
    };
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut note: Option<String> = None;
    match &*reading {
        Reading::Hashes(None) => note = Some(EN.tools_not_hex.into()),
        Reading::Hashes(Some(h)) => {
            rows.push((EN.tools_bytes.into(), h.len.to_string()));
            rows.push(("SHA-256".into(), hex(&h.sha256)));
            rows.push(("SHA-256d".into(), hex(&h.sha256d)));
            rows.push(("HASH160".into(), hex(&h.hash160)));
        }
        Reading::Encodings(r) => match r {
            Some(r) => {
                rows.push((EN.tools_encoding.into(), encoding_name(r.encoding).into()));
                if let Some(h) = &r.hrp {
                    rows.push((EN.tools_prefix.into(), h.clone()));
                }
                if let Some(v) = r.version {
                    rows.push((EN.tools_version.into(), format!("{v}")));
                }
                if let Some(c) = r.checksum {
                    rows.push((
                        EN.tools_checksum.into(),
                        if c {
                            EN.tools_holds
                        } else {
                            EN.tools_does_not_hold
                        }
                        .into(),
                    ));
                }
                rows.push((EN.tools_bytes.into(), hex(&r.bytes)));
            }
            None => note = Some(EN.tools_not_base58_bech32.into()),
        },
        Reading::Descriptor(f) => match f {
            Some(f) => {
                rows.push((EN.tools_checksum.into(), f.checksum.clone()));
                if let Some((g, holds)) = &f.given {
                    rows.push((
                        EN.tools_given.into(),
                        format!(
                            "{g} · {}",
                            if *holds {
                                EN.tools_holds
                            } else {
                                EN.tools_does_not_hold
                            }
                        ),
                    ));
                }
                rows.push((EN.tools_with_checksum_label.into(), f.with_checksum.clone()));
                rows.push((
                    EN.tools_wallet.into(),
                    if f.wallet.is_some() {
                        EN.tools_one_this_device.into()
                    } else {
                        EN.tools_not_loadable.into()
                    },
                ));
            }
            None => note = Some(EN.tools_not_a_descriptor.into()),
        },
        Reading::Key(k) => match k {
            KeyReading::Public(f) => {
                rows.push(("BIP-32".into(), f.bip32.clone()));
                for (s, k) in &f.slip132 {
                    let name = match s {
                        faraday_bip::keys::ScriptType::Legacy => EN.tools_legacy,
                        faraday_bip::keys::ScriptType::NestedSegwit => EN.tools_nested_segwit,
                        faraday_bip::keys::ScriptType::NativeSegwit => EN.tools_native_segwit,
                        faraday_bip::keys::ScriptType::Taproot => EN.tools_taproot,
                    };
                    rows.push((name.to_string(), k.clone()));
                }
                rows.push((EN.tools_depth.into(), f.depth.to_string()));
                rows.push((EN.tools_fingerprint.into(), f.fingerprint.clone()));
                rows.push((EN.tools_child.into(), f.child.clone()));
            }
            KeyReading::Private => note = Some(EN.tools_that_is_an_extended.into()),
            KeyReading::None => note = Some(EN.tools_not_an_extended.into()),
        },
        Reading::Nothing => {
            let sats = crate::tools::units_sats(&t.typed, t.unit);
            rows.push(("sat".into(), crate::ui::thousands(sats)));
            rows.push((
                "BTC".into(),
                format!("{}.{:08}", sats / 100_000_000, sats % 100_000_000),
            ));
            rows.push((
                "mBTC".into(),
                format!("{}.{:05}", sats / 100_000, sats % 100_000),
            ));
            rows.push(("bits".into(), format!("{}.{:02}", sats / 100, sats % 100)));
        }
        Reading::Miniscript(f) => match f {
            Ok(f) => {
                rows.push((EN.tools_descriptor.into(), f.descriptor.clone()));
                rows.push((EN.tools_spend_paths.into(), f.paths.len().to_string()));
                rows.push((
                    EN.tools_wallet.into(),
                    if f.wallet.is_some() {
                        EN.tools_one_this_device.into()
                    } else {
                        EN.tools_keys_need_origins.into()
                    },
                ));
            }
            Err(e) => note = Some(e.clone()),
        },
    }
    if let Some(n) = note {
        return y + ui.wrap_as((x, y, width), BODY_M, Role::Error, &n) + 10.0;
    }
    // On a small panel each value goes under its name.
    let (vx, below) = if ui.compact {
        (0.0, 20.0)
    } else {
        (140.0, 0.0)
    };
    for (label, value) in rows {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, &label);
        y += below;
        y += ui
            .wrap_as(
                (x + vx, y, width - vx),
                BODY_M.mono(),
                Role::OnSurface,
                &value,
            )
            .max(20.0)
            + 10.0;
    }
    y
}
