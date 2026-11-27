//! Buttons in each style (09 §3.2).

use crate::Pressed;
use crate::offered::Offered;
use crate::shown::Shown;
use crate::ui::Ui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Filled,
    Tonal,
    Outlined,
    Text,
    Elevated,
    Error,
}

/// A button, with its style and what a press offers (04 §12.3).
// Spec: 04 §12.3 gives "each with its Style and Offered"; the rest is the porting's.
pub struct ButtonModel<'s, A> {
    pub label: Shown<'s>,
    pub style: Style,
    pub offered: Offered<A>,
}

/// Draws the button.
pub fn draw<A: Pressed>(_m: &ButtonModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
