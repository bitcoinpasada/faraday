//! The two lanes (05 §5).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use faraday_crypto::work::{Compute, Hooks, Work};
use faraday_kernel::{JobKind, Running};
use faraday_port as port;

use crate::queue::{Item, Part};
use crate::root::{Addr, Root};
use crate::runtime::{Runtime, Workers};

/// Heavy: the jobs that ask Argon2id for memory. Light: every other job.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Lane {
    Heavy,
    Light,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct JobId {
    pub(crate) generation: u32,
    pub(crate) n: u32,
}

/// A job's stop flag and two progress counters, atomics shared with the runtime (§5.2).
pub(crate) struct Signal {
    pub(crate) stop: AtomicBool,
    pub(crate) done: AtomicU32,
    pub(crate) of: AtomicU32,
}

/// A job as the pool holds it: the `JobParts` of `Job::into_parts`, its work's answer erased to
/// the top state that started it.
pub(crate) struct Queued {
    pub(crate) id: JobId,
    pub(crate) part: Part,
    pub(crate) kind: JobKind,
    pub(crate) quiet: bool,
    pub(crate) seal: bool,
    pub(crate) run: Box<dyn FnOnce(&Compute) -> Answered + Send>,
}

/// What a lane sends back: the job's message, for the top state that started it.
pub(crate) struct Answered {
    pub(crate) id: JobId,
    pub(crate) part: Part,
    /// Whether the job was a seal, which a lock delivers (R16).
    pub(crate) seal: bool,
    pub(crate) message: Box<dyn core::any::Any + Send>,
}

/// A job a lane is running.
struct Busy {
    id: JobId,
    part: Part,
    kind: JobKind,
    seal: bool,
    began_ms: u64,
    signal: Arc<Signal>,
}

struct LaneState {
    waiting: VecDeque<Queued>,
    busy: Option<Busy>,
    thread: Option<JoinHandle<()>>,
    hand: Option<Sender<(Queued, Arc<Signal>)>>,
}

impl LaneState {
    fn idle() -> Self {
        LaneState {
            waiting: VecDeque::new(),
            busy: None,
            thread: None,
            hand: None,
        }
    }

    /// A lane thread, with the channel it takes its jobs on.
    fn spawn(which: Lane, answers: Sender<Answered>, waker: Arc<dyn port::Waker>) -> Self {
        let (hand, jobs) = std::sync::mpsc::channel();
        let name = match which {
            Lane::Heavy => "faraday-heavy",
            Lane::Light => "faraday-light",
        };
        let thread = std::thread::Builder::new()
            .name(name.to_string())
            .spawn(move || lane(jobs, answers, waker))
            .ok();
        LaneState {
            waiting: VecDeque::new(),
            busy: None,
            hand: thread.as_ref().map(|_| hand),
            thread,
        }
    }

    /// Hands the lane its next job when it is free.
    fn feed(&mut self, now_ms: u64) {
        if self.busy.is_some() {
            return;
        }
        let Some(hand) = &self.hand else {
            return;
        };
        let Some(job) = self.waiting.pop_front() else {
            return;
        };
        let signal = Arc::new(Signal {
            stop: AtomicBool::new(false),
            done: AtomicU32::new(0),
            of: AtomicU32::new(0),
        });
        let busy = Busy {
            id: job.id,
            part: job.part,
            kind: job.kind,
            seal: job.seal,
            began_ms: now_ms,
            signal: Arc::clone(&signal),
        };
        if hand.send((job, signal)).is_ok() {
            self.busy = Some(busy);
        }
    }

    /// Drops the queued jobs `drop_it` names, and asks the running one to stop if `stop_it` does.
    fn stop_where(&mut self, drop_it: impl Fn(&Queued) -> bool, stop_it: impl Fn(&Busy) -> bool) {
        self.waiting.retain(|job| !drop_it(job));
        if let Some(busy) = &self.busy
            && stop_it(busy)
        {
            busy.signal.stop.store(true, Ordering::Release);
        }
    }
}

pub(crate) struct Pool {
    workers: Workers,
    heavy: LaneState,
    light: LaneState,
    /// `Workers::Inline`'s jobs, in the order given: run on the root's thread between events.
    inline: VecDeque<Queued>,
    answers: Receiver<Answered>,
    next: u32,
    now_ms: u64,
}

impl Pool {
    /// Start the lanes with the runtime (§5.1): two threads for `Workers::Threads`, none for
    /// `Workers::Inline`.
    pub(crate) fn start(workers: Workers, waker: Arc<dyn port::Waker>) -> Self {
        let (sender, answers) = std::sync::mpsc::channel();
        let (heavy, light) = match workers {
            Workers::Inline => (LaneState::idle(), LaneState::idle()),
            Workers::Threads => (
                LaneState::spawn(Lane::Heavy, sender.clone(), Arc::clone(&waker)),
                LaneState::spawn(Lane::Light, sender, waker),
            ),
        };
        Pool {
            workers,
            heavy,
            light,
            inline: VecDeque::new(),
            answers,
            next: 0,
            now_ms: 0,
        }
    }

    fn lane_of(&mut self, lane: Lane) -> &mut LaneState {
        match lane {
            Lane::Heavy => &mut self.heavy,
            Lane::Light => &mut self.light,
        }
    }

    /// The clock the jobs started now are stamped with, for `View::running`.
    pub(crate) fn clock(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    pub(crate) fn submit(&mut self, lane: Lane, job: Queued) {
        self.next += 1;
        if self.workers == Workers::Inline {
            self.inline.push_back(job);
            return;
        }
        let now_ms = self.now_ms;
        let state = self.lane_of(lane);
        state.waiting.push_back(job);
        state.feed(now_ms);
    }

    /// `Workers::Inline`: the next job to run on the root's thread, in the order given.
    pub(crate) fn take_inline(&mut self) -> Option<Queued> {
        self.inline.pop_front()
    }

    /// `Fx::Stop`: a queued job of that kind of that part is dropped, a running one asked to stop.
    pub(crate) fn stop(&mut self, part: Part, kind: JobKind) {
        self.inline
            .retain(|job| !(job.part == part && job.kind == kind));
        for state in [&mut self.heavy, &mut self.light] {
            state.stop_where(
                |job| job.part == part && job.kind == kind,
                |busy| busy.part == part && busy.kind == kind,
            );
        }
    }

    /// R16: every queued job of the scope dropped (a seal kept at a lock), every running one asked
    /// to stop. A job of generation 0 belongs to the part outside the scope and is left alone
    /// unless `whole`, the end of the process.
    pub(crate) fn stop_all(&mut self, keep_seals: bool, whole: bool) {
        self.inline
            .retain(|job| !(whole || job.id.generation != 0) || (keep_seals && job.seal));
        for state in [&mut self.heavy, &mut self.light] {
            state.stop_where(
                |job| (whole || job.id.generation != 0) && !(keep_seals && job.seal),
                |busy| (whole || busy.id.generation != 0) && !(keep_seals && busy.seal),
            );
        }
    }

    /// Whether a job of the scope is still running, a seal included; generation 0's are not the
    /// scope's unless `whole`.
    pub(crate) fn scope_running(&self, whole: bool) -> bool {
        [&self.heavy, &self.light]
            .into_iter()
            .filter_map(|state| state.busy.as_ref())
            .any(|busy| whole || busy.id.generation != 0)
    }

    /// Whether a job of the pool is still running, a seal included.
    pub(crate) fn any_running(&self) -> bool {
        self.heavy.busy.is_some() || self.light.busy.is_some()
    }

    /// The jobs running now, with their progress, for `View::running`.
    pub(crate) fn running(&self) -> Vec<Running> {
        [&self.heavy, &self.light]
            .into_iter()
            .filter_map(|state| state.busy.as_ref())
            .map(|busy| Running {
                kind: busy.kind,
                began_ms: busy.began_ms,
                done: busy.signal.done.load(Ordering::Acquire),
                of: busy.signal.of.load(Ordering::Acquire),
            })
            .collect()
    }

    /// Every answer that came, in the order the jobs ended (§5.3). A lane that returned is given
    /// its next job.
    pub(crate) fn answers(&mut self) -> Vec<Answered> {
        let mut came = Vec::new();
        while let Ok(answer) = self.answers.try_recv() {
            for state in [&mut self.heavy, &mut self.light] {
                if state.busy.as_ref().is_some_and(|busy| busy.id == answer.id) {
                    state.busy = None;
                }
            }
            came.push(answer);
        }
        let now_ms = self.now_ms;
        self.heavy.feed(now_ms);
        self.light.feed(now_ms);
        came
    }
}

impl Drop for Pool {
    /// The lanes end with the runtime: no more jobs, the running ones asked to stop.
    fn drop(&mut self) {
        for state in [&mut self.heavy, &mut self.light] {
            state.waiting.clear();
            if let Some(busy) = &state.busy {
                busy.signal.stop.store(true, Ordering::Release);
            }
            state.hand = None;
            if let Some(thread) = state.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

/// What a lane thread reads and writes for the job it is running (§5.2).
struct Current {
    signal: Arc<Signal>,
    waker: Arc<dyn port::Waker>,
    last: Cell<(u32, u32)>,
    woke: Cell<Option<Instant>>,
}

thread_local! {
    static CURRENT: RefCell<Option<Current>> = const { RefCell::new(None) };
}

/// The root is woken at most ten times a second for progress (§5.3).
const WAKE_EVERY: Duration = Duration::from_millis(100);

/// A lane thread's entry: it mints its own `Compute` (R10) and runs its jobs one at a time.
fn lane(
    jobs: Receiver<(Queued, Arc<Signal>)>,
    answers: Sender<Answered>,
    waker: Arc<dyn port::Waker>,
) {
    #[expect(
        clippy::disallowed_methods,
        reason = "R10, the second mint site: one token for each lane thread"
    )]
    let compute = Compute::new(Hooks {
        count: counted,
        stop: stop_asked,
        progress: progressed,
    });
    while let Ok((job, signal)) = jobs.recv() {
        CURRENT.with(|current| {
            *current.borrow_mut() = Some(Current {
                signal,
                waker: Arc::clone(&waker),
                last: Cell::new((0, 0)),
                woke: Cell::new(None),
            });
        });
        // §5.1: a job that panics ends the process in every build, as release's `panic = "abort"`
        // does; a lane that only unwound would leave a lock waiting for its answer for ever.
        let Ok(answer) =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (job.run)(&compute)))
        else {
            #[expect(
                clippy::disallowed_methods,
                reason = "§5.1: a panicked job ends the process in a debug build as release's abort does"
            )]
            std::process::abort();
        };
        CURRENT.with(|current| *current.borrow_mut() = None);
        if answers.send(answer).is_err() {
            return;
        }
        waker.wake();
    }
}

/// A lane counts nothing it reports; the hooks are the token's own (§5.2).
fn counted(_work: Work) {}

/// Reads the current job's stop flag from the thread-local signal (§5.2).
fn stop_asked() -> bool {
    CURRENT.with(|current| {
        current
            .borrow()
            .as_ref()
            .is_some_and(|c| c.signal.stop.load(Ordering::Acquire))
    })
}

/// Writes the current job's progress counters (§5.2).
fn progressed(done: u32, of: u32) {
    CURRENT.with(|current| {
        let current = current.borrow();
        let Some(c) = current.as_ref() else {
            return;
        };
        c.signal.done.store(done, Ordering::Release);
        c.signal.of.store(of, Ordering::Release);
        let due = c.woke.get().is_none_or(|at| at.elapsed() >= WAKE_EVERY);
        if c.last.get() != (done, of) && due {
            c.last.set((done, of));
            c.woke.set(Some(Instant::now()));
            c.waker.wake();
        }
    });
}

impl<R: Root> Runtime<R> {
    /// A job's answer handled as one event; one of another generation is dropped and wiped (R15).
    /// While the scope winds down or is closed, only a seal's answer and generation 0's are
    /// delivered (R16).
    pub(crate) fn job_answered(&mut self, answer: Answered) {
        let generation = answer.id.generation;
        let current = generation == 0
            || (generation == self.generation && self.phase != crate::runtime::Phase::Closed);
        let served = generation == 0 || !self.winding_down() || answer.seal;
        if self.phase == crate::runtime::Phase::Ended || !current || !served {
            return;
        }
        match answer.message.downcast::<Addr<R>>() {
            Ok(addr) => self.queue.push(Item::Owed(*addr)),
            Err(_) => self.fault("a job's answer of another root"),
        }
    }

    /// `Workers::Inline`: each job the event started, run on the root's thread with its token
    /// after the cascade, its answer the next event (§5.1).
    pub(crate) fn run_inline(&mut self) -> Vec<port::Effect> {
        let mut effects = Vec::new();
        while let Some(job) = self.pool.take_inline() {
            let answer = (job.run)(&self.compute);
            effects.extend(self.settle_with(|runtime| runtime.job_answered(answer)));
        }
        effects
    }

    /// R17: the running camera stream, ended at `CameraOff`, `Ended`, a lock or a dropped scope.
    /// Whether the fact goes on to the root: one of another stream is dropped and wipes.
    pub(crate) fn stream_check(&mut self, fact: &port::event::CameraFact) -> bool {
        use port::event::CameraFact;
        let stream = match fact {
            CameraFact::Listed(_) => return true,
            CameraFact::Frame { stream, .. }
            | CameraFact::Code { stream, .. }
            | CameraFact::Seen { stream, .. }
            | CameraFact::Ended { stream, .. } => *stream,
        };
        if self.scope.stream != Some(stream) {
            return false;
        }
        if matches!(fact, CameraFact::Ended { .. }) {
            self.scope.stream = None;
        }
        true
    }
}
