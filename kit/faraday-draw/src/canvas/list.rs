//! The list a frame can be recorded into and replayed from
//! (`docs/FRAMES.md` §4.2).
//!
//! With [`Canvas::record_calls`] on, each painting call appends what it
//! is, its colour, its clip and what it needs to a list instead of
//! painting, and [`Canvas::replay`] paints the list in the order it was
//! recorded. What a call tells the caller at once (the width a line of
//! text takes), and what the canvas counts or notes about the frame (lines
//! of text, blurs, whole copies, the ink, the strings cut off), are done as
//! the call is recorded and not again when it is replayed.
//!
//! Recordable: [`Canvas::clear`], `fill_rect`, `fill_cells`, `shadow` (and
//! `elevation`), `luma`, the paths (`fill_rounded_rect`,
//! `stroke_rounded_rect`, `line`, `stroke_closed`, `fill_circle`,
//! `fill_contours`), `fill_kept`, text and icons, and the two calls that
//! read pixels other rows wrote, `frost` and `shift`, which are
//! barriers. Their variable data is copied into room the list keeps
//! between frames, so a frame recorded again allocates nothing once the
//! list has grown.
//!
//! Replayed first, whatever is waiting, by the calls that read or write the
//! canvas's pixels as a whole or take pixels the caller holds: `place`,
//! `place_scaled`, `restore`, `snapshot`, `snapshot_into`, `blend_between`,
//! `blend_noting`, `blend_within`, `swap_pixels`, `keep_layer`,
//! `paste_layer`, `fade_region`, `fade_to_background`. Two readers take
//! `&self` and so cannot paint what waits: [`Canvas::copy_region`] and the
//! shell's [`Canvas::frame`] read a canvas with nothing waiting, and the
//! caller calls [`Canvas::replay`] first.

use alloc::vec::Vec;
use faraday_text::Text;

mod bands;
pub use bands::Band;

use super::{Canvas, ShapeKey};
use faraday_tokens::palette::Color;

use crate::fonts::SizedFace;
use crate::geom::Rect;
use crate::icons::Icon;

/// What the canvas does with a painting call.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Mode {
    /// Paints it.
    #[default]
    Direct,
    /// Appends it to the list.
    Record,
    /// Paints a call of the list: what the call counts and notes was done
    /// when it was recorded.
    Replay,
}

/// One painting call.
#[derive(Clone, Copy, Debug)]
enum Op {
    Clear,
    FillRect(Rect),
    /// Cells of a grid; `bits` is where its bits start in the list's room.
    Cells {
        at: (i32, i32),
        n: usize,
        cell: i32,
        bits: usize,
    },
    Shadow {
        rect: Rect,
        radius: f32,
        blur: f32,
        spread: f32,
        dy: i32,
    },
    Luma {
        rect: Rect,
        width: u16,
        height: u16,
        pixels: (usize, usize),
        chroma: Option<(usize, usize)>,
    },
    Rounded(Rect, f32),
    StrokeRounded(Rect, f32, f32),
    Line([f32; 4], f32),
    StrokeClosed((usize, usize), f32),
    Circle(f32, f32, f32),
    /// The points of all the contours, and the lengths of the contours.
    Contours((usize, usize), (usize, usize)),
    Kept(ShapeKey),
    Text {
        face: SizedFace,
        x: i32,
        y: i32,
        s: (usize, usize),
    },
    Icon(Rect, Icon, f32),
    Frost(u32, u32),
    Shift(Rect, i32),
}

/// A recorded call with its colour and the clip it was made under.
#[derive(Clone, Copy, Debug)]
struct Entry {
    op: Op,
    clip: Rect,
    color: Color,
    /// Whether the call reads pixels other rows wrote (`docs/FRAMES.md`
    /// §4.2): the main thread paints it over the whole canvas.
    barrier: bool,
}

/// The recorded calls, and the room their data is copied into.
#[derive(Default)]
pub(super) struct List {
    ops: Vec<Entry>,
    /// Bits of cells, and the pixels of a camera frame.
    bytes: Vec<u8>,
    points: Vec<(f32, f32)>,
    lens: Vec<usize>,
    text: Text,
    /// The clip stack of the canvas while the list is painted.
    clips: Vec<Rect>,
    /// What each band keeps between frames.
    rooms: Vec<bands::Room>,
}

impl List {
    pub(super) fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    fn clear(&mut self) {
        self.ops.clear();
        self.bytes.clear();
        self.points.clear();
        self.lens.clear();
        // The text may be a seed's words: the whole buffer is wiped, its
        // capacity kept so the next frame records without allocating.
        self.text.clear();
        self.clips.clear();
    }
}

impl Canvas {
    /// Starts or stops recording the painting calls into the list. Turning
    /// it off paints what is waiting first.
    pub fn record_calls(&mut self, on: bool) {
        if !on {
            self.replay();
        }
        self.mode = if on { Mode::Record } else { Mode::Direct };
        if on {
            // Every glyph and shape a recorded call needs stays until the
            // list is painted (`docs/FRAMES.md` §4.2).
            self.glyphs.hold();
            self.shapes.hold();
        }
    }

    /// With nothing waiting, lets the caches drop what is over their
    /// budget, and holds them again if calls are still being recorded.
    pub(super) fn trim_caches(&mut self) {
        self.glyphs.release();
        self.shapes.release();
        if self.mode == Mode::Record {
            self.glyphs.hold();
            self.shapes.hold();
        }
    }

    /// How many calls wait to be painted, and how many of them are
    /// barriers. A lookup.
    pub fn waiting(&self) -> (usize, usize) {
        let barriers = self.list.ops.iter().filter(|e| e.barrier).count();
        (self.list.ops.len(), barriers)
    }

    pub(super) fn recording(&self) -> bool {
        self.mode == Mode::Record
    }

    /// Whether the call being made paints for a replay.
    pub(super) fn replaying(&self) -> bool {
        self.mode == Mode::Replay
    }

    /// Costs the painting of every call waiting. Paints the recorded
    /// calls on the canvas in the order they were recorded, each under the
    /// clip it was made under, as one band over all the rows, and empties
    /// the list, keeping its room.
    pub fn replay(&mut self) {
        if self.list.ops.is_empty() {
            return;
        }
        let list = core::mem::take(&mut self.list);
        let mut list = list;
        let was = core::mem::replace(&mut self.mode, Mode::Replay);
        core::mem::swap(&mut self.clips, &mut list.clips);
        let mut at = 0;
        while at < list.ops.len() {
            let e = list.ops[at];
            at += 1;
            self.clips.clear();
            self.clips.push(e.clip);
            self.paint_entry(&e, &list);
        }
        core::mem::swap(&mut self.clips, &mut list.clips);
        self.mode = was;
        list.clear();
        self.list = list;
        self.trim_caches();
    }

    /// Replays what waits before a call that reads or writes the pixels
    /// as a whole.
    pub(super) fn sync(&mut self) {
        if !self.list.ops.is_empty() {
            self.replay();
        }
    }

    fn paint_entry(&mut self, e: &Entry, list: &List) {
        let color = e.color;
        match e.op {
            Op::Clear => self.clear(color),
            Op::FillRect(r) => self.fill_rect(r, color),
            Op::Cells { at, n, cell, bits } => {
                let bytes = &list.bytes[bits..];
                self.fill_cells(
                    at,
                    (n, cell),
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
            } => self.shadow(rect, radius, blur, spread, dy, color),
            Op::Luma {
                rect,
                width,
                height,
                pixels,
                chroma,
            } => self.luma(
                rect,
                width,
                height,
                &list.bytes[pixels.0..pixels.1],
                chroma.map(|c| &list.bytes[c.0..c.1]),
            ),
            Op::Rounded(r, radius) => self.fill_rounded_rect(r, radius, color),
            Op::StrokeRounded(r, radius, width) => {
                self.stroke_rounded_rect(r, radius, width, color);
            }
            Op::Line([x0, y0, x1, y1], width) => self.line(x0, y0, x1, y1, width, color),
            Op::StrokeClosed(pts, width) => {
                self.stroke_closed(&list.points[pts.0..pts.1], width, color);
            }
            Op::Circle(cx, cy, radius) => self.fill_circle(cx, cy, radius, color),
            Op::Contours(pts, lens) => {
                let mut from = pts.0;
                let contours = list.lens[lens.0..lens.1].iter().map(|&n| {
                    let c = &list.points[from..from + n];
                    from += n;
                    c
                });
                self.fill_contours_of(contours, color);
            }
            Op::Kept(key) => self.fill_kept(key, Vec::new, color),
            Op::Text { face, x, y, s } => {
                self.text_with_face(&face, x, y, &list.text[s.0..s.1], color);
            }
            Op::Icon(rect, icon, size) => self.icon(rect, icon, size, color),
            Op::Frost(scale, radius) => self.frost(scale, radius, color),
            Op::Shift(rect, dy) => self.shift(rect, dy),
        }
    }

    /// Appends `op` under the current clip.
    fn push_call(&mut self, op: impl FnOnce(&mut List) -> Op, color: Color) {
        let op = op(&mut self.list);
        let clip = self.clip();
        // A path's ink is not known until it is drawn: noted from a box a
        // few pixels larger than it.
        if let Some(b) = bands::path_box(&op, &self.list) {
            self.damaged(b.intersect(&clip));
        }
        let barrier = matches!(op, Op::Frost(..) | Op::Shift(..));
        self.list.ops.push(Entry {
            op,
            clip,
            color,
            barrier,
        });
    }

    fn push_plain(&mut self, op: Op, color: Color) {
        self.push_call(|_| op, color);
    }

    // The hooks: each painting call, in recording mode, ends here.

    pub(super) fn record_clear(&mut self, color: Color) {
        self.push_plain(Op::Clear, color);
    }

    pub(super) fn record_rect(&mut self, rect: Rect, color: Color) {
        self.push_plain(Op::FillRect(rect), color);
    }

    pub(super) fn record_cells(
        &mut self,
        at: (i32, i32),
        (n, cell): (usize, i32),
        dark: impl Fn(usize, usize) -> bool,
        color: Color,
    ) {
        self.push_call(
            |l| {
                let bits = l.bytes.len();
                l.bytes.resize(bits + (n * n).div_ceil(8), 0);
                for r in 0..n {
                    for c in 0..n {
                        if dark(c, r) {
                            let i = r * n + c;
                            l.bytes[bits + i / 8] |= 1 << (i % 8);
                        }
                    }
                }
                Op::Cells { at, n, cell, bits }
            },
            color,
        );
    }

    pub(super) fn record_shadow(
        &mut self,
        (rect, radius, blur): (Rect, f32, f32),
        spread: f32,
        dy: i32,
        color: Color,
    ) {
        // Worked out per pixel from its distance to the shape: it reads
        // no pixel but its own, so it is not a barrier.
        let op = Op::Shadow {
            rect,
            radius,
            blur,
            spread,
            dy,
        };
        self.push_plain(op, color);
    }

    pub(super) fn record_luma(
        &mut self,
        rect: Rect,
        (width, height): (u16, u16),
        pixels: &[u8],
        chroma: Option<&[u8]>,
    ) {
        self.push_call(
            |l| {
                let a = l.bytes.len();
                l.bytes.extend_from_slice(pixels);
                let b = l.bytes.len();
                let chroma = chroma.map(|c| {
                    l.bytes.extend_from_slice(c);
                    (b, l.bytes.len())
                });
                Op::Luma {
                    rect,
                    width,
                    height,
                    pixels: (a, b),
                    chroma,
                }
            },
            Color::BLACK,
        );
    }

    pub(super) fn record_rounded(&mut self, rect: Rect, radius: f32, color: Color) {
        self.push_plain(Op::Rounded(rect, radius), color);
    }

    pub(super) fn record_stroke_rounded(
        &mut self,
        rect: Rect,
        radius: f32,
        width: f32,
        color: Color,
    ) {
        self.push_plain(Op::StrokeRounded(rect, radius, width), color);
    }

    pub(super) fn record_line(&mut self, p: [f32; 4], width: f32, color: Color) {
        self.push_plain(Op::Line(p, width), color);
    }

    pub(super) fn record_stroke_closed(&mut self, points: &[(f32, f32)], width: f32, color: Color) {
        self.push_call(
            |l| {
                let a = l.points.len();
                l.points.extend_from_slice(points);
                Op::StrokeClosed((a, l.points.len()), width)
            },
            color,
        );
    }

    pub(super) fn record_circle(&mut self, (cx, cy): (f32, f32), radius: f32, color: Color) {
        self.push_plain(Op::Circle(cx, cy, radius), color);
    }

    pub(super) fn record_contours(&mut self, contours: &[&[(f32, f32)]], color: Color) {
        self.push_call(
            |l| {
                let (p0, n0) = (l.points.len(), l.lens.len());
                for c in contours {
                    l.points.extend_from_slice(c);
                    l.lens.push(c.len());
                }
                Op::Contours((p0, l.points.len()), (n0, l.lens.len()))
            },
            color,
        );
    }

    /// Looks the shape up as it is recorded, so the cache is complete
    /// before anything paints.
    pub(super) fn record_kept(
        &mut self,
        key: ShapeKey,
        outlines: impl FnOnce() -> Vec<Vec<(f32, f32)>>,
        color: Color,
    ) {
        self.shapes.keep(key, outlines);
        let clip = self.clip();
        if let Some(area) = self.shapes.area(key) {
            self.damaged(area.intersect(&clip));
        }
        self.push_plain(Op::Kept(key), color);
    }

    pub(super) fn record_text(
        &mut self,
        face: &SizedFace,
        (x, y): (i32, i32),
        s: &str,
        color: Color,
    ) {
        self.push_call(
            |l| {
                let a = l.text.len();
                l.text.push_str(s);
                Op::Text {
                    face: *face,
                    x,
                    y,
                    s: (a, l.text.len()),
                }
            },
            color,
        );
    }

    pub(super) fn record_icon(&mut self, rect: Rect, icon: Icon, size_dp: f32, color: Color) {
        self.push_plain(Op::Icon(rect, icon, size_dp), color);
    }

    /// The blur reads every row: a barrier.
    pub(super) fn record_frost(&mut self, scale: u32, radius: u32, tint: Color) {
        self.push_plain(Op::Frost(scale, radius), tint);
    }

    /// The stretch moves rows between bands: a barrier.
    pub(super) fn record_shift(&mut self, rect: Rect, dy: i32) {
        self.push_plain(Op::Shift(rect, dy), Color::BLACK);
    }
}

