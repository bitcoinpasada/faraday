use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, View};

use crate::{Msg, Online, OnlineApp, Out};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

impl Feature for Online {
    type State = OnlineApp;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (OnlineApp, Effects<Msg, Out>) {
        todo!()
    }

    fn update(_state: &mut OnlineApp, _msg: Msg, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        todo!()
    }

    fn notice(_state: &mut OnlineApp, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        todo!()
    }

    fn refuses(_state: &OnlineApp, _msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        todo!()
    }
}
