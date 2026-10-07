//! How scrolled content moves (`docs/MOTION.md` §3.3).
//!
//! One [`Motion`] drives whichever region is scrolling: a pan moves it at
//! once and its speed is kept; when the fingers lift it coasts and slows;
//! a wheel notch or an arrow key glides to where it asks; a pan or a coast
//! past an end stretches the content like rubber and springs back.
//! Offsets are kept in whole device pixels, so everything on a scrolled
//! page moves together, and the fractions are carried to the next move.
//!
//! Lengths are design units and times milliseconds, as everywhere in the
//! app; `f` is pixels per design unit.

/// How long a glide takes to cover most of its way: each millisecond it
/// covers 1/τ of what is left, so a notch settles in about 140 ms.
const GLIDE_TAU_MS: f32 = 45.0;
/// How quickly a coast slows: its speed falls by e every τ.
const COAST_TAU_MS: f32 = 325.0;
/// The speed in pixels per millisecond below which a coast stops, and
/// below which a lifted pan does not start one.
const COAST_STOP_PX_MS: f32 = 0.02;
/// The spring that takes a stretch back: critically damped, at this
/// angular frequency per millisecond, so it settles in about 300 ms.
const SPRING_OMEGA: f32 = 0.02;
/// How far a finger moves, in units, before it is scrolling rather than
/// pressing.
pub const SLOP: f32 = 8.0;
/// How stiff the rubber is: the iOS constant.
const RUBBER: f32 = 0.55;
/// The longest step a tick is allowed to take, so a frame the panel
/// missed does not throw the content.
const MAX_STEP_MS: u64 = 50;
/// The step assumed for the first tick of a motion, which has no tick
/// before it to measure from.
const FIRST_STEP_MS: u64 = 16;

/// What the content itself is doing.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Content {
    /// Still, or following a finger.
    #[default]
    Rest,
    /// Gliding the rest of the way a wheel or a key asked for.
    Glide { left: f32 },
    /// Coasting at this speed, units per ms, after a flick.
    Coast { v: f32 },
}

/// The scrolling region's motion.
#[derive(Debug, Clone, Default)]
pub struct Motion {
    content: Content,
    /// Fractions of a pixel moved but not yet shown.
    carry: f32,
    /// A pan is under way: fingers are on the pad or the panel.
    panning: bool,
    /// Units panned since the last tick.
    pan_moved: f32,
    /// The pan's speed, units per ms, averaged over the last few ticks.
    pan_v: f32,
    /// How far a pan has pushed past an end: negative past the top.
    pushed: f32,
    /// How far the content is shown stretched past an end, units:
    /// negative when it is pulled down past its top.
    stretch: f32,
    /// The stretch's speed while it springs back, units per ms.
    stretch_v: f32,
    /// The stretch is springing back.
    spring: bool,
    /// The tick before this one.
    last_ms: Option<u64>,
    /// Reduce motion is on: content follows the fingers and stops,
    /// with no coast and no stretch.
    pub rigid: bool,
}

/// The region a move applies to: its offset, how far it may go, and how
/// tall it is on screen.
pub struct Region<'a> {
    /// The offset, units from the top.
    pub offset: &'a mut f32,
    /// How far it may scroll, when known.
    pub max: Option<f32>,
    /// The height it is seen through, units.
    pub view: f32,
    /// Pixels per unit.
    pub f: f32,
}

impl Region<'_> {
    /// Moves by `dy` plus whatever fractions were carried, to a whole
    /// pixel, clamped at the ends. Returns how far past an end the move
    /// would have gone.
    fn apply(&mut self, dy: f32, carry: &mut f32) -> f32 {
        let f = self.f.max(0.1);
        let want = *self.offset + dy + *carry;
        let hi = self.max.unwrap_or(f32::INFINITY).max(0.0);
        let clamped = want.clamp(0.0, hi);
        let over = want - clamped;
        let snapped = ((clamped * f).round() / f).min(hi);
        *carry = if over == 0.0 { clamped - snapped } else { 0.0 };
        *self.offset = snapped;
        over
    }
}

/// How long two-finger scrolling may pause before it is taken to have
/// ended, for a shell that does not say when the fingers lift.
pub const SCROLL_QUIET_MS: u64 = 300;
/// How long a change of screen or sheet cross-fades.
pub const FADE_MS: u64 = 150;
/// How long a switch's pill takes to slide to the other side.
pub const SWITCH_MS: u64 = 160;
/// How long a step card takes to open or close.
pub const DISCLOSE_MS: u64 = 180;
/// How far a sheet rises as it opens, in units, and over how long.
pub const SHEET_RISE: f32 = 12.0;
pub const SHEET_MS: u64 = 180;
/// How long a toast takes to rise in, and to fade out at its end.
pub const TOAST_MS: u64 = 160;

/// Ease-out cubic: fast at first, settling at the end, `t` from 0 to 1.
pub fn ease_out(t: f32) -> f32 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

/// How far through an animation of `ms` started at `at` the time `now`
/// is, 0 to 1; a start not yet set by a tick is just begun.
pub fn progress(at: Option<u64>, now: u64, ms: u64) -> f32 {
    match at {
        None => 0.1,
        Some(at) => (now.saturating_sub(at) as f32 / ms as f32).min(1.0),
    }
}

/// How long the overlay scrollbar stays after the content stops, and how
/// long it then takes to fade.
const BAR_STAY_MS: u64 = 800;
const BAR_FADE_MS: u64 = 300;

/// The overlay scrollbar's opacity, 0–255, at `now_ms` for content that
/// last moved at `moved_at` (0: not since the region opened).
pub fn bar_alpha(moved_at: u64, now_ms: u64) -> u8 {
    if moved_at == 0 {
        return 0;
    }
    let age = now_ms.saturating_sub(moved_at);
    if age <= BAR_STAY_MS {
        255
    } else if age < BAR_STAY_MS + BAR_FADE_MS {
        (255 * (BAR_STAY_MS + BAR_FADE_MS - age) / BAR_FADE_MS) as u8
    } else {
        0
    }
}

/// The displayed stretch for a push of `x` past an end, seen through a
/// region `d` tall: it grows ever more slowly and never reaches `d`.
fn rubber(x: f32, d: f32) -> f32 {
    let d = d.max(1.0);
    let s = (1.0 - 1.0 / (x.abs() * RUBBER / d + 1.0)) * d;
    if x < 0.0 { -s } else { s }
}

impl Motion {
    /// Whether anything is still moving on its own, so that frames are
    /// wanted on every tick.
    pub fn moving(&self) -> bool {
        self.content != Content::Rest || self.spring
    }

    /// How far the content is shown past an end, units: negative past
    /// the top.
    pub fn stretch(&self) -> f32 {
        self.stretch
    }

    /// Whether the content is coasting fast enough that a touch should
    /// only stop it.
    pub fn flung(&self, f: f32) -> bool {
        matches!(self.content, Content::Coast { v } if (v * f).abs() > 0.2)
    }

    /// Brings the content to where it is going at once: a glide lands,
    /// a coast and a stretch stop.
    pub fn settle(&mut self, region: &mut Region) {
        if let Content::Glide { left } = self.content {
            region.apply(left, &mut self.carry);
        }
        self.stop();
    }

    /// Stops everything at once, the stretch included.
    pub fn stop(&mut self) {
        *self = Motion {
            last_ms: self.last_ms,
            rigid: self.rigid,
            ..Motion::default()
        };
    }

    /// Moves the content by `dy` at once, stopping whatever was moving:
    /// a wheel or a key with reduce motion on.
    pub fn jump(&mut self, region: &mut Region, dy: f32) {
        self.stop();
        region.apply(dy, &mut self.carry);
    }

    /// The fingers moved the content by `dy`: at once, stretching past
    /// an end.
    pub fn pan(&mut self, region: &mut Region, dy: f32) {
        if !self.panning {
            self.panning = true;
            self.pan_v = 0.0;
            self.pan_moved = 0.0;
            // A stretch still springing back is caught where it is.
            self.pushed = if self.stretch == 0.0 {
                0.0
            } else {
                unrubber(self.stretch, region.view)
            };
            self.spring = false;
            self.stretch_v = 0.0;
        }
        self.content = Content::Rest;
        self.pan_moved += dy;
        if self.rigid {
            region.apply(dy, &mut self.carry);
            return;
        }
        let mut dy = dy;
        if self.pushed != 0.0 {
            let back = self.pushed + dy;
            if back.signum() == self.pushed.signum() {
                // Still past the end: the stretch takes all of it.
                self.pushed = back;
                self.stretch = rubber(self.pushed, region.view);
                return;
            }
            self.pushed = 0.0;
            dy = back;
        }
        self.pushed = region.apply(dy, &mut self.carry);
        self.stretch = rubber(self.pushed, region.view);
    }

    /// The fingers lifted: a stretch springs back, a flick coasts on.
    pub fn release(&mut self, f: f32) {
        if !self.panning {
            return;
        }
        self.panning = false;
        self.pushed = 0.0;
        if self.rigid {
            return;
        }
        if self.stretch != 0.0 {
            self.spring = true;
            self.stretch_v = 0.0;
        } else if (self.pan_v * f).abs() >= COAST_STOP_PX_MS {
            self.content = Content::Coast { v: self.pan_v };
        }
        self.pan_v = 0.0;
    }

    /// A wheel notch or an arrow key asked for the content to move by
    /// `dy`; it glides there, and stops hard at an end.
    pub fn glide(&mut self, dy: f32) {
        self.panning = false;
        let left = match self.content {
            Content::Glide { left } if left.signum() == dy.signum() => left,
            _ => {
                // From rest the first step is a frame's, however long
                // the idle tick before it was.
                self.last_ms = None;
                0.0
            }
        };
        self.content = Content::Glide { left: left + dy };
    }

    /// The content glides `dy` from where it is, whatever it was doing:
    /// the page bringing something into view.
    pub fn glide_to(&mut self, dy: f32) {
        self.panning = false;
        self.last_ms = None;
        self.content = Content::Glide { left: dy };
    }

    /// Advances to `now_ms`. Returns whether the content or its stretch
    /// moved.
    pub fn tick(&mut self, region: &mut Region, now_ms: u64) -> bool {
        let fresh = self
            .last_ms
            .is_none_or(|t| now_ms.saturating_sub(t) > 4 * MAX_STEP_MS);
        let dt = if fresh {
            FIRST_STEP_MS
        } else {
            now_ms
                .saturating_sub(self.last_ms.unwrap_or(now_ms))
                .min(MAX_STEP_MS)
        };
        self.last_ms = Some(now_ms);
        let f = region.f.max(0.1);
        if dt == 0 {
            return false;
        }
        let dt = dt as f32;
        if self.panning {
            // The speed is what the last ticks saw, the newest weighed most,
            // so fingers that stop before they lift stop the coast too.
            let seen = self.pan_moved / dt;
            self.pan_v = if fresh {
                seen
            } else {
                self.pan_v * 0.4 + seen * 0.6
            };
            self.pan_moved = 0.0;
        }
        let moved = self.moving();
        match self.content {
            Content::Rest => {}
            Content::Glide { left } => {
                let step = if (left * f).abs() < 0.5 {
                    left
                } else {
                    left * (1.0 - (-dt / GLIDE_TAU_MS).exp())
                };
                let over = region.apply(step, &mut self.carry);
                let left = left - step;
                self.content = if over != 0.0 || left == 0.0 {
                    Content::Rest
                } else {
                    Content::Glide { left }
                };
            }
            Content::Coast { v } => {
                let decay = (-dt / COAST_TAU_MS).exp();
                let over = region.apply(v * COAST_TAU_MS * (1.0 - decay), &mut self.carry);
                let v = v * decay;
                self.content = if over != 0.0 {
                    // It ran into an end: the speed it had carries into a
                    // stretch that springs back.
                    self.spring = true;
                    self.stretch_v = v;
                    Content::Rest
                } else if (v * f).abs() < COAST_STOP_PX_MS {
                    Content::Rest
                } else {
                    Content::Coast { v }
                };
            }
        }
        if self.spring {
            // Critically damped, in one-millisecond steps.
            let (mut x, mut v) = (self.stretch, self.stretch_v);
            for _ in 0..dt as u32 {
                let a = -SPRING_OMEGA * SPRING_OMEGA * x - 2.0 * SPRING_OMEGA * v;
                v += a;
                x += v;
            }
            if (x * f).abs() < 0.5 && (v * f).abs() < 0.05 {
                (x, v) = (0.0, 0.0);
                self.spring = false;
            }
            self.stretch = x;
            self.stretch_v = v;
        }
        moved
    }
}

/// The push that shows as `stretch`: [`rubber`]'s inverse.
fn unrubber(stretch: f32, d: f32) -> f32 {
    let d = d.max(1.0);
    let s = (stretch.abs() / d).min(0.999);
    let x = (1.0 / (1.0 - s) - 1.0) * d / RUBBER;
    if stretch < 0.0 { -x } else { x }
}
