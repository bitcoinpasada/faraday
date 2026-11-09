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
use faraday_draw::tokens::{
    BUTTON_H, BUTTON_PAD, BUTTON_PAD_V, DISABLED_CONTAINER, DISABLED_CONTENT, LABEL_L,
};

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
    /// Costs measuring its label. The width and height of a hold with
    /// `label` in `room` and the lines its label is broken onto: none
    /// where the label fits on one line, as wide as it and its padding.
    /// Where it does not fit, the hold is `room` wide and as tall as its
    /// lines need.
    fn hold_layout(&self, room: f32, label: &str) -> (f32, f32, Vec<(usize, usize)>) {
        let h = self.button_height(BUTTON_H);
        let wide = self.measure_as(LABEL_L, label) + 2.0 * BUTTON_PAD;
        if wide <= room {
            return (wide, h, Vec::new());
        }
        let lines = self
            .wrap_ranges(LABEL_L, label, room - 2.0 * BUTTON_PAD)
            .to_vec();
        let tall = lines.len() as f32 * self.line_as(LABEL_L) + 2.0 * BUTTON_PAD_V;
        (room, h.max(tall), lines)
    }

    /// Costs measuring its label. How tall [`Ui::hold_button`] draws a
    /// hold with `label` in `room`: [`BUTTON_H`] or as tall as larger text
    /// or a label wrapped onto more lines needs. Drawn nowhere.
    pub fn hold_button_height(&self, room: f32, label: &str) -> f32 {
        self.hold_layout(room, label).1
    }

    /// Costs measuring its label. A hold in `tone` at (x, y), as wide as
    /// `label` and its padding and no wider than `room`, [`BUTTON_H`] tall
    /// or as tall as larger text needs, `held` (0 to 1) of it filled from
    /// its start. A label that does not fit `room` on one line is broken
    /// at word breaks onto as many centred lines as it needs and the hold
    /// grows to hold them ([`Ui::hold_button_height`]). A press takes
    /// `action`; an action the frame dims draws it faint and takes none.
    /// Returns its width.
    pub fn hold_button(
        &mut self,
        (x, y, room): (f32, f32, f32),
        (label, tone): (&str, HoldTone),
        held: f32,
        action: A,
    ) -> f32 {
        let (w, h, lines) = self.hold_layout(room, label);
        let r = shape_full(h);
        let shown: Vec<&str> = if lines.is_empty() {
            vec![label]
        } else {
            lines.iter().map(|&(a, b)| label[a..b].trim_end()).collect()
        };
        let line = self.line_as(LABEL_L);
        let top = y + (h - shown.len() as f32 * line) / 2.0;
        let widths: Vec<f32> = shown
            .iter()
            .map(|l| self.measure(LABEL_L.size, W::S, l))
            .collect();
        if self.is_dimmed(&action) {
            // Faint in every theme: it is not available now.
            let (rect, edge) = (
                self.rect(x, y, w, h),
                self.col(Role::OnSurface.at(DISABLED_CONTAINER)),
            );
            self.c
                .stroke_rounded_rect(rect, r * self.f, self.f.max(1.0), edge);
            let ink = Role::OnSurface.at(DISABLED_CONTENT);
            self.hold_label((x, w, top, line), &shown, &widths, ink);
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
                self.hold_label((x, w, top, line), &shown, &widths, ink);
                self.c.pop_clip();
            }
        }
        w
    }

    /// The lines of a hold's label, each centred in the hold at `x`,
    /// `w` wide, from `top`, `line` apart.
    fn hold_label(
        &mut self,
        (x, w, top, line): (f32, f32, f32, f32),
        shown: &[&str],
        widths: &[f32],
        ink: impl Into<crate::Paint> + Copy,
    ) {
        for (i, (l, tw)) in shown.iter().zip(widths).enumerate() {
            let lx = x + (w - tw) / 2.0;
            self.text_mid_as((lx, top + i as f32 * line, line), LABEL_L, ink, l);
        }
    }
}
