//! The sheets drawn over a screen.

use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{CoreUi, Glyph, Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Screen};
use faraday_draw::tokens as T;
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, FIELD_H, HEADLINE_S, LABEL_L, LABEL_M, LABEL_S, TITLE_M, TITLE_S,
    sheet_head,
};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{
    STEP_INDENT, STEP_SIDE, button_rows, qr_code, sheet_box, step_circle, wrap_buttons,
};

/// The potential sheet's fields under its head: where the seed was
/// found, a passphrase or none, the fingerprint it gives, the wallet here
/// it opens if any, else the kind of single-key wallet. Returns their
/// height.
#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn potential_body(
    app: &View,
    p: &crate::inbox::Potential,
    ui: &mut Ui,
    (x, y, iw): (f32, f32, f32),
    seed: Option<&crate::inbox::FoundSeed>,
    xpub: Option<&(String, bool)>,
    takes: bool,
    matched: Option<&str>,
) -> f32 {
    let kinds = |ui: &mut Ui, x: f32, y: f32, w: f32| -> f32 {
        let row: Vec<(String, Style, Action)> = crate::inbox::KINDS
            .iter()
            .enumerate()
            .map(|(k, kind)| {
                (
                    crate::family::kind_name(*kind).to_string(),
                    Style::chosen(*kind == p.kind),
                    Action::PotentialKind(k as u8),
                )
            })
            .collect();
        button_rows(ui, x, y, w, &row) - 4.0
    };
    let mut cy = y;
    if let Some(s) = seed {
        let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
        cy += ui.wrap_as(
            (x, cy, iw),
            BODY_S,
            Role::OnSurfaceVariant,
            &fill(EN.potential_sources, &[&from.join(" · ").to_string()]),
        ) + 10.0;
    }
    if takes {
        let label = match seed.and_then(|s| s.sources.first()) {
            Some(crate::inbox::SeedSource::Slip39(_)) => EN.potential_slip_39_passphrase,
            _ => EN.potential_bip_39_passphrase,
        };
        cy += ui.wrap_as((x, cy, iw), LABEL_L, Role::OnSurface, label) + 6.0;
        if app.pass_is_public(&p.passphrase) {
            super::parts::code_field(ui, x, cy, iw, T::TEXT_FIELD_SHEET, &p.passphrase);
            ui.hit(x, cy, iw, T::TEXT_FIELD_SHEET, Action::PotentialType);
        } else if app.may_load_keys() {
            let shown = if p.shown {
                zeroize::Zeroizing::new(p.passphrase.to_string())
            } else {
                zeroize::Zeroizing::new("•".repeat(p.passphrase.chars().count().min(48)))
            };
            ui.fill(x, cy, iw, 44.0, 8.0, Role::SurfaceContainerHighest);
            ui.stroke(
                x,
                cy,
                iw,
                44.0,
                8.0,
                if p.typing {
                    Role::Primary
                } else {
                    Role::OutlineVariant
                },
            );
            let shown = ui.fit_secret(BODY_M.size, W::M, &shown, iw - 60.0);
            let tw = ui.text_mid_as((x + 12.0, cy, 44.0), BODY_M.mono(), Role::OnSurface, &shown);
            if p.typing {
                ui.caret(x + 13.0 + tw, cy + 13.0, 18.0);
            }
            ui.hit(x, cy, iw - 44.0, 44.0, Action::PotentialType);
            ui.icon(
                x + iw - 44.0,
                cy,
                44.0,
                if p.shown { Icon::EyeOff } else { Icon::Eye },
                15.0,
                if p.shown {
                    Role::Primary
                } else {
                    Role::OnSurfaceVariant
                },
            );
            let at = (x + iw - 44.0, cy, 44.0, 44.0);
            crate::compact::eye_hit(ui, at, Action::PotentialShow, p.shown);
            app.pass_chip_draw(
                ui,
                p.passphrase.is_empty(),
                crate::demo::PassField::Potential,
            );
        } else {
            crate::vault::screen::stick_field(app.medium, ui, x, cy, iw);
        }
        cy += FIELD_H + 8.0;
    }
    if let Some((key, fixed)) = xpub {
        let k = ui.fit_as(BODY_S.mono(), key, iw);
        ui.text_as(x, cy, BODY_S.mono(), Role::OnSurfaceVariant, &k);
        cy += 24.0;
        if *fixed {
            let line = fill(
                EN.potential_as_its_path,
                &[crate::family::kind_name(p.kind)],
            );
            cy += ui.wrap_as((x, cy, iw), TITLE_S, Role::OnSurface, &line) + 10.0;
        } else {
            cy += kinds(ui, x, cy, iw);
        }
    } else {
        if let Some(with) = p.with {
            ui.text_as(
                x,
                cy,
                BODY_S,
                Role::OnSurfaceVariant,
                EN.potential_fingerprint,
            );
            ui.text_as(x + 90.0, cy, BODY_M.mono(), Role::OnSurface, &fp_text(with));
            cy += 24.0;
        }
        match matched {
            Some(name) => {
                let line = fill(EN.potential_key_of_wallet, &[name]);
                cy += ui.wrap_as((x, cy, iw), LABEL_L, Role::Ok, &line) + 10.0;
            }
            None => {
                ui.text_as(
                    x,
                    cy,
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.potential_single_key,
                );
                cy += 22.0;
                cy += kinds(ui, x, cy, iw);
            }
        }
    }
    if let Some(e) = &p.error {
        cy += ui.wrap_as((x, cy, iw), BODY_M, Role::Error, e) + 8.0;
    }
    cy - y
}

/// Making a seed from the Inbox a wallet, a dialog (`docs/DESIGN.md`
/// §4.13): [`potential_body`], then Cancel and Make the wallet, or Load.
pub(super) fn potential_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(p) = app.inbox.potential() else {
        return;
    };
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
    let title = match (xpub, p.fingerprint) {
        (Some(_), Some(fp)) => fill(
            EN.potential_a_watch_only_wallet,
            &[&fp_text(fp).to_string()],
        ),
        (Some(_), None) => EN.potential_a_watch_only.to_string(),
        (None, Some(fp)) => fill(EN.potential_a_wallet, &[&fp_text(fp).to_string()]),
        _ => String::new(),
    };
    let go = match matched.as_deref() {
        Some(name) if xpub.is_none() => fill(EN.potential_load, &[name]),
        _ => EN.potential_make_wallet.to_string(),
    };
    let actions = [
        (EN.potential_cancel, Style::Secondary, Action::Cancel),
        (
            go.as_str(),
            Style::Primary.when(app.may_load_keys() || xpub.is_some()),
            Action::PotentialMake,
        ),
    ];
    let head = (Icon::Keys.into(), Role::Primary.paint(), title.as_str());
    let (seed, matched) = (seed.as_ref(), matched.as_deref());
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| potential_body(app, p, ui, (x, y, iw), seed, xpub, takes, matched),
        &actions,
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
        let natural = ui.measure_as(LABEL_L, label) + 32.0;
        if natural <= w {
            if bx > x && bx + natural > x + w {
                bx = x;
                by += 40.0;
            }
            bx += ui.button(bx, by, Some(natural), BUTTON_H, label, *style, *action) + 6.0;
        } else {
            if bx > x {
                bx = x;
                by += 40.0;
            }
            let lines = ui.wrap_lines_as(LABEL_L, label, w - 24.0).max(1);
            let line_h = ui.line_as(LABEL_L);
            let bh = 32.0 + (lines - 1) as f32 * line_h;
            // An empty-labelled button draws this style's background,
            // border and hit region exactly as any other; the label is
            // then wrapped over it, since `Ui::button` itself only ever
            // draws one line.
            ui.button(bx, by, Some(w), bh, "", *style, *action);
            let fg = style.label();
            let top = by + (bh - lines as f32 * line_h) / 2.0;
            ui.wrap_as((bx + 12.0, top, w - 24.0), LABEL_L, fg, label);
            by += bh + 6.0;
            bx = x;
        }
    }
    by - y + 44.0
}

/// The device check's buttons: Check again, then Continue anyway, whose
/// question has its hold in the body (F136), or Power off on a virtual
/// machine, which has no way on.
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
        b.push((s.devices_cancel, Style::Secondary, Action::Cancel));
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
        Role::OnSurfaceVariant.paint()
    } else {
        Role::Warning.paint()
    }
}

/// What is attached that can capture the screen or the keys
/// (`docs/STARTER.md` §3.1a): a line for each, and what holds them;
/// before a secret, the reminders due under them (F24).
pub(super) fn devices_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let s = &crate::strings::EN;
    let d = &app.devices;
    let mut lines: Vec<(String, crate::ui::Paint)> = d
        .held
        .iter()
        .map(|c| (c.line(), Role::OnSurface.paint()))
        .collect();
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
        Some((s.devices_real_computer, Role::Error))
    } else if d.asking {
        Some((s.devices_continue_ask, Role::Warning))
    } else {
        None
    };
    let buttons = devices_buttons(app);
    let head = (Icon::Warning.into(), Role::Warning.paint(), title);
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            for (l, tone) in &lines {
                cy += ui.wrap_as((x, cy, iw), BODY_M, *tone, l) + 8.0;
            }
            if let Some((f, tone)) = foot {
                cy += 4.0;
                cy += ui.wrap_as((x, cy, iw), TITLE_S, tone, f) + 8.0;
            }
            // Continue anyway's hold, under the case it is for (F136).
            if d.asking && !d.refuses() {
                crate::vault::screen::hold_button_in(
                    app,
                    ui,
                    x,
                    cy,
                    iw,
                    s.devices_continue_hold,
                    Action::DevicesHold,
                    Role::Warning.paint(),
                );
                cy += T::CHIP_LINE + T::GAP;
            }
            cy - y
        },
        &buttons,
    );
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
    let head = (app.medium.icon(), Role::Warning.paint(), title.as_str());
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += ui.wrap_as((x, cy, iw), BODY_S, Role::OnSurfaceVariant, &line) + 16.0;
            for r in &reminders {
                cy += ui.wrap_as((x, cy, iw), BODY_M, reminder_tone(r), r) + 8.0;
            }
            cy - y
        },
        &[(EN.pull_cancel, Style::Secondary, Action::Cancel)],
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
            kw.kept
                .into_iter()
                .map(|l| (l, Role::OnSurface.paint(), None))
                .collect(),
        ),
        (
            EN.kept_sections_wiped,
            kw.wiped
                .into_iter()
                .map(|(l, nowhere, s)| {
                    (
                        l,
                        if nowhere {
                            Role::Error.paint()
                        } else {
                            Role::OnSurface.paint()
                        },
                        s.filter(|_| save),
                    )
                })
                .collect(),
        ),
        (
            EN.kept_sections_then,
            vec![(then, Role::OnSurface.paint(), None)],
        ),
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

/// A dialog that says what a lock or a power-off does: its title, a line,
/// the sections, each line wrapped to the dialog, its holds, and its
/// actions.
fn kept_sheet(
    app: &View,
    ui: &mut Ui,
    (w, h): (f32, f32),
    head: (Glyph, crate::ui::Paint, &str),
    (line, line_tone, row, code): SheetLine<'_>,
    sections: &[Section],
    actions: &[(&str, Style, Action)],
) {
    // A hold (decision 5) fills while it is held, in the body; the rest
    // are pressed, the dialog's actions.
    let pressed: Vec<(&str, Style, Action)> = actions
        .iter()
        .copied()
        .filter(|a| !crate::vault::is_hold(a.2))
        .collect();
    let frame = (w, h, T::DIALOG_MAX);
    crate::compact::dialog(
        ui,
        frame,
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            if !line.is_empty() {
                let role = if line_tone == Role::Warning.paint() || line_tone == Role::Error.paint()
                {
                    BODY_M
                } else {
                    BODY_S
                };
                cy += ui.wrap_as((x, cy, iw), role, line_tone, line) + 12.0;
            }
            if let Some((label, value)) = row {
                cy += crate::compact::kv(ui, x, cy, iw, label, value, Role::OnSurface);
            }
            if let Some((code, m)) = code {
                // The same size the Starter's restart sheet draws its code.
                let side = if ui.compact { 104.0f32 } else { 160.0 }.min(iw);
                qr_code(ui, m, x, cy, side);
                cy += side + 10.0;
                cy += ui.wrap_as((x, cy, iw), BODY_S.mono(), Role::OnSurfaceVariant, code) + 12.0;
            }
            for (name, lines) in sections {
                if lines.is_empty() {
                    continue;
                }
                ui.rule(x, cy, iw, Role::OutlineVariant);
                cy += 12.0;
                ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, name);
                cy += 22.0;
                for (text, tone, act) in lines {
                    let Some(act) = *act else {
                        cy += ui.wrap_as((x, cy, iw), BODY_M, *tone, text) + 8.0;
                        continue;
                    };
                    // The line beside its button, at the right.
                    let bw = ui.measure_as(TITLE_S, EN.vault_save) + 2.0 * T::PAD;
                    let inset = (T::CHIP_LINE - ui.line_as(BODY_M)) / 2.0;
                    let tw = iw - bw - T::GAP;
                    let th = ui.wrap_as((x, cy + inset, tw), BODY_M, *tone, text);
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
            let mut hx = x;
            for &(label, _, action) in actions.iter().filter(|a| crate::vault::is_hold(a.2)) {
                let room = x + iw - hx;
                hx += crate::vault::screen::hold_button_in(
                    app,
                    ui,
                    hx,
                    cy,
                    room,
                    label,
                    action,
                    Role::Error.paint(),
                ) + T::GAP;
            }
            if hx > x {
                cy += T::CHIP_LINE + T::GAP + T::GAP_SMALL;
            }
            cy - y
        },
        &pressed,
    );
}

pub(super) fn lock_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let label = app
        .stick
        .first()
        .map(|s| s.label.clone())
        .unwrap_or_default();
    let then = fill(EN.lock_a_fresh_start, &[app.medium.noun()]);
    let attached = fill(EN.lock_is_attached, &[app.medium.a_cap()]);
    let use_it = fill(EN.lock_lock_and_use, &[app.medium.noun()]);
    let line = fill(EN.lock_nothing_has_been, &[&label.to_string()]);
    kept_sheet(
        app,
        ui,
        (w, h),
        (app.medium.icon(), Role::Warning.paint(), &attached),
        (&line, Role::OnSurfaceVariant.paint(), None, None),
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
    let blocked = app.vaults().seal_blocked();
    let (head, line, lock) = if app.seal_waits() {
        (
            (Icon::Error.into(), Role::Error.paint(), EN.vault_lock),
            (EN.seed_missing, Role::Error.paint()),
            (EN.vault_lock_without_sealing, Action::LockUnsealed),
        )
    } else if let Some(why) = blocked.as_deref() {
        (
            (Icon::Error.into(), Role::Error.paint(), EN.vault_lock),
            (why, Role::Error.paint()),
            (EN.vault_lock_without_sealing, Action::LockUnsealed),
        )
    } else {
        (
            (Icon::Lock.into(), Role::Primary.paint(), EN.vault_lock),
            ("", Role::OnSurfaceVariant.paint()),
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
                drops
                    .into_iter()
                    .map(|n| (n, Role::OnSurface.paint(), None))
                    .collect(),
            ),
        );
    }
    kept_sheet(
        app,
        ui,
        (w, h),
        (app.medium.icon(), Role::Primary.paint(), &title),
        ("", Role::OnSurfaceVariant.paint(), None, None),
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
    let hint_h = hint.map_or(0.0, |t| ui.wrap_height_as(BODY_S, t, side_w) + 8.0);
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
        ui.wrap_height_as(BODY_S, EN.never_real_seed, side_w) + 8.0
    } else {
        0.0
    };
    let keys_h =
        keys.len() as f32 * 20.0 + if keys.is_empty() { 0.0 } else { 6.0 } + hint_h + never_h;
    let typing = for_wallet.is_some() || (intake && app.may_load_keys());
    let type_row = if typing && ui.compact { 52.0 } else { 0.0 };
    // The scanner's sample goes in the "Demo only" card (`docs/DEMO.md`
    // §14).
    if app.scan_sample_offered() {
        ui.demo_offer(app.scan_sample_label(), Action::ScanSample);
    }
    let extra = rows + keys_h + type_row;
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
    let title = ui.fit_as(TITLE_M, &title, sw - 64.0);
    ui.text_as(x + 32.0, y + 26.0, TITLE_M, Role::OnSurface, &title);
    for (k, key) in keys.iter().enumerate() {
        let key = ui.fit_as(BODY_M.mono(), key, sw - 64.0);
        ui.text_as(
            x + 32.0,
            y + top + k as f32 * 20.0,
            BODY_M.mono(),
            Role::OnSurface,
            &key,
        );
    }
    if let Some(t) = hint {
        let ty = y + top + keys.len() as f32 * 20.0;
        ui.wrap_as((x + 32.0, ty, side_w), BODY_S, Role::OnSurface, t);
    }
    if seed_scan {
        let ty = y + top + keys.len() as f32 * 20.0 + hint_h;
        ui.wrap_as(
            (x + 32.0, ty, side_w),
            BODY_M,
            Role::Warning,
            EN.never_real_seed,
        );
    }
    let y = y + keys_h;
    let (px, py) = (x + 32.0, y + top);
    let rect = ui.rect(px, py, side_w, side_h);
    ui.fill(px, py, side_w, side_h, 8.0, Role::SurfaceContainerHighest);
    // A QR found and read, still in view: the brackets turn green.
    let mut read_in_view = false;
    if let Some((fw, fh, luma)) = sc.frame.as_ref() {
        ui.c.luma(
            rect,
            *fw,
            *fh,
            luma,
            sc.chroma.as_ref().map(|c| c.as_slice()),
        );
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
            let color = ui.col(if seen.read { Role::Ok } else { Role::Warning });
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
        ui.measure_as(TITLE_S, EN.scan_paste) + 32.0 + 12.0
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
                    ui.fit_as(LABEL_M, name, sw - 96.0),
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
                    ui.fit_as(LABEL_M, name, 180.0),
                    Style::chosen(on),
                    Action::ScanCamera(i as u8),
                )
            })
            .collect();
        let typing = if typing { 220.0 } else { 0.0 };
        button_rows(
            ui,
            x + 32.0,
            y - keys_h + sh - 32.0 - 44.0 + 6.0,
            sw - 64.0 - 140.0 - typing - paste_w,
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
    if let Some(action) = type_instead {
        let label = EN.scan_type_the_words;
        if ui.compact {
            ui.button(
                x + 32.0,
                foot - 52.0,
                Some(sw - 64.0),
                BUTTON_H,
                label,
                Style::Secondary,
                action,
            );
        } else {
            let bw = ui.measure_as(TITLE_S, label) + 32.0;
            left -= bw;
            ui.button(
                left,
                foot,
                Some(bw),
                BUTTON_H,
                label,
                Style::Secondary,
                action,
            );
            left -= 12.0;
        }
    }
    if paste {
        let bw = paste_w - 12.0;
        ui.button(
            left - bw,
            foot,
            Some(bw),
            BUTTON_H,
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
        BUTTON_H,
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
    let size = BODY_S;
    let pad = tokens::VIEWFINDER_BAND_PAD;
    let line_h = ui.line_as(size);
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
    let line = ui.fit_as(size, sc.state_line(), w - 2.0 * pad);
    let lw = ui.measure_as(size, &line);
    ui.text_as(x + (w - lw) / 2.0, band_y + pad, size, VIEWFINDER, &line);
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
            whole_h = ui.wrap_height_as(LABEL_S.mono(), t, tw) + 10.0;
            side = (h - 272.0 - whole_h).clamp(240.0, 560.0);
        }
        whole_h = ui.wrap_height_as(LABEL_S.mono(), t, (side + 64.0).max(420.0) - 64.0) + 10.0;
    }
    let sw = (side + 64.0).max(420.0);
    let sh = side + 242.0 + whole_h;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let t = ui.fit_as(TITLE_M, &q.title, sw - 64.0);
    ui.text_as(x + 32.0, y + 26.0, TITLE_M, Role::OnSurface, &t);
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
    let sub = ui.fit_as(BODY_S, &sub, sw - 64.0);
    ui.text_as(x + 32.0, y + 50.0, BODY_S, Role::OnSurfaceVariant, &sub);
    // Whether scanning it is safe, said beside the code itself.
    let (tag, tone) = if q.secret {
        (EN.qr_secret_whoever_scans, Role::Error)
    } else {
        (EN.qr_public_safe_to_scan, Role::Ok)
    };
    ui.text_as(x + 32.0, y + 66.0, LABEL_M, tone, tag);
    if let Some(m) = q.frames.get(q.frame) {
        qr_code(ui, m, x + (sw - side) / 2.0, y + 86.0, side);
    }
    if let Some(t) = &q.whole {
        ui.wrap_as(
            (x + 32.0, y + 86.0 + side + 10.0, sw - 64.0),
            LABEL_S.mono(),
            Role::OnSurface,
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
            BUTTON_H,
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
        BUTTON_H,
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
        let t = ui.fit_as(TITLE_M, &title, iw);
        ui.text_as(x, cy, TITLE_M, Role::OnSurface, &t);
        cy += 22.0;
        let (tag, tone) = if q.secret {
            (EN.qr_sheet_secret_whoever, Role::Error)
        } else {
            (EN.qr_sheet_public_safe, Role::Ok)
        };
        let tag = ui.fit_as(LABEL_M, tag, iw);
        ui.text_as(x, cy, LABEL_M, tone, &tag);
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
            cy += ui.wrap_as((x, cy, iw), LABEL_S.mono(), Role::OnSurface, t) + 8.0;
        }
        if !cycles.is_empty() {
            let n = cycles.len() as f32;
            let bw = (iw - (n - 1.0) * 6.0) / n;
            for (i, &(label, style, action)) in cycles.iter().enumerate() {
                ui.button(
                    x + i as f32 * (bw + 6.0),
                    cy,
                    Some(bw),
                    BUTTON_H,
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
pub(super) fn new_input_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(d) = app.new_inputs().first() else {
        return;
    };
    let what = match (d.keyboard, d.pointer) {
        (true, true) => EN.new_input_title,
        (true, false) => EN.new_input_a_new_keyboard,
        _ => EN.new_input_a_new_pointer,
    };
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
    let head = (Icon::Warning.into(), Role::Warning.paint(), what);
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += ui.wrap_as((x, cy, iw), BODY_M.mono(), Role::OnSurface, &d.name) + 6.0;
            cy += ui.wrap_as(
                (x, cy, iw),
                BODY_S,
                Role::OnSurfaceVariant,
                EN.new_input_the_name,
            ) + 12.0;
            if d.keyboard && d.code.is_none() && app.seed_missing() {
                cy += ui.wrap_as((x, cy, iw), LABEL_L, Role::Error, EN.new_input_no_code) + 12.0;
            }
            if d.keyboard
                && let Some(code) = d.code.as_deref()
            {
                ui.text_as(
                    x,
                    cy,
                    LABEL_L,
                    Role::OnSurfaceVariant,
                    EN.new_input_type_code,
                );
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
                        if done {
                            Role::OkContainer
                        } else {
                            Role::SurfaceContainerHighest
                        },
                    );
                    ui.stroke(
                        cx,
                        cy,
                        bw,
                        52.0,
                        8.0,
                        if done { Role::Ok } else { Role::OutlineVariant },
                    );
                    let cs = c.to_string();
                    let tw = ui.measure_as(HEADLINE_S.mono(), &cs);
                    let ink = if done {
                        Role::OnOkContainer
                    } else {
                        Role::OnSurface
                    };
                    ui.text_mid_as(
                        (cx + (bw - tw) / 2.0, cy, 52.0),
                        HEADLINE_S.mono(),
                        ink,
                        &cs,
                    );
                }
                cy += 64.0;
            }
            cy - y
        },
        &items,
    );
}

/// Minutes and seconds, `4:05`.
fn clock_text(ms: u64) -> String {
    let s = ms.div_ceil(1000);
    format!("{}:{:02}", s / 60, s % 60)
}

/// The idle lock coming: how long without input, when it locks, what it
/// wipes, seals and keeps, and when the machine powers off after it.
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
    let rows: Vec<(&str, String, crate::ui::Paint)> = rows
        .into_iter()
        .map(|(k, v)| (k, v, Role::OnSurface.paint()))
        .collect();
    crate::compact::kv_sheet(
        ui,
        w,
        h,
        (Icon::Lock, Role::Warning.paint(), EN.idle_warn_locking_soon),
        &body,
        &rows,
        &[(EN.idle_warn_keep_working, Style::Primary, Action::Cancel)],
    );
}

/// The lock screen after a lock for idleness: what waits in the Outbox,
/// which keeps the machine from powering off.
pub(super) fn locked_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    let shown = app.outbox.len().min(5);
    let head = (Icon::Lock.into(), Role::Primary.paint(), EN.locked_locked);
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += ui.wrap_as((x, cy, iw), BODY_M, muted, EN.locked_locked_after) + 12.0;
            if let Some(off) = app.idle_off_at() {
                let line = fill(
                    EN.locked_power_off_countdown,
                    &[&(clock_text(off.saturating_sub(app.idle_ms()))).to_string()],
                );
                cy += ui.wrap_as((x, cy, iw), LABEL_L, ink, &line) + 10.0;
            }
            if shown > 0 {
                let line = fill(
                    EN.locked_no_power_off,
                    &[&app.outbox.len().to_string(), app.medium.for_the()],
                );
                cy += ui.wrap_as((x, cy, iw), LABEL_L, Role::Warning, &line) + 8.0;
                for item in app.outbox.iter().take(shown) {
                    let n = ui.fit_as(BODY_M.mono(), &item.name, iw);
                    ui.text_as(x, cy, BODY_M.mono(), ink, &n);
                    cy += 24.0;
                }
            }
            cy - y
        },
        &[(EN.locked_continue, Style::Primary, Action::Cancel)],
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
        (Icon::Power.into(), Role::OnSurface.paint(), title),
        (
            line,
            Role::Warning.paint(),
            row,
            guide.as_deref().zip(guide_qr.as_deref()),
        ),
        &kept_sections(app, true, false, then.to_string()),
        &buttons,
    );
}

/// How the Learn sheet or Learn's supporting pane lays out its tabs and
/// the page on show: one body, [`learn_body`], drawn by both.
pub(super) struct LearnLook {
    /// The margin at each side of the tabs and the text.
    pub(super) pad: f32,
    /// The tabs' top below the box's.
    pub(super) tabs_top: f32,
    /// The room left at the tabs' end, for Done or the close button.
    pub(super) end: f32,
    /// A single tab still takes its row (not on the small panel).
    pub(super) one_tab_row: bool,
    /// The text's view's top below the tabs.
    pub(super) after_tabs: f32,
    /// How far above the text's view the rule stands.
    pub(super) rule: f32,
    /// The text's inset within the margin, at each side.
    pub(super) inset: f32,
    /// The text's top within its view.
    pub(super) text_top: f32,
    /// The room under the text's view, inside the box.
    pub(super) foot: f32,
    /// The page's title.
    pub(super) head: T::TypeRole,
    /// A section's heading.
    pub(super) heading: T::TypeRole,
    /// The running text and the steps.
    pub(super) body: T::TypeRole,
    /// The room under the title, a heading, a paragraph and a section.
    pub(super) after: [f32; 4],
    /// What is scrolled out of the text's view takes no press.
    pub(super) clip_hits: bool,
}

/// The Learn sheet's tabs: a direct way to the Spend tab first
/// (`docs/history/SIMPLIFY.md` §6.2), except from the Spend tab itself
/// and in a held app (`Faraday::held`), which has none; then one per
/// page, the one on show marked.
fn learn_tab_items(app: &View) -> Vec<(String, Style, Action)> {
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
    tabs
}

/// Learn's tabs and the page on show in the box (x, y, w, h), laid out
/// as `look` sets them: the body the Learn sheet and Learn's supporting
/// pane both draw. The text scrolls by Learn's own offset, kept after
/// the pass. Returns the text's view and how far it scrolls; nothing
/// where no page is open.
pub(super) fn learn_body(
    app: &View,
    ui: &mut Ui,
    look: &LearnLook,
    (x, y, sw, sh): (f32, f32, f32, f32),
) -> Option<(faraday_draw::Rect, f32)> {
    let page = app.learn().pages.get(app.learn().page).copied()?;
    let pad = look.pad;
    // Tabs, one per page; on a small panel they wrap.
    let tabs = learn_tab_items(app);
    let tabs_w = sw - 2.0 * pad - look.end;
    let tabs_h = if tabs.len() > 1 || look.one_tab_row {
        learn_tabs(ui, x + pad, y + look.tabs_top, tabs_w, &tabs) - 44.0 + 34.0
    } else {
        34.0
    };
    let top = y + look.tabs_top + tabs_h + look.after_tabs;
    ui.rule(
        x + pad,
        top - look.rule,
        sw - 2.0 * pad,
        Role::OutlineVariant,
    );
    let (bx, bw) = (x + pad + look.inset, sw - 2.0 * pad - 2.0 * look.inset);
    let view = sh - (top - y) - look.foot;
    let clip = ui.rect(x, top, sw, view);
    let scroll = app.learn().scroll;
    ui.c.push_clip(clip);
    let mark = ui.hits.len();
    let text = learn_page(app, ui, look, page, (bx, top + look.text_top - scroll, bw));
    if look.clip_hits {
        let mut k = mark;
        while k < ui.hits.len() {
            let r = ui.hits[k].0.intersect(&clip);
            if r.w <= 0 || r.h <= 0 {
                ui.hits.remove(k);
            } else {
                ui.hits[k].0 = r;
                k += 1;
            }
        }
    }
    ui.c.pop_clip();
    let max = (look.text_top + text - view).max(0.0);
    app.learn_scroll(max, scroll.min(max));
    Some((clip, max))
}

/// The page's title and its sections from (x, y), `w` wide: headings,
/// paragraphs, and steps with their circles, what follows a step indented
/// under it. Returns the height used.
fn learn_page(
    app: &View,
    ui: &mut Ui,
    look: &LearnLook,
    page: &'static faraday_learn::Page,
    (bx, y, bw): (f32, f32, f32),
) -> f32 {
    let ticked = app.learn_ticked(page);
    let [after_head, after_heading, after_para, after_section] = look.after;
    let mut cy = y;
    cy += ui.wrap_as((bx, cy, bw), look.head, Role::OnSurface, page.title) + after_head;
    for section in page.sections {
        if !section.heading.is_empty() {
            let at = (bx, cy, bw);
            cy += ui.wrap_as(at, look.heading, Role::Primary, section.heading) + after_heading;
        }
        // Under a step: what follows it, up to the next step or section,
        // is indented under it.
        let mut under = false;
        for para in section.paragraphs {
            if let Some((n, text)) = faraday_learn::step(para) {
                under = true;
                let tx = bx + STEP_INDENT;
                let line = ui.line_as(look.body);
                // The circle level with the text's first line.
                let circle_y = cy + (line - STEP_SIDE) / 2.0;
                let action = Some(Action::LearnStep(n));
                step_circle(ui, bx, circle_y, n, ticked.contains(&n), action);
                let at = (tx, cy, bw - STEP_INDENT);
                let h = ui.wrap_as(at, look.body, Role::OnSurface, text);
                cy += h.max(STEP_SIDE + circle_y - cy) + after_para;
                continue;
            }
            let (px, pw) = if under {
                (bx + STEP_INDENT, bw - STEP_INDENT)
            } else {
                (bx, bw)
            };
            cy += ui.wrap_as((px, cy, pw), look.body, Role::OnSurface, para) + after_para;
        }
        cy += after_section;
    }
    cy - y
}

/// The Learn pages for the screen under the sheet: a tab for
/// each, the page's sections, scrolled by the wheel or the arrow keys.
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
    if app.learn().pages.get(app.learn().page).is_none() {
        return;
    }
    let done_w = if compact { 72.0 } else { 96.0 };
    if !app.held {
        ui.button(
            x + sw - pad - done_w,
            y + 14.0,
            Some(done_w),
            BUTTON_H,
            EN.learn_done,
            Style::Secondary,
            Action::Cancel,
        );
    }
    let look = LearnLook {
        pad,
        tabs_top: 14.0,
        end: if app.held { 0.0 } else { done_w + 8.0 },
        one_tab_row: !compact,
        after_tabs: 18.0,
        rule: 8.0,
        inset: 4.0,
        text_top: 12.0,
        foot: 12.0,
        head: sheet_head(compact).medium(),
        heading: TITLE_M,
        body: BODY_M,
        after: [16.0, 8.0, 12.0, 10.0],
        clip_hits: false,
    };
    if let Some((clip, max)) = learn_body(app, ui, &look, (x, y, sw, sh)) {
        // The overlay scrollbar, held and dragged as on any page.
        ui.report_scroll(clip, max);
    }
}

/// Where a secret goes: the open vault, or, past **Save without
/// encryption?**'s hold, the Outbox unprotected (F129).
pub(super) fn secret_out_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    if let Some(plain) = app.secrets.plain() {
        return plain_sheet(app, ui, (w, h), plain);
    }
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
    // The forms the file can take, the one picked selected.
    let forms: Vec<(&str, Style, Action)> = out
        .forms
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let style = Style::chosen(i == out.form);
            (f.label, style, Action::SecretForm(i as u8))
        })
        .collect();
    // A file a visit would copy to another stick is ticked there, not put
    // in the Outbox; text Faraday cannot tell about may be a secret.
    let head = if out.ack == crate::secrets::Ack::Unknown {
        EN.secret_out_this_may
    } else {
        EN.secret_out_title
    };
    let out_label = fill(EN.secret_out_put_it_unprotected, &[app.medium.for_the()]);
    let unprotected = if out.to_visit {
        fill(EN.secret_out_unprotected_medium, &[app.medium.noun()])
    } else {
        fill(EN.secret_out_unprotected, &[app.medium.for_the()])
    };
    let to_vault = !matches!(out.keep, crate::secrets::Keep::None);
    let title = (Icon::Lock.into(), Role::Warning.paint(), head);
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        title,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            for (label, value) in [
                (EN.secret_out_file, out.name.as_str()),
                (EN.secret_out_what_label, out.what.as_str()),
                (EN.secret_out_who_can, out.gives.as_str()),
            ] {
                cy += crate::compact::kv(ui, x, cy, iw, label, value, Role::OnSurface);
            }
            cy += 4.0;
            if to_vault {
                ui.text_as(x, cy, LABEL_L, Role::Ok, EN.secret_out_sealed);
                cy += 24.0;
            }
            match vault.filter(|_| to_vault) {
                Some((name, true)) => {
                    let line = ui.fit_as(LABEL_L, &fill(EN.secret_out_in_vault, &[name]), iw);
                    ui.text_as(x, cy, LABEL_L, Role::Ok, &line);
                    cy += 30.0;
                }
                Some((name, false)) => {
                    // In place: the sheet's foot holds the Outbox's
                    // button and Cancel.
                    let label = fill(EN.secret_out_save_into, &[name]);
                    let label = ui.fit_as(TITLE_M, &label, iw - 24.0);
                    ui.button(
                        x,
                        cy,
                        Some(iw),
                        BUTTON_H,
                        &label,
                        Style::Primary,
                        Action::SecretVault,
                    );
                    cy += 58.0;
                }
                // Make or unlock one: the sheet waits, and opens again.
                None if to_vault => {
                    if let Some((label, a)) = app.vault_way(app.screen) {
                        let label = ui.fit_as(TITLE_M, &label, iw - 24.0);
                        ui.button(x, cy, Some(iw), BUTTON_H, &label, Style::Primary, a);
                        cy += 58.0;
                    }
                }
                None => {}
            }
            ui.text_as(x, cy, LABEL_L, Role::Error, &unprotected);
            cy += 24.0;
            if forms.len() > 1 {
                cy += wrap_buttons(ui, x, cy, iw, BUTTON_H, &forms) + 4.0;
            }
            cy - y
        },
        &[
            (&out_label, Style::Secondary, Action::SecretUnprotected),
            (EN.secret_out_cancel, Style::Ghost, Action::Cancel),
        ],
    );
}

/// **Save without encryption?** (F129): a secret about to be saved as a
/// plain file, outside any vault. Its two lines say where it goes and how
/// safe that is, the one explanation a working sheet carries; the hold in
/// its body saves it; **Put it in a vault** where the secret sheet's
/// secret can go into the open vault, else **Cancel**.
fn plain_sheet(app: &View, ui: &mut Ui, (w, h): (f32, f32), plain: &crate::secrets::Plain) {
    let first = match plain.then {
        crate::secrets::PlainThen::Transfer(..) => {
            fill(EN.plain_goes_into, &[&plain.name, &plain.on])
        }
        _ => fill(
            EN.plain_goes_on,
            &[&plain.name, &plain.on, app.medium.noun()],
        ),
    };
    let second = fill(EN.plain_as_safe, &[&plain.keep]);
    let to_vault = matches!(plain.then, crate::secrets::PlainThen::Secret)
        && app.vaults.open.get(app.vaults.current).is_some()
        && app
            .secrets
            .out()
            .is_some_and(|o| !matches!(o.keep, crate::secrets::Keep::None));
    let other = if to_vault {
        (EN.plain_into_vault, Style::Secondary, Action::SecretVault)
    } else {
        (EN.secret_out_cancel, Style::Ghost, Action::Cancel)
    };
    let title = (Icon::Lock.into(), Role::Warning.paint(), EN.plain_title);
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        title,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurface, &first) + T::GAP;
            cy += ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurfaceVariant, &second) + T::GAP;
            crate::vault::screen::hold_button_in(
                app,
                ui,
                x,
                cy,
                iw,
                EN.plain_hold,
                Action::SecretHold,
                Role::Error.paint(),
            );
            cy += T::CHIP_LINE + T::GAP;
            cy - y
        },
        &[other],
    );
}

/// A wallet a key not of its own can spend alone, waiting to be added
/// (F132): one line naming the wallet, the key in groups of four in the
/// mono face, and a hold that adds it, the confirmation DESIGN §4.13
/// gives an act that can lose everything; Cancel is the dialog's action.
/// Short enough that the hold shows without scrolling on the Pi panel.
pub(super) fn key_path_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(ask) = app.session.key_path_asked() else {
        return;
    };
    let Some(faraday_bip::policy::KeyPath::Other(key)) = ask.policy.key_path() else {
        return;
    };
    let line = fill(EN.wallet_key_path_sheet_line, &[&ask.name]);
    let key = faraday_ui::grouped(key);
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        (
            Icon::Error.into(),
            Role::Error.paint(),
            EN.wallet_key_path_head,
        ),
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += ui.wrap_as((x, cy, iw), BODY_M, Role::Error, &line) + T::GAP_SMALL;
            cy += ui.wrap_as((x, cy, iw), BODY_M.mono(), Role::OnSurface, &key) + T::GAP;
            crate::vault::screen::hold_button_in(
                app,
                ui,
                x,
                cy,
                iw,
                EN.wallet_key_path_hold,
                Action::KeyPathHold,
                Role::Error.paint(),
            );
            cy += T::CHIP_LINE + T::GAP_SMALL;
            cy - y
        },
        &[(
            EN.wallet_key_path_cancel,
            Style::Secondary,
            Action::KeyPathCancel,
        )],
    );
}
