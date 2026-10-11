//! `Screen::Codes` (`docs/TOTP.md` §5): one row per vault entry with a
//! TOTP secret, across every open vault, each with its code or why it
//! has none.

use crate::codes::{CodeKind, CodeRow, codes_rows, va};
use crate::screens::title;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::vaults::VaultAction as V;
use crate::{Action, Faraday};

/// The value row at the top: local time with seconds under its label and
/// the zone, and Set the time (§5.3). The zone is in the label, so the
/// time itself is never cut short on a narrow panel.
fn now_row(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let label = format!("Now · UTC{}", crate::vaults::tz_offset_text(app.tz_min));
    ui.text(x, y + 10.0, 12.0, W::R, MUTED, &label);
    let text = match app.clock_now() {
        Some(t) => crate::vaults::time_of_day_secs(crate::vaults::local_secs(t, app.tz_min)),
        None => "Not known".to_string(),
    };
    let bw = ui.measure(14.0, W::S, "Set the time") + 32.0;
    let text = ui.fit(16.0, W::S, &text, (w - bw - 12.0).max(0.0));
    ui.text(x, y + 28.0, 16.0, W::S, TEXT, &text);
    ui.button(
        x + w - bw,
        y + 8.0,
        Some(bw),
        40.0,
        "Set the time",
        Style::Secondary,
        Action::OpenSetTime,
    );
    56.0
}

/// A small text pill, right-anchored at `right`, as `network_pill` draws
/// (`docs/DESIGN.md` §4.8): the HOTP and Unreadable badges.
fn pill_right(ui: &mut Ui, right: f32, y: f32, label: &str) {
    let tw = ui.measure(12.0, W::S, label);
    let pw = tw + 24.0;
    let x = right - pw;
    ui.fill(x, y, pw, 26.0, 13.0, MUTED.with_alpha(30));
    ui.stroke(x, y, pw, 26.0, 13.0, MUTED.with_alpha(110));
    ui.text_mid(x + (pw - tw) / 2.0, y, 26.0, 12.0, W::S, MUTED, label);
}

/// One row: title and account on the left, the code or a badge on the
/// right, a progress bar under it for the time left in its period.
fn code_row(ui: &mut Ui, x: f32, y: f32, w: f32, r: &CodeRow) -> f32 {
    let action = Action::CodesOpen(r.vault, r.item);
    let h = 80.0;
    let pressed = ui.is_pressed(action);
    ui.fill(x, y, w, h, 12.0, if pressed { INNER } else { SURFACE });
    ui.stroke(x, y, w, h, 12.0, LINE);
    ui.hit(x, y, w, h, action);
    let t = ui.fit(15.0, W::S, &r.title, w - 140.0);
    ui.text(x + 16.0, y + 14.0, 15.0, W::S, TEXT, &t);
    let sub = if r.account.is_empty() {
        r.vault_name.clone()
    } else if r.vault_name.is_empty() {
        r.account.clone()
    } else {
        format!("{} · {}", r.account, r.vault_name)
    };
    if !sub.is_empty() {
        let sub = ui.fit(12.0, W::R, &sub, w - 140.0);
        ui.text(x + 16.0, y + 36.0, 12.0, W::R, MUTED, &sub);
    }
    match &r.kind {
        CodeKind::Code {
            grouped,
            left,
            period,
        } => {
            let cw = ui.measure(18.0, W::M, grouped);
            ui.text(x + w - 16.0 - cw, y + 16.0, 18.0, W::M, TEXT, grouped);
            // The seconds left at the bar's end, on its line.
            let secs = format!("{left} s");
            let sw = ui.measure(11.0, W::R, "88 s") + 10.0;
            let bw = w - 32.0 - sw;
            ui.fill(x + 16.0, y + 57.0, bw, 6.0, 3.0, INNER);
            let share = if *period == 0 {
                0.0
            } else {
                (*left as f32 / *period as f32).clamp(0.0, 1.0)
            };
            ui.fill(x + 16.0, y + 57.0, (bw * share).max(6.0), 6.0, 3.0, ACCENT);
            ui.text_right(x + w - 16.0, y + 51.0, 18.0, 11.0, W::R, MUTED, &secs);
        }
        CodeKind::Hotp => pill_right(ui, x + w - 16.0, y + 14.0, "HOTP"),
        CodeKind::Unreadable => pill_right(ui, x + w - 16.0, y + 14.0, "Unreadable"),
    }
    h + 12.0
}

/// `Screen::Codes`, desktop layout.
pub(crate) fn draw(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return compact(app, ui, x0, cw, h);
    }
    let x = x0 + 48.0;
    let w = cw - 96.0;
    let mut y = 36.0;
    title(ui, x, y, "TOTP codes");
    y += 60.0;
    y += now_row(app, ui, x, y, w) + 12.0;
    let now = app.clock_now().unwrap_or(0);
    let rows = codes_rows(app, now);
    if rows.is_empty() {
        ui.text(x, y, 14.0, W::R, DIM, "No entries with a TOTP secret");
        y += 36.0;
    } else {
        for r in &rows {
            y += code_row(ui, x, y, w, r);
        }
    }
    y += 8.0;
    if app.vaults.form.is_some() {
        crate::vault_screens::codes_form(app, ui, x, y, w);
    } else {
        let scan = "Scan a setup code";
        let add = "Add an entry";
        let sw = ui.measure(14.0, W::S, scan) + 32.0;
        ui.button(
            x,
            y,
            Some(sw),
            44.0,
            scan,
            Style::Secondary,
            va(V::ScanEntry),
        );
        let aw = ui.measure(14.0, W::S, add) + 32.0;
        ui.button(
            x + sw + 10.0,
            y,
            Some(aw),
            44.0,
            add,
            Style::Secondary,
            Action::CodesAdd,
        );
    }
    let _ = h;
}

/// [`draw`] on a small panel: one column, Now first, then a row per
/// entry with the account line dropped when there is no room, the bar
/// still under each row (§5.4).
fn compact(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let x = x0 + crate::compact::M;
    let w = cw - 2.0 * crate::compact::M;
    let mut y = 8.0;
    y += now_row(app, ui, x, y, w) + 8.0;
    let now = app.clock_now().unwrap_or(0);
    let rows = codes_rows(app, now);
    if rows.is_empty() {
        ui.text(x, y, 13.0, W::R, DIM, "No entries with a TOTP secret");
        y += 32.0;
    } else {
        for r in &rows {
            y += code_row(ui, x, y, w, r);
        }
    }
    y += 8.0;
    if app.vaults.form.is_some() {
        y += crate::vault_screens::codes_form(app, ui, x, y, w) + 16.0;
    } else {
        ui.button(
            x,
            y,
            None,
            44.0,
            "Scan a setup code",
            Style::Secondary,
            va(V::ScanEntry),
        );
        y += 52.0;
        ui.button(
            x,
            y,
            None,
            44.0,
            "Add an entry",
            Style::Secondary,
            Action::CodesAdd,
        );
        y += 52.0;
    }
    crate::compact_screens::finish(app, ui, x0, cw, h, y);
}
