use faraday_kernel::{
    Carried, Ctx, Effects, Feature, Frame, FrameSheet, Notice, Public, Route, Top, View, route,
};

use crate::{App, AppPress, DeviceApp, Msg, Page};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {
    /// The device composite refused it.
    Device(faraday_device::Refusal),
}

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
            Msg::Away(sheet) => {
                take_out(state.parts().1, sheet);
                Effects::none()
            }
            Msg::Front(sheet) => {
                cover(state.parts().1, sheet);
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

    /// Every notice goes to the device composite, which tells its children.
    fn notice(state: &mut DeviceApp, notice: &Notice<'_>, cx: &Ctx<'_>) -> Effects<Msg, Top> {
        let (device, _) = state.parts();
        <faraday_device::Device as Feature>::notice(device, notice, cx).lift(Msg::Device, out)
    }

    fn refuses(state: &DeviceApp, msg: &Msg, view: &View<'_>) -> Option<Refusal> {
        match msg {
            Msg::Device(m) => <faraday_device::Device as Feature>::refuses(state.device(), m, view)
                .map(Refusal::Device),
            _ => None,
        }
    }
}

/// A sheet comes up in its place of the stack. The Lock sheet replaces the Devices sheet: the
/// press that waited under it is dropped by the composite, and the sheet goes from the stack so
/// it does not come back empty when the Lock sheet is put away. A New input sheet waits under
/// it and stays.
fn cover(frame: &mut Frame<Page>, sheet: Option<FrameSheet>) {
    if matches!(sheet, Some(FrameSheet::Lock | FrameSheet::LockAsk)) {
        take_out(frame, FrameSheet::Devices);
    }
    frame.front(sheet);
}

/// The sheet is taken out of the stack, whatever is over it; the others keep their places.
fn take_out(frame: &mut Frame<Page>, sheet: FrameSheet) {
    if !frame.sheets().contains(&sheet) {
        return;
    }
    let keep: Vec<FrameSheet> = frame
        .sheets()
        .iter()
        .copied()
        .filter(|s| *s != sheet)
        .collect();
    while !frame.sheets().is_empty() {
        frame.front(None);
    }
    for s in keep {
        frame.front(Some(s));
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
        // The New input sheet stays until the device is decided (F202): Back and Escape do
        // nothing over it.
        Some(FrameSheet::NewInput) => Effects::none(),
        // The Devices sheet is put away, and the feature drops the press that waited under it
        // (`Notice::Left`) so a later press brings the sheet up again.
        Some(FrameSheet::Devices) => {
            take_out(state.parts().1, FrameSheet::Devices);
            // Devices alone is told: Back leaves no page, so no other child's notice applies.
            <App as Feature>::update(state, Msg::Device(faraday_device::Msg::devices_away()), cx)
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
        Out::Away(sheet) => Route::To(Msg::Away(sheet)),
        Out::Top(top) => Route::Up(top),
        Out::Search { typed, focused } => Route::To(Msg::Find { typed, focused }),
        // Nothing built sends a scan, a camera hold or a sample yet.
        Out::Leave(_) | Out::Camera(_) | Out::Sample(_) => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use faraday_app_test::{TestView, seen};
    use faraday_kernel::{Carried, Gate};
    use faraday_port::Text;
    use faraday_port::devices::{Edid, Interface, UsbDevice};

    use super::*;

    /// Sends `msg` through the app and everything it sends itself, as the runtime does.
    fn send(state: &mut DeviceApp, msg: Msg, view: &TestView) {
        let mut waiting = vec![msg];
        while let Some(msg) = waiting.pop() {
            let fx = route::<App>(state, msg, &view.ctx());
            waiting.extend(seen(fx).thens);
        }
    }

    /// A machine whose screen is an ordinary monitor, so no reminder is due at start.
    fn on_a_monitor() -> TestView {
        TestView::device().with_machine(|m| {
            let mut e = [0u8; 128];
            e[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
            e[54 + 3] = 0xfc;
            for (i, b) in b"Monitor".iter().enumerate() {
                e[54 + 5 + i] = *b;
            }
            m.screen = Some(Edid(e));
        })
    }

    fn started(view: &TestView) -> DeviceApp {
        <App as Feature>::start(&mut Carried::empty(), &view.ctx()).0
    }

    fn kvm() -> UsbDevice {
        UsbDevice {
            port: Text::of("1-2"),
            vendor: 0x1d6b,
            product: 0x0104,
            manufacturer: Text::new(),
            name: Text::of("PiKVM"),
            interfaces: vec![Interface {
                class: 3,
                subclass: 0,
                protocol: 1,
            }],
        }
    }

    fn devices(msg: faraday_devices::Msg) -> Msg {
        Msg::Device(faraday_device::Msg::Devices(msg))
    }

    fn waiting(state: &DeviceApp) -> Option<Gate> {
        state.device().devices().waiting()
    }

    /// Invariant: A refusal of the device check reaches the product: a press of Continue anyway nobody asked for is refused, never swallowed.
    /// Regression: The product's Refusal was empty, so the device check's refusal broke the build and had nowhere to go.
    /// Owner: app
    #[test]
    fn the_device_checks_refusal_reaches_the_product() {
        let view = TestView::device();
        let (state, _) = <App as Feature>::start(&mut Carried::empty(), &view.ctx());
        let msg = Msg::Device(faraday_device::Msg::Devices(
            faraday_devices::Press::LetThrough.into(),
        ));
        assert!(matches!(
            <App as Feature>::refuses(&state, &msg, &view.ctx().view),
            Some(Refusal::Device(faraday_device::Refusal::Devices(
                faraday_devices::Refusal::NotAsked
            )))
        ));
    }

    /// Invariant: The press that waited under the Devices sheet is dropped when the Lock sheet comes up over it, and the sheet does not come back when the Lock sheet is put away.
    /// Regression: A press made before the lock was asked was answered by Check again after Away, and the notice never reached the device check.
    /// Owner: app
    #[test]
    fn a_lock_sheet_drops_the_press_that_waited_under_the_devices_sheet() {
        let view = on_a_monitor();
        let mut state = started(&view);
        send(
            &mut state,
            devices(faraday_devices::Msg::Usb(vec![kvm()])),
            &view,
        );
        send(
            &mut state,
            devices(faraday_devices::Msg::Reveal(Gate(1))),
            &view,
        );
        assert_eq!(waiting(&state), Some(Gate(1)));
        assert!(state.frame().sheets().contains(&FrameSheet::Devices));

        send(
            &mut state,
            Msg::Device(faraday_device::Msg::Power(faraday_power::Msg::Lock {
                asks_first: true,
            })),
            &view,
        );
        assert_eq!(waiting(&state), None);
        assert_eq!(state.frame().sheets(), [FrameSheet::LockAsk]);
        send(&mut state, Msg::Press(AppPress::Back), &view);
        assert!(state.frame().sheets().is_empty());
        assert_eq!(waiting(&state), None);
    }

    /// Invariant: Back or Escape over the Devices sheet puts it away and the feature forgets it was up, so the next press that shows a secret brings it up again; over New input they do nothing.
    /// Regression: The sheet went but the feature kept it as up, so every later press waited with nothing on screen; Escape over New input took away a sheet that stays until its device is decided.
    /// Owner: app
    #[test]
    fn back_puts_the_devices_sheet_away_and_leaves_new_input() {
        let view = on_a_monitor();
        let mut state = started(&view);
        send(
            &mut state,
            devices(faraday_devices::Msg::Usb(vec![kvm()])),
            &view,
        );
        send(
            &mut state,
            devices(faraday_devices::Msg::Reveal(Gate(1))),
            &view,
        );
        send(&mut state, Msg::Press(AppPress::Back), &view);
        assert!(state.frame().sheets().is_empty());
        assert_eq!(waiting(&state), None);
        assert!(!state.device().devices().devices_up());
        send(
            &mut state,
            devices(faraday_devices::Msg::Reveal(Gate(2))),
            &view,
        );
        assert_eq!(state.frame().sheets(), [FrameSheet::Devices]);
        assert_eq!(waiting(&state), Some(Gate(2)));

        // A New input sheet over it stays.
        send(&mut state, Msg::Front(Some(FrameSheet::NewInput)), &view);
        send(&mut state, Msg::Press(AppPress::Back), &view);
        assert_eq!(
            state.frame().sheets(),
            [FrameSheet::NewInput, FrameSheet::Devices]
        );
    }

    /// Invariant: A sheet the feature puts away goes by name: with the Lock sheet over New input, the New input entry goes and Back then puts the Lock sheet away.
    /// Regression: The feature's put-away took the top sheet, so the Lock sheet went and a stale New input entry swallowed Back.
    /// Owner: app
    #[test]
    fn a_sheet_put_away_by_name_leaves_what_is_over_it() {
        let view = on_a_monitor();
        let mut state = started(&view);
        send(&mut state, Msg::Front(Some(FrameSheet::NewInput)), &view);
        send(
            &mut state,
            Msg::Device(faraday_device::Msg::Power(faraday_power::Msg::Lock {
                asks_first: true,
            })),
            &view,
        );
        assert_eq!(
            state.frame().sheets(),
            [FrameSheet::LockAsk, FrameSheet::NewInput]
        );
        send(&mut state, Msg::Away(FrameSheet::NewInput), &view);
        assert_eq!(state.frame().sheets(), [FrameSheet::LockAsk]);
        send(&mut state, Msg::Press(AppPress::Back), &view);
        assert!(state.frame().sheets().is_empty());
    }

    /// Invariant: Check again sends the waiting press through the hold again: with a reminder still due it stays held and the Devices sheet stays up.
    /// Regression: Pulling the KVM and pressing Check again answered the press at once and the wireless receiver's reminder was never shown.
    /// Owner: app
    #[test]
    fn check_again_keeps_the_press_held_while_a_reminder_is_due() {
        let view = on_a_monitor();
        let mut state = started(&view);
        let receiver = UsbDevice {
            port: Text::of("1-3"),
            vendor: 0x046d,
            product: 0xc52b,
            name: Text::of("Wireless Receiver"),
            ..kvm()
        };
        send(
            &mut state,
            devices(faraday_devices::Msg::Usb(vec![kvm(), receiver.clone()])),
            &view,
        );
        send(
            &mut state,
            devices(faraday_devices::Msg::Reveal(Gate(1))),
            &view,
        );
        send(
            &mut state,
            devices(faraday_devices::Msg::Usb(vec![receiver])),
            &view,
        );
        send(
            &mut state,
            devices(faraday_devices::Press::Check.into()),
            &view,
        );
        assert_eq!(waiting(&state), Some(Gate(1)));
        assert!(state.device().devices().devices_up());
        assert_eq!(state.frame().sheets(), [FrameSheet::Devices]);
    }
}
