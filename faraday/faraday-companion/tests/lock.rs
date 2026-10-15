//! The kept store behind the phone's unlock (`docs/COMPANION.md` §7,
//! §11): while it is locked no kept page opens and nothing is written;
//! the app going to the background leaves nothing of it in the core and
//! asks again next time; a phone with no screen lock keeps nothing and
//! says so, and Received works as before.

mod common;

use common::*;
use faraday_companion::route::Refusal;
use faraday_companion::{Act, Closed, CompanionEvent, Page};
use faraday_strings::EN;

#[test]
fn while_locked_no_kept_page_opens_and_nothing_is_written() {
    let mut c = phone();
    c.companion_event(CompanionEvent::KeptCounts {
        version: 2,
        descriptions: 2,
    });
    // Home shows the count the shell keeps beside the store.
    assert_eq!(c.kept_count(), 2);
    c.press(Act::Descriptions);
    assert!(matches!(c.page, Page::Unlocking(_)));
    // A kept page's own acts do nothing before the unlock.
    for inner in [Act::DescriptionRecord(0), Act::AddDescription] {
        c.press(inner);
        assert!(matches!(c.page, Page::Unlocking(_)), "{inner:?}");
    }
    c.companion_event(CompanionEvent::KeptClosed {
        why: Closed::Cancelled,
    });
    assert_eq!(c.page, Page::Closed(Closed::Cancelled));
    assert!(c.kept().is_none());
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_kept(&ours), None);
    c.press(Act::Done);
    // A recovery key that arrives is refused outright: no unlock is
    // asked, and nothing is kept anywhere.
    c.press(Act::Receive);
    paste(&mut c, &starter_key_text());
    assert_eq!(c.page, Page::Refused(Refusal::RecoveryKey));
    let (_, ours) = drain(&mut c);
    assert!(!asked_unlock(&ours));
    assert_eq!(kept_kept(&ours), None);
    assert_eq!(kept_store(&ours), None);
    assert!(c.received().is_empty());
    // An answer nobody asked for opens nothing.
    c.companion_event(CompanionEvent::Kept { bytes: Vec::new() });
    assert!(c.kept().is_none());
}

#[test]
fn going_to_the_background_leaves_nothing_of_the_store_in_the_core() {
    let mut c = on_descriptions(&[]);
    c.press(Act::AddDescription);
    paste(&mut c, &description());
    assert_eq!(c.page, Page::Description(0));
    c.companion_event(CompanionEvent::Locked);
    assert!(c.kept().is_none());
    assert_eq!(c.page, Page::Home);
    // Home still counts it; opening it asks again.
    assert_eq!(c.kept_count(), 1);
    drain(&mut c);
    c.press(Act::Descriptions);
    let (_, ours) = drain(&mut c);
    assert!(asked_unlock(&ours));
    assert!(c.kept().is_none());
    // A page that is not a kept one stays where it is.
    let mut c = on_descriptions(&[]);
    c.press(Act::Back);
    c.press(Act::Received);
    c.companion_event(CompanionEvent::Locked);
    assert_eq!(c.page, Page::Received);
    assert!(c.kept().is_none());
}

#[test]
fn with_no_screen_lock_nothing_is_kept_it_says_so_and_received_still_works() {
    let mut c = phone();
    c.press(Act::Descriptions);
    c.companion_event(CompanionEvent::KeptClosed {
        why: Closed::NoScreenLock,
    });
    assert!(
        shows(&mut c, EN.companion_closed_no_lock),
        "{:?}",
        c.texts()
    );
    c.press(Act::Done);
    // A description shared in: nothing kept.
    c.companion_event(CompanionEvent::SharedIn {
        name: None,
        bytes: description().into_bytes(),
    });
    c.companion_event(CompanionEvent::KeptClosed {
        why: Closed::NoScreenLock,
    });
    assert_eq!(c.page, Page::Closed(Closed::NoScreenLock));
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_kept(&ours), None);
    assert!(c.kept().is_none());
    // Received, as before.
    c.press(Act::Done);
    c.press(Act::Receive);
    show(
        &mut c,
        &code_of(osk_codec::ur::encode_psbt(&psbt()).as_bytes()),
    );
    assert!(matches!(c.page, Page::Arrived { again: false, .. }));
    assert_eq!(c.received().len(), 1);
}

#[test]
fn a_kept_store_it_cannot_read_is_not_written_until_deleted() {
    let mut c = phone();
    c.press(Act::Descriptions);
    let _ = unlock(&mut c, b"FCKS\x09");
    assert_eq!(c.page, Page::Closed(Closed::Unreadable));
    assert!(c.kept().is_none());
    c.press(Act::Done);
    // A description kept from Received asks the unlock again, finds the
    // store unreadable and writes nothing.
    c.press(Act::Receive);
    paste(&mut c, &description());
    c.press(Act::Open);
    c.press(Act::KeepInDescriptions);
    let _ = unlock(&mut c, b"FCKS\x09");
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_kept(&ours), None);
    c.press(Act::Done);
    c.press(Act::DeleteKeptStore);
    c.press(Act::ConfirmDelete);
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_kept(&ours), Some((Vec::new(), 0)));
}

#[test]
fn a_version_1_store_is_deleted_at_the_first_start() {
    // Version 1 held BitLocker keys beside the descriptions (F33): the
    // shell reports what it recorded, and the core has it deleted, with
    // its count, before anything is unlocked.
    let mut c = phone();
    c.companion_event(CompanionEvent::KeptCounts {
        version: 1,
        descriptions: 3,
    });
    assert_eq!(c.kept_count(), 0);
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_kept(&ours), Some((Vec::new(), 0)));
    assert!(!asked_unlock(&ours));

    // One that reaches the unlock anyway is a store it cannot read.
    c.press(Act::Descriptions);
    let _ = unlock(&mut c, b"FCKS\x01\x00\x00\x00\x00\x00\x00\x00\x00");
    assert_eq!(c.page, Page::Closed(Closed::Unreadable));
    assert!(c.kept().is_none());
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_kept(&ours), None);

    // The current version, or nothing kept, deletes nothing.
    let mut c = phone();
    c.companion_event(CompanionEvent::KeptCounts {
        version: 2,
        descriptions: 0,
    });
    let (_, ours) = drain(&mut c);
    assert_eq!(kept_kept(&ours), None);
}
