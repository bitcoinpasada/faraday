//! The lock order, the carry in hand, the scope's build (05 §7).

use faraday_kernel::{Carried, Line, Lock, Public, Show, Worded, shared};
use faraday_port as port;

use crate::root::Root;
use crate::runtime::Runtime;

/// How far a lock or an end of the scope has got (§7.2, §7.7).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Step {
    Hide,
    Stop,
    Seal,
    Wait,
    Carry,
    Wipe,
    Drop,
    Keep,
    Exit,
}

/// What the runtime holds of the scope across a lock.
pub(crate) struct Scope {
    /// The carry in hand, moved into the next build (§7.6).
    carry: Option<Carried>,
    /// `only_copy`, held aside for the whole Keep (§7.3).
    only_copy: bool,
    /// The lock being run, and the step it is at.
    lock: Option<(Lock, Step)>,
    /// The id of the `Close` change the runtime reads and never routes (§7.2 step 3).
    close: Option<shared::ChangeId>,
    /// The camera stream running, by the id of the ask answered `Started` (§5.5).
    stream: Option<port::AskId>,
}

/// The note a scope rebuilt after a wrong answer to `Keep` puts up: what was carried was lost
/// (§7.6). Nothing is retried.
#[derive(Public, Copy, PartialEq, Eq)]
pub(crate) struct KeepLost;

impl Worded for KeepLost {
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}

impl<R: Root> Runtime<R> {
    /// §7.2: the nine steps, in order, by every way a lock is asked (R19).
    pub(crate) fn begin_lock(&mut self, _lock: Lock) {
        todo!()
    }

    /// Goes on from the step waiting, once what it waited for has come.
    pub(crate) fn lock_next(&mut self) {
        todo!()
    }

    /// §7.7: the order of an end, with no seal.
    pub(crate) fn begin_end(&mut self, _end: faraday_kernel::End) {
        todo!()
    }

    /// §7.9: the Starter's Close over the scope alone.
    pub(crate) fn close_scope(&mut self) {
        todo!()
    }

    /// §7.5: the scope built from the carry, in the fixed order, at the next generation.
    pub(crate) fn build_scope(&mut self, _carried: Option<port::storage::Carry>) {
        todo!()
    }

    /// §7.6: `Answer::Kept`, a failure handing the carry back; a wrong answer is a failed keep
    /// whose carry is lost, and a rebuilt scope then puts up `KeepLost`.
    pub(crate) fn kept(&mut self, _answer: port::Answer) {
        todo!()
    }

    /// `Carry` from `Carried`, and back (§7.3).
    pub(crate) fn carry_out(&mut self, _carried: Carried) -> port::storage::Carry {
        todo!()
    }

    pub(crate) fn carry_in(&mut self, _carry: port::storage::Carry) -> Carried {
        todo!()
    }
}
