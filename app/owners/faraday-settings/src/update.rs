use faraday_kernel::{AnyChange, ChangeId, Owner};
use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, View};

use crate::{Msg, Out, Settings, State};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

impl Feature for Settings {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (State, Effects<Msg, Out>) {
        (State::new(Default::default()), Effects::none())
    }

    fn update(_state: &mut State, msg: Msg, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        match msg {
            Msg::Press(press) => match press {},
            // No change is sent to this owner yet.
            Msg::Given(..) => Effects::none(),
        }
    }

    fn notice(_state: &mut State, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        Effects::none()
    }

    fn refuses(_state: &State, msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        match msg {
            Msg::Press(press) => match *press {},
            Msg::Given(..) => None,
        }
    }
}

impl Owner for Settings {
    fn given(id: ChangeId, change: AnyChange) -> Msg {
        Msg::Given(id, change)
    }
}
