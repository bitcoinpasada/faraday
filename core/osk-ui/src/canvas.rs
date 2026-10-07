//! The framebuffer and drawing primitives.
//!
//! [`Canvas`] wraps a `tiny_skia::Pixmap` (premultiplied RGBA8888) sized
//! from the shell's [`DisplayInfo`]. Shapes go through tiny-skia; glyph
//! outlines are filled by tiny-skia into coverage bitmaps the canvas
//! caches and blends by hand. A rectangular clip stack serves scroll
//! regions.

use alloc::string::String;
use alloc::vec::Vec;

use osk_shell_api::{DisplayInfo, Frame};
use tiny_skia::{
    FillRule, Mask, Paint, PathBuilder, Pixmap, PremultipliedColorU8, Stroke, Transform,
};

use crate::color::Color;
use crate::fonts::{Coverage, GlyphCache, SizedFace};
use crate::geom::{Rect, Scale, SizeClass};
use crate::text::{self, Font, TextAlign, TextMetrics};

/// A drawing surface the size of the display.
pub struct Canvas {
    pixmap: Pixmap,
    scale: Scale,
    clips: Vec<Rect>,
    /// tiny-skia mask for the current clip and the clip it was built
    /// for, `None` when unclipped.
    mask: Option<(Rect, Mask)>,
    /// Masks built for other clips lately. A draw inside a scroll region
    /// pushes the viewport and pops back to nothing for every widget, so
    /// the same one or two clips come round again and again; building a
    /// panel-sized mask for each push and pop was most of the cost of a
    /// frame on a small board.
    spare: Vec<(Rect, Mask)>,
    /// Glyph bitmaps filled from the outlines, kept between frames.
    glyphs: GlyphCache,
    /// The strings drawn since the last [`Canvas::clear`] that ran past
    /// the side of their clip, so that some of their ink is not on the
    /// panel (`docs/PLANNING.md` §16.133 rule 3).
    cut: Vec<String>,
    /// Where each string and icon drawn since the last [`Canvas::clear`]
    /// left its ink, while [`Canvas::record_ink`] has it on: what the
    /// snapshot tool's audit reads. Off on a device.
    ink: Option<Vec<Ink>>,
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
    },
    /// An icon glyph.
    Icon(crate::widgets::Icon),
}

/// How many masks are kept for clips that come round again.
const SPARE_MASKS: usize = 4;

impl Canvas {
    /// Allocates a framebuffer for `display`. Panics on a zero-sized
    /// display; the shell must refuse those before reaching the core.
    pub fn new(display: &DisplayInfo) -> Self {
        let pixmap = Pixmap::new(u32::from(display.width), u32::from(display.height))
            .expect("display has non-zero size");
        Canvas {
            pixmap,
            scale: Scale::for_class(display.dpi, SizeClass::of(display)),
            clips: Vec::new(),
            mask: None,
            spare: Vec::new(),
            glyphs: GlyphCache::new(),
            cut: Vec::new(),
            ink: None,
        }
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

    /// The current clip rectangle.
    pub fn clip(&self) -> Rect {
        self.clips.last().copied().unwrap_or_else(|| self.bounds())
    }

    /// Restricts drawing to `rect` (intersected with the current clip)
    /// until the matching [`Canvas::pop_clip`].
    pub fn push_clip(&mut self, rect: Rect) {
        let clip = self.clip().intersect(&rect);
        self.clips.push(clip);
        self.rebuild_mask();
    }

    /// Restores the previous clip.
    pub fn pop_clip(&mut self) {
        self.clips.pop();
        self.rebuild_mask();
    }

    fn rebuild_mask(&mut self) {
        let clip = self.clip();
        if self.mask.as_ref().is_some_and(|(r, _)| *r == clip) {
            return;
        }
        if let Some(old) = self.mask.take() {
            self.spare.push(old);
            if self.spare.len() > SPARE_MASKS {
                self.spare.remove(0);
            }
        }
        if clip == self.bounds() {
            return;
        }
        if let Some(i) = self.spare.iter().position(|(r, _)| *r == clip) {
            self.mask = Some(self.spare.remove(i));
            return;
        }
        let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height()).expect("mask size");
        if !clip.is_empty() {
            let path = PathBuilder::from_rect(skia_rect(clip));
            mask.fill_path(&path, FillRule::Winding, false, Transform::identity());
        }
        self.mask = Some((clip, mask));
    }

    /// Fills the whole canvas, ignoring the clip.
    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(color.to_skia());
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

    fn note_ink(&mut self, kind: impl FnOnce() -> InkKind, rect: Option<Rect>) {
        if let (Some(ink), Some(rect)) = (&mut self.ink, rect)
            && !rect.is_empty()
        {
            ink.push(Ink { kind: kind(), rect });
        }
    }

    /// Every string drawn since the last [`Canvas::clear`] whose ink ran
    /// past the left or right edge of the clip it was drawn in: a label
    /// cut off at its row's edge. Text above or below its clip — a row
    /// scrolled out of its viewport — is not counted; that is scrolling,
    /// not cutting.
    pub fn cut_texts(&self) -> &[String] {
        &self.cut
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
        let mut paint = Self::paint(color);
        paint.anti_alias = false;
        self.pixmap
            .fill_rect(skia_rect(r), &paint, Transform::identity(), None);
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
        let path = rounded_rect_path(rect, radius, 0.0);
        self.pixmap.fill_path(
            &path,
            &Self::paint(color),
            FillRule::Winding,
            Transform::identity(),
            self.mask.as_ref().map(|m| &m.1),
        );
    }

    /// Strokes a rectangle outline `width` pixels thick, inside the rect.
    pub fn stroke_rect(&mut self, rect: Rect, width: f32, color: Color) {
        self.stroke_rounded_rect(rect, 0.0, width, color);
    }

    /// Strokes a rounded rectangle outline `width` pixels thick, inside
    /// the rect.
    pub fn stroke_rounded_rect(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        if rect.is_empty() || color.a == 0 || rect.intersect(&self.clip()).is_empty() {
            return;
        }
        let path = rounded_rect_path(rect, radius, width / 2.0);
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            &path,
            &Self::paint(color),
            &stroke,
            Transform::identity(),
            self.mask.as_ref().map(|m| &m.1),
        );
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
        self.pixmap.stroke_path(
            &path,
            &Self::paint(color),
            &stroke,
            Transform::identity(),
            self.mask.as_ref().map(|m| &m.1),
        );
    }

    /// Fills a circle.
    pub fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        let Some(path) = PathBuilder::from_circle(cx, cy, radius) else {
            return;
        };
        self.pixmap.fill_path(
            &path,
            &Self::paint(color),
            FillRule::Winding,
            Transform::identity(),
            self.mask.as_ref().map(|m| &m.1),
        );
    }

    /// Strokes an arc of a circle, starting at twelve o'clock and
    /// sweeping clockwise over `fraction` of a whole turn.
    ///
    /// The eye's ring (`docs/DESIGN.md` §4.10) is the one caller: it
    /// empties as the reveal runs. No trigonometry is available without
    /// `std`, so the arc is a polyline whose points come from rotating
    /// one radius vector.
    pub fn stroke_arc(
        &mut self,
        cx: f32,
        cy: f32,
        radius: f32,
        width: f32,
        fraction: f32,
        color: Color,
    ) {
        /// Segments a whole turn is drawn in.
        const SEGMENTS: usize = 72;
        /// Degrees in a whole turn.
        const TURN_DEG: f32 = 360.0;
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction <= 0.0 || radius <= 0.0 || color.a == 0 {
            return;
        }
        let steps = ((fraction * SEGMENTS as f32) as usize).clamp(1, SEGMENTS);
        let mut pb = PathBuilder::new();
        for i in 0..=steps {
            let deg = fraction * TURN_DEG * (i as f32 / steps as f32);
            let mut p = tiny_skia::Point::from_xy(0.0, -radius);
            Transform::from_rotate(deg).map_point(&mut p);
            let (x, y) = (cx + p.x, cy + p.y);
            if i == 0 {
                pb.move_to(x, y);
            } else {
                pb.line_to(x, y);
            }
        }
        let Some(path) = pb.finish() else { return };
        let stroke = Stroke {
            width,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            &path,
            &Self::paint(color),
            &stroke,
            Transform::identity(),
            self.mask.as_ref().map(|m| &m.1),
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
        self.note_ink(|| InkKind::Icon(icon), Some(inked));
    }

    /// Draws one line of text aligned within `rect` (vertically centred).
    pub fn text_in(&mut self, rect: Rect, s: &str, font: Font, color: Color, align: TextAlign) {
        let face = font.sized(self.scale);
        let w = text::width(&face, s);
        let x = match align {
            TextAlign::Start => rect.x,
            TextAlign::Center => rect.x + (rect.w - w) / 2,
            TextAlign::End => rect.right() - w,
        };
        let y = rect.y + (rect.h - face.line_height()) / 2;
        self.text_with_face(&face, x, y, s, color);
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
    let mut pb = PathBuilder::new();
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
            secure: osk_shell_api::SecureHardware::None,
            boot: osk_shell_api::BootState::Unknown,
            memory_mib: None,
        })
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
}
