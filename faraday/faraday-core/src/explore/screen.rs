//! The Explore screen: a loaded key's public side at a path.

use crate::drawing::View;
use crate::strings::EN;
use faraday_bip::keys::ScriptType;

use crate::Action;
use crate::explore::{PRESETS, Page, XAction};
use crate::screens::{button_rows, guide_text_learn, title};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use faraday_draw::tokens::{COLUMN_MAX, EXPLORE_INSET, PATH_FIELD_W};

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(e) = app.explore.state() else {
        return;
    };
    match e.page {
        Page::Addresses => return super::pages::addresses(app, ui, x0, cw, h),
        Page::Find => return super::pages::find(app, ui, x0, cw, h),
        Page::Path => {}
    }
    let compact = ui.compact;
    let (x, width, top, mut y) = frame(app, ui, x0, cw);
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
    ui.text_mid(
        x,
        y,
        44.0,
        13.0,
        W::R,
        Role::OnSurfaceVariant,
        EN.explore_path,
    );
    let (px, pw) = if compact {
        y += 34.0;
        (x, width)
    } else {
        (x + 120.0, width - 120.0)
    };
    let err = app.explore_path().err();
    let path = format!("m/{}{}", e.path, ui.caret_char());
    // The field is as wide as a path (`docs/LAYOUT.md` §8): on the wide
    // layout at least `PATH_FIELD_W`, wider as the path grows, never past
    // the column; the small panel's takes the panel.
    let fw = if compact {
        pw
    } else {
        (ui.measure(15.0, W::M, &path) + 24.0).clamp(PATH_FIELD_W.min(pw), pw)
    };
    ui.fill(px, y, fw, 44.0, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(
        px,
        y,
        fw,
        44.0,
        8.0,
        if err.is_some() {
            Role::Error
        } else {
            Role::Primary
        },
    );
    let path = ui.fit(15.0, W::M, &path, fw - 24.0);
    ui.text_mid(px + 12.0, y, 44.0, 15.0, W::M, Role::OnSurface, &path);
    y += 52.0;
    if let Some(m) = err {
        y += ui.wrap(px, y, pw, 13.0, W::R, Role::Error, m) + 8.0;
    }
    let presets: Vec<(String, Style, Action)> = PRESETS
        .iter()
        .enumerate()
        .map(|(i, (name, _))| (name.to_string(), Style::Ghost, Action::XPreset(i as u8)))
        .chain([
            (
                EN.explore_index_prev.to_string(),
                Style::Ghost,
                Action::XIndex(-1),
            ),
            (
                EN.explore_index_next.to_string(),
                Style::Ghost,
                Action::XIndex(1),
            ),
        ])
        .collect();
    y += button_rows(ui, px, y, pw, &presets) + 8.0;
    let Some(r) = app.explore_reading() else {
        crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 16.0);
        return;
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
    ui.text_mid(
        x,
        y,
        32.0,
        13.0,
        W::R,
        Role::OnSurfaceVariant,
        EN.explore_script,
    );
    if compact {
        y += 32.0;
    }
    y += button_rows(ui, px, y, pw, &scripts) + 6.0;
    // Each value whole, in mono; on the wide layout in groups of four,
    // wrapping between groups (`docs/LAYOUT.md` §17, DESIGN §4.5).
    for (label, value) in [
        (EN.explore_address, r.address.as_str()),
        (EN.explore_public_key, r.public_key.as_str()),
        (EN.explore_extended_key, r.xpub.as_str()),
    ] {
        ui.text(x, y, 13.0, W::R, Role::OnSurfaceVariant, label);
        if compact {
            y += 20.0;
        }
        let shown = if compact {
            value.to_string()
        } else {
            faraday_ui::grouped(value)
        };
        y += ui
            .wrap(px, y, pw, 14.0, W::M, Role::OnSurface, &shown)
            .max(20.0)
            + 12.0;
    }
    // The account the path is in: its addresses, and a search of them.
    if app.explore_account().is_some_and(|of| of.4) {
        let more = [
            (
                EN.explore_addresses.to_string(),
                Style::Secondary,
                Action::X(XAction::Page(Page::Addresses)),
            ),
            (
                EN.explore_find.to_string(),
                Style::Secondary,
                Action::X(XAction::Page(Page::Find)),
            ),
        ];
        y += button_rows(ui, x, y, width, &more);
    }
    if compact {
        y += crate::compact::screen::about_learn(
            ui,
            x,
            y,
            width,
            guide,
            Some(crate::learn::Topic::Xpubs),
        );
    }
    crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 16.0);
}

/// A script's name, as the Script row's buttons say it.
pub(super) fn script_name(s: ScriptType) -> &'static str {
    match s {
        ScriptType::NativeSegwit => EN.explore_native_segwit,
        ScriptType::Taproot => EN.explore_taproot,
        ScriptType::NestedSegwit => EN.explore_nested_segwit,
        ScriptType::Legacy => EN.explore_legacy,
    }
}

/// Where a page of Explore goes: its left edge, its width, the top of
/// its content as scrolled and where it starts. A small panel has its
/// bar for the way back and the name, and puts each value under its
/// name.
pub(super) fn frame(app: &View, ui: &Ui, x0: f32, cw: f32) -> (f32, f32, f32, f32) {
    let (x, width) = if ui.compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (
            x0 + EXPLORE_INSET,
            (cw - 2.0 * EXPLORE_INSET).min(COLUMN_MAX),
        )
    };
    let top = 12.0 - app.list_offset;
    let start = if ui.compact {
        top
    } else {
        36.0 - app.list_offset
    };
    (x, width, top, start)
}

/// The way back, the title and what explains the page, as the desktop
/// draws them. Returns where the page goes on.
fn header(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32, guide: &str) -> f32 {
    let Some(e) = app.explore.state() else {
        return y;
    };
    let y = head(
        ui,
        x,
        y,
        app.named_back((EN.explore_wallets, Action::Nav(e.back))),
        EN.explore_explore,
    );
    y + guide_text_learn(app, ui, x, y, width, guide, crate::learn::Topic::Xpubs)
}

/// The way back, named, and the title, as the desktop draws them.
/// Returns where the page goes on.
pub(super) fn head(ui: &mut Ui, x: f32, mut y: f32, back: (&str, Action), name: &str) -> f32 {
    ui.icon(
        x - 4.0,
        y,
        16.0,
        faraday_draw::widgets::Icon::ChevronLeft,
        10.0,
        Role::OnSurfaceVariant,
    );
    ui.text(x + 14.0, y, 12.0, W::R, Role::OnSurfaceVariant, back.0);
    ui.hit(x - 4.0, y - 4.0, 70.0, 24.0, back.1);
    ui.back(back.1);
    y += 22.0;
    title(ui, x, y, name);
    y + 52.0
}
