//! Easing curves: how far along its way an animation is at a share of
//! its time (`docs/MOTION.md`; Material's curves are named in
//! [`crate::tokens`]).

use crate::tokens::Easing;

/// How far along `curve` (a cubic Bézier from (0, 0) to (1, 1), as CSS
/// writes one) an animation `t` of the way through its time is, 0 to 1:
/// the curve's x solved for `t` by Newton's method, then its y.
pub fn ease(curve: Easing, t: f32) -> f32 {
    let [x1, y1, x2, y2] = curve;
    let t = t.clamp(0.0, 1.0);
    // One coordinate of the curve at parameter s, and its slope.
    let at = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    };
    let slope = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * a + 6.0 * u * s * (b - a) + 3.0 * s * s * (1.0 - b)
    };
    let mut s = t;
    for _ in 0..8 {
        let d = slope(x1, x2, s);
        if d > -1e-6 && d < 1e-6 {
            break;
        }
        s = (s - (at(x1, x2, s) - t) / d).clamp(0.0, 1.0);
    }
    at(y1, y2, s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{EASE_EMPHASIZED_DECELERATE, EASE_STANDARD, EASE_STANDARD_ACCELERATE};

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
}
