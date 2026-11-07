use std::{cmp, mem};

use crate::{
    geometry,
    identify::match_capstones::CapStoneGroup,
    prepare::PreparedImage,
    prepare::{AreaFiller, ColoredRegion, ImageBuffer, PixelColor, Row, WORK_PER_GRID_MODULE},
    version_db::VERSION_DATA_BASE,
    BitGrid, CapStone, Point,
};

/// Location of a skewed square in an image
///
/// A skewed square formed by 3 [CapStones](struct.CapStone.html) and possibly
/// an alignment pattern located in the 4th corner. This can be converted into a
/// normal [Grid](trait.Grid.html) that can be decoded.
#[derive(Debug, Clone)]
pub struct SkewedGridLocation {
    pub grid_size: usize,
    pub c: geometry::Perspective,
}

impl SkewedGridLocation {
    /// Create a SkewedGridLocation from corners
    ///
    /// Given 3 corners of a grid, this tries to match the expected features of
    /// a QR code such that the SkewedGridLocation maps onto those features.
    ///
    /// For all grids this includes searching for timing patterns between
    /// capstones to determine the grid size.
    ///
    /// For bigger grids this includes searching for an alignment pattern in the
    /// 4 corner.
    ///
    /// If no sufficient match could be produces, return `None` instead.
    pub fn from_group<S>(img: &mut PreparedImage<S>, mut group: CapStoneGroup) -> Option<Self>
    where
        S: ImageBuffer,
    {
        /* Construct the hypotenuse line from A to C. B should be to
         * the left of this line.
         */
        let h0 = group.0.center;
        let mut hd = Point {
            x: group.2.center.x - group.0.center.x,
            y: group.2.center.y - group.0.center.y,
        };

        /* Make sure A-B-C is clockwise */
        if side_of(&group.1.center, &h0, &hd) > 0 {
            mem::swap(&mut group.0, &mut group.2);
            hd.x = -hd.x;
            hd.y = -hd.y
        }

        /* Rotate each capstone so that corner 0 is top-left with respect
         * to the grid.
         */
        rotate_capstone(&mut group.0, &h0, &hd)?;
        rotate_capstone(&mut group.1, &h0, &hd)?;
        rotate_capstone(&mut group.2, &h0, &hd)?;

        /* Check the timing pattern. This doesn't require a perspective
         * transform.
         */
        let grid_size = measure_timing_pattern(img, &group)?;

        /* Make an estimate based for the alignment pattern based on extending
         * lines from capstones A and C.
         */

        let edges_seed = geometry::line_intersect(
            &group.0.corners[0],
            &group.0.corners[1],
            &group.2.corners[0],
            &group.2.corners[3],
        );
        let mut align: Point;

        /* On V2+ grids, we should use the alignment pattern. */
        if grid_size > 21 {
            /* Try to find the actual location of the alignment pattern.
             * Extending the capstones' edges is only as good as the corner
             * pixels they start from, and on a dense code it is a hundred
             * modules: where that estimate finds nothing, the point that
             * completes the parallelogram of the three capstones' centres
             * is tried, at the module size the capstones show. (The point
             * from the capstones' outer corners lands on the alignment
             * pattern itself, but read only 5, 4 and 7 of 8 generated
             * dense frames at 6, 7 and 8 pixels a module, where this one
             * reads all 8.)
             */
            let by_edges = edges_seed.and_then(|seed| {
                let size = projected_module_area(seed, &group.0, &group.2)?;
                find_alignment_pattern(img, seed, size)
            });
            align = match by_edges {
                Some(p) => p,
                None => {
                    let seed = Point {
                        x: group.0.center.x + group.2.center.x - group.1.center.x,
                        y: group.0.center.y + group.2.center.y - group.1.center.y,
                    };
                    let area = (affine_module_area(&group.0) + affine_module_area(&group.2)) / 2.0;
                    find_alignment_pattern(img, seed, area as usize)?
                }
            };
            let score = -i64::from(hd.y) * i64::from(align.x) + i64::from(hd.x) * i64::from(align.y);
            let finder = LeftMostFinder {
                line_p: hd,
                best: align,
                score,
            };
            // An alignment pattern that cannot be repainted — already
            // claimed by another candidate, say — is no alignment
            // pattern for this group.
            let found = img.repaint_and_apply(
                (align.x as usize, align.y as usize),
                PixelColor::Alignment,
                finder,
            )?;
            align = found.best;
        } else {
            align = edges_seed?;
        }

        if version_from_grid_size(grid_size) >= VERSION_DATA_BASE.len() {
            return None;
        }

        if !img.spend(WORK_PER_GRID_MODULE * grid_size as u64) {
            return None;
        }
        let c = setup_perspective(img, &group, align, grid_size)?;

        Some(SkewedGridLocation { grid_size, c })
    }

    /// Convert into a grid referencing the underlying image as source
    pub fn into_grid_image<'a, S>(self, img: &'a PreparedImage<S>) -> RefGridImage<'a, S> {
        RefGridImage { grid: self, img }
    }
}

/// Get the version for a given grid size.
///
/// The returned version can be used to fetch the VersionInfo for the given grid
/// size.
fn version_from_grid_size(grid_size: usize) -> usize {
    (grid_size - 17) / 4
}

/// A Grid that references a bigger image
///
/// Given a grid location and an image, implement the [Grid
/// trait](trait.Grid.html) so that it may be decoded by
/// [decode](fn.decode.html)
pub struct RefGridImage<'a, S> {
    grid: SkewedGridLocation,
    img: &'a PreparedImage<S>,
}

impl<S> BitGrid for RefGridImage<'_, S>
where
    S: ImageBuffer,
{
    fn size(&self) -> usize {
        self.grid.grid_size
    }

    fn bit(&self, y: usize, x: usize) -> bool {
        // A cell the transform cannot place reads as white, which is
        // what an out-of-frame cell reads as; the decoder's error
        // correction is what then decides the grid's fate.
        match self.grid.c.map(x as f64 + 0.5, y as f64 + 0.5) {
            Some(p) => PixelColor::White != self.img.get_pixel_at_point(p),
            None => false,
        }
    }
}

fn setup_perspective<S>(
    img: &PreparedImage<S>,
    caps: &CapStoneGroup,
    align: Point,
    grid_size: usize,
) -> Option<geometry::Perspective>
where
    S: ImageBuffer,
{
    let initial = geometry::Perspective::create(
        &[
            caps.1.corners[0],
            caps.2.corners[0],
            align,
            caps.0.corners[0],
        ],
        (grid_size - 7) as f64,
        (grid_size - 7) as f64,
    )?;

    jiggle_perspective(img, initial, grid_size)
}

/// Which side of the line from `h0` along `hd` the point `p` lies on, as
/// a signed distance times the line's length. In `i64`, so no product of
/// coordinates taken from the frame wraps.
fn side_of(p: &Point, h0: &Point, hd: &Point) -> i64 {
    let w = |v: i32| i64::from(v);
    (w(p.x) - w(h0.x)) * -w(hd.y) + (w(p.y) - w(h0.y)) * w(hd.x)
}

/// Rotate a capstone's corners into their canonical order.
///
/// Returns `None` when the rotated corners have no perspective
/// transform: `Perspective::create` refuses a near-degenerate corner
/// set, and the corners come from the frame. Upstream asserted that this
/// could not happen.
fn rotate_capstone(cap: &mut CapStone, h0: &Point, hd: &Point) -> Option<()> {
    let (best_idx, _) = cap
        .corners
        .iter()
        .enumerate()
        .min_by_key(|(_, a)| side_of(a, h0, hd))?;

    /* Rotate the capstone */
    cap.corners.rotate_left(best_idx);
    cap.c = geometry::Perspective::create(&cap.corners, 7.0, 7.0)?;
    Some(())
}

//* Try the measure the timing pattern for a given QR code. This does
// * not require the global perspective to have been set up, but it
// * does require that the capstone corners have been set to their
// * canonical rotation.
// *
// * For each capstone, we find a point in the middle of the ring band
// * which is nearest the centre of the code. Using these points, we do
// * a horizontal and a vertical timing scan.
// */
fn measure_timing_pattern<S>(img: &PreparedImage<S>, caps: &CapStoneGroup) -> Option<usize>
where
    S: ImageBuffer,
{
    const US: [f64; 3] = [6.5f64, 6.5f64, 0.5f64];
    const VS: [f64; 3] = [0.5f64, 6.5f64, 6.5f64];
    let tpet0 = caps.0.c.map(US[0], VS[0])?;
    let tpet1 = caps.1.c.map(US[1], VS[1])?;
    let tpet2 = caps.2.c.map(US[2], VS[2])?;

    let hscan = timing_scan(img, &tpet1, &tpet2)?;
    let vscan = timing_scan(img, &tpet1, &tpet0)?;

    let scan = cmp::max(hscan, vscan);

    /* Choose the nearest allowable grid size */
    if scan < 1 {
        return None;
    }
    let size = scan + 13;
    let ver = (size as f64 - 15.0).floor() as usize / 4;
    Some(ver * 4 + 17)
}

fn timing_scan<S>(img: &PreparedImage<S>, p0: &Point, p1: &Point) -> Option<usize>
where
    S: ImageBuffer,
{
    let mut count = 0;
    let mut previous = None;
    for p in geometry::BresenhamScan::new(p0, p1)? {
        let pixel = img.get_pixel_at_point(p);
        if let Some(previous) = previous {
            if previous != pixel {
                count += 1;
            }
        }
        previous = Some(pixel);
    }

    Some(count)
}

/// The area, in pixels, one module of the grid covers around `seed`,
/// taken from the perspective of capstones `c0` and `c2`.
fn projected_module_area(seed: Point, c0: &CapStone, c2: &CapStone) -> Option<usize> {
    /* Guess another two corners of the alignment pattern so that we
     * can estimate its size.
     */
    let (u, v) = c0.c.unmap(&seed);
    let a = c0.c.map(u, v + 1.0)?;
    let (u, v) = c2.c.unmap(&seed);
    let c = c2.c.map(u + 1.0, v)?;
    // Points can lie anywhere in i32 on a crafted frame: the differences
    // and their products are worked out in i128, where they cannot wrap.
    let d = |p: i32, q: i32| i128::from(p) - i128::from(q);
    let area = d(a.x, seed.x) * -d(c.y, seed.y) + d(a.y, seed.y) * d(c.x, seed.x);
    usize::try_from(area.unsigned_abs()).ok()
}

/// The area, in pixels, of one module of the capstone, taking its ring
/// as a parallelogram.
fn affine_module_area(cap: &CapStone) -> f64 {
    let c = &cap.corners;
    let x = |i: usize| f64::from(c[i].x);
    let y = |i: usize| f64::from(c[i].y);
    let (ux, uy) = (
        ((x(1) - x(0)) + (x(2) - x(3))) / 14.0,
        ((y(1) - y(0)) + (y(2) - y(3))) / 14.0,
    );
    let (vx, vy) = (
        ((x(3) - x(0)) + (x(2) - x(1))) / 14.0,
        ((y(3) - y(0)) + (y(2) - y(1))) / 14.0,
    );
    (ux * vy - uy * vx).abs()
}

/// The fewest modules a code with an alignment pattern spans, counted
/// for the longest side of the frame: version 2 is 25 across, and 17
/// leaves room for a code seen at an angle or filling the frame's
/// diagonal. At 1920x1080 this bounds the search to about 1.3 million
/// steps.
const MIN_MODULES_ACROSS_FRAME: usize = 17;

/// Looks outwards from `align_seed` for the alignment pattern's centre
/// square, a region about `size_estimate` pixels in area.
fn find_alignment_pattern<S>(
    img: &mut PreparedImage<S>,
    mut align_seed: Point,
    size_estimate: usize,
) -> Option<Point>
where
    S: ImageBuffer,
{
    // The estimate comes from the capstones' perspective, which a crafted
    // frame can make as large as 2^31 pixels; the spiral then takes about
    // 100 times that many steps. A module of a code that has an alignment
    // pattern (version 2 up, 25 modules or more across) is no wider than
    // `MIN_MODULES_ACROSS_FRAME` of the frame's longer side: past that the
    // frame has no such pattern, and the group is refused.
    let side = img.width().max(img.height()) / MIN_MODULES_ACROSS_FRAME;
    if size_estimate > side.saturating_mul(side) {
        return None;
    }

    /* Spiral outwards from the estimate point until we find something
     * roughly the right size. Don't look too far from the estimate
     * point.
     */
    let mut dir = 0;
    let mut step_size = 1;

    while step_size * step_size < size_estimate * 100 {
        const DX_MAP: [i32; 4] = [1, 0, -1, 0];
        const DY_MAP: [i32; 4] = [0, -1, 0, 1];
        for _pass in 0..step_size {
            // Each step counts against the frame's budget, whether or not
            // it lands on anything.
            if !img.spend(1) {
                return None;
            }
            let x = align_seed.x as usize;
            let y = align_seed.y as usize;

            // Alignment pattern should not be white
            if x < img.width() && y < img.height() && PixelColor::White != img.get_pixel_at(x, y) {
                // A region that is not a plain black one (a capstone's, say)
                // is not the pattern, and the spiral goes on to its next
                // pixel all the same.
                if let Some(ColoredRegion::Unclaimed { pixel_count, .. }) =
                    img.get_region((x, y))
                {
                    // Matches expected size of alignment pattern
                    if pixel_count >= size_estimate / 2 && pixel_count <= size_estimate * 2 {
                        return Some(align_seed);
                    }
                }
            }

            align_seed.x = align_seed.x.checked_add(DX_MAP[dir])?;
            align_seed.y = align_seed.y.checked_add(DY_MAP[dir])?;
        }

        // Cycle directions
        dir = (dir + 1) % 4;
        if dir & 1 == 0 {
            step_size += 1
        }
    }
    None
}

struct LeftMostFinder {
    line_p: Point,
    best: Point,
    score: i64,
}

impl AreaFiller for LeftMostFinder {
    fn update(&mut self, row: Row) {
        let (px, py) = (i64::from(self.line_p.x), i64::from(self.line_p.y));
        let left_d = -py * row.left as i64 + px * row.y as i64;
        let right_d = -py * row.right as i64 + px * row.y as i64;

        if left_d < self.score {
            self.score = left_d;
            self.best.x = row.left as i32;
            self.best.y = row.y as i32;
        }

        if right_d < self.score {
            self.score = right_d;
            self.best.x = row.right as i32;
            self.best.y = row.y as i32;
        }
    }
}

fn jiggle_perspective<S>(
    img: &PreparedImage<S>,
    mut perspective: geometry::Perspective,
    grid_size: usize,
) -> Option<geometry::Perspective>
where
    S: ImageBuffer,
{
    let mut best = fitness_all(img, &perspective, grid_size)?;
    let mut adjustments: [f64; 8] = [
        perspective.0[0] * 0.02f64,
        perspective.0[1] * 0.02f64,
        perspective.0[2] * 0.02f64,
        perspective.0[3] * 0.02f64,
        perspective.0[4] * 0.02f64,
        perspective.0[5] * 0.02f64,
        perspective.0[6] * 0.02f64,
        perspective.0[7] * 0.02f64,
    ];

    for _pass in 0..5 {
        for i in 0..16 {
            let j = i >> 1;
            let old = perspective.0[j];
            let step = adjustments[j];

            let new = if i & 1 != 0 { old + step } else { old - step };

            perspective.0[j] = new;
            // An adjustment the sampler cannot follow is not an
            // improvement: put the coefficient back.
            match fitness_all(img, &perspective, grid_size) {
                Some(test) if test > best => best = test,
                _ => perspective.0[j] = old,
            }
        }

        #[allow(clippy::needless_range_loop)]
        for i in 0..8 {
            adjustments[i] *= 0.5f64;
        }
    }
    Some(perspective)
}
/* Compute a fitness score for the currently configured perspective
 * transform, using the features we expect to find by scanning the
 * grid.
 */
fn fitness_all<S>(
    img: &PreparedImage<S>,
    perspective: &geometry::Perspective,
    grid_size: usize,
) -> Option<i32>
where
    S: ImageBuffer,
{
    let version = version_from_grid_size(grid_size);
    let info = &VERSION_DATA_BASE[version];
    let mut score = 0;

    /* Check the timing pattern */
    for i in 0..(grid_size as i32 - 14) {
        let expect = if 0 != i & 1 { 1 } else { -1 };
        score += fitness_cell(img, perspective, i + 7, 6)? * expect;
        score += fitness_cell(img, perspective, 6, i + 7)? * expect;
    }

    /* Check capstones */
    score += fitness_capstone(img, perspective, 0, 0)?;
    score += fitness_capstone(img, perspective, grid_size as i32 - 7, 0)?;
    score += fitness_capstone(img, perspective, 0, grid_size as i32 - 7)?;

    /* Check alignment patterns */
    let mut ap_count = 0;
    while ap_count < 7 && info.apat[ap_count] != 0 {
        ap_count += 1
    }
    for i in 1..(ap_count.saturating_sub(1)) {
        score += fitness_apat(img, perspective, 6, info.apat[i] as i32)?;
        score += fitness_apat(img, perspective, info.apat[i] as i32, 6)?;
    }
    for i in 1..ap_count {
        for j in 1..ap_count {
            score += fitness_apat(img, perspective, info.apat[i] as i32, info.apat[j] as i32)?;
        }
    }
    Some(score)
}

fn fitness_apat<S>(
    img: &PreparedImage<S>,
    perspective: &geometry::Perspective,
    cx: i32,
    cy: i32,
) -> Option<i32>
where
    S: ImageBuffer,
{
    Some(
        fitness_cell(img, perspective, cx, cy)? - fitness_ring(img, perspective, cx, cy, 1)?
            + fitness_ring(img, perspective, cx, cy, 2)?,
    )
}

fn fitness_ring<S>(
    img: &PreparedImage<S>,
    perspective: &geometry::Perspective,
    cx: i32,
    cy: i32,
    radius: i32,
) -> Option<i32>
where
    S: ImageBuffer,
{
    let mut score = 0;
    for i in 0..(radius * 2) {
        score += fitness_cell(img, perspective, cx - radius + i, cy - radius)?;
        score += fitness_cell(img, perspective, cx - radius, cy + radius - i)?;
        score += fitness_cell(img, perspective, cx + radius, cy - radius + i)?;
        score += fitness_cell(img, perspective, cx + radius - i, cy + radius)?;
    }
    Some(score)
}

fn fitness_cell<S>(
    img: &PreparedImage<S>,
    perspective: &geometry::Perspective,
    x: i32,
    y: i32,
) -> Option<i32>
where
    S: ImageBuffer,
{
    const OFFSETS: [f64; 3] = [0.3f64, 0.5f64, 0.7f64];
    let mut score = 0;
    #[allow(clippy::needless_range_loop)]
    for v in 0..3 {
        for u in 0..3 {
            // A sample the transform cannot place makes the whole
            // candidate unusable, so the score has no meaning.
            let p = perspective.map(x as f64 + OFFSETS[u], y as f64 + OFFSETS[v])?;
            if !(p.y < 0 || p.y as usize >= img.height() || p.x < 0 || p.x as usize >= img.width())
            {
                if PixelColor::White != img.get_pixel_at_point(p) {
                    score += 1
                } else {
                    score -= 1
                }
            }
        }
    }
    Some(score)
}

fn fitness_capstone<S>(
    img: &PreparedImage<S>,
    perspective: &geometry::Perspective,
    x: i32,
    y: i32,
) -> Option<i32>
where
    S: ImageBuffer,
{
    Some(
        fitness_cell(img, perspective, x + 3, y + 3)?
            + fitness_ring(img, perspective, x + 3, y + 3, 1)?
            - fitness_ring(img, perspective, x + 3, y + 3, 2)?
            + fitness_ring(img, perspective, x + 3, y + 3, 3)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Perspective;

    fn scaled(s: f64) -> Perspective {
        Perspective([s, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0])
    }

    fn capstone(c: Perspective) -> CapStone {
        let p = Point { x: 0, y: 0 };
        CapStone {
            corners: [p; 4],
            center: p,
            c,
        }
    }

    #[test]
    fn the_module_area_of_far_off_points_does_not_wrap() {
        // The guessed corners lie 2e9 pixels from the seed: the i32
        // products wrapped in release and panicked in debug.
        let seed = Point {
            x: -1_000_000_000,
            y: -1_000_000_000,
        };
        let area = projected_module_area(seed, &capstone(scaled(2e9)), &capstone(scaled(2e9)));
        assert!(area.is_some_and(|a| a > 3_000_000_000_000_000_000));
    }

    #[test]
    fn the_alignment_search_at_the_edge_of_i32_ends_with_no_pattern() {
        let mut img = PreparedImage::prepare_from_greyscale(8, 8, |_, _| 0);
        for seed in [
            Point {
                x: i32::MAX,
                y: i32::MAX,
            },
            Point {
                x: i32::MIN,
                y: i32::MIN,
            },
            Point {
                x: i32::MAX - 1,
                y: i32::MIN,
            },
        ] {
            assert_eq!(find_alignment_pattern(&mut img, seed, 4), None);
        }
    }
}
