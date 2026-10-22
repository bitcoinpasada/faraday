//! The Explore screen: a loaded key's public side at a path.

use crate::drawing::View;
use crate::strings::EN;
use faraday_bip::keys::ScriptType;

use crate::Action;
use crate::explore::PRESETS;
use crate::screens::{button_rows, guide_text, title};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(e) = app.explore.as_ref() else {
        return;
    };
    // A small panel has its bar for the way back and the name, and puts
    // each value under its name.
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 56.0, (cw - 112.0).min(900.0))
    };
    let top = 12.0 - app.list_offset;
    let mut y = if compact { top } else { 36.0 };
    let guide = EN.explore_the_public;
    if !compact {
        y = header(app, ui, x, y, width, guide);
    }
    let keys: Vec<(String, Style, Action)> = app
        .session
        .keys
        .all()
        .map(|k| {
            let fp = k.fingerprint;
            (
                crate::wallet::key_line(fp, &k.label),
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
    ui.text_mid(x, y, 44.0, 13.0, W::R, MUTED, EN.explore_path);
    let (px, pw) = if compact {
        y += 34.0;
        (x, width)
    } else {
        (x + 120.0, width - 120.0)
    };
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
    let path = format!("m/{}{}", e.path, ui.caret_char());
    let path = ui.fit(15.0, W::M, &path, pw - 24.0);
    ui.text_mid(px + 12.0, y, 44.0, 15.0, W::M, TEXT, &path);
    y += 52.0;
    if let Some(m) = err {
        y += ui.wrap(px, y, pw, 13.0, W::R, ERR, m) + 8.0;
    }
    let presets: Vec<(String, Style, Action)> = PRESETS
        .iter()
        .enumerate()
        .map(|(i, (name, _))| (name.to_string(), Style::Ghost, Action::XPreset(i as u8)))
        .chain([
            (
                EN.explore_index_1_2.to_string(),
                Style::Ghost,
                Action::XIndex(-1),
            ),
            (
                EN.explore_index_1.to_string(),
                Style::Ghost,
                Action::XIndex(1),
            ),
        ])
        .collect();
    y += button_rows(ui, px, y, pw, &presets) + 8.0;
    let Some(r) = app.explore_reading() else {
        if compact {
            crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 16.0);
        }
        return;
    };
    let script_name = |s: ScriptType| match s {
        ScriptType::NativeSegwit => EN.explore_native_segwit,
        ScriptType::Taproot => EN.explore_taproot,
        ScriptType::NestedSegwit => EN.explore_nested_segwit,
        ScriptType::Legacy => EN.explore_legacy,
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
    ui.text_mid(x, y, 32.0, 13.0, W::R, MUTED, EN.explore_script);
    if compact {
        y += 32.0;
    }
    y += button_rows(ui, px, y, pw, &scripts) + 6.0;
    let shown = app.session.shown(&r.address);
    for (label, value) in [
        (EN.explore_address, shown.as_str()),
        (EN.explore_public_key, r.public_key.as_str()),
        (EN.explore_extended_key, r.xpub.as_str()),
    ] {
        ui.text(x, y, 13.0, W::R, MUTED, label);
        if compact {
            y += 20.0;
        }
        y += ui.wrap(px, y, pw, 14.0, W::M, TEXT, value).max(20.0) + 12.0;
    }
    if compact {
        y += crate::compact::screen::about(ui, x, y, width, guide);
        crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 16.0);
    }
}

/// The way back, the title and what explains the page, as the desktop
/// draws them. Returns where the page goes on.
fn header(app: &View, ui: &mut Ui, x: f32, mut y: f32, width: f32, guide: &str) -> f32 {
    let Some(e) = app.explore.as_ref() else {
        return y;
    };
    ui.icon(
        x - 4.0,
        y,
        16.0,
        faraday_draw::widgets::Icon::ChevronLeft,
        10.0,
        MUTED,
    );
    ui.text(x + 14.0, y, 12.0, W::R, MUTED, EN.explore_wallets);
    ui.hit(x - 4.0, y - 4.0, 70.0, 24.0, Action::Nav(e.back));
    ui.back(Action::Nav(e.back));
    y += 22.0;
    title(ui, x, y, EN.explore_explore);
    y += 52.0;
    y += guide_text(app, ui, x, y, width, guide);
    y
}
