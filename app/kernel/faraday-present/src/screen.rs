//! The root's model for one frame (04 §4).

use faraday_organisms::snackbar::SnackbarModel;
use faraday_organisms::{Pressed, Ui};
use faraday_port::values::Anchor;
use faraday_templates::screen::FrameModel;

/// The root's model for one frame.
pub struct ScreenModel<'s, B, O, R: Pressed> {
    /// Which page: the motion store's scope and the page turn's.
    pub page: Anchor,
    /// The screen template's model: navigation, top app bar, pills, busy line.
    pub frame: FrameModel<'s, R>,
    /// The page under the sheets: one variant per page the root holds.
    pub body: B,
    /// Sheets and dialogs over it, each with its anchor, the lowest first; only the top one takes
    /// presses.
    pub over: Vec<(Anchor, O)>,
    /// The note on show (04 §10).
    pub note: Option<SnackbarModel<'s>>,
}

/// A body or sheet enum draws the variant it holds, each page's presses lifted into the root's
/// `R` (`Ui::within`).
pub trait Drawn<R: Pressed> {
    fn draw(&self, ui: &mut Ui<'_, R>);
}

/// What a root's `frame` calls once it has the model.
pub fn draw<B: Drawn<R>, O: Drawn<R>, R: Pressed>(
    _m: &ScreenModel<'_, B, O, R>,
    _ui: &mut Ui<'_, R>,
) {
    todo!()
}
