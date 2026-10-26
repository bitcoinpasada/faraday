//! The framebuffer and drawing primitives.
//!
//! [`Canvas`] wraps a `tiny_skia::Pixmap` (premultiplied RGBA8888) sized
//! from the shell's [`DisplayInfo`]. Shapes go through tiny-skia; glyph
//! outlines are filled by tiny-skia into coverage bitmaps the canvas
//! caches and blends by hand. A rectangular clip stack serves scroll
//! regions.

use alloc::string::String;
use alloc::vec::Vec;

use faraday_crypto::work::Work;
use faraday_shell_api::{DisplayInfo, Frame};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, PremultipliedColorU8, Stroke, Transform};

use crate::color::Color;
use crate::fonts::{Coverage, GlyphCache, SizedFace};
use crate::geom::{Rect, Scale, SizeClass};
use crate::text::{self, Font, TextAlign, TextMetrics};

/// A drawing surface the size of the display.
pub struct Canvas {
    pixmap: Pixmap,
    scale: Scale,
    clips: Vec<Rect>,
    /// Glyph bitmaps filled from the outlines, kept between frames.
    glyphs: GlyphCache,
    /// The pixels outside the clip put aside while a shape is drawn
    /// across the clip's edge.
    scratch: Vec<u8>,
    /// The strings drawn since the last [`Canvas::clear`] that ran past
    /// the side of their clip, so that some of their ink is not on the
    /// panel.
    cut: Vec<String>,
    /// Where each string and icon drawn since the last [`Canvas::clear`]
    /// left its ink, while [`Canvas::record_ink`] has it on: what the
    /// snapshot tool's audit reads. Off on a device.
    ink: Option<Vec<Ink>>,
    /// Lines of text drawn, ever: what a test compares to say a frame
    /// draws only what is seen ([`Canvas::lines_drawn`]).
    lines: u64,
    /// The pixels the canvas was last made the same as by
    /// [`Canvas::restore`], known by where they are held and how many;
    /// and the box of every pixel written since. Outside the box the
    /// canvas is those pixels still, so putting them back again copies
    /// the box alone: a sheet's frame puts back its kept page where the
    /// sheet was drawn over it, not the whole canvas.
    kept: Option<(usize, usize)>,
    damage: Option<Rect>,
    /// Notes the canvas's own costly work, a blur or a whole copy, for
    /// the application that budgets it ([`Canvas::set_counter`]); notes
    /// nothing until one is set.
    counter: fn(Work),
}

/// One thing a frame drew and the box its visible ink covers.
#[derive(Debug, Clone, PartialEq)]
pub struct Ink {
    /// What was drawn.
    pub kind: InkKind,
    /// The box of its ink inside the clip it was drawn in, in pixels.
    pub rect: Rect,
}

/// What an [`Ink`] is.
#[derive(Debug, Clone, PartialEq)]
pub enum InkKind {
    /// A line of text, and the face it was drawn in.
    Text {
        /// The characters.
        text: String,
        /// The face.
        family: crate::fonts::Family,
        /// The colour it was drawn in: what a test of its contrast
        /// against what is under it reads.
        color: Color,
    },
    /// An icon glyph.
    Icon(crate::widgets::Icon),
    /// A progress indicator: how much of its track is filled, 0 to 1, or
    /// `None` for work with no count.
    Progress(Option<f32>),
    /// A badge with no label: the dot on a destination.
    Dot,
    /// An app's name in its lettering: FARADAY, with the word of the app
    /// it heads beside it or under it.
    Wordmark,
}

/// The fewest rows between a rounded rectangle's corners worth filling
/// apart from them: below it, one path is as quick as three draws.
const SPLIT_ROWS: i32 = 64;

impl Canvas {
    /// Allocates a framebuffer for `display`. Panics on a zero-sized
    /// display; the shell must refuse those before reaching the core.
    pub fn new(display: &DisplayInfo) -> Self {
        Canvas::part(
            display.width,
            display.height,
            Scale::for_class(display.dpi, SizeClass::of(display)),
        )
    }

    /// Allocates a framebuffer `width` × `height` drawn at `scale`: a
    /// part of a display, at that display's own scale (Faraday Demo's
    /// strip). Panics on a zero size, as [`Canvas::new`] does.
    pub fn part(width: u16, height: u16, scale: Scale) -> Self {
        let pixmap =
            Pixmap::new(u32::from(width), u32::from(height)).expect("display has non-zero size");
        Canvas {
            pixmap,
            scale,
            clips: Vec::new(),
            glyphs: GlyphCache::new(),
            scratch: Vec::new(),
            cut: Vec::new(),
            ink: None,
            lines: 0,
            kept: None,
            damage: None,
            counter: |_| {},
        }
    }

    /// Has the canvas note its own costly work, a blur
    /// ([`Work::Blur`]) or a whole copy ([`Work::FullCanvas`]), with `f`:
    /// the pixel work a frame does, which takes no token
    /// (`docs/COMPUTE.md` §2.2). Cheap.
    pub fn set_counter(&mut self, f: fn(Work)) {
        self.counter = f;
    }

    /// Notes that the pixels of `r` may have been written.
    fn damaged(&mut self, r: Rect) {
        let r = r.intersect(&self.bounds());
        if r.is_empty() {
            return;
        }
        self.damage = Some(self.damage.map_or(r, |d| union(d, r)));
    }

    /// How many lines of text this canvas has drawn, counting from its
    /// making: a line wholly above or below the clip is measured, not
    /// drawn, and not counted. Two counts taken around a frame say how
    /// much text it put where it can be seen.
    pub fn lines_drawn(&self) -> u64 {
        self.lines
    }

    /// Width in pixels.
    pub fn width(&self) -> i32 {
        self.pixmap.width() as i32
    }

    /// Height in pixels.
    pub fn height(&self) -> i32 {
        self.pixmap.height() as i32
    }

    /// The whole canvas as a rectangle.
    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width(), self.height())
    }

    /// The dp scale of this display.
    pub fn scale(&self) -> Scale {
        self.scale
    }

    /// The pixels, for the shell.
    pub fn frame(&self) -> Frame<'_> {
        Frame {
            width: self.pixmap.width() as u16,
            height: self.pixmap.height() as u16,
            rgba: self.pixmap.data(),
        }
    }

    /// Copies `frame`, another canvas's pixels, in with its top-left
    /// corner at (`x`, `y`), cut at this canvas's edges: an app's frame
    /// drawn inside another's (the Starter's held desktop app). A copy
    /// of its pixels, counted as a whole copy ([`Work::FullCanvas`]).
    pub fn place(&mut self, x: i32, y: i32, frame: &Frame<'_>) {
        (self.counter)(Work::FullCanvas);
        let (fw, fh) = (i32::from(frame.width), i32::from(frame.height));
        let r = Rect::new(x, y, fw, fh).intersect(&self.bounds());
        if r.is_empty() || frame.rgba.len() < (fw * fh * 4) as usize {
            return;
        }
        let stride = self.pixmap.width() as usize * 4;
        let n = r.w as usize * 4;
        let data = self.pixmap.data_mut();
        for row in r.y..r.bottom() {
            let to = row as usize * stride + r.x as usize * 4;
            let from = ((row - y) * fw + (r.x - x)) as usize * 4;
            data[to..to + n].copy_from_slice(&frame.rgba[from..from + n]);
        }
        self.damaged(r);
    }

    /// Costs a pass over the frame's pixels. Copies `frame` in scaled
    /// down to `w` × `h` with its top-left corner at (`x`, `y`), each
    /// pixel the average of the frame's pixels it covers: a screen of
    /// another size shown in a smaller window (Faraday Demo's host).
    /// Where `w` and `h` are the frame's own, [`Canvas::place`]. Counted
    /// as a whole copy ([`Work::FullCanvas`]).
    pub fn place_scaled(&mut self, (x, y, w, h): (i32, i32, i32, i32), frame: &Frame<'_>) {
        let (fw, fh) = (usize::from(frame.width), usize::from(frame.height));
        if w as usize == fw && h as usize == fh {
            self.place(x, y, frame);
            return;
        }
        (self.counter)(Work::FullCanvas);
        let r = Rect::new(x, y, w, h).intersect(&self.bounds());
        if r.is_empty() || fw == 0 || fh == 0 || frame.rgba.len() < fw * fh * 4 {
            return;
        }
        // The frame's cells, `[from, to)`, that cell `i` of `n` covers out
        // of the frame's `m`: at least one.
        let span = |i: usize, n: usize, m: usize| {
            let from = (i * m / n).min(m - 1);
            (from, ((i + 1) * m / n).clamp(from + 1, m))
        };
        let cols: Vec<(usize, usize)> = (r.x..r.right())
            .map(|c| span((c - x) as usize, w as usize, fw))
            .collect();
        let stride = self.pixmap.width() as usize * 4;
        let data = self.pixmap.data_mut();
        for row in r.y..r.bottom() {
            let (r0, r1) = span((row - y) as usize, h as usize, fh);
            for (k, &(c0, c1)) in cols.iter().enumerate() {
                let mut sum = [0u32; 4];
                for fy in r0..r1 {
                    let line = &frame.rgba[(fy * fw + c0) * 4..(fy * fw + c1) * 4];
                    for p in line.chunks_exact(4) {
                        for (s, v) in sum.iter_mut().zip(p) {
                            *s += u32::from(*v);
                        }
                    }
                }
                let n = ((r1 - r0) * (c1 - c0)).max(1) as u32;
                let at = row as usize * stride + (r.x as usize + k) * 4;
                for (d, s) in data[at..at + 4].iter_mut().zip(sum) {
                    *d = (s / n) as u8;
                }
            }
        }
        self.damaged(r);
    }

    /// The current clip rectangle.
    pub fn clip(&self) -> Rect {
        self.clips.last().copied().unwrap_or_else(|| self.bounds())
    }

    /// Restricts drawing to `rect` (intersected with the current clip)
    /// until the matching [`Canvas::pop_clip`].
    pub fn push_clip(&mut self, rect: Rect) {
        let clip = self.clip().intersect(&rect);
        self.clips.push(clip);
    }

    /// Restores the previous clip.
    pub fn pop_clip(&mut self) {
        self.clips.pop();
    }

    /// Fills the whole canvas, ignoring the clip.
    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(color.to_skia());
        self.damaged(self.bounds());
        self.cut.clear();
        if let Some(ink) = &mut self.ink {
            ink.clear();
        }
    }

    /// Starts or stops recording where text and icons leave their ink
    /// ([`Canvas::ink`]).
    pub fn record_ink(&mut self, on: bool) {
        self.ink = on.then(Vec::new);
    }

    /// What was drawn since the last [`Canvas::clear`] and where, while
    /// recording is on; empty otherwise.
    pub fn ink(&self) -> &[Ink] {
        self.ink.as_deref().unwrap_or(&[])
    }

    /// The strings drawn since the last [`Canvas::clear`] whose ink ran
    /// past the left or right side of their clip: text a person sees cut
    /// off.
    pub fn cut(&self) -> &[String] {
        &self.cut
    }

    /// Notes a shape drawn as `kind` in `rect`, pixels, while recording
    /// is on: one a person reads as a whole, such as a progress
    /// indicator, which its fills alone do not say.
    pub fn note(&mut self, kind: InkKind, rect: Rect) {
        let clip = self.clip();
        self.note_ink(|| kind, Some(rect.intersect(&clip)));
    }

    fn note_ink(&mut self, kind: impl FnOnce() -> InkKind, rect: Option<Rect>) {
        if let (Some(ink), Some(rect)) = (&mut self.ink, rect)
            && !rect.is_empty()
        {
            ink.push(Ink { kind: kind(), rect });
        }
    }

    fn paint(color: Color) -> Paint<'static> {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;
        paint
    }

    /// Fills a rectangle.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let r = rect.intersect(&self.clip());
        if r.is_empty() || color.a == 0 {
            return;
        }
        self.damaged(r);
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
            let stride = self.pixmap.width() as usize * 4;
            let data = self.pixmap.data_mut();
            for y in r.y..r.bottom() {
                let start = y as usize * stride + r.x as usize * 4;
                let row = &mut data[start..start + r.w as usize * 4];
                for p in row.chunks_exact_mut(4) {
                    let d = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
                    p.copy_from_slice(&over(src, d, 255 - a).to_le_bytes());
                }
            }
            return;
        }
        let mut paint = Self::paint(color);
        paint.anti_alias = false;
        self.pixmap
            .fill_rect(skia_rect(r), &paint, Transform::identity(), None);
    }

    /// A soft shadow under a rounded rectangle that floats: `rect` with
    /// corners of `radius`, grown by `spread` on every side, moved down
    /// `dy`, blurred over `blur` pixels (the blur radius of CSS: half the
    /// transition lies inside the edge), in `color` at its darkest. The
    /// middle, which what floats covers, is left as it is. The clip is
    /// honoured.
    ///
    /// The shadow is worked out per pixel from its distance to the
    /// shape, so it needs no blurred image kept for each size, and a
    /// pixel costs the same whatever the blur; the blur only sets how
    /// many pixels it reaches.
    pub fn shadow(
        &mut self,
        rect: Rect,
        radius: f32,
        blur: f32,
        spread: f32,
        dy: i32,
        color: Color,
    ) {
        if rect.is_empty() || color.a == 0 || blur <= 0.0 {
            return;
        }
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
        )
        .intersect(&self.clip());
        if area.is_empty() {
            return;
        }
        self.damaged(area);
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
        let fall: Vec<u32> = (0..=(fall_len * steps_per_px) as usize + 1)
            .map(|i| {
                let t = (i as f32 / steps_per_px / fall_len).min(1.0);
                (ca as f32 * (1.0 - t) * (1.0 - t) * (1.0 + 2.0 * t)) as u32
            })
            .collect();
        let stride = self.pixmap.width() as usize * 4;
        let data = self.pixmap.data_mut();
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
                    sqrt(qx * qx + qy * qy)
                } - sr;
                let step = ((d.max(0.0) * steps_per_px) as usize).min(fall.len() - 1);
                let a = fall[step];
                if a == 0 {
                    continue;
                }
                let inv = 255 - a;
                let i = py as usize * stride + px as usize * 4;
                let p = &mut data[i..i + 4];
                p[0] = (div255(cr * a) + div255(u32::from(p[0]) * inv)) as u8;
                p[1] = (div255(cg * a) + div255(u32::from(p[1]) * inv)) as u8;
                p[2] = (div255(cb * a) + div255(u32::from(p[2]) * inv)) as u8;
                p[3] = (a + div255(u32::from(p[3]) * inv)) as u8;
            }
        }
    }

    /// Material 3's shadow of an elevation `level` (1 to 5) under a
    /// rounded rectangle that floats: a key layer and an ambient layer
    /// ([`crate::tokens::ELEVATION_KEY`], [`crate::tokens::ELEVATION_AMBIENT`]),
    /// each in black at its opacity, their dp sizes times `scale`. Costs
    /// two [`Canvas::shadow`] passes, each over a ring a few pixels wide.
    pub fn elevation(&mut self, rect: Rect, radius: f32, level: u8, scale: f32) {
        let i = usize::from(level.clamp(1, 5)) - 1;
        for (layer, opacity) in [
            (
                crate::tokens::ELEVATION_AMBIENT[i],
                crate::tokens::ELEVATION_AMBIENT_OPACITY,
            ),
            (
                crate::tokens::ELEVATION_KEY[i],
                crate::tokens::ELEVATION_KEY_OPACITY,
            ),
        ] {
            self.shadow(
                rect,
                radius,
                layer.blur * scale,
                layer.spread * scale,
                (layer.y * scale + 0.5) as i32,
                Color::BLACK.with_alpha(opacity),
            );
        }
    }

    /// Blurs the whole canvas heavily, as frosted glass shows what is
    /// behind it: reduced `scale` times on each side, box-blurred three
    /// times `radius` wide, laid under `tint` (translucent, or none at
    /// alpha 0), and enlarged again smoothly. Costs some milliseconds, so
    /// it is meant to be done once and kept ([`Canvas::snapshot`],
    /// [`Canvas::restore`]).
    pub fn frost(&mut self, scale: u32, radius: u32, tint: Color) {
        (self.counter)(Work::Blur);
        self.damaged(self.bounds());
        let (w, h) = (self.pixmap.width() as usize, self.pixmap.height() as usize);
        let s = scale.max(1) as usize;
        let (sw, sh) = (w.div_ceil(s), h.div_ceil(s));
        // Each small pixel the mean of the block it stands for, a block
        // of columns at a time.
        // On a dense screen (a block sixteen or more across) every other
        // pixel of every other row stands for its two by two: a block's
        // mean from a quarter of its pixels, which the blur after makes
        // the same to the eye, at a quarter of the reading.
        let step = if s >= 16 { 2 } else { 1 };
        let mut sums = alloc::vec![[0u32; 3]; sw * sh];
        let data = self.pixmap.data();
        for y in (0..h).step_by(step) {
            let line = &mut sums[(y / s) * sw..(y / s) * sw + sw];
            let row = &data[y * w * 4..(y + 1) * w * 4];
            for (cell, block) in line.iter_mut().zip(row.chunks(s * 4)) {
                let mut add = |p: &[u8]| {
                    cell[0] += u32::from(p[0]);
                    cell[1] += u32::from(p[1]);
                    cell[2] += u32::from(p[2]);
                };
                if step == 1 {
                    block.chunks_exact(4).for_each(&mut add);
                } else {
                    block.chunks_exact(4).step_by(step).for_each(&mut add);
                }
            }
        }
        let mut px: Vec<[u32; 3]> = sums
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let (cx, cy) = (i % sw, i / sw);
                let n = ((w - cx * s).min(s).div_ceil(step) * (h - cy * s).min(s).div_ceil(step))
                    as u32;
                [p[0] / n, p[1] / n, p[2] / n]
            })
            .collect();
        let r = radius as usize;
        for _ in 0..3 {
            px = box_blur(&px, sw, sh, r, true);
            px = box_blur(&px, sw, sh, r, false);
        }
        // The tint goes over the small picture, where it is cheap.
        let a = u32::from(tint.a);
        let tint = u32::from_le_bytes([
            div255(u32::from(tint.r) * a) as u8,
            div255(u32::from(tint.g) * a) as u8,
            div255(u32::from(tint.b) * a) as u8,
            tint.a,
        ]);
        let packed: Vec<u32> = px
            .iter()
            .map(|p| {
                let p = u32::from_le_bytes([p[0] as u8, p[1] as u8, p[2] as u8, 255]);
                over(tint, p, 255 - a)
            })
            .collect();
        // Back to full size, each pixel between the four small ones
        // around it: each small row is spread across the full width
        // once, and each full row is then between two of those. On a
        // dense screen (a small pixel sixteen or more across) the colour
        // is worked out once for each two by two pixels, at the middle of
        // the four: across a sixteenth of a blurred small pixel it moves
        // by less than a level, so the four are the same to the eye, and
        // the enlarging is a quarter of the work.
        let step = if s >= 16 { 2 } else { 1 };
        // The pixels from `k` to `k + step` lie between small pixels
        // `i0` and `i1`, `t` of 256 of the way.
        let place = |k: usize, n: usize| -> (usize, usize, u32) {
            let f = ((k * 2 + step) * 128 / s).saturating_sub(128);
            let (i0, t) = (f / 256, (f % 256) as u32);
            (i0.min(n - 1), (i0 + 1).min(n - 1), t)
        };
        let columns: Vec<(usize, usize, u32)> =
            (0..w).step_by(step).map(|x| place(x, sw)).collect();
        let cw = columns.len();
        let wide: Vec<u32> = (0..sh)
            .flat_map(|y| {
                let row = &packed[y * sw..y * sw + sw];
                columns
                    .iter()
                    .map(move |&(x0, x1, t)| lerp(row[x0], row[x1], t))
            })
            .collect();
        let mut line: Vec<u32> = alloc::vec![0; cw];
        let data = self.pixmap.data_mut();
        for y in (0..h).step_by(step) {
            let (y0, y1, t) = place(y, sh);
            let (a, b) = (&wide[y0 * cw..y0 * cw + cw], &wide[y1 * cw..y1 * cw + cw]);
            if step == 1 {
                let row = &mut data[y * w * 4..(y + 1) * w * 4];
                for ((p, &top), &bottom) in row.chunks_exact_mut(4).zip(a).zip(b) {
                    p.copy_from_slice(&(lerp(top, bottom, t) | 0xff00_0000).to_le_bytes());
                }
                continue;
            }
            for ((p, &top), &bottom) in line.iter_mut().zip(a).zip(b) {
                *p = lerp(top, bottom, t) | 0xff00_0000;
            }
            for yy in y..(y + step).min(h) {
                let row = &mut data[yy * w * 4..(yy + 1) * w * 4];
                for (pixels, &c) in row.chunks_mut(4 * step).zip(&line) {
                    let c = c.to_le_bytes();
                    for p in pixels.chunks_exact_mut(4) {
                        p.copy_from_slice(&c);
                    }
                }
            }
        }
    }

    /// Puts back a [`Canvas::snapshot`] of this canvas; one of another
    /// size is ignored. Put back again, the same pixels in the same place
    /// are copied only where something was drawn since: kept pixels
    /// are not changed in place.
    pub fn restore(&mut self, pixels: &[u8]) {
        (self.counter)(Work::FullCanvas);
        if pixels.len() != self.pixmap.data().len() {
            return;
        }
        let id = (pixels.as_ptr() as usize, pixels.len());
        if self.kept == Some(id) {
            let Some(d) = self.damage.take() else {
                return;
            };
            let stride = self.pixmap.width() as usize * 4;
            let data = self.pixmap.data_mut();
            for y in d.y..d.bottom() {
                let at = y as usize * stride + d.x as usize * 4;
                let n = d.w as usize * 4;
                data[at..at + n].copy_from_slice(&pixels[at..at + n]);
            }
            return;
        }
        self.pixmap.data_mut().copy_from_slice(pixels);
        self.kept = Some(id);
        self.damage = None;
    }

    /// A copy of every pixel, for [`Canvas::blend_between`] to fade from.
    pub fn snapshot(&self) -> Vec<u8> {
        (self.counter)(Work::FullCanvas);
        self.pixmap.data().to_vec()
    }

    /// The same copy into `into`, reusing its room.
    pub fn snapshot_into(&self, into: &mut Vec<u8>) {
        (self.counter)(Work::FullCanvas);
        into.clear();
        into.extend_from_slice(self.pixmap.data());
    }

    /// Shows `t` (0 to 1) of the way from `before` to `after`, two
    /// [`Canvas::snapshot`]s of this canvas, without drawing either
    /// again: a step of a cross-fade whose new frame is kept aside
    /// ([`Canvas::swap_pixels`]). Every pixel is written; snapshots of
    /// another size leave the canvas as it is.
    pub fn blend_between(&mut self, before: &[u8], after: &[u8], t: f32) {
        (self.counter)(Work::FullCanvas);
        let n = self.pixmap.data().len();
        if before.len() != n || after.len() != n {
            return;
        }
        // Each channel of a premultiplied pixel on its own, in 16 bits:
        // 255 x 256 fits, and a loop over bytes is one the compiler runs
        // many lanes at a time. A frosted page under a sheet differs
        // from the page before it everywhere, so most pixels of such a
        // fade are blended.
        let t = (t.clamp(0.0, 1.0) * 256.0) as u16;
        let u = 256 - t;
        self.damaged(self.bounds());
        let data = self.pixmap.data_mut();
        // Where the two frames agree the new one is copied, a run of
        // pixels at a time: most of a change of screen (the sidebar, the
        // page's ground).
        const RUN: usize = 4 * 16;
        let runs = data
            .chunks_mut(RUN)
            .zip(before.chunks(RUN))
            .zip(after.chunks(RUN));
        for ((now, was), will) in runs {
            if was == will || t == 256 {
                now.copy_from_slice(will);
                continue;
            }
            for ((now, &w), &n) in now.iter_mut().zip(was).zip(will) {
                *now = ((u16::from(w) * u + u16::from(n) * t) >> 8) as u8;
            }
        }
    }

    /// Exchanges the canvas's pixels with `pixels`, a
    /// [`Canvas::snapshot`]'s worth or empty: the frame just drawn is
    /// kept aside without a copy, and the canvas holds what `pixels`
    /// held, or the page colour's zeros when it was empty, until it is
    /// drawn over. Pixels of another size leave both as they are.
    pub fn swap_pixels(&mut self, pixels: &mut Vec<u8>) {
        let (w, h) = (self.pixmap.width(), self.pixmap.height());
        let n = self.pixmap.data().len();
        let given = if pixels.len() == n {
            core::mem::take(pixels)
        } else if pixels.is_empty() {
            alloc::vec![0; n]
        } else {
            return;
        };
        let Some(next) = tiny_skia::IntSize::from_wh(w, h).and_then(|s| Pixmap::from_vec(given, s))
        else {
            return;
        };
        *pixels = core::mem::replace(&mut self.pixmap, next).take();
        self.kept = None;
        self.damaged(self.bounds());
    }

    /// Costs a copy of the rectangle's pixels. The pixels inside `rect`,
    /// cut at the canvas's edges, a row at a time into `into`, reusing its
    /// room: for [`Canvas::fade_region`] to fade from.
    pub fn copy_region(&self, rect: Rect, into: &mut Vec<u8>) {
        into.clear();
        let r = rect.intersect(&self.bounds());
        if r.is_empty() {
            return;
        }
        let stride = self.pixmap.width() as usize * 4;
        let data = self.pixmap.data();
        for row in r.y..r.bottom() {
            let at = row as usize * stride + r.x as usize * 4;
            into.extend_from_slice(&data[at..at + r.w as usize * 4]);
        }
    }

    /// Costs a pass over the rectangle's pixels. Shows what was drawn
    /// inside `rect` since `before` was taken there ([`Canvas::copy_region`])
    /// only `t` (0 to 1) of the way from it: something that fades in or
    /// out over what it stands on. A `before` of another size leaves the
    /// canvas as it is.
    pub fn fade_region(&mut self, rect: Rect, before: &[u8], t: f32) {
        let r = rect.intersect(&self.bounds());
        if r.is_empty() || before.len() != (r.w * r.h * 4) as usize {
            return;
        }
        let t = (t.clamp(0.0, 1.0) * 256.0) as u16;
        let u = 256 - t;
        let stride = self.pixmap.width() as usize * 4;
        let n = r.w as usize * 4;
        let data = self.pixmap.data_mut();
        for (k, row) in (r.y..r.bottom()).enumerate() {
            let at = row as usize * stride + r.x as usize * 4;
            let was = &before[k * n..(k + 1) * n];
            for (now, &w) in data[at..at + n].iter_mut().zip(was) {
                *now = ((u16::from(w) * u + u16::from(*now) * t) >> 8) as u8;
            }
        }
        self.damaged(r);
    }

    /// Moves the pixels inside `rect` by `dy` rows, down for a positive
    /// `dy` and up for a negative one, ignoring the clip. Rows outside
    /// `rect` are untouched, and so are the rows that `dy` brought into
    /// it: they hold what was there before, for the caller to draw over.
    pub fn shift(&mut self, rect: Rect, dy: i32) {
        let r = rect.intersect(&self.bounds());
        if r.is_empty() || dy == 0 || dy.abs() >= r.h {
            return;
        }
        self.damaged(r);
        let stride = self.pixmap.width() as usize;
        let (x, w) = (r.x as usize, r.w as usize);
        let data = self.pixmap.pixels_mut();
        let mut row = |dst_y: i32| {
            let src = (dst_y - dy) as usize * stride + x;
            let dst = dst_y as usize * stride + x;
            data.copy_within(src..src + w, dst);
        };
        if dy > 0 {
            // Downwards, from the bottom row up, so a row is copied
            // before it is overwritten.
            for dst_y in (r.y + dy..r.bottom()).rev() {
                row(dst_y);
            }
        } else {
            for dst_y in r.y..r.bottom() + dy {
                row(dst_y);
            }
        }
    }

    /// Draws a camera frame into `rect`: scaled to cover the rectangle
    /// (the larger of the two scale factors), cropped to it and centred,
    /// as a camera app's preview is. Nearest-neighbour, so a frame costs
    /// one read and one write per pixel drawn; the clip is honoured and
    /// nothing outside `rect` is touched.
    ///
    /// `pixels` is row-major, `width × height` bytes, black 0 to white
    /// 255. A short buffer or a zero side draws nothing.
    ///
    /// `chroma` is the frame's NV12 chroma plane — `⌈width / 2⌉ ×
    /// ⌈height / 2⌉` interleaved U and V pairs — and the image is drawn
    /// in colour when it is there, grey when it is not or when it is too
    /// short for the luma it claims to belong to. The conversion is
    /// BT.601 limited range (luma 16–235, chroma centred on 128), which
    /// is what every capture path delivers as "video range", in integer
    /// arithmetic in the same pass as the scale.
    pub fn luma(
        &mut self,
        rect: Rect,
        width: u16,
        height: u16,
        pixels: &[u8],
        chroma: Option<&[u8]>,
    ) {
        let dst = rect.intersect(&self.clip());
        let (sw, sh) = (usize::from(width), usize::from(height));
        if dst.is_empty() || sw == 0 || sh == 0 || pixels.len() < sw * sh {
            return;
        }
        let (cw, ch) = (sw.div_ceil(2), sh.div_ceil(2));
        let chroma = chroma.filter(|c| c.len() >= cw * ch * 2);
        let scale = (rect.w as f32 / sw as f32).max(rect.h as f32 / sh as f32);
        if scale <= 0.0 {
            return;
        }
        // The source pixel at the rectangle's top-left corner: the crop
        // takes the same amount off both sides of the longer axis.
        let left = (sw as f32 - rect.w as f32 / scale) / 2.0;
        let top = (sh as f32 - rect.h as f32 / scale) / 2.0;
        self.damaged(dst);
        let stride = self.pixmap.width() as usize;
        let data = self.pixmap.pixels_mut();
        for y in dst.y..dst.bottom() {
            let sy = ((top + ((y - rect.y) as f32 + 0.5) / scale) as usize).min(sh - 1);
            let row = sy * sw;
            let uv_row = (sy / 2) * cw;
            let out = y as usize * stride;
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
                data[out + x as usize] =
                    PremultipliedColorU8::from_rgba(r, g, b, 255).expect("opaque pixel");
            }
        }
    }

    /// Fills a rectangle with rounded corners of `radius` pixels.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        if rect.is_empty() || color.a == 0 || rect.intersect(&self.clip()).is_empty() {
            return;
        }
        if radius <= 0.5 {
            self.fill_rect(rect, color);
            return;
        }
        let paint = Self::paint(color);
        // Opaque, the rows between the corners are whole pixels from
        // edge to edge, the same as the outline gives them: a plain
        // fill, and the corners' rows each through an outline of their
        // own that ends straight across where the plain fill begins.
        let cap = radius.min(rect.w as f32 / 2.0) as i32 + 2;
        if color.a == 255 && rect.h - 2 * cap >= SPLIT_ROWS {
            let mid = Rect::new(rect.x, rect.y + cap, rect.w, rect.h - 2 * cap);
            for (cut, top) in [(mid.y, true), (mid.bottom(), false)] {
                let ends = rounded_ends_path(rect, radius, (cut, top));
                self.draw_path(&ends, &paint, None);
            }
            self.fill_rect(mid, color);
            return;
        }
        let path = rounded_rect_path(rect, radius, 0.0);
        self.draw_path(&path, &paint, None);
    }

    /// Strokes a rounded rectangle outline `width` pixels thick, inside
    /// the rect.
    pub fn stroke_rounded_rect(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        if rect.is_empty() || color.a == 0 || rect.intersect(&self.clip()).is_empty() {
            return;
        }
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        let path = rounded_rect_path(rect, radius, width / 2.0);
        self.draw_path(&path, &Self::paint(color), Some(&stroke));
    }

    /// Draws a line `width` pixels thick between two points.
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, color: Color) {
        if color.a == 0 {
            return;
        }
        let mut pb = PathBuilder::new();
        pb.move_to(x0, y0);
        pb.line_to(x1, y1);
        let Some(path) = pb.finish() else { return };
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.draw_path(&path, &Self::paint(color), Some(&stroke));
    }

    /// Fills a circle.
    pub fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        // Wholly outside the clip, as under a pass that only measures:
        // no path is made.
        let reach = radius + 2.0;
        let clip = self.clip();
        if color.a == 0
            || clip.is_empty()
            || cx + reach <= clip.x as f32
            || cx - reach >= clip.right() as f32
            || cy + reach <= clip.y as f32
            || cy - reach >= clip.bottom() as f32
        {
            return;
        }
        let Some(path) = PathBuilder::from_circle(cx, cy, radius) else {
            return;
        };
        self.draw_path(&path, &Self::paint(color), None);
    }

    /// Fills a shape made of closed outlines through the given points, in
    /// pixels. An outline wound the other way round from the one around
    /// it is a hole.
    pub fn fill_contours(&mut self, contours: &[&[(f32, f32)]], color: Color) {
        if color.a == 0 {
            return;
        }
        let mut pb = PathBuilder::new();
        for points in contours {
            let Some((&(x0, y0), rest)) = points.split_first() else {
                continue;
            };
            pb.move_to(x0, y0);
            for &(x, y) in rest {
                pb.line_to(x, y);
            }
            pb.close();
        }
        let Some(path) = pb.finish() else { return };
        self.draw_path(&path, &Self::paint(color), None);
    }

    /// Fills `path`, or strokes it with `stroke`, inside the clip.
    ///
    /// No mask: tiny-skia blends through one about thirty times slower
    /// than without, and building a panel-sized one for each clip a frame
    /// pushes (every segment of a segmented button) was most of a
    /// frame's cost on a large display. A shape across an edge of the
    /// clip is drawn whole and the pixels of its box outside the clip are
    /// put back as they were: the same pixels as the whole shape cut by
    /// the clip, which a scroll that copies rows and draws the strip it
    /// uncovered depends on. A shape wholly outside the clip is not
    /// drawn.
    fn draw_path(&mut self, path: &tiny_skia::Path, paint: &Paint, stroke: Option<&Stroke>) {
        // Every pixel the shape's ink can reach: half a stroke's width
        // beyond the path, one more for the anti-aliased edge, and one
        // for the truncation to whole pixels.
        let b = path.bounds();
        let grow = stroke.map_or(0.0, |s| s.width / 2.0) + 1.0;
        let (l, t) = ((b.left() - grow) as i32 - 1, (b.top() - grow) as i32 - 1);
        let (r, bo) = (
            (b.right() + grow) as i32 + 1,
            (b.bottom() + grow) as i32 + 1,
        );
        let clip = self.clip();
        self.damaged(Rect::new(l, t, r - l, bo - t).intersect(&clip));
        // A shape wholly outside the clip reaches no pixel the clip
        // keeps: a card scrolled off the page costs its layout and
        // nothing more, however long the page.
        if clip.is_empty()
            || r <= clip.x
            || l >= clip.right()
            || t.max(clip.y) >= bo.min(clip.bottom())
        {
            return;
        }
        let (width, height) = (self.pixmap.width() as i32, self.pixmap.height() as i32);
        let (y0, y1) = (t.max(0), bo.min(height));
        let row = width as usize * 4;
        let region = Rect::new(l.max(0), y0, r.min(width) - l.max(0), y1 - y0);
        let pieces = outside(region, clip);
        let span = |y: i32, p: &Rect| {
            let at = y as usize * row;
            at + p.x as usize * 4..at + p.right() as usize * 4
        };
        let saved = &mut self.scratch;
        saved.clear();
        let data = self.pixmap.data();
        for p in &pieces {
            for y in p.y..p.bottom() {
                saved.extend_from_slice(&data[span(y, p)]);
            }
        }
        // Drawn on the whole canvas, which holds the whole shape: a
        // shape cut where the clip crosses one of its curves would have
        // other coverage there than the whole shape has.
        let mut pixmap = self.pixmap.as_mut();
        let ts = Transform::identity();
        match stroke {
            Some(s) => pixmap.stroke_path(path, paint, s, ts, None),
            None => pixmap.fill_path(path, paint, FillRule::Winding, ts, None),
        }
        let data = self.pixmap.data_mut();
        let mut at = 0;
        for p in &pieces {
            for y in p.y..p.bottom() {
                let r = span(y, p);
                let n = r.len();
                data[r].copy_from_slice(&self.scratch[at..at + n]);
                at += n;
            }
        }
    }

    /// Single-line metrics of `text` in `font` on this display.
    pub fn measure_text(&self, text: &str, font: Font) -> TextMetrics {
        text::measure(&font.sized(self.scale), text)
    }

    /// Draws one line of text with the top of its line box at `y`.
    /// Returns the width drawn.
    pub fn text(&mut self, x: i32, y: i32, s: &str, font: Font, color: Color) -> i32 {
        let face = font.sized(self.scale);
        self.text_with_face(&face, x, y, s, color)
    }

    /// Draws one line of text using an already-resolved face and size.
    pub fn text_with_face(
        &mut self,
        face: &SizedFace,
        x: i32,
        y: i32,
        s: &str,
        color: Color,
    ) -> i32 {
        let baseline = y + face.ascent;
        let clip = self.clip();
        let mut pen = (x.max(-32_768) as i64) << 6;
        // A line wholly above or below the clip, a line box's height to
        // spare for marks past the ascent and descent, shows nothing:
        // its width is worked out from the advances, and no glyph is
        // looked up or blended (a long list scrolled away, a page's
        // rows past the panel's foot), and nor does any line under a
        // clip with no room, a pass that only measures.
        let lh = face.line_height();
        if clip.is_empty() || y + 2 * lh <= clip.y || y - lh >= clip.bottom() {
            for c in s.chars() {
                pen += i64::from(face.advance_64(c));
            }
            return ((pen + 32) >> 6) as i32 - x;
        }
        self.lines += 1;
        let start = pen;
        let mut cut = false;
        let mut bbox: Option<Rect> = None;
        for c in s.chars() {
            // The cache and the pixmap are separate fields, so the glyph
            // stays borrowed while its coverage is blended.
            let Some(g) = self.glyphs.glyph(face, c) else {
                continue;
            };
            let gx = ((pen + 32) >> 6) as i32 + g.bearing_x;
            let gy = baseline - g.bearing_y;
            cut = cut || ink_past_sides(clip, gx, gy, g.width, g.height, &g.bitmap);
            if self.ink.is_some() && g.width > 0 && g.height > 0 {
                let r = Rect::new(gx, gy, g.width as i32, g.height as i32).intersect(&clip);
                if !r.is_empty() {
                    bbox = Some(bbox.map_or(r, |b| union(b, r)));
                }
            }
            blit_glyph(
                &mut self.pixmap,
                clip,
                gx,
                gy,
                g.width,
                g.height,
                g.bitmap,
                color,
            );
            pen += i64::from(g.advance_64);
        }
        // Every pixel its glyphs can reach: a line box above and below,
        // and a line's height either side for a mark past its advance.
        let (from, to) = (((start + 32) >> 6) as i32, ((pen + 32) >> 6) as i32);
        self.damaged(Rect::new(from - lh, y - lh, to - from + 2 * lh, 3 * lh).intersect(&clip));
        // Once per string: a scroll draws the same strip again and
        // again without a clear in between.
        if cut && !self.cut.iter().any(|t| t == s) {
            self.cut.push(String::from(s));
        }
        let family = face.family;
        self.note_ink(
            || InkKind::Text {
                text: String::from(s),
                family,
                color,
            },
            bbox,
        );
        ((pen + 32) >> 6) as i32 - x
    }

    /// Draws one icon glyph centred in `rect`. Icons carry no baseline:
    /// the glyph's own bitmap is centred in the square it is given.
    pub fn icon(&mut self, rect: Rect, icon: crate::widgets::Icon, size_dp: f32, color: Color) {
        let face = Font::icon(size_dp).sized(self.scale);
        let clip = self.clip();
        let Some(g) = self.glyphs.glyph(&face, icon.code_point()) else {
            return;
        };
        let x = rect.x + (rect.w - g.width as i32) / 2;
        let y = rect.y + (rect.h - g.height as i32) / 2;
        let inked = Rect::new(x, y, g.width as i32, g.height as i32).intersect(&clip);
        blit_glyph(
            &mut self.pixmap,
            clip,
            x,
            y,
            g.width,
            g.height,
            g.bitmap,
            color,
        );
        self.damaged(inked);
        self.note_ink(|| InkKind::Icon(icon), Some(inked));
    }

    /// Draws `s` word-wrapped inside `rect`, top-aligned, clipped to the
    /// rect. Returns the height used.
    pub fn text_wrapped(
        &mut self,
        rect: Rect,
        s: &str,
        font: Font,
        color: Color,
        align: TextAlign,
    ) -> i32 {
        let face = font.sized(self.scale);
        let lines = text::wrap(&face, s, rect.w);
        let lh = face.line_height();
        let mut y = rect.y;
        for &(start, end) in &lines {
            if y >= rect.bottom() {
                break;
            }
            let line = &s[start..end];
            let w = text::width(&face, line);
            let x = match align {
                TextAlign::Start => rect.x,
                TextAlign::Center => rect.x + (rect.w - w) / 2,
                TextAlign::End => rect.right() - w,
            };
            self.text_with_face(&face, x, y, line, color);
            y += lh;
        }
        lines.len() as i32 * lh
    }
}

/// The parts of `region` outside `clip`, as four rectangles that do not
/// overlap: the rows above the clip and below it, then beside it to the
/// left and the right in the clip's rows. An empty one is
/// `Rect::default()`, no rows.
fn outside(region: Rect, clip: Rect) -> [Rect; 4] {
    let top = clip.y.clamp(region.y, region.bottom());
    let bottom = clip.bottom().clamp(top, region.bottom());
    let band = |y0: i32, y1: i32, x0: i32, x1: i32| Rect::new(x0, y0, x1 - x0, y1 - y0);
    [
        band(region.y, top, region.x, region.right()),
        band(bottom, region.bottom(), region.x, region.right()),
        band(top, bottom, region.x, clip.x.min(region.right())),
        band(top, bottom, clip.right().max(region.x), region.right()),
    ]
    .map(|p| if p.is_empty() { Rect::default() } else { p })
}

/// The smallest rectangle holding both.
fn union(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect::new(
        x,
        y,
        a.right().max(b.right()) - x,
        a.bottom().max(b.bottom()) - y,
    )
}

/// Alpha-blends a coverage bitmap in `color` onto the premultiplied
/// framebuffer, honouring `clip`.
///
/// A free function rather than a method: it takes the framebuffer alone,
/// so a glyph can stay borrowed from the canvas's cache while it is
/// blended.
#[allow(clippy::too_many_arguments)]
fn blit_glyph(
    pixmap: &mut Pixmap,
    clip: Rect,
    x: i32,
    y: i32,
    w: usize,
    h: usize,
    bitmap: Coverage,
    color: Color,
) {
    {
        if w == 0 || h == 0 || color.a == 0 {
            return;
        }
        let clip = clip.intersect(&Rect::new(x, y, w as i32, h as i32));
        if clip.is_empty() {
            return;
        }
        let stride = pixmap.width() as usize * 4;
        let data = pixmap.data_mut();
        let (cr, cg, cb, ca) = (
            u32::from(color.r),
            u32::from(color.g),
            u32::from(color.b),
            u32::from(color.a),
        );
        for py in clip.y..clip.bottom() {
            let row = (py - y) as usize;
            let dst_row = py as usize * stride;
            for px in clip.x..clip.right() {
                let cov = u32::from(bitmap.get(row * w + (px - x) as usize));
                if cov == 0 {
                    continue;
                }
                // Source alpha after coverage, in 0..=255.
                let a = div255(ca * cov);
                let inv = 255 - a;
                let i = dst_row + px as usize * 4;
                let d = &mut data[i..i + 4];
                d[0] = (div255(cr * a) + div255(u32::from(d[0]) * inv)) as u8;
                d[1] = (div255(cg * a) + div255(u32::from(d[1]) * inv)) as u8;
                d[2] = (div255(cb * a) + div255(u32::from(d[2]) * inv)) as u8;
                d[3] = (a + div255(u32::from(d[3]) * inv)) as u8;
            }
        }
    }
}

/// Whether a glyph at `(x, y)` has ink left or right of `clip` on a
/// line the clip spans: the part of the glyph a side of the clip cuts
/// off. Rows above and below the clip do not count.
fn ink_past_sides(clip: Rect, x: i32, y: i32, w: usize, h: usize, bitmap: &Coverage) -> bool {
    let (w_i, h_i) = (w as i32, h as i32);
    if w == 0 || (x >= clip.x && x + w_i <= clip.right()) {
        return false;
    }
    let (y0, y1) = (y.max(clip.y), (y + h_i).min(clip.bottom()));
    (y0..y1).any(|py| {
        let row = (py - y) as usize * w;
        (0..w_i)
            .filter(|&col| x + col < clip.x || x + col >= clip.right())
            .any(|col| bitmap.get(row + col as usize) != 0)
    })
}

/// One pixel of BT.601 limited-range YCbCr as RGB, in integers.
///
/// Limited range is what a camera delivers as "video range": luma runs
/// 16 to 235 and the two chroma bytes are centred on 128. The
/// coefficients are the usual 8-bit fixed-point form of the BT.601
/// matrix, so the whole conversion is three multiply-adds and a shift.
fn yuv_to_rgb(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
    let c = i32::from(y) - 16;
    let d = i32::from(u) - 128;
    let e = i32::from(v) - 128;
    let clamp = |v: i32| v.clamp(0, 255) as u8;
    (
        clamp((298 * c + 409 * e + 128) >> 8),
        clamp((298 * c - 100 * d - 208 * e + 128) >> 8),
        clamp((298 * c + 516 * d + 128) >> 8),
    )
}

/// `v / 255` rounded, for `v` up to `255 × 255`.
/// One pass of a box blur `r` either side, along rows (`across`) or
/// columns, over a `w × h` image of colours; the edges repeat.
fn box_blur(px: &[[u32; 3]], w: usize, h: usize, r: usize, across: bool) -> Vec<[u32; 3]> {
    let mut out = alloc::vec![[0u32; 3]; px.len()];
    let (lines, len) = if across { (h, w) } else { (w, h) };
    let n = (2 * r + 1) as u32;
    for line in 0..lines {
        let idx = |i: usize| if across { line * w + i } else { i * w + line };
        let get = |i: isize| px[idx(i.clamp(0, len as isize - 1) as usize)];
        let mut sum = [0u32; 3];
        for i in -(r as isize)..=(r as isize) {
            let p = get(i);
            for k in 0..3 {
                sum[k] += p[k];
            }
        }
        for i in 0..len {
            out[idx(i)] = [sum[0] / n, sum[1] / n, sum[2] / n];
            let (add, sub) = (
                get(i as isize + r as isize + 1),
                get(i as isize - r as isize),
            );
            for k in 0..3 {
                sum[k] = sum[k] + add[k] - sub[k];
            }
        }
    }
    out
}

/// `src` (premultiplied) over `dst`, whose share left is `inv` of 255,
/// all four bytes at once: red with blue and green with alpha each in
/// one word.
#[inline]
fn over(src: u32, dst: u32, inv: u32) -> u32 {
    let div = |x: u32| {
        let x = x + 0x0080_0080;
        ((x + ((x >> 8) & 0x00ff_00ff)) >> 8) & 0x00ff_00ff
    };
    let rb = div((dst & 0x00ff_00ff) * inv);
    let ga = div(((dst >> 8) & 0x00ff_00ff) * inv);
    src + (rb | (ga << 8))
}

/// `t` of 256 of the way from pixel `a` to pixel `b`, all four bytes at
/// once.
#[inline]
fn lerp(a: u32, b: u32, t: u32) -> u32 {
    let u = 256 - t;
    let rb = (((a & 0x00ff_00ff) * u + (b & 0x00ff_00ff) * t) >> 8) & 0x00ff_00ff;
    let ga = ((((a >> 8) & 0x00ff_00ff) * u + ((b >> 8) & 0x00ff_00ff) * t) >> 8) & 0x00ff_00ff;
    rb | (ga << 8)
}

/// A square root good to a few parts in a million, without `std`.
fn sqrt(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut g = f32::from_bits((x.to_bits() >> 1) + 0x1fbd_1df5);
    g = 0.5 * (g + x / g);
    0.5 * (g + x / g)
}

fn div255(v: u32) -> u32 {
    (v + 128 + ((v + 128) >> 8)) >> 8
}

fn skia_rect(r: Rect) -> tiny_skia::Rect {
    tiny_skia::Rect::from_xywh(r.x as f32, r.y as f32, r.w.max(1) as f32, r.h.max(1) as f32)
        .expect("finite non-empty rect")
}

/// A rounded-rectangle path inset by `inset` pixels on every side.
fn rounded_rect_path(rect: Rect, radius: f32, inset: f32) -> tiny_skia::Path {
    let l = rect.x as f32 + inset;
    let t = rect.y as f32 + inset;
    let r = rect.right() as f32 - inset;
    let b = rect.bottom() as f32 - inset;
    let radius = radius.min((r - l) / 2.0).min((b - t) / 2.0).max(0.0);
    let mut pb = PathBuilder::with_capacity(10, 17);
    if radius <= 0.0 {
        pb.move_to(l, t);
        pb.line_to(r, t);
        pb.line_to(r, b);
        pb.line_to(l, b);
        pb.close();
    } else {
        // Circular corners approximated with cubic Béziers (k ≈ 0.5523).
        let k = 0.5523 * radius;
        pb.move_to(l + radius, t);
        pb.line_to(r - radius, t);
        pb.cubic_to(r - radius + k, t, r, t + radius - k, r, t + radius);
        pb.line_to(r, b - radius);
        pb.cubic_to(r, b - radius + k, r - radius + k, b, r - radius, b);
        pb.line_to(l + radius, b);
        pb.cubic_to(l + radius - k, b, l, b - radius + k, l, b - radius);
        pb.line_to(l, t + radius);
        pb.cubic_to(l, t + radius - k, l + radius - k, t, l + radius, t);
        pb.close();
    }
    pb.finish().expect("non-degenerate path")
}

/// The top or the bottom of [`rounded_rect_path`]'s outline with no
/// inset, its two corners and the side between them, down or up to the
/// row `cut.0` (`cut.1` true for the top) and closed straight across
/// it. Within its rows its edges are the whole outline's, so it fills
/// the same pixels there.
fn rounded_ends_path(rect: Rect, radius: f32, (cut, top): (i32, bool)) -> tiny_skia::Path {
    let l = rect.x as f32;
    let t = rect.y as f32;
    let r = rect.right() as f32;
    let b = rect.bottom() as f32;
    let radius = radius.min((r - l) / 2.0).min((b - t) / 2.0).max(0.0);
    let k = 0.5523 * radius;
    let c = cut as f32;
    let mut pb = PathBuilder::with_capacity(8, 15);
    if top {
        pb.move_to(l, c);
        pb.line_to(l, t + radius);
        pb.cubic_to(l, t + radius - k, l + radius - k, t, l + radius, t);
        pb.line_to(r - radius, t);
        pb.cubic_to(r - radius + k, t, r, t + radius - k, r, t + radius);
        pb.line_to(r, c);
    } else {
        pb.move_to(r, c);
        pb.line_to(r, b - radius);
        pb.cubic_to(r, b - radius + k, r - radius + k, b, r - radius, b);
        pb.line_to(l + radius, b);
        pb.cubic_to(l + radius - k, b, l, b - radius + k, l, b - radius);
        pb.line_to(l, c);
    }
    pb.close();
    pb.finish().expect("non-degenerate path")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> Canvas {
        Canvas::new(&DisplayInfo {
            width: 64,
            height: 32,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: faraday_shell_api::SecureHardware::None,
            boot: faraday_shell_api::BootState::Unknown,
            memory_mib: None,
        })
    }

    /// A tall card's fill, drawn fast between its corners, and its edge are
    /// the pixels of its whole outline filled or stroked, cut by a clip
    /// or not.
    #[test]
    fn a_tall_rounded_rect_is_its_whole_outline() {
        let tall = || {
            Canvas::new(&DisplayInfo {
                width: 64,
                height: 128,
                dpi: 160,
                inset_bottom: 0,
                inset_top: 0,
                buttons: 0,
                camera_fixed: true,
                secure: faraday_shell_api::SecureHardware::None,
                boot: faraday_shell_api::BootState::Unknown,
                memory_mib: None,
            })
        };
        let colors = [
            Color::rgb(90, 160, 220),
            Color::rgb(240, 200, 120),
            Color::rgb(140, 210, 180),
        ];
        let rects = [
            (Rect::new(3, 1, 40, 120), 6.5),
            (Rect::new(10, 0, 20, 128), 3.0),
            (Rect::new(2, 4, 58, 100), 14.25),
        ];
        // Across the sides and through the third's bottom corners.
        let regions = [Rect::new(8, 6, 48, 100), Rect::new(8, 10, 48, 90)];
        for (k, &(rect, radius)) in rects.iter().enumerate() {
            for width in [0.0, 1.0, 2.0, 1.5] {
                let draw = |fast: bool, clip: Option<Rect>| {
                    let mut c = tall();
                    c.clear(Color::BLACK);
                    if let Some(r) = clip {
                        c.push_clip(r);
                    }
                    let color = colors[k];
                    if fast && width == 0.0 {
                        c.fill_rounded_rect(rect, radius, color);
                    } else if fast {
                        c.stroke_rounded_rect(rect, radius, width, color);
                    } else if width == 0.0 {
                        let path = rounded_rect_path(rect, radius, 0.0);
                        c.draw_path(&path, &Canvas::paint(color), None);
                    } else {
                        let path = rounded_rect_path(rect, radius, width / 2.0);
                        let stroke = Stroke {
                            width,
                            ..Stroke::default()
                        };
                        c.draw_path(&path, &Canvas::paint(color), Some(&stroke));
                    }
                    c
                };
                let whole = draw(false, None);
                for (name, c, region) in [
                    ("fast", draw(true, None), None),
                    ("fast cut", draw(true, Some(regions[0])), Some(regions[0])),
                    ("fast cut", draw(true, Some(regions[1])), Some(regions[1])),
                    ("path cut", draw(false, Some(regions[1])), Some(regions[1])),
                ] {
                    for y in 0..128 {
                        for x in 0..64 {
                            let want = if region.is_none_or(|r| r.contains(x, y)) {
                                pixel(&whole, x, y)
                            } else {
                                [0, 0, 0, 255]
                            };
                            assert_eq!(pixel(&c, x, y), want, "{name} {k} {width} at ({x}, {y})");
                        }
                    }
                }
            }
        }
    }

    fn pixel(c: &Canvas, x: i32, y: i32) -> [u8; 4] {
        let f = c.frame();
        let i = (y as usize * usize::from(f.width) + x as usize) * 4;
        [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2], f.rgba[i + 3]]
    }

    #[test]
    fn text_blends_ink_and_respects_clip() {
        let mut c = canvas();
        c.clear(Color::BLACK);
        let w = c.text(2, 2, "III", Font::regular(16.0), Color::WHITE);
        assert!(w > 0);
        let f = c.frame();
        let lit = f.rgba.chunks(4).filter(|p| p[0] > 0).count();
        assert!(lit > 10, "text left ink");
        // Every pixel stays premultiplied-valid: colour <= alpha.
        for p in f.rgba.chunks(4) {
            assert!(p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]);
        }
        let mut c2 = canvas();
        c2.clear(Color::BLACK);
        c2.push_clip(Rect::new(0, 0, 1, 1));
        c2.text(2, 2, "III", Font::regular(16.0), Color::WHITE);
        let lit = c2.frame().rgba.chunks(4).filter(|p| p[0] > 0).count();
        assert_eq!(lit, 0);
    }

    /// Measurement reads the advances in the outline file, so a string
    /// is the same width whether or not its glyphs have been drawn.
    #[test]
    fn text_measures_the_same_before_and_after_it_is_drawn() {
        let mut c = canvas();
        let font = Font::regular(16.0);
        let before = c.measure_text("Fingerprint 0f056943", font);
        c.clear(Color::BLACK);
        c.text(2, 2, "Fingerprint 0f056943", font, Color::WHITE);
        assert_eq!(c.measure_text("Fingerprint 0f056943", font), before);
    }

    /// The glyph cache changes nothing a reader sees: the second draw of
    /// a line, with every glyph already filled, is the first draw's
    /// pixels.
    #[test]
    fn a_line_of_text_draws_the_same_pixels_twice() {
        let mut c = canvas();
        c.clear(Color::BLACK);
        c.text(2, 2, "abc 123", Font::regular(16.0), Color::WHITE);
        let first = c.frame().rgba.to_vec();
        c.clear(Color::BLACK);
        c.text(2, 2, "abc 123", Font::regular(16.0), Color::WHITE);
        assert_eq!(c.frame().rgba, &first[..]);
    }

    #[test]
    fn luma_covers_and_crops_a_wider_image() {
        // 4 x 2 source, left half black and right half white, into a
        // 32 x 32 square: cover scales by 16, so the middle two columns
        // fill the square and the outer ones are cropped away.
        let mut c = canvas();
        c.clear(Color::WHITE);
        let src = [0, 0, 255, 255, 0, 0, 255, 255];
        c.luma(Rect::new(0, 0, 32, 32), 4, 2, &src, None);
        assert_eq!(pixel(&c, 0, 0), [0, 0, 0, 255], "source column 1");
        assert_eq!(pixel(&c, 31, 31), [255, 255, 255, 255], "source column 2");
        // Nothing outside the rectangle was touched.
        assert_eq!(pixel(&c, 40, 10), [255, 255, 255, 255]);
    }

    #[test]
    fn luma_covers_and_crops_a_taller_image() {
        // 2 x 4 source, top half black and bottom half white, into a
        // 16 x 16 square: the middle two rows fill it.
        let mut c = canvas();
        c.clear(Color::WHITE);
        let src = [0, 0, 0, 0, 255, 255, 255, 255];
        c.luma(Rect::new(0, 0, 16, 16), 2, 4, &src, None);
        assert_eq!(pixel(&c, 8, 0), [0, 0, 0, 255], "source row 2");
        assert_eq!(pixel(&c, 8, 15), [255, 255, 255, 255], "source row 3");
    }

    /// The preview is in colour when a frame carries chroma and grey
    /// when it does not: a red 2 x 2 frame draws red, and the same luma
    /// without the chroma draws grey.
    #[test]
    fn a_frame_with_chroma_draws_in_colour() {
        // BT.601 limited-range red: Y 81, Cb 90, Cr 240.
        let luma = [81u8; 4];
        let mut c = canvas();
        c.luma(Rect::new(0, 0, 16, 16), 2, 2, &luma, Some(&[90, 240]));
        let [r, g, b, _] = pixel(&c, 8, 8);
        assert!(r > 200 && g < 60 && b < 60, "red, not grey: {r},{g},{b}");
        let mut c = canvas();
        c.luma(Rect::new(0, 0, 16, 16), 2, 2, &luma, None);
        let [r, g, b, _] = pixel(&c, 8, 8);
        assert_eq!((r, g, b), (81, 81, 81), "grey without chroma");
        // A chroma plane too short for the luma is ignored rather than
        // half-applied.
        let mut c = canvas();
        c.luma(Rect::new(0, 0, 16, 16), 2, 2, &luma, Some(&[90]));
        assert_eq!(pixel(&c, 8, 8), [81, 81, 81, 255]);
    }

    #[test]
    fn luma_honours_the_clip_and_refuses_a_short_buffer() {
        let mut c = canvas();
        c.clear(Color::BLACK);
        c.push_clip(Rect::new(0, 0, 4, 4));
        c.luma(Rect::new(0, 0, 16, 16), 2, 2, &[255, 255, 255, 255], None);
        c.pop_clip();
        assert_eq!(pixel(&c, 2, 2), [255, 255, 255, 255], "inside the clip");
        assert_eq!(pixel(&c, 8, 8), [0, 0, 0, 255], "outside it");
        c.luma(Rect::new(0, 0, 16, 16), 4, 4, &[255; 3], None);
        assert_eq!(
            pixel(&c, 8, 8),
            [0, 0, 0, 255],
            "a short buffer draws nothing"
        );
    }

    #[test]
    fn text_off_canvas_does_not_panic() {
        let mut c = canvas();
        c.text(-500, -500, "hello", Font::regular(16.0), Color::WHITE);
        c.text(500, 500, "hello", Font::regular(16.0), Color::WHITE);
        c.text_wrapped(
            Rect::new(60, 0, 10, 100),
            "a long text that wraps",
            Font::regular(16.0),
            Color::WHITE,
            TextAlign::Start,
        );
    }

    /// A card scrolled half out of its region looks like the whole card
    /// with the part outside the region cut away: the same pixels inside,
    /// nothing outside. Whether the shape crosses the region's top and
    /// bottom or its side, and for fills, strokes and circles alike. A
    /// card scrolled
    /// wholly out of the region leaves it as it was.
    #[test]
    fn a_page_put_back_again_is_the_page_wherever_a_sheet_drew() {
        // A sheet's frames: the kept page put back, then the sheet drawn
        // over it, a different sheet each frame.
        let mut c = canvas();
        c.clear(Color::rgb(30, 40, 50));
        c.fill_rounded_rect(Rect::new(4, 4, 40, 20), 5.0, Color::rgb(200, 90, 60));
        let page = c.snapshot();
        let frames: [&dyn Fn(&mut Canvas); 4] = [
            &|c| c.fill_rounded_rect(Rect::new(10, 6, 30, 18), 4.0, Color::WHITE),
            &|c| {
                c.shadow(
                    Rect::new(20, 8, 20, 12),
                    4.0,
                    6.0,
                    0.0,
                    2,
                    Color::BLACK.with_alpha(140),
                )
            },
            &|c| {
                c.text(2, 2, "Wg", Font::regular(14.0), Color::WHITE);
            },
            &|c| c.fill_rect(Rect::new(50, 0, 14, 32), Color::WHITE.with_alpha(90)),
        ];
        for (i, draw) in frames.iter().enumerate() {
            c.restore(&page);
            assert_eq!(c.frame().rgba, &page[..], "frame {i}: the page put back");
            draw(&mut c);
        }
        c.restore(&page);
        assert_eq!(
            c.frame().rgba,
            &page[..],
            "the last frame's sheet taken off"
        );
    }

    #[test]
    fn a_fade_part_of_the_way_is_each_pixel_part_of_the_way() {
        let mut c = canvas();
        c.clear(Color::rgb(10, 20, 30));
        let before = c.snapshot();
        c.fill_rect(Rect::new(20, 0, 10, 32), Color::rgb(210, 220, 230));
        let mut after = Vec::new();
        c.swap_pixels(&mut after);
        c.blend_between(&before, &after, 0.25);
        // Unchanged where the frames agree; a quarter of the way where
        // they do not.
        assert_eq!(pixel(&c, 5, 5), [10, 20, 30, 255]);
        assert_eq!(pixel(&c, 25, 5), [60, 70, 80, 255]);
    }

    #[test]
    fn a_menu_fading_in_is_part_of_the_way_over_what_it_stands_on_and_nowhere_else() {
        let mut c = canvas();
        c.clear(Color::rgb(10, 20, 30));
        let menu = Rect::new(16, 4, 24, 20);
        let mut before = Vec::new();
        c.copy_region(menu, &mut before);
        c.fill_rect(Rect::new(0, 0, 64, 32), Color::rgb(210, 220, 230));
        c.fade_region(menu, &before, 0.25);
        assert_eq!(pixel(&c, 20, 10), [60, 70, 80, 255], "a quarter of the way");
        assert_eq!(pixel(&c, 2, 2), [210, 220, 230, 255], "outside, as drawn");
        c.copy_region(menu, &mut before);
        c.fade_region(menu, &before, 0.6);
        assert_eq!(pixel(&c, 20, 10), [60, 70, 80, 255], "faded from itself");
    }

    #[test]
    fn a_fade_starts_on_the_old_frame_and_ends_on_the_new_one_as_drawn() {
        let mut c = canvas();
        c.clear(Color::rgb(10, 20, 30));
        c.text(2, 2, "Ab", Font::regular(14.0), Color::WHITE);
        let before = c.snapshot();
        c.clear(Color::rgb(240, 240, 235));
        c.fill_rounded_rect(Rect::new(20, 4, 30, 20), 6.0, Color::rgb(90, 160, 220));
        c.text(2, 2, "Wg", Font::regular(14.0), Color::BLACK);
        let drawn = c.snapshot();
        let mut after = Vec::new();
        c.swap_pixels(&mut after);
        assert_eq!(after, drawn, "the drawn frame is kept aside as it was");
        c.blend_between(&before, &after, 0.0);
        assert_eq!(c.frame().rgba, &before[..]);
        c.blend_between(&before, &after, 1.0);
        assert_eq!(c.frame().rgba, &drawn[..]);
    }

    #[test]
    fn a_shape_cut_by_a_clip_is_the_whole_shape_cut() {
        let region = Rect::new(8, 6, 48, 20);
        let shapes: [&dyn Fn(&mut Canvas); 7] = [
            // Wholly above, below and beside the region.
            &|c| c.fill_rounded_rect(Rect::new(12, 0, 30, 4), 2.0, Color::rgb(90, 160, 220)),
            &|c| c.fill_rounded_rect(Rect::new(12, 29, 40, 3), 1.0, Color::rgb(90, 160, 220)),
            &|c| c.stroke_rounded_rect(Rect::new(0, 10, 5, 10), 1.0, 1.0, Color::WHITE),
            // Across the top and bottom.
            &|c| c.fill_rounded_rect(Rect::new(12, 2, 30, 28), 6.5, Color::rgb(90, 160, 220)),
            &|c| {
                c.stroke_rounded_rect(
                    Rect::new(14, 1, 26, 12),
                    4.0,
                    1.5,
                    Color::rgb(240, 200, 120),
                )
            },
            // Across a side.
            &|c| c.fill_circle(54.3, 15.7, 7.2, Color::rgb(140, 210, 180)),
            // Inside.
            &|c| c.fill_circle(30.0, 16.0, 4.0, Color::WHITE),
        ];
        for (i, shape) in shapes.iter().enumerate() {
            let mut whole = canvas();
            whole.clear(Color::BLACK);
            shape(&mut whole);
            let mut cut = canvas();
            cut.clear(Color::BLACK);
            cut.push_clip(region);
            shape(&mut cut);
            cut.pop_clip();
            for y in 0..32 {
                for x in 0..64 {
                    let want = if region.contains(x, y) {
                        pixel(&whole, x, y)
                    } else {
                        [0, 0, 0, 255]
                    };
                    assert_eq!(pixel(&cut, x, y), want, "shape {i} at ({x}, {y})");
                }
            }
        }
    }
}
