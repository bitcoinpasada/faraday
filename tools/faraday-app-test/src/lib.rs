//! A tool or test-support crate: the application's test support (09 §8; 07 §2.2): `TestView`,
//! `test_compute`, `Seen`, `seen`, `answer`, `done`.

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

use faraday_crypto::work::Compute;
use faraday_kernel::shared;
use faraday_kernel::{
    AskFor, ChangeFor, Clock, Ctx, Effects, Facts, Job, Message, Mode, Note, session, vaults,
};
use faraday_port as port;
use faraday_testkit::{TestSeed, TestWallet};

/// A View for a test: an empty or loaded session, vaults and settings, and Facts for a named
/// product (device, Demo, Starter).
pub struct TestView {
    session: Box<dyn session::Read>,
    vaults: Box<dyn vaults::Read>,
    settings: shared::Settings,
    facts: Facts,
    mode: Mode,
    clock: Clock,
}

impl TestView {
    #[must_use]
    pub fn device() -> Self {
        todo!()
    }

    /// Practice, Testnet, online.
    #[must_use]
    pub fn demo() -> Self {
        todo!()
    }

    /// Through the session owner's `AddKey`.
    #[must_use]
    pub fn with_key(self, _words: TestSeed) -> Self {
        todo!()
    }

    #[must_use]
    pub fn with_wallet(self, _kit: TestWallet) -> Self {
        todo!()
    }

    /// With the test `Compute`.
    #[must_use]
    pub fn ctx(&self) -> Ctx<'_> {
        todo!()
    }
}

/// Minted with `Compute::unchecked` under expect (R2); counts costly work by kind.
#[must_use]
pub fn test_compute() -> &'static Compute {
    todo!()
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
}

#[must_use]
pub fn seen<M: Message, O: Message>(_fx: Effects<M, O>) -> Seen<M, O> {
    todo!()
}

/// Answers an ask as an adapter would.
#[must_use]
pub fn answer<M: Message>(_ask: AskFor<M>, _a: port::Answer) -> M {
    todo!()
}

/// Answers a change as an owner would.
#[must_use]
pub fn done<M: Message>(_change: ChangeFor<M>, _d: shared::Done) -> M {
    todo!()
}
