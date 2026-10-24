//! A copy of a seed by hand is checked by its form (`docs/history/NEW-WALLET.md`
//! §14.7): a copy of the words is typed back, with a camera listed or
//! not; a SeedQR is scanned when there is a camera, and typed as words
//! when there is none; a seed copied both ways is checked both ways. The
//! typed check hides the seed, masks the words taken, accepts words, four
//! letters of a word and a word's four-digit number, compares only after
//! the last word and names the first word that differs. The scan opens
//! the camera, a copy of the seed on screen says it matches, a copy with a
//! word wrong or of another seed names the first word that differs and
//! keeps the camera open, and nothing scanned there is loaded as a key or
//! put in Files. Keep the words only drops a SeedQR copy for the words,
//! offered at the foot after three scans without a match; the grid
//! dropped stays on the map drawn in part until destroyed.
//!
//! The codes are SeedSigner's published SeedQR test vectors
//! (`docs/seed_qr/README.md`, "Test SeedQRs"), drawn by the repository's
//! own encoder and read back by its own decoder, as a camera would.

use crate::common::shown;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::board::{BoardPress as BP, Thing, Zone};
use faraday_core::glance::Press;
use faraday_core::glance::sheet::{ChartAction as C, Step, Target};
use faraday_core::plan::{self, Item};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit;
use faraday_core::{Action, Faraday, ScanPurpose, Sheet, StorageEvent, bstep};
use faraday_shell_api::{App, BootState, Command, DisplayInfo, Event, Key, SecureHardware};

/// SeedSigner's 24-word vector: the words and its Standard SeedQR digits.
const ATTACK: &str = "attack pizza motion avocado network gather crop fresh patrol unusual \
                      wild holiday candy pony ranch winter theme error hybrid van cereal salon \
                      goddess expire";
const ATTACK_DIGITS: &str = "011513251154012711900771041507421289190620080870026613431420201617920614089619290300152408010643";
/// SeedSigner's 12-word vectors.
const FORUM: &str = "forum undo fragile fade shy sign arrest garment culture tube off merit";
const FORUM_COMPACT: &[u8] = b"[\xbd\x9dq\xa8\xecy\x90\x83\x1a\xff5\x9dBeE";
const GOOD: &str = "good battle boil exact add seed angle hurry success glad carbon whisper";

fn shown_on(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
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

fn cameras(app: &mut Faraday) {
    app.storage(StorageEvent::Cameras(vec![(
        "/dev/video0".into(),
        "Integrated Camera".into(),
    )]));
}

/// The copies of seed `seed` made words, a SeedQR or both, on the board:
/// what is missing is added first, as the last form cannot be taken off.
fn copies_as(app: &mut Faraday, seed: usize, words: bool, seedqr: bool) {
    let c = faraday_core::testkit::compute();
    if app.board_plan().is_some_and(|(_, a)| !a.custom) {
        app.press(Action::Board(BP::Customize));
    }
    let forms = |app: &Faraday| {
        let (shape, a) = app.board_plan().expect("a board");
        plan::copy_forms(&c, &shape, &a, seed)
    };
    let place = {
        let (shape, a) = app.board_plan().expect("a board");
        plan::map(&c, &shape, &a)
            .iter()
            .find_map(|s| match s.at {
                plan::At::Place(p)
                    if s.holds
                        .iter()
                        .any(|(w, _)| matches!(w, plan::What::Words(i) | plan::What::SeedQr(i) if *i == seed)) =>
                {
                    Some(p)
                }
                _ => None,
            })
            .expect("a place keeps the seed")
    };
    let toggle = |app: &mut Faraday, k: u8| {
        app.press(Action::Board(BP::Form(
            Zone::Paper(place),
            Thing::Seed(seed),
            k,
        )));
    };
    // Add, then take off.
    for (k, want) in [(0u8, words), (1, seedqr)] {
        let has = forms(app);
        if want && !(if k == 0 { has.words } else { has.seedqr }) {
            toggle(app, k);
        }
    }
    for (k, want) in [(0u8, words), (1, seedqr)] {
        let has = forms(app);
        if !want && (if k == 0 { has.words } else { has.seedqr }) {
            toggle(app, k);
        }
    }
    let got = forms(app);
    assert_eq!(
        (got.words, got.seedqr),
        (words, seedqr),
        "the board's copies"
    );
}

/// The seed typed into Add a key, a one-key wallet made from it, and its
/// backup planned as paper with the copies of its words and/or its SeedQR
/// as asked, a camera listed when `camera`; open on the seed's copy with
/// the seed shown.
fn backing_up_on(app: &mut Faraday, words: &str, camera: bool, copies: (bool, bool)) {
    let c = faraday_core::testkit::compute();
    if camera {
        cameras(app);
    }
    app.press(Action::Entry(None));
    // Typed in Latin letters on the keyboard.
    for c in words.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "{:?}", testkit::entry(app).error);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::Backup(0));
    testkit::backup_as(app, plan::Preset::Paper);
    copies_as(app, 0, copies.0, copies.1);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    // The template is made with the checklist, so the copy is the first
    // item left and opens by itself.
    if app.backup().and_then(|b| b.open) != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BReveal);
    let _ = app.frame();
}

fn backing_up(words: &str, camera: bool, copies: (bool, bool)) -> Faraday {
    let mut app = shown_on(1366, 800, 160);
    backing_up_on(&mut app, words, camera, copies);
    app
}

const WORDS_ONLY: (bool, bool) = (true, false);
const SEEDQR_ONLY: (bool, bool) = (false, true);
const BOTH: (bool, bool) = (true, true);

/// What a camera reads off a drawn code: the code drawn as pixels by the
/// repository's encoder, read back by its decoder.
fn read_off(matrix: &faraday_codec::qr::QrMatrix) -> Vec<u8> {
    let c = faraday_core::testkit::compute();
    let (w, h, px) = matrix.to_luma(4, faraday_codec::qr::QUIET_ZONE);
    let found = faraday_codec::decode_luma(&c, w, h, &px);
    assert_eq!(found.len(), 1, "one code in the frame");
    found[0].bytes.clone()
}

fn standard(words: &str) -> Vec<u8> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, words).unwrap();
    read_off(&faraday_codec::seedqr::encode_seedqr(&c, &m).unwrap())
}

fn compact(words: &str) -> Vec<u8> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, words).unwrap();
    read_off(&faraday_codec::seedqr::encode_compact(&c, &m).unwrap())
}

fn scan(app: &mut Faraday, bytes: Vec<u8>) {
    app.event(Event::Scanned { bytes });
    let _ = app.frame();
}

fn note(app: &Faraday) -> String {
    app.scan().and_then(|s| s.note.clone()).unwrap_or_default()
}

fn seeds_done(app: &Faraday) -> bool {
    let c = testkit::compute();
    app.backup_item_done_now(&c, Item::Copy(0))
}

/// Whether the page says anything that contains `part`.
fn says(app: &mut Faraday, part: &str) -> bool {
    app.drawn_texts().iter().any(|t| t.contains(part))
}

/// Whether the checklist offers `action` anywhere down its column,
/// scrolled to as a person would.
/// Whether the text is drawn once the page is scrolled down to it: the
/// page is as long as the mode makes it, so where a line lands in the
/// first frame is not what a person looks at.
fn scrolled_to(app: &mut Faraday, text: &str) -> bool {
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if shown(app, text) {
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

/// The words with the one at `at` (counting from one) replaced.
fn with_word(words: &str, at: usize, instead: &str) -> String {
    words
        .split_whitespace()
        .enumerate()
        .map(|(i, w)| if i + 1 == at { instead } else { w })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------
// Words by hand: typed back
// ---------------------------------------------------------------------

#[test]
fn a_words_only_plan_with_a_camera_opens_the_typed_check_not_the_camera() {
    let mut app = backing_up(ATTACK, true, WORDS_ONLY);
    assert!(offered(&mut app, Action::BCheck), "no Check my copy");
    assert!(
        !app.offers(Action::BScan),
        "the camera is offered for a copy of the words"
    );
    let _ = commands(&mut app);
    app.press(Action::BCheck);
    assert_eq!(app.sheet, None, "the camera opened");
    assert!(!commands(&mut app).contains(&Command::CameraOn));
    assert!(shown(&mut app, "Word 1 of 24"));
    testkit::type_words(&mut app, ATTACK);
    assert!(seeds_done(&app), "the typed seed did not complete the item");
    // Done, the item moves on by itself (`docs/BACKUP-FLOW.md` §17.23).
    assert_ne!(app.backup().unwrap().open, Some(faraday_core::bstep::COPY));
}

#[test]
fn one_wrong_word_is_named_and_the_right_word_and_the_rest_complete_it() {
    let mut app = backing_up(ATTACK, true, WORDS_ONLY);
    app.press(Action::BCheck);
    // Word 5, "network", typed as another word of the list.
    testkit::type_words(&mut app, &with_word(ATTACK, 5, "zoo"));
    assert!(!seeds_done(&app));
    assert!(shown(&mut app, "Word 5 differs from the seed"));
    // Typing resumes at word 5, the four before it kept.
    assert!(shown(&mut app, "Word 5 of 24"));
    testkit::type_words(&mut app, "network");
    assert!(!seeds_done(&app), "word 5 alone completed it");
    testkit::type_words(
        &mut app,
        &ATTACK
            .split_whitespace()
            .skip(5)
            .collect::<Vec<_>>()
            .join(" "),
    );
    assert!(seeds_done(&app));
}

#[test]
fn nothing_is_said_of_the_words_until_the_last_is_typed() {
    let mut app = backing_up(FORUM, false, WORDS_ONLY);
    app.press(Action::BCheck);
    // The first word wrong, the rest right but the last.
    let wrong = with_word(FORUM, 1, "zoo");
    let upto: Vec<&str> = wrong.split_whitespace().take(11).collect();
    testkit::type_words(&mut app, &upto.join(" "));
    assert!(!says(&mut app, "differs from the seed"));
    testkit::type_words(&mut app, "merit");
    assert!(shown(&mut app, "Word 1 differs from the seed"));
    assert!(shown(&mut app, "Word 1 of 12"));
}

#[test]
fn no_word_of_the_seed_is_drawn_while_the_check_is_open_and_taken_words_are_masked() {
    let mut app = backing_up(ATTACK, true, WORDS_ONLY);
    for w in ATTACK.split_whitespace() {
        assert!(shown(&mut app, w), "{w} is not drawn with the seed shown");
    }
    app.press(Action::BCheck);
    for w in ATTACK.split_whitespace() {
        assert!(!shown(&mut app, w), "{w} is drawn while the check is open");
    }
    testkit::type_words(&mut app, "attack pizza");
    for c in "mot".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    for w in ATTACK.split_whitespace() {
        assert!(!shown(&mut app, w), "{w} is drawn once typed");
    }
    // The word being typed is drawn as it is typed; the two taken are not.
    assert!(shown(&mut app, "mot"));
    let masked = app
        .drawn_texts()
        .iter()
        .filter(|t| t.as_str() == "••••••")
        .count();
    assert_eq!(masked, 2);
    assert!(shown(&mut app, "Word 3 of 24"));
}

#[test]
fn four_letters_of_a_word_and_its_four_digit_number_complete_the_item() {
    let prefixes: String = ATTACK
        .split_whitespace()
        .map(|w| w.chars().take(4).collect::<String>())
        .collect::<Vec<_>>()
        .join(" ");
    let mut app = backing_up(ATTACK, false, WORDS_ONLY);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, &prefixes);
    assert!(seeds_done(&app), "four letters of each word did not check");

    // Numbers as Show the seed prints them, mixed with words.
    let mixed: String = ATTACK
        .split_whitespace()
        .enumerate()
        .map(|(i, w)| {
            if i % 2 == 0 {
                w.to_string()
            } else {
                format!("{:04}", Language::English.index_of(w).unwrap())
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut app = backing_up(ATTACK, false, WORDS_ONLY);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, &mixed);
    assert!(seeds_done(&app), "numbers mixed with words did not check");
}

#[test]
fn a_number_past_the_list_is_a_word_not_on_it() {
    let mut app = backing_up(FORUM, false, WORDS_ONLY);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, "2048");
    assert!(shown(&mut app, "Word 1 is not on the English list"));
}

#[test]
fn show_the_seed_ends_the_check_and_it_starts_again_from_word_1() {
    let mut app = backing_up(ATTACK, true, WORDS_ONLY);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, "attack pizza motion");
    assert!(shown(&mut app, "Word 4 of 24"));
    app.press(Action::BReveal);
    assert!(shown(&mut app, "attack"), "the seed is not shown");
    assert!(!says(&mut app, "of 24"), "the check stayed open");
    app.press(Action::BCheck);
    assert!(shown(&mut app, "Word 1 of 24"));
}

#[test]
fn clear_starts_the_check_again_from_word_1() {
    let mut app = backing_up(ATTACK, false, WORDS_ONLY);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, "attack pizza motion");
    app.press(Action::BCheckClear);
    assert!(shown(&mut app, "Word 1 of 24"));
    testkit::type_words(&mut app, ATTACK);
    assert!(seeds_done(&app));
}

#[test]
fn a_lock_in_the_middle_of_the_check_leaves_nothing_of_it() {
    let mut app = backing_up(ATTACK, true, WORDS_ONLY);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, "attack pizza motion");
    app.press(Action::Lock);
    assert!(app.backup().is_none(), "the backup survived the lock");
    assert!(app.session.keys.is_empty());
    // The next power-on starts with the seed and its check empty.
    let mut app = backing_up(ATTACK, true, WORDS_ONLY);
    app.press(Action::BCheck);
    assert!(shown(&mut app, "Word 1 of 24"));
}

#[test]
fn on_a_small_panel_the_check_is_typed_on_the_word_keyboard() {
    let mut app = shown_on(480, 640, 286);
    backing_up_on(&mut app, FORUM, false, WORDS_ONLY);
    app.press(Action::BCheck);
    assert!(shown(&mut app, "Word 1 of 12"));
    // The keys of the touch screen: the first two words by their letters.
    for c in "forumundo".chars() {
        app.press(Action::EntryKey(c));
    }
    assert!(shown(&mut app, "Word 3 of 12"));
    // No word taken is drawn.
    for w in ["forum", "undo"] {
        assert!(!shown(&mut app, w), "{w} is drawn on the small panel");
    }
    // The rest from a physical keyboard.
    testkit::type_words(
        &mut app,
        &FORUM
            .split_whitespace()
            .skip(2)
            .collect::<Vec<_>>()
            .join(" "),
    );
    assert!(seeds_done(&app));
}

// ---------------------------------------------------------------------
// SeedQR by hand: scanned
// ---------------------------------------------------------------------

#[test]
fn a_seedqr_copy_with_a_camera_is_checked_by_the_scan_and_words_alone_do_not() {
    let mut app = backing_up(ATTACK, true, SEEDQR_ONLY);
    // The grid is drawn for the copy.
    assert!(scrolled_to(&mut app, "Standard SeedQR"));
    assert!(
        offered(&mut app, Action::BScan),
        "no Scan my copy with a camera"
    );
    assert!(!app.offers(Action::BCheck));
    app.press(Action::BCheck);
    testkit::type_words(&mut app, ATTACK);
    assert!(!seeds_done(&app), "typed words checked a SeedQR copy");
    app.press(Action::BScan);
    scan(&mut app, standard(ATTACK));
    assert!(seeds_done(&app));
}

#[test]
fn a_seed_copied_both_ways_is_checked_both_ways() {
    let mut app = backing_up(ATTACK, true, BOTH);
    // The words first.
    assert!(offered(&mut app, Action::BCheck));
    assert!(!app.offers(Action::BScan));
    app.press(Action::BCheck);
    testkit::type_words(&mut app, ATTACK);
    assert!(!seeds_done(&app), "the words alone checked both copies");
    // Then the scan.
    assert!(offered(&mut app, Action::BScan));
    app.press(Action::BScan);
    scan(&mut app, standard(ATTACK));
    assert!(seeds_done(&app));

    // The scan alone does not check the words.
    let mut app = backing_up(ATTACK, true, BOTH);
    app.press(Action::BScan);
    scan(&mut app, standard(ATTACK));
    assert!(!seeds_done(&app), "the scan alone checked both copies");
    app.press(Action::BCheck);
    testkit::type_words(&mut app, ATTACK);
    assert!(seeds_done(&app));
}

#[test]
fn a_seedqr_copy_with_no_camera_is_checked_by_its_typed_words() {
    let mut app = backing_up(ATTACK, false, SEEDQR_ONLY);
    assert!(offered(&mut app, Action::BCheck));
    assert!(!app.offers(Action::BScan), "Scan my copy with no camera");
    app.press(Action::BCheck);
    testkit::type_words(&mut app, ATTACK);
    assert!(seeds_done(&app));
}

#[test]
fn the_grid_is_drawn_only_for_a_seedqr_copy() {
    let mut app = backing_up(ATTACK, false, WORDS_ONLY);
    assert!(shown(&mut app, "attack"));
    assert!(!shown(&mut app, "Standard SeedQR"));
    assert!(!shown(&mut app, "Compact SeedQR"));
    let mut app = backing_up(ATTACK, false, BOTH);
    assert!(shown(&mut app, "attack"));
    assert!(shown(&mut app, "Standard SeedQR"));
    let mut app = backing_up(ATTACK, false, SEEDQR_ONLY);
    assert!(shown(&mut app, "Standard SeedQR"));
}

#[test]
fn a_drawn_standard_seedqr_of_the_seed_matches_and_adds_nothing() {
    let mut app = backing_up(ATTACK, true, SEEDQR_ONLY);
    app.press(Action::BScan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(commands(&mut app).contains(&Command::CameraOn));
    let fp = app.session.keys[0].fingerprint;
    assert_eq!(
        app.scan().map(|s| s.purpose),
        Some(ScanPurpose::CheckCopy(fp))
    );
    let inbox = app.inbox().len();

    let code = standard(ATTACK);
    assert_eq!(code, ATTACK_DIGITS.as_bytes(), "the published digits");
    scan(&mut app, code);

    assert!(
        app.scan().is_none() && app.sheet.is_none(),
        "the camera stayed open"
    );
    assert!(commands(&mut app).contains(&Command::CameraOff));
    assert_eq!(app.session.keys.len(), 1, "a key was added");
    assert_eq!(app.inbox().len(), inbox, "something went to Files");
    assert!(seeds_done(&app), "the seeds step is not done");
    let b = app.backup().unwrap();
    assert_eq!(
        faraday_core::wallet_backup::copy_scan_line(b.scanned.as_ref().unwrap()),
        "Your copy scans and matches the seed"
    );
}

#[test]
fn a_drawn_compact_seedqr_of_the_seed_matches() {
    let mut app = backing_up(FORUM, true, SEEDQR_ONLY);
    app.press(Action::BScan);
    let code = compact(FORUM);
    assert_eq!(code, FORUM_COMPACT, "the published bytes");
    scan(&mut app, code);
    assert!(app.scan().is_none(), "the camera stayed open");
    assert!(seeds_done(&app));
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_copy_with_one_word_wrong_names_that_word_and_stays_open() {
    let c = faraday_core::testkit::compute();
    let mut app = backing_up(ATTACK, true, SEEDQR_ONLY);
    app.press(Action::BScan);
    // Word 5, "network" (1190), copied as 1191: its checksum no longer
    // holds, and the word is still named.
    let mut digits = ATTACK_DIGITS.as_bytes().to_vec();
    digits[4 * 4 + 3] = b'1';
    let m = faraday_codec::qr::encode(
        &c,
        faraday_codec::qr::Payload::Numeric(&digits),
        faraday_codec::qr::Ecc::Low,
    )
    .unwrap();
    scan(&mut app, read_off(&m));
    assert_eq!(note(&app), "Your copy scans but word 5 differs");
    assert_eq!(app.sheet, Some(Sheet::Scan), "the camera closed");
    assert!(!seeds_done(&app));

    // Fixed and scanned again, it matches.
    scan(&mut app, standard(ATTACK));
    assert!(app.scan().is_none());
    assert!(seeds_done(&app));
}

#[test]
fn a_seedqr_of_another_seed_is_not_loaded_and_names_the_first_word() {
    let mut app = backing_up(FORUM, true, SEEDQR_ONLY);
    app.press(Action::BScan);
    let inbox = app.inbox().len();
    scan(&mut app, standard(GOOD));
    assert_eq!(note(&app), "Your copy scans but word 1 differs");
    assert_eq!(app.session.keys.len(), 1, "the other seed was loaded");
    assert_eq!(app.inbox().len(), inbox);
    // Something that is no SeedQR says so.
    scan(
        &mut app,
        b"bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu".to_vec(),
    );
    assert_eq!(note(&app), "Not a SeedQR");
    // Closed by the person: the camera is off, the line stays.
    let _ = commands(&mut app);
    app.press(Action::Cancel);
    assert!(app.scan().is_none());
    assert!(commands(&mut app).contains(&Command::CameraOff));
    let b = app.backup().unwrap();
    assert_eq!(
        faraday_core::wallet_backup::copy_scan_line(b.scanned.as_ref().unwrap()),
        "Your copy scans but word 1 differs"
    );
}

#[test]
fn escape_closes_the_scanner_with_its_camera() {
    let mut app = backing_up(FORUM, true, SEEDQR_ONLY);
    app.press(Action::BScan);
    assert!(commands(&mut app).contains(&Command::CameraOn));
    app.event(Event::Key(Key::Escape));
    assert!(app.sheet.is_none() && app.scan().is_none());
    assert!(
        commands(&mut app).contains(&Command::CameraOff),
        "the camera stayed on"
    );
}

// ---------------------------------------------------------------------
// Several keys, and the chart
// ---------------------------------------------------------------------

#[test]
fn each_seed_is_checked_by_its_own_forms() {
    let c = testkit::compute();
    let mut app = shown_on(1366, 768, 160);
    cameras(&mut app);
    for n in 0..2 {
        app.press(Action::Entry(None));
        for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.press(Action::EntryAdd);
    }
    app.session
        .add_wallet(
            &c,
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    let _ = app.frame();
    let seeds: Vec<usize> = app
        .backup_seed_list_now(w)
        .iter()
        .enumerate()
        .filter_map(|(i, (_, k))| k.map(|_| i))
        .collect();
    assert_eq!(seeds.len(), 2, "two seeds of Savings are here");
    // The first seed's words by hand, the second's SeedQR.
    copies_as(&mut app, seeds[0], true, false);
    copies_as(&mut app, seeds[1], false, true);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    let first = bstep::COPY + seeds[0] as u8;
    let second = bstep::COPY + seeds[1] as u8;
    let words_of = |app: &Faraday, i: usize| {
        let k = app.backup_seed_list_now(w)[i].1.unwrap();
        app.session
            .keys
            .reveal(k, faraday_keys::Reveal::Words)
            .to_string()
    };
    // The words seed: typed.
    if app.backup().unwrap().open != Some(first) {
        app.press(Action::BStep(first));
    }
    assert!(offered(&mut app, Action::BCheck));
    assert!(!app.offers(Action::BScan));
    app.press(Action::BCheck);
    let typed = words_of(&app, seeds[0]);
    testkit::type_words(&mut app, &typed);
    assert!(app.backup_item_done_now(&c, Item::Copy(seeds[0])));
    app.press(Action::BNext(first));
    // The SeedQR seed: scanned.
    if app.backup().unwrap().open != Some(second) {
        app.press(Action::BStep(second));
    }
    assert!(offered(&mut app, Action::BScan));
    assert!(!app.offers(Action::BCheck));
    assert!(!app.backup_item_done_now(&c, Item::Copy(seeds[1])));
    app.press(Action::BScan);
    let drawn = standard(&words_of(&app, seeds[1]));
    scan(&mut app, drawn);
    assert!(app.backup_item_done_now(&c, Item::Copy(seeds[1])));
}

#[test]
fn the_chart_checks_a_copy_by_the_form_of_its_line() {
    let chart_has = |app: &mut Faraday, step: Step| {
        app.press(Action::Chart(C::Open(Press::Loaded(0), Target::Key(0))));
        let _ = app.frame();
        let offers = app.offers(Action::Chart(C::Do(Press::Loaded(0), step)));
        app.press(Action::Cancel);
        offers
    };
    let fp_of = |app: &Faraday| app.session.keys[0].fingerprint.0;

    // Words: typed; a camera changes nothing.
    let mut app = backing_up(ATTACK, true, WORDS_ONLY);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let fp = fp_of(&app);
    assert!(chart_has(&mut app, Step::Type(fp, bstep::COPY)));
    assert!(!chart_has(&mut app, Step::Scan(fp)));

    // A SeedQR with a camera: scanned.
    let mut app = backing_up(ATTACK, true, SEEDQR_ONLY);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let fp = fp_of(&app);
    assert!(chart_has(&mut app, Step::Scan(fp)));
    assert!(!chart_has(&mut app, Step::Type(fp, bstep::COPY)));
    // The scan keeps the chart under the camera.
    app.press(Action::Chart(C::Open(Press::Loaded(0), Target::Key(0))));
    app.press(Action::Chart(C::Do(Press::Loaded(0), Step::Scan(fp))));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app.press(Action::Cancel);
    assert_eq!(app.screen, faraday_core::Screen::Wallets);

    // A SeedQR with no camera: typed.
    let mut app = backing_up(ATTACK, false, SEEDQR_ONLY);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let fp = fp_of(&app);
    assert!(chart_has(&mut app, Step::Type(fp, bstep::COPY)));
    assert!(!chart_has(&mut app, Step::Scan(fp)));

    // Both forms, with a camera: a row for each.
    let mut app = backing_up(ATTACK, true, BOTH);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let fp = fp_of(&app);
    assert!(chart_has(&mut app, Step::Type(fp, bstep::COPY)));
    assert!(chart_has(&mut app, Step::Scan(fp)));
}

// ---------------------------------------------------------------------
// A copy checked again and again
// ---------------------------------------------------------------------

/// What the checklist shows and goes on to: every string drawn with its
/// box, the items, the open item and whether the copy's item is done.
type Shown = (
    Vec<(String, (i32, i32, i32, i32))>,
    Vec<u8>,
    Option<u8>,
    bool,
);

fn checklist(app: &mut Faraday) -> Shown {
    let c = faraday_core::testkit::compute();
    app.settle();
    let boxes = app.drawn_text_boxes();
    (
        boxes,
        app.backup_items_now(&c),
        app.backup().and_then(|b| b.open),
        seeds_done(app),
    )
}

/// The copy's words typed in full, `times` over, each time from the
/// foot's Check my copy, or after Clear when `clear`.
fn typed_back(words: &str, times: usize, clear: bool) -> Faraday {
    let mut app = backing_up(words, false, WORDS_ONLY);
    assert!(offered(&mut app, Action::BCheck));
    for _ in 0..times {
        copy_open(&mut app);
        app.press(Action::BCheck);
        if clear {
            testkit::type_words(&mut app, "attack pizza");
            app.press(Action::BCheckClear);
        }
        testkit::type_words(&mut app, words);
        let _ = app.frame();
    }
    // Once or thirty times, the checklist ends past the copy's item.
    if app.backup().and_then(|b| b.open) == Some(faraday_core::bstep::COPY) {
        app.press(Action::BNext(faraday_core::bstep::COPY));
    }
    app
}

/// The copy's item opened again: a match moves the checklist on by
/// itself (`docs/BACKUP-FLOW.md` §17.23).
fn copy_open(app: &mut Faraday) {
    if app.backup().and_then(|b| b.open) != Some(faraday_core::bstep::COPY) {
        app.press(Action::BStep(faraday_core::bstep::COPY));
    }
}

#[test]
fn a_copy_checked_thirty_times_leaves_the_checklist_as_one_check_does() {
    for clear in [false, true] {
        let mut once = typed_back(ATTACK, 1, clear);
        let one = checklist(&mut once);
        assert!(one.3, "the typed copy did not check");
        let mut again = typed_back(ATTACK, 30, clear);
        // The same items, the same one open, the same done: the page's
        // layout follows how the checklist moved on (§17.23), not the
        // check.
        let thirty = checklist(&mut again);
        assert_eq!(
            (thirty.1, thirty.2, thirty.3),
            (one.1.clone(), one.2, one.3),
            "Clear between: {clear}"
        );
        // A copy more, one word short of the last: the check reads the
        // same in both.
        let upto: Vec<&str> = ATTACK.split_whitespace().take(23).collect();
        for app in [&mut once, &mut again] {
            copy_open(app);
            app.press(Action::BCheck);
            testkit::type_words(app, &upto.join(" "));
        }
        let short = checklist(&mut once);
        assert!(
            short.0.iter().any(|(t, _)| t == "Word 24 of 24"),
            "the check does not read one word short: {:?}",
            short.0
        );
        let short_again = checklist(&mut again);
        assert!(
            short_again.0.iter().any(|(t, _)| t == "Word 24 of 24"),
            "Clear between: {clear}"
        );
        assert_eq!(
            (short_again.1, short_again.2, short_again.3),
            (short.1, short.2, short.3),
            "Clear between: {clear}"
        );
    }
}

#[test]
fn a_copy_scanned_thirty_times_leaves_the_checklist_as_one_scan_does() {
    let scanned = |times: usize| {
        let mut app = backing_up(ATTACK, true, SEEDQR_ONLY);
        for _ in 0..times {
            app.press(Action::BScan);
            scan(&mut app, standard(ATTACK));
            let _ = commands(&mut app);
        }
        app
    };
    let mut once = scanned(1);
    let one = checklist(&mut once);
    assert!(one.3, "the scanned copy did not check");
    assert_eq!(checklist(&mut scanned(30)), one);
}

/// While the typed check is open, Check my copy is not offered: the
/// check compares on the last word.
#[test]
fn check_my_copy_is_not_offered_while_the_typed_check_is_open() {
    let mut app = backing_up(ATTACK, true, WORDS_ONLY);
    assert!(offered(&mut app, Action::BCheck), "Check my copy at first");
    app.press(Action::BCheck);
    assert!(
        !offered(&mut app, Action::BCheck),
        "not while the check is open"
    );
}

/// Save as a file under a copy of the words offers the words alone: no
/// SeedQR picture where the plan copies no SeedQR.
#[test]
fn save_as_a_file_offers_no_seedqr_picture_for_a_words_only_copy() {
    let labels = |copies| {
        let mut app = backing_up(ATTACK, true, copies);
        app.press(Action::BFile);
        testkit::secret_out(&app)
            .expect("the file is offered")
            .forms
            .iter()
            .map(|f| f.name.clone())
            .collect::<Vec<_>>()
    };
    let words = labels(WORDS_ONLY);
    assert_eq!(words.len(), 1, "{words:?}");
    assert!(words[0].ends_with("-words.txt"), "{words:?}");
    let both = labels(BOTH);
    assert!(both.iter().any(|n| n.ends_with("seedqr.png")), "{both:?}");
}

// ---------------------------------------------------------------------
// Keep the words only, the three scans, the grid drawn in part (§14.7
// item 3)
// ---------------------------------------------------------------------

/// The forms of seed 0's copies the backup's plan keeps now.
fn plan_forms(app: &Faraday) -> plan::CopyForms {
    let c = testkit::compute();
    let b = app.backup().unwrap();
    let shape = app.plan_shape_now(b.wallet);
    plan::copy_forms(&c, &shape, &b.answers, 0)
}

/// One opening of the camera on the copy that closes without a match:
/// a SeedQR of another seed read, then closed by the person.
fn scan_another(app: &mut Faraday) {
    app.press(Action::BScan);
    scan(app, standard(GOOD));
    assert_eq!(app.sheet, Some(Sheet::Scan), "another seed matched");
    app.press(Action::Cancel);
    let _ = commands(app);
    let _ = app.frame();
}

fn misses(app: &Faraday) -> u8 {
    let b = app.backup().unwrap();
    b.misses_of(app.session.keys[b.key].fingerprint.0)
}

/// A 12-word seed's SeedQR copy item, with a camera, the seed hidden so
/// the item fits the screen.
fn seedqr_hidden() -> Faraday {
    let mut app = backing_up(FORUM, true, SEEDQR_ONLY);
    app.press(Action::BReveal);
    let _ = app.frame();
    app
}

/// Where `text` is drawn, its top.
fn top_of(app: &mut Faraday, text: &str) -> Option<i32> {
    app.settle();
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == text)
        .map(|(_, r)| r.1)
}

#[test]
fn keep_the_words_only_turns_the_seedqr_item_into_a_typed_words_check() {
    let mut app = backing_up(ATTACK, true, SEEDQR_ONLY);
    assert!(
        offered(&mut app, Action::BKeepWords),
        "no Keep the words only"
    );
    app.press(Action::BKeepWords);
    let forms = plan_forms(&app);
    assert!(forms.words && !forms.seedqr, "the plan's copies: {forms:?}");
    assert!(!seeds_done(&app), "the item is done with nothing checked");
    assert_eq!(app.backup().unwrap().open, Some(bstep::COPY));
    // Now typed back: no scan, no Keep the words only, no grid.
    assert!(offered(&mut app, Action::BCheck));
    assert!(!app.offers(Action::BScan), "the scan is still asked for");
    assert!(!app.offers(Action::BKeepWords));
    app.press(Action::BReveal);
    let _ = app.frame();
    assert!(
        !shown(&mut app, "Standard SeedQR"),
        "the grid is still drawn"
    );
    app.press(Action::BCheck);
    testkit::type_words(&mut app, ATTACK);
    assert!(
        seeds_done(&app),
        "the typed words did not complete the item"
    );
}

#[test]
fn keep_the_words_only_with_the_words_already_checked_completes_the_item_at_once() {
    let mut app = backing_up(ATTACK, true, BOTH);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, ATTACK);
    assert!(!seeds_done(&app), "the words alone checked both copies");
    assert!(offered(&mut app, Action::BKeepWords));
    app.press(Action::BKeepWords);
    assert!(
        seeds_done(&app),
        "the item waits for a check already passed"
    );
    assert_ne!(
        app.backup().unwrap().open,
        Some(bstep::COPY),
        "the checklist did not move on"
    );
    let forms = plan_forms(&app);
    assert!(forms.words && !forms.seedqr, "{forms:?}");
}

#[test]
fn three_scans_without_a_match_bring_keep_the_words_only_to_the_foot() {
    let mut app = seedqr_hidden();
    for n in 1..=3u8 {
        assert!(
            !says(&mut app, "scans without a match"),
            "said before the third scan"
        );
        let body = top_of(&mut app, "Keep the words only").expect("Keep the words only");
        let foot = top_of(&mut app, "Check my copy").expect("Check my copy");
        assert!(body < foot, "Keep the words only is not in the body");
        scan_another(&mut app);
        assert_eq!(misses(&app), n);
    }
    assert!(shown(&mut app, "3 scans without a match"));
    let line = top_of(&mut app, "3 scans without a match").unwrap();
    let keep = top_of(&mut app, "Keep the words only").expect("Keep the words only");
    assert!(keep > line, "Keep the words only did not move to the foot");
    assert!(offered(&mut app, Action::BKeepWords));
    assert!(app.offers(Action::BScan), "Check my copy left the foot");
    // A match closes the camera itself, and is no scan without a match.
    app.press(Action::BScan);
    scan(&mut app, standard(FORUM));
    assert_eq!(misses(&app), 3);
    assert!(seeds_done(&app));
}

#[test]
fn a_scan_closed_by_the_person_counts_once_however_many_codes_it_read() {
    let mut app = backing_up(FORUM, true, SEEDQR_ONLY);
    app.press(Action::BScan);
    scan(&mut app, standard(GOOD));
    scan(&mut app, b"not a seedqr".to_vec());
    app.event(Event::Key(Key::Escape));
    assert!(app.scan().is_none());
    assert_eq!(misses(&app), 1);
}

#[test]
fn a_lock_drops_the_count_of_scans_without_a_match() {
    let mut app = seedqr_hidden();
    for _ in 0..3 {
        scan_another(&mut app);
    }
    assert!(top_of(&mut app, "3 scans without a match").is_some());
    app.press(Action::Lock);
    assert!(app.backup().is_none());
    let mut app = seedqr_hidden();
    assert_eq!(misses(&app), 0);
    assert!(!says(&mut app, "scans without a match"));
    let body = top_of(&mut app, "Keep the words only").expect("Keep the words only");
    let foot = top_of(&mut app, "Check my copy").expect("Check my copy");
    assert!(body < foot, "Keep the words only stayed at the foot");
}

#[test]
fn the_grid_dropped_for_the_words_stays_on_the_map_drawn_in_part_until_destroyed() {
    use faraday_core::glance::{self, Backup};
    use faraday_core::plan::{Edit, Mark, What};
    let c = testkit::compute();
    let mut app = backing_up(ATTACK, true, SEEDQR_ONLY);
    let fp = app.session.keys[0].fingerprint.to_string();
    app.press(Action::BKeepWords);
    let line = format!("Seed {fp}'s SeedQR, drawn in part, stays on the map until destroyed");
    assert!(says(&mut app, &line), "the item does not say so");
    assert_eq!(
        app.drawn_texts()
            .iter()
            .filter(|t| t.contains(&line))
            .count(),
        1,
        "said more than once"
    );
    // On the chart: the SeedQR line, marked drawn in part.
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let part_line = |app: &Faraday| {
        let g = glance::of_now(&c, app, 0).expect("a chart");
        let Backup::Plan { nodes, .. } = g.backup else {
            panic!("no plan on the chart");
        };
        nodes.iter().enumerate().find_map(|(n, node)| {
            let j = node
                .whats
                .iter()
                .zip(&node.marks)
                .position(|(w, m)| *w == What::SeedQr(0) && *m == Some(Mark::Part))?;
            Some((n as u8, j as u8, node.at))
        })
    };
    let (n, j, at) = part_line(&app).expect("no SeedQR drawn in part on the chart");
    let press = Press::Loaded(0);
    app.press(Action::Chart(C::Open(press, Target::Line(n, j))));
    let _ = app.frame();
    assert!(shown(&mut app, "Drawn in part"));
    let destroy = Action::Chart(C::Edit(press, Edit::Drop(at, What::SeedQr(0))));
    assert!(app.offers(destroy), "no I destroyed this copy");
    app.press(destroy);
    let _ = app.frame();
    let confirm = Action::Chart(C::Confirm(press));
    assert!(app.offers(confirm), "no Confirm");
    app.press(confirm);
    let _ = app.frame();
    assert!(part_line(&app).is_none(), "the line stayed once destroyed");
    assert!(app.backup().unwrap().answers.part.is_empty());
    // The words stay where they were.
    let forms = plan_forms(&app);
    assert!(forms.words && !forms.seedqr, "{forms:?}");
}

#[test]
fn a_grid_drawn_in_part_is_a_secret_where_it_is_and_no_copy_that_rebuilds() {
    use faraday_core::plan::{
        Answers, At, Found, Lost, Seed, Shape, Spot, Tag, What, alone, check,
    };
    let c = testkit::compute();
    let shape = Shape {
        m: 1,
        keys: 1,
        seeds: vec![Seed {
            name: "aaaa0001".into(),
            here: true,
            passphrase: false,
        }],
        splits: false,
        vaults: 0,
    };
    // The words at place 1; place 2 keeps the wallet sheet, and the grid
    // as `second` says.
    let plan_with = |second: Option<What>, part: bool| {
        let mut a = Answers::defaults(&shape);
        let mut two = vec![(What::Sheet, Tag::Public)];
        if let Some(w) = second {
            two.push((w, Tag::Secret));
        }
        a.set_map(vec![
            Spot {
                at: At::Place(0),
                holds: vec![(What::Words(0), Tag::Secret), (What::Sheet, Tag::Public)],
            },
            Spot {
                at: At::Place(1),
                holds: two,
            },
        ]);
        if part {
            a.part.push((1, 0));
        }
        a
    };
    let drawn_in_part = plan_with(None, true);
    let nothing = plan_with(None, false);
    let whole = plan_with(Some(What::SeedQr(0)), false);
    // One place found: the grid drawn in part is the seed there.
    assert_eq!(
        alone(&c, &shape, &drawn_in_part, At::Place(1)).spend,
        Found::Yes
    );
    assert_eq!(alone(&c, &shape, &nothing, At::Place(1)).spend, Found::No);
    assert_eq!(check(&c, &shape, &drawn_in_part).spend, Found::Yes);
    // Any one place lost: a whole grid rebuilds, one drawn in part does
    // not.
    assert_eq!(check(&c, &shape, &whole).lost, Lost::Yes);
    assert_eq!(check(&c, &shape, &drawn_in_part).lost, Lost::No);
    // It is no copy the checklist asks to check.
    let forms = plan::copy_forms(&c, &shape, &drawn_in_part, 0);
    assert!(forms.words && !forms.seedqr, "{forms:?}");
    // The plan keeps it as the vault reads it back.
    let back = Answers::from_text(&shape, &drawn_in_part.to_text()).unwrap();
    assert_eq!(back.part, vec![(1, 0)]);
}

#[test]
fn keep_the_words_only_keeps_a_plan_of_answers_its_answers_and_edits_an_edited_one() {
    use faraday_core::plan::{Answers, At, Seed, Shape, Spot, Tag, What, keep_words, seeds};
    let c = testkit::compute();
    let shape = Shape {
        m: 1,
        keys: 1,
        seeds: vec![Seed {
            name: "aaaa0001".into(),
            here: true,
            passphrase: false,
        }],
        splits: false,
        vaults: 0,
    };
    // The seeds question answered SeedQR alone: the rows swap, no edits.
    let mut a = Answers::defaults(&shape);
    a.seeds[seeds::WORDS] = false;
    a.seeds[seeds::SEEDQR] = true;
    let kept = keep_words(&shape, &a, 0);
    assert!(kept.map.is_none(), "the plan became its map");
    assert!(kept.seeds[seeds::WORDS] && !kept.seeds[seeds::SEEDQR]);
    assert_eq!(kept.part, vec![(0, 0)]);
    let forms = plan::copy_forms(&c, &shape, &kept, 0);
    assert!(forms.words && !forms.seedqr, "{forms:?}");
    // An edited map: the SeedQR becomes the words, not added twice where
    // the words are already.
    let mut e = Answers::defaults(&shape);
    e.set_map(vec![
        Spot {
            at: At::Place(0),
            holds: vec![(What::SeedQr(0), Tag::Secret)],
        },
        Spot {
            at: At::Place(1),
            holds: vec![
                (What::Words(0), Tag::Secret),
                (What::SeedQr(0), Tag::Secret),
            ],
        },
    ]);
    let kept = keep_words(&shape, &e, 0);
    let map = plan::map(&c, &shape, &kept);
    for p in 0..2 {
        let held: Vec<What> = map
            .iter()
            .filter(|s| s.at == At::Place(p))
            .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
            .collect();
        assert_eq!(held, vec![What::Words(0)], "place {p}");
    }
    assert_eq!(kept.part, vec![(0, 0), (1, 0)]);
}
