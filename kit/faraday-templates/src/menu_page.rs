//! The menu page template (09 §3.3).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

/// The model. Its fields are those of `main`'s component, ported (09 §3.4).
pub struct MenuPageModel<'s, A>(PhantomData<(&'s (), A)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &MenuPageModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
