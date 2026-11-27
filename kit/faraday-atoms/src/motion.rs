//! The motion store: what a component's motion needs between frames (04 §8.1).

use faraday_draw::geom::Rect;
use faraday_port::values::Anchor;

/// Owned by the runtime, lent to each frame's `Ui`: entries keyed by (page anchor, component
/// anchor).
pub struct MotionStore {
    entries: Vec<(ComponentId, Motion)>,
}

impl MotionStore {
    /// The runtime's only: the one store, empty.
    #[expect(
        clippy::new_without_default,
        reason = "only the runtime makes the motion store (01 R1)"
    )]
    pub fn new() -> MotionStore {
        MotionStore {
            entries: Vec::new(),
        }
    }

    /// The entry of a component, if a frame kept one.
    pub fn get(&self, id: &ComponentId) -> Option<&Motion> {
        self.entries.iter().find(|(k, _)| k == id).map(|(_, m)| m)
    }

    /// Keeps `motion` for a component, replacing its entry.
    pub fn set(&mut self, id: ComponentId, motion: Motion) {
        match self.entries.iter_mut().find(|(k, _)| *k == id) {
            Some(e) => e.1 = motion,
            None => self.entries.push((id, motion)),
        }
    }

    /// Scrolls the region `id` by `by` units (down is positive) within `0..=max` units and, when
    /// that moves it, keeps the time `now_ms` as the moment its content last moved, which is
    /// what shows the overlay scrollbar and fades it (`Motion::Region`). The runtime's, at a
    /// wheel or a finger. Returns whether the offset changed.
    pub fn scroll_region(&mut self, id: ComponentId, by: f32, max: f32, now_ms: u64) -> bool {
        let (offset, velocity, stretch) = match self.get(&id) {
            Some(Motion::Region {
                offset,
                velocity,
                stretch,
                ..
            }) => (*offset, *velocity, *stretch),
            _ => (0.0, 0.0, 0.0),
        };
        let moved = (offset + by).clamp(0.0, max.max(0.0));
        if (moved - offset).abs() < f32::EPSILON {
            return false;
        }
        self.set(
            id,
            Motion::Region {
                offset: moved,
                velocity,
                stretch,
                extent: max,
                // 0 means the content has not moved since the region opened.
                bar_shown_ms: now_ms.max(1),
            },
        );
        true
    }

    /// Drops a component's entry.
    pub fn remove(&mut self, id: &ComponentId) {
        self.entries.retain(|(k, _)| k != id);
    }

    /// How many entries are kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no entry is kept.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Numbers, rectangles and times only (K9).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Motion {
    Region {
        offset: f32,
        velocity: f32,
        stretch: f32,
        extent: f32,
        bar_shown_ms: u64,
    },
    /// The session pane, the guide's pane.
    Glide { from: Rect, to: Rect, began_ms: u64 },
    /// A sheet, a menu, the dice sheet, the snackbar.
    Shown { at: f32, began_ms: u64 },
    /// Hold-to-confirm's fill.
    Hold { began_ms: u64 },
    /// A segmented control's fill.
    Slide { from: u16, to: u16, began_ms: u64 },
    /// A page turn, a step's fade through.
    Fade { kind: Turn, began_ms: u64 },
    /// A last-frame measure of 04 §7.2's table.
    Laid { measure: f32 },
}

/// What fades: a page turn, or a step's fade through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Turn {
    Page,
    Step,
}

/// A component's key in the store: the page's anchor, the component's anchor.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ComponentId(pub Anchor, pub Anchor);

/// How long the overlay scrollbar stays after the content stops, and how long it then takes to
/// fade, milliseconds.
const BAR_STAY_MS: u64 = 800;
const BAR_FADE_MS: u64 = 300;
/// How long the indeterminate progress bar takes to cross its track once, milliseconds.
const BUSY_PASS_MS: u64 = 1800;

/// How far through an animation of `span_ms` begun at `began_ms` the time `now_ms` is, 0 to 1.
pub fn progress(began_ms: u64, now_ms: u64, span_ms: u64) -> f32 {
    if span_ms == 0 {
        return 1.0;
    }
    (now_ms.saturating_sub(began_ms) as f32 / span_ms as f32).min(1.0)
}

/// The overlay scrollbar's opacity, 0 to 255, at `now_ms` for a region whose content last moved at
/// `moved_ms` (0: not since the region opened).
pub fn bar_alpha(moved_ms: u64, now_ms: u64) -> u8 {
    if moved_ms == 0 {
        return 0;
    }
    let age = now_ms.saturating_sub(moved_ms);
    if age <= BAR_STAY_MS {
        255
    } else if age < BAR_STAY_MS + BAR_FADE_MS {
        (255 * (BAR_STAY_MS + BAR_FADE_MS - age) / BAR_FADE_MS) as u8
    } else {
        0
    }
}

/// How far across its track the indeterminate bar of a job begun at `began_ms` is at `now_ms`,
/// 0 to 1; it starts again at the left.
pub fn busy_phase(began_ms: u64, now_ms: u64) -> f32 {
    (now_ms.saturating_sub(began_ms) % BUSY_PASS_MS) as f32 / BUSY_PASS_MS as f32
}

impl<A: crate::Pressed> crate::Ui<'_, A> {
    /// How much of a hold is filled, 0 to 1 (04 §8.1, `Motion::Hold`): the entry the component
    /// `id` of the page drawn keeps (K10) begins when `pressed` is first seen and goes when it
    /// is not, so letting go empties it. Asks another frame while it fills.
    pub fn hold_fraction(&mut self, id: Anchor, pressed: bool) -> f32 {
        let key = ComponentId(self.page, id);
        if !pressed {
            self.motion.remove(&key);
            return 0.0;
        }
        let began_ms = match self.motion.get(&key) {
            Some(Motion::Hold { began_ms }) => *began_ms,
            _ => {
                self.motion.set(
                    key,
                    Motion::Hold {
                        began_ms: self.now_ms,
                    },
                );
                self.now_ms
            }
        };
        let done = progress(began_ms, self.now_ms, faraday_tokens::dims::HOLD_MS);
        if done < 1.0 {
            self.moving = true;
        }
        done
    }

    /// How far a sheet or a menu that is open has come in, 0 to 1 (`Motion::Shown`): its entry
    /// begins at the frame it is first drawn, `span_ms` long on `curve`, and it stays at 1 once
    /// there. Closed, nothing draws it and the store drops the entry (K11). Asks another frame
    /// while it moves.
    pub fn shown_fraction(
        &mut self,
        id: Anchor,
        span_ms: u64,
        curve: faraday_tokens::motion::Easing,
    ) -> f32 {
        let key = ComponentId(self.page, id);
        let (at, began_ms) = match self.motion.get(&key) {
            Some(Motion::Shown { at, began_ms }) => (*at, *began_ms),
            _ => {
                self.motion.set(
                    key,
                    Motion::Shown {
                        at: 0.0,
                        began_ms: self.now_ms,
                    },
                );
                (0.0, self.now_ms)
            }
        };
        let t = progress(began_ms, self.now_ms, span_ms);
        if t < 1.0 {
            self.moving = true;
        }
        at + (1.0 - at) * faraday_draw::ease::ease(curve, t)
    }

    /// The overlay scrollbar's opacity for the region `id` of the page drawn, 0 to 255: it
    /// stays after the content moves and then fades (`Motion::Region`'s `bar_shown_ms`). A
    /// region the store holds no motion for, or none since it opened, has none. Asks another
    /// frame while it shows.
    pub(crate) fn bar_alpha_of(&mut self, id: Anchor) -> u8 {
        let moved = match self.motion.get(&ComponentId(self.page, id)) {
            Some(Motion::Region { bar_shown_ms, .. }) if *bar_shown_ms > 0 => *bar_shown_ms,
            _ => return 0,
        };
        let age = self.now_ms.saturating_sub(moved);
        if age < BAR_STAY_MS + BAR_FADE_MS {
            self.moving = true;
        }
        bar_alpha(moved, self.now_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Offered;
    use crate::hold::{HoldModel, HoldTone};
    use crate::shown::Shown;
    use crate::ui::testing::*;
    use faraday_port::values::Pace;

    const ID: Anchor = Anchor {
        scope: "hold",
        n: 1,
    };

    /// Draws a hold at `now_ms` with the press `down` on it or not.
    fn hold_frame(store: &mut MotionStore, down: bool, now_ms: u64) -> Pace {
        let mut c = canvas(160);
        let mut pointer = at_rest(None);
        pointer.pressed = down.then_some(P(1));
        let model = HoldModel {
            label: Shown::plain("Erase"),
            tone: HoldTone::Error,
            id: ID,
            offered: Offered::Yes(P(1)),
        };
        frame_in(&mut c, &pointer, (store, now_ms), |ui| {
            crate::hold::draw(&model, ui, (10.0, 10.0, 200.0));
        })
        .pace
    }

    /// Invariant: A hold fills while its press is held, asks frames until it is full, and empties the moment it is let go.
    /// Regression: A hold is drawn empty however long it is held, keeps asking frames when full, or keeps its fill after release.
    /// Owner: kit
    #[test]
    fn a_hold_fills_while_held_and_empties_when_let_go() {
        let mut store = MotionStore::new();
        assert_eq!(hold_frame(&mut store, true, 1_000), Pace::Moving);
        assert!(matches!(
            store.get(&ComponentId(Anchor { scope: "", n: 0 }, ID)),
            Some(Motion::Hold { began_ms: 1_000 })
        ));
        assert_eq!(hold_frame(&mut store, true, 1_500), Pace::Moving);
        assert_eq!(hold_frame(&mut store, true, 1_000_000), Pace::Rest);
        assert_eq!(hold_frame(&mut store, false, 1_000_001), Pace::Rest);
        assert!(store.is_empty());
    }

    /// Invariant: The overlay scrollbar stays at full strength after the content moves, fades out in the time after, and is gone after that; none was shown if the content never moved.
    /// Regression: The bar vanishes at once, never fades, or shows for a region that never moved.
    /// Owner: kit
    #[test]
    fn the_overlay_scrollbar_stays_then_fades_then_goes() {
        assert_eq!(bar_alpha(0, 5_000), 0);
        assert_eq!(bar_alpha(1_000, 1_500), 255);
        let mid = bar_alpha(1_000, 1_000 + 800 + 150);
        assert!(mid > 0 && mid < 255);
        assert_eq!(bar_alpha(1_000, 1_000 + 800 + 300), 0);
    }

    /// Invariant: A busy bar crosses its track and starts again at the left; an animation's progress runs 0 to 1 and stops there.
    /// Regression: The busy bar sticks at its end, or progress runs past 1.
    /// Owner: kit
    #[test]
    fn a_busy_bar_starts_again_and_progress_stops_at_one() {
        assert_eq!(busy_phase(100, 100), 0.0);
        assert!(busy_phase(100, 1_000) > 0.4);
        assert!(busy_phase(100, 100 + 1_800 + 10) < 0.01);
        assert_eq!(progress(10, 10, 200), 0.0);
        assert_eq!(progress(10, 5_000, 200), 1.0);
        assert_eq!(progress(10, 5, 200), 0.0);
    }

    /// Invariant: A sheet or a menu newly open is drawn at none of its way in and asks frames until it is in; once in, it stays in and asks none.
    /// Regression: A sheet is drawn at rest, fully in, in its first frame, or asks frames for ever.
    /// Owner: kit
    #[test]
    fn a_sheet_comes_in_over_its_time_and_stays_in() {
        let mut store = MotionStore::new();
        let mut seen = Vec::new();
        for now in [2_000, 2_100, 2_200, 9_000] {
            let mut c = canvas(160);
            let pace = frame_in(&mut c, &at_rest(None), (&mut store, now), |ui| {
                seen.push(ui.shown_fraction(ID, 400, [0.0, 0.0, 1.0, 1.0]));
            })
            .pace;
            assert_eq!(pace == Pace::Moving, now < 2_400, "at {now}");
        }
        assert_eq!(seen[0], 0.0);
        assert!(seen[1] > 0.0 && seen[1] < seen[2] && seen[2] < 1.0);
        assert_eq!(seen[3], 1.0);
    }
}
