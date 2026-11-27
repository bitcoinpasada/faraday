//! The top states and `Runtime::new` (05 §1).

use std::sync::Arc;

use faraday_atoms::MotionStore;
use faraday_crypto::work::{Compute, Hooks, Work};
use faraday_kernel::{Feature, Memo, Running};
use faraday_port as port;

use crate::dispatch::Replies;
use crate::facts::Kept;
use crate::frame::Painted;
use crate::lock::Scope;
use crate::log::Log;
use crate::pool::Pool;
use crate::queue::Queue;
use crate::root::{OwnerState, Root};

pub struct Runtime<R: Root> {
    root: R,
    waker: Arc<dyn port::Waker>,
    phase: Phase,
    /// The builds of the lock scope in this process, from 1; 0 is the part outside it.
    generation: u32,
    tops: Built<R>,
    queue: Queue<R>,
    replies: Replies<R>,
    pool: Pool,
    facts: Kept,
    frame: Painted<R>,
    motion: MotionStore,
    memo: Memo,
    scope: Scope,
    log: Log,
    running: Vec<Running>,
    /// The root thread's token, for every `Ctx` of event handling (R10).
    compute: Compute,
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

impl<R: Root> Runtime<R> {
    pub fn new(root: R, waker: Arc<dyn port::Waker>, workers: Workers) -> Self {
        #[expect(
            clippy::disallowed_methods,
            reason = "R10, the first of the two mint sites: the root thread's token"
        )]
        let _compute = Compute::new(Hooks {
            count: counted,
            stop: never_stops,
            progress: no_progress,
        });
        let _ = (root, waker, workers);
        todo!()
    }

    /// One port event and everything it causes (§2). Returns the effects for
    /// the root to hand to its adapters, in the order they were made (P18).
    pub fn event(&mut self, _event: port::Event) -> Vec<port::Effect> {
        todo!()
    }

    /// The waker woke the loop: worker answers and progress (§5).
    pub fn woken(&mut self) -> Vec<port::Effect> {
        todo!()
    }

    /// The frame to paint, when it changed since the last call (§2.5).
    pub fn frame(&mut self) -> Option<port::Frame<'_>> {
        todo!()
    }

    /// The lock scope's phase (§7).
    pub fn phase(&self) -> Phase {
        todo!()
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

/// The root thread counts nothing it reports; the hooks are the token's own (§5.2).
fn counted(_work: Work) {
    todo!()
}

fn never_stops() -> bool {
    todo!()
}

fn no_progress(_done: u32, _of: u32) {
    todo!()
}

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
