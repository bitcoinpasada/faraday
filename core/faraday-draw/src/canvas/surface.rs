//! Rows of pixels a painting call writes: the whole canvas, or a band of
//! its rows (`docs/FRAMES.md` §4.2). The painting that is the same for
//! both lives here, so a band paints the bytes the canvas paints.

use alloc::vec::Vec;

use tiny_skia::{FillRule, Paint, PixmapMut, PremultipliedColorU8, Stroke, Transform};

use super::{div255, outside, over, skia_rect, yuv_to_rgb};
use crate::color::Color;
use crate::fonts::Coverage;
use crate::geom::Rect;

/// A run of whole rows of the canvas, `y0` the canvas row of the first.
/// Coordinates a call is given are the canvas's; a surface turns them
/// into its own rows.
pub(crate) struct Surface<'a> {
    pm: PixmapMut<'a>,
    y0: i32,
}

impl<'a> Surface<'a> {
    pub(crate) fn new(pm: PixmapMut<'a>, y0: i32) -> Self {
        Surface { pm, y0 }
    }

    /// The pixels the surface holds, in the canvas's coordinates.
    pub(crate) fn rect(&self) -> Rect {
        Rect::new(0, self.y0, self.pm.width() as i32, self.pm.height() as i32)
    }

    /// Canvas row `y`'s bytes, the canvas's whole width.
    pub(crate) fn row_mut(&mut self, y: i32) -> &mut [u8] {
        let stride = self.pm.width() as usize * 4;
        let at = (y - self.y0) as usize * stride;
        &mut self.pm.data_mut()[at..at + stride]
    }
}

/// Fills `rect` inside `clip` and the surface in `color`.
pub(crate) fn fill_rect(s: &mut Surface<'_>, rect: Rect, clip: Rect, color: Color) {
    let r = rect.intersect(&clip).intersect(&s.rect());
    if r.is_empty() || color.a == 0 {
        return;
    }
    if color.a < 255 {
        // Blended here, a whole pixel at a time: tiny-skia's blend of
        // a translucent fill took twenty milliseconds over a 1080p
        // panel, and a sheet's dim covers the whole of it every frame.
        let a = u32::from(color.a);
        let src = u32::from_le_bytes([
            div255(u32::from(color.r) * a) as u8,
            div255(u32::from(color.g) * a) as u8,
            div255(u32::from(color.b) * a) as u8,
            color.a,
        ]);
        for y in r.y..r.bottom() {
            let row = &mut s.row_mut(y)[r.x as usize * 4..r.right() as usize * 4];
            for p in row.chunks_exact_mut(4) {
                let d = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
                p.copy_from_slice(&over(src, d, 255 - a).to_le_bytes());
            }
        }
        return;
    }
    let mut paint = Paint::default();
    paint.set_color(color.to_skia());
    paint.anti_alias = false;
    let y0 = s.y0;
    let local = Rect::new(r.x, r.y - y0, r.w, r.h);
    s.pm.fill_rect(skia_rect(local), &paint, Transform::identity(), None);
}

/// Costs a write of each dark pixel. Fills the cells `dark` holds of an
/// `n` by `n` grid of squares `cell` pixels wide, its top-left at `at` (`Canvas::fill_cells`), inside
/// `clip`. Returns the box it may have written.
pub(crate) fn fill_cells(
    s: &mut Surface<'_>,
    clip: Rect,
    (at, n, cell): ((i32, i32), usize, i32),
    dark: impl Fn(usize, usize) -> bool,
    color: Color,
) -> Rect {
    let side = n as i32 * cell;
    let area = Rect::new(at.0, at.1, side, side)
        .intersect(&clip)
        .intersect(&s.rect());
    if area.is_empty() || cell <= 0 || color.a == 0 {
        return Rect::default();
    }
    // Each row's runs of dark cells, as columns of pixels in the area.
    let runs_of = |row: usize, runs: &mut Vec<(usize, usize)>| {
        runs.clear();
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
            let l = (at.0 + start as i32 * cell).max(area.x);
            let r = (at.0 + col as i32 * cell).min(area.right());
            if l < r {
                runs.push((l as usize, r as usize));
            }
        }
    };
    let mut runs = Vec::new();
    if color.a < 255 {
        for row in 0..n {
            runs_of(row, &mut runs);
            let top = at.1 + row as i32 * cell;
            for &(l, r) in &runs {
                fill_rect(
                    s,
                    Rect::new(l as i32, top, (r - l) as i32, cell),
                    clip,
                    color,
                );
            }
        }
        return area;
    }
    let px = [color.r, color.g, color.b, 255];
    for row in 0..n {
        let top = (at.1 + row as i32 * cell).max(area.y);
        let bottom = (at.1 + (row as i32 + 1) * cell).min(area.bottom());
        if top >= bottom {
            continue;
        }
        runs_of(row, &mut runs);
        for y in top..bottom {
            let line = s.row_mut(y);
            for &(l, r) in &runs {
                for p in line[l * 4..r * 4].chunks_exact_mut(4) {
                    p.copy_from_slice(&px);
                }
            }
        }
    }
    area
}

/// What a shadow is worked out from (`Canvas::shadow`).
#[derive(Clone, Copy)]
pub(crate) struct ShadowOf {
    pub rect: Rect,
    pub radius: f32,
    pub blur: f32,
    pub spread: f32,
    pub dy: i32,
}

impl ShadowOf {
    /// The pixels the shadow may reach, before any clip.
    pub(crate) fn reach(&self) -> (Rect, Rect, f32) {
        let ShadowOf {
            rect,
            blur,
            spread,
            dy,
            ..
        } = *self;
        // The shape whose edge the fall starts from: the spread shape
        // drawn in by half the blur, so the middle of the transition is
        // on the spread shape's edge.
        let grow = spread - blur / 2.0;
        let g = (grow + 0.5 * if grow < 0.0 { -1.0 } else { 1.0 }) as i32;
        let fall_len = blur * 1.2;
        let shape = Rect::new(
            rect.x - g,
            rect.y + dy - g,
            (rect.w + 2 * g).max(0),
            (rect.h + 2 * g).max(0),
        );
        let reach = fall_len as i32 + 1;
        let area = Rect::new(
            shape.x - reach,
            shape.y - reach,
            shape.w + 2 * reach,
            shape.h + 2 * reach,
        );
        (shape, area, grow)
    }
}

/// Costs a pass over the pixels the shadow reaches. Paints the shadow
/// `of` in `color` at its darkest inside `clip`
/// (`Canvas::shadow`). `fall` is room for the table of how dark it is at
/// each quarter pixel from the edge.
pub(crate) fn shadow(
    s: &mut Surface<'_>,
    clip: Rect,
    of: ShadowOf,
    color: Color,
    fall: &mut Vec<u32>,
) {
    let ShadowOf {
        rect, radius, blur, ..
    } = of;
    let (shape, area, grow) = of.reach();
    let area = area.intersect(&clip).intersect(&s.rect());
    if area.is_empty() {
        return;
    }
    let fall_len = blur * 1.2;
    // Covered by what floats, whatever its corners.
    let r = radius
        .max(0.0)
        .min(rect.w as f32 / 2.0)
        .min(rect.h as f32 / 2.0);
    let inset = r as i32 + 1;
    let covered = Rect::new(
        rect.x + inset,
        rect.y + inset,
        rect.w - 2 * inset,
        rect.h - 2 * inset,
    );
    let (cx, cy) = (
        shape.x as f32 + shape.w as f32 / 2.0,
        shape.y as f32 + shape.h as f32 / 2.0,
    );
    // The shape's own corners follow the growth.
    let sr = (r + grow)
        .max(0.0)
        .min(shape.w as f32 / 2.0)
        .min(shape.h as f32 / 2.0);
    let (hw, hh) = (shape.w as f32 / 2.0 - sr, shape.h as f32 / 2.0 - sr);
    let (cr, cg, cb, ca) = (
        u32::from(color.r),
        u32::from(color.g),
        u32::from(color.b),
        u32::from(color.a),
    );
    // How dark the shadow is at each quarter pixel from the edge:
    // smooth from full there to nothing at `blur`.
    let steps_per_px = 4.0;
    fall.clear();
    fall.extend((0..=(fall_len * steps_per_px) as usize + 1).map(|i| {
        let t = (i as f32 / steps_per_px / fall_len).min(1.0);
        (ca as f32 * (1.0 - t) * (1.0 - t) * (1.0 + 2.0 * t)) as u32
    }));
    for py in area.y..area.bottom() {
        // The rows the covered middle spans are only walked at their
        // two ends: the columns left of it, then those right of it.
        let (from, to) = if py >= covered.y && py < covered.bottom() && !covered.is_empty() {
            (
                covered.x.clamp(area.x, area.right()),
                covered.right().clamp(area.x, area.right()),
            )
        } else {
            (area.right(), area.right())
        };
        let data = s.row_mut(py);
        for px in (area.x..from).chain(to..area.right()) {
            // Distance outside the rounded rectangle: along a side
            // it is one coordinate's, round a corner both.
            let qx = ((px as f32 + 0.5 - cx).abs() - hw).max(0.0);
            let qy = ((py as f32 + 0.5 - cy).abs() - hh).max(0.0);
            let d = if qx == 0.0 {
                qy
            } else if qy == 0.0 {
                qx
            } else {
                super::sqrt(qx * qx + qy * qy)
            } - sr;
            let step = ((d.max(0.0) * steps_per_px) as usize).min(fall.len() - 1);
            let a = fall[step];
            if a == 0 {
                continue;
            }
            let inv = 255 - a;
            let i = px as usize * 4;
            let p = &mut data[i..i + 4];
            p[0] = (div255(cr * a) + div255(u32::from(p[0]) * inv)) as u8;
            p[1] = (div255(cg * a) + div255(u32::from(p[1]) * inv)) as u8;
            p[2] = (div255(cb * a) + div255(u32::from(p[2]) * inv)) as u8;
            p[3] = (a + div255(u32::from(p[3]) * inv)) as u8;
        }
    }
}

/// Costs a pass over the frame's pixels. A camera frame drawn into
/// `rect` inside `dst` (`Canvas::luma`, which
/// has already checked the sizes): `dst` is `rect` cut to the clip.
pub(crate) fn luma(
    s: &mut Surface<'_>,
    (rect, dst): (Rect, Rect),
    (sw, sh): (usize, usize),
    pixels: &[u8],
    chroma: Option<&[u8]>,
) {
    let dst = dst.intersect(&s.rect());
    if dst.is_empty() {
        return;
    }
    let cw = sw.div_ceil(2);
    let scale = (rect.w as f32 / sw as f32).max(rect.h as f32 / sh as f32);
    // The source pixel at the rectangle's top-left corner: the crop
    // takes the same amount off both sides of the longer axis.
    let left = (sw as f32 - rect.w as f32 / scale) / 2.0;
    let top = (sh as f32 - rect.h as f32 / scale) / 2.0;
    for y in dst.y..dst.bottom() {
        let sy = ((top + ((y - rect.y) as f32 + 0.5) / scale) as usize).min(sh - 1);
        let row = sy * sw;
        let uv_row = (sy / 2) * cw;
        let out = s.row_mut(y);
        for x in dst.x..dst.right() {
            let sx = ((left + ((x - rect.x) as f32 + 0.5) / scale) as usize).min(sw - 1);
            let luma = pixels[row + sx];
            let (r, g, b) = match chroma {
                Some(uv) => {
                    let at = (uv_row + sx / 2) * 2;
                    yuv_to_rgb(luma, uv[at], uv[at + 1])
                }
                None => (luma, luma, luma),
            };
            // Alpha 255, so the premultiplied bytes are the colour
            // itself.
            let i = x as usize * 4;
            let px = PremultipliedColorU8::from_rgba(r, g, b, 255).expect("opaque pixel");
            out[i..i + 4].copy_from_slice(&[px.red(), px.green(), px.blue(), px.alpha()]);
        }
    }
}

/// One glyph's coverage blended in `color` inside `clip`, its box at
/// (`x`, `y`) and `w` by `h`.
pub(crate) fn blit_glyph(
    s: &mut Surface<'_>,
    clip: Rect,
    (x, y, w, h): (i32, i32, usize, usize),
    bitmap: Coverage<'_>,
    color: Color,
) {
    if w == 0 || h == 0 || color.a == 0 {
        return;
    }
    let clip = clip
        .intersect(&Rect::new(x, y, w as i32, h as i32))
        .intersect(&s.rect());
    if clip.is_empty() {
        return;
    }
    let (cr, cg, cb, ca) = (
        u32::from(color.r),
        u32::from(color.g),
        u32::from(color.b),
        u32::from(color.a),
    );
    for py in clip.y..clip.bottom() {
        let row = (py - y) as usize;
        let line = s.row_mut(py);
        for px in clip.x..clip.right() {
            let cov = u32::from(bitmap.get(row * w + (px - x) as usize));
            if cov == 0 {
                continue;
            }
            // Source alpha after coverage, in 0..=255.
            let a = div255(ca * cov);
            let inv = 255 - a;
            let i = px as usize * 4;
            let d = &mut line[i..i + 4];
            d[0] = (div255(cr * a) + div255(u32::from(d[0]) * inv)) as u8;
            d[1] = (div255(cg * a) + div255(u32::from(d[1]) * inv)) as u8;
            d[2] = (div255(cb * a) + div255(u32::from(d[2]) * inv)) as u8;
            d[3] = (a + div255(u32::from(d[3]) * inv)) as u8;
        }
    }
}

/// The box of whole pixels a path's ink can reach: half a stroke's width
/// beyond the path, one more for the anti-aliased edge, and one for the
/// truncation to whole pixels.
pub(crate) fn ink_box(path: &tiny_skia::Path, stroke: Option<&Stroke>) -> Rect {
    let b = path.bounds();
    let grow = stroke.map_or(0.0, |s| s.width / 2.0) + 1.0;
    let (l, t) = ((b.left() - grow) as i32 - 1, (b.top() - grow) as i32 - 1);
    let (r, bo) = (
        (b.right() + grow) as i32 + 1,
        (b.bottom() + grow) as i32 + 1,
    );
    Rect::new(l, t, r - l, bo - t)
}

/// Costs the rasterizing of the path over its box. Fills `path`, or
/// strokes it with `stroke`, inside `clip`, in the
/// surface's rows, moved by `by` pixels as it is drawn; `visible` is the
/// canvas's last row that is shown.
///
/// No mask: tiny-skia blends through one about thirty times slower than
/// without. A shape across an edge of the clip is drawn whole and the
/// pixels of its box outside the clip are put back as they were: the same
/// pixels as the whole shape cut by the clip. A shape wholly outside the
/// clip is not drawn. `saved` is room for the pixels put aside.
pub(crate) fn path(
    s: &mut Surface<'_>,
    saved: &mut Vec<u8>,
    (clip, visible): (Rect, i32),
    (path, paint, stroke): (&tiny_skia::Path, &Paint, Option<&Stroke>),
    ink: Rect,
    by: Transform,
) {
    let (l, t, r, bo) = (ink.x, ink.y, ink.right(), ink.bottom());
    let clip = clip.intersect(&s.rect());
    // A shape wholly outside the clip reaches no pixel the clip
    // keeps: a card scrolled off the page costs its layout and
    // nothing more, however long the page.
    if clip.is_empty() || r <= clip.x || l >= clip.right() || t.max(clip.y) >= bo.min(clip.bottom())
    {
        return;
    }
    let width = s.pm.width() as i32;
    let (y0, y1) = (t.max(s.y0), bo.min(visible).min(s.rect().bottom()));
    let region = Rect::new(l.max(0), y0, r.min(width) - l.max(0), y1 - y0);
    let pieces = outside(region, clip);
    let span = |p: &Rect| p.x as usize * 4..p.right() as usize * 4;
    saved.clear();
    for p in &pieces {
        for y in p.y..p.bottom() {
            let row = s.row_mut(y);
            saved.extend_from_slice(&row[span(p)]);
        }
    }
    // Drawn on the whole shape, which the surface holds: a shape cut
    // where the clip crosses one of its curves would have other coverage
    // there than the whole shape has.
    match stroke {
        Some(st) => s.pm.stroke_path(path, paint, st, by, None),
        None => s.pm.fill_path(path, paint, FillRule::Winding, by, None),
    }
    let mut at = 0;
    for p in &pieces {
        for y in p.y..p.bottom() {
            let r = span(p);
            let n = r.len();
            s.row_mut(y)[r].copy_from_slice(&saved[at..at + n]);
            at += n;
        }
    }
}
