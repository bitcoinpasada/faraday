//! A HID touchscreen's axis ranges, read from its report descriptor.
//!
//! A laptop's touchscreen reports positions in a range of its own (0 to
//! 4095, 0 to 9600, whatever its maker chose), and a position is only a
//! place on the panel once it is scaled by that range. The driver knows
//! it, and hands it out through the `EVIOCGABS` ioctl, which this crate
//! does not make: it has no `unsafe` and no `libc` (`evdev.rs`). The
//! driver's range is not invented, though: `hid-multitouch` and
//! `hid-input` give each axis the Logical Minimum and Logical Maximum
//! the device's HID report descriptor declares for it, and the kernel
//! publishes that descriptor as a file,
//! `/sys/class/input/eventN/device/device/report_descriptor`. Reading
//! the same bytes and finding the same two numbers gives the same range
//! with a file read and no ioctl.
//!
//! Only what that needs is parsed: short items, the global Usage Page,
//! Logical Minimum, Logical Maximum, Push and Pop, local Usages, and
//! Collection and End Collection to know which application an Input's
//! X and Y belong to. A touch screen's fingers (Digitizer page, usage
//! Touch Screen) and a pen's (usage Pen) are told apart, since one
//! device often has both with different ranges. Long items are skipped.
//! The descriptor is the device's to write, so it is read as untrusted
//! bytes: a truncated item ends the parse, and a range that is empty or
//! backwards is no range.

/// An axis range as the descriptor declares it: both ends inclusive.
pub type Axis = (i32, i32);

/// The X and Y ranges of one kind of contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Logical minimum and maximum of X.
    pub x: Axis,
    /// Logical minimum and maximum of Y.
    pub y: Axis,
}

/// The ranges a descriptor declares, by application.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ranges {
    /// A touch screen's fingers: what `hid-multitouch` reports as
    /// `ABS_MT_POSITION_X`/`_Y`.
    pub touch: Option<Range>,
    /// A pen on the same screen, which the kernel reports on a node of its
    /// own.
    pub pen: Option<Range>,
}

impl Ranges {
    /// The range for a node: a multitouch node is the fingers; any other
    /// is the pen when the device has one, and the fingers when it has not.
    pub fn for_node(&self, multitouch: bool) -> Option<Range> {
        if multitouch {
            self.touch.or(self.pen)
        } else {
            self.pen.or(self.touch)
        }
    }
}

/// Generic Desktop X, as a page and usage in one.
const USAGE_X: u32 = 0x0001_0030;
/// Generic Desktop Y.
const USAGE_Y: u32 = 0x0001_0031;
/// Digitizer: Pen.
const USAGE_PEN: u32 = 0x000d_0002;
/// Digitizer: Touch Screen.
const USAGE_TOUCH_SCREEN: u32 = 0x000d_0004;

/// The global items this parser keeps.
#[derive(Debug, Clone, Copy, Default)]
struct Globals {
    page: u32,
    min: i32,
    max: i32,
}

/// Parses a report descriptor. Anything it cannot read ends the parse
/// with what was found so far.
pub fn ranges(descriptor: &[u8]) -> Ranges {
    let mut out = Ranges::default();
    let mut g = Globals::default();
    let mut stack: Vec<Globals> = Vec::new();
    let mut usages: Vec<u32> = Vec::new();
    // The usage of each open collection; the outermost application one
    // is what an Input belongs to.
    let mut collections: Vec<u32> = Vec::new();
    // X and Y as found in the open application, before both are known.
    let mut x: Option<Axis> = None;
    let mut y: Option<Axis> = None;
    let mut i = 0;
    while i < descriptor.len() {
        let prefix = descriptor[i];
        // A long item: its size is the next byte.
        if prefix == 0xfe {
            let Some(&size) = descriptor.get(i + 1) else {
                break;
            };
            i += 3 + usize::from(size);
            continue;
        }
        let size = match prefix & 3 {
            3 => 4,
            n => usize::from(n),
        };
        let Some(data) = descriptor.get(i + 1..i + 1 + size) else {
            break;
        };
        i += 1 + size;
        let mut le = [0u8; 4];
        le[..size].copy_from_slice(data);
        let unsigned = u32::from_le_bytes(le);
        let signed = match size {
            1 => i32::from(data[0] as i8),
            2 => i32::from(i16::from_le_bytes([data[0], data[1]])),
            4 => unsigned as i32,
            _ => 0,
        };
        let tag = prefix >> 4;
        match (prefix >> 2) & 3 {
            // Main.
            0 => {
                match tag {
                    // Input.
                    0x8 => {
                        let app = collections.first().copied();
                        if matches!(app, Some(USAGE_TOUCH_SCREEN | USAGE_PEN)) {
                            for &u in &usages {
                                if u == USAGE_X && x.is_none() {
                                    x = Some((g.min, g.max));
                                }
                                if u == USAGE_Y && y.is_none() {
                                    y = Some((g.min, g.max));
                                }
                            }
                        }
                    }
                    // Collection.
                    0xa => collections.push(usages.first().copied().unwrap_or(0)),
                    // End Collection: closing the application keeps what
                    // it declared.
                    0xc => {
                        let closed = collections.pop();
                        if collections.is_empty() {
                            let found = match (x.take(), y.take()) {
                                (Some(x), Some(y)) if x.0 < x.1 && y.0 < y.1 => {
                                    Some(Range { x, y })
                                }
                                _ => None,
                            };
                            let slot = match closed {
                                Some(USAGE_TOUCH_SCREEN) => Some(&mut out.touch),
                                Some(USAGE_PEN) => Some(&mut out.pen),
                                _ => None,
                            };
                            if let Some(slot) = slot
                                && slot.is_none()
                            {
                                *slot = found;
                            }
                        }
                    }
                    _ => {}
                }
                usages.clear();
            }
            // Global.
            1 => match tag {
                0x0 => g.page = unsigned,
                0x1 => g.min = signed,
                // As the kernel reads it: signed only when the minimum is.
                0x2 => g.max = if g.min < 0 { signed } else { unsigned as i32 },
                0xa => stack.push(g),
                0xb => g = stack.pop().unwrap_or_default(),
                _ => {}
            },
            // Local: a Usage of one or two bytes is on the current page.
            2 if tag == 0x0 => usages.push(if size == 4 {
                unsigned
            } else {
                g.page << 16 | unsigned
            }),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Windows-8 touch screen with a pen, as an ELAN panel declares
    /// them (cut to the items that matter): fingers 0..=3200 × 0..=1800,
    /// a pen 0..=12800 × 0..=7200 behind a Push and Pop, and a
    /// configuration collection after both.
    const ELAN: &[u8] = &[
        0x05, 0x0d, // Usage Page (Digitizer)
        0x09, 0x04, // Usage (Touch Screen)
        0xa1, 0x01, // Collection (Application)
        0x09, 0x22, //   Usage (Finger)
        0xa1, 0x02, //   Collection (Logical)
        0x09, 0x42, //     Usage (Tip Switch)
        0x15, 0x00, //     Logical Minimum (0)
        0x25, 0x01, //     Logical Maximum (1)
        0x81, 0x02, //     Input
        0xa4, //     Push
        0x05, 0x01, //     Usage Page (Generic Desktop)
        0x26, 0x80, 0x0c, //     Logical Maximum (3200)
        0x09, 0x30, //     Usage (X)
        0x81, 0x02, //     Input
        0x26, 0x08, 0x07, //     Logical Maximum (1800)
        0x09, 0x31, //     Usage (Y)
        0x81, 0x02, //     Input
        0xb4, //     Pop
        0xc0, //   End Collection
        0xc0, // End Collection
        0x05, 0x0d, // Usage Page (Digitizer)
        0x09, 0x02, // Usage (Pen)
        0xa1, 0x01, // Collection (Application)
        0x09, 0x20, //   Usage (Stylus)
        0xa1, 0x00, //   Collection (Physical)
        0x05, 0x01, //     Usage Page (Generic Desktop)
        0x15, 0x00, //     Logical Minimum (0)
        0x26, 0x00, 0x32, //     Logical Maximum (12800)
        0x09, 0x30, //     Usage (X)
        0x81, 0x02, //     Input
        0x26, 0x20, 0x1c, //     Logical Maximum (7200)
        0x09, 0x31, //     Usage (Y)
        0x81, 0x02, //     Input
        0xc0, //   End Collection
        0xc0, // End Collection
        0x05, 0x0d, // Usage Page (Digitizer)
        0x09, 0x0e, // Usage (Device Configuration)
        0xa1, 0x01, // Collection (Application)
        0x05, 0x01, //   Usage Page (Generic Desktop)
        0x25, 0x0a, //   Logical Maximum (10)
        0x09, 0x30, //   Usage (X)
        0xb1, 0x02, //   Feature
        0xc0, // End Collection
    ];

    #[test]
    fn a_touch_screens_fingers_and_pen_have_their_own_ranges() {
        let r = ranges(ELAN);
        assert_eq!(
            r.touch,
            Some(Range {
                x: (0, 3200),
                y: (0, 1800)
            })
        );
        assert_eq!(
            r.pen,
            Some(Range {
                x: (0, 12800),
                y: (0, 7200)
            })
        );
        assert_eq!(r.for_node(true), r.touch);
        assert_eq!(r.for_node(false), r.pen);
    }

    #[test]
    fn a_mouse_or_a_keyboard_declares_no_range() {
        // Generic Desktop Mouse: X and Y, relative, in a Mouse application.
        let mouse = [
            0x05, 0x01, 0x09, 0x02, 0xa1, 0x01, 0x09, 0x01, 0xa1, 0x00, 0x09, 0x30, 0x09, 0x31,
            0x15, 0x81, 0x25, 0x7f, 0x81, 0x06, 0xc0, 0xc0,
        ];
        assert_eq!(ranges(&mouse), Ranges::default());
    }

    #[test]
    fn a_maximum_is_unsigned_unless_the_minimum_is_negative() {
        // 0xff as a one-byte Logical Maximum after a minimum of 0 is 255,
        // after a minimum of -1 it is -1 (and the range is then empty).
        let mut d = vec![0x05, 0x0d, 0x09, 0x04, 0xa1, 0x01, 0x05, 0x01, 0x15, 0x00];
        d.extend([0x25, 0xff, 0x09, 0x30, 0x09, 0x31, 0x81, 0x02, 0xc0]);
        assert_eq!(
            ranges(&d).touch,
            Some(Range {
                x: (0, 255),
                y: (0, 255)
            })
        );
        d[9] = 0xff;
        assert_eq!(ranges(&d).touch, None);
    }

    #[test]
    fn a_truncated_or_junk_descriptor_gives_no_range_and_no_panic() {
        for cut in 0..ELAN.len() {
            let r = ranges(&ELAN[..cut]);
            // What was found before the cut is whole.
            if let Some(t) = r.touch {
                assert_eq!(t.x, (0, 3200));
            }
        }
        let junk: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
        let _ = ranges(&junk);
        let _ = ranges(&[0xfe, 0xff]);
        let _ = ranges(&[0xa1, 0x01, 0xc0, 0xc0, 0xc0]);
    }
}
