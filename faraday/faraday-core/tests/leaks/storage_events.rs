//! What the shell hands the app through storage: the files kept across a
//! lock, restored with the boxes, and a file read off a stick. Each holds a
//! seed's words, and the bytes must be freed wiped once the app has taken
//! the event.

use crate::forms::{PHRASE, seed_forms};
use crate::{arm, finish};
use faraday_core::{StorageEvent, testkit};
use faraday_shell_api::App;
use faraday_text::Text;

#[test]
fn kept_files_restored_with_a_seed_leave_no_copy() {
    let forms = seed_forms("");
    let kept = faraday_core::kept_from(vec![("kept-seed.txt", PHRASE.as_bytes().to_vec())]);
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    let _ = app.frame();
    finish("kept files restored", armed, false);
}

#[test]
fn a_file_read_off_a_stick_holding_a_seed_leaves_no_copy() {
    let forms = seed_forms("");
    let bytes = PHRASE.as_bytes().to_vec();
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.storage(StorageEvent::Read {
        stick: Text::of("stick"),
        name: Text::of("seed.txt"),
        bytes: bytes.into(),
    });
    let _ = app.frame();
    finish("file read through storage", armed, false);
}
