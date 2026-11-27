//! The runtime's own tests (07 §2.1): `Runtime<R>` over a test root of test features, events in,
//! effects out. The features are one probe, placed in each top state, which logs what it is
//! given. The frame is not drawn: the kit's drawing is not built when these are written, so
//! the tests that read a frame wait for it.

use core::borrow::Borrow;
use core::cell::RefCell;
use core::marker::PhantomData;
use std::sync::Arc;

use faraday_atoms::{Pressed, Ui};
use faraday_bip::keys::Network;
use faraday_kernel::settings;
use faraday_kernel::shared::{self, AnyChange, ChangeId, Done, Empty, Owner};
use faraday_kernel::{
    AtLock, Carried, Ctx, Edits, Effects, Feature, Holds, Job, JobKind, Message, Mode, Notice,
    Show, Top, View, session, vaults,
};
use faraday_port as port;
use faraday_port::event::{Event, Start, Time};
use faraday_port::{Answer, Ask, AskId, Effect, Order, Request};

use crate::facts::{Kept, default_settings};
use crate::root::{Addr, NoRefusal, Nothing, Root, Stream, Tops};
use crate::runtime::{Phase, Runtime, Workers};

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn log(line: String) {
    LOG.with(|log| log.borrow_mut().push(line));
}

fn take_log() -> Vec<String> {
    LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

enum Msg {
    Fan,
    Ask,
    Chain(u32),
    Given(ChangeId, Box<AnyChange>),
    Changed(u32),
    Mem(u32, Option<u32>),
    Typed(bool),
    Look,
    Loop,
    Job,
    Hold,
    Stop,
    Held(u32),
    Share,
    Open,
}

impl Message for Msg {
    fn kind(&self) -> &'static str {
        "test.msg"
    }

    fn public(&self) -> Option<&dyn core::fmt::Debug> {
        None
    }
}

/// A change the test owner is given: it sets Seal amounts.
struct SetSeal(bool);

impl From<SetSeal> for AnyChange {
    fn from(change: SetSeal) -> AnyChange {
        AnyChange::Settings(settings::Change::Set(settings::Set(
            settings::Choice::SealAmounts(change.0),
        )))
    }
}

impl shared::Change for SetSeal {
    type Done = ();

    fn take(done: Done) -> Result<(), Done> {
        match done {
            Done::Settings(settings::Done::Set(())) => Ok(()),
            other => Err(other),
        }
    }

    fn fault() {}
}

/// An ask the test adapter answers with `Answer::Memory`.
struct AskMem;

impl From<AskMem> for Ask {
    fn from(_ask: AskMem) -> Ask {
        Ask::Memory
    }
}

impl Request for AskMem {
    type Answer = Option<u32>;

    fn take(answer: Answer) -> Result<Option<u32>, Answer> {
        match answer {
            Answer::Memory(mib) => Ok(mib),
            other => Err(other),
        }
    }

    /// What a feature is given when the adapter answered with another kind.
    fn fault() -> Option<u32> {
        Some(u32::MAX)
    }
}

trait Place: 'static {
    const NAME: &'static str;
    const INDEX: u32;
}

macro_rules! place {
    ($place:ident, $name:literal, $index:literal) => {
        struct $place;
        impl Place for $place {
            const NAME: &'static str = $name;
            const INDEX: u32 = $index;
        }
    };
}
place!(SettingsAt, "settings", 0);
place!(SessionAt, "session", 1);
place!(VaultsAt, "vaults", 2);
place!(AppAt, "app", 3);
place!(OutsideAt, "outside", 4);

struct Probe<P>(PhantomData<P>);

struct ProbeState<P> {
    typed: bool,
    settings: shared::Settings,
    empty: Empty,
    place: PhantomData<P>,
}

impl<P> AtLock for ProbeState<P> {
    fn holds(&self) -> Holds {
        Holds {
            kept: false,
            typed: self.typed,
            session: false,
            asks_first: None,
        }
    }

    fn hide(&mut self) {}

    fn at_lock(&mut self) {}
}

impl<P> Borrow<shared::Settings> for ProbeState<P> {
    fn borrow(&self) -> &shared::Settings {
        &self.settings
    }
}

/// The session owner holds a key; every other state reads as the empty reader does.
impl<P: Place> session::Read for ProbeState<P> {
    fn keys(&self) -> &faraday_keys::Keys {
        self.empty.keys()
    }

    fn key(&self, id: faraday_kernel::KeyId) -> Option<session::KeyView<'_>> {
        self.empty.key(id)
    }

    fn wallets(&self) -> &[session::WalletView] {
        self.empty.wallets()
    }

    fn wallet(&self, id: faraday_kernel::WalletId) -> Option<&session::WalletView> {
        self.empty.wallet(id)
    }

    fn waiting(&self) -> &[session::WaitingItem] {
        self.empty.waiting()
    }

    fn inbox(&self) -> &[session::FileView] {
        self.empty.inbox()
    }

    fn outbox(&self) -> &[session::FileView] {
        self.empty.outbox()
    }

    fn seeded(&self) -> bool {
        P::NAME == "session"
    }

    fn edits(&self) -> Edits {
        Edits::NONE
    }
}

impl<P: Place> vaults::Read for ProbeState<P> {
    fn open(&self) -> &faraday_keys::vault::OpenVaults {
        vaults::Read::open(&self.empty)
    }

    fn current(&self) -> Option<faraday_keys::vault::VaultId> {
        None
    }

    fn edits(&self) -> Edits {
        Edits::NONE
    }
}

impl<P: Place> Feature for Probe<P> {
    type State = ProbeState<P>;
    type Msg = Msg;
    type Out = Top;
    type Refusal = NoRefusal;

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (ProbeState<P>, Effects<Msg, Top>) {
        log(format!("{}:start", P::NAME));
        (
            ProbeState {
                typed: false,
                settings: default_settings(),
                empty: Empty::new(),
                place: PhantomData,
            },
            Effects::none(),
        )
    }

    fn update(state: &mut ProbeState<P>, msg: Msg, cx: &Ctx<'_>) -> Effects<Msg, Top> {
        let name = P::NAME;
        match msg {
            Msg::Fan => {
                log(format!("{name}:Fan"));
                Effects::none()
                    .then(Msg::Chain(1))
                    .change(SetSeal(true), 7_u32, |context, ()| Msg::Changed(context))
                    .ask(AskMem, 5_u32, Msg::Mem)
            }
            Msg::Ask => Effects::none().ask(AskMem, P::INDEX, Msg::Mem),
            Msg::Chain(n) => {
                log(format!("{name}:Chain({n})"));
                if n < 3 {
                    Effects::none().then(Msg::Chain(n + 1))
                } else {
                    Effects::none()
                }
            }
            Msg::Given(id, _change) => {
                log(format!(
                    "{name}:Given, settings seen seal={}",
                    cx.view.settings.seal_amounts
                ));
                state.settings.seal_amounts = true;
                Effects::none().done(id, Done::Settings(settings::Done::Set(())))
            }
            Msg::Changed(context) => {
                log(format!(
                    "{name}:Changed({context}), settings seen seal={}",
                    cx.view.settings.seal_amounts
                ));
                Effects::none()
            }
            Msg::Mem(context, mib) => {
                log(format!("{name}:Mem({context}, {mib:?})"));
                Effects::none()
            }
            Msg::Typed(typed) => {
                state.typed = typed;
                Effects::none()
            }
            Msg::Look => {
                log(format!(
                    "{name}:Look session seeded={}, display width={}",
                    cx.view.session.seeded(),
                    cx.view.facts.display.width
                ));
                Effects::none()
            }
            Msg::Loop => Effects::none().then(Msg::Loop),
            Msg::Job => Effects::none().job(Job::new(JobKind("test.job"), 0_u8, |_, _| Msg::Fan)),
            Msg::Hold => Effects::none().job(Job::new(JobKind("test.hold"), 0_u8, |_, compute| {
                compute.progress(2, 5);
                let began = std::time::Instant::now();
                while !compute.stop_asked() && began.elapsed().as_secs() < 10 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Msg::Held(u32::from(compute.stop_asked()))
            })),
            Msg::Stop => Effects::none().stop(JobKind("test.hold")),
            Msg::Held(stopped) => {
                log(format!("{name}:Held({stopped})"));
                Effects::none()
            }
            Msg::Share => Effects::none().out(Top::Shared(shared::Changed::Settings(
                settings::Changed::Choices,
            ))),
            Msg::Open => Effects::none().out(Top::Open),
        }
    }

    fn notice(_state: &mut ProbeState<P>, notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Top> {
        let what = match notice {
            Notice::Left => "Left".to_string(),
            Notice::Hide => "Hide".to_string(),
            Notice::Shared(_) => "Shared".to_string(),
            Notice::Facts(facts) => format!("Facts({facts:?})"),
        };
        log(format!("{}:notice {what}", P::NAME));
        Effects::none()
    }

    fn refuses(_state: &ProbeState<P>, _msg: &Msg, _view: &View<'_>) -> Option<NoRefusal> {
        None
    }
}

impl<P: Place> Owner for Probe<P> {
    fn given(id: ChangeId, change: AnyChange) -> Msg {
        Msg::Given(id, Box::new(change))
    }
}

#[derive(Clone, PartialEq)]
struct Press;

impl Pressed for Press {
    fn hides(&self) -> bool {
        false
    }
}

/// A root whose next message is set by the test, and handed to the runtime at `CloseAsked`.
trait Injected: Root {
    fn slot(&self) -> &RefCell<Option<Addr<Self>>>;
}

macro_rules! test_root {
    ($root:ident, $outside:ty) => {
        struct $root {
            next: RefCell<Option<Addr<$root>>>,
        }

        impl $root {
            fn new() -> Self {
                $root {
                    next: RefCell::new(None),
                }
            }
        }

        impl Injected for $root {
            fn slot(&self) -> &RefCell<Option<Addr<$root>>> {
                &self.next
            }
        }

        impl Root for $root {
            type Settings = Probe<SettingsAt>;
            type Session = Probe<SessionAt>;
            type Vaults = Probe<VaultsAt>;
            type App = Probe<AppAt>;
            type Outside = $outside;
            type Press = Press;
            type Options = bool;

            fn mode(&self, _carried: &mut Carried, _facts: &faraday_kernel::Facts) -> Mode {
                Mode {
                    network: Network::Mainnet,
                    practice: false,
                    online: false,
                }
            }

            fn press(_press: Press) -> Addr<Self> {
                Addr::App(Msg::Fan)
            }

            fn input(
                &self,
                _tops: &Tops<'_, Self>,
                _input: &port::event::Input,
            ) -> Option<Addr<Self>> {
                None
            }

            fn fact(&self, _tops: &Tops<'_, Self>, _fact: Stream) -> Option<Addr<Self>> {
                None
            }

            fn close_asked(&self) -> Option<Addr<Self>> {
                self.next.borrow_mut().take()
            }

            fn frame(
                &self,
                _tops: &Tops<'_, Self>,
                _view: &View<'_>,
                _show: &Show<'_>,
                _ui: &mut Ui<'_, Press>,
            ) {
            }
        }
    };
}

test_root!(Device, Nothing);
test_root!(Starter, Probe<OutsideAt>);

struct NoWake;

impl port::Waker for NoWake {
    fn wake(&self) {}
}

fn start_event() -> Event {
    let kept = Kept::new();
    Event::Start(Start {
        display: kept.facts.display,
        medium: kept.facts.medium,
        abilities: kept.facts.abilities,
        machine: kept.facts.machine.clone(),
        time: Time {
            now_ms: 0,
            unix_secs: 0,
        },
        carried: None,
        settings: None,
        scheme: None,
        root_read_only: None,
    })
}

fn started<R: Injected>(root: R) -> Runtime<R> {
    let mut runtime = Runtime::new(root, Arc::new(NoWake), Workers::Inline);
    runtime.frame.skip = true;
    runtime.event(start_event());
    take_log();
    runtime
}

/// One message to a top state: the root hands it over at `CloseAsked`.
fn send<R: Injected>(runtime: &mut Runtime<R>, addr: Addr<R>) -> Vec<Effect> {
    *runtime.root.slot().borrow_mut() = Some(addr);
    runtime.event(Event::CloseAsked)
}

fn asked(effects: &[Effect]) -> Vec<AskId> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Ask(id, Ask::Memory) => Some(*id),
            _ => None,
        })
        .collect()
}

/// Invariant: What one event causes is handled first in first out, in the order each effect was returned, before anything else.
/// Regression: A queue that handled the newest message first, or a change's answer ahead of the `Then` returned before it, runs a cascade in another order.
/// Owner: runtime
#[test]
fn a_cascade_across_a_feature_and_an_owner_is_handled_in_the_order_made() {
    let mut runtime = started(Device::new());
    send(&mut runtime, Addr::App(Msg::Fan));
    assert_eq!(
        take_log(),
        [
            "app:Fan",
            "app:Chain(1)",
            "settings:Given, settings seen seal=false",
            "app:Chain(2)",
            "app:Changed(7), settings seen seal=true",
            "app:Chain(3)",
        ]
    );
}

/// Invariant: An ask goes to the adapter once, and its answer reaches the feature that asked with what it kept; a second answer to it changes nothing.
/// Regression: A reply table that keeps the reply after the answer delivers an adapter's repeated answer twice.
/// Owner: runtime
#[test]
fn an_ask_is_answered_once_and_its_answer_reaches_the_scope_that_asked() {
    let mut runtime = started(Device::new());
    let effects = send(&mut runtime, Addr::App(Msg::Ask));
    let ids = asked(&effects);
    assert_eq!(ids.len(), 1);
    assert_eq!(
        ids[0],
        AskId {
            generation: 1,
            n: 0
        }
    );

    runtime.event(Event::Answer(ids[0], Answer::Memory(Some(9))));
    assert_eq!(take_log(), ["app:Mem(3, Some(9))"]);

    runtime.event(Event::Answer(ids[0], Answer::Memory(Some(9))));
    assert_eq!(take_log(), Vec::<String>::new());
}

/// Invariant: An answer of another kind than the ask's reaches the feature as the ask's fault, and is noted.
/// Regression: Handing the feature the adapter's wrong variant, or dropping the reply, leaves a screen waiting for an answer that cannot come.
/// Owner: runtime
#[test]
fn an_answer_of_the_wrong_kind_reaches_the_feature_as_the_asks_fault() {
    let mut runtime = started(Device::new());
    let effects = send(&mut runtime, Addr::App(Msg::Ask));
    let ids = asked(&effects);
    runtime.event(Event::Answer(ids[0], Answer::Forgotten(Ok(()))));
    assert_eq!(take_log(), [format!("app:Mem(3, Some({}))", u32::MAX)]);
    assert!(!runtime.faults.is_empty());
}

/// Invariant: A change is served inside the event that made it, beside an ask still waiting for its answer, which is delivered when it comes.
/// Regression: Serving a change only after the pending ask is answered, or dropping the ask's reply when the change is served, loses one of them.
/// Owner: runtime
#[test]
fn a_change_is_served_beside_a_pending_ask() {
    let mut runtime = started(Device::new());
    let effects = send(&mut runtime, Addr::App(Msg::Fan));
    let log = take_log();
    assert!(log.contains(&"app:Changed(7), settings seen seal=true".to_string()));
    assert!(!log.iter().any(|line| line.contains("Mem")));
    let ids = asked(&effects);
    assert_eq!(ids.len(), 1);

    runtime.event(Event::Answer(ids[0], Answer::Memory(Some(2))));
    assert_eq!(take_log(), ["app:Mem(5, Some(2))"]);
}

/// Invariant: The settings owner is lent the settings as they were before the message, and every other top state sees what the owner then holds.
/// Regression: Lending the owner its own new state, or the others the old, shows a feature settings that are not the person's.
/// Owner: runtime
#[test]
fn the_settings_owner_reads_the_settings_as_they_were_and_the_others_as_they_are() {
    let mut runtime = started(Device::new());
    send(&mut runtime, Addr::App(Msg::Fan));
    let log = take_log();
    assert!(log.contains(&"settings:Given, settings seen seal=false".to_string()));
    assert!(log.contains(&"app:Changed(7), settings seen seal=true".to_string()));
}

/// Invariant: Another top state reads the session the session owner holds, and the owner's own slot of the view is the empty reader.
/// Regression: Lending the owner a reader of itself while it is borrowed for change, or lending the others the empty one, shows the wrong session.
/// Owner: runtime
#[test]
fn a_top_state_reads_the_session_and_the_session_owner_is_lent_the_empty_reader() {
    let mut runtime = started(Device::new());
    send(&mut runtime, Addr::App(Msg::Look));
    send(&mut runtime, Addr::Session(Msg::Look));
    assert_eq!(
        take_log(),
        [
            "app:Look session seeded=true, display width=0",
            "session:Look session seeded=false, display width=0",
        ]
    );
}

/// Invariant: A change of a fact is told to every top state built, in the order settings, session, vaults, app, and the fact is the process's: a message after it reads the new one.
/// Regression: Telling the top states in another order, or telling only the app, or keeping the fact in the scope, changes what features read.
/// Owner: runtime
#[test]
fn a_changed_fact_is_told_to_every_top_state_in_order() {
    let mut runtime = started(Device::new());
    let mut display = Kept::new().facts.display;
    display.width = 480;
    runtime.event(Event::Display(display));
    assert_eq!(
        take_log(),
        [
            "settings:notice Facts(Display)",
            "session:notice Facts(Display)",
            "vaults:notice Facts(Display)",
            "app:notice Facts(Display)",
        ]
    );
    send(&mut runtime, Addr::App(Msg::Look));
    assert_eq!(
        take_log(),
        ["app:Look session seeded=true, display width=480"]
    );
}

/// Invariant: What an owner says changed reaches every top state built, in the same fixed order.
/// Regression: A broadcast routed only to the app, or to the top states in the order they were built backwards, leaves one of them showing what is gone.
/// Owner: runtime
#[test]
fn a_shared_change_is_broadcast_to_every_top_state() {
    let mut runtime = started(Device::new());
    send(&mut runtime, Addr::Settings(Msg::Share));
    assert_eq!(
        take_log(),
        [
            "settings:notice Shared",
            "session:notice Shared",
            "vaults:notice Shared",
            "app:notice Shared",
        ]
    );
}

/// Invariant: The process is not clean once a top state has held or typed a secret, and stays not clean after it lets go of it.
/// Regression: Working the marker out afresh at each event would announce the process clean again after a lock's wipe.
/// Owner: runtime
#[test]
fn the_process_stays_not_clean_once_a_secret_was_typed() {
    let mut runtime = started(Device::new());
    let effects = send(&mut runtime, Addr::App(Msg::Typed(true)));
    assert!(matches!(
        effects.as_slice(),
        [Effect::Order(Order::Clean(false))]
    ));

    let effects = send(&mut runtime, Addr::App(Msg::Typed(false)));
    assert!(effects.is_empty());
    assert!(!runtime.facts.facts.clean);
}

/// Invariant: A second `Start` in a process is dropped and builds nothing.
/// Regression: Building the model again at a second `Start` drops what the person has open.
/// Owner: runtime
#[test]
fn a_second_start_is_dropped() {
    let mut runtime = started(Device::new());
    let effects = runtime.event(start_event());
    assert!(effects.is_empty());
    assert_eq!(take_log(), Vec::<String>::new());
    assert_eq!(runtime.phase(), Phase::Running);
}

/// Invariant: A feature that sends itself a message for ever does not stop the screen: the cascade is cut and the next event is handled.
/// Regression: With no bound on a cascade, the first message to send itself again hangs the process.
/// Owner: runtime
#[test]
fn a_cascade_with_no_end_is_cut_and_the_next_event_is_handled() {
    let mut runtime = started(Device::new());
    send(&mut runtime, Addr::App(Msg::Loop));
    assert!(!runtime.faults.is_empty());
    let effects = send(&mut runtime, Addr::App(Msg::Ask));
    assert_eq!(asked(&effects).len(), 1);
}

/// Invariant: What the Starter's own part asks for carries generation 0 and is never stale at a lock, and what the scope asks for carries the scope's generation.
/// Regression: Numbering the outside part's asks by the scope's generation drops its answers at the first lock.
/// Owner: runtime
#[test]
fn the_part_outside_the_scope_asks_with_generation_zero() {
    let mut runtime = started(Starter::new());
    assert_eq!(runtime.phase(), Phase::Closed);
    let outside = asked(&send(&mut runtime, Addr::Outside(Msg::Ask)));
    assert_eq!(
        outside,
        [AskId {
            generation: 0,
            n: 0
        }]
    );

    send(&mut runtime, Addr::Outside(Msg::Open));
    assert_eq!(runtime.phase(), Phase::Running);
    let inside = asked(&send(&mut runtime, Addr::App(Msg::Ask)));
    assert_eq!(
        inside,
        [AskId {
            generation: 1,
            n: 0
        }]
    );
}

/// Invariant: While the scope is locking, its asks are still carried and a job it starts that is not a seal is dropped unstarted, and an answer is delivered only to the part outside the scope.
/// Regression: Delivering a scope's answer after the hide puts a secret back on the screen; dropping the outside part's answer loses what the Starter asked.
/// Owner: runtime
#[test]
fn a_scope_that_is_locking_gets_only_what_serves_the_lock() {
    let mut runtime = started(Starter::new());
    let outside = asked(&send(&mut runtime, Addr::Outside(Msg::Ask)));
    send(&mut runtime, Addr::Outside(Msg::Open));
    let inside = asked(&send(&mut runtime, Addr::App(Msg::Ask)));
    take_log();

    runtime.phase = Phase::Locking;
    // The scope's ask is still carried to the adapter.
    let more = asked(&send(&mut runtime, Addr::App(Msg::Ask)));
    assert_eq!(more.len(), 1);
    // Its job is dropped before the pool sees it.
    send(&mut runtime, Addr::App(Msg::Job));

    runtime.event(Event::Answer(inside[0], Answer::Memory(Some(1))));
    assert_eq!(take_log(), Vec::<String>::new());
    runtime.event(Event::Answer(outside[0], Answer::Memory(Some(2))));
    assert_eq!(take_log(), ["outside:Mem(4, Some(2))"]);
}

/// Counts the wakes the lanes make.
struct Counting(std::sync::atomic::AtomicU32);

impl port::Waker for Counting {
    fn wake(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Release);
    }
}

fn started_on_threads() -> Runtime<Device> {
    let mut runtime = Runtime::new(
        Device::new(),
        Arc::new(Counting(Default::default())),
        Workers::Threads,
    );
    runtime.frame.skip = true;
    runtime.event(start_event());
    take_log();
    runtime
}

/// Waits for the lanes, handling what they answer, until `done` holds or five seconds pass.
fn woken_until(runtime: &mut Runtime<Device>, done: impl Fn(&Runtime<Device>) -> bool) {
    let began = std::time::Instant::now();
    while !done(runtime) && began.elapsed().as_secs() < 5 {
        runtime.woken();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

/// Invariant: A job's answer reaches the top state that started it as the next event, the same on the root's thread and on a lane.
/// Regression: An inline arm that answers in another order than the lanes, or a lane whose answer goes to no one, makes a tour render other than the device runs.
/// Owner: runtime
#[test]
fn a_job_answers_the_scope_that_started_it_inline_and_on_a_lane() {
    let mut inline = started(Device::new());
    send(&mut inline, Addr::App(Msg::Job));
    let on_inline = take_log();
    assert_eq!(on_inline.first().map(String::as_str), Some("app:Fan"));

    let mut threads = started_on_threads();
    send(&mut threads, Addr::App(Msg::Job));
    woken_until(&mut threads, |_| LOG.with(|log| !log.borrow().is_empty()));
    assert_eq!(take_log(), on_inline);
}

/// Invariant: A job's answer of a generation that is neither 0 nor the built scope's is dropped unread, and what it carries wipes as it drops.
/// Regression: A runtime that delivers a stale scope's answer hands a locked scope's job output to the scope built after it (R15).
/// Owner: runtime
#[test]
fn a_job_answer_of_another_scope_is_dropped() {
    use crate::pool::{Answered, JobId};
    use crate::queue::Part;
    let mut runtime = started(Device::new());
    let generation = runtime.generation;
    let answer = |generation: u32| Answered {
        id: JobId { generation, n: 9 },
        part: Part::App,
        seal: false,
        message: Box::new(Addr::<Device>::App(Msg::Fan)),
    };
    runtime.settle_with(|runtime| runtime.job_answered(answer(generation + 4)));
    assert_eq!(take_log(), Vec::<String>::new());
    runtime.settle_with(|runtime| runtime.job_answered(answer(generation)));
    assert_eq!(take_log().first().map(String::as_str), Some("app:Fan"));
}

/// Invariant: A job asked to stop sees the stop, and an answer it returns is handled as any answer; a job not yet started is dropped and answers nothing.
/// Regression: A stop that sets no flag leaves a long job running to its end; one that leaves the queued job in place answers a screen that asked for nothing.
/// Owner: runtime
#[test]
fn stopping_a_job_stops_it_and_a_queued_one_never_answers() {
    let mut runtime = started_on_threads();
    send(&mut runtime, Addr::App(Msg::Hold));
    send(&mut runtime, Addr::App(Msg::Hold));
    woken_until(&mut runtime, |runtime| {
        runtime.pool.running().iter().any(|r| r.done == 2)
    });
    let running = runtime.pool.running();
    assert_eq!((running.len(), running[0].done, running[0].of), (1, 2, 5));
    send(&mut runtime, Addr::App(Msg::Stop));
    woken_until(&mut runtime, |_| LOG.with(|log| !log.borrow().is_empty()));
    assert_eq!(take_log(), ["app:Held(1)"]);
    woken_until(&mut runtime, |_| false);
    assert_eq!(take_log(), Vec::<String>::new());
    assert!(runtime.pool.running().is_empty());
}

/// Invariant: A camera fact of a stream that is not the running one never reaches the root; the running stream's does until `CameraOff`.
/// Regression: A frame of a stopped stream reaching a feature shows a camera the person turned off, and its bytes outlive it.
/// Owner: runtime
#[test]
fn a_camera_fact_of_another_stream_is_dropped() {
    use port::event::CameraFact;
    let mut runtime = started(Device::new());
    let running = AskId {
        generation: 1,
        n: 3,
    };
    runtime.scope.stream = Some(running);
    let ended = |stream| CameraFact::Ended {
        stream,
        why: port::camera::Failure::Unavailable,
    };
    assert!(!runtime.stream_check(&ended(AskId {
        generation: 1,
        n: 4
    })));
    assert!(runtime.stream_check(&ended(running)));
    assert!(!runtime.stream_check(&ended(running)));
}

thread_local! {
    static FILLS: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
}

fn fill_plus_one(input: u32) -> u32 {
    FILLS.with(|fills| fills.set(fills.get() + 1));
    input + 1
}

/// Invariant: A memo entry is kept while a frame uses it, dropped at the end of the first frame that does not, and every entry is dropped by `clear`.
/// Regression: A memo that never forgets keeps a value of a screen long gone; one that forgets at once refills every frame.
/// Owner: runtime
#[test]
fn a_memo_entry_lives_one_frame_unless_used_and_clear_empties_it() {
    use faraday_kernel::Slot;
    let mut runtime = started(Device::new());
    let slot = Slot("test.memo");
    let get =
        |runtime: &Runtime<Device>, input| *runtime.memo.get(slot, 1_u32, input, fill_plus_one);
    assert_eq!(get(&runtime, 1), 2);
    runtime.memo_frame_end();
    assert_eq!(get(&runtime, 1), 2);
    runtime.memo_frame_end();
    runtime.memo_frame_end();
    assert_eq!(get(&runtime, 10), 11);
    runtime.memo_empty();
    assert_eq!(get(&runtime, 20), 21);
}
