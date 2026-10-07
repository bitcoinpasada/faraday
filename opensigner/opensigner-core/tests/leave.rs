//! Back where nothing is behind (`docs/PLANNING.md` §16.73): on Home,
//! on the lock screen and on the stored key's pad, one Back says what a
//! second does and a second inside the window leaves the app with
//! memory cleared. A key kept on the device stays kept: leaving is not
//! "Wipe and exit".

mod common;

use common::{ABANDON, Element, Harness, PANEL, PIN, SECURE_PHONE};
use opensigner_core::{LEAVE_WINDOW_MS, ScreenKind, ids, strings};
use osk_shell_api::{Command, Key};

fn says(h: &Harness, text: &str) -> bool {
    h.app.texts().iter().any(|t| t == text)
}

#[test]
fn one_back_on_home_asks_for_a_second_and_the_second_leaves() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    h.seen.clear();

    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Home, "one Back stays");
    assert!(says(&h, strings::EN.leave_again), "{:?}", h.app.texts());
    assert!(!h.seen.contains(&Command::Exit));
    assert_eq!(h.app.fingerprints().len(), 1, "nothing left memory yet");

    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Ended);
    assert!(h.seen.contains(&Command::Exit), "{:?}", h.seen);
    assert!(h.app.fingerprints().is_empty(), "memory is cleared");
    assert!(!h.app.has_pin());
    assert!(says(&h, strings::EN.leave_title), "{:?}", h.app.texts());
    assert!(
        says(&h, strings::EN.leave_ended_keys),
        "{:?}",
        h.app.texts()
    );
    assert!(h.app.rect_of(ids::BACK).is_none(), "no way back");
}

#[test]
fn the_window_closes_by_itself_and_a_tap_elsewhere_closes_it_too() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.seen.clear();

    // Runs out.
    h.key(Key::Escape);
    assert!(says(&h, strings::EN.leave_again));
    h.tick(LEAVE_WINDOW_MS + 1);
    assert!(!says(&h, strings::EN.leave_again), "the line is gone");
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Home, "a first Back again");
    assert!(!h.seen.contains(&Command::Exit));

    // Leaving Home closes it: a Back on Keys is a Back on Keys.
    h.tap(ids::at(ids::HOME_TILE_BASE, 1));
    assert_eq!(h.app.screen(), ScreenKind::Keys);
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(!says(&h, strings::EN.leave_again));
    assert!(!h.seen.contains(&Command::Exit));
}

#[test]
fn back_twice_on_the_lock_screen_leaves_and_keeps_the_stored_key() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept());
    while h.app.screen() != ScreenKind::Home {
        h.tap(ids::BACK);
    }
    h.tap(ids::STATUS_LOCK);
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    h.seen.clear();

    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    assert!(says(&h, strings::EN.leave_again), "{:?}", h.app.texts());
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Ended);
    assert!(h.seen.contains(&Command::Exit), "{:?}", h.seen);
    assert!(
        !h.seen.contains(&Command::ForgetSecret),
        "the device's copy stays: {:?}",
        h.seen
    );
    assert!(
        h.element.as_ref().is_some_and(|e| e.blob.is_some()),
        "the blob is still on the device"
    );
}

#[test]
fn back_twice_on_the_stored_key_pad_leaves_and_keeps_the_stored_key() {
    let element = {
        let mut h = Harness::kept(SECURE_PHONE, Element::default());
        h.start_load(&ABANDON);
        h.finish_load(None);
        h.open_key(0);
        h.tap(ids::KEEP_ROW);
        h.hold(ids::KEEP_HOLD);
        h.element.clone().expect("an element")
    };
    // A restart: the pad is the first screen.
    let mut h = Harness::kept(SECURE_PHONE, element);
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.seen.clear();

    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    assert!(says(&h, strings::EN.leave_again), "{:?}", h.app.texts());
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Ended);
    assert!(h.seen.contains(&Command::Exit), "{:?}", h.seen);
    assert!(!h.seen.contains(&Command::ForgetSecret), "{:?}", h.seen);
    assert!(h.element.as_ref().is_some_and(|e| e.blob.is_some()));

    // And the next start still opens on the pad, with the PIN.
    let element = h.element.clone().expect("an element");
    let mut h = Harness::kept(SECURE_PHONE, element);
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    assert_eq!(h.app.fingerprints().len(), 1, "the kept key is back");
}
