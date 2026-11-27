//! Easing curves: how far along its way an animation is at a share of
//! its time (`docs/MOTION.md`; Material's curves are named in
//! `faraday_tokens::motion`).

use faraday_tokens::motion::{Easing, EasingPath};

/// One coordinate of the cubic Bézier through `p0`, `a`, `b`, `p3` at
/// parameter `s`, and its slope.
fn cubic(p0: f32, a: f32, b: f32, p3: f32, s: f32) -> (f32, f32) {
    let u = 1.0 - s;
    let at = u * u * u * p0 + 3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s * p3;
    let slope = 3.0 * u * u * (a - p0) + 6.0 * u * s * (b - a) + 3.0 * s * s * (p3 - b);
    (at, slope)
}

/// How far along the cubic Bézier from `from` to `to` with control
/// points `c` (x1, y1, x2, y2) an animation is at `t` (in x): x solved
/// for `t` by Newton's method, then y.
fn segment(from: [f32; 2], c: [f32; 4], to: [f32; 2], t: f32) -> f32 {
    let mut s = ((t - from[0]) / (to[0] - from[0])).clamp(0.0, 1.0);
    for _ in 0..8 {
        let (x, d) = cubic(from[0], c[0], c[2], to[0], s);
        if d > -1e-6 && d < 1e-6 {
            break;
        }
        s = (s - (x - t) / d).clamp(0.0, 1.0);
    }
    cubic(from[1], c[1], c[3], to[1], s).0
}

/// How far along `curve` (a cubic Bézier from (0, 0) to (1, 1), as CSS
/// writes one) an animation `t` of the way through its time is, 0 to 1:
/// the curve's x solved for `t` by Newton's method, then its y.
pub fn ease(curve: Easing, t: f32) -> f32 {
    segment([0.0, 0.0], curve, [1.0, 1.0], t.clamp(0.0, 1.0))
}

/// The same for an easing path: the segment that holds `t` (x rescaled to
/// it), then its y.
pub fn ease_path(path: &EasingPath, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < path.join[0] {
        segment([0.0, 0.0], path.first, path.join, t)
    } else {
        segment(path.join, path.second, [1.0, 1.0], t)
    }
}

