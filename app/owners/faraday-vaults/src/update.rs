use faraday_kernel::vaults::{self, Change};
use faraday_kernel::{AnyChange, ChangeId, Done, Owner};
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
            Msg::Given(id, change) => given(id, change),
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

/// The change given, answered once. No vault can be open yet, so the runtime's Close at a lock
/// has nothing to seal or close and is answered at once: without it the lock waits at the seal
/// and the session's keys stay (05 §7.2). The vault tickets replace this with sealing.
fn given(id: ChangeId, change: AnyChange) -> Effects<Msg, Out> {
    match change {
        AnyChange::Vaults(Change::Close(_)) => {
            Effects::none().done(id, Done::Vaults(vaults::Done::Close(())))
        }
        // No other change is sent to this owner yet; another owner's is not this one's to answer.
        _ => Effects::none(),
    }
}

impl Owner for Vaults {
    fn given(id: ChangeId, change: AnyChange) -> Msg {
        Msg::Given(id, change)
    }
}
