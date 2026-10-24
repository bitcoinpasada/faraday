//! The sheets drawn over a screen.

use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{CoreUi, Glyph, Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Screen};
use faraday_draw::tokens as T;
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{
    STEP_INDENT, STEP_SIDE, button_rows, measure_wrap, qr_code, sheet_box, step_circle,
    wrap_buttons,
};

/// Making a seed from the Inbox a wallet: a passphrase or none, the fingerprint it gives,
/// the wallet here it opens if any, else the kind of single-key wallet.
/// The potential sheet on a small panel: the same fields, one under
/// another.
#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn potential_compact(
    app: &View,
    p: &crate::inbox::Potential,
    ui: &mut Ui,
    w: f32,
    h: f32,
    seed: Option<&crate::inbox::FoundSeed>,
    xpub: Option<&(String, bool)>,
    takes: bool,
    matched: Option<&str>,
) {
    let title = match (xpub, p.fingerprint) {
        (Some(_), Some(fp)) => fill(
            EN.potential_a_watch_only_wallet,
            &[&fp_text(fp).to_string()],
        ),
        (Some(_), None) => EN.potential_a_watch_only.to_string(),
        (None, Some(fp)) => fill(EN.potential_a_wallet, &[&fp_text(fp).to_string()]),
        _ => String::new(),
    };
    let kinds = |ui: &mut Ui, x: f32, y: f32, w: f32| -> f32 {
        let row: Vec<(String, Style, Action)> = crate::inbox::KINDS
            .iter()
            .enumerate()
            .map(|(k, kind)| {
                (
                    crate::family::kind_name(*kind).to_string(),
                    if *kind == p.kind {
                        Style::Primary
                    } else {
                        Style::Secondary
                    },
                    Action::PotentialKind(k as u8),
                )
            })
            .collect();
        button_rows(ui, x, y, w, &row) - 4.0
    };
    crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Keys, ACCENT, &title);
        if let Some(s) = seed {
            let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
            cy += ui.wrap(
                x,
                cy,
                iw,
                12.0,
                W::R,
                MUTED,
                &fill(EN.potential_sources, &[&from.join(" · ").to_string()]),
            ) + 10.0;
        }
        if takes {
            let label = match seed.and_then(|s| s.sources.first()) {
                Some(crate::inbox::SeedSource::Slip39(_)) => EN.potential_slip_39_passphrase,
                _ => EN.potential_bip_39_passphrase,
            };
            cy += ui.wrap(x, cy, iw, 13.0, W::S, TEXT, label) + 6.0;
            if app.pass_is_public(&p.passphrase) {
                super::parts::code_field(ui, x, cy, iw, T::TEXT_FIELD_SHEET, &p.passphrase);
                ui.hit(x, cy, iw, T::TEXT_FIELD_SHEET, Action::PotentialType);
            } else if app.may_load_keys() {
                let shown = if p.shown {
                    zeroize::Zeroizing::new(p.passphrase.to_string())
                } else {
                    zeroize::Zeroizing::new("•".repeat(p.passphrase.chars().count().min(48)))
                };
                ui.fill(x, cy, iw, 44.0, 8.0, BG);
                ui.stroke(x, cy, iw, 44.0, 8.0, if p.typing { ACCENT } else { BORDER });
                let shown = ui.fit_secret(14.0, W::M, &shown, iw - 60.0);
                let tw = ui.text_mid(x + 12.0, cy, 44.0, 14.0, W::M, TEXT, &shown);
                if p.typing
                    && !app.pass_chip_shown(
                        crate::demo::PassField::Potential,
                        p.passphrase.is_empty(),
                        p.typing,
                    )
                {
                    ui.caret(x + 13.0 + tw, cy + 13.0, 18.0);
                }
                ui.hit(x, cy, iw - 44.0, 44.0, Action::PotentialType);
                ui.icon(
                    x + iw - 44.0,
                    cy,
                    44.0,
                    if p.shown { Icon::EyeOff } else { Icon::Eye },
                    15.0,
                    if p.shown { ACCENT } else { MUTED },
                );
                ui.hit(x + iw - 44.0, cy, 44.0, 44.0, Action::PotentialShow);
                app.pass_chip_draw(
                    ui,
                    (x, cy, iw - T::TEXT_FIELD_SHEET, T::TEXT_FIELD_SHEET),
                    (p.passphrase.is_empty(), p.typing, false),
                    crate::demo::PassField::Potential,
                );
            } else {
                crate::vault::screen::stick_field(app.medium, ui, x, cy, iw);
            }
            cy += 56.0;
        }
        if let Some((key, fixed)) = xpub {
            let k = ui.fit(12.0, W::M, key, iw);
            ui.text(x, cy, 12.0, W::M, MUTED, &k);
            cy += 24.0;
            if *fixed {
                let line = fill(
                    EN.potential_as_its_path,
                    &[crate::family::kind_name(p.kind)],
                );
                cy += ui.wrap(x, cy, iw, 14.0, W::S, TEXT, &line) + 10.0;
            } else {
                cy += kinds(ui, x, cy, iw);
            }
        } else {
            if let Some(with) = p.with {
                ui.text(x, cy, 12.0, W::R, MUTED, EN.potential_fingerprint);
                ui.text(x + 90.0, cy, 13.0, W::M, TEXT, &fp_text(with));
                cy += 24.0;
            }
            match matched {
                Some(name) => {
                    let line = fill(EN.potential_key_of_wallet, &[name]);
                    cy += ui.wrap(x, cy, iw, 13.0, W::S, OK, &line) + 10.0;
                }
                None => {
                    ui.text(x, cy, 12.0, W::R, MUTED, EN.potential_single_key);
                    cy += 22.0;
                    cy += kinds(ui, x, cy, iw);
                }
            }
        }
        if let Some(e) = &p.error {
            cy += ui.wrap(x, cy, iw, 13.0, W::R, ERR, e) + 8.0;
        }
        let go = match matched {
            Some(name) if xpub.is_none() => fill(EN.potential_load, &[name]),
            _ => EN.potential_make_wallet.to_string(),
        };
        cy += crate::compact::buttons(
            ui,
            x,
            cy,
            iw,
            &[
                (EN.potential_cancel, Style::Secondary, Action::Cancel),
                (
                    &go,
                    Style::Primary.when(app.may_load_keys() || xpub.is_some()),
                    Action::PotentialMake,
                ),
            ],
        );
        cy - y
    });
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn potential_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(p) = app.inbox.potential() else {
        return;
    };
    let sw = 620.0;
    let seed = p.fingerprint.and_then(|fp| {
        app.inbox_found()
            .seeds
            .into_iter()
            .find(|s| s.fingerprint == fp)
    });
    let xpub = p.xpub.as_ref();
    let takes = xpub.is_none()
        && seed
            .as_ref()
            .is_none_or(|s| s.sources.first().is_none_or(|src| src.takes_passphrase()));
    let matched = app.potential_match();
    if ui.compact {
        potential_compact(
            app,
            p,
            ui,
            w,
            h,
            seed.as_ref(),
            xpub,
            takes,
            matched.as_deref(),
        );
        return;
    }
    let sh = if xpub.is_some() {
        370.0
    } else if takes {
        440.0
    } else {
        330.0
    };
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let ix = x + 32.0;
    let iw = sw - 64.0;
    let mut cy = y + 30.0;
    let title_text = match (xpub, p.fingerprint) {
        (Some(_), Some(fp)) => fill(
            EN.potential_a_watch_only_wallet,
            &[&fp_text(fp).to_string()],
        ),
        (Some(_), None) => EN.potential_a_watch_only.to_string(),
        (None, Some(fp)) => fill(EN.potential_a_wallet, &[&fp_text(fp).to_string()]),
        _ => String::new(),
    };
    ui.icon(ix, cy, 30.0, Icon::Keys, 18.0, ACCENT);
    let tt = ui.fit(20.0, W::S, &title_text, iw - 42.0);
    ui.text_mid(ix + 42.0, cy, 30.0, 20.0, W::S, TEXT, &tt);
    cy += 46.0;
    if let Some(s) = &seed {
        let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
        let line = ui.fit(
            13.0,
            W::R,
            &fill(EN.potential_sources, &[&from.join(" · ").to_string()]),
            iw,
        );
        ui.text(ix, cy, 13.0, W::R, MUTED, &line);
        cy += 30.0;
    }
    if takes {
        let label = match seed.as_ref().and_then(|s| s.sources.first()) {
            Some(crate::inbox::SeedSource::Slip39(_)) => EN.potential_slip_39_passphrase,
            _ => EN.potential_bip_39_passphrase,
        };
        ui.text(ix, cy, 13.0, W::S, TEXT, label);
        cy += 24.0;
        if app.pass_is_public(&p.passphrase) {
            super::parts::code_field(ui, ix, cy, iw, T::TEXT_FIELD, &p.passphrase);
            ui.hit(ix, cy, iw, T::TEXT_FIELD, Action::PotentialType);
        } else if app.may_load_keys() {
            let shown = if p.shown {
                zeroize::Zeroizing::new(p.passphrase.to_string())
            } else {
                zeroize::Zeroizing::new("•".repeat(p.passphrase.chars().count().min(48)))
            };
            ui.fill(ix, cy, iw, 40.0, 8.0, BG);
            ui.stroke(
                ix,
                cy,
                iw,
                40.0,
                8.0,
                if p.typing { ACCENT } else { BORDER },
            );
            let shown = ui.fit_secret(14.0, W::M, &shown, iw - 60.0);
            let tw = ui.text_mid(ix + 12.0, cy, 40.0, 14.0, W::M, TEXT, &shown);
            if p.typing
                && !app.pass_chip_shown(
                    crate::demo::PassField::Potential,
                    p.passphrase.is_empty(),
                    p.typing,
                )
            {
                ui.caret(ix + 13.0 + tw, cy + 11.0, 18.0);
            }
            ui.hit(ix, cy, iw - 44.0, 40.0, Action::PotentialType);
            ui.icon(
                ix + iw - 40.0,
                cy,
                40.0,
                if p.shown { Icon::EyeOff } else { Icon::Eye },
                15.0,
                if p.shown { ACCENT } else { MUTED },
            );
            ui.hit(ix + iw - 40.0, cy, 40.0, 40.0, Action::PotentialShow);
            app.pass_chip_draw(
                ui,
                (ix, cy, iw - T::TEXT_FIELD_SHEET, T::TEXT_FIELD),
                (p.passphrase.is_empty(), p.typing, false),
                crate::demo::PassField::Potential,
            );
        } else {
            crate::vault::screen::stick_field(app.medium, ui, ix, cy, iw);
        }
        cy += 52.0;
    }
    if let Some((key, fixed)) = xpub {
        let k = ui.fit(12.0, W::M, key, iw);
        ui.text(ix, cy, 12.0, W::M, MUTED, &k);
        cy += 30.0;
        ui.text_mid(ix, cy, 30.0, 13.0, W::R, MUTED, EN.potential_single_key);
        cy += 32.0;
        if *fixed {
            ui.text_mid(
                ix,
                cy,
                34.0,
                14.0,
                W::S,
                TEXT,
                &fill(
                    EN.potential_as_its_path,
                    &[crate::family::kind_name(p.kind)],
                ),
            );
        } else {
            let mut bx = ix;
            for (k, kind) in crate::inbox::KINDS.iter().enumerate() {
                let style = if *kind == p.kind {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                bx += ui.button(
                    bx,
                    cy,
                    None,
                    34.0,
                    crate::family::kind_name(*kind),
                    style,
                    Action::PotentialKind(k as u8),
                ) + 8.0;
            }
        }
        cy += 46.0;
    } else {
        if let Some(with) = p.with {
            ui.text_mid(ix, cy, 24.0, 13.0, W::R, MUTED, EN.potential_fingerprint);
            ui.text_mid(ix + 110.0, cy, 24.0, 14.0, W::M, TEXT, &fp_text(with));
            cy += 30.0;
        }
        match &matched {
            Some(name) => {
                ui.text_mid(
                    ix,
                    cy,
                    30.0,
                    13.0,
                    W::S,
                    OK,
                    &fill(EN.potential_key_of_wallet, &[&name.to_string()]),
                );
                cy += 40.0;
            }
            None => {
                ui.text_mid(ix, cy, 30.0, 13.0, W::R, MUTED, EN.potential_single_key);
                cy += 32.0;
                let mut bx = ix;
                for (k, kind) in crate::inbox::KINDS.iter().enumerate() {
                    let style = if *kind == p.kind {
                        Style::Primary
                    } else {
                        Style::Secondary
                    };
                    bx += ui.button(
                        bx,
                        cy,
                        None,
                        34.0,
                        crate::family::kind_name(*kind),
                        style,
                        Action::PotentialKind(k as u8),
                    ) + 8.0;
                }
                cy += 46.0;
            }
        }
    }
    if let Some(e) = &p.error {
        let e = ui.fit(13.0, W::R, e, iw);
        ui.text(ix, cy, 13.0, W::R, ERR, &e);
    }
    let by = y + sh - 32.0 - 46.0;
    let bw = (iw - 12.0) / 2.0;
    ui.button(
        ix,
        by,
        Some(bw),
        46.0,
        EN.potential_cancel,
        Style::Secondary,
        Action::Cancel,
    );
    let go = match &matched {
        Some(name) if xpub.is_none() => fill(EN.potential_load, &[&name.to_string()]),
        _ => EN.potential_make_wallet.to_string(),
    };
    ui.button(
        ix + bw + 12.0,
        by,
        Some(bw),
        46.0,
        &go,
        Style::Primary.when(app.may_load_keys() || xpub.is_some()),
        Action::PotentialMake,
    );
}

/// `button_rows`, except a label too wide for the row even alone is
/// never cut (`docs/DESIGN.md` "Menu row"): it takes the row whole and
/// wraps onto further lines, its chip growing to hold them. Used where
/// `button_rows`'s "a long label is cut" must not hold — the Learn
/// sheet's tabs, whose first row names a label the spec gives in full
/// (`docs/history/SIMPLIFY.md` §6.2).
pub(crate) fn learn_tabs(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    items: &[(String, Style, Action)],
) -> f32 {
    let (mut bx, mut by) = (x, y);
    for (label, style, action) in items {
        let natural = ui.measure(13.0, W::S, label) + 32.0;
        if natural <= w {
            if bx > x && bx + natural > x + w {
                bx = x;
                by += 40.0;
            }
            bx += ui.button(bx, by, Some(natural), 32.0, label, *style, *action) + 6.0;
        } else {
            if bx > x {
                bx = x;
                by += 40.0;
            }
            let lines = ui.wrap_lines(13.0, W::S, label, w - 24.0).max(1);
            let line_h = ui.line(13.0, W::S);
            let bh = 32.0 + (lines - 1) as f32 * line_h;
            // An empty-labelled button draws this style's background,
            // border and hit region exactly as any other; the label is
            // then wrapped over it, since `Ui::button` itself only ever
            // draws one line.
            ui.button(bx, by, Some(w), bh, "", *style, *action);
            let fg = style.label();
            let top = by + (bh - lines as f32 * line_h) / 2.0;
            ui.wrap(bx + 12.0, top, w - 24.0, 13.0, W::S, fg, label);
            by += bh + 6.0;
            bx = x;
        }
    }
    by - y + 44.0
}

/// The device check's buttons: Check again, then Continue anyway (asked
/// once), or Power off on a virtual machine, which has no way on.
fn devices_buttons(app: &View) -> Vec<(&'static str, Style, Action)> {
    let s = &crate::strings::EN;
    let d = &app.devices;
    // Reminders alone: go on from them, or not.
    if d.held.is_empty() {
        return vec![
            (s.devices_cancel, Style::Secondary, Action::Cancel),
            (
                s.devices_understood,
                Style::Primary,
                Action::DevicesUnderstood,
            ),
        ];
    }
    let mut b = vec![(
        s.devices_check_again,
        Style::Secondary,
        Action::DevicesCheck,
    )];
    if d.refuses() {
        b.push((s.devices_power_off, Style::Primary, Action::PowerAsk));
    } else if d.asking {
        b.push((
            s.devices_continue_yes,
            Style::Primary,
            Action::DevicesContinue,
        ));
    } else {
        b.push((
            s.devices_continue,
            Style::Secondary,
            Action::DevicesContinue,
        ));
    }
    b
}

/// The tone a reminder is drawn in: the Bolt's line says its typing is
/// encrypted, which is no warning.
fn reminder_tone(line: &str) -> crate::ui::Paint {
    if line == crate::strings::EN.devices_bolt_notice {
        MUTED
    } else {
        WARN
    }
}

/// What is attached that can capture the screen or the keys
/// (`docs/STARTER.md` §3.1a): a line for each, and what holds them;
/// before a secret, the reminders due under them (F24).
pub(super) fn devices_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let s = &crate::strings::EN;
    let d = &app.devices;
    let mut lines: Vec<(String, crate::ui::Paint)> =
        d.held.iter().map(|c| (c.line(), TEXT)).collect();
    lines.extend(
        app.device_reminders()
            .into_iter()
            .map(|r| (r.to_string(), reminder_tone(r))),
    );
    let title = if d.held.is_empty() {
        s.devices_before
    } else {
        s.devices_title
    };
    let foot = if d.refuses() {
        Some((s.devices_real_computer, ERR))
    } else if d.asking {
        Some((s.devices_continue_ask, WARN))
    } else {
        None
    };
    let buttons = devices_buttons(app);
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Warning, WARN, title);
            for (l, tone) in &lines {
                cy += ui.wrap(x, cy, iw, 14.0, W::R, *tone, l) + 8.0;
            }
            if let Some((f, tone)) = foot {
                cy += 4.0;
                cy += ui.wrap(x, cy, iw, 14.0, W::S, tone, f) + 8.0;
            }
            cy += 8.0;
            cy += crate::compact::buttons(ui, x, cy, iw, &buttons);
            cy - y
        });
        return;
    }
    let sw = 560.0f32.min(w - 48.0);
    let iw = sw - 64.0;
    let line_h = ui.line(14.0, W::R);
    let mut body = 0.0;
    for (l, _) in &lines {
        body += ui.wrap_lines(14.0, W::R, l, iw) as f32 * line_h + 10.0;
    }
    if let Some((f, _)) = foot {
        body += ui.wrap_lines(14.0, W::S, f, iw) as f32 * line_h + 14.0;
    }
    let sh = 28.0 + 30.0 + 26.0 + body + 22.0 + 46.0 + 32.0;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let ix = x + 32.0;
    ui.icon(ix, y + 28.0, 30.0, Icon::Warning, 17.0, WARN);
    ui.text_mid(ix + 42.0, y + 28.0, 30.0, 20.0, W::S, TEXT, title);
    let mut cy = y + 28.0 + 30.0 + 26.0;
    for (l, tone) in &lines {
        cy += ui.wrap(ix, cy, iw, 14.0, W::R, *tone, l) + 10.0;
    }
    if let Some((f, tone)) = foot {
        cy += 4.0;
        ui.wrap(ix, cy, iw, 14.0, W::S, tone, f);
    }
    let by = y + sh - 32.0 - 46.0;
    let n = buttons.len() as f32;
    let each = (iw - 12.0 * (n - 1.0)) / n;
    for (i, (label, style, action)) in buttons.into_iter().enumerate() {
        ui.button(
            ix + i as f32 * (each + 12.0),
            by,
            Some(each),
            46.0,
            label,
            style,
            action,
        );
    }
}

/// Something that loads a key or turns the camera on was pressed with a
/// stick attached: what it was, and the stick. It closes, and what was pressed carries on, when
/// the stick is pulled.
pub(super) fn pull_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let what = app
        .visit
        .pull()
        .and_then(|a| app.pull_what(a))
        .unwrap_or(EN.pull_default_reason);
    let title = fill(EN.pull_medium_to_reason, &[app.medium.noun(), what]);
    let line = match app.stick.len() {
        0 | 1 => fill(
            EN.pull_medium_attached,
            &[(app.stick.first().map_or("", |s| s.label.as_str()))],
        ),
        k => fill(EN.pull_attached, &[&k.to_string(), app.medium.nouns()]),
    };
    // A secret follows the pull: the reminders due go with it (F24).
    let reminders = app.devices.reminders_due();
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, app.medium.icon(), WARN, &title);
            cy += ui.wrap(x, cy, iw, 13.0, W::R, MUTED, &line) + 16.0;
            for r in &reminders {
                cy += ui.wrap(x, cy, iw, 14.0, W::R, reminder_tone(r), r) + 8.0;
            }
            if !reminders.is_empty() {
                cy += 8.0;
            }
            cy += crate::compact::buttons(
                ui,
                x,
                cy,
                iw,
                &[(EN.pull_cancel, Style::Secondary, Action::Cancel)],
            );
            cy - y
        });
        return;
    }
    let iw = 480.0 - 64.0;
    let line_h = ui.line(14.0, W::R);
    let extra: f32 = reminders
        .iter()
        .map(|r| ui.wrap_lines(14.0, W::R, r, iw) as f32 * line_h + 10.0)
        .sum();
    let sh = 196.0
        + if reminders.is_empty() {
            0.0
        } else {
            extra + 8.0
        };
    let (x, y) = sheet_box(ui, w, h, 480.0, sh);
    let ix = x + 32.0;
    ui.icon(ix, y + 30.0, 30.0, app.medium.icon(), 18.0, WARN);
    let title = ui.fit(20.0, W::S, &title, iw - 42.0);
    ui.text_mid(ix + 42.0, y + 30.0, 30.0, 20.0, W::S, TEXT, &title);
    let line = ui.fit(13.0, W::R, &line, iw);
    ui.text(ix, y + 78.0, 13.0, W::R, MUTED, &line);
    let mut cy = y + 78.0 + 34.0;
    for r in &reminders {
        cy += ui.wrap(ix, cy, iw, 14.0, W::R, reminder_tone(r), r) + 10.0;
    }
    ui.button(
        ix,
        y + sh - 32.0 - 46.0,
        Some(iw),
        46.0,
        EN.pull_cancel,
        Style::Secondary,
        Action::Cancel,
    );
}

/// The lock, write-out and power-off sheets' sections (§4.5): Kept,
/// one line per vault with its currency and one for the files For the
/// stick; then Wiped from memory, each seed and wallet with where else it
/// is, "nowhere else" alone in the danger tone, and with `save` a Save
/// beside each one in no open vault; then Then.
fn kept_sections(app: &View, power_off: bool, save: bool, then: String) -> Vec<Section> {
    let kw = app.kept_wiped(power_off);
    let then = if app.mode_changing() {
        EN.lock_mode_keeps_nothing.to_string()
    } else {
        then
    };
    vec![
        (
            EN.kept_sections_kept,
            kw.kept.into_iter().map(|l| (l, TEXT, None)).collect(),
        ),
        (
            EN.kept_sections_wiped,
            kw.wiped
                .into_iter()
                .map(|(l, nowhere, s)| (l, if nowhere { ERR } else { TEXT }, s.filter(|_| save)))
                .collect(),
        ),
        (EN.kept_sections_then, vec![(then, TEXT, None)]),
    ]
}

/// A kept sheet's section: its name and its lines, each with its tone
/// and a button beside it.
type Section = (
    &'static str,
    Vec<(String, crate::ui::Paint, Option<Action>)>,
);

/// A kept sheet's line, its tone, a labelled row, and a code to show
/// with its text.
type SheetLine<'a> = (
    &'a str,
    crate::ui::Paint,
    Option<(&'a str, &'a str)>,
    Option<(&'a str, &'a faraday_codec::qr::QrMatrix)>,
);

/// A sheet that says what a lock or a power-off does: its title, a line,
/// the sections, each line wrapped to the sheet, and its buttons.
fn kept_sheet(
    app: &View,
    ui: &mut Ui,
    (w, h): (f32, f32),
    head: (Glyph, crate::ui::Paint, &str),
    (line, line_tone, row, code): SheetLine<'_>,
    sections: &[Section],
    actions: &[(&str, Style, Action)],
) {
    let (place, margin, pad) = if ui.compact {
        ((8.0, w - 16.0), 8.0, 16.0)
    } else {
        let sw = (w - 48.0).min(600.0);
        (((w - sw) / 2.0, sw), 40.0, 32.0)
    };
    crate::compact::scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        if ui.compact {
            cy += crate::compact::sheet_head(ui, x, cy, iw, head.0, head.1, head.2);
        } else {
            ui.icon(x, cy, 30.0, head.0, 18.0, head.1);
            let t = ui.fit(20.0, W::S, head.2, iw - 42.0);
            ui.text_mid(x + 42.0, cy, 30.0, 20.0, W::S, TEXT, &t);
            cy += 46.0;
        }
        if !line.is_empty() {
            let size = if line_tone == WARN { 14.0 } else { 13.0 };
            cy += ui.wrap(x, cy, iw, size, W::R, line_tone, line) + 12.0;
        }
        if let Some((label, value)) = row {
            cy += crate::compact::kv(ui, x, cy, iw, label, value, TEXT);
        }
        if let Some((code, m)) = code {
            // The same size the Starter's restart sheet draws its code.
            let side = if ui.compact { 104.0f32 } else { 160.0 }.min(iw);
            qr_code(ui, m, x, cy, side);
            cy += side + 10.0;
            cy += ui.wrap(x, cy, iw, 12.0, W::M, MUTED, code) + 12.0;
        }
        for (name, lines) in sections {
            if lines.is_empty() {
                continue;
            }
            ui.rule(x, cy, iw, BORDER);
            cy += 12.0;
            ui.text(x, cy, 12.0, W::R, MUTED, name);
            cy += 22.0;
            for (text, tone, act) in lines {
                let Some(act) = *act else {
                    cy += ui.wrap(x, cy, iw, 14.0, W::R, *tone, text) + 8.0;
                    continue;
                };
                // The line beside its button, at the right.
                let bw = ui.measure(T::LABEL, W::S, EN.vault_save) + 2.0 * T::PAD;
                let inset = (T::CHIP_LINE - T::LABEL) / 2.0;
                let tw = iw - bw - T::GAP;
                let th = ui.wrap(x, cy + inset, tw, T::LABEL, W::R, *tone, text);
                ui.button(
                    x + iw - bw,
                    cy,
                    Some(bw),
                    T::CHIP_LINE,
                    EN.vault_save,
                    Style::Secondary,
                    act,
                );
                cy += (th + inset).max(T::CHIP_LINE) + T::GAP;
            }
            cy += 4.0;
        }
        cy += 8.0;
        // A hold (decision 5) fills while it is held; the rest are
        // pressed, at the sheet's foot.
        let mut hx = x;
        for &(label, _, action) in actions.iter().filter(|a| crate::vault::is_hold(a.2)) {
            let room = x + iw - hx;
            hx += crate::vault::screen::hold_button_in(app, ui, hx, cy, room, label, action, ERR)
                + T::GAP;
        }
        if hx > x {
            cy += T::CHIP_LINE + T::GAP + T::GAP_SMALL;
        }
        let pressed: Vec<(&str, Style, Action)> = actions
            .iter()
            .copied()
            .filter(|a| !crate::vault::is_hold(a.2))
            .collect();
        cy += crate::compact::buttons(ui, x, cy, iw, &pressed);
        cy - y
    });
}

pub(super) fn lock_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let label = app
        .stick
        .first()
        .map(|s| s.label.clone())
        .unwrap_or_default();
    // Asked for by Upgrade rather than by a stick arriving.
    let upgrade = app.upgrade_after_lock();
    let then = if upgrade {
        EN.lock_fresh_start_upgrade.to_string()
    } else {
        fill(EN.lock_a_fresh_start, &[app.medium.noun()])
    };
    let (attached, use_it, line) = if upgrade {
        (
            app.medium.upgrade().to_string(),
            EN.lock_lock_and_upgrade.to_string(),
            EN.lock_the_upgrade_runs.to_string(),
        )
    } else {
        (
            fill(EN.lock_is_attached, &[app.medium.a_cap()]),
            fill(EN.lock_lock_and_use, &[app.medium.noun()]),
            fill(EN.lock_nothing_has_been, &[&label.to_string()]),
        )
    };
    kept_sheet(
        app,
        ui,
        (w, h),
        (app.medium.icon(), WARN, &attached),
        (&line, MUTED, None, None),
        &kept_sections(app, false, false, then),
        &[
            (EN.lock_not_now, Style::Secondary, Action::NotNow),
            (&use_it, Style::Primary, Action::Lock),
        ],
    );
}

/// Lock, asked for while a vault is open with something not in it, a
/// seal waiting or Make a new wallet under way: what is kept, what is
/// wiped from memory, each key or wallet in no vault with its Save, and
/// then (decision 28). With no randomness to seal with, the line says so
/// and the lock is one without sealing.
pub(super) fn lock_ask_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let (head, line, lock) = if app.seal_waits() {
        (
            (Icon::Error.into(), ERR, EN.vault_lock),
            (EN.seed_missing, ERR),
            (EN.vault_lock_without_sealing, Action::LockUnsealed),
        )
    } else {
        (
            (Icon::Lock.into(), ACCENT, EN.vault_lock),
            ("", MUTED),
            (EN.vault_lock, Action::Lock),
        )
    };
    kept_sheet(
        app,
        ui,
        (w, h),
        head,
        (line.0, line.1, None, None),
        &kept_sections(app, false, true, EN.lock_ask_then.to_string()),
        &[
            (EN.vault_back_to_the_vault, Style::Secondary, Action::Cancel),
            (lock.0, Style::Primary, lock.1),
        ],
    );
}

/// Writing what is For the stick from a session that held a secret: the
/// lock comes first, and this says what it keeps and wipes.
pub(super) fn write_out_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let then = if app.stick.is_empty() {
        fill(EN.write_out_lock_plug, &[app.medium.a()])
    } else {
        EN.write_out_lock_here.to_string()
    };
    let title = fill(EN.write_out_target, &[app.medium.a()]);
    let mut sections = kept_sections(app, false, false, then);
    let drops = app.lock_drops();
    if !drops.is_empty() {
        // Between Wiped from memory and Then.
        let at = sections.len() - 1;
        sections.insert(
            at,
            (
                EN.write_out_drops,
                drops.into_iter().map(|n| (n, TEXT, None)).collect(),
            ),
        );
    }
    kept_sheet(
        app,
        ui,
        (w, h),
        (app.medium.icon(), ACCENT, &title),
        ("", MUTED, None, None),
        &sections,
        &[
            (EN.write_out_not_now, Style::Secondary, Action::Cancel),
            (EN.write_out_lock, Style::Primary, Action::Lock),
        ],
    );
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn scan_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(sc) = app.scan() else { return };
    let pad = if ui.compact { 16.0 } else { 32.0 };
    let side_w = if ui.compact {
        w - 16.0 - 2.0 * pad
    } else {
        (w - 200.0).clamp(320.0, 640.0)
    };
    let side_h = side_w * 0.75;
    let sw = side_w + 2.0 * pad;
    let rows = if ui.compact && app.cameras().len() > 1 {
        app.cameras().len() as f32 * 40.0
    } else {
        0.0
    };
    // A wallet's key scanner: a line per key it takes, and the way to
    // type the words instead (`docs/history/SCAN-TO-SIGN.md` §2.1).
    let for_wallet = match sc.purpose {
        crate::ScanPurpose::WalletKey(wi) => app.session.wallets.get(wi).map(|wl| (wi, wl)),
        _ => None,
    };
    let keys = for_wallet.map_or_else(Vec::new, |(wi, _)| app.wallet_key_lines(wi));
    // The Spend tab's intake: the hint its next question gives, and Type
    // the words instead while keys may load (`docs/history/SCAN-TO-SIGN.md`
    // §4.2).
    let intake = sc.purpose == crate::ScanPurpose::Intake;
    let hint = if intake { app.family_hint() } else { None };
    let hint_h = hint.map_or(0.0, |t| measure_wrap(ui, side_w, 12.0, W::R, t) + 8.0);
    // A scanner that loads a seed can be given one by Type or Paste.
    let seed_scan = app.seed_line
        && matches!(
            sc.purpose,
            crate::ScanPurpose::Seed
                | crate::ScanPurpose::KeyPart
                | crate::ScanPurpose::WalletKey(_)
                | crate::ScanPurpose::Intake
        );
    let never_h = if seed_scan {
        measure_wrap(ui, side_w, 13.0, W::R, EN.never_real_seed) + 8.0
    } else {
        0.0
    };
    let keys_h =
        keys.len() as f32 * 20.0 + if keys.is_empty() { 0.0 } else { 6.0 } + hint_h + never_h;
    let typing = for_wallet.is_some() || (intake && app.may_load_keys());
    let type_row = if typing && ui.compact { 52.0 } else { 0.0 };
    // The Sample tile, in Demo (`docs/MODES.md` §6): after Paste and Type.
    let sample = app.scan_sample_offered();
    let sample_row = if sample && ui.compact {
        faraday_draw::tokens::SCAN_TILE_ROW
    } else {
        0.0
    };
    let extra = rows + keys_h + type_row + sample_row;
    // The title, then the picture: the state line is inside it.
    let top = 56.0;
    let side_h = side_h.min(h - 24.0 - top - 88.0 - extra).max(96.0);
    let sh = side_h + top + 88.0 + extra;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let x = x + pad - 32.0;
    let sw = sw - 2.0 * (pad - 32.0);
    let title = match (sc.purpose, for_wallet) {
        (_, Some((_, wl))) => fill(EN.scan_key_for_wallet, &[&wl.name.to_string()]),
        (crate::ScanPurpose::Intake, _) => EN.scan_scan_what_you.to_string(),
        (crate::ScanPurpose::Seed, _) => EN.scan_scan_a_seedqr.to_string(),
        (crate::ScanPurpose::CheckCopy(_) | crate::ScanPurpose::CheckCodex32, _) => {
            EN.scan_scan_your_copy.to_string()
        }
        (crate::ScanPurpose::Transfer, _) => EN.scan_receive_into_downloads.to_string(),
        _ => EN.scan_scan_a_qr_code.to_string(),
    };
    let title = ui.fit(18.0, W::S, &title, sw - 64.0);
    ui.text(x + 32.0, y + 26.0, 18.0, W::S, TEXT, &title);
    for (k, key) in keys.iter().enumerate() {
        let key = ui.fit(13.0, W::M, key, sw - 64.0);
        ui.text(x + 32.0, y + top + k as f32 * 20.0, 13.0, W::M, TEXT, &key);
    }
    if let Some(t) = hint {
        let ty = y + top + keys.len() as f32 * 20.0;
        ui.wrap(x + 32.0, ty, side_w, 12.0, W::R, TEXT, t);
    }
    if seed_scan {
        let ty = y + top + keys.len() as f32 * 20.0 + hint_h;
        ui.wrap(x + 32.0, ty, side_w, 13.0, W::R, WARN, EN.never_real_seed);
    }
    let y = y + keys_h;
    let (px, py) = (x + 32.0, y + top);
    let rect = ui.rect(px, py, side_w, side_h);
    ui.fill(px, py, side_w, side_h, 8.0, BG);
    // A QR found and read, still in view: the brackets turn green.
    let mut read_in_view = false;
    if let Some((fw, fh, luma)) = sc.frame.as_ref() {
        ui.c.luma(rect, *fw, *fh, luma, sc.chroma.as_deref());
        // The code the scanner last found, outlined where it is: green
        // once read, amber while it is found and not read.
        // Gone ten preview frames after the scanner stops seeing it.
        if let Some((seen, at)) = sc.seen
            && sc.frames.wrapping_sub(at) <= crate::SEEN_FRAMES
            && seen.width > 0
            && seen.height > 0
        {
            read_in_view = seen.read;
            // The preview fills its box and crops the longer side, as
            // `Canvas::luma` draws it: the same scale and crop here.
            let (fw, fh) = (f32::from(*fw), f32::from(*fh));
            let scale = (rect.w as f32 / fw).max(rect.h as f32 / fh);
            let left = (fw - rect.w as f32 / scale) / 2.0;
            let top = (fh - rect.h as f32 / scale) / 2.0;
            let map = |(cx, cy): (u16, u16)| {
                let px = f32::from(cx) * fw / f32::from(seen.width);
                let py = f32::from(cy) * fh / f32::from(seen.height);
                (
                    rect.x as f32 + (px - left) * scale,
                    rect.y as f32 + (py - top) * scale,
                )
            };
            let color = ui.col(if seen.read { OK } else { WARN });
            let lw = (3.0 * ui.f).max(2.0);
            for i in 0..4 {
                let (a, b) = (map(seen.corners[i]), map(seen.corners[(i + 1) % 4]));
                ui.c.line(a.0, a.1, b.0, b.1, lw, color);
            }
        }
    }
    viewfinder_over(ui, (px, py, side_w, side_h), sc, read_in_view);
    // Paste, beside Type and Cancel, where the scanner takes anything:
    // dimmed where the shell has no clipboard. A seed scanner has it only
    // where the shell has one (F75, `docs/QR.md` §8 item 5).
    let paste = !ui.compact && app.scan_pastes();
    let paste_w = if paste {
        ui.measure(14.0, W::S, EN.scan_paste) + 32.0 + 12.0
    } else {
        0.0
    };
    // Which camera, when there is more than one.
    if app.cameras().len() > 1 && ui.compact {
        let chosen = app.camera();
        let row: Vec<(String, Style, Action)> = app
            .cameras()
            .iter()
            .enumerate()
            .map(|(i, (id, name))| {
                let on = chosen.map_or(i == 0, |c| c == id);
                (
                    ui.fit(12.0, W::S, name, sw - 96.0),
                    Style::chosen(on),
                    Action::ScanCamera(i as u8),
                )
            })
            .collect();
        button_rows(ui, x + 32.0, y + top + side_h + 12.0, sw - 64.0, &row);
    } else if app.cameras().len() > 1 {
        let chosen = app.camera();
        let row: Vec<(String, Style, Action)> = app
            .cameras()
            .iter()
            .enumerate()
            .map(|(i, (id, name))| {
                let on = chosen.map_or(i == 0, |c| c == id);
                (
                    ui.fit(12.0, W::S, name, 180.0),
                    Style::chosen(on),
                    Action::ScanCamera(i as u8),
                )
            })
            .collect();
        let typing = if typing { 220.0 } else { 0.0 };
        let sampling = if sample {
            crate::samples::put::scan_tile_span(ui, app.scan_sample_label())
        } else {
            0.0
        };
        button_rows(
            ui,
            x + 32.0,
            y - keys_h + sh - 32.0 - 44.0 + 6.0,
            sw - 64.0 - 140.0 - typing - sampling - paste_w,
            &row,
        );
    }
    // The foot, from the sheet's own top.
    let foot = y - keys_h + sh - 32.0 - 44.0;
    let type_instead = match for_wallet {
        Some((wi, _)) => Some(Action::EntryWallet(wi)),
        None if typing => Some(Action::Family(crate::family::FamilyAction::TypeWords)),
        None => None,
    };
    let mut left = x + sw - 32.0 - 120.0 - 12.0;
    if sample {
        left = crate::samples::put::scan_tile(
            ui,
            (x, sw, foot),
            (type_row, left),
            app.scan_sample_label(),
        );
    }
    if let Some(action) = type_instead {
        let label = EN.scan_type_the_words;
        if ui.compact {
            ui.button(
                x + 32.0,
                foot - 52.0,
                Some(sw - 64.0),
                44.0,
                label,
                Style::Secondary,
                action,
            );
        } else {
            let bw = ui.measure(14.0, W::S, label) + 32.0;
            left -= bw;
            ui.button(left, foot, Some(bw), 44.0, label, Style::Secondary, action);
            left -= 12.0;
        }
    }
    if paste {
        let bw = paste_w - 12.0;
        ui.button(
            left - bw,
            foot,
            Some(bw),
            44.0,
            EN.scan_paste,
            if app.clipboard {
                Style::Secondary
            } else {
                Style::Disabled
            },
            Action::ScanPaste,
        );
    }
    ui.button(
        x + sw - 32.0 - 120.0,
        foot,
        Some(120.0),
        44.0,
        if intake { EN.scan_done } else { EN.scan_cancel },
        if intake {
            Style::Primary
        } else {
            Style::Secondary
        },
        Action::Cancel,
    );
}

/// What is drawn over the camera's picture (`docs/QR.md` §8 item 4):
/// the corner brackets, white while looking and green while a QR that
/// read is in view, and at the bottom, on a translucent band, the state
/// line and the bar of parts read.
fn viewfinder_over(
    ui: &mut Ui,
    (x, y, w, h): (f32, f32, f32, f32),
    sc: &crate::ScanState,
    read: bool,
) {
    use faraday_draw::tokens;
    let tone = if read { VIEWFINDER_READ } else { VIEWFINDER };
    let arm = w.min(h) * tokens::VIEWFINDER_BRACKET_SHARE;
    let (st, inset) = (
        tokens::VIEWFINDER_BRACKET_STROKE,
        tokens::VIEWFINDER_BRACKET_INSET,
    );
    let (l, t) = (x + inset, y + inset);
    let (r, b) = (x + w - inset, y + h - inset);
    for (cx, right) in [(l, false), (r, true)] {
        for (cy, bottom) in [(t, false), (b, true)] {
            let ax = if right { cx - arm } else { cx };
            let ay = if bottom { cy - st } else { cy };
            ui.fill(ax, ay, arm, st, 0.0, tone);
            let bx = if right { cx - st } else { cx };
            let by = if bottom { cy - arm } else { cy };
            ui.fill(bx, by, st, arm, 0.0, tone);
        }
    }
    let size = tokens::VIEWFINDER_STATE_SIZE;
    let pad = tokens::VIEWFINDER_BAND_PAD;
    let line_h = ui.line(size, W::R);
    let parts = sc.parts_read();
    let bar = parts.map_or(0.0, |_| pad + tokens::PROGRESS_H);
    let band_h = pad + line_h + bar + pad;
    let band_y = y + h - band_h;
    ui.fill(
        x,
        band_y,
        w,
        band_h,
        0.0,
        VIEWFINDER_BAND.with_alpha(tokens::VIEWFINDER_BAND_ALPHA),
    );
    let line = ui.fit(size, W::R, sc.state_line(), w - 2.0 * pad);
    let lw = ui.measure(size, W::R, &line);
    ui.text(
        x + (w - lw) / 2.0,
        band_y + pad,
        size,
        W::R,
        VIEWFINDER,
        &line,
    );
    if let Some(share) = parts {
        let (bx, by, bw) = (x + pad, band_y + pad + line_h + pad, w - 2.0 * pad);
        ui.progress((bx, by, bw), Some(share), None);
    }
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn qr_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(q) = app.qr() else { return };
    if ui.compact {
        qr_sheet_compact(app, q, ui, w, h);
        return;
    }
    // A descriptor's whole text goes under the code, which gives up the
    // height it takes.
    let mut side = (h - 272.0).clamp(240.0, 560.0);
    let mut whole_h = 0.0;
    if let Some(t) = &q.whole {
        for _ in 0..2 {
            let tw = (side + 64.0).max(420.0) - 64.0;
            whole_h = measure_wrap(ui, tw, 11.0, W::M, t) + 10.0;
            side = (h - 272.0 - whole_h).clamp(240.0, 560.0);
        }
        whole_h = measure_wrap(ui, (side + 64.0).max(420.0) - 64.0, 11.0, W::M, t) + 10.0;
    }
    let sw = (side + 64.0).max(420.0);
    let sh = side + 242.0 + whole_h;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let t = ui.fit(18.0, W::S, &q.title, sw - 64.0);
    ui.text(x + 32.0, y + 26.0, 18.0, W::S, TEXT, &t);
    let sub = if q.frames.len() > 1 {
        fill(
            EN.qr_frame_count,
            &[
                &q.subtitle.to_string(),
                &(q.frame + 1).to_string(),
                &q.frames.len().to_string(),
            ],
        )
    } else {
        q.subtitle.clone()
    };
    let sub = ui.fit(12.0, W::R, &sub, sw - 64.0);
    ui.text(x + 32.0, y + 50.0, 12.0, W::R, MUTED, &sub);
    // Whether scanning it is safe, said beside the code itself.
    let (tag, tone) = if q.secret {
        (EN.qr_secret_whoever_scans, ERR)
    } else {
        (EN.qr_public_safe_to_scan, OK)
    };
    ui.text(x + 32.0, y + 66.0, 12.0, W::S, tone, tag);
    if let Some(m) = q.frames.get(q.frame) {
        qr_code(ui, m, x + (sw - side) / 2.0, y + 86.0, side);
    }
    if let Some(t) = &q.whole {
        ui.wrap(
            x + 32.0,
            y + 86.0 + side + 10.0,
            sw - 64.0,
            11.0,
            W::M,
            TEXT,
            t,
        );
    }
    // How it is written: the format, and for an animated code its speed
    // and how much each frame holds.
    let row = y + sh - 32.0 - 44.0 - 52.0;
    let mut bx = x + 32.0;
    // Numbered parts carry text only: a descriptor or a key.
    let text = matches!(
        q.source,
        crate::QrSource::Text(_) | crate::QrSource::Key(..)
    );
    // Each a segmented button (Material's), the chosen one ticked.
    let formats: Vec<(&str, Action)> = [EN.qr_ur, EN.qr_bbqr, EN.qr_parts]
        .into_iter()
        .zip(crate::QrFormat::ALL)
        .enumerate()
        .filter(|(_, (_, f))| *f != crate::QrFormat::Parts || text)
        .map(|(i, (label, _))| (label, Action::QrFormat(i as u8)))
        .collect();
    let chosen = crate::QrFormat::ALL.iter().position(|f| *f == q.format);
    bx += ui.segmented((bx, row, 36.0), 13.0, &formats, chosen, None) + 14.0;
    if q.frames.len() > 1 {
        let speeds: Vec<(&str, Action)> = ["2/s", "3/s", "5/s"]
            .into_iter()
            .zip(crate::QR_SPEEDS)
            .map(|(label, ms)| (label, Action::QrSpeed(ms)))
            .collect();
        let on = crate::QR_SPEEDS
            .iter()
            .position(|ms| app.qr_frame_ms() == *ms);
        ui.segmented((bx, row, 36.0), 13.0, &speeds, on, None);
    }
    let bx = x + 32.0;
    if q.offers_png() {
        // One code of public content: kept as a picture with its label.
        ui.button(
            bx,
            y + sh - 32.0 - 44.0,
            None,
            44.0,
            &fill(EN.qr_png, &[app.medium.for_the()]),
            Style::Secondary,
            Action::QrPng,
        );
    } else if q.frames.len() > 1 || q.part != crate::QR_PARTS[1] {
        let sizes: Vec<(&str, Action)> = [EN.qr_small, EN.qr_medium, EN.qr_large]
            .into_iter()
            .zip(crate::QR_PARTS)
            .map(|(label, n)| (label, Action::QrPartSize(n)))
            .collect();
        let on = crate::QR_PARTS.iter().position(|n| q.part == *n);
        ui.segmented((bx, y + sh - 32.0 - 44.0, 44.0), 14.0, &sizes, on, None);
    }
    ui.button(
        x + sw - 32.0 - 100.0,
        y + sh - 32.0 - 44.0,
        Some(100.0),
        44.0,
        EN.qr_done,
        Style::Primary,
        Action::Cancel,
    );
}

/// The QR sheet on a small panel: the code as large as the panel allows,
/// an animated code's frame in the title, and its format, speed and part
/// size as buttons that step through the choices, with PNG beside them
/// for one code of public content. The format line gives way to the code
/// (DESIGN §4.9): with it, a PSBT's frame is one pixel a module at
/// 240x320, or its buttons go under Done.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn qr_sheet_compact(app: &View, q: &crate::QrView, ui: &mut Ui, w: f32, h: f32) {
    let text = matches!(
        q.source,
        crate::QrSource::Text(_) | crate::QrSource::Key(..)
    );
    let formats: Vec<(usize, crate::QrFormat)> = crate::QrFormat::ALL
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, f)| *f != crate::QrFormat::Parts || text)
        .collect();
    let at = formats
        .iter()
        .position(|(_, f)| *f == q.format)
        .unwrap_or(0);
    let (next_i, _) = formats[(at + 1) % formats.len()];
    let fname = [EN.qr_sheet_ur, EN.qr_sheet_bbqr, EN.qr_sheet_parts][crate::QrFormat::ALL
        .iter()
        .position(|f| *f == q.format)
        .unwrap_or(0)];
    let animated = q.frames.len() > 1;
    let sp = crate::QR_SPEEDS
        .iter()
        .position(|ms| *ms == app.qr_frame_ms())
        .unwrap_or(0);
    let pp = crate::QR_PARTS
        .iter()
        .position(|n| *n == q.part)
        .unwrap_or(1);
    let mut cycles: Vec<(&str, Style, Action)> = Vec::new();
    if formats.len() > 1 {
        cycles.push((fname, Style::Secondary, Action::QrFormat(next_i as u8)));
    }
    if animated {
        cycles.push((
            ["2/s", "3/s", "5/s"][sp],
            Style::Secondary,
            Action::QrSpeed(crate::QR_SPEEDS[(sp + 1) % 3]),
        ));
    }
    if animated || q.part != crate::QR_PARTS[1] {
        cycles.push((
            [EN.qr_sheet_small, EN.qr_sheet_medium, EN.qr_sheet_large][pp],
            Style::Secondary,
            Action::QrPartSize(crate::QR_PARTS[(pp + 1) % 3]),
        ));
    }
    let rows_h = if cycles.is_empty() { 0.0 } else { 40.0 };
    crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        let title = if animated {
            fill(
                EN.qr_sheet_frame_count,
                &[
                    &q.title.to_string(),
                    &(q.frame + 1).to_string(),
                    &q.frames.len().to_string(),
                ],
            )
        } else {
            q.title.clone()
        };
        let t = ui.fit(16.0, W::S, &title, iw);
        ui.text(x, cy, 16.0, W::S, TEXT, &t);
        cy += 22.0;
        let (tag, tone) = if q.secret {
            (EN.qr_sheet_secret_whoever, ERR)
        } else {
            (EN.qr_sheet_public_safe, OK)
        };
        let tag = ui.fit(12.0, W::S, tag, iw);
        ui.text(x, cy, 12.0, W::S, tone, &tag);
        cy += 22.0;
        // The room down to the Done button: the margins, the padding, and
        // the button with its own. A descriptor's whole text and the
        // buttons scroll under the code rather than squeeze it: above the
        // text, a 2-of-3 descriptor's code is one pixel a module at
        // 240x320, which no camera reads.
        let room = h - 16.0 - 32.0 - 46.0 - 16.0 - (cy - y) - 8.0;
        let side = if q.whole.is_some() {
            iw.min(room + 16.0)
        } else {
            iw.min(room - rows_h)
        }
        .max(120.0);
        if let Some(m) = q.frames.get(q.frame) {
            qr_code(ui, m, x + (iw - side) / 2.0, cy, side);
        }
        cy += side + 8.0;
        if let Some(t) = &q.whole {
            cy += ui.wrap(x, cy, iw, 11.0, W::M, TEXT, t) + 8.0;
        }
        if !cycles.is_empty() {
            let n = cycles.len() as f32;
            let bw = (iw - (n - 1.0) * 6.0) / n;
            for (i, &(label, style, action)) in cycles.iter().enumerate() {
                ui.button(
                    x + i as f32 * (bw + 6.0),
                    cy,
                    Some(bw),
                    40.0,
                    label,
                    style,
                    action,
                );
            }
            cy += 40.0;
        }
        cy += crate::compact::buttons(
            ui,
            x,
            cy,
            iw,
            &[(EN.qr_sheet_done, Style::Primary, Action::Cancel)],
        );
        cy - y
    });
}

/// A new input device (`docs/history/PLAN.md` §4.6): its name as it gives it, the
/// code a keyboard must type, and the person's choice.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn new_input_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(d) = app.new_inputs().first() else {
        return;
    };
    if ui.compact {
        let what = match (d.keyboard, d.pointer) {
            (true, true) => EN.new_input_title,
            (true, false) => EN.new_input_a_new_keyboard,
            _ => EN.new_input_a_new_pointer,
        };
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Warning, WARN, what);
            cy += ui.wrap(x, cy, iw, 14.0, W::M, TEXT, &d.name) + 6.0;
            cy += ui.wrap(x, cy, iw, 12.0, W::R, MUTED, EN.new_input_the_name) + 12.0;
            if d.keyboard && d.code.is_none() && app.seed_missing() {
                cy += ui.wrap(x, cy, iw, 13.0, W::S, ERR, EN.new_input_no_code) + 12.0;
            }
            if d.keyboard
                && let Some(code) = d.code.as_deref()
            {
                ui.text(x, cy, 13.0, W::S, MUTED, EN.new_input_type_code);
                cy += 24.0;
                let n = code.chars().count().max(1) as f32;
                let bw = ((iw - (n - 1.0) * 6.0) / n).min(52.0);
                let typed = d.typed.chars().count();
                for (k, c) in code.chars().enumerate() {
                    let done = code.starts_with(&d.typed) && k < typed;
                    let cx = x + k as f32 * (bw + 6.0);
                    ui.fill(
                        cx,
                        cy,
                        bw,
                        52.0,
                        8.0,
                        if done { OK.with_alpha(40) } else { INNER },
                    );
                    ui.stroke(cx, cy, bw, 52.0, 8.0, if done { OK } else { BORDER });
                    let cs = c.to_string();
                    let tw = ui.measure(24.0, W::M, &cs);
                    ui.text_mid(cx + (bw - tw) / 2.0, cy, 52.0, 24.0, W::M, TEXT, &cs);
                }
                cy += 64.0;
            }
            let mut items = vec![(
                EN.new_input_ignore_button,
                Style::Secondary,
                Action::InputIgnore(d.id),
            )];
            if !d.keyboard {
                items.push((
                    EN.new_input_use_pointer,
                    Style::Primary,
                    Action::InputUse(d.id),
                ));
            }
            cy += crate::compact::buttons(ui, x, cy, iw, &items);
            cy - y
        });
        return;
    }
    let sh = if d.keyboard { 360.0 } else { 280.0 };
    let (x, y) = sheet_box(ui, w, h, 560.0, sh);
    let ix = x + 32.0;
    let iw = 560.0 - 64.0;
    let what = match (d.keyboard, d.pointer) {
        (true, true) => EN.new_input_title,
        (true, false) => EN.new_input_a_new_keyboard,
        _ => EN.new_input_a_new_pointer,
    };
    ui.icon(ix, y + 28.0, 30.0, Icon::Warning, 17.0, WARN);
    ui.text_mid(ix + 42.0, y + 28.0, 30.0, 20.0, W::S, TEXT, what);
    let name = ui.fit(14.0, W::M, &d.name, iw);
    ui.text(ix, y + 76.0, 14.0, W::M, TEXT, &name);
    ui.text(ix, y + 100.0, 13.0, W::R, MUTED, EN.new_input_the_name);
    let mut by = y + 140.0;
    if d.keyboard && d.code.is_none() && app.seed_missing() {
        let line = ui.fit(13.0, W::S, EN.new_input_no_code, iw);
        ui.text(ix, by, 13.0, W::S, ERR, &line);
        by += 112.0;
    }
    if d.keyboard
        && let Some(code) = d.code.as_deref()
    {
        ui.text(ix, by, 13.0, W::S, MUTED, EN.new_input_type_code);
        by += 28.0;
        let mut cx = ix;
        let typed: Vec<char> = d.typed.chars().collect();
        for (k, c) in code.chars().enumerate() {
            // A letter already typed in order is marked.
            let done = code.starts_with(&d.typed) && k < typed.len();
            ui.fill(
                cx,
                by,
                52.0,
                60.0,
                8.0,
                if done { OK.with_alpha(40) } else { INNER },
            );
            ui.stroke(cx, by, 52.0, 60.0, 8.0, if done { OK } else { BORDER });
            ui.text_mid(cx + 16.0, by, 60.0, 28.0, W::M, TEXT, &c.to_string());
            cx += 60.0;
        }
        by += 84.0;
    }
    let bw = (iw - 12.0) / 2.0;
    ui.button(
        ix,
        y + sh - 32.0 - 46.0,
        Some(if d.keyboard { iw } else { bw }),
        46.0,
        EN.new_input_ignore_button,
        Style::Secondary,
        Action::InputIgnore(d.id),
    );
    if !d.keyboard {
        ui.button(
            ix + bw + 12.0,
            y + sh - 32.0 - 46.0,
            Some(bw),
            46.0,
            EN.new_input_use_pointer,
            Style::Primary,
            Action::InputUse(d.id),
        );
    }
    let _ = by;
}

/// Minutes and seconds, `4:05`.
fn clock_text(ms: u64) -> String {
    let s = ms.div_ceil(1000);
    format!("{}:{:02}", s / 60, s % 60)
}

/// The idle lock coming: how long without input, when it locks, what it
/// wipes, seals and keeps, and when the machine powers off after it.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn idle_warn_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    if app.idle_lock_at().is_none() {
        return;
    }
    let keys = app.session.keys.len();
    let wallets = app.session.wallets.len();
    let sealed: Vec<String> = app
        .vaults()
        .open
        .iter()
        .filter(|v| v.changes > 0)
        .map(|v| v.label(app.vaults.store()))
        .collect();
    let mut rows: Vec<(&str, String)> = vec![(
        EN.idle_warn_wiped,
        fill(
            EN.idle_warn_open,
            &[
                &keys.to_string(),
                (if keys == 1 {
                    EN.idle_warn_key
                } else {
                    EN.idle_warn_keys
                }),
                &wallets.to_string(),
                (if wallets == 1 {
                    EN.idle_warn_wallet
                } else {
                    EN.idle_warn_wallets
                }),
                &app.vaults.open.len().to_string(),
                (if app.vaults.open.len() == 1 {
                    EN.idle_warn_vault
                } else {
                    EN.idle_warn_vaults
                }),
            ],
        ),
    )];
    if !sealed.is_empty() && app.seeded() {
        rows.push((
            EN.idle_warn_sealed,
            format!("{} · {}", sealed.join(", "), app.medium.for_the()),
        ));
    } else if !sealed.is_empty() {
        // No seed to seal with: the idle lock loses the changes.
        rows.push((EN.vault_not_sealed, sealed.join(", ")));
    }
    rows.push((
        EN.idle_warn_kept,
        format!(
            "{} {} · {} {}",
            app.inbox.len(),
            app.medium.from_the(),
            app.outbox.len(),
            app.medium.for_the()
        ),
    ));
    let off = match (app.idle_off_min(), app.online) {
        (0, _) | (_, true) => EN.idle_warn_no_power.to_string(),
        (m, false) => fill(
            EN.idle_warn_power_off,
            &[&m.to_string(), app.medium.for_the()],
        ),
    };
    rows.push((EN.idle_warn_then, off));
    let minutes = |n: u64| {
        format!(
            "{n} {}",
            if n == 1 {
                EN.idle_warn_minute
            } else {
                EN.idle_warn_minutes
            }
        )
    };
    let body = fill(
        EN.idle_warn_no_input,
        &[
            &minutes(app.idle.warn_min()).to_string(),
            &minutes(u64::from(app.idle_lock_min())).to_string(),
        ],
    );
    if ui.compact {
        let rows: Vec<(&str, String, crate::ui::Paint)> =
            rows.into_iter().map(|(k, v)| (k, v, TEXT)).collect();
        crate::compact::kv_sheet(
            ui,
            w,
            h,
            (Icon::Lock, WARN, EN.idle_warn_locking_soon),
            &body,
            &rows,
            &[(EN.idle_warn_keep_working, Style::Primary, Action::Cancel)],
        );
        return;
    }
    let sw = 600.0;
    let sh = 250.0 + rows.len() as f32 * 44.0;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let ix = x + 32.0;
    let iw = sw - 64.0;
    ui.icon(ix, y + 30.0, 30.0, Icon::Lock, 18.0, WARN);
    ui.text_mid(
        ix + 42.0,
        y + 30.0,
        30.0,
        20.0,
        W::S,
        TEXT,
        EN.idle_warn_locking_soon,
    );
    ui.text(ix, y + 76.0, 13.0, W::R, MUTED, &body);
    let mut ry = y + 108.0;
    for (k, v) in &rows {
        ui.text_mid(ix, ry, 40.0, 13.0, W::R, MUTED, k);
        let v = ui.fit(14.0, W::R, v, iw - 90.0);
        ui.text_mid(ix + 90.0, ry, 40.0, 14.0, W::R, TEXT, &v);
        ui.rule(ix, ry + 40.0, iw, BORDER);
        ry += 44.0;
    }
    ui.button(
        ix,
        y + sh - 32.0 - 46.0,
        Some(iw),
        46.0,
        EN.idle_warn_keep_working,
        Style::Primary,
        Action::Cancel,
    );
}

/// The lock screen after a lock for idleness: what waits in the Outbox,
/// which keeps the machine from powering off.
pub(super) fn locked_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let shown = app.outbox.len().min(5);
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Lock, ACCENT, EN.locked_locked);
            cy += ui.wrap(x, cy, iw, 13.0, W::R, MUTED, EN.locked_locked_after) + 12.0;
            if let Some(off) = app.idle_off_at() {
                let line = fill(
                    EN.locked_power_off_countdown,
                    &[&(clock_text(off.saturating_sub(app.idle_ms()))).to_string()],
                );
                cy += ui.wrap(x, cy, iw, 13.0, W::S, TEXT, &line) + 10.0;
            }
            if shown > 0 {
                let line = fill(
                    EN.locked_no_power_off,
                    &[&app.outbox.len().to_string(), app.medium.for_the()],
                );
                cy += ui.wrap(x, cy, iw, 13.0, W::S, WARN, &line) + 8.0;
                for item in app.outbox.iter().take(shown) {
                    let n = ui.fit(13.0, W::M, &item.name, iw);
                    ui.text(x, cy, 13.0, W::M, TEXT, &n);
                    cy += 24.0;
                }
            }
            cy += 8.0;
            cy += crate::compact::buttons(
                ui,
                x,
                cy,
                iw,
                &[(EN.locked_continue, Style::Primary, Action::Cancel)],
            );
            cy - y
        });
        return;
    }
    let sh = 200.0
        + if shown > 0 {
            40.0 + shown as f32 * 30.0
        } else {
            0.0
        };
    let (x, y) = sheet_box(ui, w, h, 520.0, sh);
    let ix = x + 32.0;
    let iw = 520.0 - 64.0;
    ui.icon(ix, y + 28.0, 28.0, Icon::Lock, 16.0, ACCENT);
    ui.text(ix + 40.0, y + 30.0, 20.0, W::S, TEXT, EN.locked_locked);
    ui.text(ix, y + 72.0, 13.0, W::R, MUTED, EN.locked_locked_after);
    let mut ry = y + 108.0;
    if let Some(off) = app.idle_off_at() {
        ui.text(
            ix,
            ry,
            13.0,
            W::S,
            TEXT,
            &fill(
                EN.locked_power_off_countdown,
                &[&(clock_text(off.saturating_sub(app.idle_ms()))).to_string()],
            ),
        );
    }
    if shown > 0 {
        ui.text(
            ix,
            ry,
            13.0,
            W::S,
            WARN,
            &fill(
                EN.locked_no_power_off,
                &[&app.outbox.len().to_string(), app.medium.for_the()],
            ),
        );
        ry += 32.0;
        for item in app.outbox.iter().take(shown) {
            ui.text(ix, ry, 14.0, W::M, TEXT, &item.name);
            ry += 30.0;
        }
    }
    ui.button(
        ix,
        y + sh - 32.0 - 46.0,
        Some(iw),
        46.0,
        EN.locked_continue,
        Style::Primary,
        Action::Cancel,
    );
}

pub(super) fn power_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let restart = app.offers_restart();
    let line = if app.secure_boot_reminder() {
        EN.power_secure_boot_off
    } else {
        ""
    };
    let row = app.bios_key().map(|key| (EN.power_bios_key, key));
    let leave = if app.online {
        EN.power_quit
    } else {
        EN.power_power_off
    };
    let title = if restart { EN.power_title } else { leave };
    let then = if restart {
        EN.power_the_computer_restarts
    } else if app.online {
        EN.power_app_closes
    } else {
        EN.power_the_computer
    };
    // With files waiting for the stick or a vault's changes unsaved,
    // leaving takes a hold (decision 5); with nothing to lose, a press.
    let hold = !app.outbox.is_empty() || app.vaults.open.iter().any(|v| v.changes > 0);
    let mut buttons = vec![(EN.power_cancel, Style::Secondary, Action::Cancel)];
    if restart && hold {
        buttons.push((EN.power_hold_restart, Style::Secondary, Action::HoldRestart));
    } else if restart {
        buttons.push((EN.power_restart, Style::Secondary, Action::Restart));
    }
    if hold {
        let held = if app.online {
            EN.power_hold_quit
        } else {
            EN.power_hold_power_off
        };
        buttons.push((held, Style::Primary, Action::HoldPowerOff));
    } else {
        buttons.push((leave, Style::Primary, Action::PowerOff));
    }
    // The guide's address, and its code: made once, kept on the state.
    let guide = app.secure_boot_guide().filter(|g| !g.is_empty());
    let guide_qr = guide.as_deref().and_then(|g| app.public_qr(g));
    kept_sheet(
        app,
        ui,
        (w, h),
        (Icon::Power.into(), TEXT, title),
        (line, WARN, row, guide.as_deref().zip(guide_qr.as_deref())),
        &kept_sections(app, true, false, then.to_string()),
        &buttons,
    );
}

/// The Learn pages for the screen under the sheet: a tab for
/// each, the page's sections, scrolled by the wheel or the arrow keys.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn learn_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let compact = ui.compact;
    let (sw, sh, pad) = if compact {
        (w - 16.0, h - 16.0, 14.0)
    } else {
        ((w - 80.0).min(860.0), h - 64.0, 28.0)
    };
    // Held, the sheet is the whole frame, a page under the holder's top
    // app bar, whose way back or navigation leaves it (`docs/STARTER.md`
    // §3.1d): no box and no Done.
    let (x, y, sw, sh) = if app.held {
        let sw = w.min(860.0);
        ((w - sw) / 2.0, 0.0, sw, h)
    } else {
        let (x, y) = sheet_box(ui, w, h, sw, sh);
        (x, y, sw, sh)
    };
    let Some(page) = app.learn().pages.get(app.learn().page).copied() else {
        return;
    };
    let ticked = app.learn_ticked(page);
    // Tabs, one per page, and Done; on a small panel they wrap.
    let done_w = if compact { 72.0 } else { 96.0 };
    let tabs_w = sw - 2.0 * pad - if app.held { 0.0 } else { done_w + 8.0 };
    if !app.held {
        ui.button(
            x + sw - pad - done_w,
            y + 14.0,
            Some(done_w),
            34.0,
            EN.learn_done,
            Style::Secondary,
            Action::Cancel,
        );
    }
    // The sheet's first row (`docs/history/SIMPLIFY.md` §6.2): a direct way to
    // the Spend tab, except from the Spend tab itself and in a held app
    // (`Faraday::held`), which has none.
    let mut tabs: Vec<(String, Style, Action)> = Vec::new();
    if app.screen != Screen::Family && !app.held {
        tabs.push((
            EN.learn_spending_step.to_string(),
            Style::Secondary,
            Action::LearnSpend,
        ));
    }
    tabs.extend(app.learn().pages.iter().enumerate().map(|(i, p)| {
        let style = if i == app.learn().page {
            Style::Primary
        } else {
            Style::Secondary
        };
        (p.title.to_string(), style, Action::LearnPage(i as u8))
    }));
    let tabs_h = if tabs.len() > 1 || !compact {
        learn_tabs(ui, x + pad, y + 14.0, tabs_w, &tabs) - 44.0 + 34.0
    } else {
        34.0
    };
    let top = y + 14.0 + tabs_h + 18.0;
    ui.rule(x + pad, top - 8.0, sw - 2.0 * pad, BORDER);
    let (bx, bw) = (x + pad + 4.0, sw - 2.0 * pad - 8.0);
    let view = sh - (top - y) - 12.0;
    let clip = ui.rect(x, top, sw, view);
    ui.c.push_clip(clip);
    let mut cy = top + 12.0 - app.learn().scroll;
    cy += ui.wrap(
        bx,
        cy,
        bw,
        if compact { 18.0 } else { 22.0 },
        W::S,
        TEXT,
        page.title,
    ) + 16.0;
    for section in page.sections {
        if !section.heading.is_empty() {
            cy += ui.wrap(
                bx,
                cy,
                bw,
                if compact { 15.0 } else { 16.0 },
                W::S,
                ACCENT,
                section.heading,
            ) + 8.0;
        }
        // Under a step: what follows it, up to the next step or section,
        // is indented under it.
        let mut under = false;
        for para in section.paragraphs {
            if let Some((n, text)) = faraday_learn::step(para) {
                under = true;
                let tx = bx + STEP_INDENT;
                let line = ui.line(14.0, W::R);
                // The circle level with the text's first line.
                let circle_y = cy + (line - STEP_SIDE) / 2.0;
                step_circle(
                    ui,
                    bx,
                    circle_y,
                    n,
                    ticked.contains(&n),
                    Some(Action::LearnStep(n)),
                );
                let h = ui.wrap(tx, cy, bw - STEP_INDENT, 14.0, W::R, TEXT, text);
                cy += h.max(STEP_SIDE + circle_y - cy) + 12.0;
                continue;
            }
            let (px, pw) = if under {
                (bx + STEP_INDENT, bw - STEP_INDENT)
            } else {
                (bx, bw)
            };
            cy += ui.wrap(px, cy, pw, 14.0, W::R, TEXT, para) + 12.0;
        }
        cy += 10.0;
    }
    ui.c.pop_clip();
    let content = cy + app.learn().scroll - top;
    let max = (content - view).max(0.0);
    app.learn_scroll(max, app.learn().scroll.min(max));
    // The overlay scrollbar, held and dragged as on any page.
    ui.report_scroll(clip, max);
}

/// Where a secret goes: the open vault, or, once the person has said they
/// understand what it gives away, the Outbox unprotected.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn secret_out_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(out) = app.secrets.out() else {
        return;
    };
    // Where it would go into a vault: a seed the vault holds already
    // says so instead of offering to save it again.
    let v = app.vaults.current;
    let vault = app.vaults.open.get(v).map(|o| {
        let held = match out.keep {
            // The key itself, told by its master key's id (F65).
            crate::secrets::Keep::Key { key, fp } => app
                .session
                .keys
                .info(key)
                .filter(|k| k.fingerprint == fp)
                .is_some_and(|k| app.vault_has_key(v, k)),
            _ => false,
        };
        (o.name.as_str(), held)
    });
    // The forms the file can take, the one picked primary.
    let forms: Vec<(&str, Style, Action)> = out
        .forms
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let style = if i == out.form {
                Style::Primary
            } else {
                Style::Secondary
            };
            (f.label, style, Action::SecretForm(i as u8))
        })
        .collect();
    let out_style = if app.secrets.ack() {
        Style::Secondary
    } else {
        Style::Disabled
    };
    // A file a visit would copy to another stick is ticked there, not put
    // in the Outbox; text Faraday cannot tell about may be a secret.
    let head = if out.ack == crate::secrets::Ack::Unknown {
        EN.secret_out_this_may
    } else {
        EN.secret_out_title
    };
    let put_label = fill(EN.secret_out_put_it_unprotected, &[app.medium.for_the()]);
    let (unprotected, out_label) = if out.to_visit {
        (
            fill(EN.secret_out_unprotected_medium, &[app.medium.noun()]),
            EN.secret_out_tick_hint,
        )
    } else {
        (
            fill(EN.secret_out_unprotected, &[app.medium.for_the()]),
            put_label.as_str(),
        )
    };
    let to_vault = !matches!(out.keep, crate::secrets::Keep::None);
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Lock, WARN, head);
            for (label, value) in [
                (EN.secret_out_file, out.name.as_str()),
                (EN.secret_out_what_label, out.what.as_str()),
                (EN.secret_out_who_can, out.gives.as_str()),
            ] {
                cy += crate::compact::kv(ui, x, cy, iw, label, value, TEXT);
            }
            cy += 4.0;
            if to_vault {
                ui.text(x, cy, 13.0, W::S, OK, EN.secret_out_sealed);
                cy += 24.0;
            }
            match vault.filter(|_| to_vault) {
                Some((name, true)) => {
                    let line = ui.fit(13.0, W::S, &fill(EN.secret_out_in_vault, &[name]), iw);
                    ui.text(x, cy, 13.0, W::S, OK, &line);
                    cy += 30.0;
                }
                Some((name, false)) => {
                    // In place: the sheet's foot holds the Outbox's
                    // button and Cancel.
                    let label = fill(EN.secret_out_save_into, &[name]);
                    let label = ui.fit(15.0, W::S, &label, iw - 24.0);
                    ui.button(
                        x,
                        cy,
                        Some(iw),
                        46.0,
                        &label,
                        Style::Primary,
                        Action::SecretVault,
                    );
                    cy += 58.0;
                }
                // Make or unlock one: the sheet waits, and opens again.
                None if to_vault => {
                    if let Some((label, a)) = app.vault_way(app.screen) {
                        let label = ui.fit(15.0, W::S, &label, iw - 24.0);
                        ui.button(x, cy, Some(iw), 46.0, &label, Style::Primary, a);
                        cy += 58.0;
                    }
                }
                None => {}
            }
            ui.text(x, cy, 13.0, W::S, ERR, &unprotected);
            cy += 24.0;
            if forms.len() > 1 {
                cy += wrap_buttons(ui, x, cy, iw, 34.0, &forms) + 4.0;
            }
            ui.checkbox(x, cy + 1.0, app.secrets.ack(), true);
            let lh = ui.wrap(
                x + 28.0,
                cy,
                iw - 28.0,
                13.0,
                W::R,
                TEXT,
                &out.ack.text(app.medium),
            );
            ui.hit(x - 4.0, cy - 6.0, iw, lh + 12.0, Action::SecretAck);
            cy += lh + 12.0;
            cy += crate::compact::buttons(
                ui,
                x,
                cy,
                iw,
                &[
                    (out_label, out_style, Action::SecretUnprotected),
                    (EN.secret_out_cancel, Style::Ghost, Action::Cancel),
                ],
            );
            cy - y
        });
        return;
    }
    let sw = 600.0f32.min(w - 48.0);
    let (bx, bw) = (32.0, sw - 64.0);
    // The acknowledgement on one line, or on two when longer.
    let ack_h = if ui.measure(13.0, W::R, &out.ack.text(app.medium)) > bw - 28.0 {
        36.0
    } else {
        18.0
    };
    let sh = 452.0
        + ack_h
        + if forms.len() > 1 { 44.0 } else { 0.0 }
        + if to_vault { 0.0 } else { -96.0 };
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let bx = x + bx;
    let mut cy = y + 26.0;
    ui.icon(bx - 4.0, cy - 2.0, 28.0, Icon::Lock, 14.0, WARN);
    ui.text(bx + 28.0, cy, 18.0, W::S, TEXT, head);
    cy += 38.0;
    for (label, value, face) in [
        (EN.secret_out_file, out.name.as_str(), W::M),
        (EN.secret_out_what_label, out.what.as_str(), W::R),
        (EN.secret_out_who_can, out.gives.as_str(), W::R),
    ] {
        ui.text(bx, cy, 12.0, W::R, MUTED, label);
        let used = ui.wrap(bx + 130.0, cy, bw - 130.0, 13.0, face, TEXT, value);
        cy += used.max(20.0) + 10.0;
    }
    cy += 6.0;
    // The vault, first, for what a vault keeps.
    if to_vault {
        ui.text(bx, cy, 13.0, W::S, OK, EN.secret_out_sealed);
        cy += 26.0;
    }
    match vault.filter(|_| to_vault) {
        Some((name, true)) => {
            ui.text_mid(
                bx,
                cy,
                42.0,
                13.0,
                W::S,
                OK,
                &fill(EN.secret_out_in_vault, &[name]),
            );
            cy += 54.0;
        }
        Some((name, false)) => {
            let label = fill(EN.secret_out_save_into, &[name]);
            ui.button(
                bx,
                cy,
                None,
                42.0,
                &label,
                Style::Primary,
                Action::SecretVault,
            );
            cy += 54.0;
        }
        // Make or unlock one: the sheet waits, and opens again.
        None if to_vault => {
            if let Some((label, a)) = app.vault_way(app.screen) {
                ui.button(bx, cy, None, 42.0, &label, Style::Primary, a);
            }
            cy += 54.0;
        }
        None => {}
    }
    if to_vault {
        ui.rule(bx, cy, bw, BORDER);
        cy += 16.0;
    }
    // The Outbox, or the stick, only once the person says they
    // understand.
    ui.text(bx, cy, 13.0, W::S, ERR, &unprotected);
    cy += 26.0;
    if forms.len() > 1 {
        cy += wrap_buttons(ui, bx, cy, bw, 34.0, &forms) + 2.0;
    }
    ui.checkbox(bx, cy + 1.0, app.secrets.ack(), true);
    let lh = ui.wrap(
        bx + 28.0,
        cy,
        bw - 28.0,
        13.0,
        W::R,
        TEXT,
        &out.ack.text(app.medium),
    );
    ui.hit(
        bx - 4.0,
        cy - 6.0,
        bw,
        lh.max(18.0) + 12.0,
        Action::SecretAck,
    );
    cy += lh.max(18.0) + 16.0;
    ui.button(
        bx,
        cy,
        None,
        40.0,
        out_label,
        out_style,
        Action::SecretUnprotected,
    );
    ui.button(
        x + sw - 32.0 - 110.0,
        y + sh - 32.0 - 40.0,
        Some(110.0),
        40.0,
        EN.secret_out_cancel,
        Style::Ghost,
        Action::Cancel,
    );
}
