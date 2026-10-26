//! The vault item page's masked secrets, each behind its eye
//! (`docs/DESIGN.md` §4.10): a press shows one, the next hides it, and
//! it hides again once its page is left ([`crate::vault::Vaults::shown`]).

use super::*;

/// A secret value at `(x, y)`, `w` wide, masked until its eye shows it;
/// `eye` its eye's press and whether it shows. Returns the height.
pub(super) fn secret(
    ui: &mut Ui,
    at: (f32, f32, f32),
    value: &str,
    mono: bool,
    eye: (Action, bool),
) -> f32 {
    secret_with(ui, at, || value.to_string().into(), mono, eye)
}

/// The eye at the right end of a secret field whose top is `y`: open
/// while the secret is masked, struck while it shows (the icon shows
/// what a press does), its press `action`.
fn secret_eye(ui: &mut Ui, x: f32, y: f32, w: f32, (action, shown): (Action, bool)) {
    let (muted, primary, inset) = (
        Role::OnSurfaceVariant,
        Role::Primary,
        Role::SurfaceContainerHighest,
    );
    let ex = x + w - tokens::TOUCH;
    let pressed = ui.is_pressed(action);
    let edge = (tokens::TOUCH - 32.0) / 2.0;
    crate::ui::fill_pressed(ui, (ex + edge, y + edge, 32.0, 32.0, 6.0), inset, pressed);
    ui.icon(
        ex,
        y,
        tokens::TOUCH,
        if shown { Icon::EyeOff } else { Icon::Eye },
        15.0,
        if shown { primary } else { muted },
    );
    crate::compact::eye_hit(ui, (ex, y, tokens::TOUCH, tokens::TOUCH), action, shown);
}

/// [`secret`] with the value made only while it shows.
pub(super) fn secret_with(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    value: impl FnOnce() -> zeroize::Zeroizing<String>,
    mono: bool,
    eye: (Action, bool),
) -> f32 {
    let (muted, inset) = (Role::OnSurfaceVariant, Role::SurfaceContainerHighest);
    let (ink, warn) = (Role::OnSurface, Role::Warning);
    let shown = eye.1;
    let face = if mono { W::M } else { W::R };
    let text = if shown {
        crate::secret_text::SecretText::of(&value())
    } else {
        crate::secret_text::SecretText::of(&"\u{2022}".repeat(24))
    };
    // The text stops short of the eye.
    let tw = w - 14.0 - tokens::TOUCH;
    // Measured in a pass that draws nothing, so the field's surface goes
    // down before the words and does not cover them.
    let th = if shown {
        ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
        let th = ui.wrap(x + 14.0, y + 14.0, tw, 14.0, face, ink, &text);
        ui.c.pop_clip();
        th
    } else {
        20.0
    };
    let bh = (th.max(20.0) + 28.0).max(tokens::TOUCH);
    ui.fill(x, y, w, bh, 8.0, inset);
    let edge = if shown {
        warn.paint()
    } else {
        Role::Warning.with_alpha(110)
    };
    ui.stroke(x, y, w, bh, 8.0, edge);
    if shown {
        ui.wrap(x + 14.0, y + 14.0, tw, 14.0, face, ink, &text);
    } else {
        ui.text(x + 14.0, y + 14.0, 14.0, W::M, muted, &text);
    }
    secret_eye(ui, x, y, w, eye);
    bh
}

/// Show SeedQR: the selected key's SeedQR, behind its eye as its words
/// are. The code is the one the eye's press made
/// ([`crate::vault::VaultArea::toggle_seedqr`]); drawing makes none.
/// Returns the height.
pub(super) fn seedqr_box(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let (muted, inset) = (Role::OnSurfaceVariant, Role::SurfaceContainerHighest);
    let warn = Role::Warning;
    let code = app.vaults.seedqr_shown(app.vault_on_page());
    let shown = code.is_some();
    // The code's side, as a code is shown anywhere in the app, or the
    // field's width less the eye where that is less.
    let class = if ui.compact {
        faraday_draw::SizeClass::Small
    } else {
        faraday_draw::SizeClass::Wide
    };
    let side = faraday_draw::tokens::qr_side(class).min(w - 14.0 - tokens::TOUCH);
    let bh = if shown { side + 28.0 } else { tokens::TOUCH };
    ui.fill(x, y, w, bh, 8.0, inset);
    let edge = if shown {
        warn.paint()
    } else {
        Role::Warning.with_alpha(110)
    };
    ui.stroke(x, y, w, bh, 8.0, edge);
    match code {
        Some(m) => crate::screens::qr_code(ui, m, x + 14.0, y + 14.0, side),
        None => {
            ui.text_mid(
                x + 14.0,
                y,
                tokens::TOUCH,
                14.0,
                W::S,
                muted,
                EN.vault_show_seedqr,
            );
        }
    }
    secret_eye(ui, x, y, w, (va(V::ShowSeedQr), shown));
    bh
}
