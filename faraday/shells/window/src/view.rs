//! The window and its drawing surface: an app's frame copied in and
//! presented, and what the screen says about itself.

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use faraday_shell_api::{
    App, BootState, Changed, Command, DisplayInfo, Event, Frame, Parts, SecureHardware,
};
use softbuffer::{Context, Surface};
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowAttributes};

use crate::pace::{FRAME, Pacer};

/// What the frame does not cover, for the moment after a resize: the
/// page colour.
const BEYOND: u32 = 0x0011_161c;

/// A window with a surface to draw into.
pub struct View {
    /// The window.
    pub window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    /// What the surface's buffers were last given ([`Shown`]).
    shown: Shown,
}

impl View {
    /// Opens a window with `attributes`, or says why not.
    pub fn open(
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
    ) -> Result<View, String> {
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Rc::new(w),
            Err(e) => return Err(format!("cannot open a window: {e}")),
        };
        let Ok(context) = Context::new(window.clone()) else {
            return Err(String::from("cannot create a drawing context"));
        };
        let Ok(surface) = Surface::new(&context, window.clone()) else {
            return Err(String::from("cannot create a drawing surface"));
        };
        Ok(View {
            window,
            surface,
            shown: Shown::default(),
        })
    }

    /// Whether the compositor paces the frames. Wayland delivers a
    /// redraw with the screen's refresh once a frame has been presented;
    /// X11 delivers it at once, so there the timer keeps the pace.
    pub fn paced(&self) -> bool {
        matches!(
            self.window.window_handle().map(|h| h.as_raw()),
            Ok(RawWindowHandle::Wayland(_))
        )
    }

    /// The screen's refresh period, from the monitor the window is on;
    /// 60 Hz when it does not say.
    pub fn refresh(&self) -> Duration {
        refresh(Some(&self.window))
    }

    /// Draws `app`'s frame into the window and presents it. A frame that
    /// asks for the next one (a fade part of the way) gets a redraw from
    /// here: no event is coming to ask on its behalf. Returns whether
    /// `--timings` printed its line with this frame.
    pub fn draw<A: App + ?Sized>(&mut self, app: &mut A, pacer: &mut Pacer) -> bool {
        let window = &self.window;
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return false;
        };
        if self.surface.resize(w, h).is_err() {
            return false;
        }
        // The compositor may still hold the buffer: the wait for it is
        // part of the gap between frames, and timed.
        let asked = Instant::now();
        let Ok(mut buffer) = self.surface.buffer_mut() else {
            return false;
        };
        let begun = Instant::now();
        let timed = pacer.timed();
        let start = timed.then_some(begun);
        let parts = app.parts();
        let rendered = timed.then(Instant::now);
        let age = buffer.age();
        let bands = self
            .shown
            .copy(&parts, &mut buffer, size.width as usize, age);
        // Only what changed since the last frame is given to the
        // compositor to take again; a frame that changed nothing still
        // presents a row, so the compositor still says when it is shown.
        let damage: Vec<softbuffer::Rect> = bands
            .iter()
            .filter_map(|a| {
                Some(softbuffer::Rect {
                    x: u32::try_from(a.x).ok()?,
                    y: u32::try_from(a.y).ok()?,
                    width: NonZeroU32::new(u32::try_from(a.w).ok()?)?,
                    height: NonZeroU32::new(u32::try_from(a.h).ok()?)?,
                })
            })
            .collect();
        let one = [softbuffer::Rect {
            x: 0,
            y: 0,
            width: w,
            height: NonZeroU32::MIN,
        }];
        // Tells winit a frame is about to be shown: on Wayland it asks
        // the compositor to say when the frame is on the screen, and
        // holds the next redraw until then, which is what paces the
        // frames to the screen's refresh.
        window.pre_present_notify();
        let damage = if damage.is_empty() {
            &one[..]
        } else {
            &damage[..]
        };
        if buffer.present_with_damage(damage).is_err() {
            // What the buffers hold is no longer known.
            self.shown = Shown::default();
        }
        let mut again = false;
        while let Some(c) = app.poll_command() {
            again |= matches!(c, Command::Draw);
        }
        if again {
            window.request_redraw();
        }
        if !pacer.paced {
            pacer.drew(begun.elapsed(), refresh(Some(window)));
        }
        if let (Some(start), Some(rendered)) = (start, rendered) {
            let now = Instant::now();
            return pacer.presented(begun - asked, rendered - start, now - rendered, now);
        }
        false
    }
}

/// Costs a pass over the window's pixels. Copies `parts` into `buffer`,
/// a window `sw` pixels wide, each pixel once: the premultiplied RGBA to
/// the window's 0RGB, a row at a time; the inner frame where it stands,
/// the base everywhere else; what neither covers (the moment after a
/// resize) the page colour.
pub fn copy(parts: &Parts<'_>, buffer: &mut [u32], sw: usize) {
    if sw == 0 {
        return;
    }
    for (y, dst) in buffer.chunks_exact_mut(sw).enumerate() {
        put(&row(parts, y, sw), dst);
    }
}

/// A rectangle of the window's pixels: columns from `x`, `w` wide, rows
/// from `y`, `h` high.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    /// Left edge.
    pub x: usize,
    /// Top edge.
    pub y: usize,
    /// Width.
    pub w: usize,
    /// Height.
    pub h: usize,
}

impl Area {
    /// Whether `other` lies wholly inside this one.
    fn holds(&self, other: &Area) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.w <= self.x + self.w
            && other.y + other.h <= self.y + self.h
    }
}

/// One frame a window's surface was given: the window's width, a hash of
/// each row (or a mark never given twice, for a row a frame changed
/// without hashing it), and the rectangles that frame changed, where the
/// app said them.
#[derive(Debug)]
struct Given {
    width: usize,
    rows: Vec<u64>,
    said: Option<Vec<Area>>,
}

/// What a window's surface was given in its last two frames, newest
/// first. The rows a buffer already holds are not copied again, and only
/// what differs from the frame last presented is damaged.
#[derive(Debug, Default)]
pub struct Shown {
    frames: [Option<Given>; 2],
    /// Marks given out so far, for rows changed without a hash.
    marks: u64,
}

impl Shown {
    /// Costs a copy of what changed, and a hash of every row where the
    /// app does not say what changed. Copies `parts` into `buffer`, a
    /// window `sw` pixels wide that holds what it was given `age` frames
    /// ago (softbuffer's age: 1 the last frame, 2 the one before, 0 not
    /// known), as [`copy`] does, but only where it differs from what the
    /// buffer holds. Where the parts say what changed
    /// ([`Parts::changed`]) only that is copied and nothing is hashed;
    /// else every row is hashed. Returns what differs from the frame
    /// last presented: the damage to present.
    pub fn copy(&mut self, parts: &Parts<'_>, buffer: &mut [u32], sw: usize, age: u8) -> Vec<Area> {
        if sw == 0 {
            return Vec::new();
        }
        let rows = buffer.len() / sw;
        if let Some(said) = parts.changed
            && at_size(&self.frames[0], sw, rows).is_some()
        {
            return self.copy_said(parts, said, buffer, sw, age);
        }
        let held = match age {
            1 => at_size(&self.frames[0], sw, rows),
            2 => at_size(&self.frames[1], sw, rows),
            _ => None,
        };
        let last = at_size(&self.frames[0], sw, rows);
        let mut hashes = Vec::with_capacity(rows);
        let mut bands: Vec<Area> = Vec::new();
        for (y, dst) in buffer.chunks_exact_mut(sw).enumerate() {
            let pieces = row(parts, y, sw);
            let h = hash(&pieces);
            if held.is_none_or(|g| g.rows[y] != h) {
                put(&pieces, dst);
            }
            if last.is_none_or(|g| g.rows[y] != h) {
                match bands.last_mut() {
                    Some(b) if b.y + b.h == y => b.h += 1,
                    _ => bands.push(Area {
                        x: 0,
                        y,
                        w: sw,
                        h: 1,
                    }),
                }
            }
            hashes.push(h);
        }
        let newest = self.frames[0].take();
        self.frames = [
            Some(Given {
                width: sw,
                rows: hashes,
                said: None,
            }),
            newest,
        ];
        bands
    }

    /// Costs a copy of what changed. [`Shown::copy`] where the parts say
    /// what changed since the last frame, `said`, which the surface was
    /// given at this size: only that is copied into `buffer`, with what
    /// changed between the frame the buffer holds and the last, and
    /// nothing is hashed.
    fn copy_said(
        &mut self,
        parts: &Parts<'_>,
        said: &[Changed],
        buffer: &mut [u32],
        sw: usize,
        age: u8,
    ) -> Vec<Area> {
        let rows = buffer.len() / sw;
        let Some(last) = at_size(&self.frames[0], sw, rows) else {
            return Vec::new();
        };
        let said: Vec<Area> = said
            .iter()
            .filter_map(|c| {
                let (x, y) = (usize::from(c.x), usize::from(c.y));
                let w = usize::from(c.width).min(sw.saturating_sub(x));
                let h = usize::from(c.height).min(rows.saturating_sub(y));
                (w > 0 && h > 0).then_some(Area { x, y, w, h })
            })
            .collect();
        let mut marked = last.rows.clone();
        self.marks += 1;
        for a in &said {
            for (y, m) in marked.iter_mut().enumerate().skip(a.y).take(a.h) {
                *m = mark(self.marks, y);
            }
        }
        // What the buffer lacks: what this frame changed, and what
        // changed between the frame it holds and the last.
        let between: Option<Vec<Area>> = match (age, at_size(&self.frames[1], sw, rows)) {
            (1, _) => Some(Vec::new()),
            (2, Some(older)) => Some(last.said.clone().unwrap_or_else(|| {
                (0..rows)
                    .filter(|&y| older.rows[y] != last.rows[y])
                    .map(|y| Area {
                        x: 0,
                        y,
                        w: sw,
                        h: 1,
                    })
                    .collect()
            })),
            _ => None,
        };
        match between {
            Some(mut between) => {
                // A fade's steps change the same bands each frame: what
                // this frame writes anyway is not written twice.
                between.retain(|b| !said.iter().any(|s| s.holds(b)));
                for a in said.iter().chain(&between) {
                    for y in a.y..a.y + a.h {
                        let dst = &mut buffer[y * sw..(y + 1) * sw];
                        put_span(&row(parts, y, sw), dst, a.x, a.x + a.w);
                    }
                }
            }
            None => copy(parts, buffer, sw),
        }
        let newest = self.frames[0].take();
        self.frames = [
            Some(Given {
                width: sw,
                rows: marked,
                said: Some(said.clone()),
            }),
            newest,
        ];
        said
    }
}

/// A frame given at this size, `sw` wide and `rows` high, if it was.
fn at_size(f: &Option<Given>, sw: usize, rows: usize) -> Option<&Given> {
    f.as_ref().filter(|g| g.width == sw && g.rows.len() == rows)
}

/// A mark for row `y` of the frame `n`th to be given without a hash: no
/// two are alike, and one is as unlikely to equal a row's hash as two
/// different rows' hashes are to be equal.
fn mark(n: u64, y: usize) -> u64 {
    (n.rotate_left(32) ^ y as u64 ^ 0x6a09_e667_f3bc_c908).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

/// One piece of a window's row: a frame's pixels, then the page colour
/// for this many more.
type Piece<'a> = (&'a [u8], usize);

/// The pieces of the window's row `y`, `sw` pixels wide: the base to the
/// left of the inner frame, the inner frame and the base to its right,
/// or the base alone where the inner frame does not reach.
fn row<'a>(parts: &Parts<'a>, y: usize, sw: usize) -> [Piece<'a>; 3] {
    let base = parts.base;
    let by = y.checked_sub(usize::from(parts.base_y));
    match parts.inner {
        Some((f, ix, iy))
            if y >= usize::from(iy)
                && y < usize::from(iy) + usize::from(f.height)
                && usize::from(ix) < sw =>
        {
            let ix = usize::from(ix);
            let iw = usize::from(f.width).min(sw - ix);
            [
                piece(&base, by, 0, ix),
                piece(&f, Some(y - usize::from(iy)), 0, iw),
                piece(&base, by, ix + iw, sw - ix - iw),
            ]
        }
        _ => [piece(&base, by, 0, sw), (&[], 0), (&[], 0)],
    }
}

/// Row `y` of `frame` (none where it is not the frame's) from its column
/// `from`, `len` pixels: the frame's pixels, the page colour for the
/// rest where the frame ends.
fn piece<'a>(frame: &Frame<'a>, y: Option<usize>, from: usize, len: usize) -> Piece<'a> {
    let (fw, fh) = (usize::from(frame.width), usize::from(frame.height));
    let shown = match y {
        Some(y) if y < fh => fw.saturating_sub(from).min(len),
        _ => 0,
    };
    if shown == 0 {
        return (&[], len);
    }
    let at = (y.unwrap_or(0) * fw + from) * 4;
    (&frame.rgba[at..at + shown * 4], len - shown)
}

/// Costs a pass over the row's pixels. Converts the row's pieces into
/// `dst`, a pixel at a time read as one big-endian word and shifted:
/// premultiplied RGBA to the window's 0RGB.
fn put(pieces: &[Piece<'_>; 3], dst: &mut [u32]) {
    put_span(pieces, dst, 0, dst.len());
}

/// Costs a pass over the span's pixels. [`put`] for the row's columns
/// `[from, to)` alone.
fn put_span(pieces: &[Piece<'_>; 3], dst: &mut [u32], from: usize, to: usize) {
    let mut x = 0;
    for &(src, beyond) in pieces {
        let n = src.len() / 4;
        let (a, b) = (from.max(x), to.min(x + n));
        if a < b {
            let src = &src[(a - x) * 4..(b - x) * 4];
            for (d, px) in dst[a..b].iter_mut().zip(src.chunks_exact(4)) {
                *d = u32::from_be_bytes([px[0], px[1], px[2], px[3]]) >> 8;
            }
        }
        x += n;
        let (a, b) = (from.max(x), to.min(x + beyond));
        if a < b {
            dst[a..b].fill(BEYOND);
        }
        x += beyond;
    }
}

/// Costs a pass over the row's pixels. A hash of the row's pieces, eight
/// words of eight bytes at a time in eight lanes: rows whose pieces hold
/// the same pixels in the same places hash the same, and a row with one
/// word changed never hashes as it did, since each step of a lane is
/// one-to-one.
fn hash(pieces: &[Piece<'_>; 3]) -> u64 {
    const K: u64 = 0x9e37_79b9_7f4a_7c15;
    let mix = |l: u64, w: u64| (l.rotate_left(23) ^ w).wrapping_mul(K);
    // Eight bytes from `b` at `i`, little-endian; zeros past its end.
    let word = |b: &[u8], i: usize| {
        let mut w = [0u8; 8];
        for (k, v) in w.iter_mut().enumerate() {
            *v = b.get(i + k).copied().unwrap_or(0);
        }
        u64::from_le_bytes(w)
    };
    let mut lanes = [1u64, 2, 3, 4, 5, 6, 7, 8];
    for &(src, beyond) in pieces {
        let mut blocks = src.chunks_exact(64);
        for b in &mut blocks {
            for (k, l) in lanes.iter_mut().enumerate() {
                let i = 8 * k;
                let w = u64::from_le_bytes([
                    b[i],
                    b[i + 1],
                    b[i + 2],
                    b[i + 3],
                    b[i + 4],
                    b[i + 5],
                    b[i + 6],
                    b[i + 7],
                ]);
                *l = mix(*l, w);
            }
        }
        let rest = blocks.remainder();
        for i in (0..rest.len()).step_by(8) {
            lanes[0] = mix(lanes[0], word(rest, i));
        }
        lanes[1] = mix(lanes[1], (src.len() as u64) << 32 | beyond as u64);
    }
    lanes
        .iter()
        .enumerate()
        .fold(0, |h, (k, l)| h ^ l.rotate_left(8 * k as u32))
}

/// The refresh period of the monitor `window` is on; 60 Hz when there is
/// no window or it does not say.
pub fn refresh(window: Option<&Window>) -> Duration {
    window
        .and_then(|w| w.current_monitor())
        .and_then(|m| m.refresh_rate_millihertz())
        .filter(|&mhz| mhz > 0)
        .map_or(FRAME, |mhz| {
            Duration::from_secs_f64(1000.0 / f64::from(mhz))
        })
}

/// The display event for a window `shown` pixels in size at the
/// compositor's `scale`, or standing for a device's screen at `ppi`
/// pixels per inch.
pub fn display((w, h): (u32, u32), scale: f64, ppi: Option<f64>) -> Event {
    Event::Display(DisplayInfo {
        width: w.clamp(320, u32::from(u16::MAX)) as u16,
        height: h.clamp(240, u32::from(u16::MAX)) as u16,
        // The compositor's scale for this window, as a density: the
        // app grows no larger than a quarter beyond it. Standing for
        // a device's screen, this screen's own density, so the app
        // is laid out at that screen's true size.
        dpi: match ppi {
            Some(p) => p.round().clamp(80.0, 640.0) as u16,
            None => (160.0 * scale).round().clamp(80.0, 640.0) as u16,
        },
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame `w` × `h` of one colour.
    fn solid(w: u16, h: u16, rgb: [u8; 3]) -> Vec<u8> {
        let px = [rgb[0], rgb[1], rgb[2], 255];
        px.repeat(usize::from(w) * usize::from(h))
    }

    #[test]
    fn the_window_shows_the_inner_frame_where_it_stands_and_the_base_round_it() {
        let base = solid(6, 4, [1, 2, 3]);
        let inner = solid(3, 2, [9, 8, 7]);
        let parts = Parts {
            base: Frame {
                width: 6,
                height: 4,
                rgba: &base,
            },
            base_y: 0,
            inner: Some((
                Frame {
                    width: 3,
                    height: 2,
                    rgba: &inner,
                },
                2,
                1,
            )),
            changed: None,
        };
        // A window a column wider and a row taller than the frame: the
        // moment after a resize.
        let mut buffer = vec![0u32; 7 * 5];
        copy(&parts, &mut buffer, 7);
        for y in 0..5 {
            for x in 0..7 {
                let want = if x >= 6 || y >= 4 {
                    BEYOND
                } else if (2..5).contains(&x) && (1..3).contains(&y) {
                    0x0009_0807
                } else {
                    0x0001_0203
                };
                assert_eq!(buffer[y * 7 + x], want, "at {x}, {y}");
            }
        }
    }

    #[test]
    fn the_app_above_and_a_strip_at_the_foot_fill_the_window() {
        // Faraday Demo at the window's own size: the app's frame at the
        // top, the host's strip below it, each copied once.
        let app = solid(5, 3, [9, 8, 7]);
        let strip = solid(5, 2, [1, 2, 3]);
        let parts = Parts {
            base: Frame {
                width: 5,
                height: 2,
                rgba: &strip,
            },
            base_y: 3,
            inner: Some((
                Frame {
                    width: 5,
                    height: 3,
                    rgba: &app,
                },
                0,
                0,
            )),
            changed: None,
        };
        let mut buffer = vec![0u32; 5 * 6];
        copy(&parts, &mut buffer, 5);
        for (i, px) in buffer.iter().enumerate() {
            let want = match i / 5 {
                0..3 => 0x0009_0807,
                3..5 => 0x0001_0203,
                _ => BEYOND,
            };
            assert_eq!(*px, want, "at {}, {}", i % 5, i / 5);
        }
    }

    /// A little generator of test pixels.
    struct Noise(u64);

    impl Noise {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn frame(&mut self, w: u16, h: u16) -> Vec<u8> {
            (0..usize::from(w) * usize::from(h) * 4)
                .map(|_| self.next() as u8)
                .collect()
        }
    }

    /// The window as the copy before words did it: each pixel's red,
    /// green and blue bytes, shifted into place one at a time.
    fn bytewise(frame: &[u8], sw: usize, rows: usize) -> Vec<u32> {
        let mut out = vec![BEYOND; sw * rows];
        for (d, px) in out.iter_mut().zip(frame.chunks_exact(4)) {
            *d = (u32::from(px[0]) << 16) | (u32::from(px[1]) << 8) | u32::from(px[2]);
        }
        out
    }

    fn whole(rgba: &[u8], w: u16, h: u16) -> Parts<'_> {
        Parts {
            base: Frame {
                width: w,
                height: h,
                rgba,
            },
            base_y: 0,
            inner: None,
            changed: None,
        }
    }

    #[test]
    fn the_copy_shows_every_pixel_as_the_copy_a_byte_at_a_time_did() {
        let mut noise = Noise(0x2545_f491_4f6c_dd1d);
        for (w, h) in [(1u16, 1u16), (7, 3), (64, 9), (333, 17)] {
            let rgba = noise.frame(w, h);
            let sw = usize::from(w);
            let mut buffer = vec![0u32; sw * usize::from(h)];
            copy(&whole(&rgba, w, h), &mut buffer, sw);
            assert!(
                buffer == bytewise(&rgba, sw, usize::from(h)),
                "at {w} × {h}"
            );
        }
    }

    /// Frames `w` × `h` presented through a surface of `n` buffers taken
    /// in turn, as softbuffer does (two on Wayland, one on X11), the
    /// buffers' age as softbuffer gives it: each frame changes a few
    /// random rows of the last. After every frame the buffer shown holds
    /// the frame, and the damage covers every row that changed.
    fn present_through(n: usize, w: u16, h: u16, frames: usize) {
        let (sw, rows) = (usize::from(w), usize::from(h));
        let mut noise = Noise(0x9e37_79b9 + n as u64);
        let mut shown = Shown::default();
        let mut buffers = vec![(vec![0u32; sw * rows], 0u8); n];
        let mut frame = noise.frame(w, h);
        let mut last: Option<Vec<u32>> = None;
        for i in 0..frames {
            // A few rows changed, now and then none.
            for _ in 0..(noise.next() % 4) {
                let y = (noise.next() % rows as u64) as usize;
                let x = (noise.next() % sw as u64) as usize;
                frame[(y * sw + x) * 4] ^= 1 + (noise.next() % 255) as u8;
            }
            let b = i % n;
            let age = buffers[b].1;
            let bands = shown.copy(&whole(&frame, w, h), &mut buffers[b].0, sw, age);
            let want = bytewise(&frame, sw, rows);
            assert!(buffers[b].0 == want, "{n} buffers: frame {i} shown whole");
            if let Some(prev) = &last {
                for y in 0..rows {
                    let changed = prev[y * sw..(y + 1) * sw] != want[y * sw..(y + 1) * sw];
                    let damaged = bands.iter().any(|a| (a.y..a.y + a.h).contains(&y));
                    assert_eq!(damaged, changed, "{n} buffers: frame {i}, row {y}");
                }
            }
            last = Some(want);
            // Presented: this buffer is the newest, the others a frame
            // older (softbuffer's age; 0 until a buffer is first shown).
            for (k, (_, a)) in buffers.iter_mut().enumerate() {
                if k == b {
                    *a = 1;
                } else if *a != 0 {
                    *a += 1;
                }
            }
        }
    }

    /// As [`present_through`], but most frames say where they changed, a
    /// few rectangles, the way a fade's steps do, and every few frames do
    /// not; each frame still shows whole, and its damage covers every
    /// pixel that changed.
    fn present_said(n: usize, w: u16, h: u16, frames: usize) {
        use faraday_shell_api::Changed;
        let (sw, rows) = (usize::from(w), usize::from(h));
        let mut noise = Noise(0x5851_f42d + n as u64);
        let mut shown = Shown::default();
        let mut buffers = vec![(vec![0u32; sw * rows], 0u8); n];
        let mut frame = noise.frame(w, h);
        let mut last: Option<Vec<u32>> = None;
        for i in 0..frames {
            let mut said = Vec::new();
            for _ in 0..(noise.next() % 3) {
                let (x, y) = (
                    (noise.next() % u64::from(w)) as u16,
                    (noise.next() % u64::from(h)) as u16,
                );
                let cw = 1 + (noise.next() % u64::from(w - x)) as u16;
                let ch = 1 + (noise.next() % u64::from(h - y)) as u16;
                for yy in y..y + ch {
                    for xx in x..x + cw {
                        let at = (usize::from(yy) * sw + usize::from(xx)) * 4;
                        frame[at + 1] = frame[at + 1].wrapping_add(1 + (noise.next() % 200) as u8);
                    }
                }
                said.push(Changed {
                    x,
                    y,
                    width: cw,
                    height: ch,
                });
            }
            let mut parts = whole(&frame, w, h);
            if i % 7 != 3 {
                parts.changed = Some(&said);
            }
            let b = i % n;
            let age = buffers[b].1;
            let bands = shown.copy(&parts, &mut buffers[b].0, sw, age);
            let want = bytewise(&frame, sw, rows);
            assert!(buffers[b].0 == want, "{n} buffers: frame {i} shown whole");
            if let Some(prev) = &last {
                for (k, (p, q)) in prev.iter().zip(&want).enumerate() {
                    let (x, y) = (k % sw, k / sw);
                    let damaged = bands
                        .iter()
                        .any(|a| (a.x..a.x + a.w).contains(&x) && (a.y..a.y + a.h).contains(&y));
                    assert!(
                        p == q || damaged,
                        "{n} buffers: frame {i}, {x}, {y} changed undamaged"
                    );
                }
            }
            last = Some(want);
            for (k, (_, a)) in buffers.iter_mut().enumerate() {
                if k == b {
                    *a = 1;
                } else if *a != 0 {
                    *a += 1;
                }
            }
        }
    }

    #[test]
    fn faraday_demos_window_shows_every_step_of_its_fades_whole() {
        use faraday_core::{Action, Faraday, Host, Screen};
        let (w, h) = (900u16, 560u16);
        let sw = usize::from(w);
        let mut host = Host::new(Faraday::new());
        let Event::Display(mut d) = display((u32::from(w), u32::from(h)), 1.0, None) else {
            unreachable!()
        };
        d.camera_fixed = false;
        host.event(Event::Display(d));
        let mut shown = Shown::default();
        let mut buffers = vec![(vec![0u32; sw * usize::from(h)], 0u8); 2];
        let mut whole_frame = vec![0u32; sw * usize::from(h)];
        let (mut now, mut i, mut said) = (1_000u64, 0usize, 0);
        let changes = [
            Action::Nav(Screen::Settings),
            Action::Learn,
            Action::Cancel,
            Action::Nav(Screen::Home),
        ];
        for change in changes {
            host.app.press(change);
            for _ in 0..24 {
                now += 16;
                host.event(Event::Tick { now_ms: now });
                while host.poll_command().is_some() {}
                let parts = host.parts();
                said += usize::from(parts.changed.is_some());
                let b = i % 2;
                let age = buffers[b].1;
                let _ = shown.copy(&parts, &mut buffers[b].0, sw, age);
                copy(&parts, &mut whole_frame, sw);
                assert!(buffers[b].0 == whole_frame, "frame {i} shown whole");
                for (k, (_, a)) in buffers.iter_mut().enumerate() {
                    *a = if k == b {
                        1
                    } else if *a != 0 {
                        *a + 1
                    } else {
                        0
                    };
                }
                i += 1;
            }
        }
        assert!(said > 8, "a fade's steps say what they changed ({said})");
    }

    #[test]
    fn a_surface_given_only_what_a_fades_steps_changed_shows_every_frame_whole() {
        for n in [1, 2] {
            present_said(n, 41, 29, 300);
        }
    }

    #[test]
    fn a_surface_given_only_the_rows_that_changed_shows_every_frame_whole() {
        for n in [1, 2] {
            present_through(n, 37, 23, 400);
        }
    }

    #[test]
    fn a_new_size_or_an_unknown_buffer_is_given_the_whole_frame() {
        let mut noise = Noise(7);
        let mut shown = Shown::default();
        let a = noise.frame(8, 4);
        let mut buffer = vec![0u32; 8 * 4];
        let bands = shown.copy(&whole(&a, 8, 4), &mut buffer, 8, 0);
        let all = |h| {
            vec![Area {
                x: 0,
                y: 0,
                w: 8,
                h,
            }]
        };
        assert_eq!(bands, all(4), "the first frame is damaged whole");
        // The same frame again into a buffer of unknown age: written whole,
        // damaged nowhere, since the window already shows it.
        let mut stale = vec![0u32; 8 * 4];
        let bands = shown.copy(&whole(&a, 8, 4), &mut stale, 8, 0);
        assert!(stale == bytewise(&a, 8, 4) && bands.is_empty());
        // A new size: whole.
        let b = noise.frame(6, 4);
        let mut buffer = vec![0u32; 6 * 4];
        let bands = shown.copy(&whole(&b, 6, 4), &mut buffer, 6, 1);
        assert!(buffer == bytewise(&b, 6, 4));
        assert_eq!(
            bands,
            vec![Area {
                x: 0,
                y: 0,
                w: 6,
                h: 4
            }]
        );
    }
}
