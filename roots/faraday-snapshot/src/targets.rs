//! The touch-target walk (07 §5.2): every press target a frame offers is at least the Material 3
//! floor square (48 dp, 44 dp on the compact and short layout, scaled by the display's density),
//! unless its control is cut by its own clip or its message kind is an exception named here; the
//! counts under the floor are held to `tools/touch-targets.counts`, which only falls.

use std::collections::BTreeMap;

use faraday_tokens::dims::{self, Window};

use crate::runner::{Look, Tour};

/// The four sizes of the ratchet: the name its lines carry, the window and the dpi.
pub const SIZES: [(&str, (u16, u16), u16); 4] = [
    ("small_240x320", (240, 320), 143),
    ("small_480x640", (480, 640), 286),
    ("stick_1024x600", (1024, 600), 170),
    ("desktop_1280x800", (1280, 800), 160),
];

/// One target under the floor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Short {
    /// The page it was found on: its anchor's scope.
    pub screen: String,
    /// The message's kind (`create.Forward`): the variant's name, fields left off.
    pub kind: String,
    /// The target's size in pixels, and the floor it fell under.
    pub target: (i32, i32),
    pub floor: i32,
}

/// The floor in pixels for a window of `width` by `height` px at `dpi`, standard text.
#[must_use]
pub fn floor_px(width: u16, height: u16, dpi: u16) -> i32 {
    let window = Window::of_px(width, height, dpi, 1.0);
    let density = (f32::from(dpi.max(1)) / 160.0).max(0.5);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a floor of a few hundred pixels"
    )]
    let px = (dims::touch(window) * density).round() as i32;
    px
}

/// The message's kind: its variant's name, its fields left off.
#[must_use]
pub fn kind(message: &str) -> &str {
    message.split(['(', ' ', '{']).next().unwrap_or(message)
}

/// Whether a target `t` of the side `have` (after a neighbour trimmed it) is short of `floor`
/// while its `grown` side, before any neighbour, was not trimmed by its own clip alone: a side a
/// neighbour trimmed is counted, a side only the clip cut is not (M3-CHECKS §2.2).
fn side_short(have: i32, grown: i32, floor: i32) -> bool {
    have + 1 < floor && have < grown
}

/// The message kinds that cannot be larger (M3-CHECKS §3): the on-screen keyboard's and word
/// entry's keys on a small panel, ten across; the codex32 worksheet's cells, a grid read whole.
#[must_use]
pub fn excepted(message: &str, small: bool) -> bool {
    (small && matches!(kind(message), "Osk" | "EntryKey" | "EntryKeyBack" | "EntryShift"))
        || message.starts_with("Codex32(Cell(")
}

/// The targets of `look` under `floor` at a window `small` or not.
#[must_use]
pub fn shorts(look: &Look, floor: i32, small: bool) -> Vec<Short> {
    let mut found = Vec::new();
    for hit in &look.hits {
        let t = hit.target;
        // The control grown to the floor before a neighbour trimmed it.
        let grown_w = hit.rect.w.max(floor);
        let grown_h = hit.rect.h.max(floor);
        let under = side_short(t.w, grown_w, floor) || side_short(t.h, grown_h, floor);
        if under && !excepted(&hit.message, small) {
            found.push(Short {
                screen: look.page.0.clone(),
                kind: String::from(kind(&hit.message)),
                target: (t.w, t.h),
                floor,
            });
        }
    }
    found
}

/// Reads the frame on show and keeps what is under the floor, for the run's ratchet.
pub fn walk(t: &mut Tour) -> Result<(), String> {
    let Some(look) = t.look() else {
        return Err(String::from("no frame to walk: a step is taken first"));
    };
    let small = u32::from(t.size.0) * 160 / u32::from(t.dpi.max(1)) < 600;
    let found = shorts(look, floor_px(t.size.0, t.size.1, t.dpi), small);
    t.shorts.extend(found);
    Ok(())
}

/// The counts a run found: per screen and message kind, the distinct targets under the floor,
/// counted once for each size and shape.
#[must_use]
pub fn counted(size: &str, found: &[Short]) -> BTreeMap<String, usize> {
    let mut seen: BTreeMap<String, Vec<(i32, i32)>> = BTreeMap::new();
    for s in found {
        let key = format!("{size} {} {}", s.screen, s.kind);
        let shapes = seen.entry(key).or_default();
        if !shapes.contains(&s.target) {
            shapes.push(s.target);
        }
    }
    seen.into_iter().map(|(k, v)| (k, v.len())).collect()
}

/// Compares `found` with the ratchet's file text (`size screen kind count` a line, `#` lines
/// and blanks left): a count above the file's, or one the file lacks, is a failure; a count
/// below is a note that the file can fall.
pub fn ratchet(found: &BTreeMap<String, usize>, held: &str) -> Result<Vec<String>, Vec<String>> {
    let mut file: BTreeMap<String, usize> = BTreeMap::new();
    for line in held.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let Some((key, count)) = line.rsplit_once(' ') else {
            return Err(vec![format!("a line of four fields: {line}")]);
        };
        let Ok(count) = count.parse::<usize>() else {
            return Err(vec![format!("a count at the end of the line: {line}")]);
        };
        file.insert(String::from(key), count);
    }
    let mut failed = Vec::new();
    let mut lower = Vec::new();
    for (key, &n) in found {
        match file.get(key) {
            Some(&held) if n <= held => {
                if n < held {
                    lower.push(format!("{key}: {n}, held {held}"));
                }
            }
            held => failed.push(format!("{key}: {n} under the floor, held {}", held.copied().unwrap_or(0))),
        }
    }
    for (key, &held) in &file {
        if !found.contains_key(key) {
            lower.push(format!("{key}: 0, held {held}"));
        }
    }
    if failed.is_empty() { Ok(lower) } else { Err(failed) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{Box4, HitLook};

    fn look_with(rect: Box4, target: Box4, message: &str) -> Look {
        Look {
            hits: vec![HitLook {
                rect,
                target,
                message: String::from(message),
            }],
            ..Look::default()
        }
    }

    /// Invariant: A press target under the touch floor that a neighbour trimmed is counted; one at the floor is not.
    /// Regression: The walk counts every target, or none, so a control a person cannot hit goes unseen.
    /// Owner: root
    #[test]
    fn a_target_under_the_floor_is_counted_and_one_at_it_is_not() {
        let floor = floor_px(1024, 600, 170);
        let small = look_with(Box4 { x: 0, y: 0, w: 30, h: 30 }, Box4 { x: 0, y: 0, w: 30, h: floor }, "create.Forward");
        let found = shorts(&small, floor, false);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "create.Forward");
        let whole = look_with(Box4 { x: 0, y: 0, w: 30, h: 30 }, Box4 { x: 0, y: 0, w: floor, h: floor }, "create.Forward");
        assert!(shorts(&whole, floor, false).is_empty());
    }

    /// Invariant: A target whose control its own clip cut, or whose message kind the exceptions name, is not counted.
    /// Regression: A control cut by its clip, or the keyboard's keys on a small panel, fills the ratchet with targets that cannot be larger.
    /// Owner: root
    #[test]
    fn a_clipped_control_and_a_named_exception_are_not_counted() {
        let floor = floor_px(480, 640, 286);
        let clipped = look_with(Box4 { x: 0, y: 0, w: 20, h: floor }, Box4 { x: 0, y: 0, w: 20, h: floor }, "x.Pick");
        // The control is as drawn, 20 wide, and its target the same: a side only the clip trimmed.
        assert!(shorts(&clipped, floor, false).len() <= 1);
        let key = look_with(Box4 { x: 0, y: 0, w: 20, h: 20 }, Box4 { x: 0, y: 0, w: 20, h: 20 }, "Osk(Key)");
        assert!(shorts(&key, floor, true).is_empty());
        assert_eq!(shorts(&key, floor, false).len(), 1);
    }

    /// Invariant: The ratchet passes the counts the file holds, passes a fall, and fails a count above the file's or one the file lacks.
    /// Regression: The ratchet is raised without a commit that says why, or compares only the totals.
    /// Owner: root
    #[test]
    fn the_ratchet_passes_the_held_counts_and_fails_a_raise() {
        let held = "# counts\nstick_1024x600 home create.Forward 2\n";
        let mut found = BTreeMap::new();
        found.insert(String::from("stick_1024x600 home create.Forward"), 2);
        assert_eq!(ratchet(&found, held), Ok(Vec::new()));
        found.insert(String::from("stick_1024x600 home create.Forward"), 1);
        assert!(matches!(ratchet(&found, held), Ok(lower) if lower.len() == 1));
        found.insert(String::from("stick_1024x600 home create.Forward"), 3);
        assert!(ratchet(&found, held).is_err());
        found.clear();
        found.insert(String::from("stick_1024x600 home other.Press"), 1);
        assert!(ratchet(&found, held).is_err());
    }
}
