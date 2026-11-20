//! Easing curves: how far along its way an animation is at a share of
//! its time (`docs/MOTION.md`; Material's curves are named in
//! [`crate::tokens`]).

use crate::tokens::{Easing, EasingPath};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{
        EASE_EMPHASIZED, EASE_EMPHASIZED_DECELERATE, EASE_STANDARD, EASE_STANDARD_ACCELERATE,
    };

    /// The curves agree with the cubic Bézier a browser draws for the
    /// same control points: CSS's own definition, x solved for t by
    /// bisection.
    #[test]
    fn materials_curves_move_as_a_browsers_cubic_bezier_does() {
        fn reference(c: Easing, t: f32) -> f32 {
            let b = |a: f64, b: f64, s: f64| {
                let u = 1.0 - s;
                3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
            };
            let (mut lo, mut hi) = (0.0f64, 1.0f64);
            for _ in 0..60 {
                let mid = (lo + hi) / 2.0;
                if b(f64::from(c[0]), f64::from(c[2]), mid) < f64::from(t) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            b(f64::from(c[1]), f64::from(c[3]), (lo + hi) / 2.0) as f32
        }
        for c in [
            EASE_STANDARD,
            EASE_EMPHASIZED_DECELERATE,
            EASE_STANDARD_ACCELERATE,
        ] {
            for i in 0..=20 {
                let t = i as f32 / 20.0;
                let (got, want) = (ease(c, t), reference(c, t));
                assert!(
                    (got - want).abs() < 2e-3,
                    "{c:?} at {t}: {got} against {want}"
                );
            }
        }
    }

    /// Material's emphasized path passes through its published points,
    /// never goes back, and agrees with a bisection of its own two
    /// Béziers.
    #[test]
    fn the_emphasized_path_runs_through_its_points_and_only_forward() {
        let [jx, jy] = EASE_EMPHASIZED.join;
        assert!(ease_path(&EASE_EMPHASIZED, 0.0).abs() < 1e-4);
        assert!((ease_path(&EASE_EMPHASIZED, jx) - jy).abs() < 2e-3);
        assert!((ease_path(&EASE_EMPHASIZED, 1.0) - 1.0).abs() < 1e-4);
        let mut last = 0.0;
        for i in 0..=200 {
            let t = i as f32 / 200.0;
            let e = ease_path(&EASE_EMPHASIZED, t);
            assert!(e >= last - 1e-4, "went back at {t}: {e} after {last}");
            last = e;
        }
        // The second segment alone, by bisection (CSS's definition).
        let c = EASE_EMPHASIZED.second;
        let b = |a: f64, b: f64, p0: f64, p3: f64, s: f64| {
            let u = 1.0 - s;
            u * u * u * p0 + 3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s * p3
        };
        for t in [0.3f32, 0.5, 0.8] {
            let (mut lo, mut hi) = (0.0f64, 1.0f64);
            for _ in 0..60 {
                let mid = (lo + hi) / 2.0;
                if b(f64::from(c[0]), f64::from(c[2]), f64::from(jx), 1.0, mid) < f64::from(t) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let want = b(
                f64::from(c[1]),
                f64::from(c[3]),
                f64::from(jy),
                1.0,
                (lo + hi) / 2.0,
            );
            assert!((f64::from(ease_path(&EASE_EMPHASIZED, t)) - want).abs() < 2e-3);
        }
    }
}
