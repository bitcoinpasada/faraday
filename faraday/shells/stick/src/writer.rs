//! The framebuffer write on a thread of its own (`docs/FRAMES.md` §6).
//!
//! On a panel that takes the core's frame as it is, a pass is a draw and
//! then a write: the kernel copies the rows into memory the bus writes
//! slowly, and the main loop has nothing else to do with that time. The
//! main thread compares the frame with the panel's kept copy and copies
//! the rows that differ into it; this thread writes those rows from the
//! kept copy, band by band under the panel's lock, while the loop draws
//! the next frame.
//!
//! At most one write is out. The next frame waits for it before it
//! touches the kept copy, so nothing queues, the loop pays what the write
//! costs beyond the draw, and the pacer (`pace.rs`) sees it in the pass.
//! The thread holds the panel and nothing of the app. A thread that ends
//! without answering ends the loop the way a panic on the main thread
//! does.

use std::io;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::panel::{Panel, Sink, write_staged};

/// Rows of the kept copy to write, and whether that makes the panel whole.
struct Job {
    bands: Vec<(usize, usize)>,
    complete: bool,
}

/// What a write came to.
pub struct Done {
    /// The write's own time, the `pwrite`s and the lock waits between
    /// them.
    pub took: Duration,
    /// How it ended; a failure has marked the panel stale.
    pub result: io::Result<()>,
}

/// The writer thread of one panel.
pub struct Writer {
    /// The panel it writes, as an address, to tell a call for another.
    panel: usize,
    jobs: Option<Sender<Job>>,
    done: Receiver<Done>,
    thread: Option<JoinHandle<()>>,
    out: bool,
}

impl Writer {
    /// Starts the thread, on a machine with more than one core. `None`
    /// when it has one, or the thread did not start: the caller writes
    /// on its own thread as before.
    pub fn start<S: Sink + Send + 'static>(panel: &Arc<Mutex<Panel<S>>>) -> Option<Writer> {
        if std::thread::available_parallelism().map_or(1, |n| n.get()) < 2 {
            return None;
        }
        let (jobs, rx) = channel::<Job>();
        let (tx, done) = channel::<Done>();
        let id = Arc::as_ptr(panel) as usize;
        let own = panel.clone();
        let thread = std::thread::Builder::new()
            .name("fb-write".into())
            .spawn(move || {
                for job in rx {
                    let at = Instant::now();
                    let result = write_staged(&own, &job.bands, job.complete);
                    let took = at.elapsed();
                    if tx.send(Done { took, result }).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Writer {
            panel: id,
            jobs: Some(jobs),
            done,
            thread: Some(thread),
            out: false,
        })
    }

    /// Whether this writes `panel`.
    pub fn is_for<S: Sink>(&self, panel: &Mutex<Panel<S>>) -> bool {
        std::ptr::eq(panel, self.panel as *const Mutex<Panel<S>>)
    }

    /// Whether a write is out.
    pub fn busy(&self) -> bool {
        self.out
    }

    /// Costs the wait for the write that is out, if one is: how it came
    /// to. A writer that died panics here, as the main thread would have.
    pub fn wait(&mut self) -> Option<Done> {
        if !self.out {
            return None;
        }
        self.out = false;
        match self.done.recv() {
            Ok(done) => Some(done),
            Err(_) => self.died(),
        }
    }

    /// Hands `bands` of the kept copy to the thread. The caller has
    /// waited for the write before.
    pub fn send(&mut self, bands: Vec<(usize, usize)>, complete: bool) {
        debug_assert!(!self.out, "one write at a time");
        let sent = self
            .jobs
            .as_ref()
            .is_some_and(|j| j.send(Job { bands, complete }).is_ok());
        if !sent {
            self.died();
        }
        self.out = true;
    }

    /// The thread is gone: raise what it died of.
    fn died(&mut self) -> ! {
        self.jobs = None;
        match self.thread.take().map(JoinHandle::join) {
            Some(Err(payload)) => std::panic::resume_unwind(payload),
            _ => panic!("the framebuffer writer ended"),
        }
    }
}

impl Drop for Writer {
    /// The write that is out finishes before the panel is let go.
    fn drop(&mut self) {
        self.jobs = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fb::{Depth, Geometry};
    use crate::orient::Turn;
    use crate::panel::lock;
    use crate::present::Presenter;

    const G: Geometry = Geometry {
        width: 16,
        height: 200,
        depth: Depth::Rgba8888,
        stride: 64,
    };

    fn frame(fill: u8) -> Vec<u8> {
        vec![fill; 64 * 200]
    }

    /// A panel and a presenter that writes it on the thread, or on the
    /// caller's when the machine has one core.
    fn rig() -> (Arc<Mutex<Panel<Vec<u8>>>>, Presenter) {
        let panel = Arc::new(Mutex::new(Panel::new(Vec::new(), G)));
        let mut p = Presenter::default();
        p.write_on_thread(&panel);
        (panel, p)
    }

    fn put(p: &mut Presenter, panel: &Arc<Mutex<Panel<Vec<u8>>>>, f: &[u8]) {
        p.present(f, G, Turn::R0, panel, None).expect("written");
    }

    #[test]
    fn after_a_run_of_frames_the_panel_holds_the_last() {
        let (panel, mut p) = rig();
        for fill in [1, 2, 3, 4, 5] {
            put(&mut p, &panel, &frame(fill));
        }
        // A frame that changes only some rows.
        let mut last = frame(5);
        last[64 * 90..64 * 95].fill(9);
        put(&mut p, &panel, &last);
        p.settle(None).expect("written");
        assert_eq!(*lock(&panel).sink(), last);
    }

    #[test]
    fn a_blank_straight_after_a_frame_leaves_the_panel_black() {
        let (panel, mut p) = rig();
        put(&mut p, &panel, &frame(7));
        p.settle(None).expect("written");
        lock(&panel).blank();
        assert!(lock(&panel).sink().iter().all(|&b| b == 0));
        // And the next frame is written whole.
        put(&mut p, &panel, &frame(7));
        p.settle(None).expect("written");
        assert_eq!(*lock(&panel).sink(), frame(7));
    }

    #[test]
    fn a_hidden_screen_leaves_none_of_the_secret_on_the_panel() {
        let (panel, mut p) = rig();
        put(&mut p, &panel, &frame(0xAA));
        // The next frame hides it at once, with no settling between.
        put(&mut p, &panel, &frame(0x11));
        p.settle(None).expect("written");
        assert!(lock(&panel).sink().iter().all(|&b| b == 0x11));
    }

    #[test]
    fn the_arrow_stays_on_the_frame_while_it_is_written() {
        let (panel, mut p) = rig();
        put(&mut p, &panel, &frame(0x20));
        lock(&panel).move_arrow(Some((4, 100))).expect("moved");
        put(&mut p, &panel, &frame(0x30));
        p.settle(None).expect("written");
        let mut want = frame(0x30);
        crate::cursor::draw_rows(&mut want, G, Turn::R0, (4, 100), 0);
        assert_eq!(*lock(&panel).sink(), want);
    }
}
