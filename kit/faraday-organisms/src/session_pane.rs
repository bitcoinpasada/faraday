//! The session pane organism (09 §3.3).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

// Spec: 09 §3.2 names this model without a shape.
pub struct SessionPaneModel<'s, A>(PhantomData<(&'s (), A)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &SessionPaneModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
