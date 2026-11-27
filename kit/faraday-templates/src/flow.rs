//! The step-flow template (04 §5, §12.4).

use faraday_atoms::Anchor;
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_molecules::status::BusyModel;
use faraday_organisms::content::Content;
use faraday_organisms::sheet::DialogModel;

/// How wide a step's card is (LAYOUT §14).
// Spec: 04 §5 and 09 §3.3 name a `width` field without its type; the kernel's `Width` is mapped
// to this by the presentation kernel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Width {
    Column,
    Wide,
    Full,
}

/// One row of Your choices.
// Spec: 04 §5 names `choices` without a row type.
pub struct ChoiceRow<'s, A> {
    pub title: Shown<'s>,
    pub answer: Shown<'s>,
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
    pub asking: Option<DialogModel<'s, A>>,
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
