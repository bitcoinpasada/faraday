//! The frost under a sheet (09 §3.2): the page blurred and dimmed by Material's scrim. A kept
//! copy of it is the runtime's (K16): it is made once, when the sheet opens, and given back to
//! the frames after.

use faraday_port::values::Anchor;
use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::keeps::MadeFrost;
use crate::ui::Ui;
use faraday_tokens::dims::*;

/// The frost under the sheet `sheet` (04 §12.1): drawn from the theme alone.
pub struct FrostModel {
    pub sheet: Anchor,
}

/// Draws the frost over what the canvas holds.
pub fn draw<A: Pressed>(m: &FrostModel, ui: &mut Ui<'_, A>) {
    ui.backdrop(m.sheet);
}

impl<A: Pressed> Ui<'_, A> {
    /// What the sheet `sheet` stands on: the page, frosted and dimmed by Material's scrim. Where
    /// the runtime gave back the copy it kept for this sheet and page, that is put back in one
    /// copy; otherwise it costs a blur of the whole canvas, and the result is handed to the
    /// runtime to keep ([`FrameOut::frosts`](crate::FrameOut)).
    pub fn backdrop(&mut self, sheet: Anchor) {
        let kept = self
            .keeps
            .kept
            .iter()
            .find(|k| k.sheet == sheet && k.page == self.page)
            .map(|k| k.pixels);
        if let Some(pixels) = kept {
            self.backdrop_from(pixels);
            return;
        }
        // Reduced eight units to a pixel and blurred a little: as soft as more blur at a
        // fraction of the work.
        let scale = (8.0 * self.f).round().max(4.0) as u32;
        let dim = self.col(Role::Scrim.with_alpha(SCRIM_ALPHA));
        self.c.frost(scale, 2, dim);
        // A page that shows a secret is never kept (V19): the copy is not even taken.
        if !self.secret && !self.keeps.made.iter().any(|m| m.sheet == sheet) {
            let pixels = self.c.snapshot();
            self.keeps.made.push(MadeFrost {
                sheet,
                page: self.page,
                pixels,
            });
        }
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
