//! How far a checksum worksheet is panned inside its view
//! (`docs/CODEX32.md` §7.5), and how the view follows the entry's front:
//! a timed move on Material's standard curve, as a change of theme fades
//! (`docs/MOTION.md` §3.5), not the wheel's glide.
//!
//! A drag, the wheel and the arrow keys set the pan at once and stop any
//! move ([`SheetPan::stop`]). When the front moves, the place the view
//! goes to is worked out once ([`SheetPan::target_for`]) and handed to
//! [`SheetPan::go_to`]; each frame after that only interpolates
//! ([`SheetPan::tick`]), from the time the frame is drawn at. A new
//! place while a move is under way starts from where the view is, at the
//! speed it has, so the move bends rather than jumps. Under Reduce motion
//! the view is placed at once.

use crate::motion::{FIRST_STEP_MS, SHEET_FOLLOW_MS, ease_move};

/// A move of the view under way: part of Material's standard curve from
/// `from` to `to`, entered at `t0` of the way along it so that it leaves
/// at the speed the view had, plus whatever of that speed was across the
/// way, dying out.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Move {
    /// Where the curve starts, across and down (behind the view, when
    /// the move took over from another).
    from: (f32, f32),
    /// Where it ends.
    to: (f32, f32),
    /// How far along the curve the move entered it, 0 to 1.
    t0: f32,
    /// The speed it took over across its way, units a millisecond.
    carry: (f32, f32),
    /// Its speed at the last frame.
    v: (f32, f32),
    /// When it started: set by the first tick after it was asked for,
    /// a frame back, so a slow first frame does not use it up.
    at: Option<u64>,
}

/// Material's standard curve's slope at `t`, measured. Arithmetic.
fn slope(t: f32) -> f32 {
    let h = 1e-3;
    (ease_move((t + h).min(1.0)) - ease_move((t - h).max(0.0)))
        / ((t + h).min(1.0) - (t - h).max(0.0))
}

impl Move {
    /// A move from `pos`, at speed `v` (units a millisecond), to `to`:
    /// the point of the curve whose speed, for the way left, is the
    /// view's speed along it (found by halving), so the view goes on as
    /// fast as it went. Arithmetic.
    fn from_moving(pos: (f32, f32), v: (f32, f32), to: (f32, f32)) -> Move {
        let d = (to.0 - pos.0, to.1 - pos.1);
        let dd = d.0 * d.0 + d.1 * d.1;
        let along = if dd > 0.0 {
            (v.0 * d.0 + v.1 * d.1) / dd
        } else {
            0.0
        };
        // Along the way, the curve entered at t0 leaves at
        // slope(t0) / (1 − e(t0)) of the way left a unit of its time.
        let want = (along * SHEET_FOLLOW_MS as f32).max(0.0);
        let g = |t: f32| slope(t) / (1.0 - ease_move(t)).max(1e-4);
        let (mut lo, mut hi) = (0.0f32, faraday_draw::tokens::SHEET_FOLLOW_ENTRY_MAX);
        let t0 = if want <= 0.0 {
            0.0
        } else if g(hi) <= want {
            hi
        } else {
            for _ in 0..24 {
                let mid = (lo + hi) / 2.0;
                if g(mid) < want {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            lo
        };
        let e = ease_move(t0);
        let from = (
            (pos.0 - to.0 * e) / (1.0 - e),
            (pos.1 - to.1 * e) / (1.0 - e),
        );
        // What of the speed the curve does not carry: across the way.
        let rest = SHEET_FOLLOW_MS as f32 * (1.0 - t0);
        let lead = slope(t0) / rest.max(1.0);
        let carry = (
            v.0 - (to.0 - from.0) * lead * (1.0 - t0),
            v.1 - (to.1 - from.1) * lead * (1.0 - t0),
        );
        Move {
            from,
            to,
            t0,
            carry,
            v,
            at: None,
        }
    }

    /// How long the move lasts, ms: what is left of the curve.
    fn ms(&self) -> f32 {
        (SHEET_FOLLOW_MS as f32 * (1.0 - self.t0)).max(1.0)
    }

    /// Where the move stands `u` of its time through, 0 to 1. Arithmetic.
    fn at_u(&self, u: f32) -> (f32, f32) {
        let e = ease_move(self.t0 + u * (1.0 - self.t0));
        let c = self.ms() * u * (1.0 - u) * (1.0 - u);
        (
            self.from.0 + (self.to.0 - self.from.0) * e + self.carry.0 * c,
            self.from.1 + (self.to.1 - self.from.1) * e + self.carry.1 * c,
        )
    }
}

/// Whether the checksum worksheet is drawn whole, and at which cell
/// (`docs/LAYOUT.md` §16.1): the largest cell from
/// `CODEX32_CELL_FIT_MIN` to `CODEX32_CELL_FIT_MAX` at which the whole
/// sheet fits the room it is drawn in, kept with the room and the sheet's
/// length it was worked out for. Kept in step by
/// [`super::Codex32Make::refit`], before every frame and before the view
/// follows the work, so drawing and the follow read the same cell.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct SheetFit {
    /// The room it was worked out for: the work width and the view's
    /// height, dp; none on one-column pages, where no sheet fits.
    pub(crate) room: Option<(f32, f32)>,
    /// The sheet's characters it was worked out for (0: no sheet yet).
    pub(crate) chars: usize,
    /// The fitted cell; none where the sheet pans in its view.
    pub(crate) cell: Option<f32>,
    /// The sheet's size at its least fitting cell, across and down, and
    /// the length it was worked out for: what the dice sheet's column
    /// beside it must leave (`docs/LAYOUT.md` §16.1).
    pub(crate) least: Option<(f32, f32)>,
    /// The characters `least` was worked out for.
    pub(crate) least_for: usize,
}

/// How far a checksum worksheet is panned inside its view: units from
/// the sheet's left edge and from its top, and the follow's move under
/// way. Drawing clamps it to the sheet it draws.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SheetPan {
    /// Units from the sheet's left edge.
    pub x: f32,
    /// Units from the sheet's top.
    pub y: f32,
    /// The follow's move, while one is under way.
    moving: Option<Move>,
    /// When the move under way last placed the view, ms: a move that
    /// takes over starts from there.
    last_ms: u64,
}

impl SheetPan {
    /// Whether the view is moving to follow the front.
    pub fn moving(&self) -> bool {
        self.moving.is_some()
    }

    /// Where the view is going: the move's end, or where it is.
    pub fn going_to(&self) -> (f32, f32) {
        self.moving.map_or((self.x, self.y), |m| m.to)
    }

    /// Where the view goes to bring the rectangle `spot` (left, top, wide,
    /// tall, in the sheet's units) into a view `view` (wide, tall) with
    /// `margin` round it where there is room: the least move from where
    /// it is going, never past `max` (across, down). Arithmetic.
    pub fn target_for(
        &self,
        spot: (f32, f32, f32, f32),
        view: (f32, f32),
        max: (f32, f32),
        margin: f32,
    ) -> (f32, f32) {
        fn axis(at: f32, lo: f32, side: f32, view: f32, max: f32, margin: f32) -> f32 {
            let m = margin.min(((view - side) / 2.0).max(0.0));
            let at = if lo - m < at {
                lo - m
            } else if lo + side + m > at + view {
                // Wider than the view: its start stays in it.
                (lo + side + m - view).min(lo - m)
            } else {
                at
            };
            at.clamp(0.0, max.max(0.0))
        }
        let (x, y, w, h) = spot;
        let (gx, gy) = self.going_to();
        (
            axis(gx, x, w, view.0, max.0, margin),
            axis(gy, y, h, view.1, max.1, margin),
        )
    }

    /// The view goes to `to`: at once under `reduce`, else over
    /// [`SHEET_FOLLOW_MS`] from where it is, at the speed it has. The
    /// same place again changes nothing.
    pub fn go_to(&mut self, to: (f32, f32), reduce: bool) {
        if reduce {
            (self.x, self.y) = to;
            self.moving = None;
            return;
        }
        if self.going_to() == to {
            return;
        }
        let old = self.moving;
        let v = old.map_or((0.0, 0.0), |m| m.v);
        let mut m = Move::from_moving((self.x, self.y), v, to);
        // Taking over from a move, it starts where that one last placed
        // the view, at that time.
        if old.is_some_and(|o| o.at.is_some()) {
            m.at = Some(self.last_ms);
        }
        self.moving = Some(m);
    }

    /// One frame at `now_ms`, `f` pixels to the unit: the view where the
    /// move stands, to a whole pixel. Returns whether it moved (a frame is
    /// wanted). Arithmetic.
    pub fn tick(&mut self, now_ms: u64, f: f32) -> bool {
        let Some(mut m) = self.moving else {
            return false;
        };
        let at = *m.at.get_or_insert(now_ms.saturating_sub(FIRST_STEP_MS));
        self.last_ms = now_ms;
        let ms = m.ms();
        let u = (now_ms.saturating_sub(at) as f32 / ms).min(1.0);
        if u >= 1.0 {
            (self.x, self.y) = m.to;
            self.moving = None;
            return true;
        }
        let (x, y) = m.at_u(u);
        // The speed now, for a move that takes over from this one.
        let du = 1.0 / ms;
        let (bx, by) = m.at_u((u - du).max(0.0));
        let dt = ((u - (u - du).max(0.0)) * ms).max(1e-3);
        m.v = ((x - bx) / dt, (y - by) / dt);
        let f = f.max(f32::MIN_POSITIVE);
        (self.x, self.y) = ((x * f).round() / f, (y * f).round() / f);
        self.moving = Some(m);
        true
    }

    /// The move ends where it was going: for a picture of the screen at
    /// rest.
    pub fn settle(&mut self) {
        if let Some(m) = self.moving.take() {
            (self.x, self.y) = m.to;
        }
    }

    /// The person moved the view: the follow's move stops where it is.
    pub fn stop(&mut self) {
        self.moving = None;
    }
}

#[cfg(test)]
mod tests {
    use super::SheetPan;
    use crate::motion::SHEET_FOLLOW_MS;

    #[test]
    fn a_new_place_mid_move_keeps_the_speed() {
        // Measured a millisecond at a time, on a fine grid: the view's
        // speed just after a new place is asked for is its speed just
        // before.
        let f = 1000.0;
        let mut p = SheetPan::default();
        p.go_to((0.0, 300.0), false);
        let mut now = 1000;
        p.tick(now, f);
        for _ in 0..100 {
            now += 1;
            p.tick(now, f);
        }
        let before = p.y;
        now += 1;
        p.tick(now, f);
        let speed = p.y - before;
        assert!(speed > 0.0);
        p.go_to((0.0, 600.0), false);
        let at = p.y;
        now += 1;
        p.tick(now, f);
        let step = p.y - at;
        assert!((step - speed).abs() <= speed * 0.15, "{speed} then {step}");
        now += SHEET_FOLLOW_MS;
        p.tick(now, f);
        assert_eq!((p.x, p.y), (0.0, 600.0));
        assert!(!p.moving());
    }
}
