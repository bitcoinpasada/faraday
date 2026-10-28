//! The Explore screen: a loaded key at a path, its public side shown and
//! its secrets each behind an eye ([`super::shown`]).

use crate::drawing::View;
use crate::strings::EN;
use faraday_bip::keys::ScriptType;

use crate::Action;
use crate::explore::shown::xprv_path;
use crate::explore::{Eye, PRESETS, Page, Shown, XAction};
use crate::screens::{button_rows, guide_text_learn, title};
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{self as T, COLUMN_MAX, EXPLORE_INSET, PATH_FIELD_W};
use faraday_draw::tokens::{BODY_L, BODY_M, BODY_S, FIELD_H, FIELD_PAD};
use faraday_keys::Reveal;
use zeroize::Zeroizing;

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
                Style::chosen(e.key == Some(fp.0)),
                Action::XKey(fp.0),
            )
        })
        .collect();
    y += button_rows(ui, x, y, width, &keys) + 4.0;
    let (px, pw) = if compact {
        (x, width)
    } else {
        (x + 120.0, width - 120.0)
    };
    let cols = Cols { x, px, pw, width };
    y += secret(app, ui, cols, y);
    // The path.
    ui.text_mid_as(
        (x, y, FIELD_H),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.explore_path,
    );
    if compact {
        y += 34.0;
    }
    let err = app.explore_path().err();
    let path = format!("m/{}{}", e.path, ui.caret_char());
    // The field is as wide as a path (`docs/LAYOUT.md` §8): on the wide
    // layout at least `PATH_FIELD_W`, wider as the path grows, never past
    // the column; the small panel's takes the panel.
    let fw = if compact {
        pw
    } else {
        (ui.measure_as(BODY_L.mono(), &path) + 24.0).clamp(PATH_FIELD_W.min(pw), pw)
    };
    // Dropped to the box's own width: its error line wraps under it.
    let boxed = ui.field_box((px, y, fw), true, err);
    let path = ui.fit_as(BODY_L.mono(), &path, fw - 2.0 * FIELD_PAD);
    ui.text_mid_as(
        (px + FIELD_PAD, y, FIELD_H),
        BODY_L.mono(),
        Role::OnSurface,
        &path,
    );
    y += boxed + 8.0;
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
                Style::chosen(*s == r.script),
                Action::XScript(i as u8),
            )
        })
        .collect();
    ui.text_mid_as(
        (x, y, 32.0),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.explore_script,
    );
    if compact {
        y += 32.0;
    }
    y += button_rows(ui, px, y, pw, &scripts) + 6.0;
    // Each value whole, in mono; on the wide layout in groups of four,
    // wrapping between groups (`docs/LAYOUT.md` §17, DESIGN §4.5). The
    // address's private key and the extended private keys each follow
    // what they are of, behind their eyes.
    let shown = app.explore_shown();
    y += value_row(ui, cols, y, EN.explore_address, &r.address);
    let path = app.explore_path().ok();
    let wif = path.as_ref().and_then(|p| shown.and_then(|s| s.wif(p)));
    y += eye_row(ui, cols, y, EN.explore_wif, Eye::Wif, wif.is_some());
    if let Some(w) = wif {
        y += value_row(ui, cols, y, "", w);
    }
    y += value_row(ui, cols, y, EN.explore_public_key, &r.public_key);
    y += value_row(ui, cols, y, EN.explore_extended_key, &r.xpub);
    let xprvs = shown.and_then(Shown::xprvs);
    y += eye_row(ui, cols, y, EN.explore_xprvs, Eye::Xprv, xprvs.is_some());
    if let Some((at, root, account)) = xprvs {
        y += value_row(ui, cols, y, EN.explore_xprv_root, root);
        if !at.is_empty() && path.as_ref().is_some_and(|p| xprv_path(p) == *at) {
            let name = fill(EN.explore_path_m, &[&at.to_string().replace('\'', "h")]);
            y += value_row(ui, cols, y, &name, account);
        }
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

/// Where the path page's rows go: the column's left edge, the values'
/// left edge and width (beside their names on the wide layout, under them
/// on a small panel) and the column's width.
#[derive(Clone, Copy)]
pub(super) struct Cols {
    pub(super) x: f32,
    pub(super) px: f32,
    pub(super) pw: f32,
    pub(super) width: f32,
}

/// A value under its name, `label` (none where it is empty): whole, in
/// mono, on the wide layout beside the name in groups of four wrapping
/// between groups. A secret value is grouped into a buffer wiped when the
/// frame drops it, and is never copied otherwise. Returns the height used.
pub(super) fn value_row(ui: &mut Ui, at: Cols, y: f32, label: &str, value: &str) -> f32 {
    let Cols { px, pw, .. } = at;
    let mut cy = y;
    if !label.is_empty() {
        cy += name(ui, at, cy, label);
    }
    let grouped = grouped(ui.compact, value);
    cy += ui
        .wrap_as((px, cy, pw), BODY_M.mono(), Role::OnSurface, &grouped)
        .max(20.0)
        + 12.0;
    cy - y
}

/// A value's name at the column's left edge. Returns the height the
/// value goes below it: nothing on the wide layout, where the value is
/// beside it.
fn name(ui: &mut Ui, at: Cols, y: f32, label: &str) -> f32 {
    ui.text_as(at.x, y, BODY_S, Role::OnSurfaceVariant, label);
    if ui.compact { 20.0 } else { 0.0 }
}

/// `value` in groups of four on the wide layout, whole on a small panel,
/// in a buffer that wipes itself when dropped.
fn grouped(compact: bool, value: &str) -> Zeroizing<String> {
    let mut out = Zeroizing::new(String::with_capacity(value.len() + value.len() / 4 + 1));
    for (i, ch) in value.chars().enumerate() {
        if !compact && i > 0 && i % 4 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

/// A row `TOUCH` tall: `label`, and at the column's right end Explore's
/// eye on what follows it, Material's standard icon button as a toggle:
/// the open eye while hidden, struck and filled while shown (the icon
/// shows what a press does); its tooltip says Show or Hide. Returns the
/// height used.
pub(super) fn eye_row(ui: &mut Ui, at: Cols, y: f32, label: &str, eye: Eye, shown: bool) -> f32 {
    let Cols { x, width, .. } = at;
    ui.text_mid_as((x, y, T::TOUCH), BODY_S, Role::OnSurfaceVariant, label);
    eye_button(ui, x + width - T::TOUCH, y, eye, shown);
    T::TOUCH
}

/// Explore's eye in a `TOUCH` square at (x, y): Material's standard icon
/// button, a toggle, `ICON_BUTTON` across in the middle of its target.
pub(super) fn eye_button(ui: &mut Ui, x: f32, y: f32, eye: Eye, shown: bool) {
    let a = Action::X(XAction::Eye(eye));
    let edge = (T::TOUCH - T::ICON_BUTTON) / 2.0;
    let (bx, by) = (x + edge, y + edge);
    if shown || ui.is_pressed(a) {
        ui.fill(
            bx,
            by,
            T::ICON_BUTTON,
            T::ICON_BUTTON,
            T::shape_full(T::ICON_BUTTON),
            crate::ui::pal::selected_or(shown, Role::SurfaceContainerHighest),
        );
    }
    ui.icon(
        bx,
        by,
        T::ICON_BUTTON,
        if shown {
            faraday_draw::widgets::Icon::EyeOff
        } else {
            faraday_draw::widgets::Icon::Eye
        },
        T::ICON_GLYPH,
        crate::ui::pal::on_selected(shown, Role::OnSurfaceVariant),
    );
    crate::compact::eye_hit(ui, (x, y, T::TOUCH, T::TOUCH), a, shown);
}

/// The key's own secret, behind its eye: its words in their numbered
/// grid and its passphrase, lent by the store for this frame; or, for a
/// key with no words, its seed's forms as the eye's press made them (the
/// seed in hex, its codex32 secret, the codex32 strings held). Returns
/// the height used.
fn secret(app: &View, ui: &mut Ui, at: Cols, y: f32) -> f32 {
    let Some(place) = app
        .explore
        .state()
        .and_then(|e| e.key)
        .and_then(|fp| app.session.keys.position_of(fp))
    else {
        return 0.0;
    };
    let shown = app.explore_shown();
    let on = shown.is_some_and(Shown::secret);
    let mut cy = y + eye_row(ui, at, y, EN.explore_secret, Eye::Secret, on);
    if !on {
        return cy - y;
    }
    if let Some(forms) = shown.and_then(Shown::seed) {
        for (k, form) in forms.iter().enumerate() {
            let index = form.as_bytes().get(8).copied().unwrap_or(b's');
            let name = match (k, index) {
                (0, _) => EN.explore_master_seed.to_string(),
                (_, b's') => EN.explore_codex32_secret.to_string(),
                (_, i) => fill(EN.explore_codex32_share, &[&char::from(i).to_string()]),
            };
            cy += value_row(ui, at, cy, &name, form);
        }
        return cy - y;
    }
    let words = app.reveal(place, Reveal::Words);
    cy += name(ui, at, cy, EN.explore_words);
    cy += word_grid(ui, &words, at.px, cy, at.pw);
    if app.session.keys[place].has_passphrase {
        let p = app.reveal(place, Reveal::Passphrase);
        cy += value_row(ui, at, cy, EN.explore_passphrase, &p);
    }
    cy - y
}

/// `words` in their numbered grid, as they are written on paper: four
/// columns, two on a small panel. Returns the height used.
fn word_grid(ui: &mut Ui, words: &str, x: f32, y: f32, w: f32) -> f32 {
    let cols = if ui.compact { 2 } else { 4 };
    let gap = T::ENTRY_CELL_GAP;
    let cell = (w - (cols - 1) as f32 * gap) / cols as f32;
    let mut n = 0;
    for (i, word) in words.split_whitespace().enumerate() {
        let cx = x + (i % cols) as f32 * (cell + gap);
        let cy = y + (i / cols) as f32 * T::ENTRY_ROW;
        let h = T::ENTRY_CELL_HEIGHT;
        ui.fill(
            cx,
            cy,
            cell,
            h,
            T::FIELD_RADIUS,
            Role::SurfaceContainerHighest,
        );
        ui.stroke(cx, cy, cell, h, T::FIELD_RADIUS, Role::OutlineVariant);
        let number = (i + 1).to_string();
        ui.text_mid_as((cx + gap, cy, h), BODY_S, Role::OnSurfaceVariant, &number);
        let wx = cx + gap + T::ENTRY_CELL_NUMBER;
        ui.text_mid_as((wx, cy, h), BODY_M.mono(), Role::OnSurface, word);
        n = i + 1;
    }
    n.div_ceil(cols) as f32 * T::ENTRY_ROW + T::GAP
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
    ui.text_as(x + 14.0, y, BODY_S, Role::OnSurfaceVariant, back.0);
    ui.hit(x - 4.0, y - 4.0, 70.0, 24.0, back.1);
    ui.back(back.1);
    y += 22.0;
    title(ui, x, y, name);
    y + 52.0
}
