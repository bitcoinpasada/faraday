//! The QR vault tool in Tools (`docs/VAULT.md` §13): Make seals a typed
//! note or a loaded key's words, the code goes to the Outbox as the
//! `.pgp` file, and leaving the tool or a lock wipes what was typed.

use faraday_core::qr_tool::{self, Pick, QrToolAction as Q};
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};

use crate::common::{has, has_part, offered, texts};
use faraday_crypto::work::Compute;
use faraday_keys::{Keys, QrError, QrHolds, QrOpenJob, Reveal};
use faraday_shell_api::{App, EntropyBytes, Event, Key};

const WORDS12: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn started() -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app
}

fn open(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "QR vault")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Types the passphrase twice and the name.
fn passphrase(app: &mut Faraday, pass: &str, name: &str) {
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Pass)));
    type_in(app, pass);
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Again)));
    type_in(app, pass);
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Name)));
    type_in(app, name);
}

/// A note typed and sealed under `pass`, named `name`.
fn make_note(app: &mut Faraday, note: &str, pass: &str, name: &str) {
    open(app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(app, note);
    app.press(Action::QrTool(Q::Next));
    passphrase(app, pass, name);
    app.press(Action::QrTool(Q::Make));
    testkit::settle(app);
}

/// A code opened under `pass` and held, as the tool does it: what was held
/// goes first, so a refusal leaves none.
fn open_code(keys: &mut Keys, c: &Compute, pass: &str, code: &[u8]) -> Result<QrHolds, QrError> {
    keys.qr_forget();
    let opened = QrOpenJob::new(code, pass).run(c)?;
    Ok(keys.qr_hold(opened))
}

#[test]
fn the_tile_opens_the_tool() {
    let mut app = started();
    open(&mut app);
    assert_eq!(app.screen, Screen::QrTool);
    assert!(qr_tool::state(&app).is_some());
}

#[test]
fn a_note_typed_and_sealed_opens_back_with_the_same_text() {
    let mut app = started();
    make_note(&mut app, "meet at the old oak", "correct horse", "");
    assert!(qr_tool::state(&app).unwrap().is_made());
    app.press(Action::QrTool(Q::File));
    let item = app
        .outbox
        .iter()
        .find(|i| i.name == "qr-vault.pgp")
        .expect("the code is in the Outbox");
    let c = testkit::compute();
    let mut keys = Keys::default();
    let holds = open_code(&mut keys, &c, "correct horse", &item.bytes).unwrap();
    assert_eq!(holds, QrHolds::Note);
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "meet at the old oak");
}

#[test]
fn a_named_code_goes_to_the_outbox_under_its_name() {
    let mut app = started();
    make_note(&mut app, "a note", "pass", "Spare");
    app.press(Action::QrTool(Q::File));
    let stem = qr_tool::state(&app).unwrap().stem();
    assert_ne!(stem, "qr-vault");
    let item = app
        .outbox
        .iter()
        .find(|i| i.name == format!("{stem}.pgp"))
        .unwrap();
    assert_eq!(item.bytes.len(), 396);
}

#[test]
fn the_room_left_is_counted_as_typed() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    let empty = qr_tool::state(&app).unwrap().room_left();
    type_in(&mut app, "hello");
    let s = qr_tool::state(&app).unwrap();
    assert_eq!(s.note_chars(), 5);
    assert_eq!(s.room_left(), empty - 5);
    app.event(Event::Key(Key::Backspace));
    assert_eq!(qr_tool::state(&app).unwrap().room_left(), empty - 4);
}

#[test]
fn typing_stops_at_the_limit() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    let room = qr_tool::state(&app).unwrap().room_left();
    type_in(&mut app, &"a".repeat(room + 20));
    let s = qr_tool::state(&app).unwrap();
    assert_eq!(s.room_left(), 0);
    assert_eq!(s.note_chars(), room);
}

#[test]
fn a_key_not_of_12_or_24_words_cannot_be_picked() {
    let mut app = started();
    let c = testkit::compute();
    let fifteen = format!("{} address", vec!["abandon"; 14].join(" "));
    app.session
        .keys
        .add_words(&c, &fifteen, "Odd", None)
        .unwrap();
    app.session
        .keys
        .add_words(&c, WORDS12, "Twelve", None)
        .unwrap();
    assert!(!app.session.keys.qr_key_offered(0));
    assert!(app.session.keys.qr_key_offered(1));
    open(&mut app);
    app.press(Action::QrTool(Q::Key(0)));
    let s = qr_tool::state(&app).unwrap();
    assert_eq!(s.key(), None);
    app.press(Action::QrTool(Q::Key(1)));
    assert_eq!(qr_tool::state(&app).unwrap().key(), Some(1));
}

#[test]
fn a_loaded_key_is_sealed_and_opens_back_as_a_seed() {
    let mut app = started();
    let c = testkit::compute();
    app.session
        .keys
        .add_words(&c, WORDS12, "Twelve", None)
        .unwrap();
    open(&mut app);
    app.press(Action::QrTool(Q::Key(0)));
    passphrase(&mut app, "a phrase", "");
    app.press(Action::QrTool(Q::Make));
    testkit::settle(&mut app);
    app.press(Action::QrTool(Q::File));
    let item = app
        .outbox
        .iter()
        .find(|i| i.name == "qr-vault.pgp")
        .unwrap();
    let mut keys = Keys::default();
    let holds = open_code(&mut keys, &c, "a phrase", &item.bytes).unwrap();
    assert_eq!(holds, QrHolds::Seed { words: 12 });
}

#[test]
fn the_passphrase_must_match_and_not_be_empty() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(&mut app, "note");
    app.press(Action::QrTool(Q::Next));
    assert!(!qr_tool::state(&app).unwrap().passphrase_ready());
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Pass)));
    type_in(&mut app, "one");
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Again)));
    type_in(&mut app, "two");
    assert!(!qr_tool::state(&app).unwrap().passphrase_ready());
    app.press(Action::QrTool(Q::Make));
    testkit::settle(&mut app);
    assert!(!qr_tool::state(&app).unwrap().is_made());
    for _ in 0..3 {
        app.event(Event::Key(Key::Backspace));
    }
    type_in(&mut app, "one");
    assert!(qr_tool::state(&app).unwrap().passphrase_ready());
}

#[test]
fn the_files_put_in_the_outbox_are_public() {
    let mut app = started();
    make_note(&mut app, "a note", "pass", "");
    // A secret reaching the Outbox trips put_outbox's debug assert.
    app.press(Action::QrTool(Q::File));
    app.press(Action::QrTool(Q::Png));
    assert!(
        app.outbox
            .iter()
            .any(|i| i.name == "qr-vault.pgp" && !i.secret)
    );
    assert!(app.outbox.iter().any(|i| i.name.ends_with(".png")));
}

#[test]
fn make_another_empties_the_flow() {
    let mut app = started();
    make_note(&mut app, "a note", "pass", "Name");
    app.press(Action::QrTool(Q::Another));
    let s = qr_tool::state(&app).unwrap();
    assert!(!s.is_made());
    assert_eq!(s.pick(), None);
    assert_eq!(s.note_chars(), 0);
    assert!(!s.passphrase_ready());
    assert_eq!(s.stem(), "qr-vault");
}

#[test]
fn leaving_wipes_what_was_typed() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(&mut app, "a note");
    app.press(Action::Nav(Screen::Home));
    assert!(qr_tool::state(&app).is_none());
    open(&mut app);
    assert_eq!(qr_tool::state(&app).unwrap().note_chars(), 0);
}

#[test]
fn a_lock_wipes_what_was_typed() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(&mut app, "a note");
    app.press(Action::Lock);
    assert!(qr_tool::state(&app).is_none());
}

/// A note typed and its passphrase typed, the seal not yet pressed.
fn ready_to_make(app: &mut Faraday) {
    open(app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(app, "meet at the old oak");
    app.press(Action::QrTool(Q::Next));
    passphrase(app, "correct horse", "");
}

#[test]
fn a_note_sealed_on_the_worker_shows_progress_then_opens_back() {
    let mut app = started();
    ready_to_make(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    assert!(app.busy().is_some(), "the screen shows Progress");
    assert!(!qr_tool::state(&app).unwrap().is_made());
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(app.busy().is_none());
    assert!(qr_tool::state(&app).unwrap().is_made());
    app.press(Action::QrTool(Q::File));
    let item = app
        .outbox
        .iter()
        .find(|i| i.name == "qr-vault.pgp")
        .expect("the code is in the Outbox");
    let c = testkit::compute();
    let mut keys = Keys::default();
    assert_eq!(
        open_code(&mut keys, &c, "correct horse", &item.bytes),
        Ok(QrHolds::Note)
    );
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "meet at the old oak");
}

/// The note typed again and its passphrase seal a code that opens to it.
fn type_and_seal_again(app: &mut Faraday) {
    type_in(app, "meet at the old oak");
    app.press(Action::QrTool(Q::Next));
    passphrase(app, "correct horse", "");
    app.press(Action::QrTool(Q::Make));
    testkit::settle(app);
    assert!(qr_tool::state(app).unwrap().is_made());
    app.press(Action::QrTool(Q::File));
    let item = app.outbox.first().expect("the code is in the Outbox");
    let c = testkit::compute();
    let mut keys = Keys::default();
    assert_eq!(
        open_code(&mut keys, &c, "correct horse", &item.bytes),
        Ok(QrHolds::Note)
    );
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "meet at the old oak");
}

/// Make pressed with no note typed seals nothing.
fn make_seals_no_empty_note(app: &mut Faraday) {
    app.press(Action::QrTool(Q::Make));
    testkit::settle(app);
    assert!(app.busy().is_none());
    assert!(!qr_tool::state(app).unwrap().is_made());
    assert!(app.outbox.is_empty());
}

#[test]
fn cancel_while_the_code_is_sealed_asks_for_the_note_again() {
    let mut app = started();
    ready_to_make(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    app.press(Action::Cancel);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    let s = qr_tool::state(&app).expect("the tool stays open");
    assert!(!s.is_made(), "no code was made");
    assert!(!s.passphrase_ready());
    assert_eq!(s.note_chars(), 0);
    assert_eq!(s.focus(), Some(qr_tool::Field::Note), "Contents is open");
    assert!(app.outbox.is_empty());
    make_seals_no_empty_note(&mut app);
    type_and_seal_again(&mut app);
}

#[test]
fn back_while_the_code_is_sealed_leaves_the_tool_and_a_new_one_starts_empty() {
    let mut app = started();
    ready_to_make(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    app.press(Action::Back);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(qr_tool::state(&app).is_none(), "nothing typed is kept");
    assert!(app.outbox.is_empty());
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    assert_eq!(qr_tool::state(&app).unwrap().note_chars(), 0);
    make_seals_no_empty_note(&mut app);
    type_and_seal_again(&mut app);
}

#[test]
fn a_lock_while_the_code_is_sealed_makes_nothing_and_keeps_nothing_typed() {
    let mut app = started();
    ready_to_make(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    app.press(Action::Lock);
    assert!(app.busy().is_none());
    assert!(qr_tool::state(&app).is_none(), "nothing typed is kept");
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(qr_tool::state(&app).is_none());
    assert!(app.outbox.is_empty());
}

const EMPTY: &[u8] = include_bytes!("../../../../tools/vectors/vault/qr/qr-empty.pgp");
const VECTOR_PASS: &str = "correct horse battery staple";

/// A note sealed under `pass`, its bytes as the Outbox has them.
fn sealed_note(note: &str, pass: &str) -> Vec<u8> {
    let mut app = started();
    make_note(&mut app, note, pass, "");
    app.press(Action::QrTool(Q::File));
    app.outbox
        .iter()
        .find(|i| i.name == "qr-vault.pgp")
        .expect("the code is in the Outbox")
        .bytes
        .clone()
}

/// The code chosen in Files, where the tool opens on its Open card.
fn open_from_files(app: &mut Faraday, code: &[u8]) {
    app.storage(StorageEvent::Restored {
        inbox: vec![("code.pgp".to_string(), code.to_vec())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::QrTool(Q::OpenFile(0)));
    assert_eq!(app.screen, Screen::QrTool);
}

/// The passphrase typed and Open pressed, the answer waited for.
fn unseal(app: &mut Faraday, pass: &str) {
    type_in(app, pass);
    app.press(Action::QrTool(Q::OpenCode));
    testkit::settle(app);
}

#[test]
fn a_code_from_files_opens_on_the_open_card() {
    let mut app = started();
    open_from_files(&mut app, EMPTY);
    assert!(has(&mut app, "QR vault code, 396 bytes"));
    assert!(has(&mut app, "Open"));
    assert!(!has(&mut app, "Contents"), "the Make cards are not shown");
    assert!(qr_tool::state(&app).unwrap().holds().is_none());
}

#[test]
fn the_right_passphrase_shows_a_note_behind_its_eye() {
    let code = sealed_note("meet at oak", "pass phrase");
    let mut app = started();
    open_from_files(&mut app, &code);
    unseal(&mut app, "pass phrase");
    assert_eq!(qr_tool::state(&app).unwrap().holds(), Some(QrHolds::Note));
    assert!(has(&mut app, "A note"));
    assert!(!has_part(&mut app, "meet at oak"), "the note is masked");
    app.press(Action::QrTool(Q::Eye));
    assert!(has_part(&mut app, "meet at oak"));
    app.press(Action::QrTool(Q::Eye));
    assert!(!has_part(&mut app, "meet at oak"));
}

#[test]
fn an_empty_qr_vault_reads_empty() {
    let mut app = started();
    open_from_files(&mut app, EMPTY);
    unseal(&mut app, VECTOR_PASS);
    assert_eq!(qr_tool::state(&app).unwrap().holds(), Some(QrHolds::Empty));
    assert!(has(&mut app, "This QR vault is empty."));
}

#[test]
fn a_seed_and_a_passphrase_are_named_and_a_passphrase_is_behind_its_eye() {
    let seed = include_bytes!("../../../../tools/vectors/vault/qr/qr-seed-12.pgp");
    let pass = include_bytes!("../../../../tools/vectors/vault/qr/qr-passphrase.pgp");
    let mut app = started();
    open_from_files(&mut app, seed);
    unseal(&mut app, VECTOR_PASS);
    assert_eq!(
        qr_tool::state(&app).unwrap().holds(),
        Some(QrHolds::Seed { words: 12 })
    );
    assert!(!has_part(&mut app, "abandon"), "the words are not shown");
    app.press(Action::Cancel);
    assert!(has(&mut app, "A seed of 12 words"));
    let mut app = started();
    open_from_files(&mut app, pass);
    unseal(&mut app, VECTOR_PASS);
    assert!(has(&mut app, "A passphrase"));
    assert!(has_part(&mut app, "Key "));
    assert!(offered(&mut app, Action::QrTool(Q::Eye)));
}

#[test]
fn a_wrong_passphrase_says_so_and_a_second_try_opens() {
    let code = sealed_note("meet at oak", "pass phrase");
    let mut app = started();
    open_from_files(&mut app, &code);
    unseal(&mut app, "not it");
    assert!(has(&mut app, "The passphrase is wrong"));
    assert!(qr_tool::state(&app).unwrap().holds().is_none());
    assert_eq!(
        qr_tool::state(&app).unwrap().focus(),
        Some(qr_tool::Field::Pass)
    );
    unseal(&mut app, "pass phrase");
    assert_eq!(qr_tool::state(&app).unwrap().holds(), Some(QrHolds::Note));
}

#[test]
fn a_code_that_needs_more_memory_than_is_free_is_named_with_no_progress() {
    let mut app = started();
    app.storage(StorageEvent::Memory { available_mib: 100 });
    open_from_files(&mut app, EMPTY);
    type_in(&mut app, VECTOR_PASS);
    app.press(Action::QrTool(Q::OpenCode));
    assert!(app.busy().is_none());
    assert!(has_part(&mut app, "This computer has"));
    assert!(qr_tool::state(&app).unwrap().holds().is_none());
}

#[test]
fn cancel_while_the_code_opens_shows_nothing_and_it_can_be_opened_again() {
    let code = sealed_note("meet at oak", "pass phrase");
    let mut app = started();
    open_from_files(&mut app, &code);
    type_in(&mut app, "pass phrase");
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::OpenCode));
    assert!(app.busy().is_some(), "the screen shows Progress");
    app.press(Action::Cancel);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    let s = qr_tool::state(&app).expect("the tool stays open");
    assert!(s.holds().is_none(), "nothing was opened");
    assert!(!has_part(&mut app, "meet at oak"));
    unseal(&mut app, "pass phrase");
    assert_eq!(qr_tool::state(&app).unwrap().holds(), Some(QrHolds::Note));
}

#[test]
fn back_and_a_lock_while_the_code_opens_leave_nothing_open() {
    let code = sealed_note("meet at oak", "pass phrase");
    for leave in [Action::Back, Action::Lock] {
        let mut app = started();
        open_from_files(&mut app, &code);
        type_in(&mut app, "pass phrase");
        testkit::hold_worker();
        testkit::press_on_worker(&mut app, Action::QrTool(Q::OpenCode));
        app.press(leave);
        assert!(app.busy().is_none());
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        assert!(qr_tool::state(&app).is_none(), "{leave:?}");
        assert!(!has_part(&mut app, "meet at oak"));
        open_from_files(&mut app, &code);
        assert!(qr_tool::state(&app).unwrap().holds().is_none());
    }
}

#[test]
fn leaving_the_tool_forgets_what_was_opened() {
    let code = sealed_note("meet at oak", "pass phrase");
    let mut app = started();
    open_from_files(&mut app, &code);
    unseal(&mut app, "pass phrase");
    app.press(Action::QrTool(Q::Eye));
    assert!(has_part(&mut app, "meet at oak"));
    assert_eq!(app.session.keys.qr_holds(), Some(QrHolds::Note));
    app.press(Action::Back);
    assert!(qr_tool::state(&app).is_none());
    assert_eq!(app.session.keys.qr_holds(), None, "the store is empty");
    open_from_files(&mut app, &code);
    assert!(!has_part(&mut app, "meet at oak"));
    assert!(!texts(&mut app).iter().any(|t| t.contains("A note")));
    unseal(&mut app, "pass phrase");
    assert!(
        !has_part(&mut app, "meet at oak"),
        "the eye is closed again"
    );
}

#[test]
fn a_lock_forgets_what_was_opened() {
    let code = sealed_note("meet at oak", "pass phrase");
    let mut app = started();
    open_from_files(&mut app, &code);
    unseal(&mut app, "pass phrase");
    assert_eq!(app.session.keys.qr_holds(), Some(QrHolds::Note));
    app.press(Action::Lock);
    assert_eq!(app.session.keys.qr_holds(), None);
}

#[test]
fn the_idle_warning_closes_the_eye_and_asks_for_the_passphrase_again() {
    let code = sealed_note("meet at oak", "pass phrase");
    let mut app = started();
    open_from_files(&mut app, &code);
    unseal(&mut app, "pass phrase");
    app.press(Action::QrTool(Q::Eye));
    assert!(has_part(&mut app, "meet at oak"));
    let last = app.now_ms();
    app.event(Event::Tick {
        now_ms: last + 26 * 60_000,
    });
    assert_eq!(app.sheet, Some(faraday_core::Sheet::IdleWarn));
    let _ = app.frame();
    assert_eq!(app.session.keys.qr_holds(), None);
    assert!(!has_part(&mut app, "meet at oak"));
    let s = qr_tool::state(&app).expect("the tool stays open");
    assert!(s.holds().is_none());
    assert_eq!(s.focus(), Some(qr_tool::Field::Pass));
}

const SEED12: &[u8] = include_bytes!("../../../../tools/vectors/vault/qr/qr-seed-12.pgp");
const SHARE: &[u8] = include_bytes!("../../../../tools/vectors/vault/qr/qr-codex32.pgp");
const PASS: &[u8] = include_bytes!("../../../../tools/vectors/vault/qr/qr-passphrase.pgp");
const NOTE: &[u8] = include_bytes!("../../../../tools/vectors/vault/qr/qr-note.pgp");

/// The key of `WORDS12` with the passphrase `TREZOR`.
const TREZOR_KEY: &str = "b4e3f5ed";

#[test]
fn an_opened_seed_goes_to_the_seed_sheet_and_loads_as_a_key() {
    let mut app = started();
    open_from_files(&mut app, SEED12);
    unseal(&mut app, VECTOR_PASS);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::SeedRead));
    assert!(has(&mut app, "QR vault read"));
    assert!(has(&mut app, "Use without a passphrase"));
    assert!(has(&mut app, "Add a passphrase"));
    assert!(app.session.keys.is_empty());
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1);
    assert!(app.session.keys[0].has_words());
}

#[test]
fn cancelling_the_seed_sheet_loads_nothing_and_the_card_offers_the_seed_again() {
    let mut app = started();
    open_from_files(&mut app, SEED12);
    unseal(&mut app, VECTOR_PASS);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::SeedRead));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::QrTool);
    assert!(app.session.keys.is_empty());
    assert!(has(&mut app, "A seed of 12 words"));
    app.press(Action::QrTool(Q::Seed));
    assert_eq!(app.sheet, Some(faraday_core::Sheet::SeedRead));
}

#[test]
fn an_opened_share_is_typed_into_the_codex32_form() {
    let mut app = started();
    open_from_files(&mut app, SHARE);
    unseal(&mut app, VECTOR_PASS);
    assert!(has(&mut app, "Add the share"));
    app.press(Action::QrTool(Q::Share));
    assert_eq!(app.screen, Screen::Entry);
    assert!(has(&mut app, "Share C · id name · 2 needed"));
}

fn add_twelve(app: &mut Faraday) {
    let c = testkit::compute();
    app.session
        .keys
        .add_words(&c, WORDS12, "Twelve", None)
        .unwrap();
}

#[test]
fn an_opened_passphrase_is_added_to_the_key_its_words_make() {
    let mut app = started();
    add_twelve(&mut app);
    open_from_files(&mut app, PASS);
    unseal(&mut app, VECTOR_PASS);
    let add = format!("Add to key {TREZOR_KEY}");
    assert!(has(&mut app, &add));
    app.press(Action::QrTool(Q::AddToKey));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 2);
    assert!(has_part(&mut app, TREZOR_KEY));
    assert!(has(&mut app, &format!("Key {TREZOR_KEY} · already here")));
    assert!(!offered(&mut app, Action::QrTool(Q::AddToKey)));
}

#[test]
fn an_opened_passphrase_whose_key_is_loaded_says_so() {
    let mut app = started();
    add_twelve(&mut app);
    let c = testkit::compute();
    app.session
        .keys
        .add_with_passphrase(&c, 0, "TREZOR", "Twelve with")
        .unwrap();
    open_from_files(&mut app, PASS);
    unseal(&mut app, VECTOR_PASS);
    assert!(has(&mut app, &format!("Key {TREZOR_KEY} · already here")));
    assert!(!offered(&mut app, Action::QrTool(Q::AddToKey)));
}

#[test]
fn an_opened_passphrase_with_no_words_loaded_says_the_key_is_not_loaded() {
    let mut app = started();
    open_from_files(&mut app, PASS);
    unseal(&mut app, VECTOR_PASS);
    assert!(has(&mut app, &format!("Key {TREZOR_KEY} is not loaded")));
    assert!(has(&mut app, "Load its words without a passphrase first."));
    assert!(!offered(&mut app, Action::QrTool(Q::AddToKey)));
}

#[test]
fn with_a_stick_plugged_in_no_key_is_loaded_and_the_card_says_why() {
    let mut shell = crate::common::Shell::new();
    shell.add(crate::common::Disk::empty("S1", "KEYSTICK"));
    let mut app = started();
    add_twelve(&mut app);
    open_from_files(&mut app, PASS);
    unseal(&mut app, VECTOR_PASS);
    shell.plug(&mut app, "S1");
    app.press(Action::Cancel);
    app.press(Action::QrTool(Q::AddToKey));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1);
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x.contains("Keys load only with no")),
        "{:?} {t:?}",
        app.screen
    );

    let mut shell = crate::common::Shell::new();
    shell.add(crate::common::Disk::empty("S1", "KEYSTICK"));
    let mut app = started();
    open_from_files(&mut app, SEED12);
    unseal(&mut app, VECTOR_PASS);
    app.press(Action::Cancel);
    shell.plug(&mut app, "S1");
    app.press(Action::Cancel);
    app.press(Action::QrTool(Q::Seed));
    assert_eq!(app.sheet, None);
    assert!(app.session.keys.is_empty());
    assert!(has_part(&mut app, "Keys load only with no"));
}

#[test]
fn an_opened_note_is_saved_into_the_open_vault_and_dimmed_with_none() {
    let mut app = started();
    open_from_files(&mut app, NOTE);
    unseal(&mut app, VECTOR_PASS);
    assert!(has(&mut app, "Save as a note"));
    assert!(has(&mut app, "Open a vault first"));
    assert!(!offered(&mut app, Action::QrTool(Q::SaveNote)));
}

#[test]
fn an_opened_note_is_saved_as_a_note_last_in_the_open_vault() {
    use faraday_core::vault::VaultAction as V;
    use faraday_vault::records::kind;
    let file = testkit::kdbx_vault("Alpha", "vault phrase", 1, Vec::new()).unwrap();
    let mut app = started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("a.kdbx".to_string(), file),
            ("code.pgp".to_string(), NOTE.to_vec()),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    type_in(&mut app, "vault phrase");
    app.press(Action::Vault(V::Unlock));
    testkit::settle(&mut app);
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    app.press(Action::Vault(V::LoadNotNow));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::QrTool(Q::OpenFile(1)));
    assert_eq!(app.screen, Screen::QrTool);
    unseal(&mut app, VECTOR_PASS);
    assert!(offered(&mut app, Action::QrTool(Q::SaveNote)));
    assert!(!has(&mut app, "Open a vault first"));
    let id = app.vaults().open[0].id;
    let before = app.vaults().records(id).of(kind::NOTE).count();
    app.press(Action::QrTool(Q::SaveNote));
    assert_eq!(app.vaults().records(id).of(kind::NOTE).count(), before + 1);
    assert!(has(&mut app, "Saved as a note"));
    assert!(!offered(&mut app, Action::QrTool(Q::SaveNote)));
    app.press(Action::QrTool(Q::SaveNote));
    assert_eq!(app.vaults().records(id).of(kind::NOTE).count(), before + 1);
}

#[test]
fn practice_does_not_load_a_seed_that_is_not_public() {
    let mut app = started();
    testkit::into_demo(&mut app);
    open_from_files(&mut app, SEED12);
    unseal(&mut app, VECTOR_PASS);
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
    testkit::settle(&mut app);
    assert!(app.session.keys.is_empty());
    assert_eq!(app.screen, Screen::QrTool);
    assert!(has_part(&mut app, "Practice takes only samples"));
}

#[test]
fn a_seed_already_loaded_says_so_on_the_card() {
    let mut app = started();
    add_twelve(&mut app);
    open_from_files(&mut app, SEED12);
    unseal(&mut app, VECTOR_PASS);
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.screen, Screen::QrTool);
    assert_eq!(app.sheet, None);
    assert!(has_part(&mut app, "already loaded"));
}

#[test]
fn a_share_is_not_typed_in_with_a_stick_plugged_in() {
    let mut shell = crate::common::Shell::new();
    shell.add(crate::common::Disk::empty("S1", "KEYSTICK"));
    let mut app = started();
    open_from_files(&mut app, SHARE);
    unseal(&mut app, VECTOR_PASS);
    shell.plug(&mut app, "S1");
    app.press(Action::Cancel);
    app.press(Action::QrTool(Q::Share));
    assert_eq!(app.screen, Screen::QrTool);
    assert!(has_part(&mut app, "Keys load only with no"));
}

/// A QR vault code holding `record`, sealed under the vector passphrase.
fn sealed_record(record: faraday_vault::Record) -> Vec<u8> {
    use faraday_keys::vault::{OpenVaults, SEAL_RANDOM_LEN, create};
    let c = testkit::compute();
    let file = create(
        &c,
        [7u8; 16],
        "Test",
        faraday_vault::Cost {
            memory_kib: 8192,
            passes: 1,
            lanes: 1,
        },
        "pass",
        &[7u8; SEAL_RANDOM_LEN],
        None,
    )
    .expect("a vault");
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let at = store.insert(id, record).expect("kept");
    store
        .qr_seal_item_job(
            &c,
            id,
            at,
            VECTOR_PASS,
            *faraday_keys::QR_SEAL_EXPONENTS.start(),
            zeroize::Zeroizing::new([7u8; faraday_keys::QR_RANDOM_LEN]),
        )
        .expect("job")
        .run(&c)
        .expect("sealed")
}

#[test]
fn an_earlier_add_a_key_target_does_not_reach_a_seed_loaded_from_the_card() {
    let mut app = started();
    app.press(Action::Entry(Some([1, 2, 3, 4])));
    app.press(Action::Nav(Screen::Home));
    open_from_files(&mut app, SEED12);
    unseal(&mut app, VECTOR_PASS);
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_passphrase_naming_a_plain_loaded_key_says_no_loaded_key_makes_it() {
    use faraday_vault::records::{field, kind};
    let code = sealed_record(
        faraday_vault::Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &[0x73, 0xc5, 0xda, 0x0a])
            .with(field::PASSPHRASE, b"TREZOR"),
    );
    let mut app = started();
    add_twelve(&mut app);
    open_from_files(&mut app, &code);
    unseal(&mut app, VECTOR_PASS);
    assert!(has(
        &mut app,
        "No loaded key makes key 73c5da0a with this passphrase"
    ));
    assert!(!has_part(&mut app, "is not loaded"));
    assert!(!offered(&mut app, Action::QrTool(Q::AddToKey)));
}

// ----- Make before the system's answers have arrived (F102) -----

/// Make is the press that waits: its button reads "Waiting for
/// randomness".
fn waiting_shown(app: &mut Faraday) -> bool {
    app.waits_for_randomness(Action::QrTool(Q::Make))
}

/// A note typed and Make pressed in a session whose seed has arrived
/// but none of the answers a code's random bytes are drawn from.
fn make_before_the_answers(app: &mut Faraday) {
    app.event(Event::Entropy(EntropyBytes::new([0x3f; 32])));
    open(app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(app, "meet at the old oak");
    app.press(Action::QrTool(Q::Next));
    passphrase(app, "correct horse", "");
    app.press(Action::QrTool(Q::Make));
    testkit::settle(app);
}

#[test]
fn make_waits_for_the_systems_answers_and_seals_by_itself() {
    let mut app = testkit::started();
    make_before_the_answers(&mut app);
    assert!(waiting_shown(&mut app));
    assert!(!qr_tool::state(&app).unwrap().is_made());
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    testkit::settle(&mut app);
    assert!(qr_tool::state(&app).unwrap().is_made(), "no second press");
    assert!(!waiting_shown(&mut app));
}

#[test]
fn make_with_nothing_to_seal_does_not_wait() {
    let mut app = testkit::started();
    app.event(Event::Entropy(EntropyBytes::new([0x3f; 32])));
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    app.press(Action::QrTool(Q::Next));
    passphrase(&mut app, "correct horse", "");
    app.press(Action::QrTool(Q::Make));
    assert!(!waiting_shown(&mut app));
}

#[test]
fn make_waiting_a_minute_gives_up_and_makes_nothing() {
    let mut app = testkit::started();
    app.event(Event::Tick { now_ms: 1_000 });
    make_before_the_answers(&mut app);
    assert!(waiting_shown(&mut app));
    app.event(Event::Tick { now_ms: 62_000 });
    assert!(!waiting_shown(&mut app));
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == faraday_strings::EN.no_randomness)
    );
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    testkit::settle(&mut app);
    assert!(!qr_tool::state(&app).unwrap().is_made());
}
