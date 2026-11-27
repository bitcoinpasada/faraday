use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, Top, View};

use crate::{App, DeviceApp, Msg};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

impl Feature for App {
    type State = DeviceApp;
    type Msg = Msg;
    type Out = Top;
    type Refusal = Refusal;

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (DeviceApp, Effects<Msg, Top>) {
        todo!()
    }

    fn update(_state: &mut DeviceApp, _msg: Msg, _cx: &Ctx<'_>) -> Effects<Msg, Top> {
        todo!()
    }

    fn notice(_state: &mut DeviceApp, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Top> {
        todo!()
    }

    fn refuses(_state: &DeviceApp, _msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        todo!()
    }
}
