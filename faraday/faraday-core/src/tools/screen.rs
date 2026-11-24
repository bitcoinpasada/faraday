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
use crate::screens::button_rows;
use crate::tools::{UNITS, tool_name};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, PAD};
use faraday_draw::tokens::{GAP_SMALL, GRID, TAB_INDICATOR_MIN, TITLE_S};

fn hex(b: &[u8]) -> String {
    String::from(&*faraday_crypto::hex::encode_string(b))
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
        ui.page_begin((x0, cw, h), app.list_offset);
        let top = 12.0;
        let y = body(app, ui, x, top, width);
        let y = y + crate::compact::screen::about(ui, x, y, width, GUIDE);
        crate::compact::screen::finish(app, ui, h, y - top + 16.0);
        return;
    }
    if crate::tools::flow::contained_tool(app) {
        return card(app, ui, x0, cw, h);
    }
    use faraday_draw::tokens::{TOOLS_COLUMN_W, TOOLS_INSET};
    let x = x0 + TOOLS_INSET;
    let width = (cw - 2.0 * TOOLS_INSET).min(TOOLS_COLUMN_W);
    let y = header(app, ui);
    body(app, ui, x, y, width);
}

/// A tool with one way in, drawn as the flow's one card: Input, then
/// Result (`docs/FLOW-CONTAINER.md` §2).
fn card(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(t) = app.tools.state() else { return };
    let back = Some(app.named_back((app.back_link_name(t.back), Action::Nav(t.back))));
    let page = crate::flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        back,
        heading: EN.tools_tools,
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::No,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&crate::tools::flow::TOOL_STEPS) else {
        return;
    };
    let scroll = t.scroll;
    let (next, again, _) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, step: u8, x: f32, y: f32, w: f32| -> f32 {
            if app_ref.tools.state().is_some_and(|t| t.tool == Tool::Units) {
                // One card: the amounts as tabs, the unit, the field and
                // what it converts to, changing as it is typed.
                let top = y;
                let mut y = y + amount_tabs(app_ref, ui, x, y, w) + GRID;
                y += mode_row(app_ref, ui, x, y, w);
                y = field(app_ref, ui, x, y, w);
                return reading_rows(app_ref, ui, x, y, w) - top;
            }
            if step == crate::tools::flow::INPUT {
                let top = y;
                let mut y = y + mode_row(app_ref, ui, x, y, w);
                y = field(app_ref, ui, x, y, w);
                let empty = app_ref.tools.state().is_none_or(|t| t.typed.is_empty());
                let style = if empty {
                    Style::PrimaryOff
                } else {
                    Style::Primary
                };
                ui.button(
                    x,
                    y,
                    None,
                    BUTTON_H,
                    EN.tools_next,
                    style,
                    Action::TStep(crate::tools::flow::RESULT),
                );
                y + BUTTON_H - top
            } else {
                reading_rows(app_ref, ui, x, y, w) - y
            }
        };
        // The foot of the left pane: Units' one button.
        let mut under = |ui: &mut Ui, x: f32, y: f32, w: f32| -> f32 {
            let Some(t) = app_ref.tools.state().filter(|t| t.tool == Tool::Units) else {
                return 0.0;
            };
            let full = t.amount_count() >= crate::tools::MAX_AMOUNTS;
            let style = if full { Style::TonalOff } else { Style::Tonal };
            // Where the pair stands the button is at the pane's foot: its
            // bottom one inset above the card's end, so the pane and the
            // card end level. Otherwise it stands under the rows.
            let scrolled = app_ref.scroll.choices_offset();
            let foot = match ui.flow_card {
                Some((cx, cy, _, ch)) if x < cx => {
                    Some(cy + ch - faraday_draw::tokens::CHOICES_INSET + scrolled)
                }
                _ => None,
            };
            let (by, used) = match foot {
                Some(end) if end - BUTTON_H >= y + GRID => (end - BUTTON_H, end - y),
                _ => (y + GRID, BUTTON_H + GRID),
            };
            ui.button(
                x,
                by,
                Some(w),
                BUTTON_H,
                EN.tools_add_amount,
                style,
                Action::TAddAmount,
            );
            used
        };
        crate::flow::contained(ui, app_ref, contained, &page, scroll, &mut body, &mut under)
    };
    app.scroll(crate::drawing::Scrolled::Tools, next, again);
}

/// Units' amounts as Material 3's tabs (the Tabs page: primary tabs,
/// fixed): each tab one amount in its own unit, its label cut where it is
/// longer than its room, and a cross at its end in a touch target of its
/// own that removes it (never the last). Returns the row's height.
fn amount_tabs(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use faraday_draw::tokens::{ICON_BUTTON, ICON_GLYPH, TAB_INDICATOR_PRIMARY, shape_full};
    let Some(t) = app.tools.state() else {
        return 0.0;
    };
    let n = t.amount_count().max(1);
    let (h, touch) = (ui.touch, ui.touch);
    let tw = w / n as f32;
    let removable = n > 1;
    ui.rule(x, y + h, w, Role::OutlineVariant);
    for i in 0..n {
        let Some((typed, unit)) = t.amount_of(i) else {
            continue;
        };
        let unit_name = UNITS.get(usize::from(unit)).map_or("", |(u, _)| *u);
        let label = if typed.is_empty() {
            fill(EN.tools_amount_tab_empty, &[unit_name])
        } else {
            fill(EN.tools_amount_tab, &[typed, unit_name])
        };
        let on = i == t.amount_shown();
        let ink = if on {
            Role::Primary.paint()
        } else {
            Role::OnSurfaceVariant.paint()
        };
        let tx = x + tw * i as f32;
        // The label has the tab less the cross's target; a longer one is
        // cut, so it never reaches the cross.
        let cross = if removable { touch } else { 0.0 };
        let room = (tw - cross - GRID).max(0.0);
        ui.press(
            crate::ui::Shape::new(tx, y, tw - cross, h, 0.0),
            ink,
            Action::TAmount(i as u8),
        );
        let shown = ui.fit_as(TITLE_S, &label, room);
        ui.text_mid_as((tx + GAP_SMALL, y, h), TITLE_S, ink, &shown);
        if on {
            let iw = (ui.measure_as(TITLE_S, &shown)).max(TAB_INDICATOR_MIN);
            let ix = tx + GAP_SMALL;
            ui.fill(
                ix,
                y + h - TAB_INDICATOR_PRIMARY,
                iw.min(tw - cross - GAP_SMALL),
                TAB_INDICATOR_PRIMARY,
                0.0,
                Role::Primary,
            );
        }
        if removable {
            let (bx, by) = (tx + tw - touch, y);
            let edge = (touch - ICON_BUTTON) / 2.0;
            let shape = crate::ui::Shape::new(
                bx + edge,
                by + edge,
                ICON_BUTTON,
                ICON_BUTTON,
                shape_full(ICON_BUTTON),
            );
            let ink = Role::OnSurfaceVariant.paint();
            ui.press_to(
                shape,
                (bx, by, touch, h),
                ink,
                Action::TRemoveAmount(i as u8),
            );
            let mark = "\u{d7}";
            let mw = ui.measure(ICON_GLYPH, W::R, mark);
            ui.text_mid(
                bx + edge + (ICON_BUTTON - mw) / 2.0,
                by + edge,
                ICON_BUTTON,
                ICON_GLYPH,
                W::R,
                ink,
                mark,
            );
        }
    }
    h
}

/// The way back and the name, handed to the top app bar, as the desktop
/// draws it. Returns where the page goes on.
fn header(app: &View, ui: &mut Ui) -> f32 {
    let Some(t) = app.tools.state() else {
        return 0.0;
    };
    let back = app.named_back((app.back_link_name(t.back), Action::Nav(t.back)));
    crate::screens::bar_head(ui, Some(back), EN.tools_tools)
}

/// The calculators, the field and what it works out. Returns where it
/// ends.
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
    y += mode_row(app, ui, x, y, width);
    let y = field(app, ui, x, y, width);
    reading_rows(app, ui, x, y, width)
}

/// The mode row, where the tool has one. Returns its height.
fn mode_row(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32) -> f32 {
    let Some(t) = app.tools.state() else {
        return 0.0;
    };
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
        return button_rows(ui, x, y, width, &row);
    }
    0.0
}

/// The field with Use a sample, Clear and, where the tool takes codes, its
/// scan and file buttons. Returns where it ends.
fn field(app: &View, ui: &mut Ui, x: f32, mut y: f32, width: f32) -> f32 {
    let Some(t) = app.tools.state() else {
        return y;
    };
    let hint = match t.tool {
        Tool::Hashes => EN.tools_text_or_hex,
        Tool::Encodings => EN.tools_base58_bech32,
        Tool::Descriptor => EN.tools_a_descriptor,
        Tool::ConvertKey => EN.tools_an_xpub_ypub,
        Tool::Units => EN.tools_an_amount,
        Tool::Miniscript => EN.tools_a_policy_e_g,
    };
    // Use a sample goes in the "Demo only" card (`docs/DEMO.md` §14).
    if app.tools_sample_shown() {
        crate::samples::put::offer(ui, Action::UseSample);
    }
    ui.fill(x, y, width, 44.0, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(x, y, width, 44.0, 8.0, Role::Primary);
    // A press places the caret (`docs/TEXT-CURSOR.md` §3).
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
    if crate::tools::takes_codes(t.tool) {
        let mut row: Vec<(String, Style, Action)> =
            vec![(EN.tools_scan.into(), Style::Secondary, Action::TScan)];
        let files: Vec<(usize, &str)> = app
            .inbox
            .iter()
            .enumerate()
            .filter(|(_, i)| crate::tools::file_fits(i.kind))
            .map(|(k, i)| (k, i.name.as_str()))
            .collect();
        if files.is_empty() {
            row.push((
                EN.tools_from_files.into(),
                Style::Disabled,
                Action::TFile(usize::MAX),
            ));
        }
        for (k, name) in files {
            row.push((
                fill(EN.tools_from_file, &[name]),
                Style::Secondary,
                Action::TFile(k),
            ));
        }
        y += button_rows(ui, x, y, width, &row);
    }
    y
}

/// What the tool worked out of what is typed: its rows, the note that
/// refuses it, and Take to Wallets. Returns where it ends.
fn reading_rows(app: &View, ui: &mut Ui, x: f32, mut y: f32, width: f32) -> f32 {
    let Some(t) = app.tools.state() else {
        return y;
    };
    let Some(reading) = app.tools_reading() else {
        return y;
    };
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut note: Option<String> = None;
    let mut warn_row: Option<usize> = None;
    let mut take = false;
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
                take = f.wallet.is_some();
                rows.push((
                    EN.tools_wallet.into(),
                    if f.wallet.is_some() {
                        EN.tools_one_this_device.into()
                    } else {
                        EN.tools_not_loadable.into()
                    },
                ));
                if !f.repeated.is_empty() {
                    warn_row = Some(rows.len());
                    let lines: Vec<String> = f
                        .repeated
                        .iter()
                        .map(|&(first, later)| repeated_key_line(first, later))
                        .collect();
                    rows.push((EN.tools_keys.into(), lines.join("\n")));
                }
            }
            None => note = Some(EN.tools_not_a_descriptor.into()),
        },
        Reading::Key(k) => match k {
            KeyReading::Public(f) => {
                rows.push(("BIP-32".into(), String::from(&*f.bip32)));
                for (s, k) in &f.slip132 {
                    let name = match s {
                        faraday_bip::keys::ScriptType::Legacy => EN.tools_legacy,
                        faraday_bip::keys::ScriptType::NestedSegwit => EN.tools_nested_segwit,
                        faraday_bip::keys::ScriptType::NativeSegwit => EN.tools_native_segwit,
                        faraday_bip::keys::ScriptType::Taproot => EN.tools_taproot,
                    };
                    rows.push((name.to_string(), String::from(&**k)));
                }
                rows.push((EN.tools_depth.into(), f.depth.to_string()));
                rows.push((EN.tools_fingerprint.into(), String::from(&*f.fingerprint)));
                rows.push((EN.tools_child.into(), String::from(&*f.child)));
            }
            KeyReading::Private => note = Some(EN.tools_that_is_an_extended.into()),
            KeyReading::None => note = Some(EN.tools_not_an_extended.into()),
        },
        Reading::Nothing => {
            let sats = crate::tools::units_sats(&t.typed, t.unit);
            rows.push(("sat".into(), String::from(&*crate::ui::thousands(sats))));
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
            Err(e) => note = Some(String::from(&**e)),
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
    for (n, (label, value)) in rows.into_iter().enumerate() {
        let role = if warn_row == Some(n) {
            Role::Warning
        } else {
            Role::OnSurface
        };
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, &label);
        y += below;
        y += ui
            .wrap_as((x + vx, y, width - vx), BODY_M.mono(), role, &value)
            .max(20.0)
            + 10.0;
    }
    if take {
        let label = EN.tools_take_to_wallets;
        let bw = ui.measure_as(LABEL_L, label) + PAD + PAD;
        ui.button(
            x,
            y,
            Some(bw),
            BUTTON_H,
            label,
            Style::Primary,
            Action::TTake,
        );
        y += BUTTON_H + PAD;
    }
    y
}

/// The line that says key `later` is the same key as key `first`
/// (counted from 0), or that one key is used twice.
pub(crate) fn repeated_key_line(first: usize, later: usize) -> String {
    if first == later {
        fill(EN.tools_key_used_twice, &[&(later + 1).to_string()])
    } else {
        fill(
            EN.tools_key_same_as,
            &[&(later + 1).to_string(), &(first + 1).to_string()],
        )
    }
}
