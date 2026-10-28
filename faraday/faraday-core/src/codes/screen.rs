//! `Screen::Codes` (`docs/TOTP.md` §5): one row per vault entry with a
//! TOTP secret, across every open vault, each with its code or why it
//! has none.

use crate::Action;
use crate::codes::{CodeKind, CodeRow, codes_rows_at, va};
use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::{Focus, SearchIn, VaultAction as V};
use faraday_draw::tokens::{BODY_L, BODY_M, BODY_S, BUTTON_H, LABEL_L, LABEL_M, TITLE_M};

/// The value row at the top: local time with seconds under its label and
/// the zone, and Set the time (§5.3). The zone is in the label, so the
/// time itself is never cut short on a narrow panel.
fn now_row(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let label = fill(
        EN.codes_now_utc,
        &[&crate::vault::tz_offset_text(app.tz_min()).to_string()],
    );
    ui.text_as(x, y + 10.0, BODY_S, Role::OnSurfaceVariant, &label);
    let text = match app.clock_now() {
        Some(t) => crate::vault::time_of_day_secs(crate::vault::local_secs(t, app.tz_min())),
        None => EN.codes_not_known.to_string(),
    };
    let bw = ui.measure_as(LABEL_L, EN.codes_set_the_time) + 32.0;
    let text = ui.fit_as(TITLE_M, &text, (w - bw - 12.0).max(0.0));
    ui.text_as(x, y + 28.0, TITLE_M, Role::OnSurface, &text);
    ui.button(
        x + w - bw,
        y + 8.0,
        Some(bw),
        BUTTON_H,
        EN.codes_set_the_time,
        Style::Secondary,
        Action::OpenSetTime,
    );
    56.0
}

/// A small text pill, right-anchored at `right`, as the network's badge draws
/// (`docs/DESIGN.md` §4.8): the HOTP and Unreadable badges.
fn pill_right(ui: &mut Ui, right: f32, y: f32, label: &str) {
    let tw = ui.measure_as(LABEL_M, label);
    let pw = tw + 24.0;
    let x = right - pw;
    ui.fill(
        x,
        y,
        pw,
        26.0,
        faraday_draw::tokens::shape_full(26.0),
        Role::SurfaceContainerHighest,
    );
    ui.stroke(
        x,
        y,
        pw,
        26.0,
        faraday_draw::tokens::shape_full(26.0),
        Role::OutlineVariant,
    );
    ui.text_mid_as(
        (x + (pw - tw) / 2.0, y, 26.0),
        LABEL_M,
        Role::OnSurfaceVariant,
        label,
    );
}

/// One row: title and account on the left, the code or a badge on the
/// right, a progress bar under it for the time left in its period.
fn code_row(ui: &mut Ui, x: f32, y: f32, w: f32, r: &CodeRow) -> f32 {
    let action = Action::CodesOpen(r.vault, r.item);
    let h = 80.0;
    let shape = faraday_draw::tokens::SHAPE_M;
    ui.fill(x, y, w, h, shape, Role::SurfaceContainerLow);
    ui.stroke(x, y, w, h, shape, Role::OutlineVariant);
    ui.hit_shaped((x, y, w, h, shape), action);
    let t = ui.fit_as(TITLE_M, &r.title, w - 140.0);
    ui.text_as(x + 16.0, y + 14.0, TITLE_M, Role::OnSurface, &t);
    let sub = if r.account.is_empty() {
        r.vault_name.clone()
    } else if r.vault_name.is_empty() {
        r.account.clone()
    } else {
        format!("{} · {}", r.account, r.vault_name)
    };
    if !sub.is_empty() {
        let sub = ui.fit_as(BODY_S, &sub, w - 140.0);
        ui.text_as(x + 16.0, y + 36.0, BODY_S, Role::OnSurfaceVariant, &sub);
    }
    match &r.kind {
        CodeKind::Code {
            grouped,
            left,
            period,
        } => {
            let cw = ui.measure_as(BODY_L.mono(), grouped);
            ui.text_as(
                x + w - 16.0 - cw,
                y + 16.0,
                BODY_L.mono(),
                Role::OnSurface,
                grouped,
            );
            // The seconds left at the bar's end, on its line.
            let secs = format!("{left} s");
            let sw = ui.measure_as(BODY_S, "88 s") + 10.0;
            let bw = w - 32.0 - sw;
            ui.fill(
                x + 16.0,
                y + 57.0,
                bw,
                6.0,
                3.0,
                Role::SurfaceContainerHighest,
            );
            let share = if *period == 0 {
                0.0
            } else {
                (*left as f32 / *period as f32).clamp(0.0, 1.0)
            };
            ui.fill(
                x + 16.0,
                y + 57.0,
                (bw * share).max(6.0),
                6.0,
                3.0,
                Role::Primary,
            );
            ui.text_right_as(
                (x + w - 16.0, y + 51.0, 18.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &secs,
            );
        }
        CodeKind::Hotp | CodeKind::Unreadable | CodeKind::NoClock => {
            let badge = match r.kind {
                CodeKind::Hotp => EN.codes_hotp,
                CodeKind::Unreadable => EN.codes_unreadable,
                _ => EN.codes_set_the_time_first,
            };
            pill_right(ui, x + w - 16.0, y + 14.0, badge);
        }
    }
    h + 12.0
}

/// Whether the search bar holds anything.
fn searched(app: &View) -> bool {
    !app.vaults.query(SearchIn::Codes).is_empty()
}

/// The search bar over the rows, while there are rows to search or the
/// bar holds something (`docs/TOTP.md` §5.3). Returns the height it takes.
fn search(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, none: bool) -> f32 {
    if none && !searched(app) {
        return 0.0;
    }
    crate::vault::screen::search_bar(app, ui, x, y, w, SearchIn::Codes)
}

/// The line in place of the rows: no entry with a TOTP secret, or none
/// the search bar's query holds.
fn none_line(app: &View) -> &'static str {
    if searched(app) {
        EN.codes_search_none
    } else {
        EN.codes_no_totp_entries
    }
}

/// `Screen::Codes`, desktop layout.
pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return compact(app, ui, x0, cw, h);
    }
    let x = x0 + 48.0;
    let w = cw - 96.0;
    let mut y = crate::screens::bar_head(ui, None, EN.codes_totp_codes);
    y += now_row(app, ui, x, y, w) + 12.0;
    let rows = codes_rows_at(app, app.clock_now());
    y += search(app, ui, x, y, w, rows.is_empty());
    if rows.is_empty() {
        ui.text_as(x, y, BODY_M, Role::OnSurfaceVariant, none_line(app));
        y += ui.line_as(BODY_M) + faraday_draw::tokens::PAD;
    } else {
        for r in &rows {
            y += code_row(ui, x, y, w, r);
        }
    }
    y += 8.0;
    if app.vaults.form.is_some() {
        crate::vault::screen::codes_form(app, ui, x, y, w);
    } else {
        let scan = EN.codes_scan_a_setup;
        let add = EN.codes_add_an_entry;
        let sw = ui.measure_as(LABEL_L, scan) + 32.0;
        ui.button(
            x,
            y,
            Some(sw),
            BUTTON_H,
            scan,
            Style::Secondary,
            va(V::ScanEntry),
        );
        let aw = ui.measure_as(LABEL_L, add) + 32.0;
        ui.button(
            x + sw + 10.0,
            y,
            Some(aw),
            BUTTON_H,
            add,
            Style::Secondary,
            Action::CodesAdd,
        );
    }
    let _ = h;
}

/// [`draw`] on a small panel: one column, Now first, then a row per
/// entry with the account line dropped when there is no room, the bar
/// still under each row (§5.4). While typing goes to the search bar,
/// Material's search view: the bar at the top and the rows it finds.
fn compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let x = x0 + crate::compact::M;
    let w = cw - 2.0 * crate::compact::M;
    let mut y = 8.0 - app.list_offset;
    let view = app.vaults.focus == Some(Focus::Search(SearchIn::Codes));
    if !view {
        y += now_row(app, ui, x, y, w) + 8.0;
    }
    let rows = codes_rows_at(app, app.clock_now());
    y += search(app, ui, x, y, w, rows.is_empty());
    if rows.is_empty() {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, none_line(app));
        y += ui.line_as(BODY_S) + faraday_draw::tokens::PAD;
    } else {
        for r in &rows {
            y += code_row(ui, x, y, w, r);
        }
    }
    y += 8.0;
    // The search view holds the bar and the rows it finds alone.
    if app.vaults.form.is_some() && !view {
        y += crate::vault::screen::codes_form(app, ui, x, y, w) + 16.0;
    } else if !view {
        ui.button(
            x,
            y,
            None,
            BUTTON_H,
            EN.codes_scan_a_setup,
            Style::Secondary,
            va(V::ScanEntry),
        );
        y += 52.0;
        ui.button(
            x,
            y,
            None,
            BUTTON_H,
            EN.codes_add_an_entry,
            Style::Secondary,
            Action::CodesAdd,
        );
        y += 52.0;
    }
    crate::compact::screen::finish(app, ui, x0, cw, h, y + app.list_offset);
}
