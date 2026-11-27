//! An adapter: `SystemAsk`, `SystemFact`, `settings.txt`, the colour scheme, writing a stick
//! (09 §6; 02 §4.10).

use std::sync::Arc;

use faraday_port::{Adapters, Effect, Event, Waker};

/// The Starter's operating system.
pub struct System {
    _private: (),
}

impl System {
    #[must_use]
    pub fn new(_waker: Arc<dyn Waker>) -> Self {
        todo!()
    }
}

impl Adapters for System {
    fn carry(&mut self, _effect: Effect) {
        todo!()
    }

    fn poll(&mut self) -> Option<Event> {
        todo!()
    }
}
