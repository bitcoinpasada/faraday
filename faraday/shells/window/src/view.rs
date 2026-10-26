//! The window and its drawing surface: an app's frame copied in and
//! presented, and what the screen says about itself.

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use faraday_shell_api::{
    App, BootState, Command, DisplayInfo, Event, Frame, Parts, SecureHardware,
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
    /// here: no event is coming to ask on its behalf.
    pub fn draw<A: App + ?Sized>(&mut self, app: &mut A, pacer: &mut Pacer) {
        let window = &self.window;
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if self.surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = self.surface.buffer_mut() else {
            return;
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
        // Only the rows that changed since the last frame are given to the
        // compositor to take again; a frame that changed nothing still
        // presents a row, so the compositor still says when it is shown.
        let damage: Vec<softbuffer::Rect> = bands
            .iter()
            .filter_map(|&(y0, y1)| {
                Some(softbuffer::Rect {
                    x: 0,
                    y: u32::try_from(y0).ok()?,
                    width: w,
                    height: NonZeroU32::new(u32::try_from(y1 - y0).ok()?)?,
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
            pacer.presented(rendered - start, now - rendered, now);
        }
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

/// What a window's surface was given in its last two frames, newest
/// first: the window's width and a hash of each of its rows. The rows a
/// buffer already holds are not copied again, and only the rows that
/// differ from the frame last presented are damaged.
#[derive(Debug, Default)]
pub struct Shown {
    frames: [Option<(usize, Vec<u64>)>; 2],
}

impl Shown {
    /// Costs a hash of every row of the window and a copy of the rows
    /// that changed. Copies `parts` into `buffer`, a window `sw` pixels
    /// wide that holds what it was given `age` frames ago (softbuffer's
    /// age: 1 the last frame, 2 the one before, 0 not known), as
    /// [`copy`] does, but only where a row differs from what the buffer
    /// holds. Returns the bands of rows, `[from, to)`, that differ from
    /// the frame last presented: the damage to present.
    pub fn copy(
        &mut self,
        parts: &Parts<'_>,
        buffer: &mut [u32],
        sw: usize,
        age: u8,
    ) -> Vec<(usize, usize)> {
        if sw == 0 {
            return Vec::new();
        }
        let rows = buffer.len() / sw;
        fn at_size(f: &Option<(usize, Vec<u64>)>, sw: usize, rows: usize) -> Option<&[u64]> {
            f.as_ref()
                .filter(|(w, h)| *w == sw && h.len() == rows)
                .map(|(_, h)| h.as_slice())
        }
        let held = match age {
            1 => at_size(&self.frames[0], sw, rows),
            2 => at_size(&self.frames[1], sw, rows),
            _ => None,
        };
        let last = at_size(&self.frames[0], sw, rows);
        let mut hashes = Vec::with_capacity(rows);
        let mut bands: Vec<(usize, usize)> = Vec::new();
        for (y, dst) in buffer.chunks_exact_mut(sw).enumerate() {
            let pieces = row(parts, y, sw);
            let h = hash(&pieces);
            if held.is_none_or(|hs| hs[y] != h) {
                put(&pieces, dst);
            }
            if last.is_none_or(|hs| hs[y] != h) {
                match bands.last_mut() {
                    Some(b) if b.1 == y => b.1 = y + 1,
                    _ => bands.push((y, y + 1)),
                }
            }
            hashes.push(h);
        }
        let newest = self.frames[0].take();
        self.frames = [Some((sw, hashes)), newest];
        bands
    }
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
    let mut x = 0;
    for &(src, beyond) in pieces {
        let n = src.len() / 4;
        for (d, px) in dst[x..x + n].iter_mut().zip(src.chunks_exact(4)) {
            *d = u32::from_be_bytes([px[0], px[1], px[2], px[3]]) >> 8;
        }
        x += n;
        dst[x..x + beyond].fill(BEYOND);
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
                    let damaged = bands.iter().any(|&(a, z)| (a..z).contains(&y));
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
        assert_eq!(bands, vec![(0, 4)], "the first frame is damaged whole");
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
        assert_eq!(bands, vec![(0, 4)]);
    }
}
