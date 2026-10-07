//! The Explore screen: a loaded key's public side at a path.

use osk_bip::keys::ScriptType;

use crate::explore::PRESETS;
use crate::screens::{button_rows, guide_text, title};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Faraday};

pub(crate) fn draw(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, _h: f32) {
    let Some(e) = app.explore.as_ref() else {
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
    ui.hit(x - 4.0, y - 4.0, 70.0, 24.0, Action::Nav(e.back));
    y += 22.0;
    title(ui, x, y, "Explore");
    y += 52.0;
    y += guide_text(
        app,
        ui,
        x,
        y,
        width,
        "The public side of a loaded key at any path: the extended public key, the public key and the \
         address there. Compare them with what another wallet shows for the same path. Nothing private \
         is shown.",
    );
    let keys: Vec<(String, Style, Action)> = app
        .session
        .keys
        .iter()
        .map(|k| {
            let fp = k.master.fingerprint();
            (
                format!("{} · {}", fp_text(fp), k.label),
                if e.key == Some(fp.0) {
                    Style::Primary
                } else {
                    Style::Secondary
                },
                Action::XKey(fp.0),
            )
        })
        .collect();
    y += button_rows(ui, x, y, width, &keys) + 4.0;
    // The path.
    ui.text_mid(x, y, 44.0, 13.0, W::R, MUTED, "Path");
    let px = x + 120.0;
    let pw = width - 120.0;
    let err = app.explore_path().err();
    ui.fill(px, y, pw, 44.0, 8.0, BG);
    ui.stroke(
        px,
        y,
        pw,
        44.0,
        8.0,
        if err.is_some() { ERR } else { ACCENT },
    );
    ui.text_mid(
        px + 12.0,
        y,
        44.0,
        15.0,
        W::M,
        TEXT,
        &format!("m/{}|", e.path),
    );
    y += 52.0;
    if let Some(m) = err {
        ui.text(px, y, 13.0, W::R, ERR, m);
        y += 24.0;
    }
    let presets: Vec<(String, Style, Action)> = PRESETS
        .iter()
        .enumerate()
        .map(|(i, (name, _))| (name.to_string(), Style::Ghost, Action::XPreset(i as u8)))
        .chain([
            ("Index -1".to_string(), Style::Ghost, Action::XIndex(-1)),
            ("Index +1".to_string(), Style::Ghost, Action::XIndex(1)),
        ])
        .collect();
    y += button_rows(ui, px, y, pw, &presets) + 8.0;
    let Some(r) = app.explore_reading() else {
        return;
    };
    let script_name = |s: ScriptType| match s {
        ScriptType::NativeSegwit => "Native SegWit",
        ScriptType::Taproot => "Taproot",
        ScriptType::NestedSegwit => "Nested SegWit",
        ScriptType::Legacy => "Legacy",
    };
    let scripts: Vec<(String, Style, Action)> = ScriptType::ALL
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (
                script_name(*s).to_string(),
                if *s == r.script {
                    Style::Primary
                } else {
                    Style::Secondary
                },
                Action::XScript(i as u8),
            )
        })
        .collect();
    ui.text_mid(x, y, 32.0, 13.0, W::R, MUTED, "Script");
    y += button_rows(ui, px, y, pw, &scripts) + 6.0;
    for (label, value) in [
        ("Address", r.address.as_str()),
        ("Public key", r.public_key.as_str()),
        ("Extended key", r.xpub.as_str()),
    ] {
        ui.text(x, y, 13.0, W::R, MUTED, label);
        y += ui.wrap(px, y, pw, 14.0, W::M, TEXT, value).max(20.0) + 12.0;
    }
}
