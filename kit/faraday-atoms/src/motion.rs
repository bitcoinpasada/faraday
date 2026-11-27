//! The motion store: what a component's motion needs between frames (04 §8.1).

use faraday_draw::geom::Rect;
use faraday_port::values::Anchor;

/// Owned by the runtime, lent to each frame's `Ui`: entries keyed by (page anchor, component
/// anchor).
pub struct MotionStore {
    entries: Vec<(ComponentId, Motion)>,
    /// How far each region still has to glide, units (down is positive): a wheel notch or an
    /// arrow key asked for it and the offset follows ([`MotionStore::glide_region`]).
    glides: Vec<(ComponentId, f32)>,
    /// Reduce motion: nothing coasts, stretches or glides; each goes to its end at once.
    rigid: bool,
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
            glides: Vec::new(),
            rigid: false,
        }
    }

    /// Sets Reduce motion (04 §8.1): while it is on a pan follows the fingers and stops at an
    /// end, a lift leaves no coast or spring, and a glide is a jump. The runtime's, from the
    /// person's setting, before it handles an input and each tick.
    pub fn set_rigid(&mut self, rigid: bool) {
        self.rigid = rigid;
    }

    /// Whether Reduce motion is on.
    pub fn rigid(&self) -> bool {
        self.rigid
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
        let offset = match self.get(&id) {
            Some(Motion::Region { offset, .. }) => *offset,
            _ => 0.0,
        };
        let moved = (offset + by).clamp(0.0, max.max(0.0));
        if (moved - offset).abs() < f32::EPSILON {
            return false;
        }
        // A jump takes the content where it asks: a coast, a stretch or a glide stops.
        self.glides.retain(|(k, _)| *k != id);
        self.set(
            id,
            Motion::Region {
                offset: moved,
                velocity: 0.0,
                stretch: 0.0,
                extent: max,
                // 0 means the content has not moved since the region opened.
                bar_shown_ms: now_ms.max(1),
            },
        );
        true
    }

    /// A wheel notch or an arrow key asks the region `id` to move by `by` units (down is
    /// positive) within `0..=max`: it glides there, each frame closing the share of what is left
    /// that `GLIDE_TAU_MS` gives, and a notch before the last has ended adds to it; it ends where
    /// a jump of the same would, and stops hard at an end. Under Reduce motion it is that jump.
    /// A coast stops. Returns whether the offset changed or will.
    pub fn glide_region(&mut self, id: ComponentId, by: f32, max: f32, now_ms: u64) -> bool {
        if self.rigid {
            return self.scroll_region(id, by, max, now_ms);
        }
        let (offset, velocity, stretch, _) = self.region_of(&id);
        let left = self
            .glides
            .iter()
            .find(|(k, _)| *k == id)
            .map_or(0.0, |(_, l)| *l);
        let target = (offset + left + by).clamp(0.0, max.max(0.0));
        let left = target - offset;
        self.glides.retain(|(k, _)| *k != id);
        if left.abs() < f32::EPSILON {
            return false;
        }
        self.glides.push((id, left));
        self.set(
            id,
            Motion::Region {
                offset,
                velocity: if stretch == 0.0 { 0.0 } else { velocity },
                stretch,
                extent: max,
                bar_shown_ms: now_ms.max(1),
            },
        );
        true
    }

    /// The region `id` as kept: offset, velocity, stretch, bar time; a fresh one at rest.
    fn region_of(&self, id: &ComponentId) -> (f32, f32, f32, u64) {
        match self.get(id) {
            Some(Motion::Region {
                offset,
                velocity,
                stretch,
                bar_shown_ms,
                ..
            }) => (*offset, *velocity, *stretch, *bar_shown_ms),
            _ => (0.0, 0.0, 0.0, 0),
        }
    }

    /// The fingers moved the region `id` by `by` units (down is positive), seen through a view
    /// `view` units tall and scrolling `0..=max`: the content follows at once and, pushed past an
    /// end, stretches like rubber. `began` is the first move of a pan, which catches a stretch
    /// still springing back where it is and starts the speed again. The speed is kept for the
    /// fling ([`MotionStore::release_region`]). Returns whether the offset or the stretch moved.
    pub fn pan_region(&mut self, id: ComponentId, pan: Pan, max: f32, now_ms: u64) -> bool {
        let Pan { by, view, began } = pan;
        let (offset, velocity, stretch, last) = self.region_of(&id);
        // A region with nothing to scroll stays still: it is not pulled past an end it has no
        // distance to.
        if max <= 0.0 {
            return false;
        }
        if self.rigid {
            return self.scroll_region(id, by, max, now_ms);
        }
        self.glides.retain(|(k, _)| *k != id);
        let recent = !began && last > 0 && now_ms.saturating_sub(last) <= PAN_RECENT_MS;
        let dt = if recent {
            now_ms.saturating_sub(last).max(1)
        } else {
            FIRST_STEP_MS
        } as f32;
        let seen = by / dt;
        let velocity = if recent {
            velocity * 0.4 + seen * 0.6
        } else {
            seen
        };
        let mut pushed = if stretch == 0.0 {
            0.0
        } else {
            unrubber(stretch, view)
        };
        let mut by = by;
        if pushed != 0.0 {
            let back = pushed + by;
            if back.signum() == pushed.signum() {
                // Still past the end: the stretch takes all of it.
                pushed = back;
                by = 0.0;
            } else {
                pushed = 0.0;
                by = back;
            }
        }
        let mut moved = offset;
        if pushed == 0.0 {
            let want = offset + by;
            moved = want.clamp(0.0, max);
            pushed = want - moved;
        }
        let shown = rubber(pushed, view);
        let changed = (moved - offset).abs() >= f32::EPSILON || (shown - stretch).abs() > 1e-4;
        self.set(
            id,
            Motion::Region {
                offset: moved,
                velocity,
                stretch: shown,
                extent: max,
                bar_shown_ms: if changed { now_ms.max(1) } else { last },
            },
        );
        changed
    }

    /// The fingers lifted from the region `id` at `now_ms`: a stretch springs back from rest, a
    /// flick coasts on at the speed the last moves saw, and fingers that stopped before they
    /// lifted leave none. `f` is pixels per unit. Returns whether it keeps moving.
    pub fn release_region(&mut self, id: ComponentId, f: f32, now_ms: u64) -> bool {
        let Some(Motion::Region {
            offset,
            velocity,
            stretch,
            extent,
            bar_shown_ms,
        }) = self.get(&id).copied()
        else {
            return false;
        };
        let fresh = bar_shown_ms > 0 && now_ms.saturating_sub(bar_shown_ms) <= PAN_RECENT_MS;
        let velocity = if stretch != 0.0 || !fresh || (velocity * f).abs() < COAST_STOP_PX_MS {
            0.0
        } else {
            velocity
        };
        self.set(
            id,
            Motion::Region {
                offset,
                velocity,
                stretch,
                extent,
                bar_shown_ms,
            },
        );
        stretch != 0.0 || velocity != 0.0
    }

    /// A touch on the region `id` stops a coast where it is. Returns whether one was going.
    pub fn stop_coast(&mut self, id: ComponentId) -> bool {
        match self.get(&id).copied() {
            Some(Motion::Region {
                offset,
                velocity,
                stretch,
                extent,
                bar_shown_ms,
            }) if stretch == 0.0 && velocity != 0.0 => {
                self.set(
                    id,
                    Motion::Region {
                        offset,
                        velocity: 0.0,
                        stretch,
                        extent,
                        bar_shown_ms,
                    },
                );
                true
            }
            _ => false,
        }
    }

    /// Advances every region's own motion by `dt_ms` of the frame clock, but for the one the
    /// fingers hold (`panning`): a flick coasts and slows (its speed falls by e every
    /// `COAST_TAU_MS`); one that runs into an end carries its speed into a stretch; a stretch
    /// springs back, critically damped. `f` is pixels per unit. Returns whether anything moved.
    pub fn advance(&mut self, dt_ms: u64, f: f32, panning: Option<ComponentId>) -> bool {
        let dt = dt_ms.min(MAX_STEP_MS) as f32;
        let mut moved = self.advance_glides(dt, f);
        for (id, motion) in &mut self.entries {
            let Motion::Region {
                offset,
                velocity,
                stretch,
                extent,
                bar_shown_ms,
            } = motion
            else {
                continue;
            };
            if self.rigid && (*velocity != 0.0 || *stretch != 0.0) {
                // Reduce motion: what coasted or stretched is at rest at once.
                (*velocity, *stretch) = (0.0, 0.0);
                moved = true;
                continue;
            }
            if Some(*id) == panning || (*velocity == 0.0 && *stretch == 0.0) || dt == 0.0 {
                continue;
            }
            moved = true;
            if *stretch == 0.0 {
                let decay = (-dt / COAST_TAU_MS).exp();
                let want = *offset + *velocity * COAST_TAU_MS * (1.0 - decay);
                let held = want.clamp(0.0, extent.max(0.0));
                *offset = held;
                *velocity *= decay;
                if (want - held).abs() < f32::EPSILON {
                    if (*velocity * f).abs() < COAST_STOP_PX_MS {
                        *velocity = 0.0;
                    }
                    *bar_shown_ms = (*bar_shown_ms).max(1);
                    continue;
                }
                // It ran into an end: the speed it had carries into a stretch that springs back.
            }
            let (mut x, mut v) = (*stretch, *velocity);
            for _ in 0..dt as u32 {
                let a = -SPRING_OMEGA * SPRING_OMEGA * x - 2.0 * SPRING_OMEGA * v;
                v += a;
                x += v;
            }
            if (x * f).abs() < 0.5 && (v * f).abs() < 0.05 {
                (x, v) = (0.0, 0.0);
            }
            *stretch = x;
            *velocity = v;
        }
        moved
    }

    /// Moves each glide on by `dt` ms: it closes `1 - e^(-dt / GLIDE_TAU_MS)` of what is left,
    /// all of it once under half a pixel (or at once under Reduce motion). Returns whether any
    /// moved.
    fn advance_glides(&mut self, dt: f32, f: f32) -> bool {
        if self.glides.is_empty() {
            return false;
        }
        let rigid = self.rigid;
        let entries = &mut self.entries;
        self.glides.retain_mut(|(id, left)| {
            let Some(Motion::Region { offset, extent, .. }) =
                entries.iter_mut().find(|(k, _)| k == id).map(|(_, m)| m)
            else {
                return false;
            };
            if dt == 0.0 && !rigid {
                return true;
            }
            let step = if rigid || (*left * f).abs() < 0.5 {
                *left
            } else {
                *left * (1.0 - (-dt / GLIDE_TAU_MS).exp())
            };
            *offset = (*offset + step).clamp(0.0, extent.max(0.0));
            *left -= step;
            *left != 0.0 && *offset > 0.0 && *offset < *extent
        });
        true
    }

    /// Drops a component's entry.
    pub fn remove(&mut self, id: &ComponentId) {
        self.glides.retain(|(k, _)| k != id);
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

/// One move of a pan: how far the fingers went, how tall the region is seen, and whether it is
/// the pan's first.
#[derive(Clone, Copy, Debug)]
pub struct Pan {
    pub by: f32,
    pub view: f32,
    pub began: bool,
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

/// How quickly a coast slows: its speed falls by e every this many milliseconds.
const COAST_TAU_MS: f32 = 325.0;
/// The speed in pixels per millisecond below which a coast stops, and below which a lifted pan
/// does not start one.
const COAST_STOP_PX_MS: f32 = 0.02;
/// The spring that takes a stretch back: critically damped, at this angular frequency per
/// millisecond, so it settles in about 300 ms.
const SPRING_OMEGA: f32 = 0.02;
/// How quickly a wheel notch's glide closes: the share of what is left falls by e every this
/// many milliseconds.
const GLIDE_TAU_MS: f32 = 45.0;
/// How stiff the rubber is: the iOS constant.
const RUBBER: f32 = 0.55;
/// The longest step an advance takes, so a frame the panel missed does not throw the content.
pub const MAX_STEP_MS: u64 = 50;
/// The step assumed for the first move of a pan or a coast, which has none before it.
pub const FIRST_STEP_MS: u64 = 16;
/// How old the last move of a pan may be and still count as the fingers moving.
const PAN_RECENT_MS: u64 = 100;

/// The displayed stretch for a push of `x` past an end, seen through a region `d` tall: it grows
/// ever more slowly and never reaches `d`.
fn rubber(x: f32, d: f32) -> f32 {
    let d = d.max(1.0);
    let s = (1.0 - 1.0 / (x.abs() * RUBBER / d + 1.0)) * d;
    if x < 0.0 { -s } else { s }
}

/// The push that shows as `stretch`: [`rubber`]'s inverse.
fn unrubber(stretch: f32, d: f32) -> f32 {
    let d = d.max(1.0);
    let s = (stretch.abs() / d).min(0.999);
    let x = (1.0 / (1.0 - s) - 1.0) * d / RUBBER;
    if stretch < 0.0 { -x } else { x }
}

/// How long the overlay scrollbar stays after the content stops, and how long it then takes to
/// fade, milliseconds.
const BAR_STAY_MS: u64 = 800;
const BAR_FADE_MS: u64 = 300;
/// How long the indeterminate progress bar takes to cross its track once, milliseconds.
const BUSY_PASS_MS: u64 = 1800;

/// The eased progress of a change of page `t` (0 to 1) of the way in: Material's emphasized
/// easing. Atoms hold the easing (04 §12.1); the templates ask for it here.
pub fn turn_ease(t: f32) -> f32 {
    faraday_draw::ease::ease_path(&faraday_tokens::motion::EASE_EMPHASIZED, t)
}

fn lerp_rect(a: Rect, b: Rect, e: f32) -> Rect {
    let l = |a: i32, b: i32| (a as f32 + (b - a) as f32 * e).round() as i32;
    Rect {
        x: l(a.x, b.x),
        y: l(a.y, b.y),
        w: l(a.w, b.w),
        h: l(a.h, b.h),
    }
}

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
    /// Where a pane that moves between two places stands: the box `to`, or, after `to` changed,
    /// the way from where it stood to the new place (`Motion::Glide` under `id`, `span_ms` long
    /// from the frame's time, eased as a page turn is). A pane first drawn, or drawn under Reduce
    /// motion, stands at `to`. Asks another frame while it moves.
    pub fn glide_rect(&mut self, id: Anchor, to: Rect, span_ms: u64) -> Rect {
        let key = ComponentId(self.page, id);
        let held = match self.motion.get(&key) {
            Some(Motion::Glide { from, to, began_ms }) => Some((*from, *to, *began_ms)),
            _ => None,
        };
        let here = |from: Rect, goal: Rect, began_ms: u64| {
            let t = progress(began_ms, self.now_ms, span_ms);
            (lerp_rect(from, goal, turn_ease(t)), t)
        };
        let (from, goal, began_ms) = match held {
            Some((from, goal, began_ms)) if goal == to => (from, goal, began_ms),
            Some((from, goal, began_ms)) => {
                let (now_at, _) = here(from, goal, began_ms);
                (now_at, to, self.now_ms)
            }
            None => (to, to, self.now_ms),
        };
        if self.reduce {
            self.motion.set(
                key,
                Motion::Glide {
                    from: to,
                    to,
                    began_ms: self.now_ms,
                },
            );
            return to;
        }
        self.motion.set(
            key,
            Motion::Glide {
                from,
                to: goal,
                began_ms,
            },
        );
        let (at, t) = here(from, goal, began_ms);
        if t < 1.0 && from != goal {
            self.moving = true;
        }
        at
    }

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
        if self.reduce {
            return 1.0;
        }
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
        if self.reduce {
            // No fade: the bar stays, then goes.
            if age < BAR_STAY_MS {
                self.moving = true;
                return 255;
            }
            return 0;
        }
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

    /// Invariant: The eased turn starts at 0, ends at 1 and never goes back; a pane that is moved glides from where it stood to its new place in the span and stands still under Reduce motion.
    /// Regression: A pane that jumps to its new place, or one that starts from its target and so never moves.
    /// Owner: kit
    #[test]
    fn a_moved_pane_glides_and_the_eased_turn_is_monotone() {
        assert!(turn_ease(0.0).abs() < 1e-3 && (turn_ease(1.0) - 1.0).abs() < 1e-3);
        let mut last = 0.0;
        for i in 0..=20 {
            let e = turn_ease(i as f32 / 20.0);
            assert!(e >= last - 1e-4);
            last = e;
        }
        let a = Rect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        let b = Rect {
            x: 200,
            y: 0,
            w: 100,
            h: 100,
        };
        let id = Anchor {
            scope: "pane",
            n: 0,
        };
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let mut seen = Vec::new();
        for (now, to) in [(0, a), (10, b), (135, b), (1000, b)] {
            frame_in(&mut c, &at_rest(None), (&mut store, now), |ui| {
                ui.set_reduce_motion(false);
                seen.push(ui.glide_rect(id, to, 250));
            });
        }
        assert_eq!(seen[0], a);
        assert_eq!(seen[1], a);
        assert!(seen[2].x > 0 && seen[2].x < 200);
        assert_eq!(seen[3], b);
        let mut still = MotionStore::new();
        for (now, to) in [(0, a), (10, b)] {
            frame_in(&mut c, &at_rest(None), (&mut still, now), |ui| {
                ui.set_reduce_motion(true);
                seen.push(ui.glide_rect(id, to, 250));
            });
        }
        assert_eq!(seen[5], b);
    }
}
