//! The frost under a sheet (09 §3.2): the page blurred and dimmed by Material's scrim. A kept
//! copy of it is the runtime's (K16): `backdrop_from` puts one back.

use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::ui::Ui;
use faraday_tokens::dims::*;

/// The frost under a sheet (04 §12.1): drawn from the theme alone.
pub struct FrostModel;

/// Draws the frost over what the canvas holds.
pub fn draw<A: Pressed>(_m: &FrostModel, ui: &mut Ui<'_, A>) {
    ui.backdrop();
}

impl<A: Pressed> Ui<'_, A> {
    /// What a sheet stands on: the page, frosted and dimmed by Material's scrim. Costs a blur
    /// of the whole canvas: the runtime keeps the result and gives it back to
    /// [`Ui::backdrop_from`] on the frames after.
    pub fn backdrop(&mut self) {
        // Reduced eight units to a pixel and blurred a little: as soft as more blur at a
        // fraction of the work.
        let scale = (8.0 * self.f).round().max(4.0) as u32;
        let dim = self.col(Role::Scrim.with_alpha(SCRIM_ALPHA));
        self.c.frost(scale, 2, dim);
    }

    /// The frosted page as the runtime kept it, put back in one copy.
    pub fn backdrop_from(&mut self, kept: &[u8]) {
        self.c.restore(kept);
    }

    /// A copy of the canvas as it is, for the runtime to keep under a sheet.
    pub fn snapshot(&mut self) -> Vec<u8> {
        self.c.snapshot()
    }
}
