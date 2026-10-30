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

/// The row before an entry's TOTP secret (`docs/TOTP.md` §6): the
/// code, live, or Set the time while the clock is not checked; HOTP's or
/// Unreadable's badge where it gives none. Returns the height used and
/// whether the secret reads as TOTP, which gives it Show as QR
/// (`docs/QR.md` §7.1).
pub(super) fn entry_code_row(
    app: &View,
    ui: &mut Ui,
    r: RecordView<'_>,
    dx: f32,
    y: f32,
) -> (f32, bool) {
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    let mut dy = y;
    let code = app.entry_code(r, app.clock_now().filter(|_| app.clock_checked()));
    let reads = code.is_ok();
    ui.text_as(dx, dy, BODY_S, muted, EN.vault_code);
    dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
    match code {
        Ok(Some(code)) => {
            let (grouped, left) = (code.grouped, code.left);
            let gw = ui.measure_as(BODY_L.mono(), &grouped);
            ui.text_as(dx, dy, BODY_L.mono(), ink, &grouped);
            ui.text_as(
                dx + gw + 12.0,
                dy + 4.0,
                BODY_S,
                muted,
                &format!("{left} s"),
            );
            dy += ui.advance_as(BODY_S, 34.0);
        }
        Ok(None) => {
            ui.button(
                dx,
                dy,
                None,
                BUTTON_H,
                EN.vault_set_the_time,
                Style::Secondary,
                Action::OpenSetTime,
            );
            dy += 44.0;
        }
        Err(faraday_keys::totp::NotTotp::Hotp) => {
            code_badge(ui, dx, dy, "HOTP");
            dy += 36.0;
        }
        Err(faraday_keys::totp::NotTotp::Unreadable) => {
            code_badge(ui, dx, dy, EN.vault_unreadable);
            dy += 36.0;
        }
    }
    (dy + 10.0 - y, reads)
}

/// **Show as QR** under an entry's masked TOTP secret: its `otpauth://`
/// URI on the QR sheet marked Secret (`docs/QR.md` §7.1). Returns the
/// height used. It stands close under the box it belongs to, in the gap
/// a secret row leaves after itself.
pub(super) fn totp_qr_button(ui: &mut Ui, dx: f32, dy: f32) -> f32 {
    let label = EN.vault_show_as_qr;
    let a = va(V::TotpQr);
    let y = dy - tokens::GAP;
    ui.button(dx, y, None, BUTTON_H, label, Style::Secondary, a);
    BUTTON_H + tokens::PAD - tokens::GAP
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
