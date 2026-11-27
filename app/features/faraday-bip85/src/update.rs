use faraday_kernel::{
    Back, CancelOffer, Carried, Ctx, Ending, FlowFx, Notice, Public, StepFlow, Steps, View, Width,
};

use crate::{Out, Own, Bip85};

/// The flow's steps, in order.
// Spec: 03 §9 names no steps of this flow.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Step {}

/// Why a step or a message is refused.
#[derive(Public)]
pub enum Refusal {}

/// The value an answered step shows in Your choices.
// Spec: 03 §9.1 names `Answer` without a shape.
#[derive(Public)]
pub struct Answer;

impl StepFlow for Bip85 {
    type Step = Step;
    type Own = Own;
    type Out = Out;
    type Refusal = Refusal;
    type Answer = Answer;

    fn steps(&self) -> Steps<Step> {
        todo!()
    }

    fn at(&self) -> Step {
        todo!()
    }

    fn answer(&self, _s: Step) -> Option<Answer> {
        todo!()
    }

    fn back_stop(&self) -> Option<Step> {
        todo!()
    }

    fn running(&self) -> bool {
        todo!()
    }

    fn begun(&self) -> bool {
        todo!()
    }

    fn cancel(&self) -> CancelOffer {
        todo!()
    }

    fn width(&self, _s: Step) -> Width {
        todo!()
    }

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (Bip85, FlowFx<Bip85>) {
        todo!()
    }

    fn forward(&mut self, _cx: &Ctx<'_>) -> Result<FlowFx<Bip85>, Refusal> {
        todo!()
    }

    fn back(&mut self) -> Back {
        todo!()
    }

    fn open(&mut self, _s: Step) -> bool {
        todo!()
    }

    fn own(&mut self, _m: Own, _cx: &Ctx<'_>) -> FlowFx<Bip85> {
        todo!()
    }

    fn notice(&mut self, _n: &Notice<'_>, _cx: &Ctx<'_>) -> FlowFx<Bip85> {
        todo!()
    }

    fn refuses(&self, _m: &Own, _view: &View<'_>) -> Option<Refusal> {
        todo!()
    }

    fn cancelled(&mut self) -> Ending {
        todo!()
    }
}
