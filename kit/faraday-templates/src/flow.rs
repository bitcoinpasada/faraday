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

/// What the step-flow template draws, filled once for every flow (04 §5, rule V12). Its fields
/// are private and it is built only by `new` and the `with_*` methods, which
/// `disallowed-methods` lets only the presentation kernel's fill call.
#[expect(dead_code, reason = "the template's draw reads them when it is built")]
pub struct FlowModel<'s, A> {
    heading: Shown<'s>,
    title: Shown<'s>,
    card: Content<'s, A>,
    choices: Vec<ChoiceRow<'s, A>>,
    under: Option<Content<'s, A>>,
    back: Offered<A>,
    cancel: Option<Offered<A>>,
    asking: Option<Box<DialogModel<'s, A>>>,
    refused: Option<Shown<'s>>,
    busy: Option<BusyModel<'s>>,
    width: Width,
    /// The anchor of the fade through.
    step: Anchor,
}

impl<'s, A> FlowModel<'s, A> {
    /// The fields every flow has: the heading, the step's title and card, Back, the card's width
    /// and the anchor of the fade through. The rest is named by the `with_*` methods.
    pub fn new(
        heading: Shown<'s>,
        title: Shown<'s>,
        card: Content<'s, A>,
        back: Offered<A>,
        width: Width,
        step: Anchor,
    ) -> Self {
        Self {
            heading,
            title,
            card,
            choices: Vec::new(),
            under: None,
            back,
            cancel: None,
            asking: None,
            refused: None,
            busy: None,
            width,
            step,
        }
    }

    /// Your choices: one row for each step answered or still to answer.
    pub fn with_choices(mut self, choices: Vec<ChoiceRow<'s, A>>) -> Self {
        self.choices = choices;
        self
    }

    /// The content under the card.
    pub fn with_under(mut self, under: Content<'s, A>) -> Self {
        self.under = Some(under);
        self
    }

    /// The Cancel the flow offers.
    pub fn with_cancel(mut self, cancel: Offered<A>) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// The dialog the flow is asking.
    pub fn with_asking(mut self, asking: Box<DialogModel<'s, A>>) -> Self {
        self.asking = Some(asking);
        self
    }

    /// The refusal the flow shows.
    pub fn with_refused(mut self, refused: Shown<'s>) -> Self {
        self.refused = Some(refused);
        self
    }

    /// The busy line the flow shows while it works.
    pub fn with_busy(mut self, busy: BusyModel<'s>) -> Self {
        self.busy = Some(busy);
        self
    }
}

/// Draws the step-flow template.
pub fn draw<A: Pressed>(_m: &FlowModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
