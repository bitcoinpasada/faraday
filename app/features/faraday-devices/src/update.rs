use faraday_kernel::session::Draw;
use faraday_kernel::{Carried, Ctx, Effects, Feature, FrameSheet, Gate, Notice, Public, View};
use faraday_port::Order;
use faraday_port::devices::InputId;

use crate::check::DeviceCheck;
use crate::inputs::{Admitted, DRAW_LEN, DRAW_WHAT, Hit, Inputs, Tried};
use crate::{COUNTS, Devices, Msg, Out, Press, Said, State};

/// Why a message is refused before `update` sees it.
#[derive(Public)]
pub enum Refusal {
    /// Continue anyway, with a virtual machine held: there is no way on.
    VirtualMachine,
    /// Let through before the sheet asked.
    NotAsked,
    /// I understand, while something is still held.
    StillHeld,
}

type Fx = Effects<Msg, Out>;

/// Asks for the randomness of device `id`'s code and targets, once.
fn draw(state: &mut State, id: u32) -> Fx {
    if state.is_asked(id) || !state.inputs().needs_code(id) {
        return Effects::none();
    }
    state.ask(id, true);
    Effects::none().change(
        Draw {
            what: DRAW_WHAT,
            n: DRAW_LEN,
        },
        id,
        |id, bytes| Msg::Drawn { id, bytes },
    )
}

/// What the adapter has not been told of the devices decided.
fn decided(state: &mut State) -> Fx {
    state
        .inputs_mut()
        .decisions()
        .into_iter()
        .fold(Effects::none(), |fx, (id, believed)| {
            fx.order(Order::DecideInput {
                id: InputId(id),
                believed,
            })
        })
}

/// The New input sheet shows while a device has acted and waits, and goes when none has; the
/// adapter's arrow aims at a pointer whose targets show.
fn input_sheet(state: &mut State) -> Fx {
    let (shown, aims) = match state.inputs().shown() {
        Some(d) => (
            true,
            (d.pointer && !d.touch && state.inputs().by_target(d)).then_some(d.id),
        ),
        None => (false, None),
    };
    let mut fx = Effects::none();
    if shown && !state.input_up() {
        state.set_input_up(true);
        fx = fx.out(Out::Front(Some(FrameSheet::NewInput)));
    } else if !shown && state.input_up() {
        state.set_input_up(false);
        state.set_target_drawn(false);
        state.set_room_fits(false);
        fx = fx.out(Out::Away(FrameSheet::NewInput));
    }
    if aims != state.aimed() {
        state.aim(aims);
        fx = fx.order(Order::AimInput(aims.map(InputId)));
    }
    fx
}

/// The Devices sheet comes up when an item is held and it is due.
fn devices_sheet(state: &mut State) -> Fx {
    if state.check_mut().take_due() && !state.devices_up() {
        state.set_devices_up(true);
        Effects::none().out(Out::Front(Some(FrameSheet::Devices)))
    } else {
        Effects::none()
    }
}

/// What a check's change led to: secrets on show go off the screen, and the sheet comes.
fn arrived(state: &mut State, arrived: bool) -> Fx {
    let fx = if arrived {
        Effects::none().out(Out::Hide)
    } else {
        Effects::none()
    };
    fx.and(devices_sheet(state))
}

/// The hold ended: the sheet goes, and what waited carries on.
fn released(state: &mut State) -> Fx {
    let mut fx = Effects::none();
    if state.devices_up() {
        state.set_devices_up(false);
        fx = fx.out(Out::Away(FrameSheet::Devices));
    }
    match state.waiting() {
        Some(gate) => {
            state.wait(None);
            fx.out(Out::Cleared(gate))
        }
        None => fx,
    }
}

/// The waiting press goes through the hold again once nothing is named (`main`'s
/// `device_released`): while a reminder is still due it stays held under the sheet, which then
/// shows the reminders and I understand.
fn rerun(state: &mut State) -> Fx {
    match state.waiting() {
        Some(_) if state.check().holding() => Effects::none(),
        _ => released(state),
    }
}

/// A press that would show a secret.
fn reveal(state: &mut State, gate: Gate) -> Fx {
    if !state.check().holding() {
        return Effects::none().out(Out::Cleared(gate));
    }
    // A second press replaces the first.
    state.wait(Some(gate));
    state.check_mut().set_asking(false);
    if state.devices_up() {
        Effects::none()
    } else {
        state.set_devices_up(true);
        Effects::none().out(Out::Front(Some(FrameSheet::Devices)))
    }
}

fn press(state: &mut State, press: Press) -> Fx {
    match press {
        Press::Check => {
            if state.check_mut().check_again() {
                Effects::none().note(Said::Still)
            } else {
                rerun(state)
            }
        }
        Press::Continue => {
            if !state.check().refuses() {
                state.check_mut().set_asking(true);
            }
            Effects::none()
        }
        Press::LetThrough => {
            if !state.check_mut().let_all_through() {
                return Effects::none();
            }
            // What the sheet listed with them is seen too.
            if state.waiting().is_some() {
                state.check_mut().understand();
            }
            released(state)
        }
        Press::Understood => {
            if !state.check().held().is_empty() {
                return Effects::none();
            }
            state.check_mut().understand();
            released(state)
        }
        Press::Cancel => {
            // What was pressed while held is not done, and Continue anyway asks again.
            state.wait(None);
            state.check_mut().set_asking(false);
            if state.devices_up() {
                state.set_devices_up(false);
                Effects::none().out(Out::Away(FrameSheet::Devices))
            } else {
                Effects::none()
            }
        }
        Press::PowerOff => Effects::none().out(Out::PowerOff),
        Press::Ignore(id) => {
            state.inputs_mut().ignore(id);
            decided(state).and(input_sheet(state))
        }
        Press::Use(id) => believe(state, id),
        Press::CloseNotice => {
            state.inputs_mut().notice_close();
            Effects::none()
        }
    }
}

/// The device is believed from now on, and the toast says what kind.
fn believe(state: &mut State, id: u32) -> Fx {
    let Some(d) = state.inputs_mut().believe(id) else {
        return Effects::none();
    };
    let said = match (d.keyboard, d.pointer, d.touch) {
        (_, _, true) => Said::TouchIn(d.name.clone()),
        (true, true, _) => Said::KeyboardPointerIn(d.name.clone()),
        (true, false, _) => Said::KeyboardIn(d.name.clone()),
        _ => Said::PointerIn(d.name.clone()),
    };
    decided(state).note(said).and(input_sheet(state))
}

/// What a held-back device's try led to.
fn tried(state: &mut State, tried: Tried, touch_wrong: bool, id: u32) -> Fx {
    match tried {
        Tried::Nothing => Effects::none(),
        Tried::Matched => believe(state, id),
        Tried::KeptOut { name, touch, limit } => {
            let said = if limit {
                Said::Ignoring {
                    alone: state.inputs().alone(),
                }
            } else if touch_wrong {
                if touch {
                    Said::WrongTaps(name.clone())
                } else {
                    Said::WrongPresses(name.clone())
                }
            } else {
                Said::WrongCode(name.clone())
            };
            decided(state).note(said).and(input_sheet(state))
        }
    }
}

/// The device acted: it asks for its sheet.
fn used(state: &mut State, id: u32) -> Fx {
    if state.inputs_mut().used(id) {
        input_sheet(state)
    } else {
        Effects::none()
    }
}

impl Feature for Devices {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (State, Fx) {
        let read = cx.view.facts.abilities.holds_back_input;
        let mut inputs = Inputs::default();
        if let Some(counts) = carried.take(COUNTS) {
            inputs.set_carry(counts);
        }
        let check = DeviceCheck::machine(read, &cx.view.facts.machine);
        let mut state = State::new(check, inputs);
        let mut fx = Effects::none();
        if read {
            fx = fx.order(Order::WatchDevices(true));
        }
        // What the machine says of itself is held from the start.
        let held = state.check_mut().refresh();
        state_with(state, held, fx)
    }

    fn update(state: &mut State, msg: Msg, _cx: &Ctx<'_>) -> Fx {
        match msg {
            Msg::Press(p) => press(state, p),
            Msg::Reveal(gate) => reveal(state, gate),
            Msg::Usb(devices) => {
                let a = state.check_mut().usb(&devices);
                arrived(state, a)
            }
            Msg::Screen(edid) => {
                let a = state.check_mut().screen_changed(edid.as_ref());
                arrived(state, a)
            }
            Msg::BootUsb(names) => {
                if names.is_empty() {
                    return Effects::none();
                }
                Effects::none().note(Said::Plugged(names))
            }
            Msg::NewInput {
                id,
                name,
                keyboard,
                pointer,
                touch,
            } => {
                let kinds = (keyboard, pointer, touch);
                let shown = name.clone();
                match state.inputs_mut().admit(id, name, kinds) {
                    Admitted::Known => Effects::none(),
                    Admitted::Ignored => decided(state).note(Said::Ignoring {
                        alone: state.inputs().alone(),
                    }),
                    Admitted::Held => {
                        let fx = draw(state, id);
                        // A notice only: the sheet comes when the device is used.
                        if keyboard {
                            fx
                        } else {
                            fx.note(Said::NewDeviceHeld(shown))
                        }
                    }
                }
            }
            Msg::Typed { id, ch } => {
                // A key typed on a keyboard takes the notice away; the keyboards stay held.
                if state.inputs().is_held(id) {
                    state.inputs_mut().notice_close();
                }
                let fx = used(state, id).and(draw(state, id));
                let t = state.inputs_mut().typed(id, ch);
                fx.and(tried(state, t, false, id))
            }
            Msg::Tapped { id } => {
                // A device's first touch or press only brings up its sheet: it could not aim at
                // what was not drawn. Where later ones landed is the root's (`Msg::Hit`).
                used(state, id).and(draw(state, id))
            }
            Msg::Hit { id, hit } => {
                if !state.inputs().is_used(id) || state.inputs().shown().is_none_or(|d| d.id != id)
                {
                    return Effects::none();
                }
                if hit == Hit::Beside && !state.target_drawn() {
                    return Effects::none();
                }
                let t = state.inputs_mut().tapped(id, hit);
                tried(state, t, true, id)
            }
            Msg::Room(fits) => {
                state.set_room_fits(fits);
                Effects::none()
            }
            Msg::Aimed(drawn) => {
                state.set_target_drawn(drawn);
                Effects::none()
            }
            Msg::Used(id) => used(state, id).and(draw(state, id)),
            Msg::Gone(id) => {
                state.inputs_mut().gone(id);
                state.ask(id, false);
                input_sheet(state)
            }
            Msg::Trust { pointer, keyboard } => {
                let (_, changed) = state.inputs_mut().set_trust(pointer, keyboard);
                if changed {
                    decided(state).and(input_sheet(state))
                } else {
                    Effects::none()
                }
            }
            Msg::Drawn { id, bytes } => {
                state.ask(id, false);
                // A device that has gone takes no answer; no randomness leaves it waiting for the
                // next time it acts.
                if let Ok(bytes) = bytes {
                    state.inputs_mut().coded(id, bytes.expose());
                }
                Effects::none()
            }
        }
    }

    fn notice(state: &mut State, notice: &Notice<'_>, _cx: &Ctx<'_>) -> Fx {
        match notice {
            // The Devices sheet is not on show any more (Back, a lock's sheet over it): what
            // waited for the check is dropped, and Continue anyway asks again. The New input
            // sheet stays until its device is decided (F202), so this does not touch it.
            Notice::Left => {
                state.wait(None);
                state.check_mut().set_asking(false);
                state.set_devices_up(false);
                Effects::none()
            }
            _ => Effects::none(),
        }
    }

    fn refuses(state: &State, msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        let check = state.check();
        match msg {
            Msg::Press(Press::Continue) if check.refuses() => Some(Refusal::VirtualMachine),
            Msg::Press(Press::LetThrough) if check.refuses() => Some(Refusal::VirtualMachine),
            Msg::Press(Press::LetThrough) if !check.asking() => Some(Refusal::NotAsked),
            Msg::Press(Press::Understood) if !check.held().is_empty() => Some(Refusal::StillHeld),
            _ => None,
        }
    }

    /// The counts of the door: devices kept out, presses beside a target, wrong letters.
    fn carry(state: &mut State, into: &mut Carried) {
        into.put(COUNTS, &state.inputs().carry());
    }
}

/// The state at start with the Devices sheet due when something is held.
fn state_with(mut state: State, _held: bool, fx: Fx) -> (State, Fx) {
    let more = devices_sheet(&mut state);
    (state, fx.and(more))
}
