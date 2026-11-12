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

use alloc::string::String;
use alloc::vec::Vec;

mod bands;
pub use bands::Band;

use super::{Canvas, ShapeKey};
use crate::color::Color;
use crate::fonts::SizedFace;
use crate::geom::Rect;
use crate::widgets::Icon;

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
    text: String,
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
        if !self.text.is_empty() {
            zeroize::Zeroize::zeroize(&mut self.text);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::text::Font;
    use crate::widgets::Icon;
    use faraday_shell_api::DisplayInfo;

    fn canvas() -> Canvas {
        Canvas::new(&DisplayInfo {
            width: 200,
            height: 160,
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

    /// Every painting call, under several clips, drawn twice over.
    fn scene(c: &mut Canvas) -> i32 {
        let ink = Color::rgb(30, 90, 200);
        let glass = Color::rgb(250, 250, 250).with_alpha(120);
        c.clear(Color::rgb(240, 236, 230));
        c.fill_rect(Rect::new(4, 4, 90, 40), ink);
        c.fill_rect(Rect::new(10, 10, 300, 30), glass);
        c.fill_rounded_rect(Rect::new(20, 50, 100, 60), 14.0, ink);
        c.fill_rounded_rect(Rect::new(30, 60, 80, 40), 10.0, glass);
        c.fill_rounded_rect(Rect::new(0, 0, 60, 300), 20.0, ink);
        c.stroke_rounded_rect(Rect::new(130, 20, 60, 50), 12.0, 2.0, ink);
        c.line(5.0, 150.0, 190.0, 120.0, 3.0, ink);
        c.stroke_closed(&[(120.0, 90.0), (180.0, 100.0), (150.0, 140.0)], 2.5, ink);
        c.fill_circle(160.0, 120.0, 12.0, glass);
        c.fill_circle(40.0, 130.0, 9.5, ink);
        let tri: &[(f32, f32)] = &[(100.0, 100.0), (118.0, 100.0), (109.0, 118.0)];
        let sq: &[(f32, f32)] = &[
            (104.0, 104.0),
            (104.0, 108.0),
            (108.0, 108.0),
            (108.0, 104.0),
        ];
        c.fill_contours(&[tri, sq], ink);
        let key = ShapeKey::new(7, 50.0, 20.0, 1.0);
        let mark = || alloc::vec![alloc::vec![(50.0, 20.0), (80.0, 24.0), (60.0, 40.5)]];
        c.fill_kept(key, mark, ink);
        c.fill_kept(key, mark, glass);
        c.fill_cells((140, 80), (9, 3), |x, y| (x * 3 + y * 5) % 4 == 0, ink);
        c.shadow(
            Rect::new(60, 60, 50, 30),
            8.0,
            10.0,
            1.0,
            4,
            Color::BLACK.with_alpha(80),
        );
        c.elevation(Rect::new(20, 20, 40, 20), 6.0, 3, 1.0);
        let luma: alloc::vec::Vec<u8> = (0..24 * 16).map(|i| (i % 251) as u8).collect();
        let chroma: alloc::vec::Vec<u8> = (0..12 * 8 * 2).map(|i| (i * 7 % 256) as u8).collect();
        c.luma(Rect::new(110, 5, 60, 40), 24, 16, &luma, Some(&chroma));
        c.luma(Rect::new(110, 50, 30, 20), 24, 16, &luma, None);
        c.push_clip(Rect::new(15, 35, 120, 70));
        c.fill_rounded_rect(Rect::new(10, 30, 130, 80), 16.0, glass);
        c.fill_circle(60.0, 70.0, 20.0, ink);
        c.stroke_rounded_rect(Rect::new(10, 30, 130, 80), 16.0, 3.0, ink);
        c.push_clip(Rect::new(40, 50, 40, 40));
        c.fill_rect(Rect::new(0, 0, 200, 160), glass);
        c.line(0.0, 60.0, 120.0, 90.0, 2.0, ink);
        c.pop_clip();
        let w = c.text(20, 36, "Wallets and files", Font::regular(14.0), ink);
        c.pop_clip();
        c.icon(Rect::new(150, 130, 24, 24), Icon::Scan, 24.0, ink);
        let w = w + c.text(8, 100, "Recorded", Font::regular(14.0), ink);
        c.text_wrapped(
            Rect::new(70, 100, 60, 50),
            "a few words wrapped inside a narrow box",
            Font::regular(14.0),
            ink,
            crate::text::TextAlign::Start,
        );
        c.push_clip(Rect::new(0, 100, 200, 40));
        c.shift(Rect::new(0, 100, 200, 40), -7);
        c.pop_clip();
        c.frost(4, 3, Color::rgb(255, 255, 255).with_alpha(60));
        c.fill_rect(Rect::new(0, 0, 40, 40), ink);
        w
    }

    #[test]
    fn a_scene_recorded_then_replayed_is_the_scene_drawn() {
        let mut direct = canvas();
        let wide = scene(&mut direct);
        let mut recorded = canvas();
        recorded.record_calls(true);
        let same_width = scene(&mut recorded);
        let (calls, barriers) = recorded.waiting();
        assert!(
            calls > 20 && barriers == 2,
            "{calls} calls, {barriers} barriers"
        );
        recorded.replay();
        assert_eq!(recorded.waiting(), (0, 0));
        assert_eq!(wide, same_width);
        assert!(direct.frame().rgba == recorded.frame().rgba);
        assert_eq!(direct.lines_drawn(), recorded.lines_drawn());
    }

    #[test]
    fn a_scene_recorded_again_paints_the_same_pixels() {
        let mut direct = canvas();
        scene(&mut direct);
        let mut recorded = canvas();
        recorded.record_calls(true);
        for _ in 0..2 {
            scene(&mut recorded);
            recorded.replay();
        }
        assert!(direct.frame().rgba == recorded.frame().rgba);
    }

    #[test]
    fn a_call_that_reads_the_canvas_paints_what_waits_first() {
        let ink = Color::rgb(30, 90, 200);
        let mut direct = canvas();
        direct.fill_rect(Rect::new(10, 10, 50, 50), ink);
        let kept = direct.snapshot();
        direct.fill_circle(40.0, 40.0, 10.0, Color::BLACK);
        direct.restore(&kept);
        let mut recorded = canvas();
        recorded.record_calls(true);
        recorded.fill_rect(Rect::new(10, 10, 50, 50), ink);
        let at = recorded.snapshot();
        assert_eq!(recorded.waiting(), (0, 0));
        recorded.fill_circle(40.0, 40.0, 10.0, Color::BLACK);
        assert_eq!(recorded.waiting().0, 1);
        recorded.restore(&at);
        assert_eq!(recorded.waiting(), (0, 0));
        assert!(direct.frame().rgba == recorded.frame().rgba);
    }

    /// Paints the waiting list in `n` bands, one after the other.
    fn replay_in(c: &mut Canvas, n: usize) {
        c.replay_banded(n, &mut |bands| {
            for b in bands {
                b.paint();
            }
        });
    }

    #[test]
    fn a_scene_painted_in_bands_is_the_scene_drawn() {
        let mut direct = canvas();
        let wide = scene(&mut direct);
        for n in [1, 2, 3, 4, 5, 7, 11, 160] {
            let mut banded = canvas();
            banded.record_calls(true);
            let same_width = scene(&mut banded);
            replay_in(&mut banded, n);
            assert_eq!(banded.waiting(), (0, 0));
            assert_eq!(wide, same_width);
            let (a, b) = (direct.frame().rgba.to_vec(), banded.frame().rgba.to_vec());
            if let Some(i) = (0..a.len()).find(|&i| a[i] != b[i]) {
                let bad = (0..a.len()).filter(|&i| a[i] != b[i]).count();
                panic!(
                    "{n} bands differ at x {} y {}, {bad} bytes",
                    i / 4 % 200,
                    i / 800
                );
            }
        }
    }

    #[test]
    fn a_path_across_a_band_edge_is_painted_whole() {
        let ink = Color::rgb(30, 90, 200);
        let draw = |c: &mut Canvas| {
            c.clear(Color::rgb(240, 236, 230));
            // Edges of two to eleven bands run through each of these.
            c.fill_rounded_rect(Rect::new(10, 10, 60, 140), 18.0, ink);
            c.stroke_rounded_rect(Rect::new(80, 5, 50, 150), 14.0, 1.0, ink);
            c.stroke_rounded_rect(Rect::new(80, 5, 50, 150), 14.0, 2.5, ink);
            c.fill_circle(150.5, 80.25, 30.0, ink);
            c.fill_circle(30.0, 5.0, 12.0, ink);
            c.line(5.0, 3.0, 190.0, 157.0, 0.5, ink);
            c.line(5.0, 3.0, 190.0, 157.0, 1.0, ink);
            c.line(100.0, 0.0, 100.0, 160.0, 4.0, ink);
            c.line(0.0, 80.0, 200.0, 80.0, 2.0, ink);
            c.line(20.0, 20.0, 60.0, 30.0, 3.0, ink);
            // Thin strokes inside a band that is not the first, moved by
            // whole rows to be drawn there.
            for (i, w) in [0.2, 0.5, 1.0, 1.0, 1.5].into_iter().enumerate() {
                let y = 100.3 + 9.7 * i as f32;
                c.line(10.3, y, 70.6, y + 4.9, w, ink);
                c.stroke_closed(&[(120.4, y), (180.1, y + 3.3), (150.5, y + 8.0)], w, ink);
            }
            c.stroke_closed(&[(110.0, 10.0), (190.0, 70.0), (120.0, 150.0)], 1.0, ink);
        };
        let mut direct = canvas();
        draw(&mut direct);
        for n in [2, 3, 4, 6, 9, 16] {
            let mut banded = canvas();
            banded.record_calls(true);
            draw(&mut banded);
            replay_in(&mut banded, n);
            assert!(
                direct.frame().rgba == banded.frame().rgba,
                "{n} bands differ"
            );
        }
    }

    #[test]
    fn more_kept_shapes_than_the_cache_holds_still_draw() {
        let ink = Color::rgb(30, 90, 200);
        let draw = |c: &mut Canvas| {
            c.clear(Color::WHITE);
            for i in 0..50 {
                let key = ShapeKey::new(100 + i, 10.0, 10.0, 1.0);
                let (x, y) = ((i % 8) as f32 * 24.0, (i / 8) as f32 * 24.0);
                let tri = move || {
                    alloc::vec![alloc::vec![
                        (x + 2.0, y + 2.0),
                        (x + 20.0, y + 4.0),
                        (x + 9.0, y + 20.5)
                    ]]
                };
                c.fill_kept(key, tri, ink);
            }
        };
        let mut direct = canvas();
        direct.record_calls(true);
        draw(&mut direct);
        direct.replay();
        let mut banded = canvas();
        banded.record_calls(true);
        draw(&mut banded);
        replay_in(&mut banded, 3);
        assert!(direct.frame().rgba == banded.frame().rgba);
        let mut plain = canvas();
        draw(&mut plain);
        assert!(plain.frame().rgba == banded.frame().rgba);
    }

    #[test]
    fn only_a_blur_and_a_stretch_wait_for_the_other_bands() {
        let mut c = canvas();
        c.record_calls(true);
        let ink = Color::BLACK;
        c.line(0.0, 10.0, 50.0, 20.0, 2.0, ink);
        c.line(0.0, 10.0, 50.0, 150.0, 2.0, ink);
        c.stroke_rounded_rect(Rect::new(10, 10, 80, 100), 30.0, 2.0, ink);
        c.shift(Rect::new(0, 0, 10, 10), 1);
        for n in [1, 2, 4] {
            assert_eq!(c.barriers_in(n), 1);
        }
    }

    #[test]
    fn a_stroked_card_across_a_band_edge_is_painted_whole() {
        let mk = || {
            Canvas::new(&DisplayInfo {
                width: 480,
                height: 640,
                dpi: 286,
                inset_bottom: 0,
                inset_top: 0,
                buttons: 0,
                camera_fixed: true,
                secure: faraday_shell_api::SecureHardware::None,
                boot: faraday_shell_api::BootState::Unknown,
                memory_mib: None,
            })
        };
        for alpha in [255u8, 120] {
            let ink = Color::rgb(30, 90, 200).with_alpha(alpha);
            let draw = |c: &mut Canvas| {
                c.clear(Color::rgb(240, 236, 230));
                if alpha == 120 {
                    c.push_clip(Rect::new(14, 32, 452, 476));
                }
                c.stroke_rounded_rect(Rect::new(43, 415, 250, 71), 35.75, 1.7875, ink);
                c.pop_clip();
            };
            let mut d = mk();
            draw(&mut d);
            let mut bad = alloc::vec::Vec::new();
            for n in 2..60 {
                let mut b = mk();
                b.record_calls(true);
                draw(&mut b);
                replay_in(&mut b, n);
                let (p, q) = (d.frame().rgba.to_vec(), b.frame().rgba.to_vec());
                let at: alloc::vec::Vec<_> = (0..p.len())
                    .filter(|&i| p[i] != q[i])
                    .map(|i| (i / 4 % 480, i / 4 / 480, p[i], q[i]))
                    .collect();
                if !at.is_empty() {
                    bad.push((n, 640 / n, at.len()));
                }
            }
            assert!(bad.is_empty(), "alpha {alpha} {bad:?}");
        }
    }

    #[test]
    fn paths_across_band_edges_and_below_the_first_band_are_painted_whole() {
        let mk = || {
            Canvas::new(&DisplayInfo {
                width: 480,
                height: 640,
                dpi: 286,
                inset_bottom: 0,
                inset_top: 0,
                buttons: 0,
                camera_fixed: true,
                secure: faraday_shell_api::SecureHardware::None,
                boot: faraday_shell_api::BootState::Unknown,
                memory_mib: None,
            })
        };
        let zig: [(f32, f32); 5] = [
            (30.0, 20.0),
            (200.0, 60.0),
            (420.0, 25.0),
            (330.0, 560.0),
            (60.0, 600.0),
        ];
        let tri: [(f32, f32); 3] = [(300.0, 100.0), (460.0, 130.0), (350.0, 330.0)];
        for alpha in [255u8, 120] {
            let ink = Color::rgb(30, 90, 200).with_alpha(alpha);
            let draw = |c: &mut Canvas| {
                c.clear(Color::rgb(240, 236, 230));
                c.push_clip(Rect::new(14, 32, 452, 476));
                for y in [20, 130, 250, 415] {
                    c.stroke_rounded_rect(Rect::new(43, y, 250, 71), 35.75, 1.7875, ink);
                    c.fill_rounded_rect(Rect::new(23, y + 10, 190, 90), 20.0, ink);
                    c.fill_rounded_rect(Rect::new(210, y, 160, 50), 12.0, ink);
                    c.fill_circle(340.0, y as f32 + 30.0, 40.0, ink);
                    c.fill_circle(100.5, y as f32 + 12.25, 9.0, ink);
                    c.line(5.0, y as f32, 470.0, y as f32 + 33.5, 2.5, ink);
                }
                c.line(100.0, 5.0, 130.0, 620.0, 3.0, ink);
                c.stroke_closed(&zig, 3.0, ink);
                c.stroke_closed(&tri, 2.0, ink);
                c.fill_contours(&[&tri[..], &zig[..]], ink);
                c.pop_clip();
                c.fill_rounded_rect(Rect::new(30, 100, 400, 480), 36.0, ink);
            };
            let mut d = mk();
            draw(&mut d);
            let mut bad = alloc::vec::Vec::new();
            for n in 2..60 {
                let mut b = mk();
                b.record_calls(true);
                draw(&mut b);
                replay_in(&mut b, n);
                let differ = d
                    .frame()
                    .rgba
                    .iter()
                    .zip(b.frame().rgba.iter())
                    .filter(|(p, q)| p != q)
                    .count();
                if differ > 0 {
                    bad.push((n, differ));
                }
            }
            assert!(bad.is_empty(), "alpha {alpha} {bad:?}");
        }
    }

    #[test]
    fn nothing_painted_is_left_behind_in_the_bands_scratch() {
        let mut c = canvas();
        c.record_calls(true);
        c.push_clip(Rect::new(0, 0, 100, 100));
        c.fill_rounded_rect(Rect::new(5, 5, 90, 300), 20.0, Color::rgb(200, 10, 10));
        c.stroke_closed(
            &[(5.0, 5.0), (80.0, 40.0), (20.0, 150.0)],
            2.0,
            Color::WHITE,
        );
        c.pop_clip();
        replay_in(&mut c, 4);
        assert!(c.scratch_is_clear());
    }
}
