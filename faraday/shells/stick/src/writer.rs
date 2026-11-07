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
//! Where the machine has a vblank to wait for (`refresh.rs`), the thread
//! waits for one before it writes a frame, on a handle of its own and
//! outside the panel's lock, so the loop keeps drawing and the arrow's
//! thread is never held by the wait. The wait is off for the run on its
//! first error, or when its returns are not on a grid of refreshes. The
//! thread measures the refresh once, with sixteen waits back to back
//! before its first write, while the loop starts.
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
use crate::refresh::{Beat, Vblank};

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
    beat: Option<Arc<Beat>>,
}

impl Writer {
    /// Starts the thread, on a machine with more than one core. `None`
    /// when it has one, or the thread did not start: the caller writes
    /// on its own thread as before.
    pub fn start<S: Sink + Send + 'static>(
        panel: &Arc<Mutex<Panel<S>>>,
        mut vblank: Option<Box<dyn Vblank>>,
    ) -> Option<Writer> {
        if std::thread::available_parallelism().map_or(1, |n| n.get()) < 2 {
            return None;
        }
        let (jobs, rx) = channel::<Job>();
        let (tx, done) = channel::<Done>();
        let id = Arc::as_ptr(panel) as usize;
        let beat = vblank.is_some().then(|| Arc::new(Beat::new()));
        let own_beat = beat.clone();
        let own = panel.clone();
        let thread = std::thread::Builder::new()
            .name("fb-write".into())
            .spawn(move || {
                // The refresh is measured once, before the first write.
                if let (Some(v), Some(b)) = (vblank.as_mut(), own_beat.as_ref()) {
                    crate::refresh::measure(v.as_mut(), b, &mut Instant::now);
                }
                for job in rx {
                    let mut waited = None;
                    if let (Some(v), Some(b)) = (vblank.as_mut(), own_beat.as_ref())
                        && b.on()
                    {
                        match v.wait() {
                            Ok(()) => {
                                let now = Instant::now();
                                b.vblank(now);
                                waited = Some(now);
                            }
                            Err(e) => b.off(&format!("error: {e}")),
                        }
                    }
                    let at = Instant::now();
                    let result = write_staged(&own, &job.bands, job.complete);
                    let took = at.elapsed();
                    if let (Some(b), Some(v)) = (own_beat.as_ref(), waited) {
                        b.written(v);
                    }
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
            beat,
        })
    }

    /// The refresh as the thread measures it, when it waits for one.
    pub fn beat(&self) -> Option<Arc<Beat>> {
        self.beat.clone()
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
        p.write_on_thread(&panel, None);
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

    /// A wait the test lets go of by hand.
    struct Gate(std::sync::mpsc::Receiver<()>);

    impl Vblank for Gate {
        fn wait(&mut self) -> io::Result<()> {
            self.0.recv().map_err(|_| io::Error::other("closed"))
        }
    }

    /// A framebuffer without the call.
    struct Refused;

    impl Vblank for Refused {
        fn wait(&mut self) -> io::Result<()> {
            Err(io::Error::other("not a tty"))
        }
    }

    type Rig = (Arc<Mutex<Panel<Vec<u8>>>>, Presenter);

    fn rig_waiting(v: Box<dyn Vblank>) -> Option<Rig> {
        let panel = Arc::new(Mutex::new(Panel::new(Vec::new(), G)));
        let mut p = Presenter::default();
        p.write_on_thread(&panel, Some(v));
        // One core: the frame is written where it is drawn, with no wait.
        p.beat().map(|_| (panel, p))
    }

    /// Lets the thread measure a steady refresh of 12 ms, as a screen
    /// would, before the frames start.
    fn let_it_measure(release: &std::sync::mpsc::Sender<()>) {
        for _ in 0..=16 {
            release.send(()).expect("released");
            std::thread::sleep(Duration::from_millis(12));
        }
    }

    #[test]
    fn a_frame_is_written_after_the_vblank_and_not_before() {
        let (release, gate) = std::sync::mpsc::channel();
        let Some((panel, mut p)) = rig_waiting(Box::new(Gate(gate))) else {
            return;
        };
        let_it_measure(&release);
        put(&mut p, &panel, &frame(0x42));
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(lock(&panel).rows_written(), 0, "written before the vblank");
        release.send(()).expect("released");
        p.settle(None).expect("written");
        assert_eq!(*lock(&panel).sink(), frame(0x42));
    }

    #[test]
    fn a_wait_that_fails_is_given_up_and_the_frames_still_go_out() {
        let Some((panel, mut p)) = rig_waiting(Box::new(Refused)) else {
            return;
        };
        for fill in [1, 2, 3] {
            put(&mut p, &panel, &frame(fill));
        }
        p.settle(None).expect("written");
        assert_eq!(*lock(&panel).sink(), frame(3));
        assert!(!p.beat().expect("a beat").on());
    }

    #[test]
    fn a_blank_does_not_wait_for_a_vblank_that_has_not_come() {
        let (release, gate) = std::sync::mpsc::channel();
        let Some((panel, mut p)) = rig_waiting(Box::new(Gate(gate))) else {
            return;
        };
        let_it_measure(&release);
        put(&mut p, &panel, &frame(0x77));
        // The writer is waiting; the panel's lock is free.
        lock(&panel).blank();
        assert!(lock(&panel).sink().iter().all(|&b| b == 0));
        release.send(()).expect("released");
        p.settle(None).expect("written");
    }
}
