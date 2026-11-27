//! The switch control (09 §3.2).

use faraday_tokens::dims::{
    STATE_FOCUS, STATE_HOVER, STATE_PRESS, SWITCH_EDGE, SWITCH_H, SWITCH_HANDLE_OFF,
    SWITCH_HANDLE_ON, SWITCH_HANDLE_PRESSED, SWITCH_STATE, SWITCH_W, alpha_of, shape_full,
};
use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::offered::Offered;
use crate::ui::Ui;

/// The switch: on or off, and what it offers. Not offered, it is drawn and takes no press.
pub struct SwitchModel<'s, A> {
    pub on: bool,
    pub offered: Offered<A>,
    /// The row's label, which the row draws.
    pub label: core::marker::PhantomData<&'s ()>,
}

/// Draws the switch with its top-left at (x, y), design units.
pub fn draw<A: Pressed>(m: &SwitchModel<'_, A>, ui: &mut Ui<'_, A>, (x, y): (f32, f32)) {
    ui.offered(&m.offered, |ui, press, _| ui.switch_with(x, y, m.on, press));
}

impl<A: Pressed> Ui<'_, A> {
    /// A switch (Material's) at (x, y), [`SWITCH_W`] by [`SWITCH_H`]: on,
    /// its track in primary and its handle in on-primary at the right;
    /// off, its track in surface-container-highest edged in outline and a
    /// smaller handle in outline at the left; pressed, the handle larger.
    /// A press takes `action`, in a target at least a touch target tall.
    pub fn switch(&mut self, x: f32, y: f32, on: bool, action: A) {
        self.switch_with(x, y, on, Some(action));
    }

    /// [`Ui::switch`] that takes a press only when `action` is given.
    pub fn switch_with(&mut self, x: f32, y: f32, on: bool, action: Option<A>) {
        let (w, h) = (SWITCH_W, SWITCH_H);
        let r = shape_full(h);
        let pressed = action.is_some() && self.pointer_down_on(x, y, w, h);
        let over = action.is_some() && self.pointer_over(x, y, w, h);
        let lit = over || action.as_ref().is_some_and(|a| self.is_focused(a));
        let (handle, layer) = if on {
            self.fill(x, y, w, h, r, Role::Primary);
            let handle = if pressed {
                Role::PrimaryContainer
            } else {
                Role::OnPrimary
            };
            (handle, Role::Primary)
        } else {
            self.fill(x, y, w, h, r, Role::SurfaceContainerHighest);
            let edge = self.rect(x, y, w, h);
            let color = self.col(Role::Outline);
            self.c
                .stroke_rounded_rect(edge, r * self.f, SWITCH_EDGE * self.f, color);
            // Material's switch: the off handle in outline, and in
            // on-surface-variant while hovered, focused or pressed.
            let handle = if pressed || lit {
                Role::OnSurfaceVariant
            } else {
                Role::Outline
            };
            (handle, Role::OnSurface)
        };
        let d = if pressed {
            SWITCH_HANDLE_PRESSED
        } else if on {
            SWITCH_HANDLE_ON
        } else {
            SWITCH_HANDLE_OFF
        };
        // The handle's centre: a track's height from the end it rests at.
        let cx = if on { x + w - h / 2.0 } else { x + h / 2.0 };
        let cy = y + h / 2.0;
        if let Some(action) = action {
            // Material's switch lays its state layer on the handle; the
            // ring is round the track.
            self.press_custom((x, y, w, h), action, |ui, a| {
                let focused = ui.is_focused(a);
                let over = ui.over.unwrap_or(false);
                let opacity = if over && ui.down {
                    Some(STATE_PRESS)
                } else if focused {
                    Some(STATE_FOCUS)
                } else if over {
                    Some(STATE_HOVER)
                } else {
                    None
                };
                if let Some(o) = opacity {
                    let s = SWITCH_STATE / 2.0;
                    let wash = layer.with_alpha(alpha_of(o));
                    ui.wash(cx - s, cy - s, 2.0 * s, 2.0 * s, s, wash);
                }
                ui.focus_ring((x, y, w, h, r), a);
            });
        }
        self.dot(cx, cy, d / 2.0, handle);
    }
}
