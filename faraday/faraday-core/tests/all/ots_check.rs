//! Check a timestamp in Tools (`docs/decisions/F230.md`): a proof, the
//! file it stamps and a block header give a verdict, and each thing that
//! is not counted is named.

use crate::common::{has, has_part};
use faraday_core::ots_check::OtsAction as O;
use faraday_core::{Action, Faraday, StorageEvent, testkit};
use faraday_shell_api::{App, Event};

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
    assert!(has(
        &mut app,
        "No path through SHA-256 or RIPEMD-160 reaches this header"
    ));
    assert!(has_part(&mut app, "Pending at https://"));
}

#[test]
fn a_header_of_another_block_is_said_not_to_match() {
    let mut app = checked(
        "examples/different-blockchains.txt.ots",
        vector("examples/different-blockchains.txt"),
        "headers/358391.hex",
    );
    assert!(has(
        &mut app,
        "No path through SHA-256 or RIPEMD-160 reaches this header"
    ));
    assert!(has_part(&mut app, "not this header"));
}

#[test]
fn a_path_through_ripemd160_is_counted() {
    let mut app = checked(
        "examples/hello-world.txt.ots",
        vector("examples/hello-world.txt"),
        "headers/358391.hex",
    );
    assert!(has(&mut app, "The header commits to this file"));
    assert!(!has(
        &mut app,
        "No path through SHA-256 or RIPEMD-160 reaches this header"
    ));
}

#[test]
fn a_path_through_sha1_is_named_and_not_counted() {
    let mut app = checked(
        "examples/sha1/a-or-b.ots",
        vector("examples/sha1/a"),
        "headers/466906.hex",
    );
    assert!(has(
        &mut app,
        "No path through SHA-256 or RIPEMD-160 reaches this header"
    ));
    assert!(has_part(&mut app, "SHA-1"));
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

/// The tool open on the proof of the different-blockchains example, with
/// the header of its block taken; the file is for the test to give.
fn with_proof_and_header(extra: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut inbox = vec![
        (
            "stamp.ots".to_string(),
            vector("examples/different-blockchains.txt.ots"),
        ),
        ("header".to_string(), vector("headers/455605.hex")),
        (
            "bad.ots".to_string(),
            vector("examples/different-blockchains.txt.ots")[..40].to_vec(),
        ),
    ];
    inbox.extend(extra.into_iter().map(|(n, b)| (n.to_string(), b)));
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox,
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
    app.press(Action::OtsCheck(O::Header(1)));
    app
}

fn digest_text() -> String {
    faraday_crypto::sha256(&vector("examples/different-blockchains.txt"))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn a_refused_proof_clears_the_result_of_the_one_before() {
    let mut app = checked(
        "examples/different-blockchains.txt.ots",
        vector("examples/different-blockchains.txt"),
        "headers/455605.hex",
    );
    assert!(has(&mut app, "The header commits to this file"));
    let mut app2 =
        with_proof_and_header(vec![("file", vector("examples/different-blockchains.txt"))]);
    app2.press(Action::OtsCheck(O::File(3)));
    assert!(has(&mut app2, "The header commits to this file"));
    app2.press(Action::OtsCheck(O::Proof(2)));
    assert!(!has(&mut app2, "The header commits to this file"));
    assert!(has(&mut app2, "Not checked yet"));
}

#[test]
fn a_refused_file_hash_or_header_clears_the_result() {
    let mut app =
        with_proof_and_header(vec![("file", vector("examples/different-blockchains.txt"))]);
    app.press(Action::OtsCheck(O::File(3)));
    assert!(has(&mut app, "The header commits to this file"));
    // A refused header (the proof file is no header).
    app.press(Action::OtsCheck(O::Header(0)));
    assert!(!has(&mut app, "The header commits to this file"));
    // The right header again, then a typed hash that is refused.
    app.press(Action::OtsCheck(O::Header(1)));
    assert!(has(&mut app, "The header commits to this file"));
    app.press(Action::OtsCheck(O::Focus(
        faraday_core::ots_check::Field::Hash,
    )));
    for ch in "12zz".chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(ch)));
    }
    assert!(!has(&mut app, "The header commits to this file"));
}

#[test]
fn typing_in_a_field_clears_the_result_before_it_is_used() {
    let mut app =
        with_proof_and_header(vec![("file", vector("examples/different-blockchains.txt"))]);
    app.press(Action::OtsCheck(O::File(3)));
    assert!(has(&mut app, "The header commits to this file"));
    app.press(Action::OtsCheck(O::Focus(
        faraday_core::ots_check::Field::Header,
    )));
    app.event(Event::Key(faraday_shell_api::Key::Char('a')));
    assert!(!has(&mut app, "The header commits to this file"));
}

#[test]
fn eighty_typed_characters_are_hex_text_and_not_a_header() {
    let mut app =
        with_proof_and_header(vec![("file", vector("examples/different-blockchains.txt"))]);
    app.press(Action::OtsCheck(O::File(3)));
    app.press(Action::OtsCheck(O::Step(2)));
    app.press(Action::OtsCheck(O::Focus(
        faraday_core::ots_check::Field::Header,
    )));
    for ch in "ab".repeat(40).chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(ch)));
    }
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert!(has(&mut app, "Not an 80-byte block header or its hex"));
    // A sign or a space inside a pair is no hex either.
    app.press(Action::OtsCheck(O::Step(1)));
    app.press(Action::OtsCheck(O::Focus(
        faraday_core::ots_check::Field::Hash,
    )));
    for ch in format!("+{}", &digest_text()[1..]).chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(ch)));
    }
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert!(has(&mut app, "Not a SHA-256: 64 hex digits"));
}

#[test]
fn the_digest_picked_as_a_text_file_is_named_a_sha256_from_that_file() {
    let text = format!("  {}\n", digest_text());
    let mut app = with_proof_and_header(vec![("digest.txt", text.into_bytes())]);
    app.press(Action::OtsCheck(O::File(3)));
    assert!(has(&mut app, "The header commits to this SHA-256"));
    assert!(has(&mut app, "SHA-256 from digest.txt"));
    assert!(!has(&mut app, "The header commits to this file"));
}

#[test]
fn picking_a_whole_file_clears_an_older_typed_hash() {
    let mut app =
        with_proof_and_header(vec![("file", vector("examples/different-blockchains.txt"))]);
    app.press(Action::OtsCheck(O::Step(1)));
    app.press(Action::OtsCheck(O::Focus(
        faraday_core::ots_check::Field::Hash,
    )));
    for ch in "12ab".chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(ch)));
    }
    app.press(Action::OtsCheck(O::File(3)));
    assert!(has(&mut app, "The header commits to this file"));
    assert!(!has_part(&mut app, "12ab"));
}

#[test]
fn a_second_counted_path_has_its_own_line() {
    // The example's last node holds one Bitcoin attestation: it is there twice.
    let mut proof = vector("examples/different-blockchains.txt.ots");
    let at = proof.len() - 13;
    let att = proof[at..].to_vec();
    proof.truncate(at);
    proof.push(0xff);
    proof.extend(&att);
    proof.extend(&att);
    let mut app = with_proof_and_header(vec![
        ("two.ots", proof),
        ("file", vector("examples/different-blockchains.txt")),
    ]);
    app.press(Action::OtsCheck(O::Proof(3)));
    app.press(Action::OtsCheck(O::File(4)));
    assert!(has(&mut app, "The header commits to this file"));
    assert!(has_part(&mut app, "also counted"));
    assert!(!has_part(&mut app, "more"));
}

#[test]
fn another_hex_value_picked_as_a_text_file_is_not_the_file() {
    let text = "ab".repeat(32);
    let mut app = with_proof_and_header(vec![("other.txt", text.into_bytes())]);
    app.press(Action::OtsCheck(O::File(3)));
    assert!(has(&mut app, "This is not the file the proof states"));
}

#[test]
fn a_proof_of_many_attestations_lists_some_and_counts_the_rest() {
    // Unknown attestations of 8 bytes of tag each, side by side.
    let magic = vector("examples/different-blockchains.txt.ots");
    let mut proof = magic[..31].to_vec();
    proof.extend([1, 0x08]);
    let digest = faraday_crypto::sha256(b"x");
    proof.extend(digest);
    for i in 0..100u8 {
        if i < 99 {
            proof.push(0xff);
        }
        proof.push(0x00);
        proof.extend([9, 9, 9, 9, 9, 9, 9, i]);
        proof.push(0);
    }
    let mut app = with_proof_and_header(vec![("many.ots", proof), ("x", b"x".to_vec())]);
    app.press(Action::OtsCheck(O::Proof(3)));
    app.press(Action::OtsCheck(O::File(4)));
    assert!(has_part(&mut app, "Unknown attestation"));
    assert!(has(&mut app, "and 88 more"));
}

fn scan_hash(app: &mut Faraday, text: &str) {
    app.press(Action::OtsCheck(O::Step(1)));
    app.press(Action::OtsCheck(O::ScanHash));
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
}

#[test]
fn a_scanned_sha256_fills_the_hash_and_the_check_matches() {
    let mut app = with_proof_and_header(vec![]);
    scan_hash(&mut app, &format!("{}\n", digest_text()));
    assert!(has(&mut app, "The header commits to this file"));
    assert!(has(&mut app, "Scanned"));
}

#[test]
fn a_refused_hash_scan_drops_the_hash_and_the_result() {
    let mut app = with_proof_and_header(vec![]);
    scan_hash(&mut app, &digest_text());
    assert!(has(&mut app, "The header commits to this file"));
    // An older refusal of a typed hash goes with it.
    app.press(Action::OtsCheck(O::Step(1)));
    app.press(Action::OtsCheck(O::Focus(
        faraday_core::ots_check::Field::Hash,
    )));
    app.event(Event::Key(faraday_shell_api::Key::Char('z')));
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert!(has(&mut app, "Not a SHA-256: 64 hex digits"));
    scan_hash(&mut app, "not a hash");
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    app.press(Action::OtsCheck(O::Step(1)));
    assert!(!has(&mut app, "Not a SHA-256: 64 hex digits"));
    app.press(Action::OtsCheck(O::Step(3)));
    assert!(!has(&mut app, "The header commits to this file"));
    assert!(has(&mut app, "Not checked yet"));
}

#[test]
fn a_scanned_text_that_is_no_sha256_is_refused() {
    let mut app = with_proof_and_header(vec![]);
    scan_hash(&mut app, "not a hash");
    assert!(has(&mut app, "Not a SHA-256: 64 hex digits"));
    assert!(!has(&mut app, "The header commits to this file"));
}

#[test]
fn a_header_from_a_file_clears_an_older_typed_header() {
    let mut app =
        with_proof_and_header(vec![("file", vector("examples/different-blockchains.txt"))]);
    app.press(Action::OtsCheck(O::Step(2)));
    app.press(Action::OtsCheck(O::Focus(
        faraday_core::ots_check::Field::Header,
    )));
    for ch in "12ab".chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(ch)));
    }
    app.press(Action::OtsCheck(O::Header(1)));
    assert!(!has_part(&mut app, "12ab"));
}

/// A proof of a pending attestation alone, for a file hashed under `op`.
fn pending_proof(op: faraday_ots::HashOp, file: &[u8]) -> Vec<u8> {
    let mut proof = vector("examples/incomplete.txt.ots")[..32].to_vec();
    proof.push(match op {
        faraday_ots::HashOp::Sha1 => 0x02,
        _ => 0x08,
    });
    proof.extend(op.digest(file));
    proof.extend(b"\x00\x83\xdf\xe3\x0d\x2e\xf9\x0c\x8e\x12\x11https://a.example");
    proof
}

/// The tool open on a header in Files, with the proof at 0 and the file at
/// 1, none taken.
fn opened(proof: Vec<u8>, file: Vec<u8>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("stamp.ots".to_string(), proof),
            ("stamped".to_string(), file),
            ("header".to_string(), vector("headers/455605.hex")),
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
    app
}

fn says_pending(app: &mut Faraday) -> bool {
    has(
        app,
        "No path through SHA-256 or RIPEMD-160 reaches this header",
    ) && has_part(app, "Pending at https://")
}

#[test]
fn a_large_file_reaches_the_verdict_a_small_one_does() {
    for size in [1 << 10, 4 << 20] {
        let file = vec![0x5au8; size];
        let mut app = opened(pending_proof(faraday_ots::HashOp::Sha256, &file), file);
        app.press(Action::OtsCheck(O::Proof(0)));
        app.press(Action::OtsCheck(O::Header(2)));
        testkit::press_on_worker(&mut app, Action::OtsCheck(O::File(1)));
        if size > 1 << 20 {
            assert_eq!(
                app.busy().map(|b| b.line),
                Some(faraday_core::strings::EN.working_hashing),
                "the screen says the file is hashed"
            );
            assert!(!has(
                &mut app,
                "No path through SHA-256 or RIPEMD-160 reaches this header"
            ));
            testkit::settle(&mut app);
        }
        assert!(app.busy().is_none());
        assert!(says_pending(&mut app), "{size} bytes");
    }
}

#[test]
fn a_file_taken_before_a_proof_naming_another_hash_is_hashed_when_the_proof_comes() {
    for size in [1 << 10, 4 << 20] {
        let file = vec![0xa5u8; size];
        let mut app = opened(pending_proof(faraday_ots::HashOp::Sha1, &file), file);
        app.press(Action::OtsCheck(O::Header(2)));
        // SHA-256 alone is taken first, as no proof is in.
        app.press(Action::OtsCheck(O::File(1)));
        testkit::press_on_worker(&mut app, Action::OtsCheck(O::Proof(0)));
        if size > 1 << 20 {
            assert!(app.busy().is_some(), "the file is hashed on the worker");
            testkit::settle(&mut app);
        }
        assert!(app.busy().is_none());
        assert!(
            !has(&mut app, "This is not the file the proof states"),
            "{size} bytes"
        );
        assert!(says_pending(&mut app), "{size} bytes");
    }
}
