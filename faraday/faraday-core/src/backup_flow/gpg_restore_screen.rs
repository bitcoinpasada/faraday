//! The cards and the sheet of a GPG key restored from its paperkey
//! (`docs/GPG-PAPERKEY.md` §4): the sheet of the public keys in Files, the
//! flow's head card with the public key and line 1 of its paperkey, and
//! step 4's lines, each checked as it is ended, typed on the docked hex
//! keyboard. Drawing only: it reads the certificate kept on the flow and
//! the lines' state kept in the flow area (`gpg_restore.rs`), and works
//! nothing out.

use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, BUTTON_H, FIELD_EDGE_ACTIVE, FIELD_GAP, FIELD_H, FIELD_PAD,
    FIELD_RADIUS, GAP, GAP_SMALL, LIST_2, LIST_PAD, PAD,
};
use faraday_draw::widgets::Icon;

use super::gpg_restore::{HEAD, RestoreFrom, RestoreLive};
use super::steps::{FlowAction as F, RestorePress as R};
use super::{BackupFlow, Step};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Paint, Style, Ui};
use crate::{Action, flow};

fn ra(p: R) -> Action {
    Action::Flow(F::Restore(p))
}

/// A label in Body S and its value under it in Body M, wrapped, in the
/// mono face for data. Returns the height with the gap under it.
fn fact(ui: &mut Ui, (x, y, w): (f32, f32, f32), label: &str, value: &str, mono: bool) -> f32 {
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, label);
    let top = ui.line_as(BODY_S) + GAP_SMALL;
    let role = if mono { BODY_M.mono() } else { BODY_M };
    let h = ui.wrap_as((x, y + top, w), role, Role::OnSurface, value);
    top + h + GAP
}

/// A line of text in Body M wrapped in `color`. Returns the height with
/// the gap under it.
fn line(ui: &mut Ui, (x, y, w): (f32, f32, f32), text: &str, color: Paint) -> f32 {
    ui.wrap_as((x, y, w), BODY_M, color, text) + GAP
}

/// A button 40 dp tall at (x, y). Returns the height with the gap under it.
fn button(ui: &mut Ui, (x, y): (f32, f32), label: &str, style: Style, action: Action) -> f32 {
    ui.button(x, y, None, BUTTON_H, label, style, action);
    BUTTON_H + GAP
}

/// The flow's heading for a restore, and its chip.
pub(crate) fn heading(f: &BackupFlow) -> Option<(&'static str, &str)> {
    let r = f.restore.as_ref()?;
    Some((EN.gpg_restore_title, r.chip.as_str()))
}

/// The head card: the public key the flow restores against, closed to a
/// line once read (§4.2, step 0).
pub(crate) fn head_card(f: &BackupFlow, r: &RestoreFrom) -> flow::Card {
    flow::Card {
        title: EN.gpg_restore_public_key.to_string(),
        summary: format!("{} · {}", r.user_id, r.fingerprint_text),
        mono: false,
        done: true,
        open: f.live.open == Some(HEAD),
        default: false,
        toggle: ra(R::Head),
        guide: None,
        learn: None,
    }
}

/// The head card's body: the user ID, the fingerprints, the dates and a
/// revocation by the key, then the bytes line 1 of its paperkey must
/// read, and Continue while the plan is not made.
pub(crate) fn head_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(r) = f.restore.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    cy += fact(ui, (x, cy, w), EN.vault_user_id, &r.user_id, false);
    cy += fact(
        ui,
        (x, cy, w),
        EN.vault_fingerprint,
        &r.fingerprint_text,
        true,
    );
    cy += fact(
        ui,
        (x, cy, w),
        EN.vault_signing_subkey,
        &r.subkey_text,
        true,
    );
    let date = |t: u64| crate::gpg::date(crate::vault::local_secs(t, app.tz_min()));
    let made = date(u64::from(r.created));
    let valid = if r.expiry == 0 {
        fill(EN.vault_gpg_no_expiry, &[&made])
    } else {
        let ends = date(u64::from(r.created) + u64::from(r.expiry));
        fill(EN.vault_gpg_validity, &[&made, &ends])
    };
    cy += fact(ui, (x, cy, w), EN.vault_valid, &valid, false);
    if let Some(t) = r.revoked {
        let revoked = fill(EN.vault_revoked_date, &[&date(u64::from(t))]);
        cy += line(ui, (x, cy, w), &revoked, Role::Error.paint());
    }
    ui.text_as(
        x,
        cy,
        BODY_S,
        Role::OnSurfaceVariant,
        EN.gpg_restore_line_one,
    );
    cy += ui.line_as(BODY_S) + GAP_SMALL;
    cy += code_lines(ui, (x, cy, w), &r.line_one);
    if f.step == Step::Where && !f.made {
        let go = ra(R::HeadContinue);
        cy += button(
            ui,
            (x, cy),
            EN.backup_flow_continue_make,
            Style::Primary,
            go,
        );
    }
    cy - y
}

/// Bytes in hex pairs in code boxes: one line where it fits in `w`, else
/// eleven bytes a line, as a paperkey line wraps (§4.5). Returns the
/// height.
fn code_lines(ui: &mut Ui, (x, y, w): (f32, f32, f32), text: &str) -> f32 {
    let size = BODY_M.size;
    let step = ui.code_height(size) + GAP_SMALL;
    if ui.measure_as(BODY_M.mono(), text) + 2.0 * GAP <= w {
        ui.code_box(x, y, size, text);
        return step + GAP;
    }
    let pairs: Vec<&str> = text.split(' ').collect();
    let mut cy = y;
    for chunk in pairs.chunks(super::gpg_restore::WRAP_BYTES) {
        ui.code_box(x, cy, size, &chunk.join(" "));
        cy += step;
    }
    cy - y + GAP
}

/// Step 4 of a restore: the lines and **Restore the key**; once
/// restored, the Result.
pub(crate) fn make_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if f.made {
        return result(app, f, ui, x, y, w);
    }
    let mut cy = y;
    if !app.may_load_keys() {
        let pull = fill(EN.backup_flow_pull_to_unlock, &[app.medium.noun()]);
        cy += crate::vault::screen::stick_banner(ui, x, cy, w, &pull);
    }
    let Some(live) = app.flows.on().and_then(|i| app.flows.restoring(i)) else {
        return cy - y;
    };
    cy += checked(live, ui, (x, cy, w));
    if !live.complete() {
        cy += typing(live, ui, (x, cy, w));
        return cy - y;
    }
    cy += button(
        ui,
        (x, cy),
        EN.gpg_restore_button,
        Style::Primary,
        ra(R::Restore),
    );
    if let Some(e) = live.refused() {
        cy += line(ui, (x, cy, w), e, Role::Error.paint());
    }
    if let Some((o, name)) = live.in_vault() {
        let open = Action::Vault(crate::vault::VaultAction::Show(*o));
        cy += button(ui, (x, cy), name, Style::Secondary, open);
    }
    cy - y
}

/// The lines checked: "Line {n} · checked" each, line 1 with the
/// fingerprint it names, and **Type line {n} again** for the last of them.
fn checked(live: &RestoreLive, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let n = live.checked();
    for k in 1..=n {
        let text = fill(EN.gpg_restore_checked, &[&k.to_string()]);
        cy += line(ui, (x, cy, w), &text, Role::Ok.paint());
        if k == 1
            && let Some(fp) = live.fingerprint_text()
        {
            let at = (x, cy - GAP, w);
            cy += ui.wrap_as(at, BODY_S.mono(), Role::OnSurfaceVariant, fp);
        }
    }
    if n > 0 {
        let again = fill(EN.gpg_restore_again, &[&n.to_string()]);
        cy += button(ui, (x, cy), &again, Style::Ghost, ra(R::Again(n as u32)));
    }
    cy - y
}

/// The line being typed: its number and the count once known, the field
/// with the digits as typed and the caret, its error, and **Next line**.
/// The hex keyboard is docked at the foot, as the passphrase keyboard is.
fn typing(live: &RestoreLive, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let n = live.next().to_string();
    let label = match live.lines() {
        Some(total) => fill(EN.gpg_restore_line_of, &[&n, &total.to_string()]),
        None => fill(EN.gpg_restore_line, &[&n]),
    };
    cy += ui.field_label(x, cy, &label);
    // As on paper: the digits wrap, in the mono face.
    let caret = if live.focused { ui.caret_char() } else { "" };
    let shown = format!("{}{caret}", live.typed());
    let role = BODY_L.mono();
    let room = w - 2.0 * FIELD_PAD;
    let text_h = ui.wrap_height_as(role, &shown, room);
    let one = ui.line_as(role);
    if text_h <= one {
        let used = ui.field_box((x, cy, w), live.focused, live.error());
        ui.text_mid_as((x + FIELD_PAD, cy, FIELD_H), role, Role::OnSurface, &shown);
        ui.press_box(x, cy, w, FIELD_H, ra(R::Field));
        cy += used + GAP;
    } else {
        // A box that grows with the lines, its edge as the field's.
        let box_h = text_h + FIELD_H - one;
        let edge = if live.error().is_some() {
            Role::Error
        } else {
            Role::Primary
        };
        let inset = FIELD_EDGE_ACTIVE / 2.0;
        let (bx, bw, bh) = (x + inset, w - FIELD_EDGE_ACTIVE, box_h - FIELD_EDGE_ACTIVE);
        ui.stroke(bx, cy + inset, bw, bh, FIELD_RADIUS, edge);
        let top = cy + (FIELD_H - one) / 2.0;
        ui.wrap_as((x + FIELD_PAD, top, room), role, Role::OnSurface, &shown);
        ui.press_box(x, cy, w, box_h, ra(R::Field));
        cy += box_h;
        if let Some(e) = live.error() {
            cy += FIELD_GAP;
            cy += ui.wrap_as((x + FIELD_PAD, cy, room), BODY_S, Role::Error, e);
        }
        cy += GAP;
    }
    cy += button(ui, (x, cy), EN.gpg_restore_next, Style::Tonal, ra(R::Next));
    // Room under it for the next button's touch target.
    cy += GAP;
    cy - y
}

/// The Result: the key's facts as Make's Result shows them, "Restored
/// into {vault}" for each vault, and the signatures by other keys not
/// kept.
fn result(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    cy += super::screen::result_rows(app, f, ui, x, cy, w);
    for &v in &f.made_in {
        let into = fill(EN.gpg_restore_into, &[&BackupFlow::vault_name(v)]);
        cy += line(ui, (x, cy, w), &into, Role::Ok.paint());
    }
    let aside = f.restore.as_ref().map_or(0, |r| r.set_aside);
    if aside > 0 {
        let text = fill(EN.gpg_restore_set_aside, &[&aside.to_string()]);
        cy += line(ui, (x, cy, w), &text, Role::OnSurfaceVariant.paint());
    }
    let go = Action::Flow(F::MadeContinue);
    cy += button(
        ui,
        (x, cy),
        EN.backup_flow_continue_make,
        Style::Primary,
        go,
    );
    cy - y
}

/// **Restore a GPG key**: a row for each public key in Files that reads,
/// its user ID and fingerprint, those that do not under them, dimmed,
/// with why, Cancel and **Scan the public key** (§4.1).
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let place = crate::modes::screen::place(ui, w);
    crate::compact::scroll_sheet(ui, place.0, h, place.1, place.2, &mut |ui, x, y, iw| {
        sheet_body(app, ui, x, y, iw)
    });
}

fn sheet_body(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    cy += crate::modes::screen::head(ui, (x, cy, w), Icon::Keys, EN.gpg_restore_title);
    let rows = app.flows.restore_rows();
    let none = rows.iter().all(|r| r.refused.is_some());
    if none {
        cy += line(
            ui,
            (x, cy, w),
            EN.gpg_restore_none,
            Role::OnSurfaceVariant.paint(),
        );
    }
    for r in rows {
        match &r.refused {
            None => {
                let it = faraday_ui::roles::ListItem {
                    headline: &r.user_id,
                    supporting: &r.fingerprint,
                    trailing: None,
                    action: ra(R::From(r.index)),
                };
                cy += ui.list_item((x, cy, w), &it, false);
            }
            // Not restored here: dimmed, taking no press, with why.
            Some(why) => {
                let inner = w - 2.0 * LIST_PAD;
                let head = ui.line_as(BODY_L);
                let why_h = ui.wrap_height_as(BODY_M, why, inner);
                let h = (head + why_h + 2.0 * GAP).max(LIST_2);
                let top = cy + (h - head - why_h) / 2.0;
                let name = ui.fit_as(BODY_L, &r.user_id, inner);
                ui.text_as(x + LIST_PAD, top, BODY_L, crate::ui::off(), &name);
                ui.wrap_as((x + LIST_PAD, top + head, inner), BODY_M, Role::Error, why);
                cy += h;
            }
        }
        ui.rule(x, cy, w, Role::OutlineVariant);
    }
    cy += PAD;
    cy += crate::compact::buttons(
        ui,
        x,
        cy,
        w,
        &[
            (EN.file_menu_cancel, Style::Ghost, Action::Cancel),
            // The way on where no row is; beside the rows, a second one.
            (
                EN.gpg_restore_scan,
                if none { Style::Primary } else { Style::Tonal },
                ra(R::Scan),
            ),
        ],
    );
    cy - y
}
