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
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::network_name;
use faraday_draw::tokens::{
    CAPTION, GAP, LABEL, MODE_PILL_HEIGHT, MODE_PILL_PAD, MODE_ROW, MODE_SHEET_WIDTH, PAD, TITLE,
};
use faraday_draw::widgets::Icon;

/// Whether the pill shows: in every mode but Mainnet.
pub(crate) fn shows_pill(app: &View) -> bool {
    app.mode() != Mode::Mainnet
}

/// The pill's label: the mode's name, and in Demo its network too.
fn pill_label(app: &View) -> &'static str {
    match app.mode() {
        Mode::Demo => EN.mode_demo_pill,
        m => m.name(),
    }
}

/// The pill's width, for a row that makes room for it.
pub(crate) fn pill_width(app: &View, ui: &Ui) -> f32 {
    ui.measure(CAPTION, W::S, pill_label(app)) + 2.0 * MODE_PILL_PAD
}

/// The caution pill at (`px`, `py`): the mode, pressed to open the sheet.
/// Returns its width.
pub(crate) fn pill(app: &View, ui: &mut Ui, px: f32, py: f32) -> f32 {
    let pw = pill_width(app, ui);
    let h = MODE_PILL_HEIGHT;
    ui.fill(px, py, pw, h, h / 2.0, WARN.with_alpha(30));
    ui.stroke(px, py, pw, h, h / 2.0, WARN.with_alpha(110));
    ui.text_mid(
        px + MODE_PILL_PAD,
        py,
        h,
        CAPTION,
        W::S,
        WARN,
        pill_label(app),
    );
    ui.hit(px, py, pw, h, Action::ModeSheet);
    pw
}

/// Settings' Mode row: "Mode" and the mode's name, a chevron; it opens
/// the sheet. Returns its height.
pub(crate) fn settings_row(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    ui.text_mid(x, y, MODE_ROW, LABEL, W::S, TEXT, EN.settings_mode);
    ui.icon(
        x + w - MODE_ROW / 2.0 - GAP,
        y,
        MODE_ROW,
        Icon::ChevronRight,
        LABEL,
        MUTED,
    );
    let name = app.mode().name();
    let nw = ui.measure(LABEL, W::R, name);
    ui.text_mid(
        x + w - 3.0 * GAP - nw,
        y,
        MODE_ROW,
        LABEL,
        W::R,
        MUTED,
        name,
    );
    ui.hit(x, y, w, MODE_ROW, Action::ModeSheet);
    MODE_ROW
}

/// The sheet: a row per mode the app offers, the one in use marked, Not
/// now at the foot. Wide, centred at most the Load sheet's width; compact,
/// as the Load sheet is.
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let place = if ui.compact {
        ((GAP, w - 2.0 * GAP), GAP, PAD)
    } else {
        let sw = (w - 3.0 * PAD).min(MODE_SHEET_WIDTH);
        (((w - sw) / 2.0, sw), 2.0 * PAD, 2.0 * PAD)
    };
    scroll_sheet(ui, place.0, h, place.1, place.2, &mut |ui, x, y, iw| {
        body(app, ui, x, y, iw)
    });
}

/// The title, the rows and Not now. Returns its height.
fn body(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    cy += if ui.compact {
        sheet_head(ui, x, cy, w, Icon::Settings, ACCENT, EN.mode_sheet_title)
    } else {
        ui.icon(x, cy, 30.0, Icon::Settings, 18.0, ACCENT);
        let t = ui.fit(TITLE, W::S, EN.mode_sheet_title, w - 42.0);
        ui.text_mid(x + 42.0, cy, 30.0, TITLE, W::S, TEXT, &t);
        46.0
    };
    let (now, fixed) = (app.mode(), app.session.keys.network_fixed());
    let modes = app.modes();
    for (i, &m) in modes.iter().enumerate() {
        let ry = cy + i as f32 * MODE_ROW;
        let live = m == now || !fixed;
        let action = Action::Mode(m);
        if live && (ui.is_pressed(action) || ui.is_hovered(action)) {
            ui.fill(x, ry + 2.0, w, MODE_ROW - 4.0, 8.0, INNER);
        }
        mode_row(ui, x, ry, w, m, m == now, live);
        if i + 1 < modes.len() {
            ui.rule(x, ry + MODE_ROW, w, INNER);
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

/// One row: the mode's name, Demo's network as its value, and the mark on
/// the one in use. A mode that cannot be chosen is dimmed, its label alone.
fn mode_row(ui: &mut Ui, x: f32, ry: f32, w: f32, m: Mode, on: bool, live: bool) {
    let mark = MODE_ROW;
    let fg = if live { TEXT } else { DIM };
    ui.text_mid(x + GAP, ry, MODE_ROW, LABEL, W::R, fg, m.name());
    if on {
        ui.icon(x + w - mark, ry, mark, Icon::Done, LABEL, ACCENT);
    }
    if m == Mode::Demo {
        let value = network_name(m.network());
        let vw = ui.measure(LABEL, W::R, value);
        let vc = if live { MUTED } else { DIM };
        ui.text_mid(
            x + w - mark - GAP - vw,
            ry,
            MODE_ROW,
            LABEL,
            W::R,
            vc,
            value,
        );
    }
}
