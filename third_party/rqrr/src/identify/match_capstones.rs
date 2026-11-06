use std::cmp;

use crate::CapStone;

#[derive(Debug, Clone)]
pub struct CapStoneGroup(pub CapStone, pub CapStone, pub CapStone);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Neighbor {
    index: usize,
    distance: f64,
}

/// Return each pair Capstone indexes that are likely to be from a QR code
/// Ordered from most symmetric to least symmetric
pub fn find_and_rank_possible_neighbors(capstones: &[CapStone], idx: usize) -> Vec<(usize, usize)> {
    const VIABILITY_THRESHOLD: f64 = 0.25;

    // The parallelogram test first; only where it gives no viable pair,
    // the capstone's perspective too, so a code seen at a steep angle
    // forms the groups it formed before.
    let res = rank_pairs(capstones, idx, true, VIABILITY_THRESHOLD);
    if res.is_empty() {
        return rank_pairs(capstones, idx, false, VIABILITY_THRESHOLD);
    }
    res
}

fn rank_pairs(capstones: &[CapStone], idx: usize, affine: bool, threshold: f64) -> Vec<(usize, usize)> {
    let (hlist, vlist) = find_possible_neighbors(capstones, idx, affine);
    let mut res = Vec::new();
    struct NeighborSet {
        score: f64,
        h_index: usize,
        v_index: usize,
    }
    /* Test each possible grouping */
    for hn in hlist {
        for vn in vlist.iter() {
            let score = {
                if hn.distance < vn.distance {
                    (1.0f64 - hn.distance / vn.distance).abs()
                } else {
                    (1.0f64 - vn.distance / hn.distance).abs()
                }
            };
            if score < threshold {
                res.push(NeighborSet {
                    score,
                    h_index: hn.index,
                    v_index: vn.index,
                });
            }
        }
    }

    // A score is a ratio of two distances taken from the frame, so it
    // can be NaN: two capstones at the same place give 0.0 / 0.0.
    // Upstream panicked on the comparison. Ordering NaN last leaves the
    // pairs that do compare in the same order they had.
    res.sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(cmp::Ordering::Greater));
    res.iter().map(|n| (n.h_index, n.v_index)).collect()
}

fn find_possible_neighbors(capstones: &[CapStone], idx: usize, affine: bool) -> (Vec<Neighbor>, Vec<Neighbor>) {
    let cap = &capstones[idx];
    let mut hlist = Vec::new();
    let mut vlist = Vec::new();

    /* Look for potential neighbours by examining the relative gradients
     * from this capstone to others.
     */
    #[allow(clippy::needless_range_loop)]
    for others_idx in 0..capstones.len() {
        if others_idx == idx {
            continue;
        }

        let cmp_cap = &capstones[others_idx];

        // The capstone's own perspective is fitted to corners a few dozen
        // pixels apart and rounded to whole pixels, so extrapolated a
        // hundred modules away it can be off by tens of modules, and a
        // dense code's capstones are that far apart. The same corners fix
        // the axes' direction and the module size well enough for a
        // neighbour test, so the perspective is only the fall-back: for a
        // ring that is not a parallelogram, and where the parallelogram
        // finds no pair at all (a code tilted steeply).
        let by_affine = if affine { affine_offsets(cap, &cmp_cap.center) } else { None };
        let (mut u, mut v) = by_affine.unwrap_or_else(|| cap.c.unmap(&cmp_cap.center));
        u = (u - 3.5f64).abs();
        v = (v - 3.5f64).abs();

        if u < 0.2f64 * v {
            hlist.push(Neighbor {
                index: others_idx,
                distance: v,
            });
        }

        if v < 0.2f64 * u {
            vlist.push(Neighbor {
                index: others_idx,
                distance: u,
            });
        }
    }

    (hlist, vlist)
}

/// Where `p` lies in a capstone's own module coordinates, taking the
/// capstone as a parallelogram (no perspective): its centre is at
/// `(3.5, 3.5)` and each axis is the mean of the two opposite edges of
/// its ring. `None` for a ring whose edges do not span a plane.
pub(crate) fn affine_offsets(cap: &CapStone, p: &crate::identify::Point) -> Option<(f64, f64)> {
    let c = &cap.corners;
    let x = |i: usize| f64::from(c[i].x);
    let y = |i: usize| f64::from(c[i].y);
    // Per module along u (corner 0 to 1, 3 to 2) and along v (0 to 3, 1 to 2).
    let (ux, uy) = (((x(1) - x(0)) + (x(2) - x(3))) / 14.0, ((y(1) - y(0)) + (y(2) - y(3))) / 14.0);
    let (vx, vy) = (((x(3) - x(0)) + (x(2) - x(1))) / 14.0, ((y(3) - y(0)) + (y(2) - y(1))) / 14.0);
    let det = ux * vy - uy * vx;
    if det.abs() < 1e-6 {
        return None;
    }
    let (dx, dy) = (f64::from(p.x) - f64::from(cap.center.x), f64::from(p.y) - f64::from(cap.center.y));
    Some((3.5 + (dx * vy - dy * vx) / det, 3.5 + (ux * dy - uy * dx) / det))
}
