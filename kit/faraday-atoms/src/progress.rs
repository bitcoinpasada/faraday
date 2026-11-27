//! Progress (09 §3.2).

use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::ui::Ui;
use faraday_tokens::dims::*;

/// A linear progress indicator: `done` of the track filled, or a bar moving along it.
pub struct ProgressModel<'s> {
    /// 0 to 1, or none for work with no count.
    pub done: Option<f32>,
    /// How far through its pass the moving bar is; none with reduce motion.
    pub phase: Option<f32>,
    /// The indicator carries no text on `main`; the lifetime is the spec's (09 §3.4).
    pub text: core::marker::PhantomData<&'s ()>,
}

/// Draws the indicator `w` wide at (x, y), design units.
pub fn draw<A: Pressed>(m: &ProgressModel<'_>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) {
    ui.progress((x, y, w), m.done, m.phase);
}

impl<A: Pressed> Ui<'_, A> {
    /// A linear progress indicator `w` wide at (x, y), [`PROGRESS_H`]
    /// tall: `done`, 0 to 1, of the track filled; or, for work with no
    /// count (`done` `None`), a bar moving along it, `phase` of the way
    /// through its pass, or with reduce motion (`phase` `None`) the
    /// whole track in primary at half strength, still.
    pub fn progress(&mut self, (x, y, w): (f32, f32, f32), done: Option<f32>, phase: Option<f32>) {
        let h = PROGRESS_H;
        let r = shape_full(h);
        let track = self.rect(x, y, w, h);
        let shown = done.map(|d| d.clamp(0.0, 1.0));
        self.c
            .note(faraday_draw::canvas::InkKind::Progress(shown), track);
        self.fill(x, y, w, h, r, Role::SurfaceContainerHighest);
        match (done, phase) {
            (Some(d), _) => {
                let dw = w * d.clamp(0.0, 1.0);
                if dw > 0.0 {
                    self.fill(x, y, dw, h, r, Role::Primary);
                }
            }
            (None, Some(p)) => {
                // The bar enters at the left, crosses and leaves at the
                // right, clipped to the track.
                let span = w * PROGRESS_SPAN;
                let at = x - span + (w + span) * p.clamp(0.0, 1.0);
                let track = self.rect(x, y, w, h);
                self.c.push_clip(track);
                self.fill(at, y, span, h, r, Role::Primary);
                self.c.pop_clip();
            }
            (None, None) => {
                self.wash(x, y, w, h, r, Role::Primary.at(0.5));
            }
        }
    }
}
