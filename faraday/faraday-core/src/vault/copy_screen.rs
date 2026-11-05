//! **Copy to another vault** (`docs/VAULT.md` §7.2): the other open vaults
//! by name, a dimmed one with its reason, and the dialog that says an
//! entry's attachments and other data stay where they are. Drawing reads
//! what the press worked out ([`super::copy::CopyTo`]).

use crate::Action;
use crate::compact::{buttons, dialog, scroll_sheet, sheet_head};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::VaultAction as V;
use faraday_draw::tokens::{
    BODY_M, BODY_S, CHART_SHEET_WIDTH, COPY_HEAD_BLOCK, COPY_HEAD_H, COPY_HEAD_TEXT_X,
    COPY_SHEET_INSET, COPY_SHEET_PAD, COPY_SHEET_SIDES, COPY_SHEET_TOP, COPY_SHEET_WIDE_PAD,
    DIALOG_MAX, DISABLED_CONTENT, GAP, GAP_SMALL, ICON_GLYPH_BUTTON, PAD, SHAPE_S, TITLE_S, TOUCH,
};
use faraday_draw::widgets::Icon;
use faraday_vault::records::field;

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// The sheet, or the dialog once a row with unread data is pressed.
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(s) = app.vault_copy_sheet() else {
        return;
    };
    if let Some(name) = s.asked() {
        ask(ui, (w, h), s, name);
        return;
    }
    let title = if s.mv {
        EN.vault_move_title
    } else {
        EN.vault_copy_title
    };
    let place = if ui.compact {
        (
            (COPY_SHEET_INSET, w - 2.0 * COPY_SHEET_INSET),
            COPY_SHEET_INSET,
            COPY_SHEET_PAD,
        )
    } else {
        let sw = (w - COPY_SHEET_SIDES).min(CHART_SHEET_WIDTH);
        (((w - sw) / 2.0, sw), COPY_SHEET_TOP, COPY_SHEET_WIDE_PAD)
    };
    scroll_sheet(ui, place.0, h, place.1, place.2, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += if ui.compact {
            sheet_head(ui, x, cy, iw, Icon::Copy, Role::Primary, title)
        } else {
            ui.icon(
                x,
                cy,
                COPY_HEAD_H,
                Icon::Copy,
                ICON_GLYPH_BUTTON,
                Role::Primary,
            );
            let t = ui.fit_as(
                faraday_draw::tokens::TITLE_L.medium(),
                title,
                iw - COPY_HEAD_TEXT_X,
            );
            ui.text_mid_as(
                (x + COPY_HEAD_TEXT_X, cy, COPY_HEAD_H),
                faraday_draw::tokens::TITLE_L.medium(),
                Role::OnSurface,
                &t,
            );
            COPY_HEAD_BLOCK
        };
        cy += ticks(ui, (x, cy, iw), &s.keeps, &|k, _| va(V::CopyKeep(k)));
        for (n, r) in s.rows.iter().enumerate() {
            let press = r.why.is_none().then(|| va(V::CopyPick(n)));
            cy += row(
                ui,
                (x, cy, iw),
                (&r.name, r.why.as_deref().unwrap_or("")),
                press,
            );
        }
        cy += buttons(
            ui,
            x,
            cy,
            iw,
            &[(EN.vault_copy_cancel, Style::Secondary, va(V::CopyCancel))],
        );
        cy - y
    });
}

/// **Keep its codex32 set** and **Keep its BIP-85 origin**, one tick for
/// each of `keeps` (a field's number and whether it is kept), pressing
/// `press(place, number)`. Returns the height with the gap under each.
pub(crate) fn ticks(
    ui: &mut Ui,
    (x, y, iw): (f32, f32, f32),
    keeps: &[(u8, bool)],
    press: &dyn Fn(usize, u8) -> Action,
) -> f32 {
    let mut cy = y;
    for (k, &(number, kept)) in keeps.iter().enumerate() {
        let label = if number == field::KEY_CODEX32 {
            EN.vault_copy_keep_codex32
        } else {
            EN.vault_copy_keep_child
        };
        ui.checkbox(x + PAD, cy + (TOUCH - ICON_GLYPH_BUTTON) / 2.0, kept, true);
        ui.text_mid_as(
            (x + PAD + ICON_GLYPH_BUTTON + GAP, cy, TOUCH),
            TITLE_S,
            Role::OnSurface,
            label,
        );
        ui.press_box(x, cy, iw, TOUCH, press(k, number));
        cy += TOUCH + GAP_SMALL;
    }
    cy - y
}

/// **Save into {vault}**: the choices of a seed about to be saved, with
/// **Save** and **Cancel**.
pub(crate) fn draw_keep(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(s) = super::keep::pending(app) else {
        return;
    };
    let title = super::keep::title(app);
    let actions = [
        (EN.vault_copy_cancel, Style::Secondary, va(V::KeepCancel)),
        (EN.vault_save, Style::Primary, va(V::KeepSave)),
    ];
    // A heading names a seed only when the sheet lists more than one.
    let heads = s.asks.len() > 1;
    dialog(
        ui,
        (w, h, DIALOG_MAX),
        (Icon::Lock.into(), Role::Primary.paint(), title.as_str()),
        &mut |ui, x, y, iw| {
            let mut cy = y;
            for a in &s.asks {
                if heads {
                    let t = ui.fit_as(TITLE_S, &a.label, iw);
                    ui.text_mid_as((x, cy, TOUCH), TITLE_S, Role::OnSurfaceVariant, &t);
                    cy += TOUCH;
                }
                let out: Vec<u8> = (s.left.iter())
                    .filter(|u| u.0 == a.fp)
                    .map(|u| u.1)
                    .collect();
                let keeps = super::keep::ticks_of(&a.fields, &out);
                cy += ticks(ui, (x, cy, iw), &keeps, &|_, n| va(V::KeepTick(a.fp, n)));
            }
            cy - y
        },
        &actions,
    );
}

/// The dialog a move always puts up, and a copy that leaves data behind.
fn ask(ui: &mut Ui, (w, h): (f32, f32), s: &super::copy::CopyTo, name: &str) {
    let head = (
        Icon::Copy.into(),
        Role::Primary.paint(),
        if s.mv {
            EN.vault_move_head
        } else {
            EN.vault_copy_unread_head
        },
    );
    let line = if s.mv {
        let mut line = fill(EN.vault_move_line, &[name, &s.from_name, &s.from_name]);
        for &(number, _) in s.keeps.iter().filter(|k| !k.1) {
            let said = if number == field::KEY_CODEX32 {
                EN.vault_move_no_codex32
            } else {
                EN.vault_move_no_child
            };
            line = format!("{line} {}", fill(said, &[&s.from_name]));
        }
        if s.unread {
            line = format!("{line} {}", fill(EN.vault_move_unread, &[&s.from_name]));
        }
        line
    } else {
        fill(EN.vault_copy_unread, &[name])
    };
    let go = if s.mv {
        EN.vault_move_go
    } else {
        EN.vault_copy_go
    };
    let actions = [
        (EN.vault_copy_cancel, Style::Secondary, va(V::CopyCancel)),
        (go, Style::Primary, va(V::CopyGo)),
    ];
    dialog(
        ui,
        (w, h, DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| ui.wrap_as((x, y, iw), BODY_M, Role::OnSurface, &line) + GAP,
        &actions,
    );
}

/// A vault's row: its name, and under it the reason it is dimmed. A dimmed
/// row takes no press. Returns the height with the gap under it.
fn row(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (name, why): (&str, &str),
    press: Option<Action>,
) -> f32 {
    let (left, inner) = (x + PAD, w - 2.0 * PAD);
    // Measured in a pass that draws nothing, so the box goes down before
    // the words and does not cover them.
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let name_h = ui.wrap_as((left, y, inner), TITLE_S, Role::OnSurface, name);
    let why_h = if why.is_empty() {
        0.0
    } else {
        ui.wrap_as((left, y, inner), BODY_S, Role::OnSurface, why)
    };
    ui.c.pop_clip();
    let h = (name_h + why_h + 2.0 * GAP).max(TOUCH);
    ui.fill(x, y, w, h, SHAPE_S, Role::SurfaceContainerHighest);
    ui.stroke(x, y, w, h, SHAPE_S, Role::OutlineVariant);
    if let Some(press) = press {
        ui.press_box(x, y, w, h, press);
    }
    let top = y + (h - name_h - why_h) / 2.0;
    let (ink, muted) = if press.is_some() {
        (Role::OnSurface.paint(), Role::OnSurfaceVariant.paint())
    } else {
        let dim = Role::OnSurface.at(DISABLED_CONTENT);
        (dim, dim)
    };
    ui.wrap_as((left, top, inner), TITLE_S, ink, name);
    if why_h > 0.0 {
        ui.wrap_as((left, top + name_h, inner), BODY_S, muted, why);
    }
    h + GAP_SMALL
}
