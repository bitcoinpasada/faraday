//! The session owner's rules for its keys, the keys held for their backup and its randomness.

use faraday_app_test::{TestView, seen};
use faraday_kernel::session::{self, AddKey, Change, Draw, KeyInput, NoRandomness, WaitingItem};
use faraday_kernel::shared::Changed;
use faraday_kernel::{
    AnyChange, AtLock, Carried, ChangeId, Ctx, Done, Feature, KeyId, Notice, Top, session::Read,
};
use faraday_keys::Refusal;
use faraday_port::bytes::{Entropy, EntropyBytes, EntropySource};
use faraday_text::secret::SecretText;

use crate::random::{NO_SEED_MS, POOL, Random};
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
    assert!(held.kept && held.session && held.asks_first.is_none());
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

/// Invariant: A key held for its backup stays held when it is removed and typed in again, under a new handle that is never an old one.
/// Regression: Removing a held key drops its hold, so the same key typed in again signs and receives with no backup.
/// Owner: app
#[test]
fn a_held_key_removed_and_added_again_is_still_held() {
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
    assert!(state.key(second).is_some_and(|k| k.held));
    assert!(matches!(state.waiting(), [WaitingItem::Held(held)] if *held == second));

    let _ = give(&mut state, &cx, 5, Change::RemoveAll(session::RemoveAll));
    let third = add_words(&mut state, &cx, 6);
    assert!(state.key(third).is_some_and(|k| k.held));

    let _ = give(&mut state, &cx, 7, Change::Release(session::Release(third)));
    assert!(state.key(third).is_some_and(|k| !k.held));
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

/// Invariant: A draw made before the seed is in gets no bytes until the system's answers are, and is then answered once with the bytes asked for.
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
            n: 40,
        }),
    );
    assert!(fx.dones.is_empty());
    assert!(!state.seeded());

    // The first answer is the seed, which draws nothing.
    let fx = seen(Session::update(
        &mut state,
        Msg::Entropy(Ok(entropy(1))),
        &cx,
    ));
    assert!(state.seeded());
    assert!(fx.dones.is_empty());

    // 40 bytes take two answers: one is not enough.
    let fx = seen(Session::update(
        &mut state,
        Msg::Entropy(Ok(entropy(2))),
        &cx,
    ));
    assert!(fx.dones.is_empty());
    let fx = seen(Session::update(
        &mut state,
        Msg::Entropy(Ok(entropy(3))),
        &cx,
    ));
    match fx.dones.as_slice() {
        [(waiting, Done::Session(session::Done::Draw(Ok(bytes))))] => {
            assert_eq!(*waiting, id(1));
            assert_eq!(bytes.expose().len(), 40);
        }
        _ => panic!("the draw was not answered once"),
    }
    // The two answers it took are asked for again.
    assert_eq!(fx.asks.len(), 2);

    let fx = seen(Session::update(
        &mut state,
        Msg::Entropy(Ok(entropy(4))),
        &cx,
    ));
    assert!(fx.dones.is_empty());
}

/// Invariant: The session asks the system for its seed and a pool of answers when it starts.
/// Regression: Nothing asks until a feature draws, so a feature that waits for the seed waits for ever.
/// Owner: app
#[test]
fn the_session_asks_for_its_seed_at_start() {
    let view = TestView::device();
    let (_state, fx) = Session::start(&mut Carried::empty(), &view.ctx());
    assert_eq!(seen(fx).asks.len(), POOL + 1);
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
    let plain = random.take_bytes("a", 32).map(|b| b.expose().clone());
    let other = random.take_bytes("b", 32).map(|b| b.expose().clone());
    let mut expected = b"faraday fresh".to_vec();
    expected.extend_from_slice(b"a");
    expected.extend_from_slice(&[3; 32]);
    assert_eq!(plain, Some(faraday_crypto::sha256(&expected).to_vec()));
    assert_ne!(plain, other);

    let mut own = Random::default();
    own.own_add(&[5; 32]);
    assert!(own.arrived(&[1; 32]));
    assert!(!own.arrived(&[3; 32]));
    let mixed = own.take_bytes("a", 32).map(|b| b.expose().clone());
    assert_ne!(mixed, plain);
}

/// Invariant: A draw that waits a minute for the system's answers is answered with no randomness, and not before.
/// Regression: A draw waits for ever when the system gives no randomness, or gives up sooner than a slow board needs.
/// Owner: app
#[test]
fn a_draw_gives_up_after_its_minute() {
    let mut random = Random::default();
    random.begin(0);
    assert!(random.arrived(&[1; 32]));
    random.wait(id(1), "a test", 32, 1_000);
    assert!(random.expired(1_000 + NO_SEED_MS - 1).is_empty());
    assert_eq!(random.expired(1_000 + NO_SEED_MS), vec![id(1)]);
    assert!(!random.is_waiting());
}

/// Invariant: A draw of more bytes than the pool's most is refused as too long, not as the system giving no randomness.
/// Regression: A draw for 833 bytes is answered as if the system gave no randomness, or is cut and read as whole.
/// Owner: app
#[test]
fn a_draw_of_too_many_bytes_is_refused_as_too_long() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let fx = give(
        &mut state,
        &cx,
        1,
        Change::Draw(Draw {
            what: "a test",
            n: 833,
        }),
    );
    assert!(matches!(
        fx.dones.as_slice(),
        [(
            _,
            Done::Session(session::Done::Draw(Err(NoRandomness::TooLong)))
        )]
    ));
}

/// Invariant: A draw made a minute after the build with no seed is answered with no randomness in the same dispatch.
/// Regression: Every draw waits its own minute, so a board that gives no randomness answers a late draw a minute later.
/// Owner: app
#[test]
fn a_draw_after_the_late_seed_minute_is_refused_at_once() {
    let view = TestView::device().with_now(NO_SEED_MS);
    let cx = view.ctx();
    let mut state = started(&view);
    state.random.begin(0);
    let fx = give(
        &mut state,
        &cx,
        1,
        Change::Draw(Draw {
            what: "a test",
            n: 32,
        }),
    );
    assert!(matches!(
        fx.dones.as_slice(),
        [(
            _,
            Done::Session(session::Done::Draw(Err(NoRandomness::System)))
        )]
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

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

fn fingerprint_of_seed(bytes: &[u8]) -> faraday_bip::keys::Fingerprint {
    let mut keys = faraday_keys::Keys::on(faraday_bip::keys::Network::Mainnet);
    keys.add_seed(faraday_app_test::test_compute(), bytes, "")
        .unwrap()
}

fn add_shares(state: &mut State, cx: &Ctx<'_>, n: u32, strings: &[&str]) -> session::Done {
    let fx = give(
        state,
        cx,
        n,
        Change::AddKey(AddKey {
            key: KeyInput::Shares(strings.iter().map(|s| SecretText::of(s)).collect()),
            passphrase: None,
        }),
    );
    match fx.dones.into_iter().next() {
        Some((_, Done::Session(done))) => done,
        _ => panic!("no answer"),
    }
}

/// Invariant: A codex32 secret string loads the master seed it carries, and a set of shares loads the seed they recover, as BIP 93's vectors give them.
/// Regression: The string's 5-bit symbols are loaded as the seed, or a share nobody typed is recorded with the key.
/// Owner: app
#[test]
fn codex32_strings_load_the_seed_of_bip_93s_vectors() {
    let view = TestView::device();
    let cx = view.ctx();

    let mut state = started(&view);
    let done = add_shares(
        &mut state,
        &cx,
        1,
        &["ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw"],
    );
    assert!(matches!(done, session::Done::AddKey(Ok(_))));
    assert_eq!(
        state.keys().first().map(|k| k.fingerprint),
        Some(fingerprint_of_seed(&hex(
            "318c6318c6318c6318c6318c6318c631"
        )))
    );

    // Vector 2: shares A and C recover the seed, and only those two are recorded.
    let mut state = started(&view);
    let done = add_shares(
        &mut state,
        &cx,
        1,
        &[
            "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
            "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
        ],
    );
    assert!(matches!(done, session::Done::AddKey(Ok(_))));
    let key = state.keys().first().unwrap();
    assert_eq!(
        key.fingerprint,
        fingerprint_of_seed(&hex("d1808e096b35b209ca12132b264662a5"))
    );
    assert_eq!(key.codex32.as_ref().map(|c| c.strings.len()), Some(2));
}

/// Invariant: SLIP-39 shares load the seed they recover with the passphrase given, and a passphrase changes the key.
/// Regression: SLIP-39 shares are refused as codex32, or the passphrase is dropped.
/// Owner: app
#[test]
fn slip39_shares_load_with_their_passphrase() {
    const SHARE: &str = "duckling enlarge academic academic agency result length solution \
                         fridge kidney coal piece deal husband erode duke ajar critical \
                         decision keyboard";
    let view = TestView::device();
    let cx = view.ctx();
    let mut keys_seen = Vec::new();
    for pass in [Some("TREZOR"), None] {
        let mut state = started(&view);
        let fx = give(
            &mut state,
            &cx,
            1,
            Change::AddKey(AddKey {
                key: KeyInput::Shares(vec![SecretText::of(SHARE)]),
                passphrase: pass.map(SecretText::of),
            }),
        );
        assert!(matches!(
            fx.dones.as_slice(),
            [(_, Done::Session(session::Done::AddKey(Ok(_))))]
        ));
        keys_seen.push(state.keys().first().map(|k| k.fingerprint));
    }
    assert_ne!(keys_seen[0], keys_seen[1]);
    let c = faraday_app_test::test_compute();
    let share = faraday_bip::slip39::Share::parse(SHARE).unwrap();
    let seed = faraday_keys::slip39_seed(c, &[share], b"TREZOR").unwrap();
    assert_eq!(keys_seen[0], Some(fingerprint_of_seed(&seed)));
}

/// Invariant: In Practice the key store takes only the published samples.
/// Regression: Practice's session takes any typed key, because nothing set the demo's list.
/// Owner: app
#[test]
fn practice_takes_only_the_published_samples() {
    let public: Vec<[u8; 4]> = faraday_keys::published::samples().to_vec();
    let set = Box::leak(Box::new(faraday_kernel::PracticeSamples {
        handouts: &[],
        public: public.leak(),
        nodes: &[],
    }));
    let view = TestView::demo().with_samples(set);
    let cx = view.ctx();
    let mut state = started(&view);
    let fx = give(
        &mut state,
        &cx,
        1,
        Change::AddKey(AddKey {
            key: KeyInput::Words(SecretText::of(
                "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
            )),
            passphrase: None,
        }),
    );
    assert!(matches!(
        fx.dones.as_slice(),
        [(
            _,
            Done::Session(session::Done::AddKey(Err(Refusal::NotAllowed(_))))
        )]
    ));
    let _ = add_words(&mut state, &cx, 2);
    assert_eq!(state.keys().len(), 1);
}

/// Invariant: An extended private key that does not read is refused with the words of what is wrong with it.
/// Regression: The refusal carries no text, so the person sees an empty line.
/// Owner: app
#[test]
fn an_xprv_that_does_not_read_is_refused_in_words() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut state = started(&view);
    let fx = give(
        &mut state,
        &cx,
        1,
        Change::AddKey(AddKey {
            key: KeyInput::Xprv(SecretText::of("xprvnotakey")),
            passphrase: None,
        }),
    );
    match fx.dones.as_slice() {
        [(_, Done::Session(session::Done::AddKey(Err(Refusal::Words(text)))))] => {
            assert!(!text.as_str().is_empty());
        }
        _ => panic!("not refused in words"),
    }
}

/// Invariant: A Practice given no samples takes no key at all.
/// Regression: A Practice the product gave no set takes any key, so the boundary fails open.
/// Owner: app
#[test]
fn practice_with_no_samples_takes_no_key() {
    let view = TestView::demo();
    let cx = view.ctx();
    let mut state = started(&view);
    let fx = give(
        &mut state,
        &cx,
        1,
        Change::AddKey(AddKey {
            key: KeyInput::Words(SecretText::of(WORDS)),
            passphrase: None,
        }),
    );
    assert!(matches!(
        fx.dones.as_slice(),
        [(
            _,
            Done::Session(session::Done::AddKey(Err(Refusal::NotAllowed(_))))
        )]
    ));
}

/// Invariant: Codex32 strings of two sets, or two different strings at one index, load no key, and the same string twice is one.
/// Regression: A secret string typed with a share of another seed loads, and the share is kept with the key.
/// Owner: app
#[test]
fn codex32_strings_of_another_set_or_clashing_load_no_key() {
    const SHARE_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
    const SHARE_C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
    const OTHER_SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
    let view = TestView::device();
    let cx = view.ctx();

    let mut state = started(&view);
    let done = add_shares(&mut state, &cx, 1, &[SHARE_A, OTHER_SECRET]);
    assert!(matches!(
        done,
        session::Done::AddKey(Err(Refusal::Words(t))) if t.as_str() == faraday_strings::EN.forms_other_seed_string
    ));
    assert!(state.keys().is_empty());

    // A string at an index already given that differs from it.
    #[expect(
        clippy::disallowed_methods,
        reason = "a test makes two secret strings of one id, which no domain loader does"
    )]
    let made = |seed: u8| {
        faraday_bip::codex32::Codex32::from_seed(0, *b"test", &[seed; 16])
            .unwrap()
            .encode()
            .as_str()
            .to_owned()
    };
    let (one, two) = (made(1), made(2));
    let clash = add_shares(&mut state, &cx, 2, &[&one, &two]);
    assert!(matches!(
        clash,
        session::Done::AddKey(Err(Refusal::Words(t))) if t.as_str() == faraday_strings::EN.forms_string_clash
    ));
    assert!(state.keys().is_empty());

    // The same string twice is one string: two shares still make the key.
    let done = add_shares(&mut state, &cx, 3, &[SHARE_A, SHARE_A, SHARE_C]);
    assert!(matches!(done, session::Done::AddKey(Ok(_))));
}
