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
use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::color::Color;
use crate::fonts::{Coverage, GlyphCache, SizedFace};
use crate::geom::{Dp, Rect, Scale, SizeClass};
pub use crate::shapes::ShapeKey;
use crate::shapes::{self, Shapes};
use crate::text::{self, Font, TextAlign, TextMetrics};

mod list;
pub use list::Band;
use list::{List, Mode};
mod surface;
pub(crate) use surface::Surface;

/// A drawing surface the size of the display.
pub struct Canvas {
    /// The visible rows, then [`hidden_rows`] below them that nothing
    /// reads: a shape running past the last visible row is drawn whole
    /// there, so its curves have the coverage the same shape has anywhere
    /// else on a page.
    pixmap: Pixmap,
    /// The visible rows.
    rows: u32,
    scale: Scale,
    clips: Vec<Rect>,
    /// Glyph bitmaps filled from the outlines, kept between frames.
    glyphs: GlyphCache,
    /// Shapes filled from their outlines, kept between frames
    /// ([`Canvas::fill_kept`]).
    shapes: Shapes,
    /// The pixels outside the clip put aside while a shape is drawn
    /// across the clip's edge.
    scratch: Vec<u8>,
    /// Emptied path builders kept for the next shape: a rounded shape or
    /// a circle takes one and gives it back, so a frame drawn again
    /// makes no path of its own.
    paths: Vec<PathBuilder>,
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
    /// The pixels of a region's content as its content left it, kept
    /// between frames so a frame reuses the room ([`Canvas::keep_layer`]).
    layer: Vec<u8>,
    damage: Option<Rect>,
    /// Notes the canvas's own costly work, a blur or a whole copy, for
    /// the application that budgets it ([`Canvas::set_counter`]); notes
    /// nothing until one is set.
    counter: fn(Work),
    /// Whether the painting calls paint, or are appended to the list
    /// ([`Canvas::record_calls`]).
    mode: Mode,
    /// The calls recorded and not yet painted.
    list: List,
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
        /// Whether some of its ink runs past the left or right side of
        /// its clip, so a person sees it cut off. A clip's top and foot
        /// are where a page scrolls and are not counted.
        cut: bool,
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
        let hidden = hidden_rows(scale);
        let pixmap = Pixmap::new(u32::from(width), u32::from(height) + hidden)
            .expect("display has non-zero size");
        Canvas {
            pixmap,
            rows: u32::from(height),
            scale,
            clips: Vec::new(),
            glyphs: GlyphCache::new(),
            shapes: Shapes::default(),
            scratch: Vec::new(),
            paths: Vec::new(),
            cut: Vec::new(),
            ink: None,
            lines: 0,
            kept: None,
            layer: Vec::new(),
            damage: None,
            counter: |_| {},
            mode: Mode::Direct,
            list: List::default(),
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

    /// How many shapes this canvas has filled from their outlines,
    /// counting from its making ([`Canvas::fill_kept`]'s misses): two
    /// counts taken around a frame drawn again say whether it filled
    /// what it could have kept. A lookup.
    pub fn outlines_filled(&self) -> u64 {
        self.shapes.filled()
    }

    /// The bytes of the visible rows, a prefix of the pixmap's.
    fn visible(&self) -> &[u8] {
        &self.pixmap.data()[..self.pixmap.width() as usize * self.rows as usize * 4]
    }

    fn visible_mut(&mut self) -> &mut [u8] {
        let n = self.pixmap.width() as usize * self.rows as usize * 4;
        &mut self.pixmap.data_mut()[..n]
    }

    /// Width in pixels.
    pub fn width(&self) -> i32 {
        self.pixmap.width() as i32
    }

    /// Height in pixels.
    pub fn height(&self) -> i32 {
        self.rows as i32
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
        debug_assert!(
            self.list.is_empty(),
            "calls recorded and not painted ([`Canvas::replay`])"
        );
        Frame {
            width: self.pixmap.width() as u16,
            height: self.rows as u16,
            rgba: self.visible(),
        }
    }

    /// Copies `frame`, another canvas's pixels, in with its top-left
    /// corner at (`x`, `y`), cut at this canvas's edges: an app's frame
    /// drawn inside another's (the Starter's held desktop app). A copy
    /// of its pixels, counted as a whole copy ([`Work::FullCanvas`]).
    pub fn place(&mut self, x: i32, y: i32, frame: &Frame<'_>) {
        self.sync();
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
        self.sync();
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
        if !self.replaying() {
            self.cut.clear();
            if let Some(ink) = &mut self.ink {
                ink.clear();
            }
        }
        if self.recording() {
            self.damaged(self.bounds());
            self.record_clear(color);
            return;
        }
        // The visible rows only: the hidden ones are drawn into and never
        // read, as a shape's pixels there depend on its coverage and
        // the colour it is blended over there, which nothing shows.
        let c = color.to_skia().premultiply().to_color_u8();
        let px = [c.red(), c.green(), c.blue(), c.alpha()];
        let n = self.pixmap.width() as usize * self.rows as usize * 4;
        for p in self.pixmap.data_mut()[..n].chunks_exact_mut(4) {
            p.copy_from_slice(&px);
        }
        self.damaged(self.bounds());
    }

    /// Starts or stops recording where text and icons leave their ink
    /// ([`Canvas::ink`]).
    pub fn record_ink(&mut self, on: bool) {
        self.ink = on.then(Vec::new);
    }

    /// Whether the ink is being recorded ([`Canvas::record_ink`]).
    pub fn recording_ink(&self) -> bool {
        self.ink.is_some()
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

    /// Where the strings [`Canvas::cut`] lists end now: a mark for
    /// [`Canvas::scrolled_across`]. A lookup.
    pub fn cut_mark(&self) -> usize {
        self.cut.len()
    }

    /// The strings cut since `mark` were drawn in a view that pans
    /// sideways: they are scrolled out of it, not cut off.
    pub fn scrolled_across(&mut self, mark: usize) {
        self.cut.truncate(mark);
    }

    /// Notes a shape drawn as `kind` in `rect`, pixels, while recording
    /// is on: one a person reads as a whole, such as a progress
    /// indicator, which its fills alone do not say.
    pub fn note(&mut self, kind: InkKind, rect: Rect) {
        let clip = self.clip();
        self.note_ink(|| kind, Some(rect.intersect(&clip)));
    }

    fn note_ink(&mut self, kind: impl FnOnce() -> InkKind, rect: Option<Rect>) {
        if self.replaying() {
            return;
        }
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
        if self.recording() {
            self.record_rect(rect, color);
            return;
        }
        let clip = self.clip();
        surface::fill_rect(&mut self.surface(), rect, clip, color);
    }

    /// Costs a write of each dark pixel. Fills in `color` the cells of an
    /// `n` by `n` grid of squares `cell` pixels wide, its top-left at
    /// `at`, for which `dark(column, row)` holds: a QR code's modules,
    /// each row's runs written straight into the pixels, the same pixels
    /// as a [`Canvas::fill_rect`] of each run. Nothing is kept.
    pub fn fill_cells(
        &mut self,
        at: (i32, i32),
        (n, cell): (usize, i32),
        dark: impl Fn(usize, usize) -> bool,
        color: Color,
    ) {
        let side = n as i32 * cell;
        let area = Rect::new(at.0, at.1, side, side).intersect(&self.clip());
        if area.is_empty() || cell <= 0 || color.a == 0 {
            return;
        }
        if self.recording() {
            self.damaged(area);
            self.record_cells(at, (n, cell), dark, color);
            return;
        }
        let clip = self.clip();
        let done = surface::fill_cells(&mut self.surface(), clip, (at, n, cell), dark, color);
        self.damaged(done);
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
        let of = surface::ShadowOf {
            rect,
            radius,
            blur,
            spread,
            dy,
        };
        let area = of.reach().1.intersect(&self.clip());
        if area.is_empty() {
            return;
        }
        self.damaged(area);
        if self.recording() {
            self.record_shadow((rect, radius, blur), spread, dy, color);
            return;
        }
        let clip = self.clip();
        surface::shadow(&mut self.surface(), clip, of, color, &mut Vec::new());
    }

    /// Material 3's shadow of an elevation `level` (1 to 5) under a
    /// rounded rectangle that floats: a key layer and an ambient layer
    /// ([`crate::tokens::ELEVATION_KEY`], [`crate::tokens::ELEVATION_AMBIENT`]),
    /// each in black at its opacity, their dp sizes times `scale`. Costs
    /// two [`Canvas::shadow`] passes, each over a ring a few pixels wide.
    pub fn elevation(&mut self, rect: Rect, radius: f32, level: u8, scale: f32) {
        self.elevation_faded(rect, radius, level, scale, 1.0);
    }

    /// Costs two [`Canvas::shadow`] passes. [`Canvas::elevation`] at
    /// `strength` (0 to 1) of its opacity: the shadow of something fading
    /// in or out.
    pub fn elevation_faded(
        &mut self,
        rect: Rect,
        radius: f32,
        level: u8,
        scale: f32,
        strength: f32,
    ) {
        let i = usize::from(level.clamp(1, 5)) - 1;
        let k = strength.clamp(0.0, 1.0);
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
                Color::BLACK.with_alpha((f32::from(opacity) * k + 0.5) as u8),
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
        let _ = (!self.replaying()).then(|| (self.counter)(Work::Blur));
        if self.recording() {
            self.record_frost(scale, radius, tint);
            return;
        }
        self.damaged(self.bounds());
        let (w, h) = (self.pixmap.width() as usize, self.rows as usize);
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
        let data = self.visible();
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
        self.sync();
        (self.counter)(Work::FullCanvas);
        if pixels.len() != self.visible().len() {
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
        self.visible_mut().copy_from_slice(pixels);
        self.kept = Some(id);
        self.damage = None;
    }

    /// A copy of every pixel, for [`Canvas::blend_between`] to fade from.
    pub fn snapshot(&mut self) -> Vec<u8> {
        self.sync();
        (self.counter)(Work::FullCanvas);
        self.visible().to_vec()
    }

    /// The same copy into `into`, reusing its room.
    pub fn snapshot_into(&mut self, into: &mut Vec<u8>) {
        self.sync();
        (self.counter)(Work::FullCanvas);
        into.clear();
        into.extend_from_slice(self.visible());
    }

    /// Shows `t` (0 to 1) of the way from `before` to `after`, two
    /// [`Canvas::snapshot`]s of this canvas, without drawing either
    /// again: a step of a cross-fade whose new frame is kept aside
    /// ([`Canvas::swap_pixels`]). Every pixel is written; snapshots of
    /// another size leave the canvas as it is.
    pub fn blend_between(&mut self, before: &[u8], after: &[u8], t: f32) {
        let mut bands = Vec::new();
        self.blend_noting(before, after, t, 0, &mut bands);
    }

    /// Costs a pass over the whole canvas. [`Canvas::blend_between`],
    /// which also sets `bands` to rectangles that hold every pixel where
    /// `before` and `after` differ, but for a ring `inset` pixels wide
    /// round the canvas's edge, which whoever holds the canvas draws over
    /// every frame and is never noted: for the fade's later steps to blend
    /// those alone ([`Canvas::blend_within`]). Differences are found 16
    /// pixels at a time from the ring's inner edge, so a band may hold a
    /// few pixels that agree; rows next to each other whose differences
    /// span about the same columns share a band.
    pub fn blend_noting(
        &mut self,
        before: &[u8],
        after: &[u8],
        t: f32,
        inset: i32,
        bands: &mut Vec<Rect>,
    ) {
        self.sync();
        (self.counter)(Work::FullCanvas);
        bands.clear();
        let n = self.visible().len();
        if before.len() != n || after.len() != n {
            return;
        }
        // Each channel of a premultiplied pixel on its own, in 16 bits:
        // 255 x 256 fits, and a loop over bytes is one the compiler runs
        // many lanes at a time. A frosted page under a sheet differs
        // from the page before it everywhere, so most pixels of such a
        // fade are blended.
        let t = (t.clamp(0.0, 1.0) * 256.0) as u16;
        self.damaged(self.bounds());
        let (w, h) = (self.pixmap.width() as usize, self.rows as usize);
        let ring = usize::try_from(inset.max(0))
            .unwrap_or(0)
            .min(w / 2)
            .min(h / 2);
        let stride = w * 4;
        let data = self.pixmap.data_mut();
        let mut spans = Bands::default();
        for y in 0..h {
            let at = y * stride;
            let (now, was, will) = (
                &mut data[at..at + stride],
                &before[at..at + stride],
                &after[at..at + stride],
            );
            if y < ring || y >= h - ring {
                blend_runs(now, was, will, t, true);
                continue;
            }
            let (l, r) = (ring * 4, stride - ring * 4);
            blend_runs(&mut now[..l], &was[..l], &will[..l], t, true);
            blend_runs(&mut now[r..], &was[r..], &will[r..], t, true);
            let span = blend_runs(&mut now[l..r], &was[l..r], &will[l..r], t, true);
            spans.row(y, span.map(|(a, b)| (a + ring, b + ring)));
        }
        spans.into_rects(bands);
    }

    /// Costs a pass over the pixels of `bands`. The next step of a fade
    /// [`Canvas::blend_noting`] began: `t` of the way from `before` to
    /// `after` inside `bands`, the rectangles that call noted, the rest
    /// left as it is, where the two agree and the canvas already shows
    /// them (or the ring it left out, which is drawn over). Snapshots of
    /// another size leave the canvas as it is.
    pub fn blend_within(&mut self, before: &[u8], after: &[u8], t: f32, bands: &[Rect]) {
        self.sync();
        (self.counter)(Work::FullCanvas);
        let n = self.visible().len();
        if before.len() != n || after.len() != n {
            return;
        }
        let t = (t.clamp(0.0, 1.0) * 256.0) as u16;
        let stride = self.pixmap.width() as usize * 4;
        for band in bands {
            let b = band.intersect(&self.bounds());
            if b.is_empty() {
                continue;
            }
            self.damaged(b);
            let data = self.pixmap.data_mut();
            for y in b.y..b.bottom() {
                let at = y as usize * stride + b.x as usize * 4;
                let end = at + b.w as usize * 4;
                blend_runs(
                    &mut data[at..end],
                    &before[at..end],
                    &after[at..end],
                    t,
                    false,
                );
            }
        }
    }

    /// Exchanges the canvas's pixels with `pixels`, a
    /// [`Canvas::snapshot`]'s worth or empty: the frame just drawn is
    /// kept aside without a copy, and the canvas holds what `pixels`
    /// held, or the page colour's zeros when it was empty, until it is
    /// drawn over. Pixels of another size leave both as they are.
    pub fn swap_pixels(&mut self, pixels: &mut Vec<u8>) {
        self.sync();
        let (w, h) = (self.pixmap.width(), self.pixmap.height());
        let n = self.visible().len();
        let total = w as usize * h as usize * 4;
        let mut given = if pixels.len() == n {
            core::mem::take(pixels)
        } else if pixels.is_empty() {
            alloc::vec![0; n]
        } else {
            return;
        };
        // The hidden rows go on the end, and come off the canvas's own
        // pixels again before they are handed back.
        given.resize(total, 0);
        let Some(next) = tiny_skia::IntSize::from_wh(w, h).and_then(|s| Pixmap::from_vec(given, s))
        else {
            return;
        };
        *pixels = core::mem::replace(&mut self.pixmap, next).take();
        pixels.truncate(n);
        self.kept = None;
        self.damaged(self.bounds());
    }

    /// Costs a copy of the rectangle's pixels. The pixels inside `rect`,
    /// cut at the canvas's edges, a row at a time into `into`, reusing its
    /// room: for [`Canvas::fade_region`] to fade from.
    pub fn copy_region(&self, rect: Rect, into: &mut Vec<u8>) {
        debug_assert!(
            self.list.is_empty(),
            "calls recorded and not painted ([`Canvas::replay`])"
        );
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

    /// Costs a copy of the rectangle's pixels. Keeps the pixels inside
    /// `rect` as the region's content layer, in room kept between frames
    /// ([`Canvas::layer`]).
    pub fn keep_layer(&mut self, rect: Rect) {
        self.sync();
        let mut layer = core::mem::take(&mut self.layer);
        let r = rect.intersect(&self.bounds());
        let held = layer.len();
        let needs = if r.is_empty() {
            0
        } else {
            (r.w * r.h * 4) as usize
        };
        // Room that grows moves: the room left behind holds nothing.
        if needs > layer.capacity() {
            layer.fill(0);
        }
        self.copy_region(rect, &mut layer);
        // A smaller region leaves nothing of the last one past its end.
        if layer.len() < held {
            let len = layer.len();
            layer.resize(held, 0);
            layer.truncate(len);
        }
        self.layer = layer;
    }

    /// Costs a pass over the layer's pixels, when it holds any. Overwrites
    /// the content layer and empties it, keeping the room: a frame that
    /// took none leaves nothing of an earlier page, a secret shown on it
    /// included (F195).
    pub fn drop_layer(&mut self) {
        if !self.layer.is_empty() {
            self.layer.fill(0);
            core::hint::black_box(&mut self.layer);
            self.layer.clear();
        }
    }

    /// The pixels [`Canvas::keep_layer`] kept, a row at a time.
    pub fn layer(&self) -> &[u8] {
        &self.layer
    }

    /// Costs a copy of the rectangle's pixels. Puts the content layer
    /// ([`Canvas::keep_layer`] of `rect`) back inside `rect` moved by `dy`
    /// rows, down for a positive `dy` and up for a negative one, ignoring
    /// the clip: the rows `dy` brought into `rect` hold what was there, for
    /// the caller to have drawn. A layer of another size puts nothing
    /// back.
    ///
    /// `margin` rows next to the rows `dy` brought in are left as they
    /// are: the caller drew them again, for ink that reaches past its shape.
    pub fn paste_layer(&mut self, rect: Rect, dy: i32, margin: i32) {
        self.sync();
        let r = rect.intersect(&self.bounds());
        if r != rect
            || r.is_empty()
            || dy.abs() >= r.h
            || self.layer.len() != (r.w * r.h * 4) as usize
        {
            return;
        }
        self.damaged(r);
        let stride = self.pixmap.width() as usize * 4;
        let n = r.w as usize * 4;
        let data = self.pixmap.data_mut();
        // Row `k` of the layer lands on row `k + dy` of the rectangle.
        let margin = margin.max(0);
        let (from, to) = if dy >= 0 { (0, r.h - dy) } else { (-dy, r.h) };
        // The rows `margin` away from the brought-in rows are kept as drawn.
        let (from, to) = if dy >= 0 {
            (from + margin, to)
        } else {
            (from, to - margin)
        };
        for k in from..to {
            let at = (r.y + k + dy) as usize * stride + r.x as usize * 4;
            let src = k as usize * n;
            data[at..at + n].copy_from_slice(&self.layer[src..src + n]);
        }
    }

    /// Costs a pass over the rectangle's pixels. Shows what was drawn
    /// inside `rect` since `before` was taken there ([`Canvas::copy_region`])
    /// only `t` (0 to 1) of the way from it: something that fades in or
    /// out over what it stands on. A `before` of another size leaves the
    /// canvas as it is.
    pub fn fade_region(&mut self, rect: Rect, before: &[u8], t: f32) {
        self.sync();
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

    /// Costs a pass over the rectangle's pixels. Draws what is inside
    /// `rect` as Material's disabled content: each pixel only the share
    /// `keep` of the way from the colour of the canvas at `(x, y)`, the
    /// background the control stands on, toward what was drawn. The
    /// background itself, and a container the same colour, do not move.
    pub fn fade_to_background(&mut self, rect: Rect, (x, y): (i32, i32), keep: f32) {
        self.sync();
        let r = rect.intersect(&self.bounds());
        if r.is_empty() || !self.bounds().contains(x, y) {
            return;
        }
        let stride = self.pixmap.width() as usize * 4;
        let keep = (keep.clamp(0.0, 1.0) * 256.0) as i32;
        let data = self.pixmap.data_mut();
        let at = y as usize * stride + x as usize * 4;
        let bg = [data[at], data[at + 1], data[at + 2], data[at + 3]];
        let n = r.w as usize * 4;
        for row in r.y..r.bottom() {
            let from = row as usize * stride + r.x as usize * 4;
            for px in data[from..from + n].chunks_exact_mut(4) {
                for (c, &b) in px.iter_mut().zip(&bg) {
                    let (b, v) = (i32::from(b), i32::from(*c));
                    *c = (b + (((v - b) * keep) >> 8)).clamp(0, 255) as u8;
                }
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
        if self.recording() {
            self.record_shift(rect, dy);
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
        self.damaged(dst);
        if self.recording() {
            self.record_luma(rect, (width, height), pixels, chroma);
            return;
        }
        surface::luma(&mut self.surface(), (rect, dst), (sw, sh), pixels, chroma);
    }

    /// Fills a rectangle with rounded corners of `radius` pixels.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        if rect.is_empty() || color.a == 0 || rect.intersect(&self.clip()).is_empty() {
            return;
        }
        if self.recording() {
            self.record_rounded(rect, radius, color);
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
                let ends = rounded_ends_path(&mut self.paths, rect, radius, (cut, top));
                self.draw_path(&ends, &paint, None);
                self.give_back(ends);
            }
            self.fill_rect(mid, color);
            return;
        }
        let path = rounded_rect_path(&mut self.paths, rect, radius, None);
        self.draw_path(&path, &paint, None);
        self.give_back(path);
    }

    /// Keeps `path`'s room for the next shape's path.
    fn give_back(&mut self, path: tiny_skia::Path) {
        give_back(&mut self.paths, path);
    }

    /// Strokes a rounded rectangle outline `width` pixels thick, inside
    /// the rect.
    pub fn stroke_rounded_rect(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        if rect.is_empty() || color.a == 0 || rect.intersect(&self.clip()).is_empty() {
            return;
        }
        if self.recording() {
            self.record_stroke_rounded(rect, radius, width, color);
            return;
        }
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        let path = rounded_rect_path(&mut self.paths, rect, radius, Some(&stroke));
        self.draw_path(&path, &Self::paint(color), None);
        self.give_back(path);
    }

    /// Draws a line `width` pixels thick between two points.
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, color: Color) {
        if color.a == 0 {
            return;
        }
        if self.recording() {
            self.record_line([x0, y0, x1, y1], width, color);
            return;
        }
        let Some(path) = line_path([x0, y0, x1, y1]) else {
            return;
        };
        self.draw_path(&path, &Self::paint(color), Some(&line_stroke(width)));
    }

    /// Strokes the closed outline through `points`, in pixels, `width`
    /// pixels thick, its corners rounded.
    pub fn stroke_closed(&mut self, points: &[(f32, f32)], width: f32, color: Color) {
        if points.is_empty() || color.a == 0 {
            return;
        }
        if color.a == 0 {
            return;
        }
        if self.recording() {
            self.record_stroke_closed(points, width, color);
            return;
        }
        let Some(path) = closed_path(points) else {
            return;
        };
        self.draw_path(&path, &Self::paint(color), Some(&closed_stroke(width)));
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
        if self.recording() {
            self.record_circle((cx, cy), radius, color);
            return;
        }
        let Some(path) = circle_path(&mut self.paths, (cx, cy), radius) else {
            return;
        };
        self.draw_path(&path, &Self::paint(color), None);
        self.give_back(path);
    }

    /// Fills a shape made of closed outlines through the given points, in
    /// pixels. An outline wound the other way round from the one around
    /// it is a hole.
    pub fn fill_contours(&mut self, contours: &[&[(f32, f32)]], color: Color) {
        if color.a == 0 {
            return;
        }
        if self.recording() {
            self.record_contours(contours, color);
            return;
        }
        self.fill_contours_of(contours.iter().copied(), color);
    }

    /// [`Canvas::fill_contours`] of contours given one at a time.
    fn fill_contours_of<'a>(
        &mut self,
        contours: impl Iterator<Item = &'a [(f32, f32)]>,
        color: Color,
    ) {
        let Some(path) = contours_path(contours) else {
            return;
        };
        self.draw_path(&path, &Self::paint(color), None);
    }

    /// Costs filling the outlines the first time `key` is drawn, then a
    /// pass over the shape's box. Fills the closed outlines `outlines`
    /// gives, in pixels, in `color`, as [`Canvas::fill_contours`] does,
    /// the same pixels; their coverage is kept under `key` and drawn
    /// again from it while the key is the same, `outlines` not called.
    /// The key names everything the outlines are worked out from
    /// ([`ShapeKey`]); the colour is applied at each draw. An app's name
    /// and its mark, drawn on every frame, are filled once.
    pub fn fill_kept(
        &mut self,
        key: ShapeKey,
        outlines: impl FnOnce() -> Vec<Vec<(f32, f32)>>,
        color: Color,
    ) {
        if color.a == 0 {
            return;
        }
        if self.recording() {
            self.record_kept(key, outlines, color);
            return;
        }
        if self.replaying() {
            // Kept when it was recorded, and nothing is dropped between.
            debug_assert!(
                self.shapes.area(key).is_some(),
                "a kept shape was dropped between record and replay"
            );
        } else {
            self.shapes.keep(key, outlines);
        }
        let clip = self.clip();
        let mut surface = Surface::new(self.pixmap.as_mut(), 0);
        let drawn = shapes::draw(&self.shapes, &mut surface, clip, key, color);
        if let Some(r) = drawn {
            self.damaged(r);
        }
    }

    /// Fills `path`, or strokes it with `stroke`, inside the clip
    /// (`surface::path`).
    fn draw_path(&mut self, path: &tiny_skia::Path, paint: &Paint, stroke: Option<&Stroke>) {
        let ink = surface::ink_box(path, stroke);
        let clip = self.clip();
        self.damaged(ink.intersect(&clip));
        let visible = self.rows as i32;
        let mut s = Surface::new(self.pixmap.as_mut(), 0);
        let args = (path, paint, stroke);
        surface::path(
            &mut s,
            &mut self.scratch,
            (clip, visible),
            args,
            ink,
            Transform::identity(),
        );
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
        let rec = self.recording();
        if !self.replaying() {
            self.lines += 1;
        }
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
            if !rec {
                let mut surface = Surface::new(self.pixmap.as_mut(), 0);
                surface::blit_glyph(
                    &mut surface,
                    clip,
                    (gx, gy, g.width, g.height),
                    g.bitmap,
                    color,
                );
            }
            pen += i64::from(g.advance_64);
        }
        // Every pixel its glyphs can reach: a line box above and below,
        // and a line's height either side for a mark past its advance.
        let (from, to) = (((start + 32) >> 6) as i32, ((pen + 32) >> 6) as i32);
        self.damaged(Rect::new(from - lh, y - lh, to - from + 2 * lh, 3 * lh).intersect(&clip));
        if rec {
            self.record_text(face, (x, y), s, color);
        }
        // Once per string: a scroll draws the same strip again and
        // again without a clear in between.
        if !self.replaying() && cut && !self.cut.iter().any(|t| t == s) {
            self.cut.push(String::from(s));
        }
        let family = face.family;
        self.note_ink(
            || InkKind::Text {
                text: String::from(s),
                family,
                color,
                cut,
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
        let rec = self.recording();
        let Some(g) = self.glyphs.glyph(&face, icon.code_point()) else {
            return;
        };
        let x = rect.x + (rect.w - g.width as i32) / 2;
        let y = rect.y + (rect.h - g.height as i32) / 2;
        let inked = Rect::new(x, y, g.width as i32, g.height as i32).intersect(&clip);
        if rec {
            self.record_icon(rect, icon, size_dp, color);
        } else {
            let mut surface = Surface::new(self.pixmap.as_mut(), 0);
            surface::blit_glyph(
                &mut surface,
                clip,
                (x, y, g.width, g.height),
                g.bitmap,
                color,
            );
        }
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
/// Bytes a fade compares at a time, 16 pixels: where the two frames
/// agree over a run, the new one is copied, most of a change of screen
/// (the sidebar, the page's ground).
const RUN: usize = 4 * 16;

/// Costs a pass over the pixels. `now` set `t` (of 256) of the way from
/// `was` to `will`, three runs of the same pixels, [`RUN`] bytes at a
/// time; returns the first pixel and the end of the runs where `was` and
/// `will` differ, if any do. Where they agree `will` is copied only with
/// `agreeing`: a later step's canvas already shows it there.
fn blend_runs(
    now: &mut [u8],
    was: &[u8],
    will: &[u8],
    t: u16,
    agreeing: bool,
) -> Option<(usize, usize)> {
    let u = 256 - t;
    let mut span: Option<(usize, usize)> = None;
    let runs = now
        .chunks_mut(RUN)
        .zip(was.chunks(RUN))
        .zip(will.chunks(RUN))
        .enumerate();
    for (i, ((now, was), will)) in runs {
        if was == will {
            if agreeing {
                now.copy_from_slice(will);
            }
            continue;
        }
        let px = (i * RUN / 4, (i * RUN + will.len()) / 4);
        span = Some(span.map_or(px, |(a, _)| (a, px.1)));
        if t == 256 {
            now.copy_from_slice(will);
            continue;
        }
        for ((now, &w), &n) in now.iter_mut().zip(was).zip(will) {
            *now = ((u16::from(w) * u + u16::from(n) * t) >> 8) as u8;
        }
    }
    span
}

/// The bands [`Canvas::blend_noting`] gathers a row at a time, as columns
/// `[x0, x1)` and rows `[y0, y1)`.
#[derive(Default)]
struct Bands(Vec<(usize, usize, usize, usize)>);

impl Bands {
    /// At most this many bands: past it, bands next to each other are
    /// joined two by two.
    const MOST: usize = 256;
    /// A row joins the band above it when each edge of its columns is
    /// within this many pixels of the band's.
    const SLACK: usize = 64;

    /// Row `y` differs over the columns of `span`, or nowhere.
    fn row(&mut self, y: usize, span: Option<(usize, usize)>) {
        let Some((a, b)) = span else {
            return;
        };
        if let Some(last) = self.0.last_mut()
            && last.3 == y
            && a.abs_diff(last.0) <= Self::SLACK
            && b.abs_diff(last.1) <= Self::SLACK
        {
            *last = (last.0.min(a), last.1.max(b), last.2, y + 1);
            return;
        }
        self.0.push((a, b, y, y + 1));
    }

    /// The bands as rectangles into `out`.
    fn into_rects(mut self, out: &mut Vec<Rect>) {
        while self.0.len() > Self::MOST {
            self.0 = self
                .0
                .chunks(2)
                .map(|p| {
                    p.iter().skip(1).fold(p[0], |a, b| {
                        (a.0.min(b.0), a.1.max(b.1), a.2.min(b.2), a.3.max(b.3))
                    })
                })
                .collect();
        }
        out.extend(self.0.into_iter().map(|(x0, x1, y0, y1)| {
            Rect::new(x0 as i32, y0 as i32, (x1 - x0) as i32, (y1 - y0) as i32)
        }));
    }
}

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

impl Canvas {
    /// The canvas's pixels as a surface.
    fn surface(&mut self) -> Surface<'_> {
        Surface::new(self.pixmap.as_mut(), 0)
    }
}

/// The stroke of a line `width` pixels thick.
fn line_stroke(width: f32) -> Stroke {
    Stroke {
        width,
        ..Stroke::default()
    }
}

/// The stroke of a closed outline `width` pixels thick, its corners
/// rounded.
fn closed_stroke(width: f32) -> Stroke {
    Stroke {
        width,
        line_join: tiny_skia::LineJoin::Round,
        ..Stroke::default()
    }
}

/// The path of a line between two points.
fn line_path([x0, y0, x1, y1]: [f32; 4]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    pb.line_to(x1, y1);
    pb.finish()
}

/// The path of the closed outline through `points`.
fn closed_path(points: &[(f32, f32)]) -> Option<tiny_skia::Path> {
    let (&(x0, y0), rest) = points.split_first()?;
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    for &(x, y) in rest {
        pb.line_to(x, y);
    }
    pb.close();
    pb.finish()
}

/// The path of a shape made of closed outlines.
fn contours_path<'a>(contours: impl Iterator<Item = &'a [(f32, f32)]>) -> Option<tiny_skia::Path> {
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
    pb.finish()
}

/// The path of a circle: built about its own whole pixel and placed, as a
/// rounded rectangle is, so a circle moved by whole pixels is the same
/// pixels.
fn circle_path(
    pool: &mut Vec<PathBuilder>,
    (cx, cy): (f32, f32),
    radius: f32,
) -> Option<tiny_skia::Path> {
    let down = |v: f32| (v as i32 - i32::from((v as i32 as f32) > v)) as f32;
    let (ox, oy) = (down(cx), down(cy));
    // Eight quadratic curves in one contour.
    let mut local = take(pool);
    local.push_circle(cx - ox, cy - oy, radius);
    let local = local.finish()?;
    let path = place(pool, &local, (ox as i32, oy as i32));
    give_back(pool, local);
    Some(path)
}

/// A path builder that cuts each point to the 1/256 pixel the
/// rasterizer cuts them to and adds a whole-pixel place as it goes, so
/// a shape built about its own origin and moved is made once, with no
/// path in between. With no place it adds the points as given.
struct Builder {
    pb: PathBuilder,
    at: Option<(i32, i32)>,
}

impl Builder {
    fn new(pool: &mut Vec<PathBuilder>, at: Option<(i32, i32)>) -> Self {
        Builder { pb: take(pool), at }
    }

    fn pt(&self, x: f32, y: f32) -> (f32, f32) {
        match self.at {
            Some((ox, oy)) => (cut(x, ox), cut(y, oy)),
            None => (x, y),
        }
    }

    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.pt(x, y);
        self.pb.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.pt(x, y);
        self.pb.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x1, y1) = self.pt(x1, y1);
        let (x, y) = self.pt(x, y);
        self.pb.quad_to(x1, y1, x, y);
    }

    fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x1, y1) = self.pt(x1, y1);
        let (x2, y2) = self.pt(x2, y2);
        let (x, y) = self.pt(x, y);
        self.pb.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.pb.close();
    }

    fn finish(self) -> tiny_skia::Path {
        self.pb.finish().expect("non-degenerate path")
    }
}

/// An emptied builder from `pool`, or a new one sized for a rounded
/// shape or a circle: ten verbs and seventeen points.
fn take(pool: &mut Vec<PathBuilder>) -> PathBuilder {
    pool.pop()
        .unwrap_or_else(|| PathBuilder::with_capacity(10, 17))
}

/// Keeps the room of `path` in `pool`, a few builders at most.
fn give_back(pool: &mut Vec<PathBuilder>, path: tiny_skia::Path) {
    if pool.len() < 4 {
        pool.push(path.clear());
    }
}

/// `v` cut down to a 1/256 pixel, after `o` whole pixels.
fn cut(v: f32, o: i32) -> f32 {
    let n = v * 256.0;
    let down = n as i32 - i32::from((n as i32 as f32) > n);
    o as f32 + down as f32 / 256.0
}

/// A rounded-rectangle path, or its outline when `stroke` is given (the
/// path then inset by half the stroke's width), placed at `rect`.
fn rounded_rect_path(
    pool: &mut Vec<PathBuilder>,
    rect: Rect,
    radius: f32,
    stroke: Option<&Stroke>,
) -> tiny_skia::Path {
    let inset = stroke.map_or(0.0, |s| s.width / 2.0);
    let l = inset;
    let t = inset;
    let r = rect.w as f32 - inset;
    let b = rect.h as f32 - inset;
    let radius = radius.min((r - l) / 2.0).min((b - t) / 2.0).max(0.0);
    // A fill is cut and placed as it is built; a stroke's outline is made
    // from the path about the origin, then placed.
    let mut pb = Builder::new(pool, stroke.is_none().then_some((rect.x, rect.y)));
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
    let path = pb.finish();
    let Some(s) = stroke else { return path };
    let outline = path.stroke(s, 1.0).expect("a stroke has an outline");
    give_back(pool, path);
    let placed = place(pool, &outline, (rect.x, rect.y));
    give_back(pool, outline);
    placed
}

/// The top or the bottom of [`rounded_rect_path`]'s outline with no
/// inset, its two corners and the side between them, down or up to the
/// row `cut.0` (`cut.1` true for the top) and closed straight across
/// it. Within its rows its edges are the whole outline's, so it fills
/// the same pixels there.
fn rounded_ends_path(
    pool: &mut Vec<PathBuilder>,
    rect: Rect,
    radius: f32,
    (cut, top): (i32, bool),
) -> tiny_skia::Path {
    let l = 0.0;
    let t = 0.0;
    let r = rect.w as f32;
    let b = rect.h as f32;
    let radius = radius.min((r - l) / 2.0).min((b - t) / 2.0).max(0.0);
    let k = 0.5523 * radius;
    let c = (cut - rect.y) as f32;
    let mut pb = Builder::new(pool, Some((rect.x, rect.y)));
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
    pb.finish()
}

/// Rows below the visible ones that a canvas draws into and never shows
/// (`docs/FRAMES.md` §3.2): the tallest curve a shape has, a corner of
/// 28 dp (`SHAPE_XL`) or a shadow's 24 dp reach (`SHADOW_REACH`) beyond
/// it, and a margin. A path the clip lets through that runs past the
/// last visible row is drawn whole, not cut there, so a curve across
/// that row has the coverage it has anywhere on a page.
const HIDDEN_DP: f32 = 64.0;

fn hidden_rows(scale: Scale) -> u32 {
    scale.px(Dp(HIDDEN_DP)).max(0) as u32
}

/// `path` with every point cut to the 1/256 pixel the rasterizer cuts
/// them to and moved by `at` whole pixels, which is exact: the same shape
/// at another whole-pixel place is the same pixels, as the move of a
/// scroll step needs (`docs/FRAMES.md` §3.2). Built and stroked at its
/// place, floating-point rounding would differ with how large the place's
/// numbers are, and a curve's edge pixel with it.
fn place(pool: &mut Vec<PathBuilder>, path: &tiny_skia::Path, at: (i32, i32)) -> tiny_skia::Path {
    use tiny_skia::PathSegment;
    let mut pb = Builder::new(pool, Some(at));
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(p) => pb.move_to(p.x, p.y),
            PathSegment::LineTo(p) => pb.line_to(p.x, p.y),
            PathSegment::QuadTo(c, p) => pb.quad_to(c.x, c.y, p.x, p.y),
            PathSegment::CubicTo(c1, c2, p) => pb.cubic_to(c1.x, c1.y, c2.x, c2.y, p.x, p.y),
            PathSegment::Close => pb.close(),
        }
    }
    pb.finish()
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
                        let path = rounded_rect_path(&mut Vec::new(), rect, radius, None);
                        c.draw_path(&path, &Canvas::paint(color), None);
                    } else {
                        let stroke = Stroke {
                            width,
                            ..Stroke::default()
                        };
                        let path = rounded_rect_path(&mut Vec::new(), rect, radius, Some(&stroke));
                        c.draw_path(&path, &Canvas::paint(color), None);
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
    fn a_fades_later_steps_over_the_bands_that_differ_show_what_a_whole_blend_shows() {
        let mut c = canvas();
        c.clear(Color::rgb(10, 20, 30));
        c.fill_rect(Rect::new(0, 0, 64, 2), Color::rgb(1, 2, 3));
        let before = c.snapshot();
        c.clear(Color::rgb(10, 20, 30));
        // The new screen differs in two places; the ring two pixels wide
        // round the edge differs too, and is drawn over by its holder.
        c.fill_rect(Rect::new(20, 4, 10, 6), Color::rgb(210, 220, 230));
        c.fill_rect(Rect::new(40, 20, 6, 8), Color::rgb(90, 0, 200));
        let mut after = Vec::new();
        c.swap_pixels(&mut after);
        let mut bands = Vec::new();
        c.blend_noting(&before, &after, 0.25, 2, &mut bands);
        assert!(!bands.is_empty());
        for b in &bands {
            assert!(b.y >= 2 && b.bottom() <= 30, "the ring is left out: {b:?}");
        }
        let within = |x, y| bands.iter().any(|b| b.contains(x, y));
        assert!(
            within(25, 6) && within(42, 24),
            "each difference is in a band"
        );
        assert!(!within(5, 15) && !within(60, 15), "where they agree is not");
        for t in [0.5, 0.75, 1.0] {
            c.blend_within(&before, &after, t, &bands);
            let mut whole = canvas();
            whole.swap_pixels(&mut before.clone());
            whole.blend_between(&before, &after, t);
            for (y, x) in (2..30).flat_map(|y| (2..62).map(move |x| (y, x))) {
                assert_eq!(pixel(&c, x, y), pixel(&whole, x, y), "at {x}, {y}, t {t}");
            }
        }
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

    /// A rounded shape drawn at another whole-pixel place is the same
    /// pixels moved, wherever on a long page it stands, so a scroll step
    /// can move the rows already drawn (`docs/FRAMES.md` §3.2).
    #[test]
    fn a_rounded_shape_moved_by_whole_pixels_is_the_same_pixels() {
        let big = || {
            Canvas::new(&DisplayInfo {
                width: 200,
                height: 1200,
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
        let draw = |c: &mut Canvas, y: i32| {
            c.stroke_rounded_rect(Rect::new(20, y, 120, 44), 22.0, 2.0, Color::rgb(20, 40, 80));
            c.fill_rounded_rect(
                Rect::new(30, y + 100, 120, 70),
                17.5,
                Color::rgb(90, 160, 220),
            );
        };
        let mut base = big();
        base.clear(Color::WHITE);
        draw(&mut base, 100);
        let mut bad = Vec::new();
        for y in 101..1000 {
            let mut c = big();
            c.clear(Color::WHITE);
            draw(&mut c, y);
            let d = y - 100;
            let mut n = 0;
            for yy in 90..300 {
                for x in 0..200 {
                    if yy + d < 1200 && pixel(&base, x, yy) != pixel(&c, x, yy + d) {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                bad.push((y, n));
            }
        }
        assert!(bad.is_empty(), "places where the pixels differ: {bad:?}");
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

    /// A frame drawn in bands, each through a clip one band deep, is the
    /// frame drawn whole, whatever band cuts a shape, a glyph or a shadow
    /// (`docs/FRAMES.md` §3.2).
    #[test]
    fn bands_cut_through_a_shape_a_glyph_and_a_shadow_make_the_whole_frame() {
        let scene = |c: &mut Canvas| {
            c.shadow(
                Rect::new(6, 5, 30, 18),
                5.0,
                8.0,
                0.0,
                2,
                Color::rgba(0, 0, 0, 90),
            );
            c.fill_rounded_rect(Rect::new(6, 5, 30, 18), 5.0, Color::rgb(90, 160, 220));
            c.stroke_rounded_rect(Rect::new(8, 7, 26, 14), 3.0, 1.5, Color::WHITE);
            c.fill_circle(48.3, 14.7, 7.2, Color::rgb(140, 210, 180));
            c.text(10, 8, "Wg", Font::regular(14.0), Color::BLACK);
        };
        let mut whole = canvas();
        whole.clear(Color::rgb(236, 238, 242));
        scene(&mut whole);
        for band in [1, 3, 7, 11, 16] {
            let mut banded = canvas();
            banded.clear(Color::rgb(236, 238, 242));
            let mut top = 0;
            while top < 32 {
                let rows = band.min(32 - top);
                banded.push_clip(Rect::new(0, top, 64, rows));
                scene(&mut banded);
                banded.pop_clip();
                top += rows;
            }
            for y in 0..32 {
                for x in 0..64 {
                    assert_eq!(
                        pixel(&banded, x, y),
                        pixel(&whole, x, y),
                        "bands {band} rows deep at ({x}, {y})"
                    );
                }
            }
        }
    }
}
