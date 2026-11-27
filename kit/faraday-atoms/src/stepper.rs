//! The stepper (09 §3.2).

use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::shown::Shown;
use crate::ui::Ui;
use faraday_draw::icons::Icon;
use faraday_tokens::dims::*;

/// A stepper: a label, a count between two icon buttons, and what the two presses are.
pub struct StepperModel<'s, A> {
    pub label: Shown<'s>,
    pub value: usize,
    pub minus: A,
    pub plus: A,
    /// The tips of the minus and the plus button.
    pub tips: (&'s str, &'s str),
    /// Not enabled, both buttons are faint and take no press.
    pub enabled: bool,
}

/// Draws the stepper's row at (x, y) in a width `w`, design units.
pub fn draw<A: Pressed>(m: &StepperModel<'_, A>, ui: &mut Ui<'_, A>, at: (f32, f32, f32)) {
    ui.showing(m.label, |ui| {
        ui.stepper(
            at,
            m.label.as_str(),
            m.value,
            (m.minus.clone(), m.plus.clone()),
            m.tips,
            m.enabled,
        );
    });
}

impl<A: Pressed> Ui<'_, A> {
    /// A stepper (`docs/DESIGN.md` §4.2): `label`, then its count between
    /// two outlined icon buttons, − pressing `minus` and + pressing
    /// `plus`, in a row [`ICON_BUTTON`] tall at (x, y) in a width `w`. On
    /// a small panel the buttons stand at the row's right end; on the
    /// wide layout [`STEPPER_LABEL_COL`] in from the label's start, or
    /// past a longer label, so they never cover it. Not `enabled`, both
    /// are faint and take no press. The bounds are the presses' own.
    pub fn stepper(
        &mut self,
        (x, y, w): (f32, f32, f32),
        label: &str,
        value: usize,
        (minus, plus): (A, A),
        tips: (&str, &str),
        enabled: bool,
    ) {
        let role = STEPPER_LABEL_TEXT;
        let span = 2.0 * ICON_BUTTON + STEPPER_COUNT_W;
        let bx = if self.compact {
            x + w - span
        } else {
            x + (self.measure_as(role, label) + GAP).max(STEPPER_LABEL_COL)
        };
        let h = ICON_BUTTON;
        // A one-line slot up to the buttons: larger text fits down to
        // Standard there (`docs/TEXT-SIZE.md` §3).
        let room = (bx - GAP - x).max(0.0);
        self.text_fit_as((x, y, h, room), role, Role::OnSurfaceVariant, label);
        let less = (bx, y);
        self.outlined_icon_button(less, Icon::Remove, tips.0, enabled, minus);
        let v = value.to_string();
        let vw = self.measure_as(STEPPER_VALUE_TEXT, &v);
        let vx = bx + h + (STEPPER_COUNT_W - vw) / 2.0;
        self.text_mid_as((vx, y, h), STEPPER_VALUE_TEXT, Role::OnSurface, &v);
        let more = (bx + h + STEPPER_COUNT_W, y);
        self.outlined_icon_button(more, Icon::Add, tips.1, enabled, plus);
    }
}
