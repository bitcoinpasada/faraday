//! The fade of what a refused press covers (04 §12.1, V11).

use faraday_draw::Rect;

use crate::Pressed;
use crate::ui::Ui;

impl<A: Pressed> Ui<'_, A> {
    /// Notes the box at (x, y, w, h) of a control or row refused while a job runs, to be drawn as
    /// Material's disabled content by [`Ui::fade_disabled`] once the page or sheet is drawn:
    /// what is drawn there, rows, links, chips and cards, goes toward what it stands on, and its
    /// container stays. The background is read just inside the left edge, clear of an outline,
    /// level with the middle of the box.
    pub fn fade_later(&mut self, (x, y, w, h): (f32, f32, f32, f32)) {
        let whole = self.rect(x, y, w, h);
        let rect = whole.intersect(&self.seen_clip());
        if rect.is_empty() {
            return;
        }
        let inset = (2.0 * self.f).ceil() as i32 + 2;
        let at = (whole.x + inset, whole.y + whole.h / 2);
        self.faded.push((rect, at));
    }

    /// Costs a pass over the pixels of the boxes noted by [`Ui::fade_later`], at the disabled
    /// content opacity. Called once the page, then the sheet, is drawn.
    pub fn fade_disabled(&mut self) {
        for (rect, at) in std::mem::take(&mut self.faded) {
            self.c
                .fade_to_background(rect, at, faraday_tokens::dims::DISABLED_CONTENT);
        }
    }
}

/// The boxes noted and where each reads its background.
pub(crate) type Faded = Vec<(Rect, (i32, i32))>;

#[cfg(test)]
mod tests {
    use crate::ui::testing::*;

    /// Invariant: A row refused while a job runs is drawn faded toward the page it stands on, and the page beside it does not change.
    /// Regression: The fade pass reads no box, or reads the row's own fill as the ground, so a refused row stays at full strength or the page beside it dims.
    /// Owner: kit
    #[test]
    fn a_refused_row_fades_toward_its_ground_and_the_page_does_not() {
        let mut c = canvas(160);
        let mut plain = canvas(160);
        let draw = |ui: &mut crate::ui::Ui<'_, P>, fade: bool| {
            ui.clear(page());
            ui.fill(20.0, 20.0, 80.0, 40.0, 0.0, primary_edge());
            if fade {
                ui.fade_later((10.0, 20.0, 100.0, 40.0));
                ui.fade_disabled();
            }
        };
        frame(&mut c, &at_rest(None), |ui| draw(ui, true));
        frame(&mut plain, &at_rest(None), |ui| draw(ui, false));
        assert_ne!(pixel(&c, 60, 40), pixel(&plain, 60, 40));
        assert_eq!(pixel(&c, 130, 40), pixel(&plain, 130, 40));
        assert_eq!(pixel(&c, 12, 40), pixel(&plain, 12, 40));
    }
}
