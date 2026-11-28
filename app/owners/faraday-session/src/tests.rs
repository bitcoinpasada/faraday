//! The session owner's rules for its keys, the keys held for their backup and its randomness.

use faraday_app_test::{TestView, seen};
use faraday_kernel::session::{self, AddKey, Change, Draw, KeyInput, WaitingItem};
use faraday_kernel::shared::Changed;
use faraday_kernel::{
    AnyChange, AtLock, Carried, ChangeId, Ctx, Done, Feature, KeyId, Notice, Top, session::Read,
};
use faraday_keys::Refusal;
use faraday_port::bytes::{Entropy, EntropyBytes, EntropySource};
use faraday_text::secret::SecretText;

use crate::random::{NO_SEED_MS, Random};
use crate::{Msg, Session, State};

/// BIP 39's first test vector.
const WORDS: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                     abandon abandon about";

fn id(n: u32) -> ChangeId {
    ChangeId { generation: 0, n }
}

fn entropy(byte: u8) -> Entropy {
    Entropy {
        bytes: EntropyBytes::new([byte; 32]),
        source: EntropySource::Os,
    }
}

fn give(state: &mut State, cx: &Ctx<'_>, n: u32, change: Change) -> seen_type::Seen {
    seen(Session::update(
        state,
        Msg::Given(id(n), AnyChange::Session(change)),
        cx,
    ))
}

mod seen_type {
    pub(super) type Seen = faraday_app_test::Seen<crate::Msg, crate::Out>;
}

fn add_words(state: &mut State, cx: &Ctx<'_>, n: u32) -> KeyId {
    let fx = give(
        state,
        cx,
        n,
        Change::AddKey(AddKey {
            key: KeyInput::Words(SecretText::of(WORDS)),
            passphrase: None,
        }),
    );
    match fx.dones.into_iter().next() {
        Some((_, Done::Session(session::Done::AddKey(Ok(key))))) => key,
        _ => panic!("the key was refused"),
    }
}

fn started(view: &TestView) -> State {
    Session::start(&mut Carried::empty(), &view.ctx()).0
}

/// Invariant: After a lock the session holds no key, no hold, no seed and none of the person's own randomness.
/// Regression: A lock wipes the keys but leaves the seed, the pool or the person's accumulator in the state, or leaves a key held.
/// Owner: app
#[test]
fn a_lock_leaves_the_session_holding_nothing() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let key = add_words(&mut state, &cx, 1);
    let _ = give(&mut state, &cx, 2, Change::Hold(session::Hold(key)));
    let _ = give(&mut state, &cx, 3, Change::Seed(session::Seed(entropy(1))));
    let _ = give(
        &mut state,
        &cx,
        4,
        Change::AddOwn(session::AddOwn(EntropyBytes::new([9; 32]))),
    );
    let held = state.holds();
    assert!(held.kept && held.session && held.asks_first.is_some());
    assert!(state.seeded() && !state.waiting().is_empty());

    state.at_lock();

    let after = state.holds();
    assert!(!after.kept && !after.typed && !after.session && after.asks_first.is_none());
    assert!(state.keys().is_empty());
    assert!(state.key(key).is_none());
    assert!(state.waiting().is_empty());
    assert!(!state.seeded());
}

/// Invariant: A key put on hold reads as held until its backup is released, and the release is told to the other features.
/// Regression: A key made in the session reads as free to sign at once, or its release is not told.
/// Owner: app
#[test]
fn a_held_key_is_free_only_after_its_release() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let key = add_words(&mut state, &cx, 1);
    assert!(!state.key(key).is_some_and(|k| k.held));

    let fx = give(&mut state, &cx, 2, Change::Hold(session::Hold(key)));
    assert_eq!(fx.dones.len(), 1);
    assert!(state.key(key).is_some_and(|k| k.held));
    assert!(matches!(state.waiting(), [WaitingItem::Held(held)] if *held == key));

    let fx = give(&mut state, &cx, 3, Change::Release(session::Release(key)));
    assert_eq!(fx.dones.len(), 1);
    assert!(matches!(
        fx.outs.as_slice(),
        [Top::Shared(Changed::Session(session::Changed::HeldReleased(released)))] if *released == key
    ));
    assert!(state.key(key).is_some_and(|k| !k.held));
    assert!(state.waiting().is_empty());
}

/// Invariant: A key removed takes its hold with it, and its handle is never given to another key.
/// Regression: A removed key's hold stays in what waits, or a new key takes the old handle.
/// Owner: app
#[test]
fn a_removed_key_leaves_no_hold_and_no_handle_behind() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let first = add_words(&mut state, &cx, 1);
    let _ = give(&mut state, &cx, 2, Change::Hold(session::Hold(first)));

    let fx = give(
        &mut state,
        &cx,
        3,
        Change::RemoveKey(session::RemoveKey(first)),
    );
    assert!(matches!(
        fx.outs.as_slice(),
        [Top::Shared(Changed::Session(session::Changed::KeyGone(gone)))] if *gone == first
    ));
    assert!(state.waiting().is_empty());
    assert!(state.key(first).is_none());

    let second = add_words(&mut state, &cx, 4);
    assert_ne!(first, second);
    assert!(state.key(first).is_none());
}

/// Invariant: A lock carries the mark that a key held for its backup is the only copy of itself.
/// Regression: The next build is not told a key was held, so a lock goes on without the warning.
/// Owner: app
#[test]
fn a_held_key_marks_the_carry_as_the_only_copy() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let key = add_words(&mut state, &cx, 1);

    let mut free = Carried::empty();
    Session::carry(&mut state, &mut free);
    assert!(!free.only_copy());

    let _ = give(&mut state, &cx, 2, Change::Hold(session::Hold(key)));
    let mut held = Carried::empty();
    Session::carry(&mut state, &mut held);
    assert!(held.only_copy());
}

/// Invariant: A key does not come in while a stick is attached, and what the keys refuse for any other reason stays refused.
/// Regression: The session adds a key with a stick in, or adds the same key twice.
/// Owner: app
#[test]
fn a_key_is_refused_with_a_stick_in_and_when_it_is_already_here() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let _ = add_words(&mut state, &cx, 1);

    let fx = give(
        &mut state,
        &cx,
        2,
        Change::AddKey(AddKey {
            key: KeyInput::Words(SecretText::of(WORDS)),
            passphrase: None,
        }),
    );
    assert!(matches!(
        fx.dones.as_slice(),
        [(
            _,
            Done::Session(session::Done::AddKey(Err(Refusal::Duplicate(_))))
        )]
    ));

    state.store.set_stick(Some("stick"));
    let fx = give(
        &mut state,
        &cx,
        3,
        Change::AddKey(AddKey {
            key: KeyInput::Words(SecretText::of(WORDS)),
            passphrase: Some(SecretText::of("a")),
        }),
    );
    assert!(matches!(
        fx.dones.as_slice(),
        [(
            _,
            Done::Session(session::Done::AddKey(Err(Refusal::StickAttached(_))))
        )]
    ));
    assert_eq!(state.keys().len(), 1);
}

/// Invariant: A passphrase is never dropped: a master seed given with one is refused, not added without it.
/// Regression: A seed arrives with a passphrase and the key added is the one without it.
/// Owner: app
#[test]
fn a_passphrase_with_a_master_seed_is_refused() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let fx = give(
        &mut state,
        &cx,
        1,
        Change::AddKey(AddKey {
            key: KeyInput::Seed(faraday_crypto::Secret::new(vec![7; 32])),
            passphrase: Some(SecretText::of("a passphrase")),
        }),
    );
    assert!(matches!(
        fx.dones.as_slice(),
        [(_, Done::Session(session::Done::AddKey(Err(_))))]
    ));
    assert!(state.keys().is_empty());
}

/// Invariant: A draw made before the seed is in gets no bytes until the system's answers are, and is then answered once.
/// Regression: A draw is answered from the pool before the seed, is dropped without an answer, or is answered twice.
/// Owner: app
#[test]
fn a_draw_before_the_seed_waits_and_is_answered_once() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);

    let fx = give(
        &mut state,
        &cx,
        1,
        Change::Draw(Draw {
            what: "a test",
            n: 32,
        }),
    );
    assert!(fx.dones.is_empty());
    assert_eq!(fx.asks.len(), 1);
    assert!(!state.seeded());

    // The first answer is the seed, which draws nothing: the draw still waits, and the session asks again.
    let fx = seen(Session::update(
        &mut state,
        Msg::Entropy(Ok(entropy(1))),
        &cx,
    ));
    assert!(state.seeded());
    assert!(fx.dones.is_empty());
    assert_eq!(fx.asks.len(), 1);

    // The second goes to the pool and serves the draw.
    let fx = seen(Session::update(
        &mut state,
        Msg::Entropy(Ok(entropy(2))),
        &cx,
    ));
    assert!(matches!(
        fx.dones.as_slice(),
        [(waiting, Done::Session(session::Done::Draw(Ok(_))))] if *waiting == id(1)
    ));

    let fx = seen(Session::update(
        &mut state,
        Msg::Entropy(Ok(entropy(3))),
        &cx,
    ));
    assert!(fx.dones.is_empty());
}

/// Invariant: A draw is the answer hashed with what it is for, so no two purposes draw the same bytes, and the person's own randomness is mixed in.
/// Regression: Two purposes draw the same bytes from one answer, or what the person added is left out.
/// Owner: app
#[test]
fn a_draw_is_the_answer_hashed_with_its_purpose_and_the_persons_own() {
    let mut random = Random::default();
    assert!(random.arrived(&[1; 32]));
    assert!(!random.arrived(&[2; 32]));
    assert!(!random.arrived(&[3; 32]));
    let plain = random.take("a").map(|b| *b.expose());
    let other = random.take("b").map(|b| *b.expose());
    let mut expected = b"faraday fresh".to_vec();
    expected.extend_from_slice(b"a");
    expected.extend_from_slice(&[3; 32]);
    assert_eq!(plain, Some(faraday_crypto::sha256(&expected)));
    assert_ne!(plain, other);

    let mut own = Random::default();
    own.own_add(&[5; 32]);
    assert!(own.arrived(&[1; 32]));
    assert!(!own.arrived(&[3; 32]));
    let mixed = own.take("a").map(|b| *b.expose());
    assert_ne!(mixed, plain);
}

/// Invariant: A draw that waits a minute for the system's answers is answered with no randomness, and not before.
/// Regression: A draw waits for ever when the system gives no randomness, or gives up sooner than a slow board needs.
/// Owner: app
#[test]
fn a_draw_gives_up_after_its_minute() {
    let mut random = Random::default();
    random.wait(id(1), "a test", 1_000);
    assert!(random.expired(1_000 + NO_SEED_MS - 1).is_empty());
    assert_eq!(random.expired(1_000 + NO_SEED_MS), vec![id(1)]);
    assert!(!random.is_waiting());
}

/// Invariant: A draw is of one answer of 32 bytes at most; one for more is refused, not cut.
/// Regression: A draw for more bytes than the answer holds is answered with fewer and read as whole.
/// Owner: app
#[test]
fn a_draw_of_more_than_one_answer_is_refused() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let fx = give(
        &mut state,
        &cx,
        1,
        Change::Draw(Draw {
            what: "a test",
            n: 33,
        }),
    );
    assert!(matches!(
        fx.dones.as_slice(),
        [(_, Done::Session(session::Done::Draw(Err(_))))]
    ));
}

/// Invariant: A tick with nothing waiting asks for nothing.
/// Regression: The session asks the adapter for entropy on every tick whether or not a draw waits.
/// Owner: app
#[test]
fn a_tick_with_no_draw_waiting_asks_nothing() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let fx = seen(Session::notice(
        &mut state,
        &Notice::Facts(faraday_kernel::FactsChanged::Time),
        &cx,
    ));
    assert!(fx.asks.is_empty() && fx.dones.is_empty());
}
