//! Which way the panel is (`docs/ORIENTATION.md` §2): whether it is
//! portrait natively, how dense it is, whether Rotation is offered on it,
//! and which quarter turn shows the frame upright in the orientation that
//! is not its native one.
//!
//! Three things say it, read once at start: the framebuffer's size,
//! which decides; the EDID base block the firmware passed, which gives
//! the diagonal and the density; and the machine's row on [`MACHINES`],
//! which says how the panel is mounted in its case. A soldered panel's
//! `panel.conf` says the same with `NATIVE` and `MOUNT`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use faraday_core::edid;
use faraday_shell_api::Orientation;

/// How far the core's frame is turned clockwise to sit on the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    /// As drawn.
    R0,
    /// A quarter clockwise: the frame's top on the panel's right edge.
    R90,
    /// A half: upside down.
    R180,
    /// Three quarters clockwise: the frame's top on the panel's left
    /// edge.
    R270,
}

impl Turn {
    /// The turn `degrees` names: 0, 90, 180 or 270.
    pub fn from_degrees(degrees: u16) -> Option<Turn> {
        match degrees {
            0 => Some(Turn::R0),
            90 => Some(Turn::R90),
            180 => Some(Turn::R180),
            270 => Some(Turn::R270),
            _ => None,
        }
    }

    /// The degrees this turn is.
    pub fn degrees(self) -> u16 {
        match self {
            Turn::R0 => 0,
            Turn::R90 => 90,
            Turn::R180 => 180,
            Turn::R270 => 270,
        }
    }

    /// Whether the frame stands across the panel: a quarter turn either
    /// way.
    pub fn quarter(self) -> bool {
        matches!(self, Turn::R90 | Turn::R270)
    }

    /// The size of the core's frame on a panel `w × h` natively: the
    /// panel's at 0 and 180, the other way round at 90 and 270.
    pub fn frame_size(self, (w, h): (u16, u16)) -> (u16, u16) {
        if self.quarter() { (h, w) } else { (w, h) }
    }

    /// The panel pixel that shows the frame's `(u, v)`, on a panel
    /// `w × h` natively (§4.1, left column). The one map the frame, the
    /// arrow and the touch go through.
    pub fn to_panel(self, (w, h): (usize, usize), (u, v): (usize, usize)) -> (usize, usize) {
        match self {
            Turn::R0 => (u, v),
            Turn::R90 => (w - 1 - v, u),
            Turn::R180 => (w - 1 - u, h - 1 - v),
            Turn::R270 => (v, h - 1 - u),
        }
    }

    /// The frame's `(u, v)` a touch at the panel pixel `(x, y)` lands
    /// on (§4.1, right column): [`Turn::to_panel`] undone.
    pub fn to_frame(self, (w, h): (u16, u16), (x, y): (u16, u16)) -> (u16, u16) {
        match self {
            Turn::R0 => (x, y),
            Turn::R90 => (y, w - 1 - x),
            Turn::R180 => (w - 1 - x, h - 1 - y),
            Turn::R270 => (h - 1 - y, x),
        }
    }
}

/// The turn as the input threads read it: set by the main loop when the
/// person chooses a rotation, read by a touchscreen's reader for every
/// touch it maps.
#[derive(Debug, Clone)]
pub struct Shared(Arc<AtomicU16>);

impl Shared {
    /// A turn of `turn`, shared.
    pub fn new(turn: Turn) -> Shared {
        Shared(Arc::new(AtomicU16::new(turn.degrees())))
    }

    /// The turn now.
    pub fn get(&self) -> Turn {
        Turn::from_degrees(self.0.load(Ordering::Relaxed)).unwrap_or(Turn::R0)
    }

    /// Sets the turn.
    pub fn set(&self, turn: Turn) {
        self.0.store(turn.degrees(), Ordering::Relaxed);
    }
}

impl Default for Shared {
    fn default() -> Shared {
        Shared::new(Turn::R0)
    }
}

/// Machines whose panel is mounted on its side in the case, by the
/// firmware's `sys_vendor` and `product_name` exactly, with the quarter
/// turn that shows a frame upright in the case's landscape. The rows are
/// the 6-inch to 10-inch machines of the kernel's
/// `drivers/gpu/drm/drm_panel_orientation_quirks.c` that it matches by
/// these two names alone: its "right side up" is 90 here, its "left side
/// up" 270. A machine not listed takes 90.
pub const MACHINES: [(&str, &str, Turn); 19] = [
    ("Acer", "One S1003", Turn::R90),
    ("Acer", "SW5-017", Turn::R90),
    ("ASUSTeK COMPUTER INC.", "T100HAN", Turn::R270),
    ("ASUSTeK COMPUTER INC.", "T101HA", Turn::R90),
    ("ASUSTeK COMPUTER INC.", "T103HAF", Turn::R90),
    ("AYADEVICE", "AYA NEO 2021", Turn::R90),
    ("ayn", "Loki Max", Turn::R270),
    ("ayn", "Loki Zero", Turn::R270),
    ("Dynabook Inc.", "dynabook K50/FR", Turn::R270),
    ("GPD", "MicroPC", Turn::R90),
    ("GPD", "G1619-01", Turn::R90),
    ("GPD", "G1617-01", Turn::R90),
    ("KD Interactive", "Kurio Smart", Turn::R90),
    ("OrangePi", "NEO-01", Turn::R90),
    (
        "ONE-NETBOOK TECHNOLOGY CO., LTD.",
        "ONE XPLAYER",
        Turn::R270,
    ),
    (
        "SAMSUNG ELECTRONICS CO., LTD.",
        "Galaxy Book 10.6",
        Turn::R90,
    ),
    ("Valve", "Jupiter", Turn::R90),
    ("Valve", "Galileo", Turn::R90),
    ("VIOS", "LTH17", Turn::R90),
];

/// The quarter turn for the orientation that is not the panel's native
/// one on the machine `vendor` `product`: its row on [`MACHINES`], or 90.
pub fn mounting(vendor: &str, product: &str) -> Turn {
    MACHINES
        .iter()
        .find(|(v, p, _)| *v == vendor.trim() && *p == product.trim())
        .map_or(Turn::R90, |(_, _, t)| *t)
}

/// The largest diagonal laid out at its EDID's density, and offered
/// Rotation: 11 inches, in tenths of a millimetre.
pub const SMALL_MAX: u32 = 2794;

/// The smallest diagonal offered Rotation: 6 inches, in tenths of a
/// millimetre.
pub const SMALL_MIN: u32 = 1524;

/// The densities an EDID is believed for: outside them its size is a
/// placeholder, not a measurement.
const DPI_RANGE: std::ops::RangeInclusive<u16> = 96..=480;

/// The density of a small panel from its EDID (§2.1): pixels over inches
/// along the diagonal, on a screen of 11 inches or less whose detailed
/// timing is the framebuffer's mode. `None` for a larger screen, one
/// with no EDID or no size, and one whose firmware chose another mode.
pub fn edid_dpi(size: Option<edid::Size>, fb: (u16, u16)) -> Option<u16> {
    let size = size?;
    if size.pixels != Some(fb) || size.diagonal_tenth_mm()? > SMALL_MAX {
        return None;
    }
    size.dpi().filter(|d| DPI_RANGE.contains(d))
}

/// What is known of the panel at start, and so where Rotation stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setup {
    /// The panel is portrait natively: taller than wide.
    pub native_portrait: bool,
    /// The quarter turn for the orientation that is not the native one.
    pub mount: Turn,
    /// Rotation is offered on this panel (§5).
    pub offered: bool,
}

impl Setup {
    /// A panel the firmware handed over (`fb`, its native mode), with the
    /// EDID's size if it passed one, on the machine `vendor` `product`:
    /// portrait when taller than wide; offered when portrait natively or
    /// when its diagonal is 6 to 11 inches.
    pub fn detected(
        fb: (u16, u16),
        size: Option<edid::Size>,
        vendor: &str,
        product: &str,
    ) -> Setup {
        let native_portrait = fb.1 > fb.0;
        let small = size
            .and_then(|s| s.diagonal_tenth_mm())
            .is_some_and(|d| (SMALL_MIN..=SMALL_MAX).contains(&d));
        Setup {
            native_portrait,
            mount: mounting(vendor, product),
            offered: native_portrait || small,
        }
    }

    /// The panel's own orientation.
    pub fn native(&self) -> Orientation {
        if self.native_portrait {
            Orientation::Portrait
        } else {
            Orientation::Landscape
        }
    }

    /// The turn that shows the frame `chosen` way round (§6): none for
    /// the panel's own orientation, the mounting's quarter for the
    /// other. A panel Rotation is not offered on is never turned.
    pub fn turn_for(&self, chosen: Orientation) -> Turn {
        if !self.offered || chosen == self.native() {
            Turn::R0
        } else {
            self.mount
        }
    }

    /// The turn at start (§8): landscape, the first frame F100 asks
    /// for, wherever Rotation is offered.
    pub fn at_start(&self) -> Turn {
        self.turn_for(Orientation::Landscape)
    }

    /// A panel whose `panel.conf` states it (`--native`, `--mount`):
    /// offered only when it names a mounting.
    pub fn stated(native_portrait: bool, mount: Option<Turn>) -> Setup {
        Setup {
            native_portrait,
            mount: mount.unwrap_or(Turn::R90),
            offered: mount.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(pixels: (u16, u16), mm: (u16, u16)) -> Option<edid::Size> {
        Some(edid::Size {
            pixels: Some(pixels),
            mm: Some(mm),
        })
    }

    #[test]
    fn a_small_panel_is_laid_out_at_its_own_density() {
        // A 7-inch 1024 × 600 panel: about 170 dpi, not the command
        // line's 160.
        assert_eq!(
            edid_dpi(size((1024, 600), (154, 90)), (1024, 600)),
            Some(169)
        );
        // The firmware chose another mode: its pixels are not the
        // timing's, so the density is not known.
        assert_eq!(edid_dpi(size((1024, 600), (154, 90)), (800, 600)), None);
        // A 14-inch laptop keeps the command line's density.
        assert_eq!(edid_dpi(size((1920, 1080), (310, 174)), (1920, 1080)), None);
        // An 11-inch screen is still small; a 12-inch one is not.
        assert!(edid_dpi(size((1366, 768), (243, 137)), (1366, 768)).is_some());
        assert_eq!(edid_dpi(size((1366, 768), (265, 149)), (1366, 768)), None);
        // No EDID, or a size no panel has.
        assert_eq!(edid_dpi(None, (1024, 600)), None);
        assert_eq!(edid_dpi(size((1024, 600), (20, 12)), (1024, 600)), None);
    }

    #[test]
    fn rotation_is_offered_on_small_panels_and_portrait_ones_only() {
        let deck = Setup::detected((800, 1280), None, "Valve", "Jupiter");
        assert!(deck.native_portrait && deck.offered);
        assert_eq!(deck.mount, Turn::R90);
        let seven = Setup::detected((1024, 600), size((1024, 600), (154, 90)), "", "");
        assert!(!seven.native_portrait && seven.offered);
        // A laptop's screen, and a monitor with no EDID: not offered.
        let laptop = Setup::detected((1920, 1080), size((1920, 1080), (310, 174)), "", "");
        assert!(!laptop.offered);
        assert!(!Setup::detected((1920, 1080), None, "", "").offered);
        // A 5-inch landscape panel is below the sizes Rotation is for.
        assert!(!Setup::detected((1280, 720), size((1280, 720), (110, 62)), "", "").offered);
    }

    #[test]
    fn the_map_and_its_inverse_meet_at_every_pixel_and_turn() {
        for (w, h) in [(1024u16, 600u16), (800, 1280), (7, 3)] {
            for turn in [Turn::R0, Turn::R90, Turn::R180, Turn::R270] {
                let (fw, fh) = turn.frame_size((w, h));
                for v in (0..fh).step_by(usize::from(fh / 3).max(1)).chain([fh - 1]) {
                    for u in (0..fw).step_by(usize::from(fw / 3).max(1)).chain([fw - 1]) {
                        let (x, y) = turn.to_panel(
                            (usize::from(w), usize::from(h)),
                            (usize::from(u), usize::from(v)),
                        );
                        assert!(x < usize::from(w) && y < usize::from(h));
                        assert_eq!(turn.to_frame((w, h), (x as u16, y as u16)), (u, v));
                    }
                }
            }
        }
        // A quarter clockwise: the frame's top-left corner is the
        // panel's top-right.
        assert_eq!(Turn::R90.to_panel((800, 1280), (0, 0)), (799, 0));
        assert_eq!(Turn::R270.to_panel((800, 1280), (0, 0)), (0, 1279));
    }

    #[test]
    fn landscape_comes_first_and_portrait_turns_by_the_mounting() {
        let deck = Setup::detected((800, 1280), None, "ASUSTeK COMPUTER INC.", "T100HAN");
        assert_eq!(deck.at_start(), Turn::R270, "landscape, turned");
        assert_eq!(deck.turn_for(Orientation::Portrait), Turn::R0, "its own");
        let seven = Setup::detected((1024, 600), size((1024, 600), (154, 90)), "", "");
        assert_eq!(seven.at_start(), Turn::R0);
        assert_eq!(seven.turn_for(Orientation::Portrait), Turn::R90);
        // The 2.8-inch panel is never turned.
        let pi = Setup::stated(true, None);
        assert_eq!(pi.turn_for(Orientation::Landscape), Turn::R0);
    }

    #[test]
    fn a_machine_on_the_list_turns_its_panel_its_own_way() {
        assert_eq!(mounting("ASUSTeK COMPUTER INC.", "T100HAN\n"), Turn::R270);
        assert_eq!(mounting("GPD", "G1619-01"), Turn::R90);
        assert_eq!(mounting("Someone", "Something"), Turn::R90);
        // A soldered panel says it itself.
        assert!(!Setup::stated(true, None).offered, "the 2.8-inch panel");
        let ten = Setup::stated(true, Some(Turn::R270));
        assert!(ten.offered && ten.mount == Turn::R270);
    }
}
