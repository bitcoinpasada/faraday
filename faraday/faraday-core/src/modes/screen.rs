//! The mode pill, the mode sheet and Settings' Mode row (F81,
//! `docs/MODES.md` §3). The pill names the mode off Mainnet and opens the
//! sheet; the sheet lists the modes the app offers, the one in use marked;
//! a press on another is `Action::Mode`, a lock into it. Drawing reads the
//! state and works nothing out.

use super::Mode;
use crate::Action;
use crate::compact::{buttons, scroll_sheet, sheet_head};
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Paint;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::network_name;
use faraday_draw::tokens::{BODY_M, LABEL_L, LABEL_M};
use faraday_draw::tokens::{
    GAP, LABEL, MODE_PILL_HEIGHT, MODE_PILL_PAD, MODE_ROW, MODE_SHEET_WIDTH, PAD,
};
use faraday_draw::widgets::Icon;

/// Whether the pill shows: in every mode but Mainnet.
pub(crate) fn shows_pill(app: &View) -> bool {
    app.mode() != Mode::Mainnet
}

/// The pill's label: the mode's name, and in Practice on a window wider
/// than the small panel its network too. On the small panel the bar has
/// no room for more than the name (`docs/MODES.md` §3).
fn pill_label(app: &View, ui: &Ui) -> &'static str {
    match app.mode() {
        Mode::Demo if !ui.compact => EN.mode_demo_pill,
        m => m.name(),
    }
}

/// The mode pill's own width.
fn mode_width(app: &View, ui: &Ui) -> f32 {
    ui.measure_as(LABEL_M, pill_label(app, ui)) + 2.0 * MODE_PILL_PAD
}

/// The pill's width, for a row that makes room for it. Load samples is
/// Faraday Demo's, in its host strip (`docs/DESKTOP-HOST.md`).
pub(crate) fn pill_width(app: &View, ui: &Ui) -> f32 {
    mode_width(app, ui)
}

/// One pill at (`px`, `py`), `pw` wide, in `tone`, pressed for `action`.
fn chip(ui: &mut Ui, (px, py, pw): (f32, f32, f32), label: &str, tone: Paint, action: Action) {
    let h = MODE_PILL_HEIGHT;
    // In its tone's container, never a tint (`docs/DESIGN.md` §3).
    let (bg, fg) = crate::ui::container_of(tone);
    let shape = (px, py, pw, h, h / 2.0);
    ui.fill(px, py, pw, h, h / 2.0, bg);
    ui.text_mid_as((px + MODE_PILL_PAD, py, h), LABEL_M, fg, label);
    ui.hit_shaped(shape, action);
}

/// The mode pill at (`px`, `py`), in the caution tone: it names the
/// mode and opens the sheet. Returns its width.
pub(crate) fn pill(app: &View, ui: &mut Ui, px: f32, py: f32) -> f32 {
    let mw = mode_width(app, ui);
    let label = pill_label(app, ui);
    chip(
        ui,
        (px, py, mw),
        label,
        Role::Warning.paint(),
        Action::ModeSheet,
    );
    mw
}

/// Settings' Mode row: "Mode" and the mode's name, a chevron; it opens
/// the sheet. Returns its height.
pub(crate) fn settings_row(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    ui.text_mid_as((x, y, MODE_ROW), LABEL_L, Role::OnSurface, EN.settings_mode);
    ui.icon(
        x + w - MODE_ROW / 2.0 - GAP,
        y,
        MODE_ROW,
        Icon::ChevronRight,
        LABEL,
        Role::OnSurfaceVariant,
    );
    let name = app.mode().name();
    let nw = ui.measure_as(BODY_M, name);
    ui.text_mid_as(
        (x + w - 3.0 * GAP - nw, y, MODE_ROW),
        BODY_M,
        Role::OnSurfaceVariant,
        name,
    );
    ui.hit(x, y, w, MODE_ROW, Action::ModeSheet);
    MODE_ROW
}

/// The sheet: a row per mode the app offers, the one in use marked, Not
/// now at the foot. Wide, centred at most the Load sheet's width; compact,
/// as the Load sheet is.
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let place = place(ui, w);
    scroll_sheet(ui, place.0, h, place.1, place.2, &mut |ui, x, y, iw| {
        body(app, ui, x, y, iw)
    });
}

/// Where a sheet of this width sits: its left edge and width, its margin
/// and its padding. Wide, centred at most the Load sheet's width; compact,
/// as the Load sheet is.
pub(crate) fn place(ui: &Ui, w: f32) -> ((f32, f32), f32, f32) {
    if ui.compact {
        ((GAP, w - 2.0 * GAP), GAP, PAD)
    } else {
        let sw = (w - 3.0 * PAD).min(MODE_SHEET_WIDTH);
        (((w - sw) / 2.0, sw), 2.0 * PAD, 2.0 * PAD)
    }
}

/// A sheet's title with its icon: the compact head, or the wide one, which
/// is larger. Returns its height. Shared by the sheets of this area and of
/// Load samples.
pub(crate) fn head(ui: &mut Ui, (x, y, w): (f32, f32, f32), icon: Icon, title: &str) -> f32 {
    if ui.compact {
        sheet_head(ui, x, y, w, icon, Role::Primary, title)
    } else {
        ui.icon(x, y, 30.0, icon, 18.0, Role::Primary);
        let t = ui.fit_as(faraday_draw::tokens::sheet_head(false), title, w - 42.0);
        ui.text_mid_as(
            (x + 42.0, y, 30.0),
            faraday_draw::tokens::sheet_head(false),
            Role::OnSurface,
            &t,
        );
        46.0
    }
}

/// The title, the rows and Not now. Returns its height.
fn body(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    cy += head(ui, (x, cy, w), Icon::Settings, EN.mode_sheet_title);
    let (now, fixed) = (app.mode(), app.session.keys.network_fixed());
    let modes = app.modes();
    for (i, &m) in modes.iter().enumerate() {
        let ry = cy + i as f32 * MODE_ROW;
        let live = m == now || !fixed;
        let action = Action::Mode(m);
        if live && (ui.is_pressed(action) || ui.is_hovered(action)) {
            ui.wash(
                x,
                ry + 2.0,
                w,
                MODE_ROW - 4.0,
                8.0,
                Role::OnSurface.at(faraday_draw::tokens::STATE_PRESS),
            );
        }
        mode_row(ui, x, ry, w, m, m == now, live);
        if i + 1 < modes.len() {
            ui.rule(x, ry + MODE_ROW, w, Role::OutlineVariant);
        }
        if live {
            ui.hit(x, ry, w, MODE_ROW, action);
        }
    }
    cy += modes.len() as f32 * MODE_ROW + PAD;
    cy += buttons(
        ui,
        x,
        cy,
        w,
        &[(EN.mode_not_now, Style::Secondary, Action::Cancel)],
    );
    cy - y
}

/// One row: the mode's name, Practice's network as its value, and the mark on
/// the one in use. A mode that cannot be chosen is dimmed, its label alone.
fn mode_row(ui: &mut Ui, x: f32, ry: f32, w: f32, m: Mode, on: bool, live: bool) {
    let mark = MODE_ROW;
    let fg = if live {
        Role::OnSurface.paint()
    } else {
        crate::ui::off()
    };
    ui.text_mid_as((x + GAP, ry, MODE_ROW), BODY_M, fg, m.name());
    if on {
        ui.icon(x + w - mark, ry, mark, Icon::Done, LABEL, Role::Primary);
    }
    if m == Mode::Demo {
        let value = network_name(m.network());
        let vw = ui.measure_as(BODY_M, value);
        let vc = if live {
            Role::OnSurfaceVariant.paint()
        } else {
            crate::ui::off()
        };
        ui.text_mid_as((x + w - mark - GAP - vw, ry, MODE_ROW), BODY_M, vc, value);
    }
}
