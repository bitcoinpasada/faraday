//! The backup's checklist item for a seed's copy by hand checks the
//! hand-drawn SeedQR: Scan my copy opens the camera, a copy of the seed
//! on screen says it matches and marks the item done, a copy with a word wrong or of another seed names
//! the first word that differs and keeps the camera open, and nothing
//! scanned there is loaded as a key or put in Files. With no camera the
//! copy is checked from its typed numbers instead.
//!
//! The codes are SeedSigner's published SeedQR test vectors
//! (`docs/seed_qr/README.md`, "Test SeedQRs"), drawn by the repository's
//! own encoder and read back by its own decoder, as a camera would.

use faraday_core::seeds::SeedsAction as S;
use faraday_core::{Action, Faraday, ScanPurpose, Sheet, StorageEvent, bstep};
use osk_bip::bip39::{Language, Mnemonic};
use osk_shell_api::{App, BootState, Command, DisplayInfo, Event, Key, SecureHardware};

/// SeedSigner's 24-word vector: the words and its Standard SeedQR digits.
const ATTACK: &str = "attack pizza motion avocado network gather crop fresh patrol unusual \
                      wild holiday candy pony ranch winter theme error hybrid van cereal salon \
                      goddess expire";
const ATTACK_DIGITS: &str = "011513251154012711900771041507421289190620080870026613431420201617920614089619290300152408010643";
/// SeedSigner's 12-word vectors.
const FORUM: &str = "forum undo fragile fade shy sign arrest garment culture tube off merit";
const FORUM_COMPACT: &[u8] = b"[\xbd\x9dq\xa8\xecy\x90\x83\x1a\xff5\x9dBeE";
const GOOD: &str = "good battle boil exact add seed angle hurry success glad carbon whisper";

fn shown() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// The seed typed into Add a key, a one-key wallet made from it, and its
/// backup's paper plan made, open on the seed's copy with the seed shown; a camera listed
/// when `camera`.
fn backing_up(words: &str, camera: bool) -> Faraday {
    let mut app = shown();
    if camera {
        app.storage(StorageEvent::Cameras(vec![(
            "/dev/video0".into(),
            "Integrated Camera".into(),
        )]));
    }
    app.press(Action::Entry(None));
    for c in words.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "{:?}", app.entry.error);
    let fp = app.session.keys[0].master.fingerprint().0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::Backup(0));
    app.press(Action::BPreset(0));
    app.press(Action::BChecklist);
    app.press(Action::BStep(bstep::COPY));
    app.press(Action::BReveal);
    let _ = app.frame();
    app
}

/// What a camera reads off a drawn code: the code drawn as pixels by the
/// repository's encoder, read back by its decoder.
fn read_off(matrix: &osk_codec::qr::QrMatrix) -> Vec<u8> {
    let (w, h, px) = matrix.to_luma(4, osk_codec::qr::QUIET_ZONE);
    let found = osk_codec::decode_luma(w, h, &px);
    assert_eq!(found.len(), 1, "one code in the frame");
    found[0].bytes.clone()
}

fn standard(words: &str) -> Vec<u8> {
    let m = Mnemonic::parse(Language::English, words).unwrap();
    read_off(&osk_codec::seedqr::encode_seedqr(&m).unwrap())
}

fn compact(words: &str) -> Vec<u8> {
    let m = Mnemonic::parse(Language::English, words).unwrap();
    read_off(&osk_codec::seedqr::encode_compact(&m).unwrap())
}

fn scan(app: &mut Faraday, bytes: Vec<u8>) {
    app.event(Event::Scanned { bytes });
    let _ = app.frame();
}

fn note(app: &Faraday) -> String {
    app.scan
        .as_ref()
        .and_then(|s| s.note.clone())
        .unwrap_or_default()
}

fn seeds_done(app: &Faraday) -> bool {
    app.backup_item_done(faraday_core::plan::Item::Copy(0))
}

/// Whether the seeds step offers `action` anywhere down its column,
/// scrolled to as a person would.
fn offered(app: &mut Faraday, action: Action) -> bool {
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(action) {
            return true;
        }
        app.event(Event::Scroll {
            x: 683,
            y: 384,
            dy: 300,
        });
    }
    false
}

fn commands(app: &mut Faraday) -> Vec<Command> {
    std::iter::from_fn(|| app.poll_command()).collect()
}

#[test]
fn a_drawn_standard_seedqr_of_the_seed_matches_and_adds_nothing() {
    let mut app = backing_up(ATTACK, true);
    assert!(
        offered(&mut app, Action::BScan),
        "no Scan my copy with a camera"
    );
    assert!(!app.offers(Action::BCheck));
    app.press(Action::BScan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(commands(&mut app).contains(&Command::CameraOn));
    let fp = app.session.keys[0].master.fingerprint();
    assert_eq!(
        app.scan.as_ref().map(|s| s.purpose),
        Some(ScanPurpose::CheckCopy(fp))
    );
    let inbox = app.inbox.len();

    let code = standard(ATTACK);
    assert_eq!(code, ATTACK_DIGITS.as_bytes(), "the published digits");
    scan(&mut app, code);

    assert!(
        app.scan.is_none() && app.sheet.is_none(),
        "the camera stayed open"
    );
    assert!(commands(&mut app).contains(&Command::CameraOff));
    assert_eq!(app.session.keys.len(), 1, "a key was added");
    assert_eq!(app.inbox.len(), inbox, "something went to Files");
    assert!(seeds_done(&app), "the seeds step is not done");
    let b = app.backup.as_ref().unwrap();
    assert_eq!(
        faraday_core::backup::copy_scan_line(b.scanned.as_ref().unwrap()),
        "Your copy scans and matches the seed"
    );
}

#[test]
fn a_drawn_compact_seedqr_of_the_seed_matches() {
    let mut app = backing_up(FORUM, true);
    app.press(Action::BScan);
    let code = compact(FORUM);
    assert_eq!(code, FORUM_COMPACT, "the published bytes");
    scan(&mut app, code);
    assert!(app.scan.is_none(), "the camera stayed open");
    assert!(seeds_done(&app));
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_copy_with_one_word_wrong_names_that_word_and_stays_open() {
    let mut app = backing_up(ATTACK, true);
    app.press(Action::BScan);
    // Word 5, "network" (1190), copied as 1191: its checksum no longer
    // holds, and the word is still named.
    let mut digits = ATTACK_DIGITS.as_bytes().to_vec();
    digits[4 * 4 + 3] = b'1';
    let m = osk_codec::qr::encode(
        osk_codec::qr::Payload::Numeric(&digits),
        osk_codec::qr::Ecc::Low,
    )
    .unwrap();
    scan(&mut app, read_off(&m));
    assert_eq!(note(&app), "Your copy scans but word 5 differs");
    assert_eq!(app.sheet, Some(Sheet::Scan), "the camera closed");
    assert!(!seeds_done(&app));

    // Fixed and scanned again, it matches.
    scan(&mut app, standard(ATTACK));
    assert!(app.scan.is_none());
    assert!(seeds_done(&app));
}

#[test]
fn a_seedqr_of_another_seed_is_not_loaded_and_names_the_first_word() {
    let mut app = backing_up(FORUM, true);
    app.press(Action::BScan);
    let inbox = app.inbox.len();
    scan(&mut app, standard(GOOD));
    assert_eq!(note(&app), "Your copy scans but word 1 differs");
    assert_eq!(app.session.keys.len(), 1, "the other seed was loaded");
    assert_eq!(app.inbox.len(), inbox);
    // Something that is no SeedQR says so.
    scan(
        &mut app,
        b"bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu".to_vec(),
    );
    assert_eq!(note(&app), "Not a SeedQR");
    // Closed by the person: the camera is off, the line stays.
    let _ = commands(&mut app);
    app.press(Action::Cancel);
    assert!(app.scan.is_none());
    assert!(commands(&mut app).contains(&Command::CameraOff));
    let b = app.backup.as_ref().unwrap();
    assert_eq!(
        faraday_core::backup::copy_scan_line(b.scanned.as_ref().unwrap()),
        "Your copy scans but word 1 differs"
    );
}

#[test]
fn with_no_camera_the_copy_is_checked_by_its_typed_numbers() {
    let mut app = backing_up(FORUM, false);
    assert!(offered(&mut app, Action::BCheck));
    assert!(!app.offers(Action::BScan), "Scan my copy with no camera");
    app.press(Action::BCheck);
    let digits = standard(FORUM);
    for &c in &digits[..44] {
        app.event(Event::Key(Key::Char(char::from(c))));
    }
    assert!(!seeds_done(&app));
    for &c in &digits[44..] {
        app.event(Event::Key(Key::Char(char::from(c))));
    }
    assert!(seeds_done(&app), "a typed match does not mark the step");
}

#[test]
fn escape_closes_the_scanner_with_its_camera() {
    let mut app = backing_up(FORUM, true);
    app.press(Action::BScan);
    assert!(commands(&mut app).contains(&Command::CameraOn));
    app.event(Event::Key(Key::Escape));
    assert!(app.sheet.is_none() && app.scan.is_none());
    assert!(
        commands(&mut app).contains(&Command::CameraOff),
        "the camera stayed on"
    );
}
