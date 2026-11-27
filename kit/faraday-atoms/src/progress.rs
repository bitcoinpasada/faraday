//! The progress control (09 §3.2).

use core::marker::PhantomData;

use crate::Pressed;
use crate::ui::Ui;

/// The model. Its fields are those of `main`'s component, ported (09 §3.4).
pub struct ProgressModel<'s>(PhantomData<(&'s (),)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &ProgressModel<'_>, _ui: &mut Ui<'_, A>) {
    todo!()
}
