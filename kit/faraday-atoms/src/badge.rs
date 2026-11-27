//! The badge control (09 §3.2).

use core::marker::PhantomData;

use crate::Pressed;
use crate::ui::Ui;

/// The model. Its fields are those of `main`'s component, ported (09 §3.4).
pub struct BadgeModel<'s>(PhantomData<(&'s (),)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &BadgeModel<'_>, _ui: &mut Ui<'_, A>) {
    todo!()
}
