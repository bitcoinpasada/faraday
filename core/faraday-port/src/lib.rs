//! The port: the one contract between the application and the adapters. Events in, effects out,
//! the frame out (`docs/spec/02-port.md`).

#![no_std]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

extern crate alloc;

pub mod ask;
pub mod bytes;
pub mod camera;
pub mod devices;
pub mod event;
pub mod frame;
pub mod host;
pub mod order;
pub mod storage;
pub mod system;
pub mod values;

pub use ask::{Answer, Ask, AskId, Failure, Request};
pub use event::Event;
pub use frame::Frame;
pub use order::{Exit, Order};

use core::fmt;

/// The root's side: every adapter of a product behind one value.
pub trait Adapters {
    /// Carry out one effect. Never waits on a stick, a child process or a
    /// camera: slow work runs on the adapter's own thread and its answer
    /// comes back through `poll`.
    fn carry(&mut self, effect: Effect);
    /// The next event, or `None` when none waits.
    fn poll(&mut self) -> Option<Event>;
}

/// Given by the root to the runtime and the adapters: wakes the root's
/// loop when a worker or an adapter thread has something.
pub trait Waker: Send + Sync {
    fn wake(&self);
}

#[expect(
    clippy::large_enum_variant,
    reason = "the port's enums carry the domain's real types, which differ in size; a value is moved once through the queue"
)]
pub enum Effect {
    Ask(AskId, Ask),
    Order(Order),
}

/// The variant names of an `Effect`, which the runtime logs (P15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EffectKind {
    Ask,
    Order,
}

impl Effect {
    pub fn kind(&self) -> EffectKind {
        match self {
            Effect::Ask(..) => EffectKind::Ask,
            Effect::Order(_) => EffectKind::Order,
        }
    }
}

impl fmt::Debug for Effect {
    /// The kind, and an ask's id and kind; an order's payload is not printed (P16).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effect::Ask(id, ask) => write!(f, "Effect::Ask({id:?}, {ask:?})"),
            Effect::Order(_) => f.write_str("Effect::Order"),
        }
    }
}
