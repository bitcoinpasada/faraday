//! The cursor: where a mouse or a touchpad is pointing, and the arrow
//! that shows it.
//!
//! The arrow is the shell's, not the core's. The core composes a frame
//! that knows nothing about pointers; the shell converts that frame into
//! the panel's own format in a buffer of its own and draws the arrow into
//! that copy, so the core's frame is never touched and the next frame
//! starts from a clean conversion — there is nothing to restore.
//!
//! It is hidden until a pointer first moves, and hidden again the moment
//! a finger touches a touchscreen, because the person's attention has
//! gone to the screen. A machine with no pointing device never has one,
//! so the Pi's panel gets exactly the bytes it got before this file
//! existed.
//!
//! Size: twelve by nineteen pixels, the arrow a desktop draws, on any
//! panel under 1,800 pixels on its short side, and doubled or more by a
//! whole number above that, so the edges stay sharp. It is sized by the
//! panel's pixels, not the density the core is told: a laptop's command
//! line says 160 dpi so the interface is comfortable, but a 1366×768 panel
//! is a desktop's density, where a doubled arrow is twice the usual size. White with a
//! one-pixel black edge, which is legible on the dark ground every screen
//! draws and on the white of a Words card.

use std::time::{Duration, Instant};

use faraday_shell_api::{Event, TouchPhase};

use crate::fb::{self, Geometry};
use crate::orient::Turn;
use crate::pointer::{Finger, Motion};
use crate::touch::Touch;

/// What one notch of the wheel is sent as, the same as the desktop
/// shell sends for a line: 48 pixels, which the core reads as a count of
/// notches and glides by its own distance in the layout's units
/// (`docs/MOTION.md` §3.3).
const WHEEL_NOTCH_PX: f32 = 48.0;

/// What a wheel reports one notch as (`REL_WHEEL_HI_RES` units).
const WHEEL_NOTCH_UNITS: f32 = 120.0;

/// The longest a touch on a buttoned pad can last and still be a tap,
/// and the longest a tap's click is held waiting for the touch that would
/// make it a drag: libinput's tap timeout (`docs/MOTION.md` §3.9). A
/// deliberate tap is over well inside this; a finger resting on the pad
/// is not tapping it.
const TAP_MS: u64 = 180;

/// The furthest the cursor may travel during a tap, in device counts
/// (frame pixels at scale 1, times the arrow's scale on a denser panel),
/// measured on the longer axis. A finger never lands and lifts on
/// exactly one point, and a stroke that carries the cursor further than
/// this was aimed somewhere else.
const TAP_TRAVEL_PX: i32 = 6;

/// Panel pixels on the short side per whole step of the arrow's scale:
/// 768 to 1,200 lines (1366×768, 1920×1080, 2560×1440, 2560×1600) draw it
/// 1:1, 2880×1800 and 3840×2160 at 2:1, as desktops scale them.
const ARROW_LINES: u16 = 1200;

/// The arrow, one character per pixel: `K` the black fill, `W` the white
/// edge, `.` nothing. The usual pointer: a tilted head with its tail
/// running down and to the right. Black with a white edge is the arrow a
/// desktop draws, and it reads on the dark ground every screen has and
/// on the white of a Words card alike.
const ARROW: [&str; 19] = [
    "W...........",
    "WW..........",
    "WKW.........",
    "WKKW........",
    "WKKKW.......",
    "WKKKKW......",
    "WKKKKKW.....",
    "WKKKKKKW....",
    "WKKKKKKKW...",
    "WKKKKKKKKW..",
    "WKKKKKKKKKW.",
    "WKKKKKKKKKKW",
    "WKKKKKKWWWWW",
    "WKKKWKKW....",
    "WKKW.WKKW...",
    "WKW..WKKW...",
    "WW....WKKW..",
    "W.....WKKW..",
    ".......WW...",
];

/// What pointing at the panel produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pointed {
    /// A contact at the cursor, which is what a click and a drag are.
    Touch(Touch),
    /// A second tap while the first one's click is still held: the held
    /// contact comes up and a new one goes down, at the cursor.
    Reclick {
        /// Pixels from the left edge.
        x: u16,
        /// Pixels from the top edge.
        y: u16,
    },
    /// Two fingers moved the content at the cursor.
    Scroll {
        /// Pixels from the left edge.
        x: u16,
        /// Pixels from the top edge.
        y: u16,
        /// Pixels the content moves; positive moves it up.
        dy: i16,
    },
    /// The two fingers lifted.
    ScrollEnd {
        /// Pixels from the left edge.
        x: u16,
        /// Pixels from the top edge.
        y: u16,
    },
    /// The wheel turned at the cursor.
    Wheel {
        /// Pixels from the left edge.
        x: u16,
        /// Pixels from the top edge.
        y: u16,
        /// Pixels the content should move; positive moves it up.
        dy: i16,
    },
    /// The mouse's back button: the core's back.
    Back,
    /// The mouse's forward button: the core's forward.
    Forward,
    /// The arrow moved to here with no button down.
    Hover {
        /// Pixels from the left edge.
        x: u16,
        /// Pixels from the top edge.
        y: u16,
    },
}

impl Pointed {
    /// What the core is sent for this, in order.
    pub fn events(self) -> impl Iterator<Item = Event> {
        let touch = |x, y, phase| Event::Touch { x, y, phase };
        let (first, second) = match self {
            Pointed::Touch(t) => (touch(t.x, t.y, t.phase), None),
            Pointed::Reclick { x, y } => (
                touch(x, y, TouchPhase::Up),
                Some(touch(x, y, TouchPhase::Down)),
            ),
            Pointed::Scroll { x, y, dy } => (Event::Scroll { x, y, dy }, None),
            Pointed::ScrollEnd { x, y } => (Event::ScrollEnd { x, y }, None),
            Pointed::Wheel { x, y, dy } => (Event::Wheel { x, y, dy }, None),
            Pointed::Hover { x, y } => (Event::Hover { x, y }, None),
            Pointed::Back => (Event::Back, None),
            Pointed::Forward => (Event::Forward, None),
        };
        std::iter::once(first).chain(second)
    }
}

/// Whole pixels of `px` plus what `carry` held, keeping the fraction in
/// `carry` for the next move, so slow moves add up instead of rounding
/// away.
fn whole(px: f32, carry: &mut f32) -> i16 {
    let total = px + *carry;
    let n = total
        .round()
        .clamp(f32::from(i16::MIN), f32::from(i16::MAX));
    *carry = total - n;
    n as i16
}

/// Where a buttoned pad's taps have got to (`docs/MOTION.md` §3.9).
///
/// A tap presses the button when the finger lifts and holds it for
/// [`TAP_MS`]; a finger that lands again inside that time keeps it held
/// and drags until it lifts, and one that lifts again at once is a
/// second click. Nothing comes after the lift: there is no drag lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tap {
    /// No touch that could be a tap, and no tap holding the button.
    Idle,
    /// A touch that could still be a tap: where the cursor was when the
    /// finger landed, and when it landed.
    Touching { x: u16, y: u16, ms: u64 },
    /// A tap holds the button down until `until`, waiting for a touch
    /// that would drag. `lifted` is the kernel's time of the lift, which
    /// the next touch is measured against.
    Tapped { lifted: u64, until: Instant },
    /// A finger landed inside the wait: the button stays held and the
    /// finger drags. Lifting it at once where it landed is a second tap.
    Again { x: u16, y: u16, ms: u64 },
    /// The finger that landed inside the wait has done more than tap: the
    /// button is held until it lifts.
    Dragging,
}

/// Where the pointer is, and whether it should be drawn.
#[derive(Debug)]
pub struct Cursor {
    x: f32,
    y: f32,
    width: u16,
    height: u16,
    /// Frame pixels one device count moves the cursor: `--pointer-speed`
    /// times the arrow's scale (`docs/MOTION.md` §3.2).
    speed: f32,
    /// The arrow's scale: what one step of the tap limit is in pixels.
    scale: f32,
    visible: bool,
    /// The device's button is held.
    button: bool,
    /// What the pad's taps are doing.
    tap: Tap,
    /// Fractions of a pixel two fingers have moved and not yet sent.
    pan_carry: f32,
    /// Fractions of a pixel the wheel has turned and not yet sent.
    wheel_carry: f32,
}

impl Cursor {
    /// A cursor in the middle of a `width × height` panel, hidden until
    /// something moves it, at `--pointer-speed` device pixels per unit.
    pub fn new(width: u16, height: u16, speed: f32) -> Cursor {
        let scale_f = f32::from(scale(width.min(height)));
        Cursor {
            x: f32::from(width) / 2.0,
            y: f32::from(height) / 2.0,
            width,
            height,
            speed: speed * scale_f,
            scale: scale_f,
            visible: false,
            button: false,
            tap: Tap::Idle,
            pan_carry: 0.0,
            wheel_carry: 0.0,
        }
    }

    /// The frame is now `width × height` (a turn of the panel,
    /// `docs/ORIENTATION.md` §7): the cursor stays where it was, brought
    /// inside the new frame.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.x = self.x.clamp(0.0, f32::from(width).max(1.0) - 1.0);
        self.y = self.y.clamp(0.0, f32::from(height).max(1.0) - 1.0);
    }

    /// Where the cursor is, in the frame's pixels: the panel's, as the
    /// person sees it.
    pub fn at(&self) -> (u16, u16) {
        (self.x as u16, self.y as u16)
    }

    /// Whether the arrow is drawn.
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Takes the arrow away: a finger has touched the screen, so the
    /// person is pointing at it and not with the mouse.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Whether a contact is down at the cursor: the button is held, or a
    /// tap holds it.
    fn pressed(&self) -> bool {
        self.button
            || matches!(
                self.tap,
                Tap::Tapped { .. } | Tap::Again { .. } | Tap::Dragging
            )
    }

    /// A contact at the cursor in `phase`.
    fn touch(&self, phase: TouchPhase) -> Pointed {
        let (x, y) = self.at();
        Pointed::Touch(Touch { x, y, phase })
    }

    /// Applies one device motion now, returning what the core should be
    /// told.
    pub fn apply(&mut self, motion: Motion) -> Option<Pointed> {
        self.apply_at(motion, Instant::now())
    }

    /// Applies one device motion at `now`, the clock [`due`](Self::due)
    /// is read against, returning what the core should be told.
    pub fn apply_at(&mut self, motion: Motion, now: Instant) -> Option<Pointed> {
        match motion {
            Motion::Move { dx, dy } => {
                self.x = (self.x + dx as f32 * self.speed)
                    .clamp(0.0, f32::from(self.width).max(1.0) - 1.0);
                self.y = (self.y + dy as f32 * self.speed)
                    .clamp(0.0, f32::from(self.height).max(1.0) - 1.0);
                self.visible = true;
                let (x, y) = self.at();
                Some(if self.pressed() {
                    self.touch(TouchPhase::Move)
                } else {
                    Pointed::Hover { x, y }
                })
            }
            Motion::Button(down) => {
                if self.button == down {
                    return None;
                }
                let held = self.pressed();
                self.button = down;
                if down {
                    // The button goes down under a tap's hold: the
                    // contact is already down, and from here the button
                    // holds it.
                    self.tap = Tap::Idle;
                    (!held).then(|| self.touch(TouchPhase::Down))
                } else {
                    Some(self.touch(TouchPhase::Up))
                }
            }
            Motion::Finger(finger) => self.finger(finger, now),
            // Two fingers while a tap holds the button drag with it, as
            // one finger does.
            Motion::Pan { dx, dy } if self.pressed() => self.apply_at(Motion::Move { dx, dy }, now),
            Motion::Pan { dy, .. } => {
                // Natural scrolling: two fingers drag the content the
                // way one finger drags it on a touchscreen, so fingers
                // moving toward the person move the content down, which
                // is the core's negative.
                let dy = whole(-(dy as f32) * self.speed, &mut self.pan_carry);
                if dy == 0 {
                    return None;
                }
                let (x, y) = self.at();
                Some(Pointed::Scroll { x, y, dy })
            }
            Motion::PanEnd => {
                self.pan_carry = 0.0;
                let (x, y) = self.at();
                Some(Pointed::ScrollEnd { x, y })
            }
            Motion::Back => Some(Pointed::Back),
            Motion::Forward => Some(Pointed::Forward),
            Motion::Wheel(units) => {
                // evdev counts a turn away from the person as positive,
                // which moves the content down; the core's positive moves
                // it up.
                let px = -(units as f32) / WHEEL_NOTCH_UNITS * WHEEL_NOTCH_PX;
                let dy = whole(px, &mut self.wheel_carry);
                if dy == 0 {
                    return None;
                }
                let (x, y) = self.at();
                Some(Pointed::Wheel { x, y, dy })
            }
        }
    }

    /// The release of a tap's click once its wait is over at `now`: the
    /// finger did not come back, so the tap was a click. The shell asks
    /// on every pass of its loop.
    pub fn due(&mut self, now: Instant) -> Option<Pointed> {
        match self.tap {
            Tap::Tapped { until, .. } if now >= until => {
                self.tap = Tap::Idle;
                (!self.button).then(|| self.touch(TouchPhase::Up))
            }
            _ => None,
        }
    }

    /// Whether a touch that landed at `from` at `landed` and lifted at
    /// `ms` here was a tap: inside [`TAP_MS`], having carried the cursor
    /// no further than [`TAP_TRAVEL_PX`].
    fn tapped(&self, from: (u16, u16), landed: u64, ms: u64) -> bool {
        let (x, y) = self.at();
        let travel = (i32::from(x) - i32::from(from.0))
            .abs()
            .max((i32::from(y) - i32::from(from.1)).abs());
        ms.saturating_sub(landed) <= TAP_MS && travel as f32 <= TAP_TRAVEL_PX as f32 * self.scale
    }

    /// What one contact on a buttoned pad did (`docs/MOTION.md` §3.9).
    /// A tap presses at the cursor when it lifts and holds the press for
    /// [`TAP_MS`]; a touch inside that time drags with the press held
    /// until it lifts, or, lifted at once, is a second tap. Any other
    /// touch moved the cursor and nothing more.
    fn finger(&mut self, finger: Finger, now: Instant) -> Option<Pointed> {
        let (x, y) = self.at();
        match (finger, self.tap) {
            (Finger::Down { ms }, Tap::Tapped { lifted, .. }) => {
                if ms.saturating_sub(lifted) <= TAP_MS {
                    self.tap = Tap::Again { x, y, ms };
                    None
                } else {
                    // Too late to drag: the tap's click ends now, before
                    // this touch, which may be a tap of its own.
                    self.tap = Tap::Touching { x, y, ms };
                    (!self.button).then(|| self.touch(TouchPhase::Up))
                }
            }
            (Finger::Down { ms }, _) => {
                self.tap = Tap::Touching { x, y, ms };
                None
            }
            (Finger::Cancel, Tap::Touching { .. }) => {
                self.tap = Tap::Idle;
                None
            }
            // A second finger or the button under the drag: it is a drag
            // and no second tap.
            (Finger::Cancel, Tap::Again { .. }) => {
                self.tap = Tap::Dragging;
                None
            }
            (Finger::Cancel, _) => None,
            (
                Finger::Up { ms },
                Tap::Touching {
                    x: fx,
                    y: fy,
                    ms: landed,
                },
            ) => {
                if self.tapped((fx, fy), landed, ms) && !self.button {
                    self.tap = Tap::Tapped {
                        lifted: ms,
                        until: now + Duration::from_millis(TAP_MS),
                    };
                    Some(self.touch(TouchPhase::Down))
                } else {
                    self.tap = Tap::Idle;
                    None
                }
            }
            (
                Finger::Up { ms },
                Tap::Again {
                    x: fx,
                    y: fy,
                    ms: landed,
                },
            ) => {
                if self.tapped((fx, fy), landed, ms) {
                    self.tap = Tap::Tapped {
                        lifted: ms,
                        until: now + Duration::from_millis(TAP_MS),
                    };
                    Some(Pointed::Reclick { x, y })
                } else {
                    self.tap = Tap::Idle;
                    Some(self.touch(TouchPhase::Up))
                }
            }
            (Finger::Up { .. }, Tap::Dragging) => {
                self.tap = Tap::Idle;
                Some(self.touch(TouchPhase::Up))
            }
            (Finger::Up { .. }, _) => None,
        }
    }
}

/// How many panel pixels one pixel of the arrow is on a panel whose short
/// side is `lines` pixels: one under 1,800, and a whole number from there.
pub fn scale(lines: u16) -> u16 {
    (lines.saturating_add(ARROW_LINES / 2) / ARROW_LINES).max(1)
}

/// The arrow's pixels within the frame, in frame pixels: the first
/// column and row and the last, inclusive, for the arrow drawn at `at`
/// on a frame `frame` in size, cut by its edges.
fn reach(scale: usize, frame: (usize, usize), at: (u16, u16)) -> (usize, usize, usize, usize) {
    let (left, top) = (usize::from(at.0), usize::from(at.1));
    let (fw, fh) = (frame.0.max(1), frame.1.max(1));
    (
        left.min(fw - 1),
        top.min(fh - 1),
        (left + ARROW[0].len() * scale - 1).min(fw - 1),
        (top + ARROW.len() * scale - 1).min(fh - 1),
    )
}

/// The panel's size, and the size of the frame on it at `turn`, in
/// pixels.
fn sizes(geometry: Geometry, turn: Turn) -> ((usize, usize), (usize, usize)) {
    let (w, h) = (geometry.width, geometry.height);
    let (fw, fh) = turn.frame_size((w, h));
    (
        (usize::from(w), usize::from(h)),
        (usize::from(fw), usize::from(fh)),
    )
}

/// The rows of the panel the arrow covers when it is drawn at `at` in
/// the frame, which `turn` turns onto the panel: the first and the
/// last, inclusive.
pub fn rows(geometry: Geometry, turn: Turn, at: (u16, u16)) -> (usize, usize) {
    let scale = usize::from(scale(geometry.width.min(geometry.height)));
    let (panel, frame) = sizes(geometry, turn);
    let (u0, v0, u1, v1) = reach(scale, frame, at);
    let (_, ya) = turn.to_panel(panel, (u0, v0));
    let (_, yb) = turn.to_panel(panel, (u1, v1));
    (ya.min(yb), ya.max(yb))
}

/// Draws the part of the arrow that falls in `out`, a band of the
/// panel's packed rows starting at row `first`, in the panel's format:
/// what a frame with the arrow in it holds there, so a band written over
/// the panel shows the arrow where the whole frame would. The arrow
/// stands at `at` in the frame, with its tip up and to the left as the
/// person sees it, and each of its pixels goes to the panel through the
/// same map as the frame's at `turn` (`docs/ORIENTATION.md` §4.1).
/// `out` is the shell's own copy; the core's frame is untouched.
pub fn draw_rows(out: &mut [u8], geometry: Geometry, turn: Turn, at: (u16, u16), first: usize) {
    let scale = usize::from(scale(geometry.width.min(geometry.height)));
    let bytes = geometry.depth.bytes_per_pixel();
    let row = geometry.row_bytes();
    let (panel, frame) = sizes(geometry, turn);
    let (left, top) = (usize::from(at.0), usize::from(at.1));

    for (r, line) in ARROW.iter().enumerate() {
        for (c, ink) in line.bytes().enumerate() {
            // One opaque pixel through the same conversion the frame
            // takes, so the arrow is in the panel's format whatever that
            // format is.
            let ink = match ink {
                b'K' => [0x00, 0x00, 0x00, 0xff],
                b'W' => [0xff, 0xff, 0xff, 0xff],
                _ => continue,
            };
            let mut pixel = [0u8; 4];
            fb::convert_into(&ink, geometry.depth, &mut pixel[..bytes]);
            for sy in 0..scale {
                let v = top + r * scale + sy;
                if v >= frame.1 {
                    break;
                }
                for sx in 0..scale {
                    let u = left + c * scale + sx;
                    if u >= frame.0 {
                        break;
                    }
                    let (x, y) = turn.to_panel(panel, (u, v));
                    let Some(y) = y.checked_sub(first) else {
                        continue;
                    };
                    let at = y * row + x * bytes;
                    if let Some(slice) = out.get_mut(at..at + bytes) {
                        slice.copy_from_slice(&pixel[..bytes]);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fb::Depth;

    /// The arrow drawn into a whole frame.
    fn draw(out: &mut [u8], geometry: Geometry, at: (u16, u16)) {
        draw_rows(out, geometry, Turn::R0, at, 0);
    }

    fn geometry(depth: Depth) -> Geometry {
        Geometry {
            width: 64,
            height: 64,
            depth,
            stride: 64 * depth.bytes_per_pixel(),
        }
    }

    #[test]
    fn a_mouse_move_then_a_click_is_a_touch_where_the_pointer_is() {
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        assert!(!cursor.visible(), "no arrow until the mouse is moved");
        assert_eq!(
            cursor.apply(Motion::Move { dx: 40, dy: -30 }),
            Some(Pointed::Hover { x: 1000, y: 510 }),
            "a move with no button down hovers"
        );
        assert!(cursor.visible());
        let (x, y) = cursor.at();
        assert_eq!((x, y), (1000, 510));

        assert_eq!(
            cursor.apply(Motion::Button(true)),
            Some(Pointed::Touch(Touch {
                x,
                y,
                phase: TouchPhase::Down
            }))
        );
        assert_eq!(
            cursor.apply(Motion::Move { dx: 5, dy: 0 }),
            Some(Pointed::Touch(Touch {
                x: x + 5,
                y,
                phase: TouchPhase::Move
            })),
            "a move with the button down drags"
        );
        assert_eq!(
            cursor.apply(Motion::Button(false)),
            Some(Pointed::Touch(Touch {
                x: x + 5,
                y,
                phase: TouchPhase::Up
            }))
        );
    }

    #[test]
    fn a_wheel_notch_scrolls_at_the_pointer() {
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 0, dy: 0 });
        let (x, y) = cursor.at();
        assert_eq!(
            cursor.apply(Motion::Wheel(120)),
            Some(Pointed::Wheel { x, y, dy: -48 }),
            "turning the wheel away moves the content down"
        );
        assert_eq!(
            cursor.apply(Motion::Wheel(-240)),
            Some(Pointed::Wheel { x, y, dy: 96 })
        );
        assert_eq!(
            cursor.apply(Motion::Wheel(-60)),
            Some(Pointed::Wheel { x, y, dy: 24 }),
            "half a notch of a fine wheel is half a notch's pixels"
        );
    }

    #[test]
    fn a_slow_pan_and_a_fine_wheel_lose_no_fraction_of_a_pixel() {
        let mut cursor = Cursor::new(1920, 1080, 0.4);
        let panned: i32 = (0..10)
            .filter_map(|_| match cursor.apply(Motion::Pan { dx: 0, dy: -1 }) {
                Some(Pointed::Scroll { dy, .. }) => Some(i32::from(dy)),
                _ => None,
            })
            .sum();
        assert_eq!(panned, 4, "ten moves of 0.4 pixels are four pixels");
        let turned: i32 = (0..12)
            .filter_map(|_| match cursor.apply(Motion::Wheel(-10)) {
                Some(Pointed::Wheel { dy, .. }) => Some(i32::from(dy)),
                _ => None,
            })
            .sum();
        assert_eq!(turned, 48, "twelve twelfths of a notch are a notch");
    }

    #[test]
    fn the_pointer_stays_on_the_panel() {
        let mut cursor = Cursor::new(1366, 768, 1.0);
        cursor.apply(Motion::Move {
            dx: -10_000,
            dy: -10_000,
        });
        assert_eq!(cursor.at(), (0, 0));
        cursor.apply(Motion::Move {
            dx: 10_000,
            dy: 10_000,
        });
        assert_eq!(cursor.at(), (1365, 767));
    }

    #[test]
    fn a_touchscreen_touch_takes_the_arrow_away() {
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 1, dy: 1 });
        assert!(cursor.visible());
        cursor.hide();
        assert!(!cursor.visible());
    }

    #[test]
    fn a_band_of_rows_shows_the_arrow_as_the_whole_frame_does() {
        let g = geometry(Depth::Rgb565);
        let row = g.row_bytes();
        let at = (20, 30);
        let mut whole = vec![0x55u8; row * usize::from(g.height)];
        draw(&mut whole, g, at);
        let (first, last) = rows(g, Turn::R0, at);
        assert_eq!((first, last), (30, 48), "nineteen rows at 1:1");
        // The band the arrow covers, and one that cuts it in half.
        for (from, to) in [(first, last), (first + 7, first + 12)] {
            let mut band = vec![0x55u8; row * (to - from + 1)];
            draw_rows(&mut band, g, Turn::R0, at, from);
            assert_eq!(band, whole[from * row..(to + 1) * row]);
        }
        // At the bottom edge the arrow is cut by the panel.
        let low = (20, g.height - 5);
        assert_eq!(
            rows(g, Turn::R0, low),
            (usize::from(g.height) - 5, usize::from(g.height) - 1)
        );
    }

    #[test]
    fn the_arrow_is_black_with_a_white_edge_at_the_pointer() {
        for depth in [Depth::Rgb565, Depth::Bgr888, Depth::Rgb888, Depth::Bgra8888] {
            let g = geometry(depth);
            let bytes = depth.bytes_per_pixel();
            let mut frame = vec![0x7fu8; usize::from(g.width) * usize::from(g.height) * bytes];
            draw(&mut frame, g, (10, 20));

            let pixel =
                |x: usize, y: usize| frame[y * g.row_bytes() + x * bytes..][..bytes].to_vec();
            let ink = |v: u8| {
                let mut out = vec![0u8; bytes];
                fb::convert_into(&[v, v, v, 0xff], depth, &mut out);
                out
            };
            let (black, white) = (ink(0), ink(0xff));
            assert_eq!(pixel(10, 20), white, "the tip is the edge colour");
            assert_eq!(pixel(11, 22), black, "and the body is black");
            assert_eq!(
                pixel(9, 20),
                vec![0x7f; bytes],
                "nothing left of the tip is touched"
            );
        }
    }

    #[test]
    fn the_arrow_is_a_desktops_size_and_doubles_only_on_a_dense_panel() {
        assert_eq!(scale(768), 1, "1366×768 draws the usual arrow");
        assert_eq!(scale(1080), 1);
        assert_eq!(scale(1600), 1);
        assert_eq!(scale(1800), 2, "2880×1800 draws it twice over");
        assert_eq!(scale(2160), 2);
        assert_eq!(scale(240), 1, "a small panel still gets a whole arrow");

        // Drawn at the bottom-right corner, the arrow is clipped and
        // nothing is written past the frame.
        let g = geometry(Depth::Rgb565);
        let mut frame = vec![0u8; usize::from(g.width) * usize::from(g.height) * 2];
        draw(&mut frame, g, (63, 63));
        assert_eq!(
            frame[63 * g.row_bytes()..][..126],
            vec![0u8; 126],
            "the last row keeps everything left of the arrow"
        );
    }
    #[test]
    fn a_mouse_count_moves_the_cursor_by_the_screens_scale() {
        for (w, h, pixels) in [(1920, 1080, 100), (3200, 1800, 200), (3840, 3000, 300)] {
            let mut cursor = Cursor::new(w, h, 1.0);
            cursor.apply(Motion::Move { dx: -10_000, dy: 0 });
            assert_eq!(cursor.at().0, 0);
            cursor.apply(Motion::Move { dx: 100, dy: 0 });
            assert_eq!(cursor.at().0, pixels, "100 counts at {w}x{h}");
        }
        let mut cursor = Cursor::new(3200, 1800, 0.5);
        cursor.apply(Motion::Move { dx: -10_000, dy: 0 });
        cursor.apply(Motion::Move { dx: 100, dy: 0 });
        assert_eq!(cursor.at().0, 100, "--pointer-speed still applies");
    }

    #[test]
    fn the_cursor_stops_at_the_edge_at_either_density() {
        for (w, h) in [(1920, 1080), (3200, 1800)] {
            let mut cursor = Cursor::new(w, h, 1.0);
            cursor.apply(Motion::Move {
                dx: 30_000,
                dy: 30_000,
            });
            assert_eq!(cursor.at(), (w - 1, h - 1));
            cursor.apply(Motion::Move {
                dx: -30_000,
                dy: -30_000,
            });
            assert_eq!(cursor.at(), (0, 0));
        }
    }

    #[test]
    fn a_five_count_tap_on_a_dense_screen_is_still_a_tap() {
        let mut cursor = Cursor::new(3200, 1800, 1.0);
        cursor.apply(Motion::Move { dx: 40, dy: -30 });
        cursor.apply(Motion::Finger(Finger::Down { ms: 1000 }));
        cursor.apply(Motion::Move { dx: 5, dy: 0 });
        let (x, y) = cursor.at();
        assert_eq!(
            cursor.apply(Motion::Finger(Finger::Up { ms: 1050 })),
            Some(Pointed::Touch(Touch {
                x,
                y,
                phase: TouchPhase::Down
            }))
        );
        // Seven counts is aimed somewhere else, at any density. The
        // touch comes long after the first tap's wait, which it ends.
        cursor.apply(Motion::Finger(Finger::Down { ms: 2000 }));
        cursor.apply(Motion::Move { dx: 7, dy: 0 });
        assert_eq!(cursor.apply(Motion::Finger(Finger::Up { ms: 2050 })), None);
    }

    #[test]
    fn a_two_finger_pan_moves_the_content_by_the_scale() {
        let mut cursor = Cursor::new(3200, 1800, 1.0);
        cursor.apply(Motion::Move { dx: 0, dy: 0 });
        let (x, y) = cursor.at();
        assert_eq!(
            cursor.apply(Motion::Pan { dx: 0, dy: 30 }),
            Some(Pointed::Scroll { x, y, dy: -60 })
        );
    }

    /// A contact at `(x, y)` in `phase`, as the cursor reports it.
    fn at(x: u16, y: u16, phase: TouchPhase) -> Option<Pointed> {
        Some(Pointed::Touch(Touch { x, y, phase }))
    }

    #[test]
    fn a_tap_on_the_pad_is_a_click_where_the_pointer_is() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 40, dy: -30 });
        let (x, y) = cursor.at();

        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Down { ms: 1000 }), ms(1000)),
            None
        );
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Up { ms: 1080 }), ms(1080)),
            at(x, y, TouchPhase::Down),
            "a short touch that went nowhere presses where it lifted"
        );
        assert_eq!(cursor.due(ms(1200)), None, "and holds the press a moment");
        assert_eq!(
            cursor.due(ms(1260)),
            at(x, y, TouchPhase::Up),
            "no finger came back, so it was a click"
        );
        assert_eq!(cursor.due(ms(1300)), None, "released once");

        // Far enough to be aimed somewhere else.
        cursor.apply_at(Motion::Finger(Finger::Down { ms: 2000 }), ms(2000));
        cursor.apply(Motion::Move { dx: 20, dy: 0 });
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Up { ms: 2050 }), ms(2050)),
            None
        );

        // Long enough to be a finger resting on the pad.
        cursor.apply_at(Motion::Finger(Finger::Down { ms: 3000 }), ms(3000));
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Up { ms: 3400 }), ms(3400)),
            None
        );

        // The button clicked under the finger, so the touch already did
        // everything it was going to do.
        cursor.apply_at(Motion::Finger(Finger::Down { ms: 4000 }), ms(4000));
        cursor.apply(Motion::Finger(Finger::Cancel));
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Up { ms: 4050 }), ms(4050)),
            None
        );
        assert_eq!(cursor.due(ms(5000)), None);
    }

    #[test]
    fn a_tap_then_a_quick_touch_that_moves_drags_until_the_finger_lifts() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 0, dy: 0 });
        let (x, y) = cursor.at();
        cursor.apply_at(Motion::Finger(Finger::Down { ms: 1000 }), ms(1000));
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Up { ms: 1060 }), ms(1060)),
            at(x, y, TouchPhase::Down)
        );
        // Back on the pad 100 ms later, inside the wait.
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Down { ms: 1160 }), ms(1160)),
            None
        );
        assert_eq!(cursor.due(ms(1300)), None, "the press stays held");
        assert_eq!(
            cursor.apply(Motion::Move { dx: 0, dy: 40 }),
            at(x, y + 40, TouchPhase::Move),
            "and the finger drags"
        );
        assert_eq!(
            cursor.apply(Motion::Move { dx: 0, dy: 60 }),
            at(x, y + 100, TouchPhase::Move)
        );
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Up { ms: 2000 }), ms(2000)),
            at(x, y + 100, TouchPhase::Up),
            "lifting the finger lets go, with no drag lock"
        );
        assert_eq!(cursor.due(ms(3000)), None);
        assert_eq!(
            cursor.apply(Motion::Move { dx: 5, dy: 0 }),
            Some(Pointed::Hover {
                x: x + 5,
                y: y + 100
            }),
            "the next stroke only moves the pointer"
        );
    }

    #[test]
    fn a_double_tap_is_two_clicks() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 0, dy: 0 });
        let (x, y) = cursor.at();
        cursor.apply_at(Motion::Finger(Finger::Down { ms: 1000 }), ms(1000));
        cursor.apply_at(Motion::Finger(Finger::Up { ms: 1060 }), ms(1060));
        cursor.apply_at(Motion::Finger(Finger::Down { ms: 1140 }), ms(1140));
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Up { ms: 1200 }), ms(1200)),
            Some(Pointed::Reclick { x, y })
        );
        assert_eq!(cursor.due(ms(1400)), at(x, y, TouchPhase::Up));
    }

    #[test]
    fn a_slow_second_touch_after_a_tap_is_a_plain_move() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 0, dy: 0 });
        let (x, y) = cursor.at();
        cursor.apply_at(Motion::Finger(Finger::Down { ms: 1000 }), ms(1000));
        cursor.apply_at(Motion::Finger(Finger::Up { ms: 1060 }), ms(1060));
        // 300 ms after the lift, past the wait. The loop had not yet
        // asked, so the touch itself ends the click.
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Down { ms: 1360 }), ms(1360)),
            at(x, y, TouchPhase::Up)
        );
        assert_eq!(
            cursor.apply(Motion::Move { dx: 30, dy: 0 }),
            Some(Pointed::Hover { x: x + 30, y })
        );
        assert_eq!(
            cursor.apply_at(Motion::Finger(Finger::Up { ms: 1500 }), ms(1500)),
            None
        );
    }

    #[test]
    fn two_fingers_during_a_tap_drag_move_the_pointer_and_scroll_nothing() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 0, dy: 0 });
        let (x, y) = cursor.at();
        cursor.apply_at(Motion::Finger(Finger::Down { ms: 1000 }), ms(1000));
        cursor.apply_at(Motion::Finger(Finger::Up { ms: 1060 }), ms(1060));
        cursor.apply_at(Motion::Finger(Finger::Down { ms: 1100 }), ms(1100));
        cursor.apply(Motion::Finger(Finger::Cancel));
        assert_eq!(
            cursor.apply(Motion::Pan { dx: 0, dy: 25 }),
            at(x, y + 25, TouchPhase::Move)
        );
    }

    #[test]
    fn two_fingers_travelling_down_the_pad_drag_the_content_down() {
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 0, dy: 0 });
        let (x, y) = cursor.at();
        assert_eq!(
            cursor.apply(Motion::Pan { dx: 0, dy: 30 }),
            Some(Pointed::Scroll { x, y, dy: -30 }),
            "fingers moving toward the person move the content with them, as on a touchscreen"
        );
        assert_eq!(
            cursor.apply(Motion::Pan { dx: 0, dy: -12 }),
            Some(Pointed::Scroll { x, y, dy: 12 })
        );
        assert_eq!(cursor.at(), (x, y), "and the pointer has not moved");
    }
}
