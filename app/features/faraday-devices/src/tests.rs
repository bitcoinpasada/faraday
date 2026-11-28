//! The device check and the held-back inputs, by what a person or the adapter does.

use faraday_app_test::{Seen, TestView, seen, with_show};
use faraday_kernel::{AtLock, Carried, CarryValue, FrameSheet, Gate, Notice, route};
use faraday_organisms::content::Block;
use faraday_port::bytes::Bytes;
use faraday_port::devices::{Edid, Interface, UsbDevice};
use faraday_port::event::Machine;
use faraday_port::{Order, Text};

use crate::inputs::Inputs;
use crate::{
    COUNTS, Counts, DeviceCheck, Devices, Hit, Msg, Out, Press, Refusal, Reminder, State,
    WRONG_LETTERS_LIMIT,
};
use faraday_kernel::Feature;

/// A machine with an ordinary monitor: its EDID settles that the screen is no TV.
fn machine() -> Machine {
    let mut m = TestView::device().ctx().view.facts.machine.clone();
    m.screen = Some(edid("Monitor"));
    m
}

fn state_on(machine: &Machine) -> State {
    State::new(DeviceCheck::machine(true, machine), Inputs::default())
}

fn run(state: &mut State, msg: Msg) -> Seen<Msg, Out> {
    let view = TestView::device();
    seen(route::<Devices>(state, msg, &view.ctx()))
}

fn usb(name: &str, class: u8, vendor: u16, product: u16) -> UsbDevice {
    UsbDevice {
        port: Text::of("1-2"),
        vendor,
        product,
        manufacturer: Text::new(),
        name: Text::of(name),
        interfaces: vec![Interface {
            class,
            subclass: 0,
            protocol: 1,
        }],
    }
}

/// A KVM's capture side or a TV: an EDID with a monitor-name descriptor.
fn edid(name: &str) -> Edid {
    let mut e = [0u8; 128];
    e[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    e[54 + 3] = 0xfc;
    for (i, b) in name.bytes().enumerate().take(13) {
        e[54 + 5 + i] = b;
    }
    Edid(e)
}

fn fronted(outs: &[Out]) -> bool {
    outs.iter()
        .any(|o| matches!(o, Out::Front(Some(FrameSheet::Devices))))
}

fn cleared(outs: &[Out]) -> Option<Gate> {
    outs.iter().find_map(|o| match o {
        Out::Cleared(g) => Some(*g),
        _ => None,
    })
}

/// Invariant: A virtual machine is held from the start and never let through: Continue anyway and the hold under it are refused, whatever was pressed before.
/// Regression: Continue anyway lets a virtual machine's host see every secret.
/// Owner: app
#[test]
fn a_virtual_machine_is_held_and_never_let_through() {
    let mut m = machine();
    m.hypervisor = true;
    let mut state = state_on(&m);
    let out = run(&mut state, Msg::Usb(vec![])).outs;
    assert!(fronted(&out));
    let view = TestView::device();
    for press in [Press::Continue, Press::LetThrough] {
        assert!(matches!(
            Devices::refuses(&state, &Msg::Press(press), &view.ctx().view),
            Some(Refusal::VirtualMachine)
        ));
    }
    let _ = run(&mut state, Msg::Press(Press::Continue));
    let _ = run(&mut state, Msg::Press(Press::LetThrough));
    assert!(state.check().refuses() && state.check().holding());
}

/// Invariant: A KVM among the USB devices is held and brings the Devices sheet up; it lets through only after Continue anyway asked.
/// Regression: A KVM is let through without the question, or not held.
/// Owner: app
#[test]
fn a_kvm_is_held_and_let_through_only_after_it_asked() {
    let mut state = state_on(&machine());
    let out = run(&mut state, Msg::Usb(vec![usb("PiKVM", 3, 0x1d6b, 0x0104)])).outs;
    assert!(fronted(&out));
    assert!(out.iter().any(|o| matches!(o, Out::Hide)));
    let view = TestView::device();
    assert!(matches!(
        Devices::refuses(&state, &Msg::Press(Press::LetThrough), &view.ctx().view),
        Some(Refusal::NotAsked)
    ));
    let _ = run(&mut state, Msg::Press(Press::Continue));
    let _ = run(&mut state, Msg::Press(Press::LetThrough));
    assert!(!state.check().holding());
}

/// Invariant: A screen whose EDID names a TV is held, and one swapped in after it was let through is held again.
/// Regression: A TV is let through once and then trusted for every screen after it.
/// Owner: app
#[test]
fn a_tv_screen_is_held_and_held_again_when_swapped() {
    let mut m = machine();
    m.chassis = Some(3);
    let mut state = state_on(&m);
    let _ = run(&mut state, Msg::Screen(Some(edid("Samsung TV"))));
    assert!(state.check().holding());
    let _ = run(&mut state, Msg::Press(Press::Continue));
    let _ = run(&mut state, Msg::Press(Press::LetThrough));
    assert!(!state.check().holding());
    let _ = run(&mut state, Msg::Screen(Some(edid("LG TV"))));
    assert!(state.check().holding());
}

/// Invariant: A new keyboard, pointer or touchscreen is held with no code until the session's randomness comes, and typing without a code counts for nothing.
/// Regression: An unknown input is believed for the name it gives, or typing before the code counts toward the door.
/// Owner: app
#[test]
fn an_unknown_input_is_held_and_typing_without_a_code_counts_nothing() {
    let mut state = state_on(&machine());
    state
        .inputs_mut()
        .admit(7, Text::of("Keyboard"), (true, false, false));
    // The draw is on its way: the randomness has not answered.
    state.ask(7, true);
    assert!(state.inputs().needs_code(7));
    for _ in 0..(WRONG_LETTERS_LIMIT + 5) {
        let r = run(&mut state, Msg::Typed { id: 7, ch: 'a' });
        assert!(
            r.orders
                .iter()
                .all(|o| !matches!(o, Order::DecideInput { .. }))
        );
    }
    assert!(state.inputs().ignored().is_empty());
    assert!(!state.inputs().closed());
}

fn drawn(state: &mut State, id: u32) {
    let bytes = Bytes::new(vec![0, 1, 2, 3, 4, 5].into_iter().chain(0..64).collect());
    let _ = run(
        state,
        Msg::Drawn {
            id,
            bytes: Ok(bytes),
        },
    );
}

/// A held device with its code, as if the session's randomness had answered: the kernel's
/// session changes are not built, so the draw itself is not asked here.
fn admit(state: &mut State, id: u32, kinds: (bool, bool, bool)) {
    state.inputs_mut().admit(id, Text::of("Device"), kinds);
    state.ask(id, true);
    drawn(state, id);
}

fn keyboard(state: &mut State, id: u32) {
    admit(state, id, (true, false, false));
}

/// Invariant: A keyboard that types the code shown is believed and the adapter is told so; one that types three codes' worth of letters without it is kept out, and the door closes after three.
/// Regression: A keyboard is believed without the code, or may try codes one after another.
/// Owner: app
#[test]
fn the_code_lets_a_keyboard_in_and_the_wrong_letters_keep_it_out() {
    let mut state = state_on(&machine());
    keyboard(&mut state, 1);
    let mut believed = false;
    for ch in "acdefh".chars() {
        let r = run(&mut state, Msg::Typed { id: 1, ch });
        believed |= r
            .orders
            .iter()
            .any(|o| matches!(o, Order::DecideInput { believed: true, .. }));
    }
    assert!(believed);

    for id in 2..5 {
        keyboard(&mut state, id);
        let mut out = false;
        for _ in 0..18 {
            let r = run(&mut state, Msg::Typed { id, ch: 'z' });
            out |= r.orders.iter().any(|o| {
                matches!(
                    o,
                    Order::DecideInput {
                        believed: false,
                        ..
                    }
                )
            });
        }
        assert!(out, "keyboard {id} is kept out after 18 wrong letters");
    }
    assert!(state.inputs().closed());
    let r = run(
        &mut state,
        Msg::NewInput {
            id: 9,
            name: Text::of("Another"),
            keyboard: true,
            pointer: false,
            touch: false,
        },
    );
    assert!(r.orders.iter().any(|o| matches!(
        o,
        Order::DecideInput {
            believed: false,
            ..
        }
    )));
}

/// Invariant: A touchscreen is let in only by pressing its targets in a row; a press beside restarts the run and three beside keep it out.
/// Regression: One lucky press lets a blind touchscreen in.
/// Owner: app
#[test]
fn a_touchscreen_needs_every_target_and_three_misses_keep_it_out() {
    let mut state = state_on(&machine());
    admit(&mut state, 3, (false, false, true));
    let _ = run(&mut state, Msg::Used(3));
    let _ = run(&mut state, Msg::Aimed(true));
    let one = run(
        &mut state,
        Msg::Hit {
            id: 3,
            hit: Hit::Target { step: 0, of: 3 },
        },
    );
    assert!(
        one.orders
            .iter()
            .all(|o| !matches!(o, Order::DecideInput { .. }))
    );
    let mut kept = false;
    for _ in 0..3 {
        let r = run(
            &mut state,
            Msg::Hit {
                id: 3,
                hit: Hit::Beside,
            },
        );
        kept |= r.orders.iter().any(|o| {
            matches!(
                o,
                Order::DecideInput {
                    believed: false,
                    ..
                }
            )
        });
    }
    assert!(kept);
}

/// Invariant: A press that would show a secret clears at once with nothing held, waits for the Devices sheet while something is, and is answered when the item is pulled and checked again.
/// Regression: A secret is shown with a device held, or a press waits with nothing to wait for.
/// Owner: app
#[test]
fn a_reveal_waits_only_while_something_is_held() {
    let mut state = state_on(&machine());
    let out = run(&mut state, Msg::Reveal(Gate(1))).outs;
    assert_eq!(cleared(&out), Some(Gate(1)));

    let _ = run(&mut state, Msg::Usb(vec![usb("PiKVM", 3, 0x1d6b, 0x0104)]));
    let out = run(&mut state, Msg::Reveal(Gate(2))).outs;
    assert_eq!(cleared(&out), None);
    // A second press replaces the first.
    let _ = run(&mut state, Msg::Reveal(Gate(3)));
    // The device is pulled, but stays named until Check again.
    let _ = run(&mut state, Msg::Usb(vec![]));
    assert!(state.check().holding());
    let out = run(&mut state, Msg::Press(Press::Check)).outs;
    assert_eq!(cleared(&out), Some(Gate(3)));
}

/// Invariant: What waited for the check is dropped when the sheet is replaced or cancelled, never answered later.
/// Regression: A press waits under a sheet that is gone and shows its secret after a lock.
/// Owner: app
#[test]
fn a_waiting_press_is_dropped_when_the_sheet_goes() {
    let mut state = state_on(&machine());
    let _ = run(&mut state, Msg::Usb(vec![usb("PiKVM", 3, 0x1d6b, 0x0104)]));
    let _ = run(&mut state, Msg::Reveal(Gate(1)));
    let view = TestView::device();
    let _ = seen(<Devices as Feature>::notice(
        &mut state,
        &Notice::Left,
        &view.ctx(),
    ));
    assert_eq!(state.waiting(), None);
    let _ = run(&mut state, Msg::Reveal(Gate(2)));
    let out = run(&mut state, Msg::Press(Press::Cancel)).outs;
    assert_eq!(cleared(&out), None);
    assert_eq!(state.waiting(), None);
}

/// Invariant: A wireless receiver holds nothing, but the first press that would show a secret waits once for I understand, and not again.
/// Regression: A receiver blocks every secret, or never reminds.
/// Owner: app
#[test]
fn a_receiver_reminds_once() {
    let mut state = state_on(&machine());
    let out = run(
        &mut state,
        Msg::Usb(vec![usb("Wireless Receiver", 3, 0x046d, 0xc52b)]),
    )
    .outs;
    assert!(!fronted(&out));
    let out = run(&mut state, Msg::Reveal(Gate(1))).outs;
    assert_eq!(cleared(&out), None);
    let out = run(&mut state, Msg::Press(Press::Understood)).outs;
    assert_eq!(cleared(&out), Some(Gate(1)));
    let out = run(&mut state, Msg::Reveal(Gate(2))).outs;
    assert_eq!(cleared(&out), Some(Gate(2)));
}

/// Invariant: A held keyboard's code and what it typed are gone after a lock, and a lock carries only the door's counts, clamped to their limits.
/// Regression: A code outlives the lock, or an edited file raises a count past its limit.
/// Owner: app
#[test]
fn a_lock_wipes_the_held_codes_and_carries_clamped_counts() {
    let mut state = state_on(&machine());
    keyboard(&mut state, 1);
    let _ = run(&mut state, Msg::Used(1));
    let _ = run(&mut state, Msg::Typed { id: 1, ch: 'a' });
    assert!(state.inputs().shown().is_some_and(|d| d.code().is_some()));
    assert!(state.holds().typed);
    state.at_lock();
    assert!(state.inputs().shown().is_none());
    assert!(!state.holds().typed);

    let big = Counts::decode(&[255, 255, 255]);
    assert_eq!(
        big.map(|c| (c.kept_out, c.beside, c.wrong)),
        Some((3, 9, 54))
    );
    assert!(Counts::decode(&[1, 2]).is_none());
    let mut carried = Carried::empty();
    let mut inputs = Inputs::default();
    inputs.set_carry(Counts {
        kept_out: 2,
        beside: 4,
        wrong: 6,
    });
    carried.put(COUNTS, &inputs.carry());
    let back = carried.take::<Counts>(COUNTS);
    assert_eq!(back.map(|c| c.kept_out), Some(2));
}

/// What the New input sheet says and offers for the device it shows, on a display `w` by `h`:
/// its words, and its buttons' labels with whether each is offered.
fn sheet_of(state: &State, w: u16, h: u16) -> (Vec<String>, Vec<String>) {
    let view = TestView::device().with_display(w, h);
    let ctx = view.ctx();
    with_show(|show| {
        let model = Devices::present_input(state, &ctx.view, show);
        let Some(dialog) = model.sheet else {
            return (Vec::new(), Vec::new());
        };
        let said = dialog
            .sheet
            .body
            .0
            .iter()
            .filter_map(|b| match b {
                Block::Heading(s) | Block::Headline(s) => Some(s.as_str().to_string()),
                _ => None,
            })
            .collect();
        let foot = dialog
            .sheet
            .foot
            .items
            .iter()
            .map(|b| b.label.as_str().to_string())
            .collect();
        (said, foot)
    })
}

/// Invariant: A pointer is let in by its Use this pointer button only where a trusted pointer exists to press it; a pointer let in by targets has Ignore and the target, and a new mouse beside a trusted pointer can be let in.
/// Regression: Use this pointer was offered in the opposite case, so a mouse beside a trusted pointer could never be let in and a lone pointer was let in by a keyboard's Tab and Enter.
/// Owner: app
#[test]
fn use_this_pointer_is_offered_only_beside_a_trusted_pointer() {
    let ignore = faraday_strings::EN.new_input_ignore_button;
    let use_it = faraday_strings::EN.new_input_use_pointer;
    // A trusted pointer exists: the new mouse has Ignore and Use, and Use lets it in.
    let mut state = state_on(&machine());
    state.inputs_mut().set_trust(true, true);
    admit(&mut state, 5, (false, true, false));
    let _ = run(&mut state, Msg::Used(5));
    let (_, foot) = sheet_of(&state, 1024, 600);
    assert_eq!(foot, [ignore, use_it]);
    let r = run(&mut state, Msg::Press(Press::Use(5)));
    assert!(
        r.orders
            .iter()
            .any(|o| matches!(o, Order::DecideInput { believed: true, .. }))
    );

    // Nothing trusted can point: the pointer is let in by targets, with Ignore and the target.
    let mut alone = state_on(&machine());
    alone.inputs_mut().set_trust(false, true);
    admit(&mut alone, 6, (false, true, false));
    let _ = run(&mut alone, Msg::Used(6));
    let (_, foot) = sheet_of(&alone, 1024, 600);
    assert_eq!(foot, [ignore]);
}

/// Invariant: Check again sends the waiting press through the hold again: while a reminder is still due it stays held under the sheet, which shows the reminder and I understand.
/// Regression: Check again answered the waiting press as soon as nothing was named, so a wireless receiver's reminder was never shown.
/// Owner: app
#[test]
fn check_again_holds_the_press_while_a_reminder_is_due() {
    let mut state = state_on(&machine());
    let kvm = usb("PiKVM", 3, 0x1d6b, 0x0104);
    let receiver = UsbDevice {
        port: Text::of("1-3"),
        ..usb("Wireless Receiver", 3, 0x046d, 0xc52b)
    };
    let _ = run(&mut state, Msg::Usb(vec![kvm, receiver.clone()]));
    let _ = run(&mut state, Msg::Reveal(Gate(1)));
    // The KVM is pulled; the receiver stays.
    let _ = run(&mut state, Msg::Usb(vec![receiver]));
    let out = run(&mut state, Msg::Press(Press::Check)).outs;
    assert_eq!(cleared(&out), None);
    assert_eq!(state.waiting(), Some(Gate(1)));
    assert!(state.devices_up());
    let view = TestView::device();
    let ctx = view.ctx();
    let (said, foot) = with_show(|show| {
        let dialog = Devices::present_devices(&state, &ctx.view, show)
            .sheet
            .unwrap();
        let said: Vec<String> = dialog
            .sheet
            .body
            .0
            .iter()
            .filter_map(|b| match b {
                Block::Heading(s) => Some(s.as_str().to_string()),
                _ => None,
            })
            .collect();
        let foot: Vec<String> = dialog
            .sheet
            .foot
            .items
            .iter()
            .map(|b| b.label.as_str().to_string())
            .collect();
        (said, foot)
    });
    assert!(said.iter().any(|s| s == Reminder::Receiver.line()));
    assert!(
        foot.iter()
            .any(|s| s == faraday_strings::EN.devices_understood)
    );
    let out = run(&mut state, Msg::Press(Press::Understood)).outs;
    assert_eq!(cleared(&out), Some(Gate(1)));
}

/// Invariant: A target is counted from the least room the New input dialog can leave, so a blind device's chance is no better than a keyboard's on any display.
/// Regression: The count came from half the display, three targets on a 1920 by 1080 display, where a 280-wide dialog needs more.
/// Owner: app
#[test]
fn the_target_count_comes_from_the_least_dialog() {
    let mut state = state_on(&machine());
    state.inputs_mut().set_trust(false, false);
    admit(&mut state, 3, (false, false, true));
    let _ = run(&mut state, Msg::Used(3));
    let step = |w, h| {
        let (said, _) = sheet_of(&state, w, h);
        let line = said.iter().find(|s| s.starts_with("Target 1 of")).cloned();
        line.and_then(|l| l.rsplit(' ').next().and_then(|n| n.parse::<usize>().ok()))
    };
    // A dialog 232 wide inside its padding leaves 184 by 144 for the target: 7 in a row.
    assert_eq!(step(1920, 1080), Some(7));
    assert_eq!(step(1024, 600), Some(7));
}

/// Invariant: A press beside the target counts only when the last frame drew a target of that device; with none drawn, presses beside count for nothing.
/// Regression: A device whose target could not be drawn was kept out and the door closed by presses beside nothing.
/// Owner: app
#[test]
fn a_press_beside_counts_only_with_a_target_drawn() {
    let mut state = state_on(&machine());
    admit(&mut state, 3, (false, false, true));
    let _ = run(&mut state, Msg::Used(3));
    for _ in 0..5 {
        let r = run(
            &mut state,
            Msg::Hit {
                id: 3,
                hit: Hit::Beside,
            },
        );
        assert!(r.orders.is_empty());
    }
    assert!(!state.inputs().closed());
    assert!(state.inputs().ignored().is_empty());
    let _ = run(&mut state, Msg::Aimed(true));
    let mut kept = false;
    for _ in 0..3 {
        let r = run(
            &mut state,
            Msg::Hit {
                id: 3,
                hit: Hit::Beside,
            },
        );
        kept |= !r.orders.is_empty();
    }
    assert!(kept);
}

/// Invariant: A keyboard held back is named by a notice that stays until a key is typed on a keyboard or its x is pressed, whatever else happens meanwhile.
/// Regression: A keyboard plugged in after start and typing nothing was never mentioned to the person.
/// Owner: app
#[test]
fn a_held_keyboard_is_named_until_a_key_or_the_x() {
    let mut state = state_on(&machine());
    let words = |state: &State| {
        with_show(|show| Devices::stays(state, show).map(|t| t.as_str().to_string()))
    };
    assert_eq!(words(&state), None);
    state
        .inputs_mut()
        .admit(7, Text::of("Acme Keys"), (true, false, false));
    let named = words(&state).unwrap_or_default();
    assert!(named.contains("Acme Keys"), "{named}");
    // Time and other devices pass: it stays.
    let _ = run(&mut state, Msg::Usb(vec![usb("PiKVM", 3, 0x1d6b, 0x0104)]));
    let _ = run(
        &mut state,
        Msg::Trust {
            pointer: true,
            keyboard: true,
        },
    );
    assert_eq!(words(&state).as_deref(), Some(named.as_str()));
    // The x closes it; the keyboard stays held.
    let _ = run(&mut state, Msg::Press(Press::CloseNotice));
    assert_eq!(words(&state), None);
    assert!(state.inputs().is_held(7));

    // A key typed on a keyboard closes it too.
    let mut typing = state_on(&machine());
    keyboard(&mut typing, 1);
    assert!(words(&typing).is_some());
    let _ = run(&mut typing, Msg::Typed { id: 1, ch: 'a' });
    assert_eq!(words(&typing), None);
}
