use faraday_kernel::{AnyChange, ChangeId, Owner};
use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, View};

use crate::{Msg, Out, State, Vaults};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

impl Feature for Vaults {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (State, Effects<Msg, Out>) {
        (State::start(carried, cx), Effects::none())
    }

    fn update(_state: &mut State, msg: Msg, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        match msg {
            // No change is sent to this owner yet.
            Msg::Given(..) => Effects::none(),
            Msg::Press(_)
            | Msg::List(_)
            | Msg::Unlock(_)
            | Msg::Contents(_)
            | Msg::Seal(_)
            | Msg::Totp(_)
            | Msg::Gpg(_) => todo!(),
        }
    }

    fn notice(_state: &mut State, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        Effects::none()
    }

    fn refuses(_state: &State, _msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        None
    }
}

impl Owner for Vaults {
    fn given(id: ChangeId, change: AnyChange) -> Msg {
        Msg::Given(id, change)
    }
}
