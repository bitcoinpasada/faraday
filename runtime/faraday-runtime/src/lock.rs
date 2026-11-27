//! The lock order, the carry in hand, the scope's build (05 §7).

use core::borrow::Borrow;

use faraday_atoms::MotionStore;
use faraday_crypto::work::Compute;
use faraday_kernel::{
    AtLock, Carried, Clock, Ctx, Effects, End, Facts, Feature, Holds, Line, Lock, Mode, Note,
    Public, Running, Show, Started, Worded, shared, vaults,
};
use faraday_port as port;

use crate::facts::{Lent, build_view, default_settings};
use crate::queue::{Item, PARTS, Part, Pending, in_scope};
use crate::root::{Addr, OwnerState, Root, has_outside};
use crate::runtime::{Built, Phase, Runtime};

/// How far a lock or an end of the scope has got (§7.2, §7.7).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Step {
    /// Hidden and stopped; the order goes on at the next queue item (an end) or once the seal
    /// has answered (a lock).
    Hide,
    /// The seal's `Close` has answered: waiting for every job of the scope to have returned.
    Seal,
    Wait,
    /// The keep (a lock) or the forget (an end) is asked and its answer awaited.
    Keep,
}

/// What the runtime holds of the scope across a lock.
pub(crate) struct Scope {
    /// `only_copy`, held aside for the whole Keep (§7.3).
    pub(crate) only_copy: bool,
    /// The lock being run, and the step it is at.
    pub(crate) lock: Option<(Lock, Step)>,
    /// The end being run, and the step it is at.
    pub(crate) end: Option<(End, Step)>,
    /// The end is the Starter's Close over the scope alone (§7.9).
    pub(crate) closing: bool,
    /// The id of the `Close` change the runtime reads and never routes (§7.2 step 3).
    pub(crate) close: Option<shared::ChangeId>,
    /// The camera stream running, by the id of the ask answered `Started` (§5.5).
    pub(crate) stream: Option<port::AskId>,
    /// The `Keep` ask waiting for its answer (§7.2 step 8).
    pub(crate) keep: Option<port::AskId>,
    /// The `Forget` ask waiting for its answer (§7.7 step 4).
    pub(crate) forget: Option<port::AskId>,
}

impl Scope {
    pub(crate) fn new() -> Self {
        Scope {
            only_copy: false,
            lock: None,
            end: None,
            closing: false,
            close: None,
            stream: None,
            keep: None,
            forget: None,
        }
    }
}

/// What a `start` reads beside the owners lent to it (§7.5 step 3).
struct StartEnv<'a> {
    facts: &'a Facts,
    mode: Mode,
    clock: &'a Clock,
    running: &'a [Running],
    started: Started,
    compute: &'a Compute,
}

/// The environment of a start, from separate fields of the runtime, so the top states built
/// so far can be lent beside it.
macro_rules! env {
    ($runtime:ident, $mode:expr, $started:expr) => {
        StartEnv {
            facts: &$runtime.facts.facts,
            mode: $mode,
            clock: &$runtime.clock,
            running: &$runtime.running,
            started: $started,
            compute: &$runtime.compute,
        }
    };
}

/// One top state's `start`, lent the owners started before it (§7.5 step 3).
fn start_one<F: Feature>(
    carried: &mut Carried,
    lent: Lent<'_>,
    env: StartEnv<'_>,
) -> (F::State, Effects<F::Msg, F::Out>) {
    let cx = Ctx {
        view: build_view(
            lent,
            env.facts,
            env.mode,
            env.clock,
            env.running,
            env.started,
        ),
        compute: env.compute,
    };
    F::start(carried, &cx)
}

/// The settings a part being started reads: the owner's, once built, else the defaults.
fn settings_of<'a, R: Root>(
    built: &'a Option<OwnerState<R::Settings>>,
    fallback: &'a shared::Settings,
) -> &'a shared::Settings {
    built
        .as_ref()
        .map_or(fallback, |s| Borrow::<shared::Settings>::borrow(s))
}

/// The note a scope rebuilt after a wrong answer to `Keep` puts up: what was carried was lost
/// (§7.6). Nothing is retried.
#[derive(Public, Copy, PartialEq, Eq)]
pub(crate) struct KeepLost;

impl Worded for KeepLost {
    fn words(&self, show: &Show<'_>) -> Line {
        Line::Fixed(show.strings().lock_keep_lost)
    }
}

/// Whether a state holds nothing at all: what A12 asserts after the wipe.
fn holds_nothing(holds: Holds) -> bool {
    !holds.kept && !holds.typed && !holds.session && holds.asks_first.is_none()
}

impl<R: Root> Runtime<R> {
    /// §7.2: the nine steps, in order, by every way a lock is asked (R19). Steps 1 to 3 here; the
    /// rest from `lock_next`, once what each waits for has come.
    pub(crate) fn begin_lock(&mut self, lock: Lock) {
        self.phase = Phase::Locking;
        self.scope.lock = Some((lock, Step::Hide));
        self.hide_scope(false);
        self.pool.stop_all(true, false);
        if self.tops.vaults.is_none() {
            self.scope.lock = Some((lock, Step::Seal));
            self.queue.push(Item::Wind);
            return;
        }
        let id = self.change_id_own();
        self.scope.close = Some(id);
        let close =
            shared::AnyChange::Vaults(vaults::Change::Close(vaults::Close { seal: lock.seal }));
        self.scope.lock = Some((lock, Step::Seal));
        self.queue.push(Item::Owed(Addr::Vaults(
            <R::Vaults as shared::Owner>::given(id, close),
        )));
    }

    /// Whether a lock or an end is waiting for the scope's jobs to return.
    pub(crate) fn waiting_for_jobs(&self) -> bool {
        matches!(self.scope.lock, Some((_, Step::Wait)))
            || matches!(self.scope.end, Some((_, Step::Wait)))
    }

    /// Goes on from the step waiting, once what it waited for has come.
    pub(crate) fn lock_next(&mut self) {
        if let Some((lock, step)) = self.scope.lock {
            self.lock_step(lock, step);
        } else if let Some((end, step)) = self.scope.end {
            self.end_step(end, step);
        }
    }

    /// Step 1: every secret on show leaves the screen. `Notice::Hide` goes to the top states of
    /// the scope (all of them where `whole`), a camera stream is turned off (R17), and the next
    /// frame is painted whole with no motion state kept.
    fn hide_scope(&mut self, whole: bool) {
        for part in PARTS {
            if self.tops.built(part) && (whole || in_scope::<R>(part)) {
                self.queue.push(Item::Notice(part, Pending::Hide));
            }
        }
        if self.scope.stream.is_some() {
            self.order(port::Order::CameraOff);
        }
        self.motion = MotionStore::new();
        self.frame.wanted = true;
    }

    /// Whether every job of the scope has returned; what came is handed to `job_answered`, which
    /// delivers only a seal's (R16).
    fn jobs_done(&mut self, whole: bool) -> bool {
        for answer in self.pool.answers() {
            self.job_answered(answer);
        }
        self.running = self.pool.running();
        !self.pool.scope_running(whole)
    }

    fn lock_step(&mut self, lock: Lock, step: Step) {
        if step == Step::Keep || step == Step::Hide {
            return;
        }
        // Step 4: `Close` has answered; now the jobs.
        self.scope.lock = Some((lock, Step::Wait));
        if !self.jobs_done(false) {
            return;
        }
        // Step 5.
        let mut carried = Carried::empty();
        self.carry_all(&mut carried);
        let mode = {
            let mut probe = carried.entries_only();
            self.root.mode(&mut probe, &self.facts.facts)
        };
        if self.mode != Some(mode) {
            carried.keep_any_mode();
        }
        // Steps 6 and 7.
        self.wipe_and_drop(false);
        let carry = self.carry_out(carried);
        if self.facts.facts.abilities.lock_ends_process {
            // Step 8.
            let id = self.ask_own(port::Ask::Keep(carry));
            self.scope.keep = Some(id);
            self.scope.lock = Some((lock, Step::Keep));
        } else {
            // Step 9, in process.
            self.scope.lock = None;
            self.rebuild(Some(carry), Started::Lock, false);
        }
    }

    /// §7.7: the order of an end, with no seal. Steps 1 and 2 here.
    pub(crate) fn begin_end(&mut self, end: End) {
        self.phase = Phase::Ending;
        self.scope.end = Some((end, Step::Hide));
        self.hide_scope(true);
        self.pool.stop_all(false, true);
        self.queue.push(Item::Wind);
    }

    /// §7.9: the Starter's Close over the scope alone.
    pub(crate) fn close_scope(&mut self) {
        self.phase = Phase::Ending;
        self.scope.end = Some((End::Quit, Step::Hide));
        self.scope.closing = true;
        self.hide_scope(false);
        self.pool.stop_all(false, false);
        self.queue.push(Item::Wind);
    }

    fn end_step(&mut self, end: End, step: Step) {
        if step == Step::Keep {
            return;
        }
        let whole = !self.scope.closing;
        self.scope.end = Some((end, Step::Wait));
        if !self.jobs_done(whole) {
            return;
        }
        self.wipe_and_drop(whole);
        if self.scope.closing {
            // The Demo's page is dropped by the Starter's navigation (§7.9).
            self.scope.end = None;
            self.scope.closing = false;
            self.out
                .push(port::Effect::Order(port::Order::Exit(port::Exit::Reset(
                    self.generation,
                ))));
            self.phase = Phase::Closed;
            self.queue.push(Item::Notice(Part::Outside, Pending::Left));
            return;
        }
        if self.facts.facts.abilities.lock_ends_process {
            let id = self.ask_own(port::Ask::Forget);
            self.scope.forget = Some(id);
            self.scope.end = Some((end, Step::Keep));
        } else {
            self.finish_end(end);
        }
    }

    /// The end's last step: the process is over.
    fn finish_end(&mut self, end: End) {
        self.scope.end = None;
        let exit = match end {
            End::PowerOff | End::Quit => port::Exit::End,
            End::RestartComputer => port::Exit::RestartComputer,
        };
        self.exit(exit);
    }

    /// `Answer::Forgotten`, or any answer to `Forget`: the end goes on, whatever it was (§7.7).
    pub(crate) fn forgotten(&mut self) {
        if let Some((end, Step::Keep)) = self.scope.end {
            self.finish_end(end);
        } else {
            self.fault("a Forget answer with no end waiting");
        }
    }

    fn exit(&mut self, exit: port::Exit) {
        self.phase = Phase::Ended;
        self.out.push(port::Effect::Order(port::Order::Exit(exit)));
    }

    /// Step 5: `Feature::carry` on the settings owner (always) and the scope's top states, in the
    /// fixed order (§7.4).
    fn carry_all(&mut self, carried: &mut Carried) {
        if let Some(state) = self.tops.settings.as_mut() {
            <R::Settings as Feature>::carry(state, carried);
        }
        if let Some(state) = self.tops.session.as_mut() {
            <R::Session as Feature>::carry(state, carried);
        }
        if let Some(state) = self.tops.vaults.as_mut() {
            <R::Vaults as Feature>::carry(state, carried);
        }
        if let Some(state) = self.tops.app.as_mut() {
            <R::App as Feature>::carry(state, carried);
        }
    }

    /// Steps 6 and 7: `AtLock::at_lock` on the top states, then the states, the waiting replies,
    /// the memo, the log, the notes and the frame's output dropped. The part outside the scope is
    /// left alone unless `whole`.
    fn wipe_and_drop(&mut self, whole: bool) {
        let outside = whole || !has_outside::<R>();
        let Built {
            settings,
            session,
            vaults,
            app,
            outside: outside_state,
        } = &mut self.tops;
        if outside && let Some(state) = settings.as_mut() {
            state.at_lock();
        }
        if let Some(state) = session.as_mut() {
            state.at_lock();
        }
        if let Some(state) = vaults.as_mut() {
            state.at_lock();
        }
        if let Some(state) = app.as_mut() {
            state.at_lock();
        }
        if outside && let Some(state) = outside_state.as_mut() {
            state.at_lock();
        }
        debug_assert!(
            settings
                .iter()
                .filter(|_| outside)
                .map(AtLock::holds)
                .chain(session.iter().map(AtLock::holds))
                .chain(vaults.iter().map(AtLock::holds))
                .chain(app.iter().map(AtLock::holds))
                .chain(outside_state.iter().filter(|_| outside).map(AtLock::holds))
                .all(holds_nothing),
            "a state still holds something after at_lock (A12)"
        );
        *session = None;
        *vaults = None;
        *app = None;
        if outside {
            *settings = None;
            *outside_state = None;
        }
        self.replies.drop_scope();
        self.queue.drop_scope();
        self.scope.close = None;
        self.memo_empty();
        self.log.drop_all();
        self.notes.drop_all();
        self.wipe_frame();
        self.motion = MotionStore::new();
        self.running = self.pool.running();
    }

    /// Step 9 in process: `Exit::Reset` for the scope dropped, then the next build from the
    /// carry (§7.6). A rebuild after a failed keep is not clean (R14).
    fn rebuild(&mut self, carry: Option<port::storage::Carry>, started: Started, lost: bool) {
        self.out
            .push(port::Effect::Order(port::Order::Exit(port::Exit::Reset(
                self.generation,
            ))));
        if started == Started::Rebuilt {
            self.facts.tainted = true;
        }
        self.build_scope_as(carry, started);
        if lost {
            self.notes.queue(Note::new(KeepLost), self.clock.now_ms);
            self.frame.wanted = true;
        }
    }

    /// §7.5: the scope built from the carry, in the fixed order, at the next generation.
    pub(crate) fn build_scope(&mut self, carried: Option<port::storage::Carry>) {
        let started = if carried.is_some() {
            Started::Lock
        } else {
            Started::Power
        };
        self.build_scope_as(carried, started);
    }

    /// The scope built from `carried`, as `started` says it began. In the Starter the settings
    /// owner and `Outside` stand outside it and are not built again.
    pub(crate) fn build_scope_as(
        &mut self,
        carried: Option<port::storage::Carry>,
        started: Started,
    ) {
        let mut carried = match carried {
            Some(carry) => self.carry_in(carry),
            None => Carried::empty(),
        };
        self.generation += 1;
        self.replies.new_generation();
        let mode = self.root.mode(&mut carried, &self.facts.facts);
        self.mode = Some(mode);
        self.started = started;
        let defaults = default_settings();
        let fallback = &defaults;

        if !has_outside::<R>() {
            let (state, effects) = {
                let readers = &self.readers;
                start_one::<R::Settings>(
                    &mut carried,
                    Lent {
                        settings: fallback,
                        session: readers,
                        vaults: readers,
                    },
                    env!(self, mode, started),
                )
            };
            self.tops.settings = Some(state);
            self.walk(Part::Settings, effects, Addr::Settings);
        }

        let (state, effects) = {
            let readers = &self.readers;
            let settings = settings_of::<R>(&self.tops.settings, fallback);
            start_one::<R::Session>(
                &mut carried,
                Lent {
                    settings,
                    session: readers,
                    vaults: readers,
                },
                env!(self, mode, started),
            )
        };
        self.tops.session = Some(state);
        self.walk(Part::Session, effects, Addr::Session);

        let (state, effects) = {
            let readers = &self.readers;
            let settings = settings_of::<R>(&self.tops.settings, fallback);
            let session = self
                .tops
                .session
                .as_ref()
                .map(|s| s as &dyn faraday_kernel::session::Read);
            start_one::<R::Vaults>(
                &mut carried,
                Lent {
                    settings,
                    session: session.unwrap_or(readers),
                    vaults: readers,
                },
                env!(self, mode, started),
            )
        };
        self.tops.vaults = Some(state);
        self.walk(Part::Vaults, effects, Addr::Vaults);

        let (state, effects) = {
            let readers = &self.readers;
            let settings = settings_of::<R>(&self.tops.settings, fallback);
            let session = self
                .tops
                .session
                .as_ref()
                .map(|s| s as &dyn faraday_kernel::session::Read);
            let vaults = self
                .tops
                .vaults
                .as_ref()
                .map(|v| v as &dyn faraday_kernel::vaults::Read);
            start_one::<R::App>(
                &mut carried,
                Lent {
                    settings,
                    session: session.unwrap_or(readers),
                    vaults: vaults.unwrap_or(readers),
                },
                env!(self, mode, started),
            )
        };
        self.tops.app = Some(state);
        self.walk(Part::App, effects, Addr::App);

        // `Outside` is built with the scope where it stands inside it: `Nothing`, which holds
        // nothing and is told every notice.
        if !has_outside::<R>() {
            let (state, effects) = {
                let readers = &self.readers;
                let settings = settings_of::<R>(&self.tops.settings, fallback);
                start_one::<R::Outside>(
                    &mut carried,
                    Lent {
                        settings,
                        session: readers,
                        vaults: readers,
                    },
                    env!(self, mode, started),
                )
            };
            self.tops.outside = Some(state);
            self.walk(Part::Outside, effects, Addr::Outside);
        }

        // R22: what no start took is dropped, and wipes as it drops.
        drop(carried);
        self.phase = Phase::Running;
    }

    /// §7.5: at `Start` in the Starter, the part outside the scope is built first, with
    /// generation 0; the phase is `Closed` until `Top::Open`.
    pub(crate) fn build_outside(&mut self) {
        let mut carried = Carried::empty();
        let mode = self.root.mode(&mut carried, &self.facts.facts);
        self.mode = Some(mode);
        self.started = Started::Power;
        let defaults = default_settings();
        let fallback = &defaults;

        let (state, effects) = {
            let readers = &self.readers;
            start_one::<R::Settings>(
                &mut carried,
                Lent {
                    settings: fallback,
                    session: readers,
                    vaults: readers,
                },
                env!(self, mode, Started::Power),
            )
        };
        self.tops.settings = Some(state);
        self.walk(Part::Settings, effects, Addr::Settings);

        let (state, effects) = {
            let readers = &self.readers;
            let settings = settings_of::<R>(&self.tops.settings, fallback);
            start_one::<R::Outside>(
                &mut carried,
                Lent {
                    settings,
                    session: readers,
                    vaults: readers,
                },
                env!(self, mode, Started::Power),
            )
        };
        self.tops.outside = Some(state);
        self.walk(Part::Outside, effects, Addr::Outside);
        drop(carried);
        self.phase = Phase::Closed;
    }

    /// `Event::Start`: once per process, in `Starting` (§2.2); a later one is dropped and noted
    /// (P17). The facts of the process are kept, the self-test is run before the first state is
    /// built, and the model is built (§7.5).
    pub(crate) fn started_by(&mut self, start: port::event::Start) {
        if self.phase != Phase::Starting {
            self.fault("a second Start");
            return;
        }
        let port::event::Start {
            display,
            medium,
            abilities,
            machine,
            time,
            carried,
            root_read_only,
            ..
        } = start;
        let facts = &mut self.facts.facts;
        facts.display = display;
        facts.medium = medium;
        facts.abilities = abilities;
        facts.machine = machine;
        facts.root_read_only = root_read_only;
        self.clock.now_ms = time.now_ms;
        self.clock.unix_secs = time.unix_secs;
        self.run_selftest();
        if has_outside::<R>() {
            self.build_outside();
        } else {
            self.build_scope(carried);
        }
    }

    /// §7.6: `Answer::Kept`, a failure handing the carry back; a wrong answer is a failed keep
    /// whose carry is lost, and a rebuilt scope then puts up `KeepLost`.
    pub(crate) fn kept(&mut self, answer: port::Answer) {
        let Some((_, Step::Keep)) = self.scope.lock else {
            self.fault("a Keep answer with no lock waiting");
            return;
        };
        self.scope.lock = None;
        let only_copy = self.scope.only_copy;
        match answer {
            port::Answer::Kept(Ok(())) => self.exit(port::Exit::Lock),
            port::Answer::Kept(Err((_failure, carry))) if only_copy => {
                self.rebuild(Some(carry), Started::Rebuilt, false);
            }
            port::Answer::Kept(Err(_)) => self.exit(port::Exit::Lock),
            _ if only_copy => {
                self.scope.only_copy = false;
                self.rebuild(None, Started::Rebuilt, true);
            }
            _ => self.exit(port::Exit::Lock),
        }
    }

    /// `Carry` from `Carried` (§7.3): `only_copy` is held aside for the whole Keep.
    pub(crate) fn carry_out(&mut self, carried: Carried) -> port::storage::Carry {
        self.scope.only_copy = carried.only_copy();
        carried.into_carry()
    }

    /// `Carried` from `Carry`, with the `only_copy` held aside given back.
    pub(crate) fn carry_in(&mut self, carry: port::storage::Carry) -> Carried {
        Carried::from_carry(carry, std::mem::take(&mut self.scope.only_copy))
    }
}
