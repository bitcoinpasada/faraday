//! Drawing a passphrase field (`docs/DESIGN.md` §4.3, "Text field"): dots
//! for what is typed, the characters while the eye is open, the **No
//! passphrase** pill while it is empty, the caret while it is focused, and
//! in Practice the `faraday` chip and its code box. Drawing reads: no
//! state is written and nothing is worked out.

use super::{PassField, Which};
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{CoreUi, Ui, W};
use faraday_draw::tokens;
use faraday_draw::tokens::{BODY_M, BODY_S};
use faraday_text::Text;

/// How a field is drawn.
pub(crate) struct Look {
    /// It has focus: the caret, and the outline in the primary colour.
    pub on: bool,
    /// The press that gives it focus.
    pub press: Action,
    /// Its place in Practice's chips.
    pub chip: crate::demo::PassField,
    /// What an empty field without focus says, in place of the pill; none
    /// leaves it blank.
    pub hint: Option<&'static str>,
}

/// One field of `f`, `h` tall and `w` wide at (x, y): the outlined box with
/// its text and the eye at the right end, the one eye of
/// `typing.pass_eye`.
pub(crate) fn draw(
    app: &View,
    ui: &mut Ui,
    f: &PassField,
    which: Which,
    (x, y, w, h): (f32, f32, f32, f32),
    look: &Look,
) {
    let text = match which {
        Which::First => f.as_str(),
        Which::Again => f.again_str(),
    };
    if app.pass_is_public(text) {
        crate::screens::parts::code_field(ui, x, y, w, h, text);
        ui.press_box(x, y, w, h, look.press);
        return;
    }
    let shown = app.typing.pass_eye.shown;
    ui.fill(
        x,
        y,
        w,
        h,
        tokens::RADIUS_SMALL,
        Role::SurfaceContainerHighest,
    );
    ui.stroke(
        x,
        y,
        w,
        h,
        tokens::RADIUS_SMALL,
        if look.on {
            Role::Primary
        } else {
            Role::OutlineVariant
        },
    );
    ui.press_box(x, y, w - h, h, look.press);
    let inset = tokens::TYPED_INSET;
    let room = w - inset - h;
    match look.hint {
        Some(hint) if text.is_empty() && !look.on => {
            let line = ui.fit_as(BODY_M.mono(), hint, room);
            ui.text_mid_as((x + inset, y, h), BODY_M.mono(), Role::OnSurface, &line);
        }
        _ => {
            // A dot for each character, so the caret and a press fall
            // between the characters they stand for (`docs/TEXT-CURSOR.md`
            // §3).
            let held: Text = if shown {
                Text::of(text)
            } else {
                Text::repeated("•", text.chars().count())
            };
            ui.typed(
                x + inset,
                y,
                h,
                BODY_M.size,
                W::M,
                Role::OnSurface,
                &held,
                room,
                look.on,
                look.press,
            );
        }
    }
    let marked = app.typing.pass_eye.waits();
    crate::vault::screen::eye_at_marked(ui, (x, y, w), h, shown, marked, Action::PassEye);
    app.pass_chip_draw(ui, text.is_empty(), look.chip);
}

/// A masked passphrase field with its label at the left (above it on a
/// small panel): dots for what is typed, the **No passphrase** pill
/// while it is empty, the caret while it is focused. In Practice `chip` is the press of the
/// `faraday` chip the empty field offers, and `faraday` is drawn in its
/// code box. Returns the height used.
#[allow(
    clippy::too_many_arguments,
    reason = "a passphrase field's position, width, label, text, reveal chip and action"
)]
pub(crate) fn pass_field(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    text: &str,
    (on, chip, shown, marked): (bool, Option<Action>, bool, bool),
    action: Action,
) -> f32 {
    let (px, py, pw) = if ui.compact {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, label);
        (x, y + 22.0, w)
    } else {
        ui.text_mid_as((x, y, 40.0), BODY_S, Role::OnSurfaceVariant, label);
        (x + 150.0, y, 380.0f32.min(w - 150.0))
    };
    let r = tokens::FIELD_RADIUS;
    ui.fill(px, py, pw, 40.0, r, Role::SurfaceContainerHighest);
    ui.stroke(
        px,
        py,
        pw,
        40.0,
        r,
        if on { Role::Primary } else { Role::Outline },
    );
    dots_field(ui, px, py, pw, text, (on, chip, shown, marked), action);
    py - y + 48.0
}

/// What a BIP-39 passphrase field 40 tall and `w` wide at (x, y) shows:
/// a dot for each character typed (the characters while `shown`, the eye
/// open), with the selection and the caret
/// while `on`, or the **No passphrase** pill while it is empty.
/// In Practice (`chip`) the empty field
/// offers the `faraday` chip, and `faraday` is drawn in its code box.
pub(crate) fn dots_field(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    text: &str,
    (on, chip, shown, marked): (bool, Option<Action>, bool, bool),
    action: Action,
) {
    ui.press_box(x, y, w, tokens::TEXT_FIELD, action);
    if text.is_empty() {
        ui.no_passphrase(x, y, 40.0, on);
        // Use faraday goes in the "Demo only" card (`docs/DEMO.md` §5.4).
        if let Some(a) = chip {
            ui.demo_offer(EN.demo_use_faraday, a);
        }
    } else if chip.is_some() && text == crate::demo::PASSPHRASE {
        super::super::screens::parts::code_field(ui, x, y, w, tokens::TEXT_FIELD, text);
    } else {
        let dots = Text::repeated("•", text.chars().count());
        let held = Text::of(text);
        ui.typed(
            x + 12.0,
            y,
            40.0,
            BODY_M.size,
            W::M,
            Role::OnSurface,
            if shown { &held } else { &dots },
            w - 24.0 - tokens::TEXT_FIELD,
            on,
            action,
        );
        crate::vault::screen::eye_at_marked(
            ui,
            (x, y, w),
            tokens::TEXT_FIELD,
            shown,
            marked,
            Action::PassEye,
        );
    }
}

/// A passphrase field with an eye at its right end, as
/// [`secret_box`](crate::vault::screen::secret_box) draws it: in Practice what
/// holds `faraday` is in its code box, and an empty field hands the "Practice
/// only" card its **Use faraday** (`docs/DEMO.md` §5.4). `(shown,
/// focused)`, `(action, eye)`.
pub(crate) fn pass_box(
    app: &crate::drawing::View,
    ui: &mut crate::ui::Ui,
    (x, y, w): (f32, f32, f32),
    b: &crate::vault::TextBox,
    (shown_eye, focused): (bool, bool),
    (action, eye): (Action, Action),
    field: crate::demo::PassField,
) {
    let show = app.pass_shows(shown_eye, &b.text);
    // While a vault job runs the field takes no typing.
    if app.vaults.working.is_some() {
        crate::vault::screen::secret_box_off(ui, (x, y, w), b, show);
    } else {
        crate::vault::screen::secret_box(ui, x, y, w, b, show, focused, action, eye);
    }
    app.pass_chip_draw(ui, b.text.is_empty(), field);
}

/// A tool's passphrase in a box at (x, y), `w` wide, with the one eye of
/// `typing.pass_eye` at its right end: Practice's `faraday` in its code
/// box, the dots, or the characters while the eye is open. `focused` is
/// the caret, `action` the press that focuses it.
pub(crate) fn tool_box(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    text: &str,
    (focused, action): (bool, Action),
) {
    use crate::demo::Show;
    let show = app.pass_shows(app.typing.pass_eye.shown, text);
    if show == Show::Code {
        crate::screens::code_field(ui, x, y, w, tokens::TEXT_FIELD, text);
        ui.press_box(x, y, w, tokens::TEXT_FIELD, action);
        return;
    }
    let shown = show == Show::Shown;
    crate::vault::screen::text_box_of(ui, (x, y, w), text, !shown, focused, action);
    crate::vault::screen::eye_at_marked(
        ui,
        (x, y, w),
        tokens::FIELD_H,
        shown,
        false,
        Action::PassEye,
    );
}
