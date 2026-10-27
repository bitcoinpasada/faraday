//! Faraday's mark, the keyhole shield (`docs/DESIGN.md` §2.4, F10, F137,
//! F140): its three outlines in the mark's own units, worked out once and
//! kept, so that drawing it only scales them.
//!
//! The shield is `WIDTH` by `HEIGHT`. Its sides run straight to the
//! height over the golden ratio, then curve to the tip; its top corners
//! are rounded at the band times the golden ratio. The band, the lighter
//! rim, is `BAND` wide everywhere: the inner shield is the outer one
//! inset by it. The keyhole's head is as wide as its foot, the inner
//! width over the golden ratio squared; the slot runs from the head's
//! centre to the foot the head's width times the golden ratio; its neck
//! keeps the proportion it had to the head; and its middle, between the
//! head's top and the foot, stays where F137 put it.

use std::sync::OnceLock;

/// The shield's width, in the mark's units.
pub(crate) const WIDTH: f32 = 100.0;
/// The shield's height, in the mark's units.
pub(crate) const HEIGHT: f32 = 135.3;
/// The band's width, in the mark's units.
const BAND: f32 = 7.0;
/// The golden ratio.
const PHI: f32 = 1.618_034;
/// The neck's width over the head's (9 to 30 as F10 drew it).
const NECK: f32 = 0.3;
/// The keyhole's middle, between the head's top and the foot, in the
/// mark's units from the top (F137's).
const KEY_MIDDLE: f32 = 56.776;
/// How far down from a side's foot to the tip the curve's control point
/// sits, as a share of that drop; it sits `CURVE_IN` inside the side.
const CURVE_DOWN: f32 = 0.625;
/// How far inside the side the curve's control point sits.
const CURVE_IN: f32 = 2.0;
/// Straight pieces per quarter of a rounded corner.
const CORNER_PIECES: usize = 8;
/// Straight pieces per curve from a side to the tip.
const CURVE_PIECES: usize = 16;
/// Straight pieces of the keyhole's head.
const HEAD_PIECES: usize = 48;

/// The mark's outlines in its own units: the outer shield and the inner
/// one clockwise, the keyhole wound against them so that it is a hole.
pub(crate) struct Outlines {
    pub outer: Vec<(f32, f32)>,
    pub inner: Vec<(f32, f32)>,
    pub hole: Vec<(f32, f32)>,
}

/// The mark's outlines, worked out on the first call and kept.
pub(crate) fn outlines() -> &'static Outlines {
    static OUTLINES: OnceLock<Outlines> = OnceLock::new();
    OUTLINES.get_or_init(|| {
        let outer = outer();
        let inner = inset(&outer, BAND);
        Outlines {
            outer,
            inner,
            hole: hole(),
        }
    })
}

/// The outer shield, clockwise from the top of the left side.
fn outer() -> Vec<(f32, f32)> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let r = BAND * PHI;
    let side = HEIGHT / PHI;
    let mut out = Vec::new();
    for (cx, from) in [(r, PI), (WIDTH - r, PI + FRAC_PI_2)] {
        out.extend((0..=CORNER_PIECES).map(|i| {
            let a = from + FRAC_PI_2 * i as f32 / CORNER_PIECES as f32;
            (cx + r * a.cos(), r + r * a.sin())
        }));
    }
    let tip = (WIDTH / 2.0, HEIGHT);
    let ctrl_y = side + CURVE_DOWN * (HEIGHT - side);
    // A quadratic curve from the foot of a side to the tip.
    let curve = |sx: f32, cx: f32| {
        (0..=CURVE_PIECES).map(move |i| {
            let t = i as f32 / CURVE_PIECES as f32;
            let u = 1.0 - t;
            (
                u * u * sx + 2.0 * u * t * cx + t * t * tip.0,
                u * u * side + 2.0 * u * t * ctrl_y + t * t * tip.1,
            )
        })
    };
    out.extend(curve(WIDTH, WIDTH - CURVE_IN));
    let left: Vec<(f32, f32)> = curve(0.0, CURVE_IN).collect();
    out.extend(left.into_iter().rev().skip(1));
    out
}

/// A clockwise convex outline moved `by` inward everywhere: the part of
/// the plane inside every edge's line moved in by `by`.
fn inset(outline: &[(f32, f32)], by: f32) -> Vec<(f32, f32)> {
    let far = WIDTH + HEIGHT;
    let mut poly = vec![(-far, -far), (far, -far), (far, far), (-far, far)];
    for (i, &a) in outline.iter().enumerate() {
        let b = outline[(i + 1) % outline.len()];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len <= f32::EPSILON {
            continue;
        }
        // Inward, for a clockwise outline with y down.
        let (nx, ny) = (-dy / len, dx / len);
        let at = (a.0 + nx * by, a.1 + ny * by);
        let side = |p: (f32, f32)| (p.0 - at.0) * nx + (p.1 - at.1) * ny;
        let mut kept = Vec::with_capacity(poly.len() + 1);
        for j in 0..poly.len() {
            let (p, q) = (poly[(j + poly.len() - 1) % poly.len()], poly[j]);
            let (sp, sq) = (side(p), side(q));
            let cross = || {
                let t = sp / (sp - sq);
                (p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1))
            };
            if sq >= 0.0 {
                if sp < 0.0 {
                    kept.push(cross());
                }
                kept.push(q);
            } else if sp >= 0.0 {
                kept.push(cross());
            }
        }
        poly = kept;
    }
    poly
}

/// The keyhole, counter-clockwise: the round head and the slot widening
/// below it to the foot.
fn hole() -> Vec<(f32, f32)> {
    let head = (WIDTH - 2.0 * BAND) / (PHI * PHI);
    let r = head / 2.0;
    let slot = head * PHI;
    let waist = NECK * r;
    let cx = WIDTH / 2.0;
    let cy = KEY_MIDDLE - (r + slot) / 2.0 + r;
    let join = (r * r - waist * waist).sqrt();
    let from = join.atan2(-waist);
    let to = join.atan2(waist) + std::f32::consts::TAU;
    let mut hole: Vec<(f32, f32)> = (0..=HEAD_PIECES)
        .map(|i| {
            let a = from + (to - from) * i as f32 / HEAD_PIECES as f32;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    hole.extend([(cx + r, cy + slot), (cx - r, cy + slot)]);
    hole.reverse();
    hole
}
