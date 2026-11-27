//! What a lock wipes, and what one build of the scope leaves the next (03 §2.3; 05 §7.3).

use faraday_port::bytes::Bytes;
use faraday_port::storage::Files;
use faraday_text::Text;

use crate::message::Public;

pub use faraday_kernel_derive::AtLock;

pub trait AtLock {
    /// What this state holds now; read by the runtime for the clean marker (`Order::Clean`),
    /// handing a stick out, the idle lock and the Lock sheet.
    fn holds(&self) -> Holds;
    /// Take every secret on show off the screen, keeping what was typed: at Lock pressed (even
    /// while a seal waits), at the idle warning, and at every step change of a flow (§9).
    fn hide(&mut self);
    /// Wipe what a lock wipes. Called after the seal and the carry, before the scope's states
    /// are dropped (05 §7.2).
    fn at_lock(&mut self);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Holds {
    /// A key store or an open vault holds material.
    pub kept: bool,
    /// A secret on its way: typed, rolled, scanned, shown.
    pub typed: bool,
    /// Something a lock loses that is not secret.
    pub session: bool,
    /// A lock would lose this; the Lock sheet names it and asks first.
    pub asks_first: Option<Loss>,
}

/// What a lock would lose, for the Lock sheet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Loss(pub &'static str);

/// What one build of the scope leaves the next, by name (05 §7.3).
pub struct Carried {
    inbox: Files,
    outbox: Files,
    entries: Vec<(CarryName, Bytes)>,
    only_copy: bool,
}

/// "<feature>.<entry>", a constant of the feature's crate. Compared by `name` alone.
#[derive(Public, Copy)]
pub struct CarryName {
    name: &'static str,
    first: bool,
    any_mode: bool,
}

impl CarryName {
    /// A feature's crate declares each of its names as a `const` (09 §4.1).
    pub const fn new(_name: &'static str, _first: bool, _any_mode: bool) -> Self {
        todo!()
    }
}

impl PartialEq for CarryName {
    fn eq(&self, _other: &Self) -> bool {
        todo!()
    }
}

/// A carried value: Public, so no secret crosses a lock in an entry.
pub trait CarryValue: Public + Sized {
    fn encode(&self) -> Bytes;
    fn decode(bytes: &[u8]) -> Option<Self>;
}

/// Where a held file lives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileBox {
    Inbox,
    Outbox,
}

impl Carried {
    pub fn put<V: CarryValue>(&mut self, _name: CarryName, _value: &V) {
        todo!()
    }

    /// Once; then `None`.
    pub fn take<V: CarryValue>(&mut self, _name: CarryName) -> Option<V> {
        todo!()
    }

    /// The session's crate only: a held file, its bytes moved, never copied.
    pub fn put_file(&mut self, _to: FileBox, _name: Text, _bytes: Bytes) {
        todo!()
    }

    pub fn take_files(&mut self, _from: FileBox) -> Files {
        todo!()
    }

    /// Something carried has no other copy (05 §7.6). The session's crate only.
    pub fn mark_only_copy(&mut self) {
        todo!()
    }
}
