//! The touch controller: turning its packets into
//! [`osk_shell_api::Event::Touch`].
//!
//! A controller reports multitouch (`ABS_MT_POSITION_X`,
//! `ABS_MT_POSITION_Y`, `ABS_MT_TRACKING_ID`) and single touch (`ABS_X`,
//! `ABS_Y`, `BTN_TOUCH`); both are handled, and only the first contact is
//! used, because nothing in OpenSigner is a gesture. Which devices are
//! read, and why, is `evdev.rs`.
//!
//! Coordinates: a controller reports in a grid of its own, which need not
//! be the panel's — the Goodix on the 480×640 portrait panel reports in
//! 640×480 — so a raw position is scaled by the panel size over the grid
//! and clamped, exactly as the SeedSigner fork's `touch.py` does. The fork
//! asks the driver for its real ranges with an `EVIOCGABS` ioctl; this
//! crate forbids `unsafe` and has no `libc`. A laptop's touchscreen is a
//! HID device, and its ranges are the ones its report descriptor
//! declares, which `hid.rs` reads from sysfs ([`Parser::with_range`]).
//! The Pi's panel is not HID: its grid is told to the shell by
//! `--touch-grid`, and the name to look for by `--touch-name`, which the
//! image writes from the panel directory's `panel.conf`.
//!
//! Packets end at `EV_SYN`, and a touch is delivered then: a tracking id
//! of zero or more opens a contact (Down) or continues one (Move), and a
//! tracking id of −1, or `BTN_TOUCH` going to zero, closes it (Up).
//!
//! A multitouch screen reports every finger, each in a slot of its own
//! (`ABS_MT_SLOT`). Only slot 0's are read, the finger that touched first
//! while none was down: a second finger neither moves the first nor ends
//! it. Once a device has said it reports multitouch positions, its
//! `ABS_X`/`ABS_Y`, the kernel's copy of whichever finger is oldest, are
//! not read.

use osk_shell_api::TouchPhase;

use crate::evdev::{
    ABS_MT_POSITION_X, ABS_MT_POSITION_Y, ABS_MT_SLOT, ABS_MT_TRACKING_ID, ABS_X, ABS_Y, BTN_TOUCH,
    EV_ABS, EV_KEY, EV_SYN, RawEvent, SYN_REPORT,
};

/// The name to look for when no `--touch-name` is given: the controller
/// on the panel this shell was written for.
pub const DEFAULT_TOUCH_NAME: &str = "Goodix";
/// The reporting grid to assume when no `--touch-grid` is given: what
/// that controller reports for the 480×640 panel.
pub const DEFAULT_TOUCH_GRID: (u16, u16) = (640, 480);

/// A contact to hand to the core, already in panel pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Touch {
    /// Pixels from the left edge.
    pub x: u16,
    /// Pixels from the top edge.
    pub y: u16,
    /// Down, Move or Up.
    pub phase: TouchPhase,
}

/// The packet state machine: raw events in, one touch per `EV_SYN` out.
#[derive(Debug)]
pub struct Parser {
    width: u16,
    height: u16,
    /// Where the controller's x and y start: zero on a grid.
    origin: (i64, i64),
    /// The controller's reporting grid, which a position is scaled from.
    grid_width: i64,
    /// The reporting grid's other axis.
    grid_height: i64,
    /// The slot the multitouch events that follow belong to.
    slot: i32,
    /// The device has reported a multitouch position: its single-touch
    /// axes are the kernel's copy and are not read.
    multitouch: bool,
    x: i64,
    y: i64,
    /// A position has been reported at least once, so the coordinates mean
    /// something.
    positioned: bool,
    /// A contact is open.
    down: bool,
    /// This packet opened or continued a contact.
    saw_contact: bool,
    /// This packet ended a contact.
    saw_release: bool,
    /// Where the last delivered touch was, so a packet that repeats a
    /// position is not a Move.
    last: Option<(u16, u16)>,
}

impl Parser {
    /// A parser for a panel `width × height` pixels whose controller
    /// reports in a `grid` of its own. A grid axis of zero would be a
    /// division by zero, so it is read as one.
    pub fn new(width: u16, height: u16, grid: (u16, u16)) -> Parser {
        Parser {
            width,
            height,
            origin: (0, 0),
            grid_width: i64::from(grid.0.max(1)),
            grid_height: i64::from(grid.1.max(1)),
            slot: 0,
            multitouch: false,
            x: 0,
            y: 0,
            positioned: false,
            down: false,
            saw_contact: false,
            saw_release: false,
            last: None,
        }
    }

    /// A parser for a panel `width × height` pixels whose controller
    /// reports in the ranges a HID descriptor declares, both ends
    /// inclusive.
    pub fn with_range(width: u16, height: u16, range: crate::hid::Range) -> Parser {
        let span = |(lo, hi): crate::hid::Axis| (i64::from(hi) - i64::from(lo) + 1).max(1);
        Parser {
            origin: (i64::from(range.x.0), i64::from(range.y.0)),
            grid_width: span(range.x),
            grid_height: span(range.y),
            ..Parser::new(width, height, (1, 1))
        }
    }

    /// Raw grid position → panel pixel, scaled and clamped.
    fn transform(&self) -> (u16, u16) {
        let x = (self.x - self.origin.0) * i64::from(self.width) / self.grid_width;
        let y = (self.y - self.origin.1) * i64::from(self.height) / self.grid_height;
        (
            x.clamp(0, i64::from(self.width) - 1) as u16,
            y.clamp(0, i64::from(self.height) - 1) as u16,
        )
    }

    /// Feeds one raw event; returns a touch when the packet it closed
    /// carries one.
    pub fn feed(&mut self, ev: RawEvent) -> Option<Touch> {
        match ev.kind {
            EV_ABS => {
                match ev.code {
                    ABS_MT_SLOT => self.slot = ev.value,
                    // Another finger's.
                    ABS_MT_POSITION_X | ABS_MT_POSITION_Y | ABS_MT_TRACKING_ID
                        if self.slot != 0 => {}
                    ABS_X | ABS_Y if self.multitouch => {}
                    ABS_MT_POSITION_X | ABS_X => {
                        self.multitouch |= ev.code == ABS_MT_POSITION_X;
                        self.x = i64::from(ev.value);
                        self.positioned = true;
                    }
                    ABS_MT_POSITION_Y | ABS_Y => {
                        self.multitouch |= ev.code == ABS_MT_POSITION_Y;
                        self.y = i64::from(ev.value);
                        self.positioned = true;
                    }
                    ABS_MT_TRACKING_ID => {
                        if ev.value >= 0 {
                            self.saw_contact = true;
                        } else {
                            self.saw_release = true;
                        }
                    }
                    _ => {}
                }
                None
            }
            EV_KEY if ev.code == BTN_TOUCH => {
                if ev.value == 0 {
                    self.saw_release = true;
                } else {
                    self.saw_contact = true;
                }
                None
            }
            EV_SYN if ev.code == SYN_REPORT => self.sync(),
            _ => None,
        }
    }

    /// End of packet: decide what, if anything, the packet said.
    fn sync(&mut self) -> Option<Touch> {
        let (contact, release) = (self.saw_contact, self.saw_release);
        self.saw_contact = false;
        self.saw_release = false;

        if release {
            if !self.down {
                return None;
            }
            self.down = false;
            // A release reports no new position; it happens where the
            // contact last was.
            let (x, y) = self.last.unwrap_or_else(|| self.transform());
            self.last = None;
            return Some(Touch {
                x,
                y,
                phase: TouchPhase::Up,
            });
        }

        if !self.positioned {
            return None;
        }
        let (x, y) = self.transform();

        if contact && !self.down {
            self.down = true;
            self.last = Some((x, y));
            return Some(Touch {
                x,
                y,
                phase: TouchPhase::Down,
            });
        }

        if self.down && self.last != Some((x, y)) {
            self.last = Some((x, y));
            return Some(Touch {
                x,
                y,
                phase: TouchPhase::Move,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::evdev::{EVENT_SIZE_32, EVENT_SIZE_64, decode};

    /// One `struct input_event` of `size` bytes with a zero timestamp.
    fn event(size: usize, kind: u16, code: u16, value: i32) -> Vec<u8> {
        let mut bytes = vec![0u8; size - 8];
        bytes.extend_from_slice(&kind.to_le_bytes());
        bytes.extend_from_slice(&code.to_le_bytes());
        bytes.extend_from_slice(&value.to_le_bytes());
        bytes
    }

    /// A whole Goodix tap: press, two moves, release.
    fn tap_bytes(size: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for e in [
            // press at raw (320, 240): the middle of the reporting grid
            event(size, EV_ABS, ABS_MT_TRACKING_ID, 0),
            event(size, EV_ABS, ABS_MT_POSITION_X, 320),
            event(size, EV_ABS, ABS_MT_POSITION_Y, 240),
            event(size, EV_SYN, SYN_REPORT, 0),
            // the same position again: not a move
            event(size, EV_ABS, ABS_MT_TRACKING_ID, 0),
            event(size, EV_SYN, SYN_REPORT, 0),
            // a real move
            event(size, EV_ABS, ABS_MT_POSITION_X, 640),
            event(size, EV_SYN, SYN_REPORT, 0),
            // release
            event(size, EV_ABS, ABS_MT_TRACKING_ID, -1),
            event(size, EV_SYN, SYN_REPORT, 0),
        ] {
            out.extend_from_slice(&e);
        }
        out
    }

    fn run(bytes: &[u8], size: usize) -> Vec<Touch> {
        let mut parser = Parser::new(480, 640, DEFAULT_TOUCH_GRID);
        bytes
            .chunks_exact(size)
            .filter_map(|c| decode(c, size))
            .filter_map(|e| parser.feed(e))
            .collect()
    }

    #[test]
    fn a_tap_decodes_the_same_at_both_struct_sizes() {
        for size in [EVENT_SIZE_32, EVENT_SIZE_64] {
            let touches = run(&tap_bytes(size), size);
            assert_eq!(
                touches,
                vec![
                    // 320 × 480 / 640 = 240; 240 × 640 / 480 = 320
                    Touch {
                        x: 240,
                        y: 320,
                        phase: TouchPhase::Down
                    },
                    // 640 is off the right edge of the grid: clamped
                    Touch {
                        x: 479,
                        y: 320,
                        phase: TouchPhase::Move
                    },
                    Touch {
                        x: 479,
                        y: 320,
                        phase: TouchPhase::Up
                    },
                ],
                "struct input_event of {size} bytes"
            );
        }
    }

    #[test]
    fn single_touch_devices_work_too() {
        let size = EVENT_SIZE_32;
        let mut bytes = Vec::new();
        for e in [
            event(size, EV_KEY, BTN_TOUCH, 1),
            event(size, EV_ABS, ABS_X, 0),
            event(size, EV_ABS, ABS_Y, 0),
            event(size, EV_SYN, SYN_REPORT, 0),
            event(size, EV_KEY, BTN_TOUCH, 0),
            event(size, EV_SYN, SYN_REPORT, 0),
        ] {
            bytes.extend_from_slice(&e);
        }
        assert_eq!(
            run(&bytes, size),
            vec![
                Touch {
                    x: 0,
                    y: 0,
                    phase: TouchPhase::Down
                },
                Touch {
                    x: 0,
                    y: 0,
                    phase: TouchPhase::Up
                },
            ]
        );
    }

    #[test]
    fn a_contact_with_no_position_yet_is_not_a_touch() {
        let size = EVENT_SIZE_32;
        let mut bytes = Vec::new();
        for e in [
            event(size, EV_ABS, ABS_MT_TRACKING_ID, 0),
            event(size, EV_SYN, SYN_REPORT, 0),
        ] {
            bytes.extend_from_slice(&e);
        }
        assert_eq!(run(&bytes, size), vec![]);
    }

    #[test]
    fn a_release_with_no_contact_is_ignored() {
        let size = EVENT_SIZE_64;
        let mut bytes = Vec::new();
        for e in [
            event(size, EV_ABS, ABS_MT_TRACKING_ID, -1),
            event(size, EV_SYN, SYN_REPORT, 0),
        ] {
            bytes.extend_from_slice(&e);
        }
        assert_eq!(run(&bytes, size), vec![]);
    }

    #[test]
    fn negative_positions_clamp_to_the_top_left() {
        let size = EVENT_SIZE_32;
        let mut bytes = Vec::new();
        for e in [
            event(size, EV_ABS, ABS_MT_TRACKING_ID, 0),
            event(size, EV_ABS, ABS_MT_POSITION_X, -40),
            event(size, EV_ABS, ABS_MT_POSITION_Y, -40),
            event(size, EV_SYN, SYN_REPORT, 0),
        ] {
            bytes.extend_from_slice(&e);
        }
        assert_eq!(
            run(&bytes, size),
            vec![Touch {
                x: 0,
                y: 0,
                phase: TouchPhase::Down
            }]
        );
    }

    /// Events of one size, from (type, code, value) triples.
    fn packets(size: usize, events: &[(u16, u16, i32)]) -> Vec<u8> {
        events
            .iter()
            .flat_map(|&(k, c, v)| event(size, k, c, v))
            .collect()
    }

    fn run_with(parser: &mut Parser, bytes: &[u8], size: usize) -> Vec<Touch> {
        bytes
            .chunks_exact(size)
            .filter_map(|c| decode(c, size))
            .filter_map(|e| parser.feed(e))
            .collect()
    }

    #[test]
    fn a_laptop_touchscreen_is_scaled_by_its_descriptors_range() {
        // A 1920 × 1080 panel whose controller reports 0..=3200 by
        // 0..=1800, the kernel's single-touch copy riding along.
        let range = crate::hid::Range {
            x: (0, 3200),
            y: (0, 1800),
        };
        let mut parser = Parser::with_range(1920, 1080, range);
        let size = EVENT_SIZE_64;
        let bytes = packets(
            size,
            &[
                (EV_ABS, ABS_MT_SLOT, 0),
                (EV_ABS, ABS_MT_TRACKING_ID, 7),
                (EV_ABS, ABS_MT_POSITION_X, 1600),
                (EV_ABS, ABS_MT_POSITION_Y, 900),
                (EV_KEY, BTN_TOUCH, 1),
                (EV_ABS, ABS_X, 1600),
                (EV_ABS, ABS_Y, 900),
                (EV_SYN, SYN_REPORT, 0),
                (EV_ABS, ABS_MT_POSITION_X, 3200),
                (EV_ABS, ABS_MT_POSITION_Y, 1800),
                (EV_ABS, ABS_X, 3200),
                (EV_ABS, ABS_Y, 1800),
                (EV_SYN, SYN_REPORT, 0),
            ],
        );
        assert_eq!(
            run_with(&mut parser, &bytes, size),
            vec![
                // 1600 × 1920 / 3201 and 900 × 1080 / 1801: the middle.
                Touch {
                    x: 959,
                    y: 539,
                    phase: TouchPhase::Down
                },
                // The far corner is the last pixel, not past it.
                Touch {
                    x: 1919,
                    y: 1079,
                    phase: TouchPhase::Move
                },
            ]
        );
    }

    #[test]
    fn a_range_that_does_not_start_at_zero_is_moved_to_the_corner() {
        let range = crate::hid::Range {
            x: (100, 1099),
            y: (-50, 949),
        };
        let mut parser = Parser::with_range(1000, 1000, range);
        let size = EVENT_SIZE_64;
        let bytes = packets(
            size,
            &[
                (EV_KEY, BTN_TOUCH, 1),
                (EV_ABS, ABS_X, 100),
                (EV_ABS, ABS_Y, -50),
                (EV_SYN, SYN_REPORT, 0),
            ],
        );
        assert_eq!(
            run_with(&mut parser, &bytes, size),
            vec![Touch {
                x: 0,
                y: 0,
                phase: TouchPhase::Down
            }]
        );
    }

    #[test]
    fn a_second_finger_neither_moves_nor_ends_the_first() {
        let mut parser = Parser::new(480, 640, DEFAULT_TOUCH_GRID);
        let size = EVENT_SIZE_64;
        let bytes = packets(
            size,
            &[
                // First finger, slot 0.
                (EV_ABS, ABS_MT_SLOT, 0),
                (EV_ABS, ABS_MT_TRACKING_ID, 1),
                (EV_ABS, ABS_MT_POSITION_X, 320),
                (EV_ABS, ABS_MT_POSITION_Y, 240),
                (EV_KEY, BTN_TOUCH, 1),
                (EV_ABS, ABS_X, 320),
                (EV_ABS, ABS_Y, 240),
                (EV_SYN, SYN_REPORT, 0),
                // Second finger, slot 1, far away.
                (EV_ABS, ABS_MT_SLOT, 1),
                (EV_ABS, ABS_MT_TRACKING_ID, 2),
                (EV_ABS, ABS_MT_POSITION_X, 10),
                (EV_ABS, ABS_MT_POSITION_Y, 10),
                (EV_SYN, SYN_REPORT, 0),
                // It lifts.
                (EV_ABS, ABS_MT_TRACKING_ID, -1),
                (EV_SYN, SYN_REPORT, 0),
                // The first moves, its slot named again.
                (EV_ABS, ABS_MT_SLOT, 0),
                (EV_ABS, ABS_MT_POSITION_X, 0),
                (EV_ABS, ABS_X, 0),
                (EV_SYN, SYN_REPORT, 0),
                // And lifts.
                (EV_ABS, ABS_MT_TRACKING_ID, -1),
                (EV_KEY, BTN_TOUCH, 0),
                (EV_SYN, SYN_REPORT, 0),
            ],
        );
        assert_eq!(
            run_with(&mut parser, &bytes, size),
            vec![
                Touch {
                    x: 240,
                    y: 320,
                    phase: TouchPhase::Down
                },
                Touch {
                    x: 0,
                    y: 320,
                    phase: TouchPhase::Move
                },
                Touch {
                    x: 0,
                    y: 320,
                    phase: TouchPhase::Up
                },
            ]
        );
    }
}
