use faraday_kernel::{
    Carried, Ctx, Effects, Feature, FrameSheet, Notice, Public, Route, Top, View, route,
};

use crate::{App, AppPress, DeviceApp, Msg, Page};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {}

impl Feature for App {
    type State = DeviceApp;
    type Msg = Msg;
    type Out = Top;
    type Refusal = Refusal;

    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (DeviceApp, Effects<Msg, Top>) {
        let (device, effects) = <faraday_device::Device as Feature>::start(carried, cx);
        (DeviceApp::new(device), effects.lift(Msg::Device, out))
    }

    fn update(state: &mut DeviceApp, msg: Msg, cx: &Ctx<'_>) -> Effects<Msg, Top> {
        match msg {
            Msg::Device(m) => {
                let (device, _) = state.parts();
                route::<faraday_device::Device>(device, m, cx).lift(Msg::Device, out)
            }
            Msg::Front(sheet) => {
                state.parts().1.front(sheet);
                Effects::none()
            }
            Msg::Open(page) => {
                let left = state.parts().1.nav_mut().open(page);
                left_settings(left, state)
            }
            Msg::Find { typed, focused } => {
                state.set_find(typed, focused);
                Effects::none()
            }
            Msg::Press(AppPress::Back) => back(state, cx),
            Msg::Press(AppPress::Forward) => {
                let left = state.parts().1.nav_mut().forward();
                left_settings(left, state)
            }
            Msg::Press(press) => Self::update(state, Msg::from(press), cx),
        }
    }

    /// No notice reaches the device composite's children yet: Home and Power take none.
    fn notice(_state: &mut DeviceApp, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Top> {
        Effects::none()
    }

    fn refuses(state: &DeviceApp, msg: &Msg, view: &View<'_>) -> Option<Refusal> {
        match msg {
            Msg::Device(m) => <faraday_device::Device as Feature>::refuses(state.device(), m, view)
                .map(|r| match r {}),
            _ => None,
        }
    }
}

/// Back goes first to the top sheet of the frame's stack, then one page back (06 §4.3).
fn back(state: &mut DeviceApp, cx: &Ctx<'_>) -> Effects<Msg, Top> {
    match state.frame().sheets().first() {
        // The power feature's own sheets: Away puts the sheet away in its state and tells the
        // frame.
        Some(FrameSheet::Lock | FrameSheet::LockAsk | FrameSheet::IdleWarn | FrameSheet::Power) => {
            <App as Feature>::update(
                state,
                Msg::Device(faraday_device::Msg::Power(faraday_power::Msg::Press(
                    faraday_power::Press::Away,
                ))),
                cx,
            )
        }
        Some(_) => {
            state.parts().1.front(None);
            Effects::none()
        }
        None => {
            let left = state.parts().1.nav_mut().back();
            left_settings(left, state)
        }
    }
}

/// A page left by the navigation: where it was the settings owner's, and the page shown now is
/// not, the owner is told (A13), so what its screen held goes and the frame and the owner stay
/// in step (06 §4.4). The device composite's pages hold nothing a feature's screen needs yet.
fn left_settings(left: Option<Page>, state: &DeviceApp) -> Effects<Msg, Top> {
    match left {
        Some(Page::Settings(_)) if !matches!(state.frame().nav().shown(), Page::Settings(_)) => {
            Effects::none().out(Top::SettingsLeft)
        }
        _ => Effects::none(),
    }
}

/// What the device composite tells the app: a page to show or a sheet to put up is the app's
/// own message, an ending goes to the runtime.
fn out(out: faraday_device::Out) -> Route<Msg, Top> {
    use faraday_device::Out;
    match out {
        Out::Open(page) => Route::To(Msg::Open(Page::Device(page))),
        Out::Vaults => Route::To(Msg::Open(Page::Vaults(faraday_vaults::Page::Vaults))),
        Out::Settings => Route::To(Msg::Open(Page::Settings(faraday_settings::Page::Settings))),
        Out::Front(sheet) => Route::To(Msg::Front(sheet)),
        Out::Top(top) => Route::Up(top),
        Out::Search { typed, focused } => Route::To(Msg::Find { typed, focused }),
        // Nothing built sends a scan, a camera hold or a sample yet.
        Out::Leave(_) | Out::Camera(_) | Out::Sample(_) => todo!(),
    }
}
