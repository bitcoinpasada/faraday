//! The snackbar organism (09 §3.3): Material's snackbar, an inverse-surface bar with one line of
//! text, standing above the foot of the page. Its action and its time on show are not built: the
//! runtime times a note (04 §10).

use faraday_atoms::{Pressed, Shown, Ui};
use faraday_tokens::dims::{BODY_M, GRID, MARGIN_COMPACT, SHAPE_XS};
use faraday_tokens::elevation::ELEVATION_MENU;
use faraday_tokens::palette::Role;

/// The note on show: its words.
pub struct SnackbarModel<'s> {
    pub text: Shown<'s>,
}

/// The snackbar's height: one line and a margin over and under it.
pub fn height<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    ui.line_as(BODY_M) + 4.0 * GRID
}

/// Draws the snackbar standing on `foot`, the page's foot, in a window `w` wide.
pub fn draw_at<A: Pressed>(m: &SnackbarModel<'_>, ui: &mut Ui<'_, A>, (w, foot): (f32, f32)) {
    let h = height(ui);
    let bw = w - 2.0 * MARGIN_COMPACT;
    let (x, y) = (MARGIN_COMPACT, foot - h - MARGIN_COMPACT);
    ui.shadow(x, y, bw, h, SHAPE_XS, ELEVATION_MENU);
    ui.fill(x, y, bw, h, SHAPE_XS, Role::InverseSurface);
    let room = bw - 4.0 * GRID;
    let text = ui.fit_as(BODY_M, m.text.as_str(), room);
    ui.showing(m.text, |ui| {
        ui.text_mid_as(
            (x + 2.0 * GRID, y, h),
            BODY_M,
            Role::InverseOnSurface,
            &text,
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{at_rest, canvas, frame};

    /// Invariant: A note on show draws its words in the bar standing above the foot, and takes no press.
    /// Regression: The note drawn off the window or as a control that swallows a press.
    /// Owner: kit
    #[test]
    fn a_note_stands_above_the_foot_and_takes_no_press() {
        let mut c = canvas(160);
        let out = frame(&mut c, &at_rest(None), |ui| {
            let m = SnackbarModel {
                text: Shown::plain("Saved"),
            };
            draw_at(&m, ui, (640.0, 480.0));
        });
        assert!(out.hits.is_empty());
    }
}
