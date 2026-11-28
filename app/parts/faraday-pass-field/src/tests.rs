//! The passphrase field's rules: what each kind takes, the stick rule, the pair, the eye, and
//! what a lock, a clear and a print leave of what was typed.

use faraday_app_test::{TestView, seen};
use faraday_kernel::{AtLock, Message};
use faraday_port::event::Key;
use faraday_port::storage::{StickId, StickInfo};
use faraday_text::Text;

use crate::{By, Kind, Last, Msg, Press, Refused, State, Which, differ, ready, update};

const PHRASE: &str = "correct horse battery staple";

fn type_in(state: &mut State, text: &str, by: By) {
    let view = TestView::device();
    let cx = view.ctx();
    for c in text.chars() {
        let _ = seen(update(state, Msg::Key(Key::Char(c), by), &cx));
    }
}

fn key(state: &mut State, key: Key) -> Last {
    let view = TestView::device();
    let _ = seen(update(state, Msg::Key(key, By::Screen), &view.ctx()));
    state.last()
}

fn with_stick() -> TestView {
    TestView::device().with_sticks(vec![StickInfo {
        id: StickId(Text::of("stick")),
        label: Text::of("stick"),
        boot: false,
        serial: None,
        files: Vec::new(),
        vaults: Vec::new(),
        make: Default::default(),
    }])
}

/// Invariant: A BIP-39, Krux-making and Lightning passphrase takes printable ASCII only and says so for any other letter; a vault, KeePass, Krux-reading and Core wallet passphrase takes any non-control character; a control character is no key.
/// Regression: A non-ASCII letter typed into a BIP-39 passphrase is accepted, so the seed it makes differs from what the person's other wallet gives for the same text.
/// Owner: app
#[test]
fn each_kind_takes_the_characters_of_its_standard() {
    for kind in [Kind::Bip39, Kind::KefMake, Kind::Lightning] {
        let mut s = State::new(kind);
        assert_eq!(key(&mut s, Key::Char('é')), Last::Refused(Refused::Ascii));
        assert_eq!(key(&mut s, Key::Char('a')), Last::Typed);
        assert_eq!(s.borrow().as_str(), "a");
    }
    for kind in [Kind::KefRead, Kind::Vault, Kind::Kdbx, Kind::CoreWallet] {
        let mut s = State::new(kind);
        assert_eq!(key(&mut s, Key::Char('é')), Last::Typed);
        assert_eq!(key(&mut s, Key::Char('\u{7}')), Last::Ignored);
        assert_eq!(s.borrow().as_str(), "é");
    }
}

/// Invariant: A character past the kind's most is refused as too long: 256 bytes for BIP-39, the Krux make's most, 1024 for a Core wallet, none for the rest.
/// Regression: A Krux read's passphrase is capped at the Krux make's most, or a BIP-39 passphrase grows past what the standard takes.
/// Owner: app
#[test]
fn a_character_past_the_most_is_refused_and_a_kind_without_a_most_takes_it() {
    let cases = [
        (Kind::Bip39, Some(faraday_bip::bip39::MAX_PASSPHRASE_BYTES)),
        (Kind::KefMake, Some(faraday_keys::kef::PASSPHRASE_MAX)),
        (Kind::CoreWallet, Some(1024)),
        (Kind::KefRead, None),
        (Kind::Vault, None),
    ];
    for (kind, most) in cases {
        let mut s = State::new(kind);
        type_in(&mut s, &"a".repeat(most.unwrap_or(1300)), By::Screen);
        assert_eq!(s.bytes(), most.unwrap_or(1300));
        let last = key(&mut s, Key::Char('b'));
        match most {
            Some(_) => assert_eq!(last, Last::Refused(Refused::TooLong)),
            None => assert_eq!(last, Last::Typed),
        }
    }
}

/// Invariant: With a stick in the way no character, Backspace, Delete, Enter, Tab or Shift-Tab goes in and a press on the field takes nothing; Escape is still the screen's.
/// Regression: Shift-Tab moves between a pair's fields with a stick attached, or a character is typed while keys may not load.
/// Owner: app
#[test]
fn a_stick_in_the_way_refuses_every_key_that_would_change_the_field() {
    let view = with_stick();
    let cx = view.ctx();
    let mut s = State::pair(Kind::Bip39);
    for k in [
        Key::Char('a'),
        Key::Backspace,
        Key::Delete,
        Key::Enter,
        Key::Tab,
        Key::BackTab,
    ] {
        let _ = seen(update(&mut s, Msg::Key(k, By::Screen), &cx));
        assert_eq!(s.last(), Last::Refused(Refused::Stick), "{k:?}");
    }
    assert!(!s.holds_any() && !s.on_again());
    let _ = seen(update(&mut s, Msg::Press(Press::Box(Which::Again)), &cx));
    let _ = seen(update(&mut s, Msg::Press(Press::Eye), &cx));
    assert!(!s.on_again() && !s.shown());
    let _ = seen(update(&mut s, Msg::Key(Key::Escape, By::Screen), &cx));
    assert_eq!(s.last(), Last::Escape);
}

/// Invariant: A pair is ready when its two texts are alike and not empty, differs while they do not match, and Tab moves typing between the two.
/// Regression: A pair whose repeat is a prefix of the passphrase, or whose both fields are empty, counts as ready.
/// Owner: app
#[test]
fn a_pair_is_ready_only_when_both_are_alike_and_not_empty() {
    let mut s = State::pair(Kind::Vault);
    assert!(!s.ready() && !s.differ() && !s.differs_now());
    type_in(&mut s, "abc", By::Screen);
    assert_eq!(key(&mut s, Key::Tab), Last::Typed);
    assert!(s.on_again());
    type_in(&mut s, "ab", By::Screen);
    assert!(!s.ready() && s.differ() && s.differs_now());
    type_in(&mut s, "c", By::Screen);
    assert!(s.ready() && !s.differ());
    assert_eq!(key(&mut s, Key::BackTab), Last::Typed);
    assert!(!s.on_again());
    assert!(ready("a", "a") && !ready("", "") && !differ("", "") && differ("a", "ab"));
    let mut single = State::new(Kind::Vault);
    assert!(!single.ready() && !single.differ());
    type_in(&mut single, "x", By::Screen);
    assert!(single.ready());
    assert_eq!(key(&mut single, Key::Tab), Last::Ignored);
}

/// Invariant: A passphrase typed with a hardware keyboard waits for the eye to be opened once; one typed on screen does not, a field that does not wait never does, and emptying the field ends the wait.
/// Regression: Add a key goes on with a passphrase the hardware keys gave in another layout, before the person has seen it.
/// Owner: app
#[test]
fn a_hardware_passphrase_waits_for_the_eye_to_be_opened_once() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut s = State::new(Kind::Bip39).waiting_for_eye();
    type_in(&mut s, "a", By::Screen);
    assert!(!s.waits());
    type_in(&mut s, "b", By::Hardware);
    assert!(s.waits());
    let _ = seen(update(&mut s, Msg::Press(Press::Eye), &cx));
    assert!(s.shown() && !s.waits());
    let _ = seen(update(&mut s, Msg::Press(Press::Eye), &cx));
    assert!(!s.shown() && !s.waits());

    let mut plain = State::new(Kind::Bip39);
    type_in(&mut plain, "ab", By::Hardware);
    assert!(!plain.waits());

    let mut s = State::new(Kind::Bip39).waiting_for_eye();
    type_in(&mut s, "a", By::Hardware);
    let _ = key(&mut s, Key::Backspace);
    assert!(!s.waits());
}

/// Invariant: Hiding closes the eye and keeps what was typed; a lock empties both texts, closes the eye, and leaves nothing held.
/// Regression: A passphrase typed in a pair survives a lock in the state, or the idle warning leaves the characters on show.
/// Owner: app
#[test]
fn a_lock_empties_both_texts_and_hide_only_closes_the_eye() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut s = State::pair(Kind::Vault).waiting_for_eye();
    type_in(&mut s, PHRASE, By::Hardware);
    let _ = key(&mut s, Key::Tab);
    type_in(&mut s, PHRASE, By::Screen);
    let _ = seen(update(&mut s, Msg::Press(Press::Eye), &cx));
    assert!(s.holds().typed && s.shown());

    s.hide();
    assert!(!s.shown() && s.holds().typed && s.ready());

    let _ = seen(update(&mut s, Msg::Press(Press::Eye), &cx));
    s.at_lock();
    let held = s.holds();
    assert!(!held.kept && !held.typed && !held.session && held.asks_first.is_none());
    assert!(s.borrow().is_empty() && !s.holds_any() && !s.shown() && !s.waits() && !s.on_again());
    assert!(s.read(Which::Again, true).revealed().is_none());
}

/// Invariant: A clear when the screen is done, the entry given up or a stick arrives empties both texts and closes the eye; a take moves the passphrase out and leaves the field empty.
/// Regression: A screen left by an arrow or Escape leaves its passphrase in the field for the next screen to show.
/// Owner: app
#[test]
fn a_clear_empties_both_texts_and_a_take_leaves_the_field_empty() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut s = State::pair(Kind::Bip39);
    type_in(&mut s, "one", By::Screen);
    let taken = s.take();
    assert_eq!(taken.as_str(), "one");
    assert!(s.is_empty());
    let _ = seen(update(&mut s, Msg::Fill(Text::of("faraday")), &cx));
    assert!(s.ready());
    let _ = seen(update(&mut s, Msg::Press(Press::Eye), &cx));
    let _ = seen(update(&mut s, Msg::Clear, &cx));
    assert!(!s.holds_any() && !s.shown());
    let _ = seen(update(&mut s, Msg::FillFirst(Text::of("a")), &cx));
    let _ = seen(update(&mut s, Msg::FillAgain(Text::of("b")), &cx));
    assert!(s.differ());
}

/// Invariant: The typed passphrase is never Clone and never in a Debug print or a logged message: the state prints lengths, and a key's message is withheld from the log.
/// Regression: A derive of Clone or Debug on the state, or a key message logged whole, copies or prints the passphrase.
/// Owner: app
#[test]
fn the_passphrase_is_never_cloned_printed_or_logged() {
    use core::marker::PhantomData;
    trait NotClone {
        fn is_clone() -> bool {
            false
        }
    }
    impl<T> NotClone for T {}
    struct Probe<T>(PhantomData<T>);
    #[expect(dead_code, reason = "the probe's other answer is the trait's")]
    impl<T: Clone> Probe<T> {
        fn is_clone() -> bool {
            true
        }
    }
    assert!(!Probe::<State>::is_clone());

    let mut s = State::pair(Kind::Vault);
    type_in(&mut s, PHRASE, By::Screen);
    let printed = format!("{s:?}");
    assert!(!printed.contains("horse") && !printed.contains(PHRASE), "{printed}");

    let msg = Msg::Key(Key::Char('h'), By::Screen);
    assert!(msg.public().is_none());
    assert!(!format!("{msg:?}").contains('h'));
}

/// Invariant: The read view hands out the text only while the eye is open, and counts characters, not bytes, for the dots; focus follows the field typing goes to.
/// Regression: The view gives the passphrase while the eye is closed, so the screen draws it.
/// Owner: app
#[test]
fn the_read_view_gives_the_text_only_while_the_eye_is_open() {
    let view = TestView::device();
    let cx = view.ctx();
    let mut s = State::pair(Kind::Vault);
    type_in(&mut s, "é1", By::Screen);
    let first = s.read(Which::First, true);
    assert_eq!(first.count(), 2);
    assert!(first.revealed().is_none() && first.focused());
    assert!(!s.read(Which::Again, true).focused());
    let _ = seen(update(&mut s, Msg::Press(Press::Eye), &cx));
    assert_eq!(s.read(Which::First, true).revealed(), Some("é1"));
    assert_eq!(s.read(Which::Again, true).revealed(), Some(""));
    let _ = seen(update(&mut s, Msg::Press(Press::Box(Which::Again)), &cx));
    assert!(s.read(Which::Again, true).focused());
    assert!(!s.read(Which::Again, false).focused());
}
