//! The lock order, the carry in hand, the scope's build (05 §7).

use core::borrow::Borrow;

use faraday_crypto::work::Compute;
use faraday_kernel::{
    Carried, Clock, Ctx, Effects, Facts, Feature, Line, Lock, Mode, Public, Running, Show, Started,
    Worded, shared,
};
use faraday_port as port;

use crate::facts::{Lent, build_view, default_settings};
use crate::queue::Part;
use crate::root::{Addr, OwnerState, Root, has_outside};
use crate::runtime::{Phase, Runtime};

/// How far a lock or an end of the scope has got (§7.2, §7.7).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Step {
    Hide,
    Stop,
    Seal,
    Wait,
    Carry,
    Wipe,
    Drop,
    Keep,
    Exit,
}

/// What the runtime holds of the scope across a lock.
pub(crate) struct Scope {
    /// The carry in hand, moved into the next build (§7.6).
    pub(crate) carry: Option<Carried>,
    /// `only_copy`, held aside for the whole Keep (§7.3).
    pub(crate) only_copy: bool,
    /// The lock being run, and the step it is at.
    pub(crate) lock: Option<(Lock, Step)>,
    /// The id of the `Close` change the runtime reads and never routes (§7.2 step 3).
    pub(crate) close: Option<shared::ChangeId>,
    /// The camera stream running, by the id of the ask answered `Started` (§5.5).
    pub(crate) stream: Option<port::AskId>,
}

impl Scope {
    pub(crate) fn new() -> Self {
        Scope {
            carry: None,
            only_copy: false,
            lock: None,
            close: None,
            stream: None,
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
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}

impl<R: Root> Runtime<R> {
    /// §7.2: the nine steps, in order, by every way a lock is asked (R19).
    pub(crate) fn begin_lock(&mut self, _lock: Lock) {
        todo!()
    }

    /// Goes on from the step waiting, once what it waited for has come.
    pub(crate) fn lock_next(&mut self) {
        todo!()
    }

    /// §7.7: the order of an end, with no seal.
    pub(crate) fn begin_end(&mut self, _end: faraday_kernel::End) {
        todo!()
    }

    /// §7.9: the Starter's Close over the scope alone.
    pub(crate) fn close_scope(&mut self) {
        todo!()
    }

    /// §7.5: the scope built from the carry, in the fixed order, at the next generation. In the
    /// Starter the settings owner and `Outside` stand outside it and are not built again.
    pub(crate) fn build_scope(&mut self, carried: Option<port::storage::Carry>) {
        let started = if carried.is_some() {
            Started::Lock
        } else {
            Started::Power
        };
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
    pub(crate) fn kept(&mut self, _answer: port::Answer) {
        todo!()
    }

    /// `Carry` from `Carried`, and back (§7.3).
    pub(crate) fn carry_out(&mut self, _carried: Carried) -> port::storage::Carry {
        todo!()
    }

    pub(crate) fn carry_in(&mut self, _carry: port::storage::Carry) -> Carried {
        todo!()
    }
}
