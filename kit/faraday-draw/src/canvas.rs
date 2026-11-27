//! The framebuffer and drawing primitives.
//!
//! [`Canvas`] wraps a `tiny_skia::Pixmap` (premultiplied RGBA8888) sized
//! from the port's `Display`. Shapes go through tiny-skia; glyph
//! outlines are filled by tiny-skia into coverage bitmaps the canvas
//! caches and blends by hand. A rectangular clip stack serves scroll
//! regions.

use alloc::vec::Vec;

use faraday_crypto::work::Work;
use faraday_port::values::Display;
use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};

use faraday_tokens::palette::Color;

use crate::skia::ToSkia;
use crate::fonts::{Coverage, GlyphCache, SizedFace};
use crate::geom::{Dp, Rect, Scale};
pub use crate::shapes::ShapeKey;
use crate::shapes::{self, Shapes};
use crate::text::{self, Font, TextAlign, TextMetrics};

#[path = "blend.rs"]
mod blend;
pub use blend::BlendBand;
mod list;
pub use list::Band;
use list::{List, Mode};
#[path = "surface.rs"]
mod surface;
pub(crate) use surface::Surface;

/// The pixels of a canvas: its width and height and the premultiplied RGBA rows, which the
/// runtime puts in the port's frame.
pub struct Pixels<'a> {
    pub width: u16,
    pub height: u16,
    pub rgba: &'a [u8],
}

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
    cut: Vec<faraday_text::Text>,
    /// Where each string and icon drawn since the last [`Canvas::clear`]
    /// or [`Canvas::restore`] left its ink, while [`Canvas::record_ink`] has it on: what the
    /// snapshot tool's audit reads. Off on a device.
    ink: Option<Vec<Inked>>,
    /// What is drawn now comes from a secret ([`Canvas::mark_secret`]).
    secret: bool,
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

/// What one frame drew, in drawing order; kept only while the record is on (04 §9, V21).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ink(pub Vec<Inked>);

/// The call list a frame records and the canvas replays in bands.
pub struct Calls(List);

/// One thing a frame drew and the box its visible ink covers.
#[derive(Debug, Clone, PartialEq)]
pub struct Inked {
    /// What was drawn.
    pub kind: InkKind,
    /// The box of its ink inside the clip it was drawn in, in pixels.
    pub rect: Rect,
    /// It was drawn from a `Shown::secret` (04 K12).
    pub secret: bool,
}

/// What an [`Ink`] is.
#[derive(Debug, Clone, PartialEq)]
pub enum InkKind {
    /// A line of text, and the face it was drawn in.
    Text {
        /// The characters.
        text: faraday_text::Text,
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
    Icon(crate::icons::Icon),
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
    pub fn new(display: &Display) -> Self {
        Canvas::part(
            display.width,
            display.height,
            Scale::for_class(display.dpi, crate::geom::size_class_of(display)),
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
            secret: false,
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

    /// Counts what was drawn since `mark` (a [`Canvas::lines_drawn`]) as
    /// one line: a line of text set in several faces, a bold word among
    /// plain ones, is one line, drawn run by run.
    pub fn join_lines_since(&mut self, mark: u64) {
        if self.lines > mark {
            self.lines = mark + 1;
        }
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
    pub fn pixels(&self) -> Pixels<'_> {
        debug_assert!(
            self.list.is_empty(),
            "calls recorded and not painted ([`Canvas::replay`])"
        );
        Pixels {
            width: self.pixmap.width() as u16,
            height: self.rows as u16,
            rgba: self.visible(),
        }
    }

    /// Copies `frame`, another canvas's pixels, in with its top-left
    /// corner at (`x`, `y`), cut at this canvas's edges: an app's frame
    /// drawn inside another's (the Starter's held desktop app). A copy
    /// of its pixels, counted as a whole copy ([`Work::FullCanvas`]).
    pub fn place(&mut self, x: i32, y: i32, frame: &Pixels<'_>) {
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
    pub fn place_scaled(&mut self, (x, y, w, h): (i32, i32, i32, i32), frame: &Pixels<'_>) {
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

    /// Marks what is drawn next as drawn from a secret, in the ink record, until it is
    /// unmarked.
    pub fn mark_secret(&mut self, on: bool) {
        self.secret = on;
    }

    /// The call list recorded and not yet painted, taken out of the canvas.
    pub fn take_calls(&mut self) -> Calls {
        Calls(core::mem::take(&mut self.list))
    }

    /// Puts a call list taken by [`Canvas::take_calls`] back, to be replayed.
    pub fn put_calls(&mut self, calls: Calls) {
        self.list = calls.0;
    }

    /// Whether the ink is being recorded ([`Canvas::record_ink`]).
    pub fn recording_ink(&self) -> bool {
        self.ink.is_some()
    }

    /// What was drawn since the last [`Canvas::clear`] or
    /// [`Canvas::restore`] and where, while recording is on; empty
    /// otherwise.
    pub fn ink(&self) -> &[Inked] {
        self.ink.as_deref().unwrap_or(&[])
    }

    /// The strings drawn since the last [`Canvas::clear`] whose ink ran
    /// past the left or right side of their clip: text a person sees cut
    /// off.
    pub fn cut(&self) -> &[faraday_text::Text] {
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

    /// Drops every cached glyph bitmap: the cache says which characters
    /// were drawn, so nothing of a shown secret stays in it past Lock.
    pub fn forget_glyphs(&mut self) {
        self.glyphs.clear();
    }

    /// How many glyph bitmaps the cache holds.
    pub fn cached_glyphs(&self) -> usize {
        self.glyphs.len()
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
            ink.push(Inked {
                kind: kind(),
                rect,
                secret: self.secret,
            });
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
    /// ([`faraday_tokens::dims::ELEVATION_KEY`], [`faraday_tokens::dims::ELEVATION_AMBIENT`]),
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
                faraday_tokens::dims::ELEVATION_AMBIENT[i],
                faraday_tokens::dims::ELEVATION_AMBIENT_OPACITY,
            ),
            (
                faraday_tokens::dims::ELEVATION_KEY[i],
                faraday_tokens::dims::ELEVATION_KEY_OPACITY,
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
        // The frame drawn over the snapshot starts again here, so the
        // ink recorded for the one before it is not counted twice.
        if !self.replaying()
            && let Some(ink) = &mut self.ink
        {
            ink.clear();
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
                    false,
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
        if !self.replaying() && cut && !self.cut.iter().any(|t| t.as_str() == s) {
            self.cut.push(faraday_text::Text::of(s));
        }
        let family = face.family;
        self.note_ink(
            || InkKind::Text {
                text: faraday_text::Text::of(s),
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
    pub fn icon(&mut self, rect: Rect, icon: crate::icons::Icon, size_dp: f32, color: Color) {
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
                icon.flipped(),
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

