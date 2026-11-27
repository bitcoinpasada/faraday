//! `Facts` kept from port facts, `View` and `Show` built (05 §4).

use faraday_kernel::{Facts, Mode, NoteUp, Show, Started, View};
use faraday_port as port;

use crate::root::Root;
use crate::runtime::Runtime;

/// What the process keeps across an in-process rebuild (§4.2).
pub(crate) struct Kept {
    pub(crate) facts: Facts,
    /// Every field of the process's `Start` but `carried`, updated by later facts.
    pub(crate) start: Option<StartFacts>,
    /// The clean marker's latch (R14): stays set across a rebuild.
    pub(crate) tainted: bool,
    pub(crate) clean: bool,
}

/// The process's `Start`, less its carry (§4.2).
pub(crate) struct StartFacts {
    pub(crate) display: port::values::Display,
    pub(crate) medium: port::values::Medium,
    pub(crate) abilities: port::event::Abilities,
    pub(crate) machine: port::event::Machine,
}

impl<R: Root> Runtime<R> {
    /// `View` for a dispatch to `for_part`: its own slot holds the empty reader (R13).
    pub(crate) fn view_for(&self, _for_part: crate::queue::Part, _mode: Mode) -> View<'_> {
        todo!()
    }

    /// `Show` for a frame, with the memo and the note up (§4.1).
    pub(crate) fn show(&self, _note: Option<NoteUp<'_>>) -> Show<'_> {
        todo!()
    }

    /// §2.2: the field of `Facts` replaced, then `Notice::Facts` to every top state.
    pub(crate) fn keep_fact(&mut self, _event: port::Event) {
        todo!()
    }

    /// R14: `holds()` of every top state read; `Order::Clean` when the value changed.
    pub(crate) fn clean_marker(&mut self) -> Option<port::Effect> {
        todo!()
    }

    /// The self-test, run once per process on the root's thread (§4.2).
    pub(crate) fn run_selftest(&mut self) {
        todo!()
    }

    /// How the scope's build began, as `View::started` says (§7.5).
    pub(crate) fn started(&self) -> Started {
        todo!()
    }
}
