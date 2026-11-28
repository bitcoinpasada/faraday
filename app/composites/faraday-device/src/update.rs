use faraday_devices::Devices;
use faraday_guide_sheet::GuideSheet;
use faraday_home::Home;
use faraday_kernel::{
    Carried, Ctx, Effects, End, Feature, Notice, Public, Route, Top, View, route,
};
use faraday_power::Power;

use crate::{Device, Msg, Out, Page, State};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {
    /// The device check's: a virtual machine is never let through, and the sheet asks first.
    Devices(faraday_devices::Refusal),
}

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
            Msg::Power(m) => {
                let covered = lock_up(state);
                let fx = route::<Power>(&mut state.power, m, cx).lift(Msg::Power, power_out);
                if covered || !lock_up(state) {
                    return fx;
                }
                // The Lock sheet comes up over whatever is open: what waited under the Devices
                // sheet goes (`main`'s `lock_sheet_replaces_devices`). Only devices is told: a
                // notice to power would give a mode change's Lock sheet up.
                let left = <Devices as Feature>::notice(&mut state.devices, &Notice::Left, cx);
                fx.and(left.lift(Msg::Devices, devices_out))
            }
            Msg::Guide(m) => {
                route::<GuideSheet>(&mut state.guide, m, cx).lift(Msg::Guide, guide_out)
            }
            Msg::Devices(m) => {
                route::<Devices>(&mut state.devices, m, cx).lift(Msg::Devices, devices_out)
            }
            Msg::Press(press) => Self::update(state, Msg::from(press), cx),
            // The features and areas below are not built: no page of theirs is shown and no
            // sheet of theirs comes up, so no message of theirs arrives.
            Msg::Scan(_)
            | Msg::Keys(_)
            | Msg::Wallets(_)
            | Msg::Signing(_)
            | Msg::Checks(_)
            | Msg::Tools(_)
            | Msg::Files(_) => todo!(),
        }
    }

    /// Every notice goes to each child that is built, in order (03 A21); the other children are
    /// not built and take none.
    fn notice(state: &mut State, notice: &Notice<'_>, cx: &Ctx<'_>) -> Effects<Msg, Out> {
        <Home as Feature>::notice(&mut state.home, notice, cx)
            .lift(Msg::Home, home_out)
            .and(
                <Power as Feature>::notice(&mut state.power, notice, cx)
                    .lift(Msg::Power, power_out),
            )
            .and(
                <Devices as Feature>::notice(&mut state.devices, notice, cx)
                    .lift(Msg::Devices, devices_out),
            )
            .and(
                <GuideSheet as Feature>::notice(&mut state.guide, notice, cx)
                    .lift(Msg::Guide, guide_out),
            )
    }

    fn refuses(state: &State, msg: &Msg, view: &View<'_>) -> Option<Refusal> {
        match msg {
            Msg::Home(m) => Home::refuses(&state.home, m, view).map(|r| match r {}),
            Msg::Power(m) => Power::refuses(&state.power, m, view).map(|r| match r {}),
            Msg::Guide(m) => GuideSheet::refuses(&state.guide, m, view).map(|r| match r {}),
            Msg::Devices(m) => Devices::refuses(&state.devices, m, view).map(Refusal::Devices),
            _ => None,
        }
    }
}

/// A Lock sheet is up: the one a lock asked for, or the one after it.
fn lock_up(state: &State) -> bool {
    matches!(
        state.power.sheet(),
        Some(faraday_power::Which::LockAsk | faraday_power::Which::Locked)
    )
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
        H::GuideWord(i) => Route::To(Msg::Guide(faraday_guide_sheet::Msg::OpenWord(
            home_place(),
            i,
        ))),
        H::Guide => Route::To(Msg::Guide(faraday_guide_sheet::Msg::Open(home_place()))),
        // A PSBT in Files and an unfinished backup are opened by the Spend and Back up flows,
        // which are not built here: the lead's press shows the page that holds the PSBT.
        H::SignPsbt(_) => Route::Up(Out::Open(Page::Signing(faraday_signing_area::Page::Spend))),
        H::FinishBackup(_) => Route::Up(Out::Open(Page::WALLETS)),
        H::Place(place) => match place {
            faraday_home::Place::Wallets => Route::Up(Out::Open(Page::WALLETS)),
            faraday_home::Place::Check => Route::Up(Out::Open(Page::CHECK)),
            faraday_home::Place::Files => Route::Up(Out::Open(Page::FILES)),
            faraday_home::Place::Tools => Route::Up(Out::Open(Page::TOOLS)),
            faraday_home::Place::Vaults => Route::Up(Out::Vaults),
            faraday_home::Place::Settings => Route::Up(Out::Settings),
        },
        H::Search { typed, focused } => Route::Up(Out::Search { typed, focused }),
    }
}

/// Where the guide opens from Home: the Home page's section, in a window this composite knows
/// nothing of. The window's class and what the page leaves free are the root's: its follow
/// message carries them, and the guide becomes the pane then where the window holds one.
fn home_place() -> faraday_guide_sheet::Place {
    faraday_guide_sheet::Place {
        key: faraday_text::Text::of("home"),
        how_screen: None,
        release: faraday_text::Text::default(),
        room: faraday_guide_sheet::Room {
            extra_large: false,
            compact: false,
            held: false,
            place_free: true,
        },
    }
}

/// What the guide asks of the composite: its sheet goes up.
pub(crate) fn guide_out(out: faraday_guide_sheet::Out) -> Route<Msg, Out> {
    match out {
        faraday_guide_sheet::Out::Front(sheet) => Route::Up(Out::Front(sheet)),
    }
}

/// What Power asks of the composite: its sheets and its endings go up.
pub(crate) fn power_out(out: faraday_power::Out) -> Route<Msg, Out> {
    match out {
        faraday_power::Out::Front(sheet) => Route::Up(Out::Front(sheet)),
        faraday_power::Out::Top(top) => Route::Up(Out::Top(top)),
    }
}

/// What the device check asks of the composite: its sheets and the end go up, and secrets on
/// show go off the screen when something that can capture it arrives (06 §7; `main`'s
/// `concerns_changed`).
pub(crate) fn devices_out(out: faraday_devices::Out) -> Route<Msg, Out> {
    use faraday_devices::Out as D;
    match out {
        D::Front(sheet) => Route::Up(Out::Front(sheet)),
        D::Hide => Route::Up(Out::Top(Top::Hide)),
        D::PowerOff => Route::Up(Out::Top(Top::End(End::PowerOff))),
        // The features that wait for the check are not built: no gate is open to answer.
        D::Cleared(_) => Route::Drop,
    }
}
