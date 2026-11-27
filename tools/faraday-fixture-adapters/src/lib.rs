//! A tool or test-support crate: the tour root's, the journeys' and the leak crate's adapters:
//! fixture sticks, fixed randomness, scripted clock and input, recorded print (09 §8).

use std::path::Path;

use faraday_port::event::Key;
use faraday_port::storage::{Sha256, StickId};
use faraday_port::values::Point;
use faraday_port::{Adapters, Effect, Event};
use faraday_text::Text;

/// Sticks from `out/testkit`, randomness, a clock and an input script.
pub struct Fixtures {
    _private: (),
}

impl Fixtures {
    #[must_use]
    pub fn device(_kit: &Path) -> Self {
        todo!()
    }

    #[must_use]
    pub fn demo(_kit: &Path) -> Self {
        todo!()
    }

    #[must_use]
    pub fn starter(_kit: &Path) -> Self {
        todo!()
    }

    /// Queues a script of input, in order.
    pub fn input(&mut self, _script: &[Scripted]) {
        todo!()
    }

    /// Moves the scripted clock on by `ms`.
    pub fn advance(&mut self, _ms: u64) {
        todo!()
    }

    /// What a journey asserts: every file written, by stick, name and hash.
    #[must_use]
    pub fn written(&self) -> &[(StickId, Text, Sha256)] {
        todo!()
    }
}

impl Adapters for Fixtures {
    /// 02 §10's tour-root column.
    fn carry(&mut self, _effect: Effect) {
        todo!()
    }

    fn poll(&mut self) -> Option<Event> {
        todo!()
    }
}

/// One step of an input script.
pub enum Scripted {
    Touch(Point),
    Key(Key),
    Text(Text),
    Stick(StickId, bool),
    Wait(u64),
}
