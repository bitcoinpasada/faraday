//! The step-flow template (04 §5, §12.4).

use faraday_atoms::Anchor;
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_molecules::status::BusyModel;
use faraday_organisms::content::Content;
use faraday_organisms::sheet::DialogModel;

/// How wide a step's card is (LAYOUT §14): the kernel's `Width` (03 §9.1), mapped by the
/// presentation kernel, since the kit may not name the kernel (04 §5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Width {
    Column,
    Wide,
    Full,
}

/// One row of Your choices (04 §5).
pub struct ChoiceRow<'s, A> {
    /// The step's title.
    pub title: Shown<'s>,
    /// The step's answer worded by `ShowFlow::choice`, or "Not chosen yet".
    pub answer: Shown<'s>,
    /// A press opens the step.
    pub offered: Offered<A>,
}

/// What the step-flow template draws, filled once for every flow (04 §5).
pub struct FlowModel<'s, A> {
    pub heading: Shown<'s>,
    pub title: Shown<'s>,
    pub card: Content<'s, A>,
    pub choices: Vec<ChoiceRow<'s, A>>,
    pub under: Option<Content<'s, A>>,
    pub back: Offered<A>,
    pub cancel: Option<Offered<A>>,
    pub asking: Option<Box<DialogModel<'s, A>>>,
    pub refused: Option<Shown<'s>>,
    pub busy: Option<BusyModel<'s>>,
    pub width: Width,
    /// The anchor of the fade through.
    pub step: Anchor,
}

/// Draws the step-flow template.
pub fn draw<A: Pressed>(_m: &FlowModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
