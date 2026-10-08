//! The Tools screen: OpenSigner's calculators, one at a time.

use opensigner_core::tools::Tool;
use osk_bip::compile::{PolicyScript, compile};
use osk_bip::descriptor::checksum_facts;
use osk_bip::hashes::hashes;
use osk_bip::slip132::{KeyReading, key_facts};
use osk_codec::encodings::{ReadAs, read as encoding_of, read_input};

use crate::screens::{button_rows, guide_text, title};
use crate::tools::{UNITS, tool_name};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::{Action, Faraday};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub(crate) fn draw(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, _h: f32) {
    let Some(t) = app.tools.as_ref() else {
        return;
    };
    let x = x0 + 56.0;
    let width = (cw - 112.0).min(900.0);
    let mut y = 36.0;
    ui.icon(
        x - 4.0,
        y,
        16.0,
        osk_ui::widgets::Icon::ChevronLeft,
        10.0,
        MUTED,
    );
    ui.text(x + 14.0, y, 12.0, W::R, MUTED, "Wallets");
    ui.hit(x - 4.0, y - 4.0, 70.0, 24.0, Action::Nav(t.back));
    y += 22.0;
    title(ui, x, y, "Tools");
    y += 52.0;
    y += guide_text(
        app,
        ui,
        x,
        y,
        width,
        "Calculators: each takes what you type, works one thing out and keeps nothing. None of them \
         touches a key; an extended private key is refused.",
    );
    let tools: Vec<(String, Style, Action)> = Tool::ALL
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            (
                tool_name(*tool).to_string(),
                if *tool == t.tool {
                    Style::Primary
                } else {
                    Style::Secondary
                },
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
                    ReadAs::Auto => "Read as: automatic",
                    ReadAs::Text => "Text",
                    ReadAs::Hex => "Hex",
                };
                (n.to_string(), *r == t.read_as)
            })
            .collect(),
        Tool::Units => UNITS
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (format!("In {n}"), usize::from(t.unit) == i))
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
            .map(|(i, (n, on))| {
                (
                    n,
                    if on { Style::Primary } else { Style::Ghost },
                    Action::TMode(i as u8),
                )
            })
            .collect();
        y += button_rows(ui, x, y, width, &row);
    }
    // The field.
    let shown = if t.typed.is_empty() {
        match t.tool {
            Tool::Hashes => "Text or hex",
            Tool::Encodings => "Base58, bech32 or hex",
            Tool::Descriptor => "A descriptor",
            Tool::ConvertKey => "An xpub, ypub, zpub, tpub…",
            Tool::Units => "An amount",
            Tool::Miniscript => "A policy, e.g. or(pk(9a6a2580),and(pk(…),older(1000)))",
        }
        .to_string()
    } else {
        format!("{}{}", t.typed, ui.caret_char())
    };
    ui.fill(x, y, width, 44.0, 8.0, BG);
    ui.stroke(x, y, width, 44.0, 8.0, ACCENT);
    let s = ui.fit(14.0, W::M, &shown, width - 100.0);
    ui.text_mid(
        x + 12.0,
        y,
        44.0,
        14.0,
        W::M,
        if t.typed.is_empty() { DIM } else { TEXT },
        &s,
    );
    ui.button(
        x + width - 76.0,
        y + 6.0,
        Some(68.0),
        32.0,
        "Clear",
        Style::Ghost,
        Action::TClear,
    );
    y += 58.0;
    if t.typed.trim().is_empty() {
        return;
    }
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut note: Option<String> = None;
    match t.tool {
        Tool::Hashes => {
            let h = hashes(&read_input(t.typed.trim(), t.read_as));
            rows.push(("Bytes".into(), h.len.to_string()));
            rows.push(("SHA-256".into(), hex(&h.sha256)));
            rows.push(("SHA-256d".into(), hex(&h.sha256d)));
            rows.push(("HASH160".into(), hex(&h.hash160)));
        }
        Tool::Encodings => match encoding_of(t.typed.trim()) {
            Some(r) => {
                rows.push(("Encoding".into(), format!("{:?}", r.encoding)));
                if let Some(h) = &r.hrp {
                    rows.push(("Prefix".into(), h.clone()));
                }
                if let Some(v) = r.version {
                    rows.push(("Version".into(), format!("{v}")));
                }
                if let Some(c) = r.checksum {
                    rows.push((
                        "Checksum".into(),
                        if c { "holds" } else { "does not hold" }.into(),
                    ));
                }
                rows.push(("Bytes".into(), hex(&r.bytes)));
            }
            None => note = Some("Not Base58, bech32, bech32m or hex".into()),
        },
        Tool::Descriptor => match checksum_facts(&t.typed) {
            Some(f) => {
                rows.push(("Checksum".into(), f.checksum.clone()));
                if let Some((g, holds)) = &f.given {
                    rows.push((
                        "Given".into(),
                        format!("{g} · {}", if *holds { "holds" } else { "does not hold" }),
                    ));
                }
                rows.push(("With it".into(), f.with_checksum.clone()));
                rows.push((
                    "Wallet".into(),
                    if f.wallet.is_some() {
                        "one this device loads".into()
                    } else {
                        "not one this device loads".into()
                    },
                ));
            }
            None => note = Some("Not a descriptor".into()),
        },
        Tool::ConvertKey => match key_facts(&t.typed, app.session.network()) {
            KeyReading::Public(f) => {
                rows.push(("BIP-32".into(), f.bip32.clone()));
                for (s, k) in &f.slip132 {
                    let name = match s {
                        osk_bip::keys::ScriptType::Legacy => "Legacy",
                        osk_bip::keys::ScriptType::NestedSegwit => "Nested SegWit",
                        osk_bip::keys::ScriptType::NativeSegwit => "Native SegWit",
                        osk_bip::keys::ScriptType::Taproot => "Taproot",
                    };
                    rows.push((name.to_string(), k.clone()));
                }
                rows.push(("Depth".into(), f.depth.to_string()));
                rows.push(("Fingerprint".into(), f.fingerprint.clone()));
                rows.push(("Child".into(), f.child.clone()));
            }
            KeyReading::Private => {
                note =
                    Some("That is an extended private key: this tool takes public keys only".into())
            }
            KeyReading::None => note = Some("Not an extended key".into()),
        },
        Tool::Units => {
            let digits: u64 = t.typed.parse().unwrap_or(0);
            let sats = digits
                .saturating_mul(UNITS[usize::from(t.unit)].1)
                .min(opensigner_core::tools::MAX_SATS);
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
        Tool::Miniscript => match compile(&t.typed, t.script, &app.tools_keys()) {
            Ok(f) => {
                rows.push(("Descriptor".into(), f.descriptor.clone()));
                rows.push(("Spend paths".into(), f.paths.len().to_string()));
                rows.push((
                    "Wallet".into(),
                    if f.wallet.is_some() {
                        "one this device loads".into()
                    } else {
                        "keys need origins and both chains to load".into()
                    },
                ));
            }
            Err(e) => note = Some(e),
        },
    }
    if let Some(n) = note {
        ui.wrap(x, y, width, 13.0, W::R, ERR, &n);
        return;
    }
    for (label, value) in rows {
        ui.text(x, y, 13.0, W::R, MUTED, &label);
        y += ui
            .wrap(x + 140.0, y, width - 140.0, 13.0, W::M, TEXT, &value)
            .max(20.0)
            + 10.0;
    }
}
