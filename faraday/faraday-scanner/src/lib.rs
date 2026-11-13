//! Reads the QR codes in a shell's camera frames on a worker thread, and
//! says what each pass saw (`docs/QR.md` §1, "the found code outlined"
//! and "too fine for this camera").
//!
//! A pass that finds a code's finder patterns reports a [`Seen`]: its
//! four corners in the frame, the size of one module in pixels, and
//! whether it read, and whether the pass was over the whole frame. The
//! scan sheet outlines the code with the corners; the core says a code
//! found in whole-frame passes but never read, its modules small, is one
//! this camera cannot resolve, and offers the fix (an animated code, or
//! a file).
//!
//! The newest frame replaces the one not
//! yet taken, and a frame, a reduced copy and a payload are wiped when
//! this crate lets go of them. What the worker reads of a frame is the
//! shell's choice, a [`ReadMode`]: the stick and the Pi, whose
//! processors read a 1920 × 1080 frame only slowly, pass
//! [`ReadMode::Reduced`] (the reduced copy every pass, the full frame
//! on the pass after a code is seen in the copy and not read, and on
//! every [`FULL_FRAME_EVERY`]th miss); the desktop shells, whose
//! cameras give 1280 × 720 for the dense codes that need every pixel,
//! pass [`ReadMode::Full`] (the full frame every pass, paced by how
//! long the pass takes, since the worker takes the newest frame each
//! time it is free). Either way the worker reads every
//! other frame inverted, so a code printed light on dark reads too
//! (`docs/QR.md` §1).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use zeroize::{Zeroize, Zeroizing};

mod scan;

pub use scan::{reduce, reduce_chroma, reduce_factor};

/// How many frames the reduced copy may find nothing in before the full
/// frame is read as well. Odd, so that the full frame is read in both
/// polarities: the worker reads every other frame inverted.
const FULL_FRAME_EVERY: u32 = 3;

/// What the worker reads of each frame it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadMode {
    /// The reduced copy every pass, the full frame after a code seen in
    /// the copy and not read, and on every [`FULL_FRAME_EVERY`]th miss:
    /// the stick and the Pi.
    Reduced,
    /// The full frame every pass: the desktop shells, which ask their
    /// cameras for 1280 × 720.
    Full,
}

/// What one pass saw of a code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seen {
    /// The frame's size.
    pub width: u16,
    /// The frame's height.
    pub height: u16,
    /// The code's corners in the frame: top-left, top-right,
    /// bottom-right, bottom-left.
    pub corners: [(u16, u16); 4],
    /// One module's side, in frame pixels.
    pub module_px: f32,
    /// The pass read it.
    pub read: bool,
    /// The pass looked at the whole frame, not a reduced copy of it:
    /// only then is `module_px` a measure of what the camera resolves.
    pub whole: bool,
}

/// What a pass found: a code's payload, what it saw, and how long it
/// took.
enum Found {
    Code(Vec<u8>),
    Seen(Seen),
    Took(Duration),
}

/// The most pass times kept for a shell that does not take them
/// ([`Scanner::take_pass_times`]).
const PASS_TIMES: usize = 64;

/// One camera frame waiting to be read.
struct Job {
    width: usize,
    height: usize,
    luma: Zeroizing<Vec<u8>>,
}

/// The frame the worker has not taken yet, and whether it should stop.
#[derive(Default)]
struct Pending {
    frame: Option<Job>,
    stop: bool,
}

/// A worker thread reading the codes in the frames it is offered.
pub struct Scanner {
    pending: Arc<(Mutex<Pending>, Condvar)>,
    ended: Arc<AtomicBool>,
    found: Receiver<Found>,
    codes: std::cell::RefCell<Vec<Vec<u8>>>,
    seen: std::cell::Cell<Option<Seen>>,
    times: std::cell::RefCell<Vec<Duration>>,
    thread: Option<JoinHandle<()>>,
}

impl Scanner {
    /// Starts the worker, which reads frames as `mode` says.
    #[must_use]
    pub fn new(mode: ReadMode) -> Scanner {
        let pending = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let ended = Arc::new(AtomicBool::new(false));
        let (tx, found) = channel();
        let thread = {
            let pending = Arc::clone(&pending);
            let ended = Arc::clone(&ended);
            std::thread::spawn(move || {
                work(&pending, &tx, mode);
                ended.store(true, Ordering::Relaxed);
            })
        };
        Scanner {
            pending,
            ended,
            found,
            codes: std::cell::RefCell::new(Vec::new()),
            seen: std::cell::Cell::new(None),
            times: std::cell::RefCell::new(Vec::new()),
            thread: Some(thread),
        }
    }

    /// Hands the worker the newest frame, replacing the one it has not
    /// taken yet.
    pub fn offer(&self, width: u16, height: u16, luma: Vec<u8>) {
        let (lock, wake) = &*self.pending;
        let Ok(mut pending) = lock.lock() else {
            return;
        };
        if pending.stop {
            return;
        }
        pending.frame = Some(Job {
            width: usize::from(width),
            height: usize::from(height),
            luma: Zeroizing::new(luma),
        });
        wake.notify_one();
    }

    fn drain(&self) {
        while let Ok(f) = self.found.try_recv() {
            match f {
                Found::Code(b) => self.codes.borrow_mut().push(b),
                Found::Seen(s) => self.seen.set(Some(s)),
                Found::Took(d) => {
                    let mut times = self.times.borrow_mut();
                    if times.len() < PASS_TIMES {
                        times.push(d);
                    }
                }
            }
        }
    }

    /// The payload of a code the worker read, or `None`.
    #[must_use]
    pub fn poll(&self) -> Option<Vec<u8>> {
        self.drain();
        let mut codes = self.codes.borrow_mut();
        (!codes.is_empty()).then(|| codes.remove(0))
    }

    /// What the latest pass that found a code saw, once.
    #[must_use]
    pub fn poll_seen(&self) -> Option<Seen> {
        self.drain();
        self.seen.take()
    }

    /// How long each of the worker's passes over a frame took since the
    /// last call, for the dev card's timings: the decode, nothing of
    /// what it read.
    #[must_use]
    pub fn take_pass_times(&self) -> Vec<Duration> {
        self.drain();
        std::mem::take(&mut *self.times.borrow_mut())
    }

    /// Ends the worker and wipes what it still held.
    pub fn stop(&mut self) {
        {
            let (lock, wake) = &*self.pending;
            if let Ok(mut pending) = lock.lock() {
                pending.stop = true;
                pending.frame = None;
                wake.notify_all();
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        while let Ok(f) = self.found.try_recv() {
            if let Found::Code(mut b) = f {
                b.zeroize();
            }
        }
        for b in self.codes.borrow_mut().iter_mut() {
            b.zeroize();
        }
        self.codes.borrow_mut().clear();
    }

    /// Whether the worker is still running.
    #[must_use]
    pub fn running(&self) -> bool {
        self.thread.is_some() && !self.ended.load(Ordering::Relaxed)
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        self.stop();
    }
}

fn work(pending: &(Mutex<Pending>, Condvar), tx: &Sender<Found>, mode: ReadMode) {
    let (lock, wake) = pending;
    let mut reading = Reading::default();
    let mut inverted = false;
    let mut last = false;
    loop {
        let mut job = {
            let Ok(mut state) = lock.lock() else {
                return;
            };
            while state.frame.is_none() && !state.stop {
                let Ok(next) = wake.wait(state) else {
                    return;
                };
                state = next;
            }
            if state.stop {
                return;
            }
            match state.frame.take() {
                Some(job) => job,
                None => continue,
            }
        };
        // Every other frame the worker takes is read inverted. It counts
        // the frames it takes, not the ones offered: a pass slower than
        // the camera would otherwise take the same parity every time.
        // The read of the whole frame that follows a code seen but not
        // read keeps that pass's polarity, since it is the same code.
        if reading.whole_next {
            inverted = last;
        }
        if inverted {
            job.luma.iter_mut().for_each(|b| *b = 255 - *b);
        }
        last = inverted;
        inverted = !inverted;
        let began = Instant::now();
        let (code, seen) = read(&job, mode, &mut reading);
        if tx.send(Found::Took(began.elapsed())).is_err() {
            return;
        }
        if let Some(s) = seen
            && tx.send(Found::Seen(s)).is_err()
        {
            return;
        }
        if let Some(bytes) = code
            && tx.send(Found::Code(bytes)).is_err()
        {
            return;
        }
    }
}

/// One frame's first code, and what was seen of it, at `scale` frame
/// pixels a pixel of `luma`. Costs a decode of the frame.
pub fn look(
    width: usize,
    height: usize,
    luma: &[u8],
    scale: usize,
    frame: (u16, u16),
) -> (Option<Vec<u8>>, Option<Seen>) {
    if width == 0 || height == 0 || luma.len() < width * height {
        return (None, None);
    }
    let mut image =
        rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| luma[y * width + x]);
    let mut seen = None;
    for grid in image.detect_grids() {
        let b = grid.bounds;
        let side = |i: usize, j: usize| side_len(b[i], b[j]);
        let edge = (side(0, 1) + side(1, 2) + side(2, 3) + side(3, 0)) / 4.0;
        let modules = rqrr::BitGrid::size(&grid.grid).max(1) as f64;
        let corner = |i: usize| corner_in(b[i], scale, frame);
        let mut bytes = Vec::new();
        let read = grid.decode_to(&mut bytes).is_ok();
        let here = Seen {
            width: frame.0,
            height: frame.1,
            corners: [corner(0), corner(1), corner(2), corner(3)],
            module_px: (edge * scale as f64 / modules) as f32,
            read,
            whole: scale <= 1,
        };
        if read {
            return (Some(bytes), Some(here));
        }
        bytes.zeroize();
        seen.get_or_insert(here);
    }
    (None, seen)
}

/// The distance between two grid points. Points can lie anywhere in `i32`
/// on a crafted frame: the differences are taken in `f64`, where they
/// cannot wrap.
fn side_len(p: rqrr::Point, q: rqrr::Point) -> f64 {
    let (dx, dy) = (
        f64::from(p.x) - f64::from(q.x),
        f64::from(p.y) - f64::from(q.y),
    );
    (dx * dx + dy * dy).sqrt()
}

/// A grid point in the frame, `scale` times its place in the copy, held
/// to the frame. The product is taken in `i64`, where it cannot wrap.
fn corner_in(p: rqrr::Point, scale: usize, frame: (u16, u16)) -> (u16, u16) {
    let s = scale as i64;
    (
        (i64::from(p.x) * s).clamp(0, i64::from(frame.0)) as u16,
        (i64::from(p.y) * s).clamp(0, i64::from(frame.1)) as u16,
    )
}

/// What the reduced mode carries from one pass to the next.
#[derive(Default)]
struct Reading {
    /// Passes in a row that read nothing.
    misses: u32,
    /// The last pass saw a code in the reduced copy and could not read
    /// it: the next reads the whole frame.
    whole_next: bool,
}

/// One frame, read as `mode` says. Costs a decode of the frame: a
/// reduced copy and, now and then, the full frame, or the full frame.
fn read(job: &Job, mode: ReadMode, st: &mut Reading) -> (Option<Vec<u8>>, Option<Seen>) {
    let (w, h) = (job.width, job.height);
    let frame = (w as u16, h as u16);
    if mode == ReadMode::Full {
        return look(w, h, &job.luma, 1, frame);
    }
    if std::mem::take(&mut st.whole_next) {
        let (code, whole) = look(w, h, &job.luma, 1, frame);
        st.misses = if code.is_some() { 0 } else { st.misses + 1 };
        return (code, whole);
    }
    let factor = scan::reduce_factor(w, h);
    let (rw, rh, reduced) = scan::reduce(w, h, &job.luma);
    let reduced = Zeroizing::new(reduced);
    let (code, seen) = look(usize::from(rw), usize::from(rh), &reduced, factor, frame);
    if code.is_some() {
        st.misses = 0;
        return (code, seen);
    }
    if factor <= 1 {
        return (None, seen);
    }
    st.misses += 1;
    if seen.is_some() {
        st.whole_next = true;
        return (None, seen);
    }
    if !st.misses.is_multiple_of(FULL_FRAME_EVERY) {
        return (None, seen);
    }
    let (code, full) = look(w, h, &job.luma, 1, frame);
    if code.is_some() {
        st.misses = 0;
    }
    (code, full.or(seen))
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_codec::qr::{self, Ecc, Payload, QUIET_ZONE};

    #[test]
    fn far_off_grid_points_give_a_length_and_a_clamped_corner_without_a_wrap() {
        let far = rqrr::Point {
            x: i32::MAX,
            y: i32::MIN,
        };
        let near = rqrr::Point { x: 0, y: 0 };
        assert!(
            side_len(
                far,
                rqrr::Point {
                    x: i32::MIN,
                    y: i32::MAX
                }
            ) > 6.0e9
        );
        assert!(side_len(far, near) > 2.0e9);
        assert_eq!(corner_in(far, 3, (640, 480)), (640, 0));
        let low = rqrr::Point {
            x: i32::MIN,
            y: i32::MAX,
        };
        assert_eq!(corner_in(low, 3, (640, 480)), (0, 480));
    }

    /// A token for the work these tests ask, counting nothing. Cheap.
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: faraday-scanner's tests, which cannot reach faraday-core's testkit"
    )]
    fn compute() -> faraday_crypto::work::Compute {
        faraday_crypto::work::Compute::new(|_| {})
    }

    fn frame(scale: usize) -> (usize, usize, Vec<u8>) {
        let c = compute();
        let m = qr::encode(&c, Payload::Bytes(b"faraday scanner"), Ecc::Low).unwrap();
        m.to_luma(scale, QUIET_ZONE)
    }

    /// A frame whose search takes all of its budget (`rqrr`'s `PATCHES.md`):
    /// closing the sheet on it waits for that much and no longer.
    #[test]
    fn a_stop_during_a_crafted_frame_ends_soon() {
        const W: usize = 1920;
        let runs = [1, 1, 3, 1, 1, 1];
        let mut row = Vec::with_capacity(W);
        for (i, run) in runs.iter().cycle().enumerate() {
            if row.len() >= W {
                break;
            }
            row.extend(std::iter::repeat_n(
                if i % 2 == 0 { 25u8 } else { 235 },
                *run,
            ));
        }
        row.truncate(W);
        let frame = row.repeat(1080); // not text: pixel bytes
        for mode in [ReadMode::Full, ReadMode::Reduced] {
            let mut scanner = Scanner::new(mode);
            scanner.offer(W as u16, 1080, frame.clone());
            std::thread::sleep(Duration::from_millis(5));
            let began = Instant::now();
            scanner.stop();
            let took = began.elapsed();
            assert!(took < Duration::from_secs(5), "{mode:?}: {took:?}");
        }
    }

    #[test]
    fn a_read_code_is_outlined_where_it_is() {
        let (w, h, px) = frame(6);
        let (code, seen) = look(w, h, &px, 1, (w as u16, h as u16));
        assert_eq!(code.as_deref(), Some(&b"faraday scanner"[..]));
        let s = seen.unwrap();
        assert!(s.read);
        assert!((s.module_px - 6.0).abs() < 1.0, "{}", s.module_px);
        // The top-left corner is inside the quiet zone's edge.
        assert!(s.corners[0].0 >= (QUIET_ZONE * 6 - 6) as u16);
    }

    /// A code of `bytes` bytes at `scale` pixels a module, in the middle of a
    /// white 1280 × 720 frame: what a phone held up to a laptop's camera
    /// gives for a large PSBT. Its version is returned with it.
    fn dense_frame(bytes: usize, scale: usize) -> (Vec<u8>, Vec<u8>, u8) {
        let c = compute();
        let payload: Vec<u8> = (0..bytes).map(|i| (i * 7 + i / 13) as u8).collect();
        let m = qr::encode(&c, Payload::Bytes(&payload), Ecc::Low).unwrap();
        let (cw, ch, code) = m.to_luma(scale, QUIET_ZONE);
        let (w, h) = (1280, 720);
        let mut px = vec![255u8; w * h];
        let (x0, y0) = ((w - cw) / 2, (h - ch) / 2);
        for y in 0..ch {
            px[(y0 + y) * w + x0..(y0 + y) * w + x0 + cw]
                .copy_from_slice(&code[y * cw..(y + 1) * cw]);
        }
        (payload, px, m.version())
    }

    /// Offers `px` until the worker reads a code, or gives up.
    fn read_by_worker(mode: ReadMode, w: u16, h: u16, px: &[u8]) -> Option<Vec<u8>> {
        let scanner = Scanner::new(mode);
        for _ in 0..2000 {
            scanner.offer(w, h, px.to_vec());
            std::thread::sleep(std::time::Duration::from_millis(5));
            if let Some(c) = scanner.poll() {
                return Some(c);
            }
        }
        None
    }

    #[test]
    fn a_dense_code_reads_at_full_frame() {
        let (payload, px, version) = dense_frame(1500, 3);
        assert!(version >= 25, "version {version}");
        assert_eq!(
            read_by_worker(ReadMode::Full, 1280, 720, &px).as_deref(),
            Some(&payload[..])
        );
    }

    #[test]
    fn a_light_on_dark_code_reads() {
        let (payload, px, _) = dense_frame(300, 6);
        let dark: Vec<u8> = px.iter().map(|b| 255 - b).collect();
        for mode in [ReadMode::Full, ReadMode::Reduced] {
            assert_eq!(
                read_by_worker(mode, 1280, 720, &dark).as_deref(),
                Some(&payload[..]),
                "{mode:?}"
            );
        }
    }

    #[test]
    fn the_worker_reads_and_reports() {
        let (w, h, px) = frame(4);
        let scanner = Scanner::new(ReadMode::Reduced);
        scanner.offer(w as u16, h as u16, px);
        let mut code = None;
        for _ in 0..200 {
            if let Some(c) = scanner.poll() {
                code = Some(c);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(code.as_deref(), Some(&b"faraday scanner"[..]));
        assert!(scanner.poll_seen().is_some_and(|s| s.read));
        // The pass that read it was timed, once.
        assert!(!scanner.take_pass_times().is_empty());
        assert!(scanner.take_pass_times().is_empty());
    }

    /// A version 23 code at error correction H (109 modules a side),
    /// `scale` pixels a module and blurred as a lens blurs it, black on
    /// white or inverted, in the middle of a `w` × `h` frame.
    fn blurred_dense_frame(w: usize, h: usize, scale: usize, inverted: bool) -> (Vec<u8>, Vec<u8>) {
        let c = compute();
        let payload: Vec<u8> = (0..qr::byte_capacity(23, Ecc::High))
            .map(|i| (i * 31 + i / 7 + 5) as u8)
            .collect();
        let m = qr::encode(&c, Payload::Bytes(&payload), Ecc::High).unwrap();
        assert_eq!(m.version(), 23);
        let (cw, ch, code) = m.to_luma(scale, QUIET_ZONE);
        let mut px = vec![235f32; w * h];
        let (x0, y0) = ((w - cw) / 2, (h - ch) / 2);
        for y in 0..ch {
            for x in 0..cw {
                px[(y0 + y) * w + x0 + x] = if code[y * cw + x] < 128 { 25.0 } else { 235.0 };
            }
        }
        // A Gaussian blur of one pixel's sigma, along rows then columns.
        let k: Vec<f32> = (-3i32..=3).map(|i| (-(i * i) as f32 / 2.0).exp()).collect();
        let t: f32 = k.iter().sum();
        let pass = |src: &[f32], horizontal: bool| -> Vec<f32> {
            let mut out = vec![0f32; w * h];
            for y in 0..h {
                for x in 0..w {
                    let mut a = 0.0;
                    for (i, kv) in k.iter().enumerate() {
                        let d = i as isize - 3;
                        let (xx, yy) = if horizontal {
                            ((x as isize + d).clamp(0, w as isize - 1) as usize, y)
                        } else {
                            (x, (y as isize + d).clamp(0, h as isize - 1) as usize)
                        };
                        a += kv * src[yy * w + xx];
                    }
                    out[y * w + x] = a / t;
                }
            }
            out
        };
        let blurred = pass(&pass(&px, true), false);
        let luma = blurred
            .iter()
            .map(|&v| if inverted { 255 - v as u8 } else { v as u8 })
            .collect();
        (payload, luma)
    }

    #[test]
    fn a_dense_code_reads_in_a_blurred_1080p_frame() {
        for inverted in [false, true] {
            let (payload, px) = blurred_dense_frame(1920, 1080, 8, inverted);
            for mode in [ReadMode::Full, ReadMode::Reduced] {
                assert_eq!(
                    read_by_worker(mode, 1920, 1080, &px).as_deref(),
                    Some(&payload[..]),
                    "{mode:?}, inverted {inverted}"
                );
            }
        }
    }

    #[test]
    fn a_code_seen_in_the_copy_and_not_read_is_read_whole_on_the_next_pass() {
        // Four pixels a module in a 1080p frame: the copy, a third of
        // that, sees the code and cannot read it, and the next pass is
        // the whole frame's, whose verdict the sheet shows.
        let (payload, px) = blurred_dense_frame(1920, 1080, 4, false);
        let job = Job {
            width: 1920,
            height: 1080,
            luma: Zeroizing::new(px),
        };
        let mut st = Reading::default();
        let (code, seen) = read(&job, ReadMode::Reduced, &mut st);
        if let (None, Some(seen)) = (code, seen) {
            assert!(!seen.whole, "the copy gives no verdict");
            let (code, seen) = read(&job, ReadMode::Reduced, &mut st);
            assert!(seen.is_none_or(|s| s.whole));
            assert!(code.is_none_or(|c| c == payload));
        }
    }
}
