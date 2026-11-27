//! One event at a time, the cascade (05 §2).

use std::collections::VecDeque;

use faraday_kernel::{FactsChanged, shared};
use faraday_port as port;

use crate::root::{Addr, Root};
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

/// What a notice carries until it is dispatched.
pub(crate) enum Pending {
    Left,
    Hide,
    Shared(shared::Changed),
    Facts(FactsChanged),
}

/// One item of an event's queue: a message, or a notice for one top state.
pub(crate) enum Item<R: Root> {
    Msg(Addr<R>),
    Notice(Part, Pending),
}

pub(crate) struct Queue<R: Root> {
    items: VecDeque<Item<R>>,
    dispatched: u32,
}

impl<R: Root> Queue<R> {
    pub(crate) fn push(&mut self, _item: Item<R>) {
        todo!()
    }

    pub(crate) fn pop(&mut self) -> Option<Item<R>> {
        todo!()
    }
}

impl<R: Root> Runtime<R> {
    /// §2.1: the event becomes messages, which are dispatched first in first out; the orders
    /// and asks come back in the order made, with `Order::Clean` when the marker changed.
    pub(crate) fn settle(&mut self, _event: port::Event) -> Vec<port::Effect> {
        todo!()
    }

    /// §2.2: what one event becomes, pushed on the queue.
    pub(crate) fn become_messages(&mut self, _event: port::Event) {
        todo!()
    }

    /// §2.3: a notice to each top state built, in the fixed order.
    pub(crate) fn notify(&mut self, _pending: Pending) {
        todo!()
    }

    /// R8: past the bound, drop the queued `Then`s and notices and note a fault.
    pub(crate) fn bound_reached(&mut self) {
        todo!()
    }
}
