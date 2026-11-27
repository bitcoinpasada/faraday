//! The two lanes (05 §5).

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use faraday_crypto::work::{Compute, Hooks, Work};
use faraday_kernel::{JobKind, Running};
use faraday_port as port;

use crate::queue::Part;
use crate::root::Root;
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
    pub(crate) message: Box<dyn core::any::Any + Send>,
}

struct LaneState {
    waiting: VecDeque<Queued>,
    busy: Option<(JobId, Arc<Signal>)>,
    thread: Option<JoinHandle<()>>,
    hand: Option<Sender<(Queued, Arc<Signal>)>>,
}

pub(crate) struct Pool {
    workers: Workers,
    heavy: LaneState,
    light: LaneState,
    answers: Receiver<Answered>,
    next: u32,
}

impl Pool {
    /// Start the lanes with the runtime (§5.1).
    pub(crate) fn start(_workers: Workers, _waker: Arc<dyn port::Waker>) -> Self {
        todo!()
    }

    pub(crate) fn submit(&mut self, _lane: Lane, _job: Queued) {
        todo!()
    }

    /// `Fx::Stop`: a queued job of that kind of that part is dropped, a running one asked to stop.
    pub(crate) fn stop(&mut self, _part: Part, _kind: JobKind) {
        todo!()
    }

    /// R16: every queued job dropped (a seal kept at a lock), every running one asked to stop.
    pub(crate) fn stop_all(&mut self, _keep_seals: bool) {
        todo!()
    }

    /// The jobs running now, with their progress, for `View::running`.
    pub(crate) fn running(&self) -> Vec<Running> {
        todo!()
    }

    /// Every answer that came, in the order the jobs ended (§5.3).
    pub(crate) fn answers(&mut self) -> Vec<Answered> {
        todo!()
    }
}

/// A lane thread's entry: it mints its own `Compute` (R10) and runs its jobs one at a time.
fn lane(_lane: Lane, _jobs: Receiver<(Queued, Arc<Signal>)>, _answers: Sender<Answered>) {
    #[expect(
        clippy::disallowed_methods,
        reason = "R10, the second mint site: one token for each lane thread"
    )]
    let _compute = Compute::new(Hooks {
        count: counted,
        stop: stop_asked,
        progress: progressed,
    });
    todo!()
}

fn counted(_work: Work) {
    todo!()
}

/// Reads the current job's stop flag from the thread-local signal (§5.2).
fn stop_asked() -> bool {
    todo!()
}

/// Writes the current job's progress counters (§5.2).
fn progressed(_done: u32, _of: u32) {
    todo!()
}

impl<R: Root> Runtime<R> {
    /// A job's answer handled as one event; one of another generation is dropped and wiped (R15).
    pub(crate) fn job_answered(&mut self, _answer: Answered) {
        todo!()
    }

    /// R17: the running camera stream, ended at `CameraOff`, `Ended`, a lock or a dropped scope.
    pub(crate) fn stream_check(&mut self, _fact: &port::event::CameraFact) -> bool {
        todo!()
    }
}
