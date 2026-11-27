//! A tool or test-support crate: the application's test support (09 §8; 07 §2.2): `TestView`,
//! `test_compute`, `Seen`, `seen`, `answer`, `done`, and the helpers an update test needs beside
//! them: `asked`, `changed` and `work`, which open an ask, a change and a job the way the
//! runtime does, since a test outside the application directories may call `into_parts` (A16).

use std::cell::RefCell;

use faraday_bip::keys::Network;
use faraday_crypto::work::{Compute, Hooks, Work};
use faraday_kernel::shared;
use faraday_kernel::{
    AskFor, ChangeFor, ChangeReply, Clock, Ctx, Effects, Facts, Fx, Job, JobKind, Message, Mode,
    Note, Reply, Started, View, ViewOrder, session, vaults,
};
use faraday_keys::Keys;
use faraday_port as port;
use faraday_testkit::{TestSeed, TestWallet};

mod facts;

/// What an update test reads of the session: the keys added with `with_key`, which a test reads
/// through `Keys` (`faraday-keys`' own reads). A key or a wallet by id, and the lists, are empty:
/// `KeyId` and `WalletId` are minted by the session owner only, which is not built yet.
struct TestSession {
    keys: Keys,
}

impl session::Read for TestSession {
    fn keys(&self) -> &Keys {
        &self.keys
    }

    fn key(&self, _id: faraday_kernel::KeyId) -> Option<session::KeyView<'_>> {
        None
    }

    fn wallets(&self) -> &[session::WalletView] {
        &[]
    }

    fn wallet(&self, _id: faraday_kernel::WalletId) -> Option<&session::WalletView> {
        None
    }

    fn waiting(&self) -> &[session::WaitingItem] {
        &[]
    }

    fn inbox(&self) -> &[session::FileView] {
        &[]
    }

    fn outbox(&self) -> &[session::FileView] {
        &[]
    }

    fn seeded(&self) -> bool {
        false
    }

    fn edits(&self) -> faraday_kernel::Edits {
        faraday_kernel::Edits::NONE
    }
}

/// A View for a test: an empty or loaded session, vaults and settings, and Facts for a named
/// product (device, Demo, Starter).
pub struct TestView {
    session: Box<dyn session::Read>,
    vaults: Box<dyn vaults::Read>,
    settings: shared::Settings,
    facts: Facts,
    mode: Mode,
    clock: Clock,
    seeds: Vec<TestSeed>,
    wallets: Vec<TestWallet>,
    started: Started,
}

impl TestView {
    /// The offline device: Mainnet, a stick, no network.
    #[must_use]
    pub fn device() -> Self {
        TestView::on(
            Network::Mainnet,
            facts::device(),
            Mode {
                network: Network::Mainnet,
                practice: false,
                online: false,
            },
        )
    }

    /// Practice, Testnet, online.
    #[must_use]
    pub fn demo() -> Self {
        TestView::on(
            Network::Testnet,
            facts::demo(),
            Mode {
                network: Network::Testnet,
                practice: true,
                online: true,
            },
        )
    }

    fn on(network: Network, facts: Facts, mode: Mode) -> Self {
        TestView {
            session: Box::new(TestSession {
                keys: Keys::on(network),
            }),
            vaults: Box::new(shared::Empty::new()),
            settings: facts::settings(),
            facts,
            mode,
            clock: Clock {
                now_ms: 0,
                unix_secs: 1_790_000_000,
                offset_secs: 0,
                set: false,
                last_input_ms: 0,
            },
            seeds: Vec::new(),
            wallets: Vec::new(),
            started: Started::Power,
        }
    }

    /// The key the words give, added to the session's keys (the session owner's `AddKey` does
    /// the same through `Keys::add_words`, which is not built yet). Panics on words the keys
    /// refuse: a test names its seeds from `faraday-testkit`.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test support: a test seed the keys refuse is the test's own mistake"
    )]
    pub fn with_key(mut self, words: TestSeed) -> Self {
        self.seeds.push(words);
        let mut keys = Keys::on(self.mode.network);
        for seed in &self.seeds {
            keys.add_words(test_compute(), seed.words, seed.name, None)
                .expect("a test seed");
        }
        self.session = Box::new(TestSession { keys });
        self
    }

    /// The wallet, kept for a test to read with `wallets_held`: the session's own list stays
    /// empty until `WalletId` can be minted by test support.
    #[must_use]
    pub fn with_wallet(mut self, kit: TestWallet) -> Self {
        self.wallets.push(kit);
        self
    }

    /// This build of the scope began as `started` says (a lock, a power-on).
    #[must_use]
    pub fn with_started(mut self, started: Started) -> Self {
        self.started = started;
        self
    }

    /// With the test `Compute`.
    #[must_use]
    pub fn ctx(&self) -> Ctx<'_> {
        Ctx {
            view: View {
                session: &*self.session,
                vaults: &*self.vaults,
                settings: &self.settings,
                facts: &self.facts,
                mode: self.mode,
                clock: Clock {
                    now_ms: self.clock.now_ms,
                    unix_secs: self.clock.unix_secs,
                    offset_secs: self.clock.offset_secs,
                    set: self.clock.set,
                    last_input_ms: self.clock.last_input_ms,
                },
                running: &[],
                started: self.started,
            },
            compute: test_compute(),
        }
    }
}

impl TestView {
    /// The wallets given to `with_wallet`.
    #[must_use]
    pub fn wallets_held(&self) -> &[TestWallet] {
        &self.wallets
    }
}

thread_local! {
    static COUNTED: RefCell<Vec<Work>> = const { RefCell::new(Vec::new()) };
    static COMPUTE: &'static Compute = mint();
}

fn note(work: Work) {
    COUNTED.with(|counted| counted.borrow_mut().push(work));
}

/// The test token: a mint of this crate's, which counts what it is asked to do by kind.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: the test support's own token, which counts by kind (R2)"
)]
fn mint() -> &'static Compute {
    Box::leak(Box::new(Compute::new(Hooks {
        count: note,
        stop: || false,
        progress: |_, _| {},
    })))
}

/// Minted once per thread (a `Compute` is `!Send`); counts costly work by kind, read with
/// `counted` and cleared with `forget_counts`.
#[must_use]
pub fn test_compute() -> &'static Compute {
    COMPUTE.with(|compute| *compute)
}

/// The costly work the test token on this thread has counted since the last `forget_counts`, in
/// order.
#[must_use]
pub fn counted() -> Vec<Work> {
    COUNTED.with(|counted| counted.borrow().clone())
}

/// Clears the count of this thread's test token.
pub fn forget_counts() {
    COUNTED.with(|counted| counted.borrow_mut().clear());
}

/// The kind of one effect, for the order an update returned them in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Order,
    Ask,
    Change,
    Done,
    Job,
    Stop,
    Then,
    Out,
    Note,
    View,
}

/// What an update returned, by kind, in order.
pub struct Seen<M, O> {
    pub orders: Vec<port::Order>,
    pub asks: Vec<AskFor<M>>,
    pub changes: Vec<ChangeFor<M>>,
    pub jobs: Vec<Job<M>>,
    pub thens: Vec<M>,
    pub outs: Vec<O>,
    pub notes: Vec<Note>,
    /// An owner's answers to a change it was given.
    pub dones: Vec<(shared::ChangeId, shared::Done)>,
    /// The jobs asked to stop.
    pub stops: Vec<JobKind>,
    /// What the update asked of the view.
    pub views: Vec<ViewOrder>,
    /// Every effect's kind, in the order the update returned them across the kinds.
    pub order: Vec<Kind>,
}

#[must_use]
pub fn seen<M: Message, O: Message>(fx: Effects<M, O>) -> Seen<M, O> {
    let mut seen = Seen {
        orders: Vec::new(),
        asks: Vec::new(),
        changes: Vec::new(),
        jobs: Vec::new(),
        thens: Vec::new(),
        outs: Vec::new(),
        notes: Vec::new(),
        dones: Vec::new(),
        stops: Vec::new(),
        views: Vec::new(),
        order: Vec::new(),
    };
    for one in fx.into_list() {
        match one {
            Fx::Order(o) => {
                seen.order.push(Kind::Order);
                seen.orders.push(o);
            }
            Fx::Ask(a) => {
                seen.order.push(Kind::Ask);
                seen.asks.push(a);
            }
            Fx::Change(c) => {
                seen.order.push(Kind::Change);
                seen.changes.push(c);
            }
            Fx::Done(id, d) => {
                seen.order.push(Kind::Done);
                seen.dones.push((id, d));
            }
            Fx::Job(j) => {
                seen.order.push(Kind::Job);
                seen.jobs.push(j);
            }
            Fx::Stop(k) => {
                seen.order.push(Kind::Stop);
                seen.stops.push(k);
            }
            Fx::Then(m) => {
                seen.order.push(Kind::Then);
                seen.thens.push(m);
            }
            Fx::Out(o) => {
                seen.order.push(Kind::Out);
                seen.outs.push(o);
            }
            Fx::Note(n) => {
                seen.order.push(Kind::Note);
                seen.notes.push(n);
            }
            Fx::View(v) => {
                seen.order.push(Kind::View);
                seen.views.push(v);
            }
        }
    }
    seen
}

/// The request an ask carries and the reply that turns its answer into the asker's message.
#[must_use]
pub fn asked<M: Message>(ask: AskFor<M>) -> (port::Ask, Reply<M>) {
    ask.into_parts()
}

/// The change an owner is given and the reply that turns its `Done` into the asker's message.
#[must_use]
pub fn changed<M: Message>(change: ChangeFor<M>) -> (shared::AnyChange, ChangeReply<M>) {
    change.into_parts()
}

/// A job's work run inline with the test `Compute`: its kind and the message it ends in.
#[must_use]
pub fn work<M: Message>(job: Job<M>) -> (JobKind, M) {
    let parts = job.into_parts();
    let message = (parts.run)(test_compute());
    (parts.kind, message)
}

/// Answers an ask as an adapter would.
#[must_use]
pub fn answer<M: Message>(ask: AskFor<M>, a: port::Answer) -> M {
    let (_request, reply) = ask.into_parts();
    reply(a).0
}

/// Answers a change as an owner would.
#[must_use]
pub fn done<M: Message>(change: ChangeFor<M>, d: shared::Done) -> M {
    let (_change, reply) = change.into_parts();
    reply(d).0
}
