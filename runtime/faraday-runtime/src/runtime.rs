//! The top states and `Runtime::new` (05 §1).

use std::sync::Arc;

use faraday_atoms::MotionStore;
use faraday_crypto::work::{Compute, Hooks, Work};
use faraday_kernel::{Clock, Feature, Memo, Mode, Running, Started, shared};
use faraday_port as port;

use crate::dispatch::Replies;
use crate::facts::Kept;
use crate::frame::{Notes, Painted};
use crate::lock::Scope;
use crate::log::Log;
use crate::pool::Pool;
use crate::queue::{Part, Queue};
use crate::root::{OwnerState, Root};

pub struct Runtime<R: Root> {
    pub(crate) root: R,
    pub(crate) waker: Arc<dyn port::Waker>,
    pub(crate) phase: Phase,
    /// The builds of the lock scope in this process, from 1; 0 is the part outside it.
    pub(crate) generation: u32,
    pub(crate) tops: Built<R>,
    pub(crate) queue: Queue<R>,
    pub(crate) replies: Replies<R>,
    pub(crate) pool: Pool,
    pub(crate) facts: Kept,
    pub(crate) frame: Painted<R>,
    pub(crate) motion: MotionStore,
    pub(crate) memo: Memo,
    pub(crate) scope: Scope,
    pub(crate) log: Log,
    pub(crate) running: Vec<Running>,
    /// The root thread's token, for every `Ctx` of event handling (R10).
    pub(crate) compute: Compute,
    /// The orders and asks the event being handled has made, in the order made (P18).
    pub(crate) out: Vec<port::Effect>,
    /// What a part is lent where it has no session or vaults to read (A19).
    pub(crate) readers: shared::Empty,
    /// The adapter's time, as the last `Event::Time` gave it.
    pub(crate) clock: Clock,
    /// The mode of the build the scope is in; `None` until the first build.
    pub(crate) mode: Option<Mode>,
    /// How the scope's build began (§7.5).
    pub(crate) started: Started,
    /// The notes waiting and the one on show (04 §10).
    pub(crate) notes: Notes,
    /// What the runtime dropped or refused, for diagnostics; the most recent 64.
    pub(crate) faults: Vec<&'static str>,
}

/// The top states as built. The settings owner and `Outside` stand outside the lock scope in the
/// Starter (§7.9); the rest exist while the scope is built.
pub(crate) struct Built<R: Root> {
    pub(crate) settings: Option<OwnerState<R::Settings>>,
    pub(crate) session: Option<OwnerState<R::Session>>,
    pub(crate) vaults: Option<OwnerState<R::Vaults>>,
    pub(crate) app: Option<<R::App as Feature>::State>,
    pub(crate) outside: Option<<R::Outside as Feature>::State>,
}

impl<R: Root> Built<R> {
    pub(crate) fn none() -> Self {
        Built {
            settings: None,
            session: None,
            vaults: None,
            app: None,
            outside: None,
        }
    }

    /// Whether a top state exists now; a state not built gets no notice (§2.3).
    pub(crate) fn built(&self, part: Part) -> bool {
        match part {
            Part::Settings => self.settings.is_some(),
            Part::Session => self.session.is_some(),
            Part::Vaults => self.vaults.is_some(),
            Part::App => self.app.is_some(),
            Part::Outside => self.outside.is_some(),
        }
    }
}

impl<R: Root> Runtime<R> {
    pub fn new(root: R, waker: Arc<dyn port::Waker>, workers: Workers) -> Self {
        #[expect(
            clippy::disallowed_methods,
            reason = "R10, the first of the two mint sites: the root thread's token"
        )]
        let compute = Compute::new(Hooks {
            count: counted,
            stop: never_stops,
            progress: no_progress,
        });
        let pool = Pool::start(workers, Arc::clone(&waker));
        Runtime {
            root,
            waker,
            phase: Phase::Starting,
            generation: 0,
            tops: Built::none(),
            queue: Queue::new(),
            replies: Replies::new(),
            pool,
            facts: Kept::new(),
            frame: Painted::new(),
            motion: MotionStore::new(),
            memo: Memo::new(),
            scope: Scope::new(),
            log: Log::new(),
            running: Vec::new(),
            compute,
            out: Vec::new(),
            readers: shared::Empty::new(),
            clock: Clock {
                now_ms: 0,
                unix_secs: 0,
                offset_secs: 0,
                set: false,
                last_input_ms: 0,
            },
            mode: None,
            started: Started::Power,
            notes: Notes::new(),
            faults: Vec::new(),
        }
    }

    /// One port event and everything it causes (§2). Returns the effects for
    /// the root to hand to its adapters, in the order they were made (P18).
    pub fn event(&mut self, event: port::Event) -> Vec<port::Effect> {
        if self.phase == Phase::Ended {
            return Vec::new();
        }
        self.settle(event)
    }

    /// The waker woke the loop: worker answers and progress (§5).
    pub fn woken(&mut self) -> Vec<port::Effect> {
        self.running = self.pool.running();
        let mut effects = Vec::new();
        for answer in self.pool.answers() {
            effects.extend(self.settle_with(|runtime| runtime.job_answered(answer)));
        }
        effects
    }

    /// The frame to paint, when it changed since the last call (§2.5).
    pub fn frame(&mut self) -> Option<port::Frame<'_>> {
        self.frame.handed()
    }

    /// The lock scope's phase (§7).
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Notes something the runtime dropped or refused, for diagnostics (05 §2.1, R8, §1.1).
    pub(crate) fn fault(&mut self, what: &'static str) {
        if self.faults.len() == FAULTS {
            self.faults.remove(0);
        }
        self.faults.push(what);
    }

    /// Turn the log on or off (§8). The dev card's verbose run, the tour root and tests.
    pub fn log(&mut self, _on: bool) {
        todo!()
    }

    #[expect(
        unreachable_code,
        unused_variables,
        reason = "`todo!()` has no iterator type to return"
    )]
    pub fn log_lines(&self) -> impl Iterator<Item = &crate::log::LogLine> + '_ {
        let line: Option<&crate::log::LogLine> = todo!();
        line.into_iter()
    }
}

/// How many faults the runtime keeps.
const FAULTS: usize = 64;

/// The root thread counts nothing it reports; the hooks are the token's own (§5.2).
fn counted(_work: Work) {}

fn never_stops() -> bool {
    false
}

fn no_progress(_done: u32, _of: u32) {}

/// Closed: the Starter's scope is not built. Ended: the process is over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Starting,
    Running,
    Locking,
    Ending,
    Closed,
    Ended,
}

/// `Inline`: the tour root and runtime tests (§5.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Workers {
    Threads,
    Inline,
}
