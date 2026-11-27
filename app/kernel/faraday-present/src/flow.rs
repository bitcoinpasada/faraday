//! The step-flow template filled from `StepFlow` (04 §5).

use faraday_kernel::{Flow, FlowMsg, Line, Show, StepFlow, View};
use faraday_organisms::content::Content;
use faraday_organisms::{Pressed, Ui};
use faraday_templates::flow::FlowModel;

use crate::stages::{Page, Present};

/// What only the flow can say: written in its present module.
pub trait ShowFlow: StepFlow
where
    <Self as StepFlow>::Own: From<<Self as ShowFlow>::Press>,
{
    /// What a press on the flow's own controls sends.
    type Press: Pressed;

    fn title(&self, s: Self::Step, show: &Show<'_>) -> Line;

    /// One line of Your choices: never a secret (`Answer: Public`).
    fn choice(&self, a: &Self::Answer, show: &Show<'_>) -> ChoiceLine;

    /// The current step's card: the content organism's blocks.
    fn card<'s>(
        &'s self,
        view: &View<'s>,
        show: &Show<'s>,
    ) -> Content<'s, FlowPress<Self::Step, <Self as ShowFlow>::Press>>;

    /// What stands under Your choices and the foot pinned in the pane.
    fn under<'s>(
        &'s self,
        _view: &View<'s>,
        _show: &Show<'s>,
    ) -> Option<Content<'s, FlowPress<Self::Step, <Self as ShowFlow>::Press>>> {
        None
    }
}

/// One line of Your choices, worded.
pub struct ChoiceLine(pub Line);

/// A step flow's presses.
#[derive(Clone, PartialEq, Pressed)]
pub enum FlowPress<S, P> {
    Forward,
    Back,
    ForwardAgain,
    Open(S),
    Cancel,
    CancelSure,
    CancelNot,
    Own(P),
}

impl<S, M: From<P>, P> From<FlowPress<S, P>> for FlowMsg<S, M> {
    fn from(_press: FlowPress<S, P>) -> Self {
        todo!()
    }
}

pub type PressOf<F> = FlowPress<<F as StepFlow>::Step, <F as ShowFlow>::Press>;

impl<F: ShowFlow> Present for F
where
    <F as StepFlow>::Own: From<<F as ShowFlow>::Press>,
{
    type Model<'s>
        = FlowModel<'s, PressOf<F>>
    where
        Self: 's;

    fn present<'s>(_s: &'s Flow<F>, _view: &View<'s>, _show: &Show<'s>) -> Self::Model<'s> {
        todo!()
    }
}

impl<F: ShowFlow> Page for F
where
    <F as StepFlow>::Own: From<<F as ShowFlow>::Press>,
{
    type Press = PressOf<F>;

    /// The step-flow template.
    fn draw(_model: &Self::Model<'_>, _ui: &mut Ui<'_, PressOf<F>>) {
        todo!()
    }
}

/// Fills the step-flow template's model from `StepFlow` and `ShowFlow`, once, for every flow.
#[allow(
    clippy::disallowed_methods,
    reason = "V12: this fill is the one caller of FlowModel::new and its with_* methods"
)]
pub fn fill<'s, F: ShowFlow>(
    _flow: &'s Flow<F>,
    _view: &View<'s>,
    _show: &Show<'s>,
) -> FlowModel<'s, PressOf<F>>
where
    <F as StepFlow>::Own: From<<F as ShowFlow>::Press>,
{
    todo!()
}
