//! Session security through the shell contract (`docs/PLANNING.md` §5.2,
//! §8.5 #1, #2, #6, §16.21; UX.md I1–I4): the session PIN, auto-lock,
//! the lock screen, auto-wipe, wipe-and-exit, entropy and sealing.

mod common;

use common::{ABANDON, ENTROPY, Harness, PANEL, PHONE, PIN};
use opensigner_core::load::Step;
use opensigner_core::session::{ATTEMPTS, DEFAULT_LOCK_MS, DEFAULT_WIPE_MS, LOCK_OPTIONS, PIN_MAX};
use opensigner_core::{AssuranceTier, ScreenKind, ids, strings};
use osk_bip::keys::ScriptType;
use osk_shell_api::{App, ButtonId, Command, EntropyBytes, Event, Key};
use osk_ui::widgets::keyboard::KeyInput;

fn loaded(display: osk_shell_api::DisplayInfo) -> Harness {
    let mut h = Harness::new(display);
    h.start_load(&ABANDON);
    h.finish_load(None);
    assert_eq!(h.app.fingerprints().len(), 1);
    h
}

#[test]
fn the_first_key_sets_a_pin_and_later_keys_do_not() {
    let mut h = Harness::new(PANEL);
    assert!(!h.app.has_pin());
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::Confirm));
    h.tap(ids::LOAD_HOLD);
    assert_eq!(
        h.app.load_step(),
        Some(Step::Pin),
        "the PIN step follows the hold"
    );
    assert!(h.app.fingerprints().is_empty(), "not added yet");
    // The pad takes the keyboard, and Enter submits only what the ✓
    // would: three digits are not a PIN (§16.99).
    h.type_text("123");
    h.key(Key::Enter);
    assert_eq!(h.app.load_step(), Some(Step::Pin));
    for _ in 0..3 {
        h.key(Key::Backspace);
    }
    // Too short: ✓ is disabled.
    h.pad(ids::LOAD_PIN_KEYBOARD, KeyInput::Char('1'));
    assert!(
        h.app
            .key_rect(ids::LOAD_PIN_KEYBOARD, KeyInput::Done)
            .is_none(),
        "✓ is dead below four digits"
    );
    h.pad(ids::LOAD_PIN_KEYBOARD, KeyInput::Backspace);
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "1234");
    assert_eq!(h.app.load_step(), Some(Step::PinConfirm));
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "1235");
    assert_eq!(h.app.load_step(), Some(Step::Pin), "a mismatch starts over");
    h.type_pin(ids::LOAD_PIN_KEYBOARD, PIN);
    h.type_pin(ids::LOAD_PIN_KEYBOARD, PIN);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.has_pin());
    assert_eq!(h.app.fingerprints().len(), 1);
    assert_eq!(
        h.app.is_sealed(0),
        Some(true),
        "sealed under the session key"
    );
    // The second key skips the PIN step.
    h.open_load();
    h.load_choices(0);
    for w in ABANDON {
        h.type_text(w);
        h.key(Key::Enter);
    }
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_ADD_PASSPHRASE, ids::LOAD_PASS_CONTINUE);
    h.type_text("TREZOR");
    h.key(Key::Enter);
    h.tap(ids::LOAD_WHICH_CONTINUE);
    h.tap(ids::LOAD_HOLD);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.fingerprints().len(), 2);
}

#[test]
fn back_from_the_pin_step_returns_to_confirm_without_a_key() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.tap(ids::LOAD_HOLD);
    assert_eq!(h.app.load_step(), Some(Step::Pin));
    h.send(Event::Button(ButtonId::Back));
    assert_eq!(h.app.load_step(), Some(Step::Confirm));
    assert!(h.app.fingerprints().is_empty());
    assert!(!h.app.has_pin());
}

#[test]
fn auto_lock_fires_after_the_timeout_and_input_resets_it() {
    let mut h = loaded(PANEL);
    assert_eq!(h.app.lock_after_ms(), DEFAULT_LOCK_MS);
    assert!(!h.app.is_locked());
    let before = h.app.addresses(0, ScriptType::NativeSegwit, false, 3);
    assert_eq!(before.len(), 3);
    // Idle for most of the timeout, then a touch: the timer restarts.
    h.tick(DEFAULT_LOCK_MS - 5_000);
    assert_eq!(h.app.lock_in_ms(), Some(5_000));
    h.open_settings();
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    assert_eq!(h.app.lock_in_ms(), Some(DEFAULT_LOCK_MS));
    h.tick(DEFAULT_LOCK_MS - 1);
    assert!(!h.app.is_locked(), "one millisecond early");
    h.tick(1);
    assert!(h.app.is_locked());
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    assert_eq!(h.app.lock_in_ms(), None);
    // Locked: the master keys are gone but the fingerprints show.
    assert!(
        h.app
            .addresses(0, ScriptType::NativeSegwit, false, 3)
            .is_empty()
    );
    assert_eq!(h.app.fingerprints().len(), 1);
    assert!(h.app.rect_of(ids::LOCK_KEYBOARD).is_some());
    assert!(h.app.rect_of(ids::BACK).is_none(), "no navigation");
    assert!(
        h.app.rect_of(ids::HOME_SCAN).is_none(),
        "the hub is not drawn"
    );
    // Nothing but the pad works. One Back does not unlock; it only says
    // what a second Back would do (`tests/leave.rs`).
    h.send(Event::Button(ButtonId::Back));
    assert!(h.app.is_locked());
    // Locking asked for entropy, which the harness answered: the key
    // rotated and the seed was re-sealed, so the right PIN still opens
    // it and the addresses are identical.
    h.unlock(PIN);
    assert!(!h.app.is_locked());
    assert_eq!(h.app.screen(), ScreenKind::Home, "a lock leaves the flow");
    assert_eq!(
        h.app.addresses(0, ScriptType::NativeSegwit, false, 3),
        before
    );
    assert_eq!(h.app.fingerprints()[0].to_hex(), *b"73c5da0a");
    assert_eq!(h.app.attempts_left(), ATTEMPTS);
}

/// "Lock now" seals the session, and the PIN opens it again. Nothing
/// counts the auto-lock down at the user: `docs/DESIGN.md` §2.10 leaves
/// the only countdown to the eye's ring, so a tick inside the last
/// minute redraws nothing.
#[test]
fn lock_now_seals_the_session_and_the_pin_opens_it() {
    let mut h = loaded(PHONE);
    assert!(h.app.rect_of(ids::at(ids::HOME_TILE_BASE, 4)).is_some());
    h.open_settings();
    assert!(h.app.rect_of(ids::SETTINGS_LOCK).is_some());
    h.tap(ids::BACK);
    h.tick(DEFAULT_LOCK_MS - 48_000);
    assert_eq!(h.app.lock_in_ms(), Some(48_000));
    assert!(h.drain().is_empty(), "already drained by tick");
    h.now_ms += 1000;
    h.app.event(Event::Tick { now_ms: h.now_ms });
    assert!(h.drain().is_empty(), "the status line has no countdown");
    h.open_settings();
    h.tap(ids::SETTINGS_LOCK);
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    h.unlock(PIN);
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

#[test]
fn five_wrong_pins_wipe_everything() {
    let mut h = loaded(PANEL);
    h.tick(DEFAULT_LOCK_MS);
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    for i in 1..ATTEMPTS {
        h.type_pin(ids::LOCK_KEYBOARD, "0000");
        assert_eq!(h.app.attempts_left(), ATTEMPTS - i);
        assert!(h.app.is_locked());
        assert_eq!(h.app.fingerprints().len(), 1);
    }
    h.type_pin(ids::LOCK_KEYBOARD, "0000");
    assert!(!h.app.is_locked());
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.fingerprints().is_empty(), "wiped");
    assert!(!h.app.has_pin(), "the next key sets a new PIN");
    assert!(h.app.rect_of(ids::HOME_SCAN).is_some(), "the empty Home");
}

#[test]
fn auto_wipe_fires_and_is_never_shorter_than_the_lock() {
    let mut h = loaded(PANEL);
    assert_eq!(h.app.wipe_after_ms(), Some(DEFAULT_WIPE_MS));
    assert_eq!(h.app.wipe_in_ms(), Some(DEFAULT_WIPE_MS));
    h.tick(DEFAULT_LOCK_MS);
    assert!(h.app.is_locked());
    h.tick(DEFAULT_WIPE_MS - DEFAULT_LOCK_MS - 1);
    assert_eq!(h.app.fingerprints().len(), 1);
    h.tick(1);
    assert!(h.app.fingerprints().is_empty(), "wiped while locked");
    assert!(!h.app.is_locked());
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.wipe_in_ms(), None, "nothing to wipe");
    // Settings: the two timer Choices keep wipe ≥ lock.
    h.set_setting(
        ids::SETTINGS_LOCK_AFTER_ROW,
        ids::SETTINGS_LOCK_AFTER_BASE,
        3,
    );
    assert_eq!(h.app.lock_after_ms(), LOCK_OPTIONS[3]);
    assert_eq!(h.app.wipe_after_ms(), Some(1_800_000), "wipe moved up");
    h.set_setting(
        ids::SETTINGS_WIPE_AFTER_ROW,
        ids::SETTINGS_WIPE_AFTER_BASE,
        0,
    );
    assert_eq!(h.app.wipe_after_ms(), Some(300_000));
    assert_eq!(h.app.lock_after_ms(), 300_000, "lock moved down");
    h.set_setting(
        ids::SETTINGS_WIPE_AFTER_ROW,
        ids::SETTINGS_WIPE_AFTER_BASE,
        4,
    );
    assert_eq!(h.app.wipe_after_ms(), None);
    h.set_setting(
        ids::SETTINGS_LOCK_AFTER_ROW,
        ids::SETTINGS_LOCK_AFTER_BASE,
        0,
    );
    assert_eq!(h.app.lock_after_ms(), 30_000);
    // With "never", only the lock fires.
    h.tap(ids::BACK);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.tick(3_600_000);
    assert!(h.app.is_locked());
    assert_eq!(h.app.fingerprints().len(), 1);
}

#[test]
fn wipe_and_exit_wipes_then_exits_and_selftest_exit_wipes_too() {
    let mut h = loaded(PANEL);
    h.open_settings();
    h.tap(ids::SETTINGS_EXIT_ROW);
    assert_eq!(h.app.screen(), ScreenKind::WipeAndExit);
    assert_eq!(h.app.fingerprints().len(), 1, "the tap alone does nothing");
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    h.tap(ids::SETTINGS_EXIT_ROW);
    let (x, y) = h.center(ids::SETTINGS_EXIT);
    h.app.event(Event::Touch {
        x,
        y,
        phase: osk_shell_api::TouchPhase::Down,
    });
    h.app.event(Event::Tick {
        now_ms: h.now_ms + osk_ui::widgets::tokens::HOLD_MS + 10,
    });
    h.app.event(Event::Touch {
        x,
        y,
        phase: osk_shell_api::TouchPhase::Up,
    });
    let commands = h.drain();
    let exit = commands
        .iter()
        .position(|c| *c == Command::Exit)
        .expect("exit");
    let entropy = commands
        .iter()
        .position(|c| *c == Command::RequestEntropy)
        .expect("the wipe rotates the session key");
    assert!(entropy < exit, "wiped before exiting: {commands:?}");
    assert!(h.app.fingerprints().is_empty());
    assert!(!h.app.has_pin());
    // A terminal screen, not the live wipe button offering to wipe
    // nothing (UX review 2026-09-07, §3.8).
    assert_eq!(h.app.screen(), ScreenKind::Ended);
    assert!(h.app.hold_buttons().is_empty(), "nothing left to press");
    assert!(h.app.rect_of(ids::BACK).is_none(), "no way back");
    assert!(h.app.rect_of(ids::SETTINGS_EXIT).is_none());
    h.tap_at(4, 4);
    assert_eq!(h.app.screen(), ScreenKind::Ended, "nothing reopens it");
}

#[test]
fn tier_d_lock_is_a_wipe() {
    let mut h = Harness::with_tier(PANEL, AssuranceTier::D, true);
    h.start_load(&ABANDON);
    h.finish_load(None);
    assert_eq!(h.app.fingerprints().len(), 1);
    h.tick(DEFAULT_LOCK_MS);
    assert!(!h.app.is_locked());
    assert!(h.app.fingerprints().is_empty());
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

/// §4.3: "The pad is in digit order unless the 'Shuffle PIN pad'
/// setting is on; it is off by default." Turned on, the order follows the
/// session key, so two sessions with the same entropy draw the same pad
/// and a rotation redraws it.
#[test]
fn the_pad_is_in_digit_order_until_the_setting_shuffles_it() {
    let mut a = loaded(PANEL);
    let mut b = loaded(PANEL);
    assert!(!a.app.scramble_pin(), "off by default");
    assert_eq!(a.app.pin_scramble_seed(), None, "no seed while it is off");
    for h in [&mut a, &mut b] {
        h.open_settings();
        h.tap(ids::SETTINGS_SCRAMBLE);
        assert!(h.app.scramble_pin());
        h.tap(ids::BACK);
    }
    let seed = a.app.pin_scramble_seed().expect("scrambled");
    assert_eq!(
        b.app.pin_scramble_seed(),
        Some(seed),
        "same entropy, same pad"
    );
    // A different shell answer gives a different order.
    let mut d = Harness::without_entropy(PANEL);
    d.drain();
    d.app.event(Event::Entropy(EntropyBytes::new([0x11; 32])));
    d.open_settings();
    d.tap(ids::SETTINGS_SCRAMBLE);
    assert_ne!(d.app.pin_scramble_seed(), Some(seed));
    let e = Harness::with_tier(PANEL, AssuranceTier::A, true);
    assert!(!e.app.scramble_pin());
    assert_eq!(e.app.pin_scramble_seed(), None);
    // Locking rotates the key, so the lock pad differs from the set pad,
    // and the frame is redrawn once the shell's entropy arrives.
    a.open_settings();
    h_toggle(&mut a);
    a.tap(ids::SETTINGS_LOCK);
    assert!(a.app.is_locked());
    let rotated = a.app.pin_scramble_seed().expect("scrambled");
    assert_ne!(rotated, seed, "a lock rotates the session key");
    let one = a.app.key_rect(ids::LOCK_KEYBOARD, KeyInput::Char('1'));
    a.pad(ids::LOCK_KEYBOARD, KeyInput::Char('1'));
    assert_eq!(
        a.app.key_rect(ids::LOCK_KEYBOARD, KeyInput::Char('1')),
        one,
        "the drawn pad matched the state when the tap landed"
    );
    // The digits are a permutation whatever the seed: every digit has a key.
    for d in '0'..='9' {
        assert!(
            a.app
                .key_rect(ids::LOCK_KEYBOARD, KeyInput::Char(d))
                .is_some()
        );
    }
    let _ = ENTROPY;
}

/// Toggles scrambling off and on again through Settings, starting from
/// on.
fn h_toggle(h: &mut Harness) {
    h.tap(ids::SETTINGS_SCRAMBLE);
    assert!(!h.app.scramble_pin());
    assert_eq!(h.app.pin_scramble_seed(), None);
    h.tap(ids::SETTINGS_SCRAMBLE);
    assert!(h.app.scramble_pin());
}

#[test]
fn a_key_is_not_added_until_the_shell_has_answered_the_entropy_request() {
    let mut h = Harness::without_entropy(PANEL);
    assert!(h.app.session_weak());
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::Confirm));
    // The shell owes the session its entropy, so "Add key" has nothing
    // to act on: the tap does nothing and the step stays where it was.
    h.tap(ids::LOAD_HOLD);
    assert_eq!(h.app.load_step(), Some(Step::Confirm));
    assert!(h.app.fingerprints().is_empty(), "not added yet");
    // The shell answers at last, and the same tap adds the key, sealed
    // under a session key that was never weak while it held one.
    h.app.event(Event::Entropy(EntropyBytes::new(ENTROPY)));
    assert!(!h.app.session_weak());
    h.tap(ids::LOAD_HOLD);
    h.set_pin_if_asked();
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.fingerprints().len(), 1);
    assert_eq!(h.app.is_sealed(0), Some(true));
    // A lock and an unlock leave the key where it was.
    let before = h.app.addresses(0, ScriptType::NativeSegwit, false, 2);
    h.open_settings();
    h.tap(ids::SETTINGS_LOCK);
    assert!(h.app.is_locked());
    h.unlock(PIN);
    assert_eq!(
        h.app.addresses(0, ScriptType::NativeSegwit, false, 2),
        before
    );
}

#[test]
fn a_background_period_locks_at_once_and_the_wipe_timer_keeps_running() {
    // A short one: the app is locked and the key is still there.
    let mut h = loaded(PANEL);
    let before = h.app.addresses(0, ScriptType::NativeSegwit, false, 2);
    h.send(Event::Lock);
    assert!(h.app.is_locked(), "locked without waiting for the timeout");
    h.tick(60_000);
    assert_eq!(h.app.fingerprints().len(), 1);
    h.unlock(PIN);
    assert_eq!(
        h.app.addresses(0, ScriptType::NativeSegwit, false, 2),
        before
    );
    // A long one: the wipe deadline passed while the app was away, and
    // the tick that carries the elapsed time back finds nothing to keep.
    let mut h = loaded(PANEL);
    h.send(Event::Lock);
    h.tick(DEFAULT_WIPE_MS + 1);
    assert!(h.app.fingerprints().is_empty(), "wiped on the way back");
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

#[test]
fn locking_cancels_a_wizard_and_a_backup_flow_still_reads_the_words() {
    let mut h = loaded(PANEL);
    // Key detail › quiz reads the sealed words.
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_VERIFY);
    assert_eq!(h.app.screen(), ScreenKind::Backup);
    h.tap(ids::QUIZ_START);
    assert!(h.app.quiz_view().is_some());
    // The lock fires mid-quiz: the flow is dropped.
    h.tick(DEFAULT_LOCK_MS);
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    h.unlock(PIN);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.backup_step(), None);
    assert_eq!(h.app.has_mnemonic(0), Some(true));
    // The words still open after the rotation.
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_VERIFY);
    h.tap(ids::QUIZ_START);
    assert!(h.app.quiz_view().is_some());
}
/// §16.99: a laptop booted from the stick has a keyboard, and that is how
/// its person types a PIN. The pad is unchanged and takes the keyboard's
/// digits, Backspace and Enter as well as touch.
#[test]
fn the_lock_screen_takes_the_pin_from_a_keyboard() {
    let dots = |h: &Harness, n: usize| {
        let want: String = core::iter::repeat_n('\u{2022}', n).collect();
        assert!(
            h.app.texts().contains(&want),
            "{n} dots are not on the screen: {:?}",
            h.app.texts()
        );
    };
    let mut h = loaded(PANEL);
    h.tick(DEFAULT_LOCK_MS);
    assert_eq!(h.app.screen(), ScreenKind::Lock);

    // Three digits are not a PIN: Enter submits nothing and costs
    // nothing, exactly as the dead ✓ does.
    h.type_text("258");
    h.key(Key::Enter);
    assert!(h.app.is_locked());
    assert_eq!(h.app.attempts_left(), ATTEMPTS);
    dots(&h, 3);

    // Backspace takes one digit away.
    h.key(Key::Backspace);
    dots(&h, 2);

    // A wrong PIN typed costs one attempt and says what the pad says.
    h.type_text("99");
    h.key(Key::Enter);
    assert!(h.app.is_locked());
    assert_eq!(h.app.attempts_left(), ATTEMPTS - 1);
    let wrong = strings::fill1(
        strings::EN.lock_wrong,
        &format!("{}", h.app.attempts_left()),
    );
    assert!(h.app.texts().contains(&wrong), "{:?}", h.app.texts());

    // A digit past the longest PIN is ignored, as the pad's is.
    h.type_text("1234567890");
    dots(&h, PIN_MAX);
    for _ in 0..PIN_MAX {
        h.key(Key::Backspace);
    }

    h.type_text(PIN);
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.attempts_left(), ATTEMPTS);
}

/// The shuffle moves the keys and not the digits: a typed 2 is a 2
/// wherever the pad draws it.
#[test]
fn a_shuffled_pad_takes_the_same_typed_digits() {
    let mut h = loaded(PANEL);
    h.open_settings();
    h.tap(ids::SETTINGS_SCRAMBLE);
    assert!(h.app.scramble_pin());
    h.tap(ids::SETTINGS_LOCK);
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    assert!(h.app.pin_scramble_seed().is_some(), "the pad is shuffled");

    h.type_text(PIN);
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Home);
}
