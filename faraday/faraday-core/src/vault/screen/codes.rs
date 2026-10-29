//! The entry form on the codes screen and the entry detail's badges.

use super::*;

/// A small text pill (`docs/DESIGN.md` §4.8): the entry detail's HOTP
/// and Unreadable badges in place of a code.
pub(super) fn code_badge(ui: &mut Ui, x: f32, y: f32, label: &str) {
    let muted = Role::OnSurfaceVariant;
    let tw = ui.measure_as(LABEL_M, label);
    let pw = tw + 24.0;
    ui.fill(x, y, pw, 26.0, 13.0, Role::SurfaceContainerHighest);
    ui.stroke(x, y, pw, 26.0, 13.0, Role::OutlineVariant);
    ui.text_mid_as((x + (pw - tw) / 2.0, y, 26.0), LABEL_M, muted, label);
}

/// The entry form on the codes screen (`docs/TOTP.md` §5.3): Add an
/// entry, or the form a scanned setup code filled in. Always a new
/// entry — editing one happens on Vault contents, where its row leads.
/// Returns the height it took.
pub(crate) fn codes_form(app: &View, ui: &mut Ui, dx: f32, dy: f32, dw: f32) -> f32 {
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    let top = dy;
    let mut dy = dy;
    let Some(form) = &app.vaults.form else {
        return 0.0;
    };
    ui.text_as(dx, dy, TITLE_M, ink, EN.vault_add_an_entry);
    dy += ui.line_as(TITLE_M) + faraday_draw::tokens::PAD;
    for (k, (_, label, b, masked)) in form.fields.iter().enumerate() {
        ui.text_as(dx, dy, BODY_S, muted, label);
        dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
        let focused = app.vaults.focus == Some(Focus::Field(k));
        if vault::multiline(form.kind, form.fields[k].0) {
            dy += text_area(ui, dx, dy, dw, b, focused, va(V::FocusField(k))) + 12.0;
        } else if *masked {
            secret_box(
                ui,
                dx,
                dy,
                dw,
                b,
                app.field_shows(form, k),
                focused,
                va(V::FocusField(k)),
                va(V::ShowTyped),
            );
            app.password_chip(ui, form, k, b);
            dy += FIELD_H + 12.0;
        } else {
            text_box(ui, dx, dy, dw, b, false, focused, va(V::FocusField(k)));
            dy += FIELD_H + 12.0;
        }
    }
    dy += 8.0;
    ui.button(
        dx,
        dy,
        Some(120.0),
        BUTTON_H,
        EN.vault_save,
        Style::Primary,
        va(V::FormSave),
    );
    ui.button(
        dx + 130.0,
        dy,
        Some(120.0),
        BUTTON_H,
        EN.vault_cancel,
        Style::Ghost,
        va(V::FormCancel),
    );
    dy += 56.0;
    dy - top
}
