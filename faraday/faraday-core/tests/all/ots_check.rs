//! Check a timestamp in Tools (`docs/decisions/F230.md`): a proof, the
//! file it stamps and a block header give a verdict, and each thing that
//! is not counted is named.

use crate::common::{has, has_part};
use faraday_core::ots_check::OtsAction as O;
use faraday_core::{Action, Faraday, StorageEvent, testkit};

const V: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/vectors/opentimestamps"
);

fn vector(path: &str) -> Vec<u8> {
    std::fs::read(format!("{V}/{path}")).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The tool open with a proof, a file and a header in Files, each taken.
fn checked(proof: &str, file: Vec<u8>, header: &str) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("stamp.ots".to_string(), vector(proof)),
            ("stamped".to_string(), file),
            ("header".to_string(), vector(header)),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(faraday_core::Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Check a timestamp")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::OtsCheck(O::Proof(0)));
    app.press(Action::OtsCheck(O::File(1)));
    app.press(Action::OtsCheck(O::Header(2)));
    app
}

#[test]
fn a_proof_through_sha256_is_committed_by_the_header_of_its_block() {
    let mut app = checked(
        "examples/different-blockchains.txt.ots",
        vector("examples/different-blockchains.txt"),
        "headers/455605.hex",
    );
    assert!(has(&mut app, "The header commits to this file"));
    assert!(has(&mut app, "455605"));
    assert!(has(
        &mut app,
        "00000000000000000264b495946e8dccbb6e201e6f00d017a9eb1b82a6952c1e"
    ));
}

#[test]
fn a_changed_file_is_not_the_one_the_proof_states() {
    let mut app = checked(
        "examples/different-blockchains.txt.ots",
        b"another file".to_vec(),
        "headers/455605.hex",
    );
    assert!(has(&mut app, "This is not the file the proof states"));
    assert!(!has(&mut app, "The header commits to this file"));
}

#[test]
fn a_proof_with_only_a_pending_attestation_names_its_calendar() {
    let mut app = checked(
        "examples/incomplete.txt.ots",
        vector("examples/incomplete.txt"),
        "headers/455605.hex",
    );
    assert!(has(&mut app, "The header does not commit to this file"));
    assert!(has_part(&mut app, "Pending at https://"));
}

#[test]
fn a_header_of_another_block_is_said_not_to_match() {
    let mut app = checked(
        "examples/different-blockchains.txt.ots",
        vector("examples/different-blockchains.txt"),
        "headers/358391.hex",
    );
    assert!(has(&mut app, "The header does not commit to this file"));
    assert!(has_part(&mut app, "not this header"));
}

#[test]
fn a_path_through_ripemd160_is_named_and_not_counted() {
    let mut app = checked(
        "examples/hello-world.txt.ots",
        vector("examples/hello-world.txt"),
        "headers/358391.hex",
    );
    assert!(has(&mut app, "The header does not commit to this file"));
    assert!(has_part(&mut app, "RIPEMD-160"));
}

#[test]
fn a_file_that_is_no_header_is_refused_in_a_line() {
    let mut app = checked(
        "examples/different-blockchains.txt.ots",
        vector("examples/different-blockchains.txt"),
        "examples/different-blockchains.txt",
    );
    assert!(has(&mut app, "Not an 80-byte block header or its hex"));
}
