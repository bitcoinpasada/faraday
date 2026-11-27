//! Shadows (09 §3.2).

use crate::Pressed;
use crate::ui::Ui;

/// The shadow of something that floats (04 §12.1): drawn from the theme alone.
pub struct ShadowModel;

/// Draws the shadow of something at (x, y, w, h) with corners of `r`, at Material's elevation
/// `level` (1 to 5), before it.
pub fn draw<A: Pressed>(
    _m: &ShadowModel,
    ui: &mut Ui<'_, A>,
    (x, y, w, h, r): (f32, f32, f32, f32, f32),
    level: u8,
) {
    ui.shadow(x, y, w, h, r, level);
}

impl<A: Pressed> Ui<'_, A> {
    /// A soft shadow under something that floats at (x, y), w by h with
    /// corners of `r`, at Material 3's elevation `level` (1 to 5): drawn
    /// before it, black in every theme. High contrast draws
    /// none: a shadow there would lower the contrast of the text and
    /// edges beside it, and what floats has its outline.
    pub fn shadow(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, level: u8) {
        if self.theme.is_high_contrast() {
            return;
        }
        self.shadow_faded((x, y, w, h, r), level, 1.0);
    }

    /// [`Ui::shadow`] under something `shown` (0 to 1) of the way faded
    /// in, at that share of its strength, at (x, y), w by h with corners
    /// of `r`.
    pub fn shadow_faded(
        &mut self,
        (x, y, w, h, r): (f32, f32, f32, f32, f32),
        level: u8,
        shown: f32,
    ) {
        if self.theme.is_high_contrast() || shown <= 0.0 {
            return;
        }
        let rect = self.rect(x, y, w, h);
        self.c
            .elevation_faded(rect, r * self.f, level, self.f, shown);
    }
}
