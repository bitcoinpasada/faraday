//! Orders, asks and replies, changes served (05 §3).

use core::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;

use faraday_crypto::work::Compute;
use faraday_kernel::shared::Owner;
use faraday_kernel::{
    AtLock, ChangeId, Ctx, Effects, Fault, Feature, Fx, Job, Message, Notice, Running, Top, route,
    session, shared, vaults,
};
use faraday_port as port;

use crate::facts::{Lent, as_was, build_view};
use crate::pool::{Answered, JobId, Lane, Queued};
use crate::queue::{Item, Part, Pending};
use crate::root::{Addr, Root, has_outside};
use crate::runtime::{Phase, Runtime};

/// A kept reply, turned into the message of the top state that asked.
pub(crate) type KeptReply<R> = Box<dyn FnOnce(port::Answer) -> (Addr<R>, Fault) + Send>;

/// A kept change reply, likewise.
pub(crate) type KeptChange<R> = Box<dyn FnOnce(shared::Done) -> (Addr<R>, Fault) + Send>;

/// The reply table: each id to its reply and its top state (§3.1).
pub(crate) struct Replies<R: Root> {
    asks: HashMap<(u32, u32), (Part, KeptReply<R>)>,
    changes: HashMap<(u32, u32), (Part, KeptChange<R>)>,
    /// The next `n` of an ask, a change and a job: the part outside the scope (generation 0)
    /// counts apart from the scope (§3.1).
    next: [Counters; 2],
}

#[derive(Clone, Copy, Default)]
struct Counters {
    ask: u32,
    change: u32,
    job: u32,
}

impl<R: Root> Replies<R> {
    pub(crate) fn new() -> Self {
        Replies {
            asks: HashMap::new(),
            changes: HashMap::new(),
            next: [Counters::default(); 2],
        }
    }

    fn counters(&mut self, generation: u32) -> &mut Counters {
        &mut self.next[usize::from(generation != 0)]
    }

    /// The scope is built again: its ids count from the start (§3.1).
    pub(crate) fn new_generation(&mut self) {
        self.next[1] = Counters::default();
    }

    /// §7.2 step 7: the scope's waiting replies are dropped (their contexts are `Public`); the
    /// part outside the scope keeps its own (generation 0).
    pub(crate) fn drop_scope(&mut self) {
        self.asks.retain(|(generation, _), _| *generation == 0);
        self.changes.retain(|(generation, _), _| *generation == 0);
    }

    pub(crate) fn job_id(&mut self, generation: u32) -> JobId {
        let counters = self.counters(generation);
        let n = counters.job;
        counters.job += 1;
        JobId { generation, n }
    }
}

impl<R: Root> Runtime<R> {
    /// The generation a part's asks, changes and jobs carry: 0 outside the scope (R26).
    pub(crate) fn gen_of(&self, part: Part) -> u32 {
        match part {
            Part::Outside => 0,
            Part::Settings if has_outside::<R>() => 0,
            _ => self.generation,
        }
    }

    /// Whether the scope is locking or ending: only what serves `Close` is delivered (R16).
    pub(crate) fn winding_down(&self) -> bool {
        matches!(self.phase, Phase::Locking | Phase::Ending)
    }

    /// R16: while the scope winds down, what is delivered to a top state is what serves
    /// `Close` (the vault owner's) and every answer of generation 0.
    pub(crate) fn delivers(&self, part: Part) -> bool {
        !self.winding_down() || part == Part::Vaults || self.gen_of(part) == 0
    }

    /// Walk the effects a top state returned, in order (§3), from `Effects::into_list`.
    /// `wrap` addresses one of that top state's own messages to it.
    pub(crate) fn walk<M: Message>(
        &mut self,
        from: Part,
        effects: Effects<M, Top>,
        wrap: fn(M) -> Addr<R>,
    ) {
        for fx in effects.into_list() {
            match fx {
                Fx::Order(order) => self.order(order),
                Fx::Ask(ask) => {
                    let (request, reply) = ask.into_parts();
                    self.ask(
                        from,
                        request,
                        Box::new(move |answer| {
                            let (message, fault) = reply(answer);
                            (wrap(message), fault)
                        }),
                    );
                }
                Fx::Change(change) => {
                    let (change, reply) = change.into_parts();
                    self.change(
                        from,
                        change,
                        Box::new(move |done| {
                            let (message, fault) = reply(done);
                            (wrap(message), fault)
                        }),
                    );
                }
                Fx::Done(id, done) => self.done(id, done),
                Fx::Job(job) => self.job(from, job, wrap),
                Fx::Stop(kind) => self.pool.stop(from, kind),
                Fx::Then(message) => self.queue.push(Item::Msg(wrap(message))),
                Fx::Out(top) => self.top(from, top),
                Fx::Note(note) => {
                    if self.notes.queue(note, self.clock.now_ms) {
                        self.frame.want();
                    }
                }
                Fx::View(order) => self.frame.orders.push(order),
            }
        }
    }

    /// `Fx::Order`: carried to the root in the order made. `CameraOff` also ends the stream.
    pub(crate) fn order(&mut self, order: port::Order) {
        if matches!(order, port::Order::CameraOff) {
            self.scope.stream = None;
        }
        self.out.push(port::Effect::Order(order));
    }

    /// `Fx::Ask`: a new `AskId`, the reply kept, `Effect::Ask` appended.
    pub(crate) fn ask(&mut self, from: Part, ask: port::Ask, reply: KeptReply<R>) {
        let generation = self.gen_of(from);
        let counters = self.replies.counters(generation);
        let n = counters.ask;
        counters.ask += 1;
        self.replies.asks.insert((generation, n), (from, reply));
        self.out
            .push(port::Effect::Ask(port::AskId { generation, n }, ask));
    }

    /// An ask of the runtime's own (§7.2 step 8, §7.7): a new `AskId` of the scope's generation,
    /// no reply kept, `Effect::Ask` appended.
    pub(crate) fn ask_own(&mut self, ask: port::Ask) -> port::AskId {
        let generation = self.generation;
        let counters = self.replies.counters(generation);
        let n = counters.ask;
        counters.ask += 1;
        let id = port::AskId { generation, n };
        self.out.push(port::Effect::Ask(id, ask));
        id
    }

    /// A change of the runtime's own (§7.2 step 3): a new `ChangeId` of the scope's generation.
    pub(crate) fn change_id_own(&mut self) -> ChangeId {
        let generation = self.generation;
        let counters = self.replies.counters(generation);
        let n = counters.change;
        counters.change += 1;
        ChangeId { generation, n }
    }

    /// `Fx::Change`: a new `ChangeId`, the reply kept, `Owner::given` pushed to its owner.
    pub(crate) fn change(&mut self, from: Part, change: shared::AnyChange, reply: KeptChange<R>) {
        if !self.delivers(from) {
            return;
        }
        let owner = match &change {
            shared::AnyChange::Session(_) => Part::Session,
            shared::AnyChange::Vaults(_) => Part::Vaults,
            shared::AnyChange::Settings(_) => Part::Settings,
        };
        if !self.tops.built(owner) {
            self.fault("a change to an owner not built");
            return;
        }
        let generation = self.gen_of(from);
        let counters = self.replies.counters(generation);
        let n = counters.change;
        counters.change += 1;
        let id = ChangeId { generation, n };
        self.replies.changes.insert((generation, n), (from, reply));
        let addr = match owner {
            Part::Session => Addr::Session(<R::Session as Owner>::given(id, change)),
            Part::Vaults => Addr::Vaults(<R::Vaults as Owner>::given(id, change)),
            _ => Addr::Settings(<R::Settings as Owner>::given(id, change)),
        };
        self.queue.push(Item::Owed(addr));
    }

    /// `Fx::Done`: the kept reply under `id` called, the asker's message pushed. A change whose
    /// id is not in the table is dropped and noted (R12).
    pub(crate) fn done(&mut self, id: ChangeId, done: shared::Done) {
        if self.scope.close == Some(id) {
            // The runtime's own change at a lock: read, never routed (§7.2 step 3).
            self.scope.close = None;
            self.lock_next();
            return;
        }
        let Some((part, reply)) = self.replies.changes.remove(&(id.generation, id.n)) else {
            self.fault("a change answer with no change waiting");
            return;
        };
        if !self.delivers(part) {
            return;
        }
        let (addr, fault) = reply(done);
        if fault == Fault::WrongKind {
            self.fault("a change answer of the wrong kind");
        }
        self.queue.push(Item::Owed(addr));
    }

    /// `Event::Answer`: the kept reply called; an answer not in the table is dropped and wiped
    /// as it drops (R11).
    pub(crate) fn answered(&mut self, id: port::AskId, answer: port::Answer) {
        if self.scope.keep == Some(id) {
            self.scope.keep = None;
            self.kept(answer);
            return;
        }
        if self.scope.forget == Some(id) {
            self.scope.forget = None;
            self.forgotten();
            return;
        }
        let Some((part, reply)) = self.replies.asks.remove(&(id.generation, id.n)) else {
            self.fault("an answer with no ask waiting");
            return;
        };
        if !self.delivers(part) {
            return;
        }
        if matches!(answer, port::Answer::Camera(Ok(_))) {
            self.scope.stream = Some(id);
        }
        let (addr, fault) = reply(answer);
        if fault == Fault::WrongKind {
            self.fault("an answer of the wrong kind");
        }
        self.queue.push(Item::Owed(addr));
    }

    /// `Fx::Job`: opened with `into_parts` and handed to the pool, its answer addressed to the
    /// top state that started it. Once the scope is locking or ending, a job of it that is not
    /// a seal is dropped at once, unstarted (§3.3).
    fn job<M: Message>(&mut self, from: Part, job: Job<M>, wrap: fn(M) -> Addr<R>) {
        let parts = job.into_parts();
        let generation = self.gen_of(from);
        if self.winding_down() && !parts.seal && generation != 0 {
            return;
        }
        let id = self.replies.job_id(generation);
        let lane = if parts.heavy {
            Lane::Heavy
        } else {
            Lane::Light
        };
        let run = parts.run;
        let seal = parts.seal;
        self.pool.clock(self.clock.now_ms);
        self.pool.submit(
            lane,
            Queued {
                id,
                part: from,
                kind: parts.kind,
                quiet: parts.quiet,
                seal: parts.seal,
                run: Box::new(move |compute| Answered {
                    id,
                    part: from,
                    seal,
                    message: Box::new(wrap(run(compute))),
                }),
            },
        );
    }

    /// `Fx::Out`: handled as 05 §1.1 says.
    pub(crate) fn top(&mut self, from: Part, top: Top) {
        match top {
            Top::Hide => self.notify(Pending::Hide),
            Top::Shared(changed) => self.notify(Pending::Shared(Arc::new(changed))),
            Top::Lock(lock) => {
                if self.phase == Phase::Running {
                    self.begin_lock(lock);
                } else {
                    self.fault("a Lock while the scope is not running");
                }
            }
            Top::End(end) => {
                // A Quit from inside the scope, where a part stands outside it, closes the scope
                // alone (§7.7, §7.9).
                let inside = matches!(from, Part::Session | Part::Vaults | Part::App);
                if self.phase == Phase::Running
                    && has_outside::<R>()
                    && end == faraday_kernel::End::Quit
                    && inside
                {
                    self.close_scope();
                } else if matches!(self.phase, Phase::Running | Phase::Closed) {
                    self.begin_end(end);
                } else {
                    self.fault("an End while one is under way");
                }
            }
            Top::ModeLock => {
                if let Some(addr) = self.root.mode_lock_asked() {
                    self.queue.push(Item::Msg(addr));
                }
            }
            Top::SelfTest => self.run_selftest(),
            Top::Open => {
                if has_outside::<R>() && self.phase == Phase::Closed {
                    self.build_scope(None);
                } else {
                    self.fault("an Open where there is nothing to open");
                }
            }
        }
    }

    /// One message to its top state through `route` (A9), with `View` built as R13 says.
    pub(crate) fn dispatch(&mut self, addr: Addr<R>) {
        let part = addr.part();
        match &addr {
            Addr::Settings(m) => self.log_message(m),
            Addr::Session(m) => self.log_message(m),
            Addr::Vaults(m) => self.log_message(m),
            Addr::App(m) => self.log_message(m),
            Addr::Outside(m) => self.log_message(m),
        }
        self.deliver(part, Some(addr), None);
    }

    /// One notice to its top state (§2.3).
    pub(crate) fn dispatch_notice(&mut self, part: Part, pending: Pending) {
        let notice = match &pending {
            Pending::Left => Notice::Left,
            Pending::Hide => Notice::Hide,
            Pending::Shared(changed) => Notice::Shared(changed),
            Pending::Facts(facts) => Notice::Facts(*facts),
        };
        self.deliver(part, None, Some(&notice));
    }

    /// Lend `View` to one top state, as 04 §4.1 and R13 say, and hand it a message or a notice:
    /// the top state is borrowed `&mut`, every other is lent `&`, and the slot of the one being
    /// dispatched to, and every slot of the part outside the scope, hold the empty reader.
    fn deliver(&mut self, part: Part, msg: Option<Addr<R>>, notice: Option<&Notice<'_>>) {
        let Some(mode) = self.mode else {
            self.fault("a dispatch before the model was built");
            return;
        };
        let started = self.started;
        let outside = has_outside::<R>();
        let Runtime {
            tops,
            facts,
            clock,
            running,
            readers,
            compute,
            ..
        } = &mut *self;
        let facts = &facts.facts;
        let readers: &shared::Empty = readers;
        let compute: &Compute = compute;
        let running: &[Running] = running;
        let fallback = shared::Settings::default();
        match part {
            Part::Settings => {
                let Some(state) = tops.settings.as_mut() else {
                    self.fault("a message to a top state not built");
                    return;
                };
                let before = as_was(Borrow::<shared::Settings>::borrow(&*state));
                let (session, vaults) = if outside {
                    (readers as &dyn session::Read, readers as &dyn vaults::Read)
                } else {
                    (
                        tops.session
                            .as_ref()
                            .map_or(readers as &dyn session::Read, |s| s as &dyn session::Read),
                        tops.vaults
                            .as_ref()
                            .map_or(readers as &dyn vaults::Read, |v| v as &dyn vaults::Read),
                    )
                };
                let lent = Lent {
                    settings: &before,
                    session,
                    vaults,
                };
                let cx = Ctx {
                    view: build_view(lent, facts, mode, clock, running, started),
                    compute,
                };
                let message = match msg {
                    Some(Addr::Settings(m)) => Some(m),
                    _ => None,
                };
                let fx = step::<R::Settings>(state, message, notice, &cx);
                self.walk(Part::Settings, fx, Addr::Settings);
            }
            Part::Session => {
                let Some(state) = tops.session.as_mut() else {
                    self.fault("a message to a top state not built");
                    return;
                };
                let settings = tops
                    .settings
                    .as_ref()
                    .map_or(&fallback, |s| Borrow::<shared::Settings>::borrow(s));
                let lent = Lent {
                    settings,
                    session: readers,
                    vaults: tops
                        .vaults
                        .as_ref()
                        .map_or(readers as &dyn vaults::Read, |v| v as &dyn vaults::Read),
                };
                let cx = Ctx {
                    view: build_view(lent, facts, mode, clock, running, started),
                    compute,
                };
                let message = match msg {
                    Some(Addr::Session(m)) => Some(m),
                    _ => None,
                };
                let fx = step::<R::Session>(state, message, notice, &cx);
                self.walk(Part::Session, fx, Addr::Session);
            }
            Part::Vaults => {
                let Some(state) = tops.vaults.as_mut() else {
                    self.fault("a message to a top state not built");
                    return;
                };
                let settings = tops
                    .settings
                    .as_ref()
                    .map_or(&fallback, |s| Borrow::<shared::Settings>::borrow(s));
                let lent = Lent {
                    settings,
                    session: tops
                        .session
                        .as_ref()
                        .map_or(readers as &dyn session::Read, |s| s as &dyn session::Read),
                    vaults: readers,
                };
                let cx = Ctx {
                    view: build_view(lent, facts, mode, clock, running, started),
                    compute,
                };
                let message = match msg {
                    Some(Addr::Vaults(m)) => Some(m),
                    _ => None,
                };
                let fx = step::<R::Vaults>(state, message, notice, &cx);
                self.walk(Part::Vaults, fx, Addr::Vaults);
            }
            Part::App => {
                let Some(state) = tops.app.as_mut() else {
                    self.fault("a message to a top state not built");
                    return;
                };
                let settings = tops
                    .settings
                    .as_ref()
                    .map_or(&fallback, |s| Borrow::<shared::Settings>::borrow(s));
                let lent = Lent {
                    settings,
                    session: tops
                        .session
                        .as_ref()
                        .map_or(readers as &dyn session::Read, |s| s as &dyn session::Read),
                    vaults: tops
                        .vaults
                        .as_ref()
                        .map_or(readers as &dyn vaults::Read, |v| v as &dyn vaults::Read),
                };
                let cx = Ctx {
                    view: build_view(lent, facts, mode, clock, running, started),
                    compute,
                };
                let message = match msg {
                    Some(Addr::App(m)) => Some(m),
                    _ => None,
                };
                let fx = step::<R::App>(state, message, notice, &cx);
                self.walk(Part::App, fx, Addr::App);
            }
            Part::Outside => {
                let Some(state) = tops.outside.as_mut() else {
                    self.fault("a message to a top state not built");
                    return;
                };
                let settings = tops
                    .settings
                    .as_ref()
                    .map_or(&fallback, |s| Borrow::<shared::Settings>::borrow(s));
                let lent = Lent {
                    settings,
                    session: readers,
                    vaults: readers,
                };
                let cx = Ctx {
                    view: build_view(lent, facts, mode, clock, running, started),
                    compute,
                };
                let message = match msg {
                    Some(Addr::Outside(m)) => Some(m),
                    _ => None,
                };
                let fx = step::<R::Outside>(state, message, notice, &cx);
                self.walk(Part::Outside, fx, Addr::Outside);
            }
        }
    }
}

/// One top state's turn: a message through `route`, or a notice (with `hide` after a `Hide`).
fn step<F: Feature>(
    state: &mut F::State,
    msg: Option<F::Msg>,
    notice: Option<&Notice<'_>>,
    cx: &Ctx<'_>,
) -> Effects<F::Msg, F::Out> {
    if let Some(msg) = msg {
        return route::<F>(state, msg, cx);
    }
    if let Some(notice) = notice {
        let effects = F::notice(state, notice, cx);
        if matches!(notice, Notice::Hide) {
            state.hide();
        }
        return effects;
    }
    Effects::none()
}
