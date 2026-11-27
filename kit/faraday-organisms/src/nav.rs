//! The navigation and the top app bar (09 §3.3).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

/// The destinations, the Scan button, the power button, the app's name and the lock tag: one
/// model, drawn as the rail, the bar or the sidebar as the screen template places it (04 V10).
/// Its fields are those of `main`'s component (09 §3.4).
pub struct NavModel<'s, A>(PhantomData<(&'s (), A)>);

/// The title, the way back and the pills. Its fields are those of `main`'s component (09 §3.4).
pub struct TopBarModel<'s, A>(PhantomData<(&'s (), A)>);

/// Draws the navigation as the rail.
pub fn draw<A: Pressed>(_m: &NavModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}

/// Draws the navigation as the bar.
pub fn draw_bar<A: Pressed>(_m: &NavModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}

/// Draws the top app bar.
pub fn draw_top<A: Pressed>(_m: &TopBarModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
