//! Reads the QR codes in a shell's camera frames, on a worker thread.
//!
//! The core is `no_std` and has one thread by design: it owns the
//! screen, the routing and the framebuffer, and it does them one event
//! at a time. A decode costs more than the gap between camera frames on
//! a small board — 235 ms of a Pi 3B+'s pass, 445 at worst
//! (`docs/PLANNING.md` §16.36) — so a core that decoded answered taps
//! and painted at the decode rate, and the scan screen felt frozen
//! while codes still read.
//!
//! So the decode moved out here, where a shell that has threads can
//! afford it. A shell hands every frame to both: to the core as
//! `Event::CameraFrame`, which is the preview, and to [`Scanner::offer`],
//! which is the reading. Each pass it calls [`Scanner::poll`] and sends
//! whatever came back as `Event::Scanned`, which the core routes as it
//! routes any other code.
//!
//! Nothing here is queued. [`Scanner::offer`] replaces the frame the
//! worker has not taken yet, so a board that decodes slower than it
//! captures simply decodes fewer frames and always the newest one; a
//! frame behind the newest pictures where the camera was, not where it
//! is.
//!
//! A frame may picture a SeedQR, so every frame, every reduced copy and
//! every payload here is wiped when this crate lets go of it.
//!
//! ```no_run
//! # use opensigner_scanner::Scanner;
//! let scanner = Scanner::new();
//! // Every frame the shell takes, after it has gone to the core:
//! scanner.offer(640, 480, vec![0u8; 640 * 480]);
//! // Every pass:
//! while let Some(bytes) = scanner.poll() {
//!     // osk_shell_api::Event::Scanned { bytes }
//!     let _ = bytes;
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use opensigner_core::scan;
use zeroize::{Zeroize, Zeroizing};

/// How many frames the reduced copy may find nothing in before the full
/// frame is read as well.
///
/// The reduced copy is a quarter of the pixels and finds every code a
/// person holds up to the viewfinder; the full frame is for a code too
/// small or too dense to survive the reduction, and reading it on every
/// miss is what a scene with no code in it used to cost. Reading it on
/// one miss in four costs about a quarter of that and still resolves a
/// dense code within a second of the camera settling on it.
const FULL_FRAME_EVERY: u32 = 4;

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
    /// Set by the worker when it has ended on its own.
    ended: Arc<AtomicBool>,
    found: Receiver<Vec<u8>>,
    thread: Option<JoinHandle<()>>,
}

impl Scanner {
    /// Starts the worker. It waits until a frame is offered and costs
    /// nothing until one is.
    #[must_use]
    pub fn new() -> Scanner {
        let pending = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let ended = Arc::new(AtomicBool::new(false));
        let (tx, found) = channel();
        let thread = {
            let pending = Arc::clone(&pending);
            let ended = Arc::clone(&ended);
            std::thread::spawn(move || {
                work(&pending, &tx);
                ended.store(true, Ordering::Relaxed);
            })
        };
        Scanner {
            pending,
            ended,
            found,
            thread: Some(thread),
        }
    }

    /// Hands the worker the newest frame, replacing the one it has not
    /// taken yet. Never blocks and never queues: the frame it replaces
    /// is wiped, not read.
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

    /// The payload of a code the worker read, or `None` when it has
    /// found nothing since the last call. At most one code per decode,
    /// so a shell calls this until it answers `None`.
    #[must_use]
    pub fn poll(&self) -> Option<Vec<u8>> {
        self.found.try_recv().ok()
    }

    /// Ends the worker, waits for it, and wipes the frame and the
    /// payloads it still held. Harmless when it has already stopped.
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
        while let Ok(mut bytes) = self.found.try_recv() {
            bytes.zeroize();
        }
    }

    /// Whether the worker is still running. False once [`Scanner::stop`]
    /// has been called.
    #[must_use]
    pub fn running(&self) -> bool {
        self.thread.is_some() && !self.ended.load(Ordering::Relaxed)
    }
}

impl Default for Scanner {
    fn default() -> Scanner {
        Scanner::new()
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The worker: take the newest frame, read it, send what it found.
fn work(pending: &(Mutex<Pending>, Condvar), tx: &Sender<Vec<u8>>) {
    let (lock, wake) = pending;
    let mut misses = 0u32;
    loop {
        let job = {
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
        match read(&job, &mut misses) {
            Some(bytes) => {
                if tx.send(bytes).is_err() {
                    return;
                }
            }
            None => continue,
        }
    }
}

/// The policy: the reduced copy every frame, and the full frame on
/// every [`FULL_FRAME_EVERY`]th frame the reduced copy found nothing in.
fn read(job: &Job, misses: &mut u32) -> Option<Vec<u8>> {
    let (w, h) = (job.width, job.height);
    let (rw, rh, reduced) = scan::reduce(w, h, &job.luma);
    let reduced = Zeroizing::new(reduced);
    if let Some(bytes) = scan::decode_one(usize::from(rw), usize::from(rh), &reduced) {
        *misses = 0;
        return Some(bytes);
    }
    if scan::reduce_factor(w, h) <= 1 {
        return None;
    }
    *misses += 1;
    if !(*misses).is_multiple_of(FULL_FRAME_EVERY) {
        return None;
    }
    let found = scan::decode_one(w, h, &job.luma);
    if found.is_some() {
        *misses = 0;
    }
    found
}

/// The first code in one frame, read the same way but here and now: the
/// reduced copy, then the full frame. For a shell with no thread to
/// spare and for the scripted one, which has a frame and needs its
/// answer before the next line of the script.
#[must_use]
pub fn decode_frame(width: u16, height: u16, luma: &[u8]) -> Option<Vec<u8>> {
    scan::decode_frame(usize::from(width), usize::from(height), luma)
}
