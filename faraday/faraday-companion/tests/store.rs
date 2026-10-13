//! Received and its store (`docs/COMPANION.md` §4.4, §7, §11): what the
//! core hands the shell to keep comes back as the same files; a store it
//! cannot read is left alone until the person deletes it; Delete and
//! Delete all leave the store without the files.

mod common;

use common::*;
use faraday_companion::store::{self, Kind};
use faraday_companion::{Act, Companion, CompanionEvent, Page};

/// A phone that has received a PSBT, a description and a file from the
/// device, and the store it last handed the shell.
fn holding_three() -> (Companion, Vec<u8>) {
    let mut c = receiving();
    show(
        &mut c,
        &code_of(osk_codec::ur::encode_psbt(&psbt()).as_bytes()),
    );
    c.press(Act::ScanAgain);
    show(&mut c, &code_of(description().as_bytes()));
    c.press(Act::ScanAgain);
    let env = faraday_qr::envelope::pack("notes.txt", b"Meet at noon.\n", "file").expect("env");
    show(&mut c, &code_of(&env));
    assert!(matches!(c.page, Page::Arrived { again: false, .. }));
    c.press(Act::Back);
    let (_, ours) = drain(&mut c);
    let kept = kept_store(&ours).expect("the store");
    assert_eq!(c.received().len(), 3);
    (c, kept)
}

#[test]
fn the_store_written_and_read_back_gives_the_same_files() {
    let (c, kept) = holding_three();
    let mut again = phone();
    again.companion_event(CompanionEvent::Received {
        bytes: kept.clone(),
    });
    assert!(!again.unreadable);
    let a: Vec<_> = c.received().into_iter().cloned().collect();
    let b: Vec<_> = again.received().into_iter().cloned().collect();
    assert_eq!(a, b);
    assert_eq!(
        b.iter().map(|k| k.kind).collect::<Vec<_>>(),
        [Kind::File, Kind::Description, Kind::Psbt],
        "newest first"
    );
    assert!(b.iter().all(|k| k.time == 1_791_200_000));
    assert_eq!(b[0].name, "notes.txt");
    // Read back and written again, it is the same bytes.
    assert_eq!(again.store_bytes(), kept);
}

#[test]
fn a_store_it_cannot_read_is_left_alone_until_the_person_deletes_it() {
    let (_, kept) = holding_three();
    let mut damaged = kept.clone();
    let last = damaged.len() - 1;
    damaged[last] ^= 1;
    for unreadable in [damaged, b"FCRS\x09".to_vec(), b"not a store".to_vec()] {
        let mut c = phone();
        c.companion_event(CompanionEvent::Received {
            bytes: unreadable.clone(),
        });
        assert!(c.unreadable);
        // Something arrives: shown and kept here, but the shell's store
        // is not written over.
        c.press(Act::Receive);
        show(&mut c, &code_of(description().as_bytes()));
        assert!(matches!(c.page, Page::Arrived { .. }));
        let (_, ours) = drain(&mut c);
        assert_eq!(kept_store(&ours), None, "the store was written over");
        // Received says so, and offers the deletion.
        c.press(Act::Back);
        c.press(Act::Received);
        assert!(
            c.texts()
                .iter()
                .any(|t| t == faraday_strings::EN.companion_unreadable_value)
        );
        c.press(Act::DeleteStore);
        c.press(Act::KeepIt);
        let (_, ours) = drain(&mut c);
        assert_eq!(kept_store(&ours), None);
        c.press(Act::DeleteStore);
        c.press(Act::ConfirmDelete);
        let (_, ours) = drain(&mut c);
        let written = kept_store(&ours).expect("the store, once deleted");
        let files = store::read(&written).expect("a store this build reads");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].bytes, description().as_bytes());
        assert!(!c.unreadable);
    }
}

#[test]
fn delete_leaves_the_store_without_the_file() {
    let (mut c, _) = holding_three();
    c.press(Act::Received);
    let gone = c.received()[1].sha256;
    c.press(Act::Record(1));
    assert_eq!(c.page, Page::Record(gone));
    c.press(Act::Delete);
    c.press(Act::ConfirmDelete);
    assert_eq!(c.page, Page::Received);
    let (_, ours) = drain(&mut c);
    let files = store::read(&kept_store(&ours).expect("the store")).expect("readable");
    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|f| f.sha256 != gone));
    assert!(c.received().iter().all(|f| f.sha256 != gone));
}

#[test]
fn delete_all_leaves_an_empty_store() {
    let (mut c, _) = holding_three();
    c.press(Act::Received);
    c.press(Act::DeleteAll);
    c.press(Act::ConfirmDelete);
    assert_eq!(c.page, Page::Received);
    let (_, ours) = drain(&mut c);
    let files = store::read(&kept_store(&ours).expect("the store")).expect("readable");
    assert!(files.is_empty());
    assert!(c.received().is_empty());
    assert!(!c.offers(Act::DeleteAll));
}

#[test]
fn save_hands_the_file_to_the_document_creator() {
    let (mut c, _) = holding_three();
    c.press(Act::Received);
    c.press(Act::Record(2));
    c.press(Act::Save);
    let (cmds, _) = drain(&mut c);
    assert!(cmds.contains(&osk_shell_api::Command::WriteFile {
        kind: osk_shell_api::FileKind::Psbt,
        name_hint: "received-1.psbt".into(),
        bytes: psbt(),
    }));
}
