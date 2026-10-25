//! Mice and touchpads: packets in, cursor motion out.
//!
//! A pointer is a touch at a place the person aims with. The core has no
//! pointer and needs none: the cursor lives here, `BTN_LEFT` down, the
//! moves while it is down, and `BTN_LEFT` up are
//! [`Event::Touch`](faraday_shell_api::Event::Touch) Down, Move and Up at the
//! cursor, and the wheel is [`Event::Scroll`](faraday_shell_api::Event::Scroll)
//! there. The thumb buttons (`BTN_SIDE` and `BTN_BACK`, `BTN_EXTRA` and
//! `BTN_FORWARD`) are the core's
//! [`Event::Back`](faraday_shell_api::Event::Back) and
//! [`Event::Forward`](faraday_shell_api::Event::Forward), on the press
//! (`docs/history/NEW-WALLET.md` §13.1). Every screen therefore works with a mouse for the same reason
//! it works with a finger, and nothing in the core knows the difference.
//!
//! No acceleration: one device count moves the cursor by the arrow's scale
//! (`cursor::scale`), times `--pointer-speed` (`docs/MOTION.md` §3.2).
//! A signing device is not a desktop and there is no muscle memory to
//! honour; predictable is what a person aiming at a 48 dp row wants.
//!
//! A touchpad moves the cursor by the change in its reported finger
//! position while `BTN_TOUCH` is set, so lifting and replacing a finger
//! moves nothing. Its button is `BTN_LEFT`; a pad that has none clicks
//! when a finger lands (`BTN_TOUCH` with `BTN_TOOL_FINGER`), which makes
//! every stroke on such a pad a drag, the way a finger on the panel is.
//!
//! A pad that has a button behaves as a laptop's does under libinput and
//! Windows (`docs/MOTION.md` §3.9). A short touch that hardly moves is a
//! tap, which the cursor turns into a click, and a tap followed quickly
//! by a touch that moves is a drag. While two fingers are down and the
//! button is not held, their vertical travel scrolls instead of moving
//! the cursor. While the button is held, every finger moves the cursor
//! and none scrolls: a finger resting on a clickpad's click and a second
//! one moving beside it drag, and the pointer follows the finger that
//! moved.
//!
//! Positions are followed contact by contact through the multitouch
//! slots (`ABS_MT_SLOT`, `ABS_MT_TRACKING_ID`, `ABS_MT_POSITION_*`)
//! once the device has reported any; in each packet the contact that
//! travelled furthest is the one that moves the pointer, and a contact's
//! first position moves nothing, so a finger landing or lifting never
//! makes the cursor jump. A device that reports no multitouch axes is
//! followed by `ABS_X`/`ABS_Y`, the kernel's single-touch emulation,
//! which tracks the oldest contact.

use crate::Wake;
use crate::evdev::{
    ABS_MT_POSITION_X, ABS_MT_POSITION_Y, ABS_MT_SLOT, ABS_MT_TRACKING_ID, ABS_X, ABS_Y, BTN_BACK,
    BTN_EXTRA, BTN_FORWARD, BTN_LEFT, BTN_SIDE, BTN_TOOL_DOUBLETAP, BTN_TOOL_FINGER, BTN_TOUCH,
    EV_ABS, EV_KEY, EV_REL, EV_SYN, REL_WHEEL, REL_WHEEL_HI_RES, REL_X, REL_Y, RawEvent,
    SYN_REPORT,
};

/// `REL_WHEEL_HI_RES` units in one notch of the wheel.
const HI_RES_PER_NOTCH: i32 = 120;

/// The most contacts followed at once. A pad reports up to five; a slot
/// past this is ignored rather than grown into.
const SLOTS: usize = 10;

/// What one contact on a buttoned touchpad did, which is everything the
/// cursor needs to decide whether it was a tap. The travel is in frame
/// pixels, which only the cursor knows, so the decision is split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finger {
    /// A finger landed.
    Down {
        /// The kernel's timestamp, in milliseconds.
        ms: u64,
    },
    /// The finger lifted.
    Up {
        /// The kernel's timestamp, in milliseconds.
        ms: u64,
    },
    /// The touch cannot be a tap any more: the button clicked under it,
    /// or a second finger joined it.
    Cancel,
}

/// What a pointing device said, before the cursor turns it into a touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// The pointer moved by this many device units.
    Move {
        /// Units to the right.
        dx: i32,
        /// Units down.
        dy: i32,
    },
    /// Two fingers travelled this far on the pad, in device units, with
    /// the button not held. The cursor turns the vertical travel into a
    /// scroll that moves the content with the fingers, as a finger on a
    /// touchscreen does, or, while a tap holds the button down, moves
    /// the pointer by both.
    Pan {
        /// Units to the right.
        dx: i32,
        /// Units toward the person.
        dy: i32,
    },
    /// The two fingers that were panning lifted, or the button went down
    /// under them.
    PanEnd,
    /// The button went down (`true`) or came up (`false`).
    Button(bool),
    /// One contact on a buttoned pad. It never moves the cursor.
    Finger(Finger),
    /// The wheel turned this many 120ths of a notch; positive is away
    /// from the person, which scrolls the content down. A wheel that
    /// clicks reports whole notches, 120 at a time; a fine one reports
    /// the fractions it turned.
    Wheel(i32),
    /// The back thumb button went down.
    Back,
    /// The forward thumb button went down.
    Forward,
}

/// One multitouch slot: a contact the pad is following.
#[derive(Debug, Clone, Copy, Default)]
struct Slot {
    /// Where the contact is now, axis by axis: a driver sends only the
    /// axis that changed.
    at: (i32, i32),
    /// Where it was at the last packet, so its travel is a delta; `None`
    /// for a contact that has just landed.
    last: Option<(i32, i32)>,
    /// This packet reported a position for it.
    new: bool,
}

/// One pointing device's packet state machine.
#[derive(Debug)]
pub struct Pointer {
    /// The wheel reports 120ths of a notch, so the whole-notch axis is
    /// ignored and a turn is not counted twice.
    hi_res: bool,
    /// The device has no button, so a finger landing on it is the click.
    /// A pad that has one reports its touches instead, and the cursor
    /// decides whether one of them was a tap.
    tap_clicks: bool,
    /// The device has `ABS_X`/`ABS_Y`, which are followed until it
    /// reports a multitouch position.
    abs_single: bool,
    dx: i32,
    dy: i32,
    notches: i32,
    fine: i32,
    /// The last single-touch position the device reported, axis by axis.
    abs: (i32, i32),
    /// This packet reported a single-touch position.
    abs_new: bool,
    /// Where the single-touch finger was at the last packet.
    last_abs: Option<(i32, i32)>,
    /// The device has reported a multitouch position, so contacts are
    /// followed slot by slot and `ABS_X`/`ABS_Y` are ignored.
    multitouch: bool,
    /// The contacts, by slot.
    slots: [Slot; SLOTS],
    /// Device units two fingers have travelled since the last packet
    /// that reported any.
    pan: (i32, i32),
    /// Two fingers have panned since they landed.
    panned: bool,
    /// The fingers that panned lifted in this packet.
    pan_ended: bool,
    /// The slot the `ABS_MT_*` events now belong to.
    slot: usize,
    touching: bool,
    finger: bool,
    /// Two fingers are on the pad: `BTN_TOOL_DOUBLETAP`.
    two: bool,
    /// This touch has already been cancelled, so it is cancelled once.
    cancelled: bool,
    /// `BTN_LEFT` is held down.
    left: bool,
    /// What this touch did, in the order it did it.
    fingers: Vec<Finger>,
    button: Option<bool>,
}

impl Pointer {
    /// A parser for a device whose wheel is `hi_res`, which clicks by
    /// tapping when `tap_clicks`, and which reports `ABS_X`/`ABS_Y`
    /// when `abs_single`.
    pub fn new(hi_res: bool, tap_clicks: bool, abs_single: bool) -> Pointer {
        Pointer {
            hi_res,
            tap_clicks,
            abs_single,
            dx: 0,
            dy: 0,
            notches: 0,
            fine: 0,
            abs: (0, 0),
            abs_new: false,
            last_abs: None,
            multitouch: false,
            slots: [Slot::default(); SLOTS],
            pan: (0, 0),
            panned: false,
            pan_ended: false,
            slot: 0,
            touching: false,
            finger: false,
            two: false,
            cancelled: false,
            left: false,
            fingers: Vec::new(),
            button: None,
        }
    }

    /// Ends this touch's chance of being a tap, once.
    fn cancel(&mut self) {
        if self.touching && !self.cancelled && !self.tap_clicks {
            self.cancelled = true;
            self.fingers.push(Finger::Cancel);
        }
    }

    /// The slot the multitouch events now belong to, when it is one this
    /// parser follows.
    fn slot_mut(&mut self) -> Option<&mut Slot> {
        self.slots.get_mut(self.slot)
    }

    /// Records one multitouch position axis for the current slot.
    fn mt_position(&mut self, x: bool, value: i32) {
        self.multitouch = true;
        if let Some(slot) = self.slot_mut() {
            if x {
                slot.at.0 = value;
            } else {
                slot.at.1 = value;
            }
            slot.new = true;
        }
    }

    /// Feeds one raw event, appending a [`Wake::Pointer`] for everything
    /// the packet it closed said.
    pub fn feed(&mut self, ev: RawEvent, out: &mut Vec<Wake>) {
        match ev.kind {
            EV_REL => match ev.code {
                REL_X => self.dx += ev.value,
                REL_Y => self.dy += ev.value,
                REL_WHEEL if !self.hi_res => self.notches += ev.value,
                REL_WHEEL_HI_RES if self.hi_res => self.fine += ev.value,
                _ => {}
            },
            EV_ABS => match ev.code {
                ABS_MT_SLOT => self.slot = usize::try_from(ev.value).unwrap_or(SLOTS),
                ABS_MT_TRACKING_ID => {
                    // A new contact, or one that lifted: either way it
                    // has no last position to travel from.
                    if let Some(slot) = self.slot_mut() {
                        slot.last = None;
                        slot.new = false;
                    }
                }
                ABS_MT_POSITION_X => self.mt_position(true, ev.value),
                ABS_MT_POSITION_Y => self.mt_position(false, ev.value),
                ABS_X if self.abs_single => {
                    self.abs.0 = ev.value;
                    self.abs_new = true;
                }
                ABS_Y if self.abs_single => {
                    self.abs.1 = ev.value;
                    self.abs_new = true;
                }
                _ => {}
            },
            EV_KEY => match ev.code {
                BTN_LEFT => {
                    self.left = ev.value != 0;
                    self.button = Some(self.left);
                    if self.left {
                        self.cancel();
                        // Two fingers that were scrolling stop when the
                        // button goes down under them: from here they
                        // move the pointer.
                        self.pan_ended |= std::mem::take(&mut self.panned);
                    }
                }
                BTN_TOUCH => {
                    let down = ev.value != 0;
                    if self.touching != down {
                        self.touching = down;
                        if !down {
                            self.last_abs = None;
                            for slot in &mut self.slots {
                                slot.last = None;
                            }
                            self.pan_ended |= std::mem::take(&mut self.panned);
                        }
                        if self.tap_clicks && (self.finger || !down) {
                            self.button = Some(down);
                        } else if !self.tap_clicks {
                            if down {
                                self.cancelled = false;
                                self.fingers.push(Finger::Down { ms: ev.time_ms });
                                // A touch that starts under a second
                                // finger, or under a button already
                                // held, is not a tap.
                                if self.two || self.left {
                                    self.cancel();
                                }
                            } else {
                                self.fingers.push(Finger::Up { ms: ev.time_ms });
                            }
                        }
                    }
                }
                // A thumb button acts on its press; its release and the
                // kernel's repeats do nothing.
                BTN_SIDE | BTN_BACK if ev.value == 1 => out.push(Wake::Pointer(Motion::Back)),
                BTN_EXTRA | BTN_FORWARD if ev.value == 1 => {
                    out.push(Wake::Pointer(Motion::Forward));
                }
                BTN_TOOL_FINGER => self.finger = ev.value != 0,
                BTN_TOOL_DOUBLETAP => {
                    let two = ev.value != 0;
                    if self.two != two {
                        self.two = two;
                        // The two fingers are at two places and the
                        // single-touch axes report one of them: the jump
                        // from the one to the other moves nothing. The
                        // slots follow each contact and need no reset.
                        self.last_abs = None;
                        if two {
                            self.cancel();
                        } else {
                            self.pan_ended |= std::mem::take(&mut self.panned);
                        }
                    }
                }
                _ => {}
            },
            EV_SYN if ev.code == SYN_REPORT => self.sync(out),
            _ => {}
        }
    }

    /// The travel this packet reported: the contact that moved furthest,
    /// or the single-touch finger on a device without slots.
    fn travel(&mut self) -> Option<(i32, i32)> {
        if self.multitouch {
            let mut best: Option<(i32, i32)> = None;
            for slot in &mut self.slots {
                if !std::mem::take(&mut slot.new) {
                    continue;
                }
                if let Some((lx, ly)) = slot.last
                    && self.touching
                {
                    let d = (slot.at.0 - lx, slot.at.1 - ly);
                    let far = |d: (i32, i32)| d.0.abs() + d.1.abs();
                    if best.is_none_or(|b| far(d) > far(b)) {
                        best = Some(d);
                    }
                }
                slot.last = self.touching.then_some(slot.at);
            }
            self.abs_new = false;
            best
        } else if std::mem::take(&mut self.abs_new) && self.touching {
            let d = self
                .last_abs
                .map(|(lx, ly)| (self.abs.0 - lx, self.abs.1 - ly));
            self.last_abs = Some(self.abs);
            d
        } else {
            None
        }
    }

    /// End of packet: the move first, so a click in the same packet lands
    /// where the pointer now is.
    fn sync(&mut self, out: &mut Vec<Wake>) {
        if let Some((dx, dy)) = self.travel() {
            // Two fingers scroll only while nothing holds the button;
            // under a held click every finger moves the pointer.
            if self.two && !self.left {
                self.pan.0 += dx;
                self.pan.1 += dy;
            } else {
                self.dx += dx;
                self.dy += dy;
            }
        }
        if (self.dx, self.dy) != (0, 0) {
            out.push(Wake::Pointer(Motion::Move {
                dx: self.dx,
                dy: self.dy,
            }));
            self.dx = 0;
            self.dy = 0;
        }
        if self.pan != (0, 0) {
            out.push(Wake::Pointer(Motion::Pan {
                dx: self.pan.0,
                dy: self.pan.1,
            }));
            self.pan = (0, 0);
            self.panned = true;
        }
        if std::mem::take(&mut self.pan_ended) {
            self.panned = false;
            out.push(Wake::Pointer(Motion::PanEnd));
        }
        for finger in self.fingers.drain(..) {
            out.push(Wake::Pointer(Motion::Finger(finger)));
        }
        if let Some(down) = self.button.take() {
            out.push(Wake::Pointer(Motion::Button(down)));
        }
        let turned = self.notches * HI_RES_PER_NOTCH + self.fine;
        if turned != 0 {
            out.push(Wake::Pointer(Motion::Wheel(turned)));
            self.notches = 0;
            self.fine = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(time_ms: u64, kind: u16, code: u16, value: i32) -> RawEvent {
        RawEvent {
            time_ms,
            kind,
            code,
            value,
        }
    }

    fn ev(kind: u16, code: u16, value: i32) -> RawEvent {
        at(0, kind, code, value)
    }

    fn syn() -> RawEvent {
        ev(EV_SYN, SYN_REPORT, 0)
    }

    fn syn_at(time_ms: u64) -> RawEvent {
        at(time_ms, EV_SYN, SYN_REPORT, 0)
    }

    fn run(pointer: &mut Pointer, events: &[RawEvent]) -> Vec<Motion> {
        let mut out = Vec::new();
        for e in events {
            pointer.feed(*e, &mut out);
        }
        out.into_iter()
            .map(|w| match w {
                Wake::Pointer(m) => m,
                other => panic!("a pointer produced {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_mouse_moves_clicks_and_scrolls() {
        let mut mouse = Pointer::new(false, false, false);
        assert_eq!(
            run(
                &mut mouse,
                &[
                    ev(EV_REL, REL_X, 20),
                    ev(EV_REL, REL_Y, -5),
                    syn(),
                    ev(EV_KEY, BTN_LEFT, 1),
                    syn(),
                    ev(EV_KEY, BTN_LEFT, 0),
                    syn(),
                    ev(EV_REL, REL_WHEEL, -1),
                    syn(),
                ],
            ),
            vec![
                Motion::Move { dx: 20, dy: -5 },
                Motion::Button(true),
                Motion::Button(false),
                Motion::Wheel(-120),
            ]
        );
    }

    #[test]
    fn the_thumb_buttons_are_back_and_forward_on_the_press() {
        let mut mouse = Pointer::new(false, false, false);
        assert_eq!(
            run(
                &mut mouse,
                &[
                    ev(EV_KEY, BTN_SIDE, 1),
                    syn(),
                    ev(EV_KEY, BTN_SIDE, 2),
                    syn(),
                    ev(EV_KEY, BTN_SIDE, 0),
                    syn(),
                    ev(EV_KEY, BTN_EXTRA, 1),
                    syn(),
                    ev(EV_KEY, BTN_EXTRA, 0),
                    syn(),
                    ev(EV_KEY, BTN_BACK, 1),
                    syn(),
                    ev(EV_KEY, BTN_FORWARD, 1),
                    syn(),
                ],
            ),
            vec![Motion::Back, Motion::Forward, Motion::Back, Motion::Forward],
            "one each per press, none for a repeat or a release"
        );
    }

    #[test]
    fn the_thumb_buttons_reach_the_core_as_back_and_forward() {
        use crate::cursor::Pointed;
        use faraday_shell_api::Event;
        assert_eq!(
            Pointed::Back.events().collect::<Vec<_>>(),
            vec![Event::Back]
        );
        assert_eq!(
            Pointed::Forward.events().collect::<Vec<_>>(),
            vec![Event::Forward]
        );
    }

    #[test]
    fn a_fine_wheel_is_counted_once_and_in_its_fractions() {
        let mut mouse = Pointer::new(true, false, false);
        // The whole-notch axis a fine wheel also sends is ignored.
        assert_eq!(
            run(
                &mut mouse,
                &[
                    ev(EV_REL, REL_WHEEL_HI_RES, 60),
                    ev(EV_REL, REL_WHEEL, 1),
                    syn(),
                ],
            ),
            vec![Motion::Wheel(60)],
            "half a notch turns half a notch"
        );
        assert_eq!(
            run(&mut mouse, &[ev(EV_REL, REL_WHEEL_HI_RES, 60), syn()],),
            vec![Motion::Wheel(60)]
        );
    }

    #[test]
    fn a_touchpad_moves_by_the_fingers_travel_and_clicks_with_its_button() {
        let mut pad = Pointer::new(false, false, true);
        assert_eq!(
            run(
                &mut pad,
                &[
                    ev(EV_KEY, BTN_TOOL_FINGER, 1),
                    ev(EV_KEY, BTN_TOUCH, 1),
                    ev(EV_ABS, ABS_X, 1000),
                    ev(EV_ABS, ABS_Y, 500),
                    syn(),
                    ev(EV_ABS, ABS_X, 1030),
                    ev(EV_ABS, ABS_Y, 490),
                    syn(),
                    ev(EV_KEY, BTN_LEFT, 1),
                    syn(),
                    ev(EV_KEY, BTN_LEFT, 0),
                    ev(EV_KEY, BTN_TOUCH, 0),
                    syn(),
                    // A finger set down somewhere else does not jump.
                    ev(EV_KEY, BTN_TOUCH, 1),
                    ev(EV_ABS, ABS_X, 200),
                    ev(EV_ABS, ABS_Y, 900),
                    syn(),
                ],
            ),
            vec![
                Motion::Finger(Finger::Down { ms: 0 }),
                Motion::Move { dx: 30, dy: -10 },
                Motion::Finger(Finger::Cancel),
                Motion::Button(true),
                Motion::Finger(Finger::Up { ms: 0 }),
                Motion::Button(false),
                Motion::Finger(Finger::Down { ms: 0 }),
            ]
        );
    }

    #[test]
    fn a_touchpad_with_no_button_clicks_when_a_finger_lands() {
        let mut pad = Pointer::new(false, true, true);
        assert_eq!(
            run(
                &mut pad,
                &[
                    ev(EV_KEY, BTN_TOOL_FINGER, 1),
                    ev(EV_KEY, BTN_TOUCH, 1),
                    ev(EV_ABS, ABS_X, 100),
                    ev(EV_ABS, ABS_Y, 100),
                    syn(),
                    ev(EV_ABS, ABS_X, 140),
                    syn(),
                    ev(EV_KEY, BTN_TOUCH, 0),
                    ev(EV_KEY, BTN_TOOL_FINGER, 0),
                    syn(),
                ],
            ),
            vec![
                Motion::Button(true),
                Motion::Move { dx: 40, dy: 0 },
                Motion::Button(false),
            ]
        );
    }
    #[test]
    fn a_short_touch_on_a_buttoned_pad_is_reported_for_the_cursor_to_judge() {
        let mut pad = Pointer::new(false, false, true);
        assert_eq!(
            run(
                &mut pad,
                &[
                    at(1000, EV_KEY, BTN_TOOL_FINGER, 1),
                    at(1000, EV_KEY, BTN_TOUCH, 1),
                    at(1000, EV_ABS, ABS_X, 500),
                    at(1000, EV_ABS, ABS_Y, 400),
                    syn_at(1000),
                    at(1080, EV_KEY, BTN_TOUCH, 0),
                    at(1080, EV_KEY, BTN_TOOL_FINGER, 0),
                    syn_at(1080),
                ],
            ),
            vec![
                Motion::Finger(Finger::Down { ms: 1000 }),
                Motion::Finger(Finger::Up { ms: 1080 }),
            ],
            "the pad says when the finger landed and when it lifted"
        );

        // The button was already held when the finger landed, so this
        // touch is the press and not a tap.
        assert_eq!(
            run(
                &mut pad,
                &[
                    at(2000, EV_KEY, BTN_LEFT, 1),
                    syn_at(2000),
                    at(2010, EV_KEY, BTN_TOOL_FINGER, 1),
                    at(2010, EV_KEY, BTN_TOUCH, 1),
                    syn_at(2010),
                    at(2060, EV_KEY, BTN_TOUCH, 0),
                    at(2060, EV_KEY, BTN_TOOL_FINGER, 0),
                    at(2060, EV_KEY, BTN_LEFT, 0),
                    syn_at(2060),
                ],
            ),
            vec![
                Motion::Button(true),
                Motion::Finger(Finger::Down { ms: 2010 }),
                Motion::Finger(Finger::Cancel),
                Motion::Finger(Finger::Up { ms: 2060 }),
                Motion::Button(false),
            ],
            "a touch under a held button is not a tap"
        );
    }

    #[test]
    fn two_fingers_on_a_pad_scroll_instead_of_moving_the_cursor() {
        let mut pad = Pointer::new(false, false, true);
        assert_eq!(
            run(
                &mut pad,
                &[
                    ev(EV_KEY, BTN_TOOL_FINGER, 1),
                    ev(EV_KEY, BTN_TOUCH, 1),
                    ev(EV_ABS, ABS_X, 500),
                    ev(EV_ABS, ABS_Y, 400),
                    syn(),
                    ev(EV_KEY, BTN_TOOL_DOUBLETAP, 1),
                    syn(),
                    // The device now reports the other finger, wherever
                    // it is, and that jump moves nothing.
                    ev(EV_ABS, ABS_X, 200),
                    ev(EV_ABS, ABS_Y, 900),
                    syn(),
                    ev(EV_ABS, ABS_Y, 930),
                    syn(),
                    // Across the pad, which the cursor scrolls nothing
                    // for.
                    ev(EV_ABS, ABS_X, 400),
                    syn(),
                ],
            ),
            vec![
                Motion::Finger(Finger::Down { ms: 0 }),
                Motion::Finger(Finger::Cancel),
                Motion::Pan { dx: 0, dy: 30 },
                Motion::Pan { dx: 200, dy: 0 },
            ],
            "a second finger scrolls, and no touch of two fingers is a tap"
        );

        // One finger lifts and the other carries on from where it is.
        assert_eq!(
            run(
                &mut pad,
                &[
                    ev(EV_KEY, BTN_TOOL_DOUBLETAP, 0),
                    syn(),
                    ev(EV_ABS, ABS_X, 700),
                    ev(EV_ABS, ABS_Y, 100),
                    syn(),
                    ev(EV_ABS, ABS_X, 712),
                    syn(),
                ],
            ),
            vec![Motion::PanEnd, Motion::Move { dx: 12, dy: 0 }],
            "the pan ends when a finger lifts, and the jump back to one \
             finger's position moves nothing"
        );
    }

    #[test]
    fn a_contact_landing_moves_nothing_and_the_one_that_moves_is_followed() {
        let mut pad = Pointer::new(false, false, false);
        assert_eq!(
            run(
                &mut pad,
                &[
                    ev(EV_KEY, BTN_TOOL_FINGER, 1),
                    ev(EV_KEY, BTN_TOUCH, 1),
                    ev(EV_ABS, ABS_MT_SLOT, 0),
                    ev(EV_ABS, ABS_MT_POSITION_X, 100),
                    ev(EV_ABS, ABS_MT_POSITION_Y, 100),
                    syn(),
                    ev(EV_ABS, ABS_MT_SLOT, 1),
                    ev(EV_ABS, ABS_MT_POSITION_X, 900),
                    ev(EV_ABS, ABS_MT_POSITION_Y, 900),
                    syn(),
                    ev(EV_ABS, ABS_MT_SLOT, 0),
                    ev(EV_ABS, ABS_MT_POSITION_X, 120),
                    syn(),
                ],
            ),
            vec![
                Motion::Finger(Finger::Down { ms: 0 }),
                Motion::Move { dx: 20, dy: 0 },
            ],
            "a second contact's first position is not a move"
        );
    }

    /// A clickpad as the kernel reports one, through the cursor, to what
    /// the core is sent (`docs/MOTION.md` §3.9). Every event's kernel
    /// time is also the loop's clock, and the loop asks the cursor for a
    /// due release before each event, as the shell's main loop does on
    /// every pass.
    mod pad {
        use std::time::{Duration, Instant};

        use faraday_shell_api::{Event, TouchPhase};

        use super::*;
        use crate::cursor::Cursor;

        /// A buttoned pad with multitouch slots on a 1920 × 1080 panel,
        /// whose cursor starts in the middle, at (960, 540).
        struct Pad {
            pointer: Pointer,
            cursor: Cursor,
            t0: Instant,
            events: Vec<Event>,
        }

        impl Pad {
            fn new() -> Pad {
                Pad::on(1920, 1080)
            }

            fn on(width: u16, height: u16) -> Pad {
                Pad {
                    pointer: Pointer::new(false, false, true),
                    cursor: Cursor::new(width, height, 1.0),
                    t0: Instant::now(),
                    events: Vec::new(),
                }
            }

            fn clock(&self, ms: u64) -> Instant {
                self.t0 + Duration::from_millis(ms)
            }

            /// The loop's pass at `ms` with nothing read.
            fn wait(&mut self, ms: u64) {
                let now = self.clock(ms);
                self.events
                    .extend(self.cursor.due(now).into_iter().flat_map(|p| p.events()));
            }

            /// One packet at `ms`: `body` and the `SYN_REPORT` after it.
            fn packet(&mut self, ms: u64, body: &[(u16, u16, i32)]) {
                self.wait(ms);
                let now = self.clock(ms);
                let mut out = Vec::new();
                for &(kind, code, value) in body {
                    self.pointer.feed(at(ms, kind, code, value), &mut out);
                }
                self.pointer.feed(syn_at(ms), &mut out);
                for wake in out {
                    let Wake::Pointer(motion) = wake else {
                        panic!("a pointer produced {wake:?}");
                    };
                    self.events.extend(
                        self.cursor
                            .apply_at(motion, now)
                            .into_iter()
                            .flat_map(|p| p.events()),
                    );
                }
            }

            fn take(&mut self) -> Vec<Event> {
                std::mem::take(&mut self.events)
            }
        }

        /// A contact landing in `slot` at `(x, y)`.
        fn land(slot: i32, id: i32, x: i32, y: i32) -> [(u16, u16, i32); 4] {
            [
                (EV_ABS, ABS_MT_SLOT, slot),
                (EV_ABS, ABS_MT_TRACKING_ID, id),
                (EV_ABS, ABS_MT_POSITION_X, x),
                (EV_ABS, ABS_MT_POSITION_Y, y),
            ]
        }

        /// The contact in `slot` now at `(x, y)`.
        fn moved(slot: i32, x: i32, y: i32) -> [(u16, u16, i32); 3] {
            [
                (EV_ABS, ABS_MT_SLOT, slot),
                (EV_ABS, ABS_MT_POSITION_X, x),
                (EV_ABS, ABS_MT_POSITION_Y, y),
            ]
        }

        /// The contact in `slot` lifted.
        fn lifted(slot: i32) -> [(u16, u16, i32); 2] {
            [
                (EV_ABS, ABS_MT_SLOT, slot),
                (EV_ABS, ABS_MT_TRACKING_ID, -1),
            ]
        }

        fn key(code: u16, down: bool) -> (u16, u16, i32) {
            (EV_KEY, code, i32::from(down))
        }

        fn touch(x: u16, y: u16, phase: TouchPhase) -> Event {
            Event::Touch { x, y, phase }
        }

        /// One finger landing at `from` and lifting `ms` later: a tap.
        fn tap(pad: &mut Pad, from: u64, ms: u64) {
            let mut down = land(0, 1, 500, 500).to_vec();
            down.extend([key(BTN_TOUCH, true), key(BTN_TOOL_FINGER, true)]);
            pad.packet(from, &down);
            let mut up = lifted(0).to_vec();
            up.extend([key(BTN_TOUCH, false), key(BTN_TOOL_FINGER, false)]);
            pad.packet(from + ms, &up);
        }

        /// A finger resting on the clickpad's bottom presses it, and a
        /// second finger lands higher up, ready to drag.
        fn click_and_second_finger(pad: &mut Pad) {
            let mut a = land(0, 1, 500, 900).to_vec();
            a.extend([key(BTN_TOUCH, true), key(BTN_TOOL_FINGER, true)]);
            pad.packet(1000, &a);
            pad.packet(1100, &[key(BTN_LEFT, true)]);
            let mut b = land(1, 2, 300, 400).to_vec();
            b.extend([key(BTN_TOOL_FINGER, false), key(BTN_TOOL_DOUBLETAP, true)]);
            pad.packet(1200, &b);
        }

        #[test]
        fn a_held_click_and_a_second_finger_drag_where_the_moving_finger_goes() {
            let mut pad = Pad::new();
            click_and_second_finger(&mut pad);
            // The second finger moves; the pressing one only trembles.
            let mut step = moved(1, 300, 440).to_vec();
            step.extend(moved(0, 501, 900));
            pad.packet(1220, &step);
            pad.packet(1240, &moved(1, 300, 480));
            pad.packet(1300, &[key(BTN_LEFT, false)]);
            assert_eq!(
                pad.take(),
                vec![
                    touch(960, 540, TouchPhase::Down),
                    touch(960, 580, TouchPhase::Move),
                    touch(960, 620, TouchPhase::Move),
                    touch(960, 620, TouchPhase::Up),
                ],
                "the button stays held and the pointer follows the finger that moves, \
                 with no scroll"
            );
        }

        #[test]
        fn a_tap_then_a_quick_touch_that_moves_drags_until_it_lifts() {
            let mut pad = Pad::new();
            tap(&mut pad, 1000, 60);
            let mut down = land(0, 2, 600, 600).to_vec();
            down.extend([key(BTN_TOUCH, true), key(BTN_TOOL_FINGER, true)]);
            pad.packet(1150, &down);
            pad.packet(1170, &moved(0, 600, 640));
            pad.packet(1190, &moved(0, 600, 700));
            // Held still past the wait: still held.
            pad.wait(1500);
            let mut up = lifted(0).to_vec();
            up.extend([key(BTN_TOUCH, false), key(BTN_TOOL_FINGER, false)]);
            pad.packet(1600, &up);
            pad.wait(2000);
            assert_eq!(
                pad.take(),
                vec![
                    touch(960, 540, TouchPhase::Down),
                    touch(960, 580, TouchPhase::Move),
                    touch(960, 640, TouchPhase::Move),
                    touch(960, 640, TouchPhase::Up),
                ]
            );
        }

        #[test]
        fn a_single_tap_is_still_a_click() {
            let mut pad = Pad::new();
            tap(&mut pad, 1000, 60);
            pad.wait(1300);
            assert_eq!(
                pad.take(),
                vec![
                    touch(960, 540, TouchPhase::Down),
                    touch(960, 540, TouchPhase::Up),
                ]
            );
        }

        #[test]
        fn two_fingers_with_no_click_still_scroll() {
            let mut pad = Pad::new();
            let mut a = land(0, 1, 500, 500).to_vec();
            a.extend([key(BTN_TOUCH, true), key(BTN_TOOL_FINGER, true)]);
            pad.packet(1000, &a);
            let mut b = land(1, 2, 700, 500).to_vec();
            b.extend([key(BTN_TOOL_FINGER, false), key(BTN_TOOL_DOUBLETAP, true)]);
            pad.packet(1010, &b);
            let mut step = moved(0, 500, 530).to_vec();
            step.extend(moved(1, 700, 530));
            pad.packet(1030, &step);
            let mut up = lifted(0).to_vec();
            up.extend(lifted(1));
            up.extend([key(BTN_TOUCH, false), key(BTN_TOOL_DOUBLETAP, false)]);
            pad.packet(1100, &up);
            pad.wait(1500);
            assert_eq!(
                pad.take(),
                vec![
                    Event::Scroll {
                        x: 960,
                        y: 540,
                        dy: -30
                    },
                    Event::ScrollEnd { x: 960, y: 540 },
                ],
                "the content moves with the fingers and nothing is pressed"
            );
        }

        #[test]
        fn a_slow_touch_after_a_tap_only_moves_the_pointer() {
            let mut pad = Pad::new();
            tap(&mut pad, 1000, 60);
            let mut down = land(0, 2, 600, 600).to_vec();
            down.extend([key(BTN_TOUCH, true), key(BTN_TOOL_FINGER, true)]);
            pad.packet(1360, &down);
            pad.packet(1380, &moved(0, 630, 600));
            let mut up = lifted(0).to_vec();
            up.extend([key(BTN_TOUCH, false), key(BTN_TOOL_FINGER, false)]);
            pad.packet(1500, &up);
            assert_eq!(
                pad.take(),
                vec![
                    touch(960, 540, TouchPhase::Down),
                    touch(960, 540, TouchPhase::Up),
                    Event::Hover { x: 990, y: 540 },
                ],
                "the tap was a click, and the touch 300 ms after it is no drag"
            );
        }

        #[test]
        fn a_held_click_and_a_second_finger_drag_the_scrollbar_and_scroll_the_page() {
            use faraday_core::{Action, Faraday, Screen, StorageEvent};
            use faraday_shell_api::{App, BootState, DisplayInfo, SecureHardware};

            let mut app = Faraday::new();
            app.event(Event::Display(DisplayInfo {
                width: 1280,
                height: 800,
                dpi: 160,
                inset_bottom: 0,
                inset_top: 0,
                buttons: 0,
                camera_fixed: false,
                secure: SecureHardware::None,
                boot: BootState::Unknown,
                memory_mib: None,
            }));
            app.storage(StorageEvent::Restored {
                inbox: Vec::new(),
                outbox: Vec::new(),
                kept: Vec::new(),
            });
            app.press(Action::Nav(Screen::Catalog));
            app.settle();
            app.event(Event::Tick { now_ms: 1_000 });
            while app.poll_command().is_some() {}
            let _ = app.frame();

            let mut pad = Pad::on(1280, 800);
            // Point at the bottom of the scrollbar, at the right edge.
            let mut a = land(0, 1, 100, 100).to_vec();
            a.extend([key(BTN_TOUCH, true), key(BTN_TOOL_FINGER, true)]);
            pad.packet(100, &a);
            pad.packet(120, &moved(0, 734, 490));
            let mut up = lifted(0).to_vec();
            up.extend([key(BTN_TOUCH, false), key(BTN_TOOL_FINGER, false)]);
            pad.packet(500, &up);
            // Press there, then drag up with a second finger.
            click_and_second_finger(&mut pad);
            for (i, y) in [300, 200, 100, -400].into_iter().enumerate() {
                pad.packet(1300 + 20 * i as u64, &moved(1, 300, y));
            }
            let mut low = None;
            let mut top = None;
            for event in pad.take() {
                let phase = match event {
                    Event::Touch { phase, .. } => Some(phase),
                    _ => None,
                };
                app.event(event);
                while app.poll_command().is_some() {}
                let _ = app.frame();
                match phase {
                    Some(TouchPhase::Down) => low = Some(app.list_offset),
                    Some(TouchPhase::Move) => top = Some(app.list_offset),
                    _ => {}
                }
            }
            let (low, top) = (low.expect("pressed"), top.expect("dragged"));
            assert!(low > 0.0, "pressing low on the bar moved the page down");
            assert_eq!(top, 0.0, "dragging the bar to the top brought the page up");
            pad.packet(1500, &[key(BTN_LEFT, false)]);
            for event in pad.take() {
                app.event(event);
            }
            while app.poll_command().is_some() {}
            assert_eq!(app.screen, Screen::Catalog, "nothing under it was pressed");
        }
    }
}
