//! Shapes filled from their outlines once and kept: an app's name in its
//! lettering and its mark, drawn on every frame of most screens
//! (`docs/SPEED.md` §3.8). A [`Canvas`](crate::Canvas) keeps each one's
//! coverage, a byte a pixel, as it keeps glyphs, and blends it in the
//! colour asked for at each draw.
//!
//! The pixels are the ones [`Canvas::fill_contours`](crate::Canvas::fill_contours)
//! draws: the coverage is tiny-skia's own, filled with the outlines moved
//! by whole pixels only, and the blend is the one tiny-skia's pipeline
//! does for a solid fill, opaque or not.

use alloc::vec::Vec;

use tiny_skia::{FillRule, Mask, PathBuilder, Transform};

use crate::canvas::Surface;
use faraday_tokens::palette::Color;

use crate::skia::ToSkia;
use crate::geom::Rect;

/// Everything a kept shape's outlines are worked out from: which shape,
/// where its origin falls in pixels, and its scale, pixels to the unit
/// of its outlines. Two draws under the same key fill the same outlines,
/// so a key is the whole of what is kept in step: a move, a new size or a
/// new display scale each make a new key, and the colour is
/// not kept at all, so a change of theme needs nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeKey([u32; 4]);

impl ShapeKey {
    /// The key of shape `shape` (the caller's own number for it) with its
    /// origin at (`x`, `y`) pixels, `scale` pixels to its unit. Cheap.
    pub fn new(shape: u32, x: f32, y: f32, scale: f32) -> ShapeKey {
        ShapeKey([shape, x.to_bits(), y.to_bits(), scale.to_bits()])
    }
}

/// How many bytes of kept coverage before the shapes are dropped whole.
/// A screen draws two or three (the name, its word, the mark), some
/// tens of kilobytes at the largest scale; a megabyte holds those at
/// many places and sizes, and overflowing it costs one frame's fills.
/// It is a memory budget, not a screen dimension, so it is not a design
/// token.
const SHAPES_BUDGET: usize = 1024 * 1024;

/// The most shapes kept: a lookup reads them in turn.
const SHAPES_MOST: usize = 32;

/// One kept shape: its key, the pixel its coverage's top-left is at, and
/// the coverage, `w` by `h` bytes.
#[derive(Debug)]
struct Kept {
    key: ShapeKey,
    x: i32,
    y: i32,
    w: usize,
    h: usize,
    cover: Vec<u8>,
}

/// The kept shapes of one canvas.
///
/// Invariant: the coverage under a key is tiny-skia's fill of the
/// outlines that key names, as they were when it was kept. The outlines
/// are a function of the key alone (the caller's promise in
/// [`ShapeKey`]), so what is kept never goes stale; a canvas made anew
/// (a new display) starts with none.
#[derive(Debug, Default)]
pub(crate) struct Shapes {
    kept: Vec<Kept>,
    bytes: usize,
    /// Shapes filled from their outlines, ever: a miss each.
    filled: u64,
    /// Whether nothing is dropped, whatever the budget, while calls are
    /// recorded for painting later ([`Shapes::hold`]).
    held: bool,
}

impl Shapes {
    /// How many shapes have been filled from their outlines, counting
    /// from the canvas's making. A lookup.
    pub(crate) fn filled(&self) -> u64 {
        self.filled
    }

    /// Drops nothing from now on, whatever the budget, until
    /// [`Shapes::release`]: every shape a list of recorded calls will
    /// paint is there when it is painted.
    pub(crate) fn hold(&mut self) {
        self.held = true;
    }

    /// Drops shapes at the budget again, and drops them now if more are
    /// kept than the budget holds. Call it with no recorded call waiting.
    pub(crate) fn release(&mut self) {
        self.held = false;
        if self.bytes > SHAPES_BUDGET || self.kept.len() > SHAPES_MOST {
            self.kept.clear();
            self.bytes = 0;
        }
    }

    /// The box of pixels the shape under `key` covers, if it is kept. A
    /// lookup.
    pub(crate) fn area(&self, key: ShapeKey) -> Option<Rect> {
        let k = self.kept.iter().find(|k| k.key == key)?;
        Some(Rect::new(k.x, k.y, k.w as i32, k.h as i32))
    }

    /// Costs filling the outlines on a miss; a lookup on a hit. Has the
    /// shape under `key` kept, as a draw would, without drawing it.
    pub(crate) fn keep(&mut self, key: ShapeKey, outlines: impl FnOnce() -> Vec<Vec<(f32, f32)>>) {
        let _ = self.get(key, outlines);
    }

    /// Costs filling the outlines on a miss; a lookup on a hit. The
    /// shape kept under `key`, filled from `outlines` (closed outlines in
    /// pixels) and kept when it is not there.
    fn get(
        &mut self,
        key: ShapeKey,
        outlines: impl FnOnce() -> Vec<Vec<(f32, f32)>>,
    ) -> Option<&Kept> {
        if let Some(i) = self.kept.iter().position(|k| k.key == key) {
            return self.kept.get(i);
        }
        self.filled += 1;
        let kept = fill(key, &outlines())?;
        if !self.held
            && (self.bytes + kept.cover.len() > SHAPES_BUDGET || self.kept.len() >= SHAPES_MOST)
        {
            self.kept.clear();
            self.bytes = 0;
        }
        self.bytes += kept.cover.len();
        self.kept.push(kept);
        self.kept.last()
    }
}

/// Costs filling the outlines. Their coverage, in a box of whole pixels
/// one pixel wider than them on every side, the outlines moved by whole
/// pixels into it: a whole-pixel move is exact in floating point and in
/// tiny-skia's fixed point, so the coverage is what a fill in place has.
fn fill(key: ShapeKey, outlines: &[Vec<(f32, f32)>]) -> Option<Kept> {
    let points = || outlines.iter().flatten();
    let (mut l, mut t, mut r, mut b) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in points() {
        (l, t, r, b) = (l.min(x), t.min(y), r.max(x), b.max(y));
    }
    if !(l.is_finite() && t.is_finite() && r.is_finite() && b.is_finite()) {
        return None;
    }
    let (x0, y0) = (floor(l) - 1, floor(t) - 1);
    let (w, h) = (ceil(r) + 1 - x0, ceil(b) + 1 - y0);
    let mut pb = PathBuilder::new();
    for o in outlines {
        let Some((&(fx, fy), rest)) = o.split_first() else {
            continue;
        };
        pb.move_to(fx - x0 as f32, fy - y0 as f32);
        for &(px, py) in rest {
            pb.line_to(px - x0 as f32, py - y0 as f32);
        }
        pb.close();
    }
    let path = pb.finish()?;
    let mut mask = Mask::new(u32::try_from(w).ok()?, u32::try_from(h).ok()?)?;
    mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
    Some(Kept {
        key,
        x: x0,
        y: y0,
        w: w as usize,
        h: h as usize,
        cover: mask.data().to_vec(),
    })
}

fn floor(v: f32) -> i32 {
    let i = v as i32;
    if (i as f32) > v { i - 1 } else { i }
}

fn ceil(v: f32) -> i32 {
    let i = v as i32;
    if (i as f32) < v { i + 1 } else { i }
}

/// Costs a pass over the shape's box. Blends the shape kept under `key`
/// in `color` into `surface` inside `clip`; returns the pixels it may
/// have written, `None` when the shape is not kept or none of it is
/// inside the clip. It only reads the shapes.
pub(crate) fn draw(
    shapes: &Shapes,
    surface: &mut Surface<'_>,
    clip: Rect,
    key: ShapeKey,
    color: Color,
) -> Option<Rect> {
    let k = shapes.kept.iter().find(|k| k.key == key)?;
    let area = Rect::new(k.x, k.y, k.w as i32, k.h as i32)
        .intersect(&clip)
        .intersect(&surface.rect());
    if area.is_empty() {
        return None;
    }
    // The colour as tiny-skia's pipeline holds it: premultiplied, each
    // byte rounded from its float.
    let p = color.to_skia().premultiply();
    let byte = |v: f32| (v * 255.0 + 0.5) as u16;
    let s = [
        byte(p.red()),
        byte(p.green()),
        byte(p.blue()),
        byte(p.alpha()),
    ];
    let opaque = color.a == 255;
    for y in area.y..area.bottom() {
        let row = &k.cover[(y - k.y) as usize * k.w..][..k.w];
        let data = surface.row_mut(y);
        for x in area.x..area.right() {
            let c = u16::from(row[(x - k.x) as usize]);
            if c == 0 {
                continue;
            }
            let i = x as usize * 4;
            let d = &mut data[i..i + 4];
            if opaque {
                // tiny-skia's opaque fill: Source, the coverage lerped.
                for (d, s) in d.iter_mut().zip(s) {
                    *d = div255(u16::from(*d) * (255 - c) + s * c) as u8;
                }
            } else {
                // Source over, the source scaled by the coverage first.
                let sc = s.map(|v| div255(v * c));
                let inv = 255 - sc[3];
                for (d, s) in d.iter_mut().zip(sc) {
                    *d = (s + div255(u16::from(*d) * inv)) as u8;
                }
            }
        }
    }
    Some(area)
}

/// tiny-skia's low-precision division by 255.
fn div255(v: u16) -> u16 {
    (v + 255) >> 8
}

