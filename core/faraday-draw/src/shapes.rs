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

use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, Transform};

use crate::color::Color;
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
}

impl Shapes {
    /// How many shapes have been filled from their outlines, counting
    /// from the canvas's making. A lookup.
    pub(crate) fn filled(&self) -> u64 {
        self.filled
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
        if self.bytes + kept.cover.len() > SHAPES_BUDGET || self.kept.len() >= SHAPES_MOST {
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
/// in `color` into `pixmap` inside `clip`, filling it from `outlines` on
/// a miss; returns the pixels it may have written.
pub(crate) fn draw(
    shapes: &mut Shapes,
    pixmap: &mut Pixmap,
    clip: Rect,
    (key, outlines): (ShapeKey, impl FnOnce() -> Vec<Vec<(f32, f32)>>),
    color: Color,
) -> Option<Rect> {
    let k = shapes.get(key, outlines)?;
    let area = Rect::new(k.x, k.y, k.w as i32, k.h as i32).intersect(&clip);
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
    let stride = pixmap.width() as usize * 4;
    let data = pixmap.data_mut();
    for y in area.y..area.bottom() {
        let row = &k.cover[(y - k.y) as usize * k.w..][..k.w];
        let at = y as usize * stride;
        for x in area.x..area.right() {
            let c = u16::from(row[(x - k.x) as usize]);
            if c == 0 {
                continue;
            }
            let i = at + x as usize * 4;
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

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

    use super::ShapeKey;
    use crate::{Canvas, Color, Rect};

    const W: u16 = 120;
    const H: u16 = 60;

    fn canvas() -> Canvas {
        let mut c = Canvas::new(&DisplayInfo {
            width: W,
            height: H,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        });
        backdrop(&mut c);
        c
    }

    /// Bands of colour under where the shapes go, some of them laid
    /// translucent, so a blend shows against more than one colour; the
    /// same pixels each time.
    fn backdrop(c: &mut Canvas) {
        c.clear(Color::rgb(250, 248, 240));
        for i in 0..10u8 {
            let band = Rect::new(i32::from(i) * 12, 0, 7, i32::from(H));
            c.fill_rect(
                band,
                Color::rgba(30 + i * 20, 200 - i * 15, 90, 120 + i * 13),
            );
        }
    }

    /// A ring with a star cut through it, at (x, y), `s` pixels to the
    /// unit: curves in straight pieces and a hole, as the name's letters
    /// and the mark are.
    fn shape(x: f32, y: f32, s: f32) -> Vec<Vec<(f32, f32)>> {
        let at = |a: f32, r: f32| (x + (20.0 + r * cos(a)) * s, y + (20.0 + r * sin(a)) * s);
        let tau = core::f32::consts::TAU;
        let outer = (0..48).map(|k| at(tau * k as f32 / 48.0, 18.3)).collect();
        let star = (0..10)
            .rev()
            .map(|k| at(tau * k as f32 / 10.0, if k % 2 == 0 { 11.7 } else { 4.4 }))
            .collect();
        vec![outer, star]
    }

    /// Cosine by its series: the crate has no float library.
    fn cos(a: f32) -> f32 {
        sin(a + core::f32::consts::FRAC_PI_2)
    }

    /// Sine by its series, `a` brought into -π..π first.
    fn sin(a: f32) -> f32 {
        let tau = core::f32::consts::TAU;
        let mut x = a % tau;
        if x > core::f32::consts::PI {
            x -= tau;
        }
        let (mut term, mut sum) = (x, x);
        for n in 1..12 {
            term *= -x * x / ((2 * n) as f32 * (2 * n + 1) as f32);
            sum += term;
        }
        sum
    }

    /// The pixels `colour` fills the shape at `at` with, from its
    /// outlines, under `clip`.
    fn filled(at: (f32, f32, f32), clip: Rect, colour: Color) -> Vec<u8> {
        let mut c = canvas();
        c.push_clip(clip);
        let outlines = shape(at.0, at.1, at.2);
        let contours: Vec<&[(f32, f32)]> = outlines.iter().map(Vec::as_slice).collect();
        c.fill_contours(&contours, colour);
        c.frame().rgba.to_vec()
    }

    /// Draws the shape at `at` from what `c` keeps, over the backdrop.
    fn kept(c: &mut Canvas, at: (f32, f32, f32), clip: Rect, colour: Color) -> Vec<u8> {
        backdrop(c);
        c.push_clip(clip);
        let key = ShapeKey::new(7, at.0, at.1, at.2);
        c.fill_kept(key, || shape(at.0, at.1, at.2), colour);
        c.pop_clip();
        c.frame().rgba.to_vec()
    }

    /// The name and the mark are drawn from what the canvas kept as they
    /// are from their outlines: at whole and part pixels, at several
    /// sizes, opaque and translucent, cut by a clip or not, the first time
    /// and every time after, in a new colour too, and filled once.
    #[test]
    fn a_kept_shape_is_drawn_as_its_outlines_are_filled() {
        let colours = [
            Color::rgb(20, 30, 40),
            Color::rgba(200, 40, 90, 140),
            Color::rgb(250, 250, 250),
            Color::rgba(0, 0, 0, 31),
            Color::rgb(103, 80, 164),
        ];
        let clips = [
            Rect::new(0, 0, i32::from(W), i32::from(H)),
            Rect::new(23, 9, 41, 30),
        ];
        let places = [
            (5.0, 3.0, 1.0),
            (17.37, 9.81, 0.6),
            (40.5, 2.25, 1.37),
            (71.06, 1.5, 1.21),
        ];
        for at in places {
            for clip in clips {
                let mut c = canvas();
                for (i, &colour) in colours.iter().enumerate() {
                    assert!(
                        kept(&mut c, at, clip, colour) == filled(at, clip, colour),
                        "{at:?} {clip:?} colour {i}: kept and filled differ"
                    );
                }
                assert_eq!(c.outlines_filled(), 1, "{at:?} {clip:?}: filled once");
            }
        }
    }

    /// What is kept follows what it is drawn from: a shape moved by part
    /// of a pixel, or drawn at another size, is filled anew and drawn as
    /// its outlines are; the same place and size again fills nothing.
    #[test]
    fn a_kept_shape_moved_or_resized_is_filled_again() {
        let colour = Color::rgb(60, 70, 200);
        let clip = Rect::new(0, 0, i32::from(W), i32::from(H));
        let mut c = canvas();
        let draws = [
            ((10.0, 5.0, 1.0), 1),
            ((10.25, 5.0, 1.0), 2),
            ((10.25, 5.0, 1.1), 3),
            ((10.0, 5.0, 1.0), 3),
            ((10.25, 5.0, 1.1), 3),
        ];
        for (n, (at, fills)) in draws.into_iter().enumerate() {
            assert!(
                kept(&mut c, at, clip, colour) == filled(at, clip, colour),
                "draw {n}: {at:?}"
            );
            assert_eq!(c.outlines_filled(), fills, "draw {n}: {at:?}");
        }
    }

    /// A QR code's rows of modules written straight into the canvas are
    /// the pixels of a rectangle filled for each run of dark modules,
    /// cut by a clip or not.
    #[test]
    fn cells_written_straight_are_the_runs_filled() {
        let n = 21;
        let dark = |col: usize, row: usize| (col * 7 + row * 3 + col * row) % 5 < 2;
        let clips = [
            Rect::new(0, 0, i32::from(W), i32::from(H)),
            Rect::new(13, 7, 50, 31),
        ];
        for clip in clips {
            for colour in [Color::BLACK, Color::rgba(10, 20, 30, 100)] {
                let mut by_rows = canvas();
                by_rows.push_clip(clip);
                by_rows.fill_cells((9, 4), (n, 2), dark, colour);
                let mut by_runs = canvas();
                by_runs.push_clip(clip);
                for row in 0..n {
                    let mut col = 0;
                    while col < n {
                        if !dark(col, row) {
                            col += 1;
                            continue;
                        }
                        let start = col;
                        while col < n && dark(col, row) {
                            col += 1;
                        }
                        let r = Rect::new(
                            9 + 2 * start as i32,
                            4 + 2 * row as i32,
                            2 * (col - start) as i32,
                            2,
                        );
                        by_runs.fill_rect(r, colour);
                    }
                }
                assert!(
                    by_rows.frame().rgba == by_runs.frame().rgba,
                    "{clip:?} {colour:?}"
                );
            }
        }
    }
}
