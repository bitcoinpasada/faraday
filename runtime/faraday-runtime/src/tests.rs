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
    AtLock, Carried, CarryName, CarryValue, Ctx, Edits, Effects, End, Feature, Holds, Job, JobKind,
    Lock, Message, Mode, Notice, Public, Show, Top, View, session, vaults,
};
use faraday_port as port;
use faraday_port::event::{Event, Start, Time};
use faraday_port::{Answer, Ask, AskId, Effect, Order, Request};

use crate::facts::Kept;
use crate::root::{Addr, NoRefusal, Nothing, Root, Stream, Tops};
use crate::runtime::{Phase, Runtime, Workers};

/// What a test root draws as its frame (none: nothing).
type Draw = fn(&View<'_>, &mut Ui<'_, Press>);

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static DRAW: core::cell::Cell<Option<Draw>> = const { core::cell::Cell::new(None) };
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
    Lock(Lock),
    End(End),
    /// A carry that holds an only copy, as the session puts (§7.6).
    Hold1,
}

impl Message for Msg {
    fn kind(&self) -> &'static str {
        "test.msg"
    }

    fn public(&self) -> Option<&dyn core::fmt::Debug> {
        match self {
            Msg::Look => Some(&"look"),
            _ => None,
        }
    }
}

/// What the app carries across a lock.
#[derive(Public, Copy, PartialEq, Eq)]
struct Token(u8);

impl CarryValue for Token {
    fn encode(&self) -> faraday_port::bytes::Bytes {
        faraday_port::bytes::Bytes::new(vec![self.0])
    }

    fn decode(bytes: &[u8]) -> Option<Token> {
        match bytes {
            [token] => Some(Token(*token)),
            _ => None,
        }
    }
}

const TOKEN: CarryName = CarryName::new("test.token", false, false);

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
    /// Whether this state marks its carry as holding an only copy.
    only_copy: bool,
    settings: shared::Settings,
    empty: Empty,
    place: PhantomData<P>,
}

impl<P: Place> AtLock for ProbeState<P> {
    fn holds(&self) -> Holds {
        Holds {
            kept: false,
            typed: self.typed,
            session: false,
            asks_first: None,
        }
    }

    fn hide(&mut self) {
        log(format!("{}:hide", P::NAME));
    }

    fn at_lock(&mut self) {
        log(format!("{}:at_lock", P::NAME));
    }
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

    fn start(carried: &mut Carried, _cx: &Ctx<'_>) -> (ProbeState<P>, Effects<Msg, Top>) {
        log(format!("{}:start", P::NAME));
        if P::NAME == "app"
            && let Some(Token(token)) = carried.take::<Token>(TOKEN)
        {
            log(format!("app:carried {token}"));
        }
        (
            ProbeState {
                typed: false,
                only_copy: false,
                settings: shared::Settings::default(),
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
            Msg::Lock(lock) => Effects::none().out(Top::Lock(lock)),
            Msg::End(end) => Effects::none().out(Top::End(end)),
            Msg::Hold1 => {
                state.only_copy = true;
                Effects::none()
            }
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

    fn carry(state: &mut ProbeState<P>, into: &mut Carried) {
        log(format!("{}:carry", P::NAME));
        if P::NAME == "app" {
            into.put(TOKEN, &Token(7));
            if state.only_copy {
                into.mark_only_copy();
            }
        }
    }
}

impl<P: Place> Owner for Probe<P> {
    fn given(id: ChangeId, change: AnyChange) -> Msg {
        Msg::Given(id, Box::new(change))
    }
}

#[derive(Clone, PartialEq)]
struct Press(bool);

impl Pressed for Press {
    fn hides(&self) -> bool {
        self.0
    }
}

/// A root whose next message is set by the test, and handed to the runtime at `CloseAsked`.
trait Injected: Root {
    fn slot(&self) -> &RefCell<Option<Addr<Self>>>;
}

macro_rules! test_root {
    ($root:ident, $outside:ty, $has_outside:expr) => {
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
            const OUTSIDE: bool = $has_outside;

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
                view: &View<'_>,
                _show: &Show<'_>,
                ui: &mut Ui<'_, Press>,
            ) {
                if let Some(draw) = DRAW.get() {
                    draw(view, ui);
                }
            }
        }
    };
}

test_root!(Device, Nothing, false);
test_root!(Starter, Probe<OutsideAt>, true);

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

/// Invariant: A process that stays clean sends `Order::Clean(true)` once at `Start`, and nothing for it after.
/// Regression: Sending the marker only on a change leaves the grant helper waiting for ever on a process that never held a secret.
/// Owner: runtime
#[test]
fn a_process_that_stays_clean_says_so_once_at_start() {
    let mut runtime = Runtime::new(Device::new(), Arc::new(NoWake), Workers::Inline);
    runtime.frame.skip = true;
    let at_start = runtime.event(start_event());
    let cleans = |effects: &[Effect]| {
        effects
            .iter()
            .filter(|e| matches!(e, Effect::Order(Order::Clean(true))))
            .count()
    };
    assert_eq!(cleans(&at_start), 1);
    let later = send(&mut runtime, Addr::App(Msg::Look));
    assert!(
        later
            .iter()
            .all(|e| !matches!(e, Effect::Order(Order::Clean(_))))
    );
}

/// Invariant: A job that reports no progress is in `View::running` at the next frame.
/// Regression: Reading the pool only when a lane wakes the root hides the busy line of a job that never reports.
/// Owner: runtime
#[test]
fn a_job_with_no_progress_is_running_at_the_next_frame() {
    let mut runtime = started_on_threads();
    assert!(runtime.running.is_empty());
    send(&mut runtime, Addr::App(Msg::Job));
    assert_eq!(runtime.running.len(), 1);
    assert_eq!(runtime.running[0].kind, JobKind("test.job"));
    woken_until(&mut runtime, |runtime| runtime.pool.running().is_empty());
}

/// Invariant: A change of the downloads or of the Starter's system facts is told to every top state built.
/// Regression: Keeping the fact and telling no one leaves the Starter's pages showing the list as it was.
/// Owner: runtime
#[test]
fn downloads_and_system_facts_are_told_to_every_top_state() {
    use port::event::{HostFact, SystemFact};
    let mut runtime = started(Device::new());
    runtime.event(Event::Host(HostFact::Downloads {
        dir: port::Text::of("d"),
        files: Vec::new(),
    }));
    assert_eq!(
        take_log().last().map(String::as_str),
        Some("app:notice Facts(Downloads)")
    );
    runtime.event(Event::System(SystemFact::NoWriter));
    assert_eq!(
        take_log().last().map(String::as_str),
        Some("app:notice Facts(System)")
    );
}

/// Invariant: A press of a hiding variant records `Mark::Hides` for its event, and the next event starts with no marks.
/// Regression: A runtime that never asks `Pressed::hides` leaves the screens check blind to the presses that hide a secret.
/// Owner: runtime
#[test]
fn a_hiding_press_records_the_hides_mark() {
    use port::event::{Input, Key};
    let mut runtime = started(Device::new());
    runtime.frame.pointer.focused = Some(Press(true));
    runtime.event(Event::Input(Input::Key(Key::Enter)));
    assert_eq!(runtime.seen().marks, [crate::Mark::Hides]);
    runtime.frame.pointer.focused = Some(Press(false));
    runtime.event(Event::Input(Input::Key(Key::Enter)));
    assert!(runtime.seen().marks.is_empty());
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

fn exit_of(effects: &[Effect]) -> Option<&port::Exit> {
    effects.iter().find_map(|effect| match effect {
        Effect::Order(Order::Exit(exit)) => Some(exit),
        _ => None,
    })
}

fn keep_asked(effects: Vec<Effect>) -> Option<(AskId, port::storage::Carry)> {
    effects.into_iter().find_map(|effect| match effect {
        Effect::Ask(id, Ask::Keep(carry)) => Some((id, carry)),
        _ => None,
    })
}

fn device_that_ends_its_process() -> Runtime<Device> {
    let mut runtime = started(Device::new());
    runtime.facts.facts.abilities.lock_ends_process = true;
    runtime
}

/// Invariant: A lock runs hide, carry and wipe in that order, asks the adapter to keep the carry, and the process ends with the lock's exit only once the keep is answered; the next process starts from what was carried.
/// Regression: A lock that wipes before it carries loses what the next scope needs; one that exits before the keep is answered loses the carry; one whose carry is not read at the next start rebuilds an empty scope.
/// Owner: runtime
#[test]
fn a_lock_runs_its_order_and_the_process_ends_with_the_lock_after_the_keep() {
    let mut runtime = device_that_ends_its_process();
    let lock = Lock {
        seal: true,
        idle: false,
    };
    let effects = send(&mut runtime, Addr::App(Msg::Lock(lock)));
    let log = take_log();
    let at = |what: &str| log.iter().position(|line| line == what);
    assert!(at("session:hide") < at("vaults:Given, settings seen seal=false"));
    assert!(at("vaults:Given, settings seen seal=false") < at("settings:carry"));
    assert!(at("app:carry") < at("settings:at_lock"));
    assert_eq!(at("outside:carry"), None);
    assert_eq!(runtime.phase(), Phase::Locking);
    assert!(exit_of(&effects).is_none());
    let (id, carry) = keep_asked(effects).expect("the keep is asked");
    assert_eq!(carry.kept.len(), 1);
    assert!(runtime.tops.app.is_none() && runtime.tops.session.is_none());

    let effects = runtime.event(Event::Answer(id, Answer::Kept(Ok(()))));
    assert!(matches!(exit_of(&effects), Some(port::Exit::Lock)));
    assert_eq!(runtime.phase(), Phase::Ended);
    assert!(runtime.event(Event::CloseAsked).is_empty());

    // The next process: its `Start` brings the carry.
    let mut next = Runtime::new(Device::new(), Arc::new(NoWake), Workers::Inline);
    next.frame.skip = true;
    let Event::Start(mut start) = start_event() else {
        return;
    };
    start.carried = Some(carry);
    take_log();
    next.event(Event::Start(start));
    assert!(take_log().contains(&"app:carried 7".to_string()));
    assert_eq!(next.started, faraday_kernel::Started::Lock);
}

/// Invariant: A lock that does not end the process drops the scope, tells the adapters with `Exit::Reset` of the dropped generation before the next scope's first effect, and builds the next scope from the carry in hand with no keep.
/// Regression: An in-process lock that asks `Keep` hands the Inbox to an adapter for nothing; one that sends `Reset` after the next scope's asks lets an adapter drop the new scope's asks.
/// Owner: runtime
#[test]
fn an_in_process_lock_resets_the_adapters_and_rebuilds_from_the_carry() {
    let mut runtime = started(Device::new());
    let before = runtime.generation;
    let effects = send(
        &mut runtime,
        Addr::App(Msg::Lock(Lock {
            seal: false,
            idle: true,
        })),
    );
    assert!(matches!(
        exit_of(&effects),
        Some(port::Exit::Reset(generation)) if *generation == before
    ));
    assert!(keep_asked(effects).is_none());
    assert_eq!(runtime.phase(), Phase::Running);
    assert_eq!(runtime.generation, before + 1);
    assert_eq!(runtime.started, faraday_kernel::Started::Lock);
    assert!(take_log().contains(&"app:carried 7".to_string()));
}

/// Invariant: A lock with a job running asks it to stop and waits for it; the job's answer is delivered to no top state and the lock then finishes.
/// Regression: A lock that finishes before the job returns leaves a signing job's copy of the keys live; one that delivers the answer puts a stopped job's result on a screen the lock ended.
/// Owner: runtime
#[test]
fn a_lock_waits_for_a_running_job_and_delivers_no_answer_after_it() {
    let mut runtime = started_on_threads();
    send(&mut runtime, Addr::App(Msg::Hold));
    woken_until(&mut runtime, |runtime| {
        runtime.pool.running().iter().any(|r| r.done == 2)
    });
    send(
        &mut runtime,
        Addr::App(Msg::Lock(Lock {
            seal: true,
            idle: false,
        })),
    );
    // The job was asked to stop at once; the lock is done when it has returned.
    woken_until(&mut runtime, |runtime| runtime.phase() == Phase::Running);
    assert_eq!(runtime.phase(), Phase::Running);
    assert_eq!(runtime.generation, 2);
    assert!(!take_log().iter().any(|line| line.contains("Held")));
}

/// Invariant: A camera stream running at a lock is turned off by the first step, and no fact of it reaches the root afterwards.
/// Regression: A frame of the camera that arrives after the hide puts a code on screen the lock took off.
/// Owner: runtime
#[test]
fn no_camera_frame_comes_after_the_lock_turns_the_camera_off() {
    let mut runtime = started(Device::new());
    let stream = AskId {
        generation: 1,
        n: 3,
    };
    runtime.scope.stream = Some(stream);
    let effects = send(
        &mut runtime,
        Addr::App(Msg::Lock(Lock {
            seal: false,
            idle: false,
        })),
    );
    let off = effects
        .iter()
        .position(|effect| matches!(effect, Effect::Order(Order::CameraOff)));
    let reset = effects
        .iter()
        .position(|effect| matches!(effect, Effect::Order(Order::Exit(_))));
    assert!(off.is_some() && off < reset);
    let ended = port::event::CameraFact::Ended {
        stream,
        why: port::camera::Failure::Unavailable,
    };
    assert!(!runtime.stream_check(&ended));
}

/// Invariant: An end seals nothing and keeps nothing: it hides, wipes, asks the adapter to forget what it holds where the process ends, and exits with the end asked for; where nothing is held it exits at once.
/// Regression: An end that seals writes a vault the person chose to power off without; one that exits before the forget leaves a carry file in the RAM directory.
/// Owner: runtime
#[test]
fn an_end_runs_without_a_seal_and_forgets_the_carry_before_it_exits() {
    let mut runtime = device_that_ends_its_process();
    let effects = send(&mut runtime, Addr::App(Msg::End(End::PowerOff)));
    let log = take_log();
    assert!(log.contains(&"app:at_lock".to_string()));
    assert!(!log.iter().any(|line| line.starts_with("vaults:Given")));
    assert!(!log.iter().any(|line| line.ends_with(":carry")));
    assert!(exit_of(&effects).is_none());
    let forget = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Ask(id, Ask::Forget) => Some(*id),
            _ => None,
        })
        .expect("the forget is asked");
    let effects = runtime.event(Event::Answer(forget, Answer::Forgotten(Ok(()))));
    assert!(matches!(exit_of(&effects), Some(port::Exit::End)));
    assert_eq!(runtime.phase(), Phase::Ended);

    let mut restart = started(Device::new());
    let effects = send(&mut restart, Addr::App(Msg::End(End::RestartComputer)));
    assert!(matches!(
        exit_of(&effects),
        Some(port::Exit::RestartComputer)
    ));
}

/// Invariant: When the keep fails the process ends, unless the carry holds an only copy: then the scope is built again in the process from the carry handed back, marked rebuilt; an answer that is not a keep's loses the carry and the rebuilt scope puts up a note.
/// Regression: A failed keep that always ends the process loses a key waiting for its backup; one that always rebuilds keeps a process that cannot write a stick.
/// Owner: runtime
#[test]
fn a_failed_keep_ends_the_process_unless_an_only_copy_is_held() {
    let fails = |runtime: &mut Runtime<Device>| {
        let effects = send(
            runtime,
            Addr::App(Msg::Lock(Lock {
                seal: true,
                idle: false,
            })),
        );
        let (id, carry) = keep_asked(effects).expect("the keep is asked");
        runtime.event(Event::Answer(
            id,
            Answer::Kept(Err((port::Failure::Adapter, carry))),
        ))
    };
    let mut plain = device_that_ends_its_process();
    let effects = fails(&mut plain);
    assert!(matches!(exit_of(&effects), Some(port::Exit::Lock)));

    let mut held = device_that_ends_its_process();
    send(&mut held, Addr::App(Msg::Hold1));
    take_log();
    let effects = fails(&mut held);
    assert!(matches!(exit_of(&effects), Some(port::Exit::Reset(1))));
    assert_eq!(held.phase(), Phase::Running);
    assert_eq!(held.started, faraday_kernel::Started::Rebuilt);
    assert!(take_log().contains(&"app:carried 7".to_string()));

    let mut lost = device_that_ends_its_process();
    send(&mut lost, Addr::App(Msg::Hold1));
    let effects = send(
        &mut lost,
        Addr::App(Msg::Lock(Lock {
            seal: true,
            idle: false,
        })),
    );
    let (id, _carry) = keep_asked(effects).expect("the keep is asked");
    lost.event(Event::Answer(id, Answer::Memory(None)));
    assert_eq!(lost.started, faraday_kernel::Started::Rebuilt);
    assert!(lost.notes.up().is_some());
    assert!(!take_log().contains(&"app:carried 7".to_string()));
}

/// Invariant: The Starter's Quit from inside the scope closes the scope alone: no seal, no forget, `Exit::Reset`, the phase Closed, and the part outside the scope is told it was left and keeps its state.
/// Regression: A Close that ends the process takes the Starter down with its Demo; one that wipes the Starter's own part loses its page.
/// Owner: runtime
#[test]
fn the_starters_close_drops_the_scope_and_leaves_the_part_outside() {
    let mut runtime = started(Starter::new());
    send(&mut runtime, Addr::Outside(Msg::Open));
    take_log();
    let effects = send(&mut runtime, Addr::App(Msg::End(End::Quit)));
    let log = take_log();
    assert!(matches!(exit_of(&effects), Some(port::Exit::Reset(1))));
    assert_eq!(runtime.phase(), Phase::Closed);
    assert!(runtime.tops.app.is_none());
    assert!(runtime.tops.outside.is_some() && runtime.tops.settings.is_some());
    assert!(log.contains(&"outside:notice Left".to_string()));
    assert!(!log.contains(&"outside:at_lock".to_string()));
    assert!(!log.iter().any(|line| line.starts_with("vaults:Given")));
}

/// Invariant: A lock and an end empty the memo, and the log is dropped with them; the log keeps the kind of every message and the message itself only where it is public, in a ring of 512 lines.
/// Regression: A memo that outlives a lock hands a value of the old scope to the next; a log that keeps a withheld message keeps a secret; an unbounded log grows for as long as the verbose run lasts.
/// Owner: runtime
#[test]
fn the_memo_is_empty_after_a_lock_and_an_end_and_the_log_holds_only_public_messages() {
    use faraday_kernel::Slot;
    let slot = Slot("test.memo");
    // The same key with another input: an entry still kept answers with its old value (and a
    // debug build refuses it), an empty memo fills from the input.
    let get = |runtime: &Runtime<Device>, input: u32| {
        *runtime.memo.get(slot, 1_u32, input, fill_plus_one)
    };

    let mut runtime = started(Device::new());
    runtime.log(true);
    send(&mut runtime, Addr::App(Msg::Look));
    send(&mut runtime, Addr::App(Msg::Fan));
    let lines: Vec<(String, Option<String>)> = runtime
        .log_lines()
        .map(|line| (line.kind.clone(), line.public.clone()))
        .collect();
    assert!(lines.contains(&("test.msg".to_string(), Some("\"look\"".to_string()))));
    assert!(lines.contains(&("test.msg".to_string(), None)));
    for _ in 0..600 {
        runtime.log.push(crate::LogLine {
            time_ms: 0,
            generation: 1,
            kind: String::new(),
            public: None,
        });
    }
    assert_eq!(runtime.log_lines().count(), 512);

    assert_eq!(get(&runtime, 1), 2);
    send(
        &mut runtime,
        Addr::App(Msg::Lock(Lock {
            seal: false,
            idle: false,
        })),
    );
    assert!(runtime.log_lines().all(|line| line.generation == 2));
    assert_eq!(get(&runtime, 5), 6);

    runtime.log(true);
    send(&mut runtime, Addr::App(Msg::Look));
    send(&mut runtime, Addr::App(Msg::End(End::PowerOff)));
    assert_eq!(runtime.log_lines().count(), 0);
    assert_eq!(get(&runtime, 9), 10);
}

// What a person sees of the frame: a scrolled region's bar, a hold, a sheet's frost. The root
// draws what the test sets (`DRAW`) on a display of 320 by 480.

const PAGE: faraday_atoms::Anchor = faraday_atoms::Anchor {
    scope: "test",
    n: 1,
};
const SHEET: faraday_atoms::Anchor = faraday_atoms::Anchor {
    scope: "test",
    n: 2,
};
const REGION: faraday_atoms::Anchor = faraday_atoms::Anchor {
    scope: "test",
    n: 3,
};
const HOLD: faraday_atoms::Anchor = faraday_atoms::Anchor {
    scope: "test",
    n: 4,
};

fn draw_region(_view: &View<'_>, ui: &mut Ui<'_, Press>) {
    ui.set_page(PAGE);
    let view = faraday_draw::geom::Rect::new(0, 0, 200, 200);
    ui.region(faraday_atoms::region::Region::new(REGION, view), |_| {
        400.0_f32
    });
}

fn draw_hold(_view: &View<'_>, ui: &mut Ui<'_, Press>) {
    ui.set_page(PAGE);
    let model = faraday_atoms::hold::HoldModel {
        label: faraday_atoms::Shown::plain("Erase"),
        tone: faraday_atoms::hold::HoldTone::Error,
        id: HOLD,
        offered: faraday_atoms::Offered::Yes(Press(false)),
    };
    faraday_atoms::hold::draw(&model, ui, (10.0, 10.0, 200.0));
}

/// A sheet over a page, in the scope as it was built at power on only.
fn draw_sheet(view: &View<'_>, ui: &mut Ui<'_, Press>) {
    ui.set_page(PAGE);
    if view.started == faraday_kernel::Started::Power {
        ui.backdrop(SHEET);
        ui.add_sheet(SHEET);
        ui.layer += 1;
    }
}

/// The same over a page that shows a secret.
fn draw_sheet_over_secret(view: &View<'_>, ui: &mut Ui<'_, Press>) {
    ui.showing(&faraday_atoms::Shown::secret("abandon"), |_| {});
    draw_sheet(view, ui);
}

fn drawing(draw: Draw) -> Runtime<Device> {
    DRAW.set(Some(draw));
    let mut runtime = Runtime::new(Device::new(), Arc::new(NoWake), Workers::Inline);
    let Event::Start(mut start) = start_event() else {
        panic!("a start");
    };
    start.display.width = 320;
    start.display.height = 480;
    runtime.event(Event::Start(start));
    take_log();
    runtime
}

/// The clock at `now_ms`, and a frame drawn at it.
fn at(runtime: &mut Runtime<Device>, now_ms: u64) {
    runtime.frame.wanted = true;
    runtime.event(Event::Time(Time {
        now_ms,
        unix_secs: now_ms / 1000,
    }));
}

fn touch(runtime: &mut Runtime<Device>, at: port::values::Point, phase: port::event::TouchPhase) {
    runtime.event(Event::Input(port::event::Input::Touch { at, phase }));
}

fn pixels(runtime: &mut Runtime<Device>) -> Vec<u8> {
    runtime
        .frame()
        .map(|frame| frame.rgba.to_vec())
        .unwrap_or_default()
}

fn presses() -> usize {
    take_log().iter().filter(|line| *line == "app:Fan").count()
}

/// Invariant: A scrolled region shows its scrollbar once its content has moved, keeps asking frames while it shows, and draws it no more after it has faded.
/// Regression: Nothing writes the region's offset and the time it moved, so the overlay bar never shows; or the bar is drawn for ever and the screen never rests.
/// Owner: runtime
#[test]
fn a_scrolled_region_shows_its_bar_after_a_scroll_and_it_fades() {
    use port::values::Pace;
    let mut runtime = drawing(draw_region);
    at(&mut runtime, 1_000);
    let before = pixels(&mut runtime);
    assert_eq!(runtime.frame.pace(), Pace::Rest);
    runtime.event(Event::Input(port::event::Input::Scroll {
        at: port::values::Point { x: 50, y: 50 },
        dy: 40,
    }));
    let shown = pixels(&mut runtime);
    assert_ne!(before, shown, "the scroll draws");
    assert_eq!(runtime.frame.pace(), Pace::Moving);
    at(&mut runtime, 11_000);
    let gone = pixels(&mut runtime);
    assert_ne!(shown, gone, "the bar is gone");
    assert_eq!(runtime.frame.pace(), Pace::Rest);
    at(&mut runtime, 12_000);
    assert_eq!(
        pixels(&mut runtime),
        gone,
        "the page at rest does not change"
    );
}

/// Invariant: A hold sends its press once when it is held to its end, sends nothing when the contact lifts before, and sends nothing more when it lifts after.
/// Regression: A hold that sends on lift fires when let go early; one sent at its end and again at the lift fires twice.
/// Owner: runtime
#[test]
fn a_hold_sends_its_press_once_when_held_to_its_end_and_never_when_let_go_early() {
    use port::event::TouchPhase::{Down, Up};
    let mut runtime = drawing(draw_hold);
    at(&mut runtime, 1_000);
    let rect = runtime.hits()[0].rect;
    let on = port::values::Point {
        x: rect.x + rect.w / 2,
        y: rect.y + rect.h / 2,
    };
    take_log();
    touch(&mut runtime, on, Down);
    at(&mut runtime, 1_050);
    touch(&mut runtime, on, Up);
    at(&mut runtime, 20_000);
    assert_eq!(presses(), 0, "let go early");

    touch(&mut runtime, on, Down);
    at(&mut runtime, 20_050);
    assert_eq!(presses(), 0, "not yet");
    at(&mut runtime, 30_000);
    assert_eq!(presses(), 1, "held to its end");
    at(&mut runtime, 40_000);
    touch(&mut runtime, on, Up);
    assert_eq!(presses(), 0, "once for the contact");

    // Enter does not press a hold either.
    runtime.frame.pointer.focused = Some(Press(false));
    runtime.event(Event::Input(port::event::Input::Key(
        port::event::Key::Enter,
    )));
    assert_eq!(presses(), 0);
}

/// Invariant: The frost under a sheet is made once when the sheet opens, kept while it stays over the same page, and gone when the sheet closes.
/// Regression: The page is blurred again at every frame the sheet is up; or a frost outlives its sheet in the runtime's memory.
/// Owner: runtime
#[test]
fn the_frost_under_a_sheet_is_made_once_and_gone_when_the_sheet_closes() {
    let mut runtime = drawing(draw_sheet);
    at(&mut runtime, 1_000);
    assert_eq!(runtime.seen().kept.len(), 1);
    at(&mut runtime, 2_000);
    let seen = runtime.seen();
    assert_eq!(seen.kept.len(), 1);
    assert_eq!(seen.kept[0].taken_at, 1_000, "not made again");
    assert!(!seen.kept[0].secret);
    assert!(seen.kept[0].pixels.iter().any(|&b| b != 0));
    DRAW.set(Some(|_, ui| ui.set_page(PAGE)));
    at(&mut runtime, 3_000);
    assert!(runtime.seen().kept.is_empty());
}

/// Invariant: No frost is kept of a page that shows a secret, and none outlives a lock.
/// Regression: A kept copy of a frame that showed a secret is on the heap after the secret left the screen.
/// Owner: runtime
#[test]
fn no_frost_of_a_secret_is_kept_and_none_outlives_a_lock() {
    let mut runtime = drawing(draw_sheet_over_secret);
    at(&mut runtime, 1_000);
    assert!(runtime.seen().kept.is_empty(), "a secret page");

    let mut runtime = drawing(draw_sheet);
    at(&mut runtime, 1_000);
    assert_eq!(runtime.seen().kept.len(), 1);
    send(
        &mut runtime,
        Addr::App(Msg::Lock(Lock {
            seal: false,
            idle: true,
        })),
    );
    assert!(runtime.seen().kept.is_empty());
    at(&mut runtime, 2_000);
    assert!(
        runtime.seen().kept.is_empty(),
        "the scope built after the lock draws no sheet"
    );
}
