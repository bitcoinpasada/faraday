use std::cmp;

use crate::identify::match_capstones::CapStoneGroup;
use crate::identify::Point;

/// How many regions one image can colour at a time.
///
/// A region's index travels in the pixel values themselves, as
/// `PixelColor::Discarded(idx)`, which is written as `idx + 5`. A `u8`
/// pixel holds indices 0 through 250, so the table that describes them
/// is exactly that long. A frame of a large symbol has far more black
/// regions than that — a version 40 symbol at three pixels a module has
/// thousands — so the table is a cache: when it is full, the least
/// recently used region is repainted black and its index reused, which
/// is what upstream's `LruCache` did and what keeps large symbols
/// decodable.
pub(crate) const MAX_REGIONS: usize = 251;

/// An black-and-white image that can be mutated on search for QR codes
///
/// During search for QR codes, some black zones will be recolored in
/// 'different' shades of black. This is done to speed up the search and
/// mitigate the impact of a huge zones.
pub struct PreparedImage<S> {
    buffer: S,
    regions: Box<RegionTable>,
}

/// The regions coloured so far, indexed by region index, least recently
/// used first out.
///
/// Upstream held the same 251 entries in an `LruCache<u8,
/// ColoredRegion>`. The key is a `u8` bounded by `MAX_REGIONS`, so the
/// entries are an array and recency is a stamp per entry; eviction picks
/// the smallest stamp.
#[derive(Clone)]
pub(crate) struct RegionTable {
    entries: [Option<TableEntry>; MAX_REGIONS],
    fill: usize,
    clock: u64,
}

#[derive(Copy, Clone)]
struct TableEntry {
    region: ColoredRegion,
    used: u64,
}

impl RegionTable {
    fn new() -> Box<Self> {
        Box::new(RegionTable {
            entries: [None; MAX_REGIONS],
            fill: 0,
            clock: 0,
        })
    }

    fn stamp(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// The region at an index, made the most recently used.
    fn get(&mut self, idx: u8) -> Option<ColoredRegion> {
        let stamp = self.stamp();
        let entry = self.entries.get_mut(idx as usize)?.as_mut()?;
        entry.used = stamp;
        Some(entry.region)
    }

    /// The index to colour the next region with.
    ///
    /// While the table has room this is a fresh index. Once it is full
    /// the least recently used entry is dropped and returned with its
    /// index, for the caller to repaint black before reusing the index.
    fn claim(&mut self) -> (u8, Option<ColoredRegion>) {
        if self.fill < MAX_REGIONS {
            let idx = self.fill as u8;
            self.fill += 1;
            return (idx, None);
        }

        let (idx, _) = self
            .entries
            .iter()
            .enumerate()
            .min_by_key(|(_, e)| e.map(|e| e.used).unwrap_or(0))
            .map(|(i, e)| (i as u8, e))
            .unwrap_or((0, &None));
        let evicted = self.entries[idx as usize].take().map(|e| e.region);
        (idx, evicted)
    }

    fn put(&mut self, idx: u8, region: ColoredRegion) {
        let stamp = self.stamp();
        if let Some(slot) = self.entries.get_mut(idx as usize) {
            *slot = Some(TableEntry {
                region,
                used: stamp,
            });
        }
    }
}

impl<S> Clone for PreparedImage<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        PreparedImage {
            buffer: self.buffer.clone(),
            regions: self.regions.clone(),
        }
    }
}

pub trait ImageBuffer {
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn get_pixel(&self, x: usize, y: usize) -> u8;
    fn set_pixel(&mut self, x: usize, y: usize, val: u8);
}

#[cfg(feature = "img")]
impl<
        T: image::GenericImage<Pixel = image::Luma<u8>>
            + image::GenericImageView<Pixel = image::Luma<u8>>,
    > ImageBuffer for T
{
    fn width(&self) -> usize {
        self.width() as usize
    }

    fn height(&self) -> usize {
        self.height() as usize
    }

    fn get_pixel(&self, x: usize, y: usize) -> u8 {
        self.get_pixel(x as u32, y as u32).0[0]
    }

    fn set_pixel(&mut self, x: usize, y: usize, val: u8) {
        let pixel = image::Luma::<u8>::from([val; 1]);

        self.put_pixel(x as u32, y as u32, pixel);
    }
}

#[derive(Clone, Debug)]
pub struct BasicImageBuffer {
    w: usize,
    h: usize,
    pixels: Box<[u8]>,
}

impl ImageBuffer for BasicImageBuffer {
    fn width(&self) -> usize {
        self.w
    }

    fn height(&self) -> usize {
        self.h
    }

    fn get_pixel(&self, x: usize, y: usize) -> u8 {
        let w = self.width();
        self.pixels[(y * w) + x]
    }

    fn set_pixel(&mut self, x: usize, y: usize, val: u8) {
        let w = self.width();
        self.pixels[(y * w) + x] = val
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Row {
    pub left: usize,
    pub right: usize,
    pub y: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PixelColor {
    White,
    Black,
    CapStone,
    Alignment,
    Tmp1,
    Discarded(u8),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ColoredRegion {
    Unclaimed {
        color: PixelColor,
        src_x: usize,
        src_y: usize,
        pixel_count: usize,
    },
    CapStone,
    Alignment,
    Tmp1,
}

impl From<u8> for PixelColor {
    fn from(x: u8) -> Self {
        match x {
            0 => PixelColor::White,
            1 => PixelColor::Black,
            2 => PixelColor::CapStone,
            3 => PixelColor::Alignment,
            4 => PixelColor::Tmp1,
            x => PixelColor::Discarded(x - 5),
        }
    }
}

impl From<PixelColor> for u8 {
    fn from(c: PixelColor) -> Self {
        match c {
            PixelColor::White => 0,
            PixelColor::Black => 1,
            PixelColor::CapStone => 2,
            PixelColor::Alignment => 3,
            PixelColor::Tmp1 => 4,
            PixelColor::Discarded(x) => x + 5,
        }
    }
}

impl PartialEq<u8> for PixelColor {
    fn eq(&self, other: &u8) -> bool {
        let rep: u8 = (*self).into();
        rep == *other
    }
}

pub trait AreaFiller {
    fn update(&mut self, row: Row);
}

impl<F> AreaFiller for F
where
    F: FnMut(Row),
{
    fn update(&mut self, row: Row) {
        self(row)
    }
}

struct AreaCounter(usize);

impl AreaFiller for AreaCounter {
    fn update(&mut self, row: Row) {
        self.0 += row.right - row.left + 1;
    }
}

impl<S> PreparedImage<S>
where
    S: ImageBuffer,
{
    pub fn prepare(mut buf: S) -> Self {
        let w = buf.width();
        let h = buf.height();
        let mut row_average = vec![0; w];
        let mut avg_v = 0;
        let mut avg_u = 0;

        let threshold_s = cmp::max(w / 8, 1);

        for y in 0..h {
            row_average.fill(0);

            for x in 0..w {
                let (v, u) = if y % 2 == 0 {
                    (w - 1 - x, x)
                } else {
                    (x, w - 1 - x)
                };
                avg_v = avg_v * (threshold_s - 1) / threshold_s + buf.get_pixel(v, y) as usize;
                avg_u = avg_u * (threshold_s - 1) / threshold_s + buf.get_pixel(u, y) as usize;
                row_average[v] += avg_v;
                row_average[u] += avg_u;
            }

            #[allow(clippy::needless_range_loop)]
            for x in 0..w {
                let fill = if (buf.get_pixel(x, y) as usize)
                    < row_average[x] * (100 - 5) / (200 * threshold_s)
                {
                    PixelColor::Black
                } else {
                    PixelColor::White
                };
                buf.set_pixel(x, y, fill.into());
            }
        }

        PreparedImage {
            buffer: buf,
            regions: RegionTable::new(),
        }
    }

    /// Group [CapStones](struct.CapStone.html) into [Grids](struct.Grid.html)
    /// that are likely QR codes
    ///
    /// Return a vector of Grids
    pub fn detect_grids<'a>(
        &'a mut self,
    ) -> Vec<crate::Grid<crate::identify::grid::RefGridImage<'a, S>>>
    where
        S: Clone,
    {
        let mut res = Vec::new();
        let stones = crate::capstones_from_image(self);
        let groups = self.find_groupings(stones);
        let locations: Vec<_> = groups
            .into_iter()
            .filter_map(|v| crate::SkewedGridLocation::from_group(self, v))
            .collect();
        for grid_location in locations {
            // A grid whose own four corners cannot be placed in the
            // image is not a grid worth reporting.
            let far = grid_location.grid_size as f64 + 1.0;
            let c = &grid_location.c;
            let bounds = match (c.map(0.0, 0.0), c.map(far, 0.0), c.map(far, far), c.map(0.0, far))
            {
                (Some(a), Some(b), Some(c), Some(d)) => [a, b, c, d],
                _ => continue,
            };
            let grid = grid_location.into_grid_image(self);
            res.push(crate::Grid { grid, bounds });
        }

        res
    }

    /// Find CapStones that form a grid
    ///
    /// By trying to match up the relative perspective of 3
    /// [CapStones](struct.CapStone.html) along with other criteria we can find the
    /// CapStones that corner the same QR code.
    fn find_groupings(&mut self, capstones: Vec<crate::CapStone>) -> Vec<CapStoneGroup>
    where
        S: Clone,
    {
        let mut used_capstones = Vec::new();
        let mut groups = Vec::new();
        for idx in 0..capstones.len() {
            if used_capstones.contains(&idx) {
                continue;
            }
            let pairs = crate::identify::find_and_rank_possible_neighbors(&capstones, idx);
            for pair in pairs {
                if used_capstones.contains(&pair.0) || used_capstones.contains(&pair.1) {
                    continue;
                }
                let group_under_test = CapStoneGroup(
                    capstones[pair.0].clone(),
                    capstones[idx].clone(),
                    capstones[pair.1].clone(),
                );
                // Confirm that this group has the other requirements of a QR code.
                // A copy of the input image is used to not contaminate the
                // original on an incorrect set of CapStones
                let mut image_copy = self.clone();
                if crate::SkewedGridLocation::from_group(&mut image_copy, group_under_test.clone())
                    .is_none()
                {
                    continue;
                }
                // This is a viable set, save this grouping
                groups.push(group_under_test);
                used_capstones.push(pair.0);
                used_capstones.push(idx);
                used_capstones.push(pair.1);
            }
        }
        groups
    }

    pub fn without_preparation(buf: S) -> Self {
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                assert!(buf.get_pixel(x, y) < 2);
            }
        }

        PreparedImage {
            buffer: buf,
            regions: RegionTable::new(),
        }
    }

    /// Return the width of the image
    pub fn width(&self) -> usize {
        self.buffer.width()
    }

    /// Return the height of the image
    pub fn height(&self) -> usize {
        self.buffer.height()
    }

    /// The region covering a pixel, or `None` where there is none to
    /// give.
    ///
    /// `None` means the pixel is white, or names a region index the
    /// table no longer holds, or could not be repainted. Every caller
    /// treats all three as "nothing of interest at this pixel".
    /// Upstream panicked on the first two.
    pub(crate) fn get_region(&mut self, (x, y): (usize, usize)) -> Option<ColoredRegion> {
        let color: PixelColor = self.buffer.get_pixel(x, y).into();
        match color {
            PixelColor::Discarded(r) => self.regions.get(r),
            PixelColor::Black => {
                let (reg_idx, evicted) = self.regions.claim();
                // The evicted region goes back to plain black, so that
                // it can be found again later under another index.
                if let Some(ColoredRegion::Unclaimed {
                    src_x,
                    src_y,
                    color,
                    ..
                }) = evicted
                {
                    let _ = self.flood_fill(
                        src_x,
                        src_y,
                        color.into(),
                        PixelColor::Black.into(),
                        |_| (),
                    );
                }

                let next_reg_color = PixelColor::Discarded(reg_idx);
                let counter = self.repaint_and_apply((x, y), next_reg_color, AreaCounter(0))?;
                let new_reg = ColoredRegion::Unclaimed {
                    color: next_reg_color,
                    src_x: x,
                    src_y: y,
                    pixel_count: counter.0,
                };
                self.regions.put(reg_idx, new_reg);
                Some(new_reg)
            }
            PixelColor::Tmp1 => Some(ColoredRegion::Tmp1),
            PixelColor::Alignment => Some(ColoredRegion::Alignment),
            PixelColor::CapStone => Some(ColoredRegion::CapStone),
            PixelColor::White => None,
        }
    }

    /// Repaint the region at a pixel, or `None` if there is nothing to
    /// repaint.
    ///
    /// A white pixel has no region, and a pixel already in the target
    /// colour has been repainted by an earlier candidate — an alignment
    /// pattern shared with another capstone group reaches the second
    /// case. Upstream panicked on both.
    pub(crate) fn repaint_and_apply<F>(
        &mut self,
        (x, y): (usize, usize),
        target_color: PixelColor,
        fill: F,
    ) -> Option<F>
    where
        F: AreaFiller,
    {
        let src = self.buffer.get_pixel(x, y);
        if PixelColor::White == src || target_color == src {
            return None;
        }

        Some(self.flood_fill(x, y, src, target_color.into(), fill))
    }

    pub fn get_pixel_at_point(&self, p: Point) -> PixelColor {
        let x = cmp::max(0, cmp::min((self.width() - 1) as i32, p.x));
        let y = cmp::max(0, cmp::min((self.height() - 1) as i32, p.y));
        self.buffer.get_pixel(x as usize, y as usize).into()
    }

    pub fn get_pixel_at(&self, x: usize, y: usize) -> PixelColor {
        self.buffer.get_pixel(x, y).into()
    }

    #[cfg(feature = "img")]
    pub fn write_state_to(&self, p: &str) {
        let mut dyn_img = image::RgbImage::new(self.width() as u32, self.height() as u32);
        const COLORS: [[u8; 3]; 8] = [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [255, 255, 0],
            [255, 0, 255],
            [0, 255, 255],
            [128, 128, 128],
            [128, 0, 128],
        ];
        for y in 0..self.height() {
            for x in 0..self.width() {
                let px = self.buffer.get_pixel(x, y);
                dyn_img.get_pixel_mut(x as u32, y as u32).0 = if px == 0 {
                    [255, 255, 255]
                } else if px == 1 {
                    [0, 0, 0]
                } else {
                    let i = self.buffer.get_pixel(x, y) - 2;
                    COLORS[(i % 8) as usize]
                }
            }
        }
        dyn_img.save(p).unwrap();
    }

    fn flood_fill<F>(&mut self, x: usize, y: usize, from: u8, to: u8, mut fill: F) -> F
    where
        F: AreaFiller,
    {
        if from == to {
            return fill;
        }
        let w = self.width();
        let mut queue = Vec::new();
        queue.push((x, y));

        while let Some((x, y)) = queue.pop() {
            // Bail early in case there is nothing to fill
            if self.buffer.get_pixel(x, y) == to || self.buffer.get_pixel(x, y) != from {
                continue;
            }

            let mut left = x;
            let mut right = x;

            while left > 0 && self.buffer.get_pixel(left - 1, y) == from {
                left -= 1;
            }
            while right < w - 1 && self.buffer.get_pixel(right + 1, y) == from {
                right += 1
            }

            /* Fill the extent */
            for idx in left..=right {
                self.buffer.set_pixel(idx, y, to);
            }

            fill.update(Row { left, right, y });

            /* Seed new flood-fills */
            if y > 0 {
                let mut seeded_previous = false;
                for x in left..=right {
                    let p = self.buffer.get_pixel(x, y - 1);
                    if p == from {
                        if !seeded_previous {
                            queue.push((x, y - 1));
                        }
                        seeded_previous = true;
                    } else {
                        seeded_previous = false;
                    }
                }
            }
            if y < self.height() - 1 {
                let mut seeded_previous = false;
                for x in left..=right {
                    let p = self.buffer.get_pixel(x, y + 1);
                    if p == from {
                        if !seeded_previous {
                            queue.push((x, y + 1));
                        }
                        seeded_previous = true;
                    } else {
                        seeded_previous = false;
                    }
                }
            }
        }
        fill
    }
}

impl PreparedImage<BasicImageBuffer> {
    /// Given a function with binary output, generate a searchable image
    ///
    /// If the given function returns `true` the matching pixel will be 'black'.
    pub fn prepare_from_bitmap<F>(w: usize, h: usize, mut fill: F) -> Self
    where
        F: FnMut(usize, usize) -> bool,
    {
        let capacity = w.checked_mul(h).expect("Image dimensions caused overflow");
        let mut pixels = Vec::with_capacity(capacity);

        for y in 0..h {
            for x in 0..w {
                let col = if fill(x, y) {
                    PixelColor::Black
                } else {
                    PixelColor::White
                };
                pixels.push(col.into())
            }
        }
        let pixels = pixels.into_boxed_slice();
        let buffer = BasicImageBuffer { w, h, pixels };
        PreparedImage::without_preparation(buffer)
    }

    /// Given a byte valued function, generate a searchable image
    ///
    /// The values returned by the function are interpreted as luminance. i.e. a
    /// value of 0 is black, 255 is white.
    pub fn prepare_from_greyscale<F>(w: usize, h: usize, mut fill: F) -> Self
    where
        F: FnMut(usize, usize) -> u8,
    {
        let capacity = w.checked_mul(h).expect("Image dimensions caused overflow");
        let mut data = Vec::with_capacity(capacity);
        for y in 0..h {
            for x in 0..w {
                data.push(fill(x, y));
            }
        }
        let pixels = data.into_boxed_slice();
        let buffer = BasicImageBuffer { w, h, pixels };
        PreparedImage::prepare(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img_from_array(array: [[u8; 3]; 3]) -> PreparedImage<BasicImageBuffer> {
        let mut pixels = Vec::new();
        for col in array.iter() {
            for item in col.iter() {
                if *item == 0 {
                    pixels.push(0)
                } else {
                    pixels.push(1)
                }
            }
        }
        let buffer = BasicImageBuffer {
            w: 3,
            h: 3,
            pixels: pixels.into_boxed_slice(),
        };

        PreparedImage {
            buffer,
            regions: RegionTable::new(),
        }
    }

    #[test]
    fn test_flood_fill_full() {
        let mut test_full = img_from_array([[1, 1, 1], [1, 1, 1], [1, 1, 1]]);

        test_full.flood_fill(0, 0, 1, 2, &mut |_| ());

        for x in 0..3 {
            for y in 0..3 {
                assert_eq!(test_full.get_pixel_at(x, y), 2);
            }
        }
    }

    #[test]
    fn test_flood_fill_single() {
        let mut test_single = img_from_array([[1, 0, 1], [0, 1, 0], [1, 0, 1]]);

        test_single.flood_fill(1, 1, 1, 2, &mut |_| ());

        for x in 0..3 {
            for y in 0..3 {
                if x == 1 && y == 1 {
                    assert_eq!(test_single.get_pixel_at(x, y), 2);
                } else {
                    let col = if (x + y) % 2 == 0 { 1 } else { 0 };
                    assert_eq!(test_single.get_pixel_at(x, y), col);
                }
            }
        }
    }

    #[test]
    fn test_flood_fill_ring() {
        let mut test_ring = img_from_array([[1, 1, 1], [1, 0, 1], [1, 1, 1]]);

        test_ring.flood_fill(0, 0, 1, 2, &mut |_| ());

        for x in 0..3 {
            for y in 0..3 {
                if x == 1 && y == 1 {
                    assert_eq!(test_ring.get_pixel_at(x, y), 0);
                } else {
                    assert_eq!(test_ring.get_pixel_at(x, y), 2);
                }
            }
        }
    }

    #[test]
    fn test_flood_fill_u() {
        let mut test_u = img_from_array([[1, 0, 1], [1, 0, 1], [1, 1, 1]]);

        test_u.flood_fill(0, 0, 1, 2, &mut |_| ());

        for x in 0..3 {
            for y in 0..3 {
                if x == 1 && (y == 0 || y == 1) {
                    assert_eq!(test_u.get_pixel_at(x, y), 0);
                } else {
                    assert_eq!(test_u.get_pixel_at(x, y), 2);
                }
            }
        }
    }

    #[test]
    fn test_flood_fill_empty() {
        let mut test_empty = img_from_array([[0, 0, 0], [0, 0, 0], [0, 0, 0]]);

        test_empty.flood_fill(1, 1, 1, 2, &mut |_| ());

        for x in 0..3 {
            for y in 0..3 {
                assert_eq!(test_empty.get_pixel_at(x, y), 0)
            }
        }
    }

    #[test]
    fn test_get_region() {
        let mut test_u = img_from_array([[1, 0, 1], [1, 0, 1], [1, 1, 1]]);

        let reg = test_u.get_region((0, 0)).expect("black pixel has a region");
        let (color, src_x, src_y, pixel_count) = match reg {
            ColoredRegion::Unclaimed {
                color,
                src_x,
                src_y,
                pixel_count,
            } => (color, src_x, src_y, pixel_count),
            x => panic!("Expected Region::Unclaimed, got {:?}", x),
        };
        assert_eq!(0, src_x);
        assert_eq!(0, src_y);
        assert_eq!(7, pixel_count);
        for x in 0..3 {
            for y in 0..3 {
                if x == 1 && (y == 0 || y == 1) {
                    assert_eq!(PixelColor::White, test_u.get_pixel_at(x, y));
                } else {
                    assert_eq!(color, test_u.get_pixel_at(x, y));
                }
            }
        }
    }
}
