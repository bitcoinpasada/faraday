//! What a lock wipes, and what one build of the scope leaves the next (03 §2.3; 05 §7.3).

use faraday_port::bytes::Bytes;
use faraday_port::storage::{Carry, Files};
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
    entries: Vec<Entry>,
    only_copy: bool,
}

/// One named entry: the encoding of a `Public` value, with the flags of its name.
struct Entry {
    name: Text,
    first: bool,
    any_mode: bool,
    bytes: Bytes,
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
    pub const fn new(name: &'static str, first: bool, any_mode: bool) -> Self {
        CarryName {
            name,
            first,
            any_mode,
        }
    }
}

impl PartialEq for CarryName {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
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
    /// What the first build of a process starts from: nothing carried (05 §7.5).
    pub fn empty() -> Carried {
        Carried {
            inbox: Vec::new(),
            outbox: Vec::new(),
            entries: Vec::new(),
            only_copy: false,
        }
    }

    /// An entry under `name`; one already under that name is replaced.
    pub fn put<V: CarryValue>(&mut self, name: CarryName, value: &V) {
        let bytes = value.encode();
        self.entries
            .retain(|entry| entry.name.as_str() != name.name);
        self.entries.push(Entry {
            name: Text::of(name.name),
            first: name.first,
            any_mode: name.any_mode,
            bytes,
        });
    }

    /// Once; then `None`. An entry that cannot be read is gone all the same (05 R22).
    pub fn take<V: CarryValue>(&mut self, name: CarryName) -> Option<V> {
        let at = self
            .entries
            .iter()
            .position(|entry| entry.name.as_str() == name.name)?;
        let entry = self.entries.remove(at);
        V::decode(entry.bytes.expose())
    }

    /// The session's crate only: a held file, its bytes moved, never copied.
    pub fn put_file(&mut self, to: FileBox, name: Text, bytes: Bytes) {
        match to {
            FileBox::Inbox => self.inbox.push((name, bytes)),
            FileBox::Outbox => self.outbox.push((name, bytes)),
        }
    }

    pub fn take_files(&mut self, from: FileBox) -> Files {
        match from {
            FileBox::Inbox => core::mem::take(&mut self.inbox),
            FileBox::Outbox => core::mem::take(&mut self.outbox),
        }
    }

    /// Something carried has no other copy (05 §7.6). The session's crate only.
    pub fn mark_only_copy(&mut self) {
        self.only_copy = true;
    }

    /// The runtime's: whether something carried has no other copy.
    pub fn only_copy(&self) -> bool {
        self.only_copy
    }

    /// The runtime's: what the carry maps onto (05 §7.3). Each entry is a file of `kept` named by
    /// its name, the entries marked `first` before the rest; `only_copy` is not in it.
    pub fn into_carry(mut self) -> Carry {
        let mut kept: Files = Vec::with_capacity(self.entries.len());
        let entries = core::mem::take(&mut self.entries);
        let (first, rest): (Vec<Entry>, Vec<Entry>) =
            entries.into_iter().partition(|entry| entry.first);
        for entry in first.into_iter().chain(rest) {
            kept.push((entry.name, entry.bytes));
        }
        Carry {
            inbox: core::mem::take(&mut self.inbox),
            outbox: core::mem::take(&mut self.outbox),
            kept,
        }
    }

    /// The runtime's: a carry read back, each file of `kept` an entry by its name. `only_copy` is
    /// the runtime's to say, which it held aside (05 §7.3).
    pub fn from_carry(carry: Carry, only_copy: bool) -> Carried {
        let Carry {
            inbox,
            outbox,
            kept,
        } = carry;
        Carried {
            inbox,
            outbox,
            entries: kept
                .into_iter()
                .map(|(name, bytes)| Entry {
                    name,
                    first: false,
                    any_mode: false,
                    bytes,
                })
                .collect(),
            only_copy,
        }
    }

    /// The runtime's: a copy of the entries alone, no file. Entries are `Public` values, so the
    /// copy holds nothing secret; the runtime reads the mode asked for from it.
    pub fn entries_only(&self) -> Carried {
        Carried {
            inbox: Vec::new(),
            outbox: Vec::new(),
            entries: self
                .entries
                .iter()
                .map(|entry| Entry {
                    name: Text::of(entry.name.as_str()),
                    first: entry.first,
                    any_mode: entry.any_mode,
                    bytes: Bytes::new(entry.bytes.expose().clone()),
                })
                .collect(),
            only_copy: false,
        }
    }

    /// The runtime's: a lock that changes the mode carries only the entries marked `any_mode`
    /// (05 §7.3). The files stay.
    pub fn keep_any_mode(&mut self) {
        self.entries.retain(|entry| entry.any_mode);
    }
}
