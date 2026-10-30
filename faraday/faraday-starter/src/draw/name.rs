//! What to write's Name field and Confirm's row for it
//! (`docs/STICK-IDENTITY.md` §6): Material 3's outlined text field
//! (m3.material.io, Text fields), its label above it, the caret where
//! the person puts it (`docs/TEXT-CURSOR.md`), "No name" as its
//! placeholder while it is empty, and a supporting line that says how FAT
//! writes the name, or why it cannot.

use faraday_draw::tokens::{BODY_L, FIELD_PAD, GAP};
use faraday_strings::fill;
use faraday_ui::{Role, W};

use super::{S, Ui, fact_row};
use crate::decide::Target;
use crate::name::checked;
use crate::{Starter, StarterAction};

/// What to write's Name field, on a stick or card written whole. Returns
/// its height.
pub(super) fn section(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(field) = starter.name_field() else {
        return 0.0;
    };
    let mut y = y0 + 2.0 * GAP;
    y += ui.field_label(x, y, S.starter_name_label);
    let result = checked(&field.typed);
    let focused = field.focused;
    let action = StarterAction::Name.host();
    let fh = ui.field_h();
    let boxed = ui.field_box((x, y, w), focused, result.as_ref().err().copied());
    if field.typed.is_empty() {
        let at = (x + FIELD_PAD, y, fh);
        ui.text_mid_as(at, BODY_L, Role::OnSurfaceVariant, S.starter_name_none);
    }
    ui.typed(
        x + FIELD_PAD,
        y,
        fh,
        BODY_L.size,
        W::from(BODY_L.face),
        Role::OnSurface,
        &field.typed,
        w - 2.0 * FIELD_PAD,
        focused,
        action,
    );
    ui.press_box(x, y, w, fh, action);
    y += boxed;
    // How FAT writes it, where that is not as typed.
    if let Ok(to) = &result
        && !to.is_empty()
        && to != field.typed.trim_matches(' ')
    {
        let line = fill(S.starter_name_written_as, &[to]);
        let at = (x + FIELD_PAD, y + GAP / 2.0, w - 2.0 * FIELD_PAD);
        y += GAP / 2.0 + ui.field_supporting(at, Role::OnSurfaceVariant, &line, focused);
    }
    y - y0
}

/// Confirm's row for the name, when one is written. Returns its height.
pub(super) fn confirm_row(starter: &Starter, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    match starter.name_order() {
        Some(name) => fact_row(ui, x, y, w, S.starter_name_label, &name),
        None => 0.0,
    }
}

/// Done's line: what the check after writing compared, and what was
/// expected to differ (`docs/STICK-IDENTITY.md` §4.3).
pub(super) fn checked_line(starter: &Starter, target: Target) -> &'static str {
    let named = starter.name_order().is_some();
    let signed = starter.boot_order().is_some();
    match (target, named, signed) {
        (Target::Pi, false, _) => S.starter_checked_card,
        (Target::Pi, true, _) => S.starter_checked_card_named,
        (Target::Stick, false, false) => S.starter_checked,
        (Target::Stick, false, true) => S.starter_checked_signed,
        (Target::Stick, true, false) => S.starter_checked_named,
        (Target::Stick, true, true) => S.starter_checked_named_signed,
    }
}
