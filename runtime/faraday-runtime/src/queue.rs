//! One event at a time, the cascade (05 §2).

use std::collections::VecDeque;
use std::sync::Arc;

use faraday_kernel::{FactsChanged, shared};
use faraday_port as port;

use crate::root::{Addr, Root, has_outside};
use crate::runtime::Runtime;

/// One event's cascade is bounded (R8).
pub(crate) const CASCADE_BOUND: u32 = 10_000;

/// A top state, in the fixed order of a notice (§2.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Part {
    Settings,
    Session,
    Vaults,
    App,
    Outside,
}

/// The order a notice goes in (§2.3).
pub(crate) const PARTS: [Part; 5] = [
    Part::Settings,
    Part::Session,
    Part::Vaults,
    Part::App,
    Part::Outside,
];

/// What a notice carries until it is dispatched. A shared change goes to every top state, so it
/// is held once.
pub(crate) enum Pending {
    Left,
    Hide,
    Shared(Arc<shared::Changed>),
    Facts(FactsChanged),
}

impl Pending {
    /// One copy for each top state's queue item.
    fn dup(&self) -> Pending {
        match self {
            Pending::Left => Pending::Left,
            Pending::Hide => Pending::Hide,
            Pending::Shared(changed) => Pending::Shared(Arc::clone(changed)),
            Pending::Facts(facts) => Pending::Facts(*facts),
        }
    }
}

/// One item of an event's queue: a message, or a notice for one top state.
pub(crate) enum Item<R: Root> {
    /// A message of a `Then`, a press, a fact or a key.
    Msg(Addr<R>),
    /// A message that answers something: a change's `given`, the asker's message of a `Done` or
    /// of an answer. Past the bound it is carried into the next event (R8).
    Owed(Addr<R>),
    Notice(Part, Pending),
    /// The runtime's own: the next step of a lock or an end (§7.2, §7.7), after the notices queued
    /// before it.
    Wind,
}

pub(crate) struct Queue<R: Root> {
    items: VecDeque<Item<R>>,
    /// What the bound carried over from the event before.
    carried: VecDeque<Item<R>>,
    /// Dispatches of the event being handled.
    pub(crate) dispatched: u32,
}

/// Whether a top state is in the lock scope: the part outside it is not (§7.9).
pub(crate) fn in_scope<R: Root>(part: Part) -> bool {
    match part {
        Part::Outside | Part::Settings => !has_outside::<R>(),
        _ => true,
    }
}

impl<R: Root> Queue<R> {
    pub(crate) fn new() -> Self {
        Queue {
            items: VecDeque::new(),
            carried: VecDeque::new(),
            dispatched: 0,
        }
    }

    pub(crate) fn push(&mut self, item: Item<R>) {
        self.items.push_back(item);
    }

    pub(crate) fn pop(&mut self) -> Option<Item<R>> {
        self.items.pop_front()
    }

    /// A new event: the count starts again, and what answers carried over goes first.
    fn begin(&mut self) {
        self.dispatched = 0;
        while let Some(item) = self.carried.pop_back() {
            self.items.push_front(item);
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// §7.2 step 7: what is queued for the scope's top states is dropped; what is for the part
    /// outside the scope stays.
    pub(crate) fn drop_scope(&mut self) {
        let outside = |item: &Item<R>| match item {
            Item::Msg(addr) | Item::Owed(addr) => !in_scope::<R>(addr.part()),
            Item::Notice(part, _) => !in_scope::<R>(*part),
            Item::Wind => false,
        };
        self.items.retain(outside);
        self.carried.retain(outside);
    }
}

impl<R: Root> Runtime<R> {
    /// §2.1: the event becomes messages, which are dispatched first in first out; the orders
    /// and asks come back in the order made, with `Order::Clean` when the marker changed, and a
    /// frame is drawn after.
    pub(crate) fn settle(&mut self, event: port::Event) -> Vec<port::Effect> {
        self.settle_with(|runtime| runtime.become_messages(event))
    }

    /// One event of the runtime's own (a job's end) handled as `settle` handles a port event.
    pub(crate) fn settle_with(&mut self, begin: impl FnOnce(&mut Self)) -> Vec<port::Effect> {
        self.queue.begin();
        self.marks.clear();
        begin(self);
        self.drain();
        self.draw_after();
        if let Some(clean) = self.clean_marker() {
            self.out.push(clean);
        }
        let out = std::mem::take(&mut self.out);
        if self.phase != crate::runtime::Phase::Ended {
            for effect in &out {
                self.log_port(&effect.kind());
            }
        }
        out
    }

    /// Steps 2 and 3: pop the front item, dispatch it, until the queue is empty or the bound.
    pub(crate) fn drain(&mut self) {
        while !self.queue.is_empty() {
            if self.queue.dispatched >= CASCADE_BOUND {
                self.bound_reached();
                return;
            }
            let Some(item) = self.queue.pop() else {
                return;
            };
            self.queue.dispatched += 1;
            match item {
                Item::Msg(addr) | Item::Owed(addr) => self.dispatch(addr),
                Item::Notice(part, pending) => self.dispatch_notice(part, pending),
                Item::Wind => self.lock_next(),
            }
        }
    }

    /// §2.2: what one event becomes, pushed on the queue.
    pub(crate) fn become_messages(&mut self, event: port::Event) {
        use port::Event;
        if self.phase == crate::runtime::Phase::Starting && !matches!(event, Event::Start(_)) {
            self.fault("an event before Start");
            return;
        }
        self.log_port(&event.kind());
        match event {
            Event::Start(start) => self.started_by(start),
            Event::Time(time) => self.timed(time),
            Event::Input(input) => self.input(input),
            Event::Answer(id, answer) => self.answered(id, answer),
            Event::CloseAsked => {
                if let Some(addr) = self.root.close_asked() {
                    self.queue.push(Item::Msg(addr));
                }
            }
            fact => self.keep_fact(fact),
        }
    }

    /// §2.3: a notice to each top state built, in the fixed order.
    pub(crate) fn notify(&mut self, pending: Pending) {
        for part in PARTS {
            if self.tops.built(part) {
                self.queue.push(Item::Notice(part, pending.dup()));
            }
        }
    }

    /// R8: past the bound, drop the queued `Then`s and notices and note a fault. What answers
    /// something is carried into the next event.
    pub(crate) fn bound_reached(&mut self) {
        self.fault("a cascade past the bound");
        while let Some(item) = self.queue.pop() {
            match item {
                Item::Owed(addr) => self.queue.carried.push_back(Item::Owed(addr)),
                Item::Wind => self.queue.carried.push_back(Item::Wind),
                _ => {}
            }
        }
    }
}
