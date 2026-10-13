//! BitLocker keys (`docs/COMPANION.md` §4.6, §7, §11): the hub app's key
//! text is kept, after the phone's unlock, as the computer, the drive and
//! the key it was written with; its digits show only while held; Delete
//! the key, from Back in Windows or from Delete, leaves no key in the
//! store; a key is never offered for Share, Save or a code, and never
//! reaches Received.

mod common;

use common::*;
use faraday_companion::kept;
use faraday_companion::{Act, Companion, CompanionEvent, Page};
use faraday_strings::EN;
use osk_shell_api::{App, Event, TouchPhase};

/// The phone with the hub app's key pasted on Receive and kept, on the
/// key's record.
fn keeping() -> Companion {
    let mut c = receiving();
    paste(&mut c, &hub_key().write().expect("the hub app's text"));
    let (_, ours) = drain(&mut c);
    assert!(asked_unlock(&ours), "no unlock asked: {ours:?}");
    c.companion_event(CompanionEvent::Kept { bytes: Vec::new() });
    assert_eq!(c.page, Page::Key(0));
    c
}

fn keys_in(bytes: &[u8]) -> usize {
    kept::read(bytes).expect("a store it reads").keys.len()
}

#[test]
fn the_hub_apps_key_text_is_kept_as_the_computer_drive_and_key_it_was_written_with() {
    let mut c = keeping();
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_store(&ours), None, "a key reached Received");
    let (bytes, n, k) = kept_kept(&ours).expect("the kept store written");
    assert_eq!((n, k), (0, 1));
    let store = kept::read(&bytes).expect("a store it reads");
    assert_eq!(store.keys[0].text, hub_key());
    assert!(c.received().is_empty());

    // Scanned the same, it is the same key, kept once.
    c.press(Act::Back);
    c.press(Act::Receive);
    show(
        &mut c,
        &code_of(hub_key().write().expect("text").as_bytes()),
    );
    assert_eq!(c.page, Page::Key(0));
    assert_eq!(c.kept().map(|k| k.keys.len()), Some(1));
}

#[test]
fn the_key_is_masked_and_shows_only_while_held() {
    let mut c = keeping();
    let digits = "205568";
    assert!(shows(&mut c, "DESKTOP-4K2Q"));
    assert!(!shows(&mut c, digits), "shown unmasked: {:?}", c.texts());
    let (x, y) = c.where_offered(Act::Hold).expect("Hold to show");
    c.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    assert!(shows(&mut c, digits), "{:?}", c.texts());
    c.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    assert!(!shows(&mut c, digits));
}

#[test]
fn back_in_windows_then_delete_the_key_leaves_no_key_in_the_store() {
    let mut c = keeping();
    drain(&mut c);
    c.press(Act::BackInWindows);
    assert_eq!(c.page, Page::BackInWindows(0));
    for line in [
        EN.companion_delete_key_ask,
        EN.companion_key_not_needed,
        EN.companion_key_shown_again,
        EN.companion_delete_the_key,
        EN.companion_keep_it,
    ] {
        assert!(shows(&mut c, line), "{line}: {:?}", c.texts());
    }
    // Keep it closes the sheet and keeps the key.
    c.press(Act::KeepIt);
    assert_eq!(c.page, Page::Key(0));
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_kept(&ours), None);
    c.press(Act::BackInWindows);
    c.press(Act::ConfirmDelete);
    let (_, ours) = drain(&mut c);
    let (bytes, _, keys) = kept_kept(&ours).expect("the kept store written");
    assert_eq!((keys, keys_in(&bytes)), (0, 0));
    assert!(!matches!(c.page, Page::Key(_) | Page::BackInWindows(_)));
}

#[test]
fn delete_asks_once_without_the_two_lines_and_leaves_no_key() {
    let mut c = keeping();
    drain(&mut c);
    c.press(Act::Delete);
    assert!(shows(&mut c, EN.companion_delete_key_ask));
    assert!(!shows(&mut c, EN.companion_key_not_needed));
    assert!(!shows(&mut c, EN.companion_key_shown_again));
    c.press(Act::ConfirmDelete);
    let (_, ours) = drain(&mut c);
    let (bytes, _, keys) = kept_kept(&ours).expect("the kept store written");
    assert_eq!((keys, keys_in(&bytes)), (0, 0));
}

#[test]
fn a_key_is_never_offered_for_share_save_or_a_code() {
    let mut c = keeping();
    drain(&mut c);
    for act in [Act::Share, Act::Save, Act::ShowAsQr] {
        assert!(!c.offers(act), "{act:?} offered for a key");
        c.press(act);
        let (_, ours) = drain(&mut c);
        assert!(ours.is_empty(), "{act:?}: {ours:?}");
        assert!(c.sending.is_none());
    }
    // Send to Faraday lists Received only, which holds no key.
    c.press(Act::Back);
    c.press(Act::Send);
    assert!(!c.offers(Act::SendKept(0)));
}

#[test]
fn homes_bitlocker_row_shows_only_while_a_key_is_kept() {
    let mut c = phone();
    assert!(!shows(&mut c, EN.companion_home_bitlocker));
    assert!(!c.offers(Act::Keys));
    let mut c = keeping();
    c.press(Act::Back);
    assert_eq!(c.page, Page::Home);
    assert!(shows(&mut c, EN.companion_home_bitlocker));
    c.press(Act::Keys);
    assert_eq!(c.page, Page::Keys);
    c.press(Act::KeyRecord(0));
    c.press(Act::Delete);
    c.press(Act::ConfirmDelete);
    c.press(Act::Back);
    assert_eq!(c.page, Page::Home);
    assert!(!shows(&mut c, EN.companion_home_bitlocker));
}
