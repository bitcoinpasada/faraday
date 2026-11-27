use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, Top, View};

use crate::{Msg, Outside, StarterApp};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

impl Feature for Outside {
    type State = StarterApp;
    type Msg = Msg;
    type Out = Top;
    type Refusal = Refusal;

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (StarterApp, Effects<Msg, Top>) {
        todo!()
    }

    fn update(_state: &mut StarterApp, _msg: Msg, _cx: &Ctx<'_>) -> Effects<Msg, Top> {
        todo!()
    }

    fn notice(_state: &mut StarterApp, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Top> {
        todo!()
    }

    fn refuses(_state: &StarterApp, _msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        todo!()
    }
}
