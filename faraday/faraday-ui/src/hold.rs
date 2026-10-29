//! A hold (`docs/DESIGN.md`'s hold, F136): Material's outlined button in
//! a tone that fills from its start while it is held and acts once held
//! to the end; let go before, nothing happens. Round at its ends (F108),
//! its edge in its tone (error, warning or primary) at 3:1 or more on
//! every surface, the part held filled in that tone's container with the
//! label there in the container's on-role, so the label keeps its
//! contrast on both parts.

use crate::pal::*;
use crate::{Shape, Ui, W};
use faraday_draw::tokens::shape_full;
use faraday_draw::tokens::{BUTTON_H, BUTTON_PAD, DISABLED_CONTAINER, DISABLED_CONTENT, LABEL_L};

/// The tone of a hold: what holding it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldTone {
    /// What cannot be undone (an erase, a delete, a key marked revoked):
    /// error.
    Error,
    /// What wants care (a fee, a plain save): warning.
    Warning,
    /// What goes on (a signature, a wallet added): primary.
    Primary,
}

impl HoldTone {
    /// Its edge and label, its container, and the label on the container.
    pub const fn roles(self) -> (Role, Role, Role) {
        match self {
            HoldTone::Error => (Role::Error, Role::ErrorContainer, Role::OnErrorContainer),
            HoldTone::Warning => (
                Role::Warning,
                Role::WarningContainer,
                Role::OnWarningContainer,
            ),
            HoldTone::Primary => (
                Role::Primary,
                Role::PrimaryContainer,
                Role::OnPrimaryContainer,
            ),
        }
    }
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Costs measuring its label. A hold in `tone` at (x, y), as wide as
    /// `label` and its padding and no wider than `room` (the label cut to
    /// fit), [`BUTTON_H`] tall or as tall as larger text needs, `held`
    /// (0 to 1) of it filled from its start. A press takes `action`; an
    /// action the frame dims draws it faint and takes none. Returns its
    /// width.
    pub fn hold_button(
        &mut self,
        (x, y, room): (f32, f32, f32),
        (label, tone): (&str, HoldTone),
        held: f32,
        action: A,
    ) -> f32 {
        let h = self.button_height(BUTTON_H);
        let r = shape_full(h);
        let w = (self.measure_as(LABEL_L, label) + 2.0 * BUTTON_PAD).min(room);
        let shown = self.fit_as(LABEL_L, label, w - 2.0 * BUTTON_PAD);
        let tw = self.measure(LABEL_L.size, W::S, &shown);
        let lx = x + (w - tw) / 2.0;
        if self.dimmed.contains(&action) {
            // Faint in every theme: it is not available now.
            let (rect, edge) = (
                self.rect(x, y, w, h),
                self.col(Role::OnSurface.at(DISABLED_CONTAINER)),
            );
            self.c
                .stroke_rounded_rect(rect, r * self.f, self.f.max(1.0), edge);
            let ink = Role::OnSurface.at(DISABLED_CONTENT);
            self.text_mid_as((lx, y, h), LABEL_L, ink, &shown);
            return w;
        }
        let (ink, container, on_container) = tone.roles();
        let part = held.clamp(0.0, 1.0);
        let whole = self.rect(x, y, w, h);
        let filled = faraday_draw::Rect::new(
            whole.x,
            whole.y,
            (whole.w as f32 * part).round() as i32,
            whole.h,
        );
        if part > 0.0 {
            let fill = self.col(container);
            self.c.push_clip(filled);
            self.c.fill_rounded_rect(whole, r * self.f, fill);
            self.c.pop_clip();
        }
        self.stroke(x, y, w, h, r, ink);
        self.press(Shape::new(x, y, w, h, r), ink.paint(), action);
        // The label in two parts: on the part held in the container's
        // on-role, on the rest in the tone.
        let rest = faraday_draw::Rect::new(
            filled.right(),
            whole.y,
            whole.right() - filled.right(),
            whole.h,
        );
        for (clip, ink) in [(filled, on_container), (rest, ink)] {
            if clip.w > 0 {
                self.c.push_clip(clip);
                self.text_mid_as((lx, y, h), LABEL_L, ink, &shown);
                self.c.pop_clip();
            }
        }
        w
    }
}
