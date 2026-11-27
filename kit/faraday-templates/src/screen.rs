//! The screen template: the navigation, the top app bar, the page, the sheets over it and the
//! snackbar (04 §4, §12.4).

use core::marker::PhantomData;

use faraday_atoms::Anchor;
use faraday_atoms::{Pressed, Ui};
use faraday_organisms::snackbar::SnackbarModel;

/// The screen template's model: navigation, top app bar, pills, busy line.
// Spec: 09 §3.3 names `FrameModel<'s, R>` without a shape.
pub struct FrameModel<'s, R>(PhantomData<(&'s (), R)>);

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
