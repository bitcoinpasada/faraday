//! Mice and touchpads: packets in, cursor motion out.
//!
//! A pointer is a touch at a place the person aims with. The core has no
//! pointer and needs none: the cursor lives here, `BTN_LEFT` down, the
//! moves while it is down, and `BTN_LEFT` up are
//! [`Event::Touch`](osk_shell_api::Event::Touch) Down, Move and Up at the
//! cursor, and the wheel is [`Event::Scroll`](osk_shell_api::Event::Scroll)
//! there. Every screen therefore works with a mouse for the same reason
//! it works with a finger, and nothing in the core knows the difference.
//!
//! No acceleration: one device unit is one pixel, times `--pointer-speed`.
//! A signing device is not a desktop and there is no muscle memory to
//! honour; predictable is what a person aiming at a 48 dp row wants.
//!
//! A touchpad moves the cursor by the change in its reported finger
//! position while `BTN_TOUCH` is set, so lifting and replacing a finger
//! moves nothing. Its button is `BTN_LEFT`; a pad that has none clicks
//! when a finger lands (`BTN_TOUCH` with `BTN_TOOL_FINGER`), which makes
//! every stroke on such a pad a drag, the way a finger on the panel is.
//!
//! A pad that has a button gets two gestures, which are the two a laptop
//! person already makes. A short touch that hardly moves is a tap, and a
//! tap is a click at the cursor. While `BTN_TOOL_DOUBLETAP` says two
//! fingers are down, the finger's vertical travel scrolls instead of
//! moving the cursor. Which axes a position is read from depends on what
//! the device reports: a device whose `abs` bitmap has `ABS_X` is
//! followed by `ABS_X`/`ABS_Y`, the kernel's single-touch emulation,
//! which tracks the oldest contact, and its `ABS_MT_*` events are
//! ignored; a device without `ABS_X` is followed by the
//! `ABS_MT_POSITION_*` of slot 0.

use crate::Wake;
use crate::evdev::{
    ABS_MT_POSITION_X, ABS_MT_POSITION_Y, ABS_MT_SLOT, ABS_X, ABS_Y, BTN_LEFT, BTN_TOOL_DOUBLETAP,
    BTN_TOOL_FINGER, BTN_TOUCH, EV_ABS, EV_KEY, EV_REL, EV_SYN, REL_WHEEL, REL_WHEEL_HI_RES, REL_X,
    REL_Y, RawEvent, SYN_REPORT,
};

/// `REL_WHEEL_HI_RES` units in one notch of the wheel.
const HI_RES_PER_NOTCH: i32 = 120;

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
    /// Two fingers travelled this far down the pad, in device units.
    /// The cursor turns it into a scroll that moves the content with
    /// the fingers, as a finger on a touchscreen does.
    Pan {
        /// Units toward the person.
        dy: i32,
    },
    /// The button went down (`true`) or came up (`false`).
    Button(bool),
    /// One contact on a buttoned pad. It never moves the cursor.
    Finger(Finger),
    /// The wheel turned this many notches; positive is away from the
    /// person, which scrolls the content down.
    Wheel(i32),
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
    /// Positions come from `ABS_X`/`ABS_Y` rather than from the
    /// multitouch axes of slot 0.
    abs_single: bool,
    dx: i32,
    dy: i32,
    notches: i32,
    fine: i32,
    /// The last position the device reported, axis by axis: a driver
    /// sends only the axis that changed.
    abs: (i32, i32),
    /// This packet reported a position.
    abs_new: bool,
    /// Where the finger was at the last packet, so a pad reports a delta.
    last_abs: Option<(i32, i32)>,
    /// Device units two fingers have travelled down the pad since the
    /// last packet that reported any.
    pan: i32,
    /// The slot the `ABS_MT_*` events now belong to.
    slot: i32,
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
    /// tapping when `tap_clicks`, and whose position is `ABS_X`/`ABS_Y`
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
            pan: 0,
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
                ABS_MT_SLOT => self.slot = ev.value,
                ABS_X if self.abs_single => {
                    self.abs.0 = ev.value;
                    self.abs_new = true;
                }
                ABS_Y if self.abs_single => {
                    self.abs.1 = ev.value;
                    self.abs_new = true;
                }
                ABS_MT_POSITION_X if !self.abs_single && self.slot == 0 => {
                    self.abs.0 = ev.value;
                    self.abs_new = true;
                }
                ABS_MT_POSITION_Y if !self.abs_single && self.slot == 0 => {
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
                    }
                }
                BTN_TOUCH => {
                    let down = ev.value != 0;
                    if self.touching != down {
                        self.touching = down;
                        if !down {
                            self.last_abs = None;
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
                BTN_TOOL_FINGER => self.finger = ev.value != 0,
                BTN_TOOL_DOUBLETAP => {
                    let two = ev.value != 0;
                    if self.two != two {
                        self.two = two;
                        // The two fingers are at two places and the
                        // device reports one of them: the jump from the
                        // one to the other moves nothing.
                        self.last_abs = None;
                        if two {
                            self.cancel();
                        }
                    }
                }
                _ => {}
            },
            EV_SYN if ev.code == SYN_REPORT => self.sync(out),
            _ => {}
        }
    }

    /// End of packet: the move first, so a click in the same packet lands
    /// where the pointer now is.
    fn sync(&mut self, out: &mut Vec<Wake>) {
        if std::mem::take(&mut self.abs_new) && self.touching {
            if let Some((lx, ly)) = self.last_abs {
                if self.two {
                    self.pan += self.abs.1 - ly;
                } else {
                    self.dx += self.abs.0 - lx;
                    self.dy += self.abs.1 - ly;
                }
            }
            self.last_abs = Some(self.abs);
        }
        if (self.dx, self.dy) != (0, 0) {
            out.push(Wake::Pointer(Motion::Move {
                dx: self.dx,
                dy: self.dy,
            }));
            self.dx = 0;
            self.dy = 0;
        }
        if self.pan != 0 {
            out.push(Wake::Pointer(Motion::Pan { dy: self.pan }));
            self.pan = 0;
        }
        for finger in self.fingers.drain(..) {
            out.push(Wake::Pointer(Motion::Finger(finger)));
        }
        if let Some(down) = self.button.take() {
            out.push(Wake::Pointer(Motion::Button(down)));
        }
        let whole = self.fine / HI_RES_PER_NOTCH;
        if whole != 0 {
            self.fine -= whole * HI_RES_PER_NOTCH;
            self.notches += whole;
        }
        if self.notches != 0 {
            out.push(Wake::Pointer(Motion::Wheel(self.notches)));
            self.notches = 0;
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
                Motion::Wheel(-1),
            ]
        );
    }

    #[test]
    fn a_fine_wheel_is_counted_once_and_in_whole_notches() {
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
            vec![],
            "half a notch turns nothing yet"
        );
        assert_eq!(
            run(&mut mouse, &[ev(EV_REL, REL_WHEEL_HI_RES, 60), syn()],),
            vec![Motion::Wheel(1)]
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
                    // Across the pad, which scrolls nothing.
                    ev(EV_ABS, ABS_X, 400),
                    syn(),
                ],
            ),
            vec![
                Motion::Finger(Finger::Down { ms: 0 }),
                Motion::Finger(Finger::Cancel),
                Motion::Pan { dy: 30 },
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
            vec![Motion::Move { dx: 12, dy: 0 }],
            "the jump back to one finger's position moves nothing"
        );
    }

    #[test]
    fn a_pad_with_no_single_touch_axes_is_followed_by_its_first_contact() {
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
            "the second slot's positions are not the pointer's"
        );
    }
}
