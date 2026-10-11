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
//! Size: twelve by nineteen pixels at 160 dpi, scaled by a whole number
//! of times for a denser screen, so the edges stay sharp. White with a
//! one-pixel black edge, which is legible on the dark ground every screen
//! draws and on the white of a Words card.

use osk_shell_api::TouchPhase;

use crate::fb::{self, Geometry};
use crate::pointer::{Finger, Motion};
use crate::touch::Touch;

/// Frame pixels of scroll per notch of the wheel, the same distance the
/// desktop shell moves a line.
const WHEEL_NOTCH_PX: i32 = 48;

/// The longest a touch on a buttoned pad can last and still be a tap.
/// A deliberate tap is over well inside this; a finger resting on the
/// pad is not tapping it.
const TAP_MS: u64 = 200;

/// The furthest the cursor may travel during a tap, in frame pixels,
/// measured on the longer axis. A finger never lands and lifts on
/// exactly one point, and a stroke that carries the cursor further than
/// this was aimed somewhere else.
const TAP_TRAVEL_PX: i32 = 6;

/// The dpi the arrow is drawn at 1:1: a desktop's nominal density, so
/// the arrow is the size a person's own computer draws it at that density
/// and twice that on a laptop panel at 160.
const ARROW_DPI: u16 = 96;

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
    /// A tap on the pad, which is a contact down and up at the cursor
    /// without anything between them.
    Click {
        /// Pixels from the left edge.
        x: u16,
        /// Pixels from the top edge.
        y: u16,
    },
    /// A wheel step at the cursor.
    Scroll {
        /// Pixels from the left edge.
        x: u16,
        /// Pixels from the top edge.
        y: u16,
        /// Pixels the content moves; positive moves it up.
        dy: i16,
    },
}

/// Where the pointer is, and whether it should be drawn.
#[derive(Debug)]
pub struct Cursor {
    x: f32,
    y: f32,
    width: u16,
    height: u16,
    speed: f32,
    visible: bool,
    down: bool,
    /// The touch that could still turn out to be a tap: where the cursor
    /// was when the finger landed, and when it landed.
    tap: Option<(u16, u16, u64)>,
}

impl Cursor {
    /// A cursor in the middle of a `width × height` panel, hidden until
    /// something moves it, at `--pointer-speed` device pixels per unit.
    pub fn new(width: u16, height: u16, speed: f32) -> Cursor {
        Cursor {
            x: f32::from(width) / 2.0,
            y: f32::from(height) / 2.0,
            width,
            height,
            speed,
            visible: false,
            down: false,
            tap: None,
        }
    }

    /// Where the cursor is, in panel pixels.
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

    /// Applies one device motion, returning what the core should be told.
    pub fn apply(&mut self, motion: Motion) -> Option<Pointed> {
        match motion {
            Motion::Move { dx, dy } => {
                self.x = (self.x + dx as f32 * self.speed)
                    .clamp(0.0, f32::from(self.width).max(1.0) - 1.0);
                self.y = (self.y + dy as f32 * self.speed)
                    .clamp(0.0, f32::from(self.height).max(1.0) - 1.0);
                self.visible = true;
                let (x, y) = self.at();
                self.down.then_some(Pointed::Touch(Touch {
                    x,
                    y,
                    phase: TouchPhase::Move,
                }))
            }
            Motion::Button(down) => {
                if self.down == down {
                    return None;
                }
                self.down = down;
                let (x, y) = self.at();
                Some(Pointed::Touch(Touch {
                    x,
                    y,
                    phase: if down {
                        TouchPhase::Down
                    } else {
                        TouchPhase::Up
                    },
                }))
            }
            Motion::Finger(finger) => self.finger(finger),
            Motion::Pan { dy } => {
                // Natural scrolling: two fingers drag the content the
                // way one finger drags it on a touchscreen, so fingers
                // moving toward the person move the content down, which
                // is the core's negative.
                let dy = -(dy as f32 * self.speed).round() as i32;
                if dy == 0 {
                    return None;
                }
                let (x, y) = self.at();
                Some(Pointed::Scroll {
                    x,
                    y,
                    dy: dy.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
                })
            }
            Motion::Wheel(notches) => {
                // evdev counts a turn away from the person as positive,
                // which moves the content down; the core's positive moves
                // it up.
                let dy =
                    (-notches * WHEEL_NOTCH_PX).clamp(i32::from(i16::MIN), i32::from(i16::MAX));
                let (x, y) = self.at();
                Some(Pointed::Scroll {
                    x,
                    y,
                    dy: dy as i16,
                })
            }
        }
    }

    /// What one contact on a buttoned pad did. A tap is a touch that
    /// lifted inside [`TAP_MS`] having carried the cursor no further
    /// than [`TAP_TRAVEL_PX`]; anything else moved the cursor and
    /// nothing more.
    fn finger(&mut self, finger: Finger) -> Option<Pointed> {
        let (x, y) = self.at();
        match finger {
            Finger::Down { ms } => {
                self.tap = Some((x, y, ms));
                None
            }
            Finger::Cancel => {
                self.tap = None;
                None
            }
            Finger::Up { ms } => {
                let (from_x, from_y, landed) = self.tap.take()?;
                let travel = (i32::from(x) - i32::from(from_x))
                    .abs()
                    .max((i32::from(y) - i32::from(from_y)).abs());
                (ms.saturating_sub(landed) <= TAP_MS && travel <= TAP_TRAVEL_PX)
                    .then_some(Pointed::Click { x, y })
            }
        }
    }
}

/// How many device pixels one pixel of the arrow is at `dpi`: the
/// nearest whole multiple of a desktop's density, and never none.
pub fn scale(dpi: u16) -> u16 {
    ((dpi + ARROW_DPI / 2) / ARROW_DPI).max(1)
}

/// Draws the arrow into a converted frame, whose rows are packed.
///
/// `out` is the shell's own copy of the frame, in the panel's format;
/// the core's frame is untouched.
pub fn draw(out: &mut [u8], geometry: Geometry, at: (u16, u16), dpi: u16) {
    let scale = usize::from(scale(dpi));
    let bytes = geometry.depth.bytes_per_pixel();
    let row = geometry.row_bytes();
    let (width, height) = (usize::from(geometry.width), usize::from(geometry.height));
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
            let mut pixel = vec![0u8; bytes];
            fb::convert_into(&ink, geometry.depth, &mut pixel);
            for sy in 0..scale {
                let y = top + r * scale + sy;
                if y >= height {
                    break;
                }
                for sx in 0..scale {
                    let x = left + c * scale + sx;
                    if x >= width {
                        break;
                    }
                    let at = y * row + x * bytes;
                    if let Some(slice) = out.get_mut(at..at + bytes) {
                        slice.copy_from_slice(&pixel);
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
        assert_eq!(cursor.apply(Motion::Move { dx: 40, dy: -30 }), None);
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
            cursor.apply(Motion::Wheel(1)),
            Some(Pointed::Scroll { x, y, dy: -48 }),
            "turning the wheel away moves the content down"
        );
        assert_eq!(
            cursor.apply(Motion::Wheel(-2)),
            Some(Pointed::Scroll { x, y, dy: 96 })
        );
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
    fn the_arrow_is_black_with_a_white_edge_at_the_pointer() {
        for depth in [Depth::Rgb565, Depth::Bgr888, Depth::Rgb888, Depth::Bgra8888] {
            let g = geometry(depth);
            let bytes = depth.bytes_per_pixel();
            let mut frame = vec![0x7fu8; usize::from(g.width) * usize::from(g.height) * bytes];
            draw(&mut frame, g, (10, 20), 96);

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
    fn the_arrow_grows_with_the_screens_density_and_stays_on_it() {
        assert_eq!(scale(96), 1);
        assert_eq!(scale(72), 1, "a coarse screen still gets a whole arrow");
        assert_eq!(scale(160), 2, "a laptop panel draws it twice over");
        assert_eq!(scale(227), 2);
        assert_eq!(scale(286), 3);

        // Drawn at the bottom-right corner, the arrow is clipped and
        // nothing is written past the frame.
        let g = geometry(Depth::Rgb565);
        let mut frame = vec![0u8; usize::from(g.width) * usize::from(g.height) * 2];
        draw(&mut frame, g, (63, 63), 320);
        assert_eq!(
            frame[63 * g.row_bytes()..][..126],
            vec![0u8; 126],
            "the last row keeps everything left of the arrow"
        );
    }
    #[test]
    fn a_tap_on_the_pad_is_a_click_where_the_pointer_is() {
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 40, dy: -30 });
        let (x, y) = cursor.at();

        assert_eq!(
            cursor.apply(Motion::Finger(Finger::Down { ms: 1000 })),
            None
        );
        assert_eq!(
            cursor.apply(Motion::Finger(Finger::Up { ms: 1080 })),
            Some(Pointed::Click { x, y }),
            "a short touch that went nowhere clicks"
        );

        // Far enough to be aimed somewhere else.
        cursor.apply(Motion::Finger(Finger::Down { ms: 2000 }));
        cursor.apply(Motion::Move { dx: 20, dy: 0 });
        assert_eq!(cursor.apply(Motion::Finger(Finger::Up { ms: 2050 })), None);

        // Long enough to be a finger resting on the pad.
        cursor.apply(Motion::Finger(Finger::Down { ms: 3000 }));
        assert_eq!(cursor.apply(Motion::Finger(Finger::Up { ms: 3400 })), None);

        // The button clicked under the finger, so the touch already did
        // everything it was going to do.
        cursor.apply(Motion::Finger(Finger::Down { ms: 4000 }));
        cursor.apply(Motion::Finger(Finger::Cancel));
        assert_eq!(cursor.apply(Motion::Finger(Finger::Up { ms: 4050 })), None);
    }

    #[test]
    fn two_fingers_travelling_down_the_pad_drag_the_content_down() {
        let mut cursor = Cursor::new(1920, 1080, 1.0);
        cursor.apply(Motion::Move { dx: 0, dy: 0 });
        let (x, y) = cursor.at();
        assert_eq!(
            cursor.apply(Motion::Pan { dy: 30 }),
            Some(Pointed::Scroll { x, y, dy: -30 }),
            "fingers moving toward the person move the content with them, as on a touchscreen"
        );
        assert_eq!(
            cursor.apply(Motion::Pan { dy: -12 }),
            Some(Pointed::Scroll { x, y, dy: 12 })
        );
        assert_eq!(cursor.at(), (x, y), "and the pointer has not moved");
    }
}
