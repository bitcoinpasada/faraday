//! Step flows: a typed state machine driven by one runner (03 §9).

use crate::cx::{Ctx, View};
use crate::effects::Effects;
use crate::feature::{Feature, Notice};
use crate::lock::{AtLock, Carried, Holds, Loss};
use crate::message::{Message, Public};
use crate::words::{Drawn, Line, Refused, Worded};

pub trait StepFlow: AtLock + Sized + 'static {
    /// The flow's steps: one enum, Copy, Public.
    type Step: Copy + Eq + Public;
    /// The flow's own messages (presses on a step's card, answers).
    type Own: Message;
    type Out: Message + From<Ending>;
    type Refusal: Refused;
    /// The value an answered step shows in Your choices. Public, so it can hold no secret.
    type Answer: Public;

    // What the runner and present read.

    /// In order; whether the count is known.
    fn steps(&self) -> Steps<Self::Step>;
    fn at(&self) -> Self::Step;
    fn answer(&self, s: Self::Step) -> Option<Self::Answer>;
    /// The first step Back may not reach.
    fn back_stop(&self) -> Option<Self::Step>;
    fn running(&self) -> bool;
    /// Listed among flows in progress.
    fn begun(&self) -> bool;
    fn cancel(&self) -> CancelOffer;
    fn width(&self, s: Self::Step) -> Width;

    // What the runner calls.

    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (Self, FlowFx<Self>);
    fn forward(&mut self, cx: &Ctx<'_>) -> Result<FlowFx<Self>, Self::Refusal>;
    fn back(&mut self) -> Back;
    /// A row of Your choices; false: not reachable.
    fn open(&mut self, s: Self::Step) -> bool;
    fn own(&mut self, m: Self::Own, cx: &Ctx<'_>) -> FlowFx<Self>;
    fn notice(&mut self, n: &Notice<'_>, cx: &Ctx<'_>) -> FlowFx<Self>;
    fn refuses(&self, m: &Self::Own, view: &View<'_>) -> Option<Self::Refusal>;
    /// The flow is cancelled. No effects: a Cancel can order nothing.
    fn cancelled(&mut self) -> Ending;
    /// At a lock: what the next build's `start` needs. Most flows carry nothing.
    fn carry(&mut self, _into: &mut Carried) {}
}

pub enum FlowMsg<S, M> {
    Forward,
    Back,
    ForwardAgain,
    Open(S),
    Cancel,
    CancelSure,
    CancelNot,
    Own(M),
}

pub type FlowFx<F> =
    Effects<FlowMsg<<F as StepFlow>::Step, <F as StepFlow>::Own>, <F as StepFlow>::Out>;

impl<S: Public, M: Message> Message for FlowMsg<S, M> {
    fn kind(&self) -> &'static str {
        todo!()
    }

    fn public(&self) -> Option<&dyn core::fmt::Debug> {
        todo!()
    }
}

/// The state of a step flow: the kernel's part and the flow's own.
pub struct Flow<F: StepFlow> {
    inner: F,
    asking: Option<Loss>,
    refused: Option<F::Refusal>,
    again: Option<F::Step>,
}

impl<F: StepFlow> Flow<F> {
    pub fn inner(&self) -> &F {
        &self.inner
    }

    pub fn asking(&self) -> Option<Loss> {
        self.asking
    }

    pub fn refused(&self) -> Option<&F::Refusal> {
        self.refused.as_ref()
    }

    pub fn again(&self) -> Option<F::Step> {
        self.again
    }
}

impl<F: StepFlow> AtLock for Flow<F> {
    fn holds(&self) -> Holds {
        self.inner.holds()
    }

    fn hide(&mut self) {
        self.inner.hide();
    }

    fn at_lock(&mut self) {
        self.inner.at_lock();
    }
}

/// What the runner itself refuses (A29), or the flow's own refusal.
#[derive(Public)]
pub enum FlowRefusal<R: Refused> {
    /// Back, Open and Cancel while the step runs.
    Running,
    /// Back or Open to `back_stop()` or before.
    BackStop,
    /// Cancel where `cancel()` is `NotOffered`.
    NotOffered,
    Own(R),
}

impl<R: Refused> Worded for FlowRefusal<R> {
    fn words(&self, _show: &crate::cx::Show<'_>) -> Line {
        todo!()
    }
}

impl<R: Refused> Refused for FlowRefusal<R> {
    fn drawn(&self) -> Drawn {
        todo!()
    }
}

/// A step flow is a feature: the blanket impl whose `update` is the step-flow runner (A29).
impl<F: StepFlow> Feature for F {
    type State = Flow<F>;
    type Msg = FlowMsg<F::Step, F::Own>;
    type Out = F::Out;
    type Refusal = FlowRefusal<F::Refusal>;

    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (Self::State, Effects<Self::Msg, Self::Out>) {
        let (inner, effects) = <F as StepFlow>::start(carried, cx);
        let flow = Flow {
            inner,
            asking: None,
            refused: None,
            again: None,
        };
        (flow, effects)
    }

    fn update(
        _state: &mut Self::State,
        _msg: Self::Msg,
        _cx: &Ctx<'_>,
    ) -> Effects<Self::Msg, Self::Out> {
        todo!()
    }

    fn notice(
        _state: &mut Self::State,
        _notice: &Notice<'_>,
        _cx: &Ctx<'_>,
    ) -> Effects<Self::Msg, Self::Out> {
        todo!()
    }

    fn refuses(_state: &Self::State, _msg: &Self::Msg, _view: &View<'_>) -> Option<Self::Refusal> {
        todo!()
    }

    fn carry(_state: &mut Self::State, _into: &mut Carried) {
        todo!()
    }
}

/// The steps in order, and whether their count is known ("Step N of M" or "Step N").
pub struct Steps<S> {
    pub list: Vec<S>,
    pub known: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CancelOffer {
    Now,
    Ask(Loss),
    NotOffered,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Back {
    Stepped,
    Leave,
}

/// How a flow ended: Back from its first step, a Cancel, or its last step done.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Ending {
    Left,
    Cancelled,
    Finished,
}

/// How wide a step's card is (LAYOUT §14).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Width {
    Column,
    Wide,
    Full,
}
