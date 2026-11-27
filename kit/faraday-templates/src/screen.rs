//! The screen template: the navigation, the top app bar, the page, the sheets over it and the
//! snackbar (04 §4, §12.4).

use faraday_atoms::Anchor;
use faraday_atoms::{Pressed, Ui};
use faraday_molecules::status::BusyModel;
use faraday_organisms::nav::{NavModel, TopBarModel};
use faraday_organisms::snackbar::SnackbarModel;

/// The screen template's model: the navigation, the top app bar with its pills, the busy line.
/// The template chooses the rail, the bar or the sidebar by the room (04 V10).
pub struct FrameModel<'s, R> {
    pub nav: NavModel<'s, R>,
    pub top: TopBarModel<'s, R>,
    pub busy: Option<BusyModel<'s>>,
}

/// Draws the frame: the page's anchor, the page under the sheets, the sheets over it (each with
/// its anchor, the lowest first) and the note on show.
pub fn draw<'s, R: Pressed, O>(
    _page: Anchor,
    _frame: &FrameModel<'s, R>,
    _ui: &mut Ui<'_, R>,
    _body: impl FnOnce(&mut Ui<'_, R>),
    _over: &[(Anchor, O)],
    _draw_over: impl Fn(&O, &mut Ui<'_, R>),
    _note: Option<&SnackbarModel<'s>>,
) {
    todo!()
}
