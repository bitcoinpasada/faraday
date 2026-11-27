use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, View};

use crate::{Msg, Out, SeedChecks, State};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

impl Feature for SeedChecks {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (State, Effects<Msg, Out>) {
        (State::new(), Effects::none())
    }

    fn update(_state: &mut State, _msg: Msg, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        todo!()
    }

    fn notice(_state: &mut State, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        todo!()
    }

    fn refuses(_state: &State, _msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        todo!()
    }
}
