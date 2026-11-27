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
        Effects(Vec::new())
    }

    /// These effects, then `more`.
    pub fn and(mut self, more: Self) -> Self {
        self.0.extend(more.0);
        self
    }

    /// The runtime's only: the effects, in the order they were made (05 §3).
    pub fn into_list(self) -> Vec<Fx<M, O>> {
        self.0
    }

    fn push(mut self, fx: Fx<M, O>) -> Self {
        self.0.push(fx);
        self
    }

    pub fn order(self, o: port::Order) -> Self {
        self.push(Fx::Order(o))
    }

    pub fn ask<R: port::Request + 'static, C: Public>(
        self,
        r: R,
        context: C,
        reply: fn(C, R::Answer) -> M,
    ) -> Self
    where
        R::Answer: 'static,
    {
        self.push(Fx::Ask(AskFor::new(r, context, reply)))
    }

    pub fn change<R: shared::Change + 'static, C: Public>(
        self,
        r: R,
        context: C,
        reply: fn(C, R::Done) -> M,
    ) -> Self
    where
        R::Done: 'static,
    {
        self.push(Fx::Change(ChangeFor::new(r, context, reply)))
    }

    /// Only an owner: the answer to a change it was given (§7.3).
    pub fn done(self, id: shared::ChangeId, done: shared::Done) -> Self {
        self.push(Fx::Done(id, done))
    }

    pub fn job(self, j: Job<M>) -> Self {
        self.push(Fx::Job(j))
    }

    /// Ask the runtime to stop this feature's running job of this kind (§8.3).
    pub fn stop(self, kind: JobKind) -> Self {
        self.push(Fx::Stop(kind))
    }

    pub fn then(self, m: M) -> Self {
        self.push(Fx::Then(m))
    }

    pub fn out(self, o: O) -> Self {
        self.push(Fx::Out(o))
    }

    pub fn note<W: Worded + Public>(self, n: W) -> Self {
        self.push(Fx::Note(Note::new(n)))
    }

    pub fn view(self, v: ViewOrder) -> Self {
        self.push(Fx::View(v))
    }

    /// A composite takes a child's effects into its own (§6): the order is kept, a child's
    /// message becomes the composite's, and each out is routed by `out`.
    pub fn lift<N: Message, P: Message>(
        self,
        msg: fn(M) -> N,
        out: fn(O) -> Route<N, P>,
    ) -> Effects<N, P> {
        let mut lifted = Vec::with_capacity(self.0.len());
        for fx in self.0 {
            match fx {
                Fx::Order(o) => lifted.push(Fx::Order(o)),
                Fx::Ask(a) => lifted.push(Fx::Ask(a.map(msg))),
                Fx::Change(c) => lifted.push(Fx::Change(c.map(msg))),
                Fx::Done(id, done) => lifted.push(Fx::Done(id, done)),
                Fx::Job(j) => lifted.push(Fx::Job(j.map(msg))),
                Fx::Stop(kind) => lifted.push(Fx::Stop(kind)),
                Fx::Then(m) => lifted.push(Fx::Then(msg(m))),
                Fx::Out(o) => match out(o) {
                    Route::To(n) => lifted.push(Fx::Then(n)),
                    Route::Up(p) => lifted.push(Fx::Out(p)),
                    Route::Drop => {}
                },
                Fx::Note(n) => lifted.push(Fx::Note(n)),
                Fx::View(v) => lifted.push(Fx::View(v)),
            }
        }
        Effects(lifted)
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
    /// An answer of another kind than the request's reaches the reply as `R::fault()`, with
    /// `Fault::WrongKind` for the runtime to note (P9).
    pub fn new<R: port::Request + 'static, C: Public>(
        request: R,
        context: C,
        reply: fn(C, R::Answer) -> M,
    ) -> Self
    where
        R::Answer: 'static,
    {
        AskFor {
            request: request.into(),
            reply: Box::new(move |answer| match R::take(answer) {
                Ok(answer) => (reply(context, answer), Fault::None),
                Err(_wrong) => (reply(context, R::fault()), Fault::WrongKind),
            }),
        }
    }

    pub fn map<N: Message>(self, f: fn(M) -> N) -> AskFor<N> {
        let reply = self.reply;
        AskFor {
            request: self.request,
            reply: Box::new(move |answer| {
                let (m, fault) = reply(answer);
                (f(m), fault)
            }),
        }
    }

    /// The runtime's only: the request to carry and the reply to keep.
    pub fn into_parts(self) -> (port::Ask, Reply<M>) {
        (self.request, self.reply)
    }
}

/// A change to a shared owner, answered once as the asker's own message (§7.3).
pub struct ChangeFor<M> {
    change: shared::AnyChange,
    reply: ChangeReply<M>,
}

impl<M: Message> ChangeFor<M> {
    pub fn new<R: shared::Change + 'static, C: Public>(
        change: R,
        context: C,
        reply: fn(C, R::Done) -> M,
    ) -> Self
    where
        R::Done: 'static,
    {
        ChangeFor {
            change: change.into(),
            reply: Box::new(move |done| match R::take(done) {
                Ok(done) => (reply(context, done), Fault::None),
                Err(_wrong) => (reply(context, R::fault()), Fault::WrongKind),
            }),
        }
    }

    pub fn map<N: Message>(self, f: fn(M) -> N) -> ChangeFor<N> {
        let reply = self.reply;
        ChangeFor {
            change: self.change,
            reply: Box::new(move |done| {
                let (m, fault) = reply(done);
                (f(m), fault)
            }),
        }
    }

    /// The runtime's only: the change to hand to its owner and the reply to keep.
    pub fn into_parts(self) -> (shared::AnyChange, ChangeReply<M>) {
        (self.change, self.reply)
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
    pub fn new<I: Send + 'static>(kind: JobKind, input: I, work: fn(I, &Compute) -> M) -> Self {
        Job {
            kind,
            quiet: false,
            heavy: false,
            seal: false,
            run: Box::new(move |compute| work(input, compute)),
        }
    }

    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    /// The heavy lane, for work that asks Argon2id for memory (05 §5.1). Only the shared
    /// owners' crates call it.
    pub fn heavy(mut self) -> Self {
        self.heavy = true;
        self
    }

    /// A seal: it runs to its end at a lock. Only the vault owner's crate calls it.
    pub fn seal(mut self) -> Self {
        self.seal = true;
        self
    }

    pub fn map<N: Message>(self, f: fn(M) -> N) -> Job<N> {
        let run = self.run;
        Job {
            kind: self.kind,
            quiet: self.quiet,
            heavy: self.heavy,
            seal: self.seal,
            run: Box::new(move |compute| f(run(compute))),
        }
    }

    /// The runtime's only: the job's parts, for the pool (05 §5).
    pub fn into_parts(self) -> JobParts<M> {
        JobParts {
            kind: self.kind,
            quiet: self.quiet,
            heavy: self.heavy,
            seal: self.seal,
            run: self.run,
        }
    }
}

/// "<feature>.<job>", a constant of the feature's crate.
#[derive(Public, Copy, PartialEq, Eq)]
pub struct JobKind(pub &'static str);

/// A typed value the frame's snackbar words when it is drawn (§10).
pub struct Note(Box<dyn Worded>);

impl Note {
    pub fn new<W: Worded + Public>(w: W) -> Self {
        Note(Box::new(w))
    }
}

/// What an update may ask of the view (04 §8.2).
pub enum ViewOrder {
    /// Bring the anchor's place into view, gliding unless Reduce motion.
    Reveal(faraday_port::values::Anchor),
    /// The region named starts at its top.
    Top(faraday_port::values::Anchor),
}
