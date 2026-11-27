//! Effects: one ordered list of orders, asks, changes, jobs, outs and notes (03 §4).

use faraday_crypto::work::Compute;
use faraday_port as port;

use crate::message::{Message, Public};
use crate::shared;
use crate::words::Worded;

/// One ordered list: an update's effects in the order it returned them (A15).
#[must_use]
pub struct Effects<M, O>(Vec<Fx<M, O>>);

pub enum Fx<M, O> {
    /// To an adapter, no answer (02 §4.9).
    Order(port::Order),
    /// To an adapter, answered once (02 §2). A write is whole bytes in one ask, written whole or
    /// not at all (A34).
    Ask(AskFor<M>),
    /// To a shared owner: the session, the vaults, the settings (§7).
    Change(ChangeFor<M>),
    /// Only an owner: the answer to a change it was given (§7.3).
    Done(shared::ChangeId, shared::Done),
    /// To the worker pool (§8).
    Job(Job<M>),
    /// Ask the runtime to stop this feature's running job of this kind (§8.3).
    Stop(JobKind),
    /// A message to this same feature, handled before the next event.
    Then(M),
    /// To the parent (A6).
    Out(O),
    /// A short line the frame shows (a snackbar), typed (§10).
    Note(Note),
    /// To the runtime's motion store: where the view goes (04 §8.2).
    View(ViewOrder),
}

impl<M: Message, O: Message> Effects<M, O> {
    pub fn none() -> Self {
        todo!()
    }

    /// The runtime's only: the effects, in the order they were made (05 §3).
    pub fn into_list(self) -> Vec<Fx<M, O>> {
        todo!()
    }

    pub fn order(self, _o: port::Order) -> Self {
        todo!()
    }

    pub fn ask<R: port::Request, C: Public>(
        self,
        _r: R,
        _context: C,
        _reply: fn(C, R::Answer) -> M,
    ) -> Self {
        todo!()
    }

    pub fn change<R: shared::Change, C: Public>(
        self,
        _r: R,
        _context: C,
        _reply: fn(C, R::Done) -> M,
    ) -> Self {
        todo!()
    }

    pub fn job(self, _j: Job<M>) -> Self {
        todo!()
    }

    pub fn then(self, _m: M) -> Self {
        todo!()
    }

    pub fn out(self, _o: O) -> Self {
        todo!()
    }

    pub fn note<W: Worded + Public>(self, _n: W) -> Self {
        todo!()
    }

    pub fn view(self, _v: ViewOrder) -> Self {
        todo!()
    }

    /// A composite takes a child's effects into its own (§6).
    pub fn lift<N: Message, P: Message>(
        self,
        _msg: fn(M) -> N,
        _out: fn(O) -> Route<N, P>,
    ) -> Effects<N, P> {
        todo!()
    }
}

/// What a composite does with a child's out.
pub enum Route<M, O> {
    /// A message to this composite: usually another child's, wrapped.
    To(M),
    /// Up to this composite's parent.
    Up(O),
    /// Nothing; the out is answered by doing nothing here.
    Drop,
}

/// What the runtime keeps of an ask: the reply, built only by `new` and `map`.
pub type Reply<M> = Box<dyn FnOnce(port::Answer) -> (M, Fault) + Send>;

/// What the runtime keeps of a change: the reply, built only by `new` and `map`.
pub type ChangeReply<M> = Box<dyn FnOnce(shared::Done) -> (M, Fault) + Send>;

/// P9: the runtime notes `WrongKind`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fault {
    None,
    WrongKind,
}

pub struct AskFor<M> {
    request: port::Ask,
    /// Built only by `new` and `map`; holds a Public context and fn pointers.
    reply: Reply<M>,
}

impl<M: Message> AskFor<M> {
    pub fn new<R: port::Request, C: Public>(
        _request: R,
        _context: C,
        _reply: fn(C, R::Answer) -> M,
    ) -> Self {
        todo!()
    }

    pub fn map<N: Message>(self, _f: fn(M) -> N) -> AskFor<N> {
        todo!()
    }

    /// The runtime's only: the request to carry and the reply to keep.
    pub fn into_parts(self) -> (port::Ask, Reply<M>) {
        todo!()
    }
}

/// A change to a shared owner, answered once as the asker's own message (§7.3).
pub struct ChangeFor<M> {
    change: shared::AnyChange,
    reply: ChangeReply<M>,
}

impl<M: Message> ChangeFor<M> {
    pub fn new<R: shared::Change, C: Public>(
        _change: R,
        _context: C,
        _reply: fn(C, R::Done) -> M,
    ) -> Self {
        todo!()
    }

    pub fn map<N: Message>(self, _f: fn(M) -> N) -> ChangeFor<N> {
        todo!()
    }

    /// The runtime's only: the change to hand to its owner and the reply to keep.
    pub fn into_parts(self) -> (shared::AnyChange, ChangeReply<M>) {
        todo!()
    }
}

pub struct Job<M> {
    kind: JobKind,
    /// Runs without a busy line.
    quiet: bool,
    /// On the heavy lane (05 §5.1).
    heavy: bool,
    /// Runs to its end at a lock (05 §7.2).
    seal: bool,
    run: Box<dyn FnOnce(&Compute) -> M + Send>,
}

/// A job opened by the runtime: its kind, its lane and flags, and the work to run.
pub struct JobParts<M> {
    pub kind: JobKind,
    pub quiet: bool,
    pub heavy: bool,
    pub seal: bool,
    pub run: Box<dyn FnOnce(&Compute) -> M + Send>,
}

impl<M: Message> Job<M> {
    /// The input is moved in, a secret included; `work` is a function.
    pub fn new<I: Send + 'static>(_kind: JobKind, _input: I, _work: fn(I, &Compute) -> M) -> Self {
        todo!()
    }

    pub fn quiet(self) -> Self {
        todo!()
    }

    /// The heavy lane, for work that asks Argon2id for memory (05 §5.1). Only the shared
    /// owners' crates call it.
    pub fn heavy(self) -> Self {
        todo!()
    }

    /// A seal: it runs to its end at a lock. Only the vault owner's crate calls it.
    pub fn seal(self) -> Self {
        todo!()
    }

    pub fn map<N: Message>(self, _f: fn(M) -> N) -> Job<N> {
        todo!()
    }

    /// The runtime's only: the job's parts, for the pool (05 §5).
    pub fn into_parts(self) -> JobParts<M> {
        todo!()
    }
}

/// "<feature>.<job>", a constant of the feature's crate.
#[derive(Public, Copy, PartialEq, Eq)]
pub struct JobKind(pub &'static str);

/// A typed value the frame's snackbar words when it is drawn (§10).
pub struct Note(Box<dyn Worded>);

impl Note {
    pub fn new<W: Worded + Public>(_w: W) -> Self {
        todo!()
    }
}

/// What an update may ask of the view (04 §8.2).
pub enum ViewOrder {
    /// Bring the anchor's place into view, gliding unless Reduce motion.
    Reveal(faraday_port::values::Anchor),
    /// The region named starts at its top.
    Top(faraday_port::values::Anchor),
}
