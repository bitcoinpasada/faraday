//! Finding the words and the line numbers on a page: the frame is scaled to
//! the detector's input, the detector's word and number kernels are joined
//! into axis-aligned boxes, and a page whose words cover a small part of the
//! frame is looked at once more on the region that holds them.
//!
//! Work is bounded for any frame: one pass over its pixels to scale it, the
//! detector's fixed cost, a flood fill of the 144 by 192 maps, and a join of
//! at most `MAX_KERNELS` kernels per kind (more is reported, not read). A
//! frame is looked at at most twice, and a frame with a side over `MAX_SIDE`
//! is refused (`Error::Size`).

use zeroize::Zeroizing;

use crate::detector::{Maps, detector_maps};
use crate::nn::Buf;
use crate::{DETECTOR_HEIGHT, DETECTOR_WIDTH, Error};

/// Pixels (of a 144 by 192 map) a kernel must have to count.
const MIN_KERNEL_PIXELS: usize = 2;
/// Output value to box height scale.
const HEIGHT_SCALE: f32 = 8.0;
/// Each end of a box is extended by this share of its height.
const END_EXTENSION: f32 = 0.25;
/// Most kernels of one kind a page may hold before it is reported as too
/// full to read (a hostile frame: thousands of specks).
pub const MAX_KERNELS: usize = 1024;
/// The longest side of a frame, in pixels; a longer one is refused with
/// `Error::Size` (the scaling holds a few bytes for each pixel of a side).
pub const MAX_SIDE: usize = 20_000;
/// Words cover less than this share of the frame: look again on their region.
const ZOOM_SHARE: f32 = 0.5;
/// The region of that second look reaches this share of its size past the
/// words, on each side.
const ZOOM_MARGIN: f32 = 0.15;
/// Two boxes are pieces of one word if they overlap along the line and
/// their heights overlap by at least this share of the lower one's.
const SAME_LINE: f32 = 0.5;
/// A box whose kernels reach over more than this share of its height may hold
/// two lines. The kernels of one line reach 0.45 to 0.73 of the box's height
/// on twelve published strips down one page at twice their size (whole rows
/// of the map), and a real word on a page of two columns reached more than
/// 0.75; two lines joined are the kernel of one plus the pitch between them:
/// one line's 0.55 or so plus a pitch of the height or so, over a height the
/// network gave of 1.0 to 1.6 lines: 0.9 to 1.5. What it does not flag: a
/// merged kernel whose height the network gave as the two lines' whole (about
/// 2 lines, 0.75 to 0.85), which only the page's median rule can catch.
///
/// Provisional: set above the largest reach seen on real words, tried on
/// synthetic maps; not measured on real pages of two tight lines.
const TWO_LINES_KERNEL: f32 = 0.8;

/// A box in the frame's pixels: left, top, right, bottom. It may reach past
/// the frame (the writing runs off it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left.
    pub x0: f32,
    /// Top.
    pub y0: f32,
    /// Right.
    pub x1: f32,
    /// Bottom.
    pub y1: f32,
}

impl Rect {
    /// Height of the box.
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Width of the box.
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// The two boxes share some area.
    pub(crate) fn overlaps(&self, o: &Rect) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }

    pub(crate) fn union(&self, o: &Rect) -> Rect {
        Rect {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }
}

/// What the detector found on a page.
#[derive(Debug, Clone, Default)]
pub struct Found {
    /// Word boxes, in the frame's pixels.
    pub words: Vec<Rect>,
    /// Line-number boxes, in the frame's pixels.
    pub numbers: Vec<Rect>,
    /// More than `MAX_KERNELS` word or number kernels: nothing is listed.
    pub too_many: bool,
    /// For each word box, whether its kernels reach over so much of the height
    /// the network gave it that it may hold two lines (a missing entry is
    /// false).
    pub tall: Vec<bool>,
}

/// Where the frame stands in the detector's input.
struct Placement {
    /// Offset of the scaled frame in the input.
    ox: usize,
    oy: usize,
    /// Size of the scaled frame in the input.
    dw: usize,
    dh: usize,
    /// Size of the frame (or region).
    w: usize,
    h: usize,
}

impl Placement {
    fn to_frame(&self, mx: f32, my: f32) -> (f32, f32) {
        // A map pixel is two input pixels.
        let x = (2.0 * mx - self.ox as f32) * self.w as f32 / self.dw as f32;
        let y = (2.0 * my - self.oy as f32) * self.h as f32 / self.dh as f32;
        (x, y)
    }
}

/// `(source index, destination index, weight)` of the pieces in which a
/// source pixel and a destination pixel overlap, for a line of `n` source
/// pixels scaled to `m`. A destination pixel's weights add up to `n`.
fn axis(n: usize, m: usize) -> Vec<(usize, usize, u64)> {
    let (nn, mm) = (n as u64, m as u64);
    let mut out = Vec::with_capacity(n + m);
    let (mut i, mut d) = (0usize, 0usize);
    while i < n && d < m {
        let (si, ei) = (i as u64 * mm, (i as u64 + 1) * mm);
        let (sd, ed) = (d as u64 * nn, (d as u64 + 1) * nn);
        let (lo, hi) = (si.max(sd), ei.min(ed));
        if hi > lo {
            out.push((i, d, hi - lo));
        }
        if ei <= ed {
            i += 1;
        }
        if ed <= ei {
            d += 1;
        }
    }
    out
}

/// The frame scaled to fit the detector's input, aspect kept, by averaging
/// the pixels each new pixel covers, and padded with its median grey.
fn scale_to_input(w: usize, h: usize, gray: &[u8]) -> (Buf<u8>, Placement) {
    let (iw, ih) = (DETECTOR_WIDTH, DETECTOR_HEIGHT);
    let (dw, dh) = if w * ih <= h * iw {
        (((2 * w * ih + h) / (2 * h)).clamp(1, iw), ih)
    } else {
        (iw, ((2 * h * iw + w) / (2 * w)).clamp(1, ih))
    };
    let xs = axis(w, dw);
    let ys = axis(h, dh);
    let mut acc: Zeroizing<Vec<u64>> = Zeroizing::new(vec![0; dw * dh]);
    for &(sy, dy, wy) in &ys {
        let Some(row) = gray.get(sy * w..(sy + 1) * w) else {
            continue;
        };
        let Some(out) = acc.get_mut(dy * dw..(dy + 1) * dw) else {
            continue;
        };
        for &(sx, dx, wx) in &xs {
            if let (Some(&v), Some(o)) = (row.get(sx), out.get_mut(dx)) {
                *o += u64::from(v) * wx * wy;
            }
        }
    }
    let total = (w as u64) * (h as u64);
    let small: Vec<u8> = acc
        .iter()
        .map(|&a| ((a + total / 2) / total.max(1)).min(255) as u8)
        .collect();
    let mut hist = [0usize; 256];
    for &v in &small {
        if let Some(c) = hist.get_mut(usize::from(v)) {
            *c += 1;
        }
    }
    let n = small.len();
    let nth = |k: usize| -> usize {
        let mut seen = 0;
        for (g, &c) in hist.iter().enumerate() {
            seen += c;
            if seen > k {
                return g;
            }
        }
        255
    };
    let median = ((nth((n - 1) / 2) + nth(n / 2)) / 2) as u8;
    let (ox, oy) = ((iw - dw) / 2, (ih - dh) / 2);
    let mut canvas: Buf<u8> = Zeroizing::new(vec![median; iw * ih]);
    for y in 0..dh {
        if let (Some(src), Some(dst)) = (
            small.get(y * dw..(y + 1) * dw),
            canvas.get_mut((oy + y) * iw + ox..(oy + y) * iw + ox + dw),
        ) {
            dst.copy_from_slice(src);
        }
    }
    (
        canvas,
        Placement {
            ox,
            oy,
            dw,
            dh,
            w,
            h,
        },
    )
}

/// A box in map pixels with the height the network gave it.
#[derive(Clone, Copy)]
pub(crate) struct MapBox {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    /// The height the network gave the word (the box may be higher where the
    /// kernel is).
    h: f32,
    /// The rows the kernels reach, top and bottom.
    k0: f32,
    k1: f32,
}

/// For each box, whether its kernels reach over more of its height than one
/// line's do (see `TWO_LINES_KERNEL`): it may hold two lines, a kernel merged
/// over both or two kernels of one box joined.
pub(crate) fn two_line_flags(boxes: &[MapBox]) -> Vec<bool> {
    boxes
        .iter()
        .map(|b| {
            let reach = b.k1 - b.k0;
            reach > TWO_LINES_KERNEL * b.h
        })
        .collect()
}

/// The 8-connected components of `channel > 0` with at least
/// `MIN_KERNEL_PIXELS` pixels, as boxes; `None` if there are more than
/// `MAX_KERNELS`.
pub(crate) fn kernels(maps: &Maps, channel: usize) -> Option<Vec<MapBox>> {
    let (rows, cols) = (Maps::ROWS, Maps::COLUMNS);
    let kernel = maps.channel(channel);
    let height = maps.channel(7);
    let mut seen = vec![false; rows * cols];
    let mut out = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut heights: Vec<f32> = Vec::new();
    for start in 0..rows * cols {
        if seen[start]
            || kernel
                .get(start)
                .is_none_or(|&v| v.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater))
        {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        heights.clear();
        let (mut x0, mut x1, mut y0, mut y1) = (cols, 0usize, rows, 0usize);
        let mut sum_y = 0.0f32;
        while let Some(p) = stack.pop() {
            let (y, x) = (p / cols, p % cols);
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
            sum_y += y as f32 + 0.5;
            heights.push(height.get(p).copied().unwrap_or(0.0));
            for ny in y.saturating_sub(1)..=(y + 1).min(rows - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(cols - 1) {
                    let q = ny * cols + nx;
                    if !seen[q] && kernel.get(q).is_some_and(|&v| v > 0.0) {
                        seen[q] = true;
                        stack.push(q);
                    }
                }
            }
        }
        if heights.len() < MIN_KERNEL_PIXELS {
            continue;
        }
        if out.len() >= MAX_KERNELS {
            return None;
        }
        heights.sort_by(f32::total_cmp);
        let n = heights.len();
        let med = (heights[(n - 1) / 2] + heights[n / 2]) / 2.0;
        let h = (med * HEIGHT_SCALE).max(1.0);
        let c = sum_y / n as f32;
        out.push(MapBox {
            x0: x0 as f32 - END_EXTENSION * h,
            x1: x1 as f32 + 1.0 + END_EXTENSION * h,
            y0: (c - h / 2.0).min(y0 as f32),
            y1: (c + h / 2.0).max(y1 as f32 + 1.0),
            h,
            k0: y0 as f32,
            k1: y1 as f32 + 1.0,
        });
    }
    Some(out)
}

/// Joins boxes that are pieces of one word: they overlap along the line and
/// stand on the same line. Nothing is split.
pub(crate) fn join(boxes: Vec<MapBox>) -> Vec<MapBox> {
    let n = boxes.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (boxes[i], boxes[j]);
            let along = a.x0 <= b.x1 && b.x0 <= a.x1;
            let low = (a.y1 - a.y0).min(b.y1 - b.y0);
            let across = a.y1.min(b.y1) - a.y0.max(b.y0);
            if along && across >= SAME_LINE * low {
                let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                parent[rj.max(ri)] = rj.min(ri);
            }
        }
    }
    let mut merged: Vec<Option<MapBox>> = vec![None; n];
    for (i, &b) in boxes.iter().enumerate() {
        let r = root(&mut parent, i);
        merged[r] = Some(match merged[r] {
            None => b,
            Some(m) => MapBox {
                x0: m.x0.min(b.x0),
                y0: m.y0.min(b.y0),
                x1: m.x1.max(b.x1),
                y1: m.y1.max(b.y1),
                h: m.h.max(b.h),
                k0: m.k0.min(b.k0),
                k1: m.k1.max(b.k1),
            },
        });
    }
    merged.into_iter().flatten().collect()
}

/// One detection on a frame (or on a region of it, `gray` being that region
/// only), boxes in the region's pixels.
fn pass(w: usize, h: usize, gray: &[u8]) -> Result<Found, Error> {
    let (input, place) = scale_to_input(w, h, gray);
    let maps = detector_maps(&input)?;
    drop(input);
    let (Some(words), Some(numbers)) = (kernels(&maps, 0), kernels(&maps, 1)) else {
        return Ok(Found {
            too_many: true,
            ..Found::default()
        });
    };
    let frame = |b: MapBox| {
        let (x0, y0) = place.to_frame(b.x0, b.y0);
        let (x1, y1) = place.to_frame(b.x1, b.y1);
        Rect { x0, y0, x1, y1 }
    };
    let words = join(words);
    let tall = two_line_flags(&words);
    Ok(Found {
        words: words.into_iter().map(frame).collect(),
        numbers: join(numbers).into_iter().map(frame).collect(),
        too_many: false,
        tall,
    })
}

/// The box around all the boxes, if any.
fn around(found: &Found) -> Option<Rect> {
    found
        .words
        .iter()
        .chain(found.numbers.iter())
        .copied()
        .reduce(|a, b| a.union(&b))
}

/// Costs two runs of the detector and a pass over the frame's pixels, and
/// finds the words and the line numbers on a frame (`w` by `h`, row-major,
/// 0 black to 255 white; a photo is read upright, as it is given).
///
/// The frame is scaled to the detector's input (aspect kept, padded with its
/// median grey). If the words and numbers found stand in a region covering
/// less than half of the frame, that region (with a margin) is scaled and
/// detected again. Word and number kernels are the 8-connected components of
/// the detector's maps above 0 with at least 2 pixels; a box is as high as the
/// median of the height map over its kernel times 8, each end extended by a
/// quarter of that height, and boxes of one line that overlap are joined.
/// Boxes are axis-aligned: the page is read as it stands, upright.
///
/// The second detection stands only if it keeps what the first found: every
/// word box of the first overlaps a box of the second, which has at least as
/// many words and numbers, and none of its boxes touches the region's border
/// where that is not the frame's (a word there is cut short). Otherwise the
/// first stands.
///
/// A frame of any size, flat, noise or all dark, never panics. One that holds
/// more than `MAX_KERNELS` kernels of a kind answers `Found::too_many` and no
/// boxes; a frame with a side over `MAX_SIDE` is `Error::Size`.
pub fn find_words(w: usize, h: usize, gray: &[u8]) -> Result<Found, Error> {
    if w.checked_mul(h) != Some(gray.len()) {
        return Err(Error::Size);
    }
    if gray.is_empty() {
        return Ok(Found::default());
    }
    if w > MAX_SIDE || h > MAX_SIDE {
        return Err(Error::Size);
    }
    let first = pass(w, h, gray)?;
    let Some(r) = around(&first).filter(|_| !first.too_many) else {
        return Ok(first);
    };
    let (mw, mh) = (r.width() * ZOOM_MARGIN, r.height() * ZOOM_MARGIN);
    let x0 = (r.x0 - mw).max(0.0) as usize;
    let y0 = (r.y0 - mh).max(0.0) as usize;
    let x1 = ((r.x1 + mw).max(0.0) as usize + 1).min(w);
    let y1 = ((r.y1 + mh).max(0.0) as usize + 1).min(h);
    if x1 <= x0 + 8 || y1 <= y0 + 8 {
        return Ok(first);
    }
    let (rw, rh) = (x1 - x0, y1 - y0);
    if (rw as f32) * (rh as f32) >= ZOOM_SHARE * (w as f32) * (h as f32) {
        return Ok(first);
    }
    let mut part: Buf<u8> = Zeroizing::new(Vec::with_capacity(rw * rh));
    for y in y0..y1 {
        if let Some(row) = gray.get(y * w + x0..y * w + x1) {
            part.extend_from_slice(row);
        }
    }
    let mut second = pass(rw, rh, &part)?;
    let (dx, dy) = (x0 as f32, y0 as f32);
    for b in second.words.iter_mut().chain(second.numbers.iter_mut()) {
        b.x0 += dx;
        b.x1 += dx;
        b.y0 += dy;
        b.y1 += dy;
    }
    if second_stands(&first, &second, (x0, y0, x1, y1), (w, h)) {
        Ok(second)
    } else {
        Ok(first)
    }
}

/// The second detection (boxes in the frame's pixels, on the region
/// `x0, y0, x1, y1` of a frame of `w` by `h`) stands in place of the first
/// only if it has no more than `MAX_KERNELS` kernels, as many words and as
/// many numbers, every word box of the first overlaps one of its boxes, and
/// none of its boxes touches a border of the region that is not the frame's
/// (a word there is cut short).
pub(crate) fn second_stands(
    first: &Found,
    second: &Found,
    (x0, y0, x1, y1): (usize, usize, usize, usize),
    (w, h): (usize, usize),
) -> bool {
    if second.too_many
        || second.words.len() < first.words.len()
        || second.numbers.len() < first.numbers.len()
    {
        return false;
    }
    let (rx0, ry0, rx1, ry1) = (x0 as f32, y0 as f32, x1 as f32, y1 as f32);
    let cut = second.words.iter().chain(second.numbers.iter()).any(|b| {
        (x0 > 0 && b.x0 <= rx0)
            || (y0 > 0 && b.y0 <= ry0)
            || (x1 < w && b.x1 >= rx1)
            || (y1 < h && b.y1 >= ry1)
    });
    !cut && first
        .words
        .iter()
        .all(|a| second.words.iter().any(|b| a.overlaps(b)))
}
