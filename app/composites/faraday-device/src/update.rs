use faraday_home::Home;
use faraday_kernel::{
    Carried, Ctx, Effects, Feature, FrameSheet, Notice, Public, Route, View, route,
};
use faraday_power::Power;

use crate::{Device, Msg, Out, Page, State};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

impl Feature for Device {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (State, Effects<Msg, Out>) {
        State::start(carried, cx)
    }

    fn update(state: &mut State, msg: Msg, cx: &Ctx<'_>) -> Effects<Msg, Out> {
        match msg {
            Msg::Home(m) => route::<Home>(&mut state.home, m, cx).lift(Msg::Home, home_out),
            Msg::Power(m) => route::<Power>(&mut state.power, m, cx).lift(Msg::Power, power_out),
            Msg::Press(press) => Self::update(state, Msg::from(press), cx),
            // The features and areas below are not built: no page of theirs is shown and no
            // sheet of theirs comes up, so no message of theirs arrives.
            Msg::Devices(_)
            | Msg::Scan(_)
            | Msg::Guide(_)
            | Msg::Keys(_)
            | Msg::Wallets(_)
            | Msg::Signing(_)
            | Msg::Checks(_)
            | Msg::Tools(_)
            | Msg::Files(_) => todo!(),
        }
    }

    /// No notice reaches a child yet: Home and Power take none, and the others are not built.
    fn notice(_state: &mut State, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        Effects::none()
    }

    fn refuses(state: &State, msg: &Msg, view: &View<'_>) -> Option<Refusal> {
        match msg {
            Msg::Home(m) => Home::refuses(&state.home, m, view).map(|r| match r {}),
            Msg::Power(m) => Power::refuses(&state.power, m, view).map(|r| match r {}),
            _ => None,
        }
    }
}

/// What Home asks of the composite (06 §4.3): the flows it begins are other features' pages.
pub(crate) fn home_out(out: faraday_home::Out) -> Route<Msg, Out> {
    use faraday_home::Out as H;
    match out {
        H::CreateWallet => Route::Up(Out::Open(Page::Wallets(faraday_wallets_area::Page::Create))),
        H::RestoreWallet => Route::Up(Out::Open(Page::Wallets(
            faraday_wallets_area::Page::Restore,
        ))),
        H::SpendBackup => Route::Up(Out::Open(Page::Signing(faraday_signing_area::Page::Spend))),
        H::GuideWord(_) => Route::Up(Out::Front(Some(FrameSheet::Guide))),
    }
}

/// What Power asks of the composite: its sheets and its endings go up.
pub(crate) fn power_out(out: faraday_power::Out) -> Route<Msg, Out> {
    match out {
        faraday_power::Out::Front(sheet) => Route::Up(Out::Front(sheet)),
        faraday_power::Out::Top(top) => Route::Up(Out::Top(top)),
    }
}
