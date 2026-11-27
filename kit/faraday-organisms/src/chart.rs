//! The chart organism (09 §3.3).

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Ui};

// Spec: 09 §3.2 names this model without a shape.
pub struct ChartModel<'s, A>(PhantomData<(&'s (), A)>);

// Spec: 09 §3.2 names this model without a shape.
pub struct WalletCardModel<'s, A>(PhantomData<(&'s (), A)>);

/// Draws the model.
pub fn draw<A: Pressed>(_m: &ChartModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
