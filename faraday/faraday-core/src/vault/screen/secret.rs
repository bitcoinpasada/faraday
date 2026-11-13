//! The vault item page's masked secrets, each behind its eye
//! (`docs/DESIGN.md` §4.10): a press shows one, the next hides it, and
//! it hides again once its page is left ([`crate::vault::Vaults::shown`]).

use super::*;
use faraday_draw::tokens::{BODY_M, LABEL_L};

/// A secret value at `(x, y)`, `w` wide, masked until its eye shows it;
/// `eye` its eye's press and whether it shows. Returns the height.
pub(crate) fn secret(
    ui: &mut Ui,
    at: (f32, f32, f32),
    value: &str,
    mono: bool,
    eye: (Action, bool),
) -> f32 {
    secret_with(ui, at, || value, mono, eye)
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
    crate::ui::fill_pressed(
        ui,
        (ex + edge, y + edge, 32.0, 32.0, tokens::shape_full(32.0)),
        inset,
        pressed,
    );
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

/// [`secret`] with the value taken (made or borrowed) only while it shows.
pub(crate) fn secret_with<V: AsRef<str>>(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    value: impl FnOnce() -> V,
    mono: bool,
    eye: (Action, bool),
) -> f32 {
    let (muted, inset) = (Role::OnSurfaceVariant, Role::SurfaceContainerHighest);
    let (ink, warn) = (Role::OnSurface, Role::Warning);
    let shown = eye.1;
    let role = if mono { BODY_M.mono() } else { BODY_M };
    let text = if shown {
        crate::secret_text::SecretText::of(value().as_ref())
    } else {
        crate::secret_text::SecretText::of(&"\u{2022}".repeat(24))
    };
    // The text stops short of the eye.
    let tw = w - 14.0 - tokens::TOUCH;
    // Measured in a pass that draws nothing, so the field's surface goes
    // down before the words and does not cover them.
    let th = if shown {
        ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
        let th = ui.wrap_as((x + 14.0, y + 14.0, tw), role, ink, &text);
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
        Role::Outline.paint()
    };
    ui.stroke(x, y, w, bh, 8.0, edge);
    if shown {
        ui.wrap_as((x + 14.0, y + 14.0, tw), role, ink, &text);
    } else {
        ui.text_as(x + 14.0, y + 14.0, BODY_M.mono(), muted, &text);
    }
    secret_eye(ui, x, y, w, eye);
    bh
}

/// Show SeedQR: the selected key's SeedQR, behind its eye as its words
/// are. The code is the one the eye's press made
/// ([`crate::vault::VaultArea::toggle_seedqr`]); drawing makes none.
/// Returns the height.
pub(super) fn seedqr_box(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let code = app.vaults.seedqr_shown(app.vault_secrets_on_page());
    let shown = code.is_some();
    code_with(
        ui,
        (x, y, w),
        code,
        EN.vault_show_seedqr,
        (va(V::ShowSeedQr), shown),
    )
}

/// A code behind an eye: `code` while it shows, else `label`. Drawing only
/// reads it. Returns the height.
pub(crate) fn code_with(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    code: Option<&faraday_codec::qr::QrMatrix>,
    label: &str,
    eye: (Action, bool),
) -> f32 {
    let (muted, inset) = (Role::OnSurfaceVariant, Role::SurfaceContainerHighest);
    let warn = Role::Warning;
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
        Role::Outline.paint()
    };
    ui.stroke(x, y, w, bh, 8.0, edge);
    match code {
        Some(m) => crate::screens::qr_code(ui, m, x + 14.0, y + 14.0, side),
        None => {
            ui.text_mid_as((x + 14.0, y, tokens::TOUCH), LABEL_L, muted, label);
        }
    }
    secret_eye(ui, x, y, w, eye);
    bh
}

/// A small text file's text on the signing page: masked until its eye
/// shows it. Its lines are kept with the text only while it shows.
/// Returns the height.
pub(super) fn sign_text(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    text: &str,
    eye: (Action, bool),
) -> f32 {
    let (muted, inset) = (Role::OnSurfaceVariant, Role::SurfaceContainerHighest);
    let (ink, warn) = (Role::OnSurface, Role::Warning);
    let shown = eye.1;
    // The text stops short of the eye.
    let tw = w - 14.0 - tokens::TOUCH;
    let role = BODY_S.mono();
    let laid = shown.then(|| {
        let basis = ui.wrap_basis(role, tw);
        let lines = crate::gpg::SignWrap::lines_of(&app.vaults.sign_wraps, text, basis, || {
            ui.wrap_ranges(role, text, tw)
        });
        ui.wrapped_with(role, text, tw, lines)
    });
    let th = laid.as_ref().map_or(20.0, |l| l.h);
    let bh = (th.max(20.0) + 28.0).max(tokens::TOUCH);
    ui.fill(x, y, w, bh, 8.0, inset);
    let edge = if shown {
        warn.paint()
    } else {
        Role::Outline.paint()
    };
    ui.stroke(x, y, w, bh, 8.0, edge);
    match &laid {
        Some(l) => ui.draw_wrapped((x + 14.0, y + 14.0), ink, l),
        None => {
            let dots = "\u{2022}".repeat(24);
            ui.text_as(x + 14.0, y + 14.0, BODY_M.mono(), muted, &dots);
        }
    }
    secret_eye(ui, x, y, w, eye);
    bh
}
