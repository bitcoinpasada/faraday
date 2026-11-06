//! The list painted in bands of rows (`docs/FRAMES.md` §4.2).
//!
//! [`Canvas::replay_banded`] splits the canvas's rows into bands, each a
//! `&mut [u8]` of its own, and hands them to a caller that paints them
//! side by side (threads, or one after the other). A band reads the
//! list and the two caches and writes its own rows and its own room,
//! nothing else, so it is `Send`. A call that reads pixels other rows
//! wrote ends a run and is painted over the whole canvas between two runs.

use alloc::vec::Vec;
use core::ops::Range;

use tiny_skia::{Paint, PathBuilder, PixmapMut, Stroke, Transform};

use super::{Entry, List, Mode, Op};
use crate::canvas::surface::{self, ShadowOf, Surface};
use crate::canvas::{
    Canvas, SPLIT_ROWS, circle_path, closed_path, closed_stroke, contours_path, give_back,
    line_path, line_stroke, rounded_ends_path, rounded_rect_path,
};
use crate::color::Color;
use crate::fonts::{GlyphCache, SizedFace};
use crate::geom::{Rect, Scale};
use crate::shapes::{self, Shapes};
use crate::text::Font;

/// What a band keeps between frames, so it allocates nothing once it has
/// grown.
pub(crate) struct Room {
    saved: Vec<u8>,
    scratch: Vec<u8>,
    /// The first and the one past the last row of the scratch written.
    dirty: (i32, i32),
    paths: Vec<PathBuilder>,
    fall: Vec<u32>,
}

/// One band of the canvas's rows, and what it paints them from.
pub struct Band<'a> {
    list: &'a List,
    glyphs: &'a GlyphCache,
    shapes: &'a Shapes,
    rows: &'a mut [u8],
    room: &'a mut Room,
    /// The calls of the run it paints.
    run: Range<usize>,
    width: u32,
    /// The canvas row of the band's first row.
    top: i32,
    /// Whether it is the first band, which has no limit above, or the
    /// last, which has none below and holds the hidden rows.
    first: bool,
    last: bool,
    /// The canvas's visible rows, and all its rows.
    visible: i32,
    total: i32,
    scale: Scale,
}

/// The first row of band `i` of `n` over `rows` rows.
fn edge(rows: i32, n: usize, i: usize) -> i32 {
    (i as i64 * i64::from(rows) / n as i64) as i32
}

fn grown(x0: f32, y0: f32, x1: f32, y1: f32, by: f32) -> Rect {
    let (l, t) = ((x0 - by) as i32 - 1, (y0 - by) as i32 - 1);
    let (r, b) = ((x1 + by) as i32 + 2, (y1 + by) as i32 + 2);
    Rect::new(l, t, r - l, b - t)
}

fn bounds_of(points: &[(f32, f32)], by: f32) -> Option<Rect> {
    let (&(x, y), rest) = points.split_first()?;
    let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
    for &(x, y) in rest {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    Some(grown(x0, y0, x1, y1, by))
}

/// The pixels a path call can reach, whatever its clip: a few pixels
/// more than its ink. `None` for a call that is not a path.
pub(super) fn path_box(op: &Op, list: &List) -> Option<Rect> {
    Some(match *op {
        Op::Rounded(r, _) | Op::StrokeRounded(r, ..) => {
            Rect::new(r.x - 3, r.y - 3, r.w + 6, r.h + 6)
        }
        Op::Line([x0, y0, x1, y1], w) => {
            grown(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1), w + 3.0)
        }
        Op::StrokeClosed(p, w) => bounds_of(&list.points[p.0..p.1], w + 3.0)?,
        Op::Circle(cx, cy, r) => grown(cx, cy, cx, cy, r + 3.0),
        Op::Contours(p, _) => bounds_of(&list.points[p.0..p.1], 3.0)?,
        _ => return None,
    })
}

impl Default for Room {
    fn default() -> Self {
        Room {
            saved: Vec::new(),
            scratch: Vec::new(),
            dirty: (i32::MAX, i32::MIN),
            paths: Vec::new(),
            fall: Vec::new(),
        }
    }
}

impl Canvas {
    /// Whether every band's scratch holds only zeros.
    #[cfg(test)]
    pub(super) fn scratch_is_clear(&self) -> bool {
        self.list
            .rooms
            .iter()
            .all(|r| r.scratch.iter().all(|&b| b == 0))
    }

    /// How many of the calls waiting are barriers when the list is
    /// painted in any number of bands: the blurs and stretches. A lookup.
    pub fn barriers_in(&self, _bands: usize) -> usize {
        self.list.ops.iter().filter(|e| e.barrier).count()
    }

    /// Costs the painting of every call waiting. Paints the recorded
    /// calls in `n` bands of rows and empties the list. For each run of
    /// calls between barriers `run` is given the bands, each ready to
    /// [`Band::paint`] the run, and must paint them all (on any threads,
    /// in any order) before it returns; the barriers are painted here,
    /// over the whole canvas, between runs.
    pub fn replay_banded(&mut self, n: usize, run: &mut dyn FnMut(&mut [Band<'_>])) {
        if self.list.ops.is_empty() {
            return;
        }
        let n = n.clamp(1, self.rows.max(1) as usize);
        let mut list = core::mem::take(&mut self.list);
        let was = core::mem::replace(&mut self.mode, Mode::Replay);
        core::mem::swap(&mut self.clips, &mut list.clips);
        let mut rooms = core::mem::take(&mut list.rooms);
        rooms.resize_with(n, Room::default);
        let mut at = 0;
        while at < list.ops.len() {
            let mut end = at;
            while end < list.ops.len() && !list.ops[end].barrier {
                end += 1;
            }
            if end > at {
                self.paint_run(&list, &mut rooms[..n], at..end, run);
            }
            if end < list.ops.len() {
                let e = list.ops[end];
                self.clips.clear();
                self.clips.push(e.clip);
                self.paint_entry(&e, &list);
            }
            at = end + 1;
        }
        core::mem::swap(&mut self.clips, &mut list.clips);
        self.mode = was;
        list.clear();
        list.rooms = rooms;
        self.list = list;
        self.trim_caches();
    }

    fn paint_run(
        &mut self,
        list: &List,
        rooms: &mut [Room],
        calls: Range<usize>,
        run: &mut dyn FnMut(&mut [Band<'_>]),
    ) {
        let n = rooms.len();
        let width = self.pixmap.width();
        let total = self.pixmap.height() as i32;
        let visible = self.rows as i32;
        let stride = width as usize * 4;
        let mut rest = self.pixmap.data_mut();
        let mut bands = Vec::with_capacity(n);
        for (i, room) in rooms.iter_mut().enumerate() {
            let a = edge(visible, n, i);
            let b = if i + 1 == n {
                total
            } else {
                edge(visible, n, i + 1)
            };
            let (mine, tail) = core::mem::take(&mut rest).split_at_mut((b - a) as usize * stride);
            rest = tail;
            bands.push(Band {
                list,
                glyphs: &self.glyphs,
                shapes: &self.shapes,
                rows: mine,
                room,
                run: calls.clone(),
                width,
                top: a,
                first: i == 0,
                last: i + 1 == n,
                visible,
                total,
                scale: self.scale,
            });
        }
        run(&mut bands);
    }
}

impl Band<'_> {
    /// Costs the painting of the run's calls. Paints the calls of the
    /// run into this band's rows, in the order recorded.
    pub fn paint(&mut self) {
        let list = self.list;
        for k in self.run.clone() {
            self.entry(&list.ops[k], list);
        }
        self.clear_scratch();
    }

    /// The canvas rows the band paints, open at the end that has no
    /// edge.
    fn limits(&self) -> (i32, i32) {
        let h = (self.rows.len() / (self.width as usize * 4)) as i32;
        (
            if self.first { i32::MIN } else { self.top },
            if self.last { i32::MAX } else { self.top + h },
        )
    }

    fn entry(&mut self, e: &Entry, list: &List) {
        let (color, clip) = (e.color, e.clip);
        let (width, top) = (self.width, self.top);
        let h = (self.rows.len() / (width as usize * 4)) as u32;
        if h == 0 {
            return;
        }
        let (lo, hi) = self.limits();
        // A call whose box is wholly in other bands' rows paints nothing
        // here.
        macro_rules! surf {
            () => {
                Surface::new(
                    PixmapMut::from_bytes(&mut *self.rows, width, h).expect("a band has rows"),
                    top,
                )
            };
        }
        match e.op {
            Op::Clear => self.clear(color),
            Op::FillRect(r) => surface::fill_rect(&mut surf!(), r, clip, color),
            Op::Cells { at, n, cell, bits } => {
                let bytes = &list.bytes[bits..];
                surface::fill_cells(
                    &mut surf!(),
                    clip,
                    (at, n, cell),
                    |c, r| {
                        let i = r * n + c;
                        bytes[i / 8] >> (i % 8) & 1 == 1
                    },
                    color,
                );
            }
            Op::Shadow {
                rect,
                radius,
                blur,
                spread,
                dy,
            } => {
                let of = ShadowOf {
                    rect,
                    radius,
                    blur,
                    spread,
                    dy,
                };
                let mut s = surf!();
                surface::shadow(&mut s, clip, of, color, &mut self.room.fall);
            }
            Op::Luma {
                rect,
                width: sw,
                height: sh,
                pixels,
                chroma,
            } => {
                let dst = rect.intersect(&clip);
                surface::luma(
                    &mut surf!(),
                    (rect, dst),
                    (usize::from(sw), usize::from(sh)),
                    &list.bytes[pixels.0..pixels.1],
                    chroma.map(|c| &list.bytes[c.0..c.1]),
                );
            }
            Op::Kept(key) => {
                shapes::draw(self.shapes, &mut surf!(), clip, key, color);
            }
            Op::Text { face, x, y, s } => {
                let lh = face.line_height();
                if y + 2 * lh <= lo || y - lh >= hi {
                    return;
                }
                self.text(&face, (x, y), &list.text[s.0..s.1], (clip, color));
            }
            Op::Icon(rect, icon, size) => {
                let face = Font::icon(size).sized(self.scale);
                let Some(g) = self.glyphs.peek(&face, icon.code_point()) else {
                    return;
                };
                let x = rect.x + (rect.w - g.width as i32) / 2;
                let y = rect.y + (rect.h - g.height as i32) / 2;
                let dims = (x, y, g.width, g.height);
                surface::blit_glyph(&mut surf!(), clip, dims, g.bitmap, color);
            }
            Op::Rounded(r, radius) => self.rounded(r, radius, clip, color),
            Op::StrokeRounded(..)
            | Op::Line(..)
            | Op::StrokeClosed(..)
            | Op::Circle(..)
            | Op::Contours(..) => {
                self.path_op(e, list);
            }
            Op::Frost(..) | Op::Shift(..) => {
                debug_assert!(false, "a barrier is painted over the whole canvas");
            }
        }
    }

    /// Costs the rasterizing of the path. A stroked or filled path call.
    fn path_op(&mut self, e: &Entry, list: &List) {
        let (color, clip) = (e.color, e.clip);
        match e.op {
            Op::StrokeRounded(r, radius, w) => {
                let stroke = Stroke {
                    width: w,
                    ..Stroke::default()
                };
                let (t, b) = (r.y - 3, r.bottom() + 3);
                let build = |pool: &mut Vec<PathBuilder>| {
                    let at = Rect::new(r.x, r.y, r.w, r.h);
                    Some(rounded_rect_path(pool, at, radius, Some(&stroke)))
                };
                // The outline is filled: the stroke made it already.
                self.shaped((t, b), clip, &build, None, &Canvas::paint(color));
            }
            Op::Line(p, w) => {
                let stroke = line_stroke(w);
                let cons = path_box(&e.op, list).unwrap_or_default();
                let build = |_: &mut Vec<PathBuilder>| line_path(p);
                let rows = (cons.y, cons.bottom());
                self.shaped(rows, clip, &build, Some(&stroke), &Canvas::paint(color));
            }
            Op::StrokeClosed(p, w) => {
                let stroke = closed_stroke(w);
                let cons = path_box(&e.op, list).unwrap_or_default();
                let build = |_: &mut Vec<PathBuilder>| closed_path(&list.points[p.0..p.1]);
                let rows = (cons.y, cons.bottom());
                self.shaped(rows, clip, &build, Some(&stroke), &Canvas::paint(color));
            }
            Op::Circle(cx, cy, radius) => {
                let reach = radius + 3.0;
                let rows = ((cy - reach) as i32 - 1, (cy + reach) as i32 + 2);
                let build = |pool: &mut Vec<PathBuilder>| circle_path(pool, (cx, cy), radius);
                self.shaped(rows, clip, &build, None, &Canvas::paint(color));
            }
            Op::Contours(p, lens) => {
                let cons = path_box(&e.op, list).unwrap_or_default();
                let build = |_: &mut Vec<PathBuilder>| {
                    let mut from = p.0;
                    contours_path(list.lens[lens.0..lens.1].iter().map(|&n| {
                        let c = &list.points[from..from + n];
                        from += n;
                        c
                    }))
                };
                let rows = (cons.y, cons.bottom());
                self.shaped(rows, clip, &build, None, &Canvas::paint(color));
            }
            _ => {}
        }
    }

    /// The visible rows of this band, filled with one colour.
    fn clear(&mut self, color: Color) {
        let (width, top) = (self.width, self.top);
        let h = (self.rows.len() / (width as usize * 4)) as i32;
        let c = color.to_skia().premultiply().to_color_u8();
        let px = [c.red(), c.green(), c.blue(), c.alpha()];
        let visible = (self.visible - top).clamp(0, h) as usize;
        let n = visible * width as usize * 4;
        for p in self.rows[..n].chunks_exact_mut(4) {
            p.copy_from_slice(&px);
        }
    }

    /// A line of text, glyph by glyph, as `Canvas::text_with_face`
    /// paints it.
    fn text(&mut self, face: &SizedFace, (x, y): (i32, i32), text: &str, ink: (Rect, Color)) {
        let (clip, color) = ink;
        let (width, top) = (self.width, self.top);
        let h = (self.rows.len() / (width as usize * 4)) as u32;
        let pm = PixmapMut::from_bytes(&mut *self.rows, width, h).expect("a band has rows");
        let mut surface = Surface::new(pm, top);
        let baseline = y + face.ascent;
        let mut pen = (x.max(-32_768) as i64) << 6;
        for c in text.chars() {
            let Some(g) = self.glyphs.peek(face, c) else {
                continue;
            };
            let gx = ((pen + 32) >> 6) as i32 + g.bearing_x;
            let gy = baseline - g.bearing_y;
            let dims = (gx, gy, g.width, g.height);
            surface::blit_glyph(&mut surface, clip, dims, g.bitmap, color);
            pen += i64::from(g.advance_64);
        }
    }

    /// A rounded rectangle's fill, as `Canvas::fill_rounded_rect` paints
    /// it.
    fn rounded(&mut self, r: Rect, radius: f32, clip: Rect, color: Color) {
        let (width, top) = (self.width, self.top);
        let h = (self.rows.len() / (width as usize * 4)) as u32;
        if radius <= 0.5 {
            let mut s = Surface::new(
                PixmapMut::from_bytes(&mut *self.rows, width, h).expect("a band has rows"),
                top,
            );
            surface::fill_rect(&mut s, r, clip, color);
            return;
        }
        let paint = Canvas::paint(color);
        let cap = radius.min(r.w as f32 / 2.0) as i32 + 2;
        if color.a == 255 && r.h - 2 * cap >= SPLIT_ROWS {
            let mid = Rect::new(r.x, r.y + cap, r.w, r.h - 2 * cap);
            for (cut, up) in [(mid.y, true), (mid.bottom(), false)] {
                let rows = if up {
                    (r.y - 3, cut + 3)
                } else {
                    (cut - 3, r.bottom() + 3)
                };
                let build = |pool: &mut Vec<PathBuilder>| {
                    let at = Rect::new(r.x, r.y, r.w, r.h);
                    Some(rounded_ends_path(pool, at, radius, (cut, up)))
                };
                self.shaped(rows, clip, &build, None, &paint);
            }
            let mut s = Surface::new(
                PixmapMut::from_bytes(&mut *self.rows, width, h).expect("a band has rows"),
                top,
            );
            surface::fill_rect(&mut s, mid, clip, color);
            return;
        }
        let build = |pool: &mut Vec<PathBuilder>| {
            let at = Rect::new(r.x, r.y, r.w, r.h);
            Some(rounded_rect_path(pool, at, radius, None))
        };
        self.shaped((r.y - 3, r.bottom() + 3), clip, &build, None, &paint);
    }

    /// Costs the rasterizing of the path. Paints a path whose ink lies
    /// within canvas rows `rows` (the first row and the one past the
    /// last), as far as this band's rows go.
    ///
    /// The rasterizer does not give the same pixels for a path moved by
    /// whole rows, so a path is never moved: it is painted at its own
    /// canvas coordinates, as the direct canvas paints it, straight into
    /// the first band when it lies wholly in it, otherwise into the
    /// room's scratch, a whole canvas of rows. The band's own rows of
    /// the ink are copied in, the path painted, and the same rows copied
    /// back.
    fn shaped(
        &mut self,
        (t, b): (i32, i32),
        clip: Rect,
        build: &dyn Fn(&mut Vec<PathBuilder>) -> Option<tiny_skia::Path>,
        stroke: Option<&Stroke>,
        paint: &Paint,
    ) {
        let (lo, hi) = self.limits();
        if b <= lo || t >= hi {
            return;
        }
        let (width, top, total) = (self.width, self.top, self.total);
        let h = (self.rows.len() / (width as usize * 4)) as i32;
        let room = &mut *self.room;
        let Some(path) = build(&mut room.paths) else {
            return;
        };
        let ink = surface::ink_box(&path, stroke);
        let args = (&path, paint, stroke);
        let at = (clip, self.visible);
        let (y0, y1) = (ink.y.max(lo).max(0), ink.bottom().min(hi).min(total));
        let (x0, x1) = (ink.x.max(0), ink.right().min(width as i32));
        if y0 < y1 && x0 < x1 {
            if self.first && ink.bottom() <= top + h {
                let pm = PixmapMut::from_bytes(&mut *self.rows, width, h as u32)
                    .expect("a band has rows");
                let mut s = Surface::new(pm, 0);
                surface::path(
                    &mut s,
                    &mut room.saved,
                    at,
                    args,
                    ink,
                    Transform::identity(),
                );
            } else {
                let stride = width as usize * 4;
                room.scratch.resize(total as usize * stride, 0);
                let cols = x0 as usize * 4..x1 as usize * 4;
                for y in y0..y1 {
                    let own = (y - top) as usize * stride;
                    let sc = y as usize * stride;
                    room.scratch[sc..sc + stride][cols.clone()]
                        .copy_from_slice(&self.rows[own..own + stride][cols.clone()]);
                }
                {
                    let pm = PixmapMut::from_bytes(&mut room.scratch, width, total as u32)
                        .expect("a scratch has rows");
                    let mut s = Surface::new(pm, 0);
                    let ink = Rect::new(ink.x, y0, ink.w, y1 - y0);
                    surface::path(
                        &mut s,
                        &mut room.saved,
                        at,
                        args,
                        ink,
                        Transform::identity(),
                    );
                }
                for y in y0..y1 {
                    let own = (y - top) as usize * stride;
                    let sc = y as usize * stride;
                    self.rows[own..own + stride][cols.clone()]
                        .copy_from_slice(&room.scratch[sc..sc + stride][cols.clone()]);
                }
                // The rasterizer writes the whole shape, rows of other
                // bands included: all of them are cleared afterwards.
                let (d0, d1) = (ink.y.max(0), ink.bottom().min(total));
                room.dirty = (room.dirty.0.min(d0), room.dirty.1.max(d1));
            }
        }
        give_back(&mut room.paths, path);
    }

    /// Costs the clearing of the rows the scratch was written. Zeroes
    /// them, so no pixel of a secret outlives the paint in the scratch.
    fn clear_scratch(&mut self) {
        let room = &mut *self.room;
        let stride = self.width as usize * 4;
        let (a, b) = (room.dirty.0.max(0) as usize, room.dirty.1.max(0) as usize);
        if a < b {
            let end = (b * stride).min(room.scratch.len());
            let start = (a * stride).min(end);
            room.scratch[start..end].fill(0);
        }
        room.dirty = (i32::MAX, i32::MIN);
    }
}
