//! The two stages of a screen (04 §2).

use faraday_kernel::{Feature, Show, View};
use faraday_organisms::{Pressed, Ui};

/// The first stage. The model may name the kit's model types (`Offered`, `Content`, a template's
/// model), never a drawing function.
pub trait Present: Feature {
    type Model<'s>
    where
        Self: 's;

    fn present<'s>(state: &'s Self::State, view: &View<'s>, show: &Show<'s>) -> Self::Model<'s>;
}

/// The second stage. `Press` is what a press on this page sends (04 §2.1); `Msg` is `From<Press>`,
/// checked where `Page` is implemented.
pub trait Page: Present {
    type Press: Pressed;

    fn draw(model: &Self::Model<'_>, ui: &mut Ui<'_, Self::Press>);
}
