//! Layout tests for the screens pass A rebuilt (UX.md §5, §6): the app
//! bar, the Home grid, the key menu, the address explorer and the
//! chunking rule, at the two portrait reference sizes.

mod common;

/// One `#[test]` per reference size over a function taking the display.
/// nextest runs every test in a process of its own, so a walk written
/// this way spreads over the cores instead of holding one while the
/// rest wait (`docs/PLANNING.md` §16.119 rule 4). What each walk
/// asserts is what it asserted as one test.
macro_rules! at_every_size {
    ($at:ident => $tiny:ident, $panel:ident, $phone:ident, $desktop:ident) => {
        #[test]
        fn $tiny() {
            $at(SIZES[0]);
        }

        #[test]
        fn $panel() {
            $at(SIZES[1]);
        }

        #[test]
        fn $phone() {
            $at(SIZES[2]);
        }

        #[test]
        fn $desktop() {
            $at(SIZES[3]);
        }
    };
}

use common::{
    ABANDON, DESKTOP, Element, Harness, PANEL, PHONE, PIN, SECURE_PHONE, TILE_TOOLS, TINY,
    UNVERIFIED_PHONE,
};
use opensigner_core::codes;
use opensigner_core::create::Step as CreateStep;
use opensigner_core::ids;
use opensigner_core::quiz::QuizState;
use opensigner_core::sign::{QrMode, Stage as SignStage, Step as SignStep};
use opensigner_core::{AssuranceTier, ScreenKind, strings};
use osk_shell_api::{App, DisplayInfo, Event, FileKind, Key, TouchPhase};
use osk_ui::widgets::keyboard::KeyInput;
use osk_ui::widgets::{is_chunked, tokens};

const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");
const MESSAGE: &[u8] = include_bytes!("../../../tools/vectors/message.txt");
const SIGNED_MESSAGE: &[u8] = include_bytes!("../../../tools/vectors/signed-message.txt");
const WALLET_POLICY: &[u8] = include_bytes!("../../../tools/vectors/psbt/wallet-2of3.policy");
/// The committed regtest group's first share, as its 24 words.
const SHARE_0_WORDS: &str =
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-0.txt");

/// An LND cipher seed at the scrypt cost a released LND uses, from
/// `tools/reference/aezeed/README.md`. Its passphrase is empty.
const LND_CIPHER_SEED: &str = "above judge emerge veteran reform crunch system all snap please \
    shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";

/// One cosigner of that wallet, as the text the builder's scanner reads.
fn cosigner_key() -> Vec<u8> {
    let policy = osk_bip::policy::WalletPolicy::parse(
        core::str::from_utf8(WALLET_POLICY).expect("the committed policy"),
    )
    .expect("the committed policy");
    policy.keys()[1].key_text().into_bytes()
}

/// The size class a display is drawn at.
fn class_of(display: DisplayInfo) -> osk_ui::SizeClass {
    osk_ui::SizeClass::of(&display)
}

/// dp to pixels on a display.
fn px(display: DisplayInfo, dp: f32) -> i32 {
    (dp * f32::from(display.dpi) / 160.0 + 0.5) as i32
}

fn loaded(display: DisplayInfo) -> Harness {
    let mut h = Harness::new(display);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h
}

#[test]
fn no_home_tile_is_below_the_fold_at_either_small_size() {
    for display in [PANEL, TINY] {
        let h = loaded(display);
        assert_eq!(h.app.screen(), ScreenKind::Home);
        for i in 0..3 {
            let r = h
                .app
                .rect_of(ids::at(ids::HOME_TILE_BASE, i))
                .unwrap_or_else(|| panic!("tile {i} on {}x{}", display.width, display.height));
            assert!(
                r.bottom() <= i32::from(display.height),
                "tile {i} is below the fold on {}x{}",
                display.width,
                display.height
            );
        }
    }
}

/// §5 Menu is a list, and the key page's rows are more than the two
/// panel sizes have the height for, so it is a list that scrolls: every
/// row is reachable and none is stranded below the fold of a fixed
/// frame.
#[test]
fn every_row_of_the_key_page_is_reachable() {
    for display in [PANEL, TINY] {
        let mut h = loaded(display);
        h.open_key(0);
        assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
        let rows = [ids::DETAIL_BACKUP, ids::KEY_FORGET];
        for id in rows {
            let r = h
                .app
                .reveal(id)
                .unwrap_or_else(|| panic!("row {} on {}", id.0, display.width));
            assert!(
                r.bottom() <= i32::from(display.height),
                "row {} cannot be brought into view on {}x{}",
                id.0,
                display.width,
                display.height
            );
        }
        assert_document(&h, "key detail", display);
    }
}

/// §5 Addresses is a list: the script-type value row, the Receive |
/// Change pair, one reference row per address, and a "More" row that
/// extends the run. §4.1's pager is gone, and nothing is below the fold.
#[test]
fn the_addresses_screen_is_a_list_that_grows() {
    for display in [PANEL, TINY, PHONE] {
        let mut h = loaded(display);
        h.open_single_sig(0);
        h.tap(ids::WALLET_ADDRESSES);
        assert_eq!(h.app.screen(), ScreenKind::Addresses);
        for id in [ids::ADDR_RECEIVE, ids::ADDR_CHANGE] {
            let r = h.app.reveal(id).expect("a half of the pair");
            assert!(r.h >= px(display, tokens::TOUCH), "still a touch target");
        }
        // Ten rows first; "More" adds ten more, and the list stays a
        // list — one address per row, all of it inside its region.
        for i in 0..10 {
            h.app
                .reveal(ids::at(ids::ADDR_ROW_BASE, i))
                .unwrap_or_else(|| panic!("address row {i}"));
        }
        assert!(h.app.rect_of(ids::at(ids::ADDR_ROW_BASE, 10)).is_none());
        h.tap(ids::ADDR_MORE);
        h.app
            .reveal(ids::at(ids::ADDR_ROW_BASE, 19))
            .expect("the twentieth address");
        h.drain();
        assert_document(&h, "addresses", display);
    }
}

/// §5 Address: a row of the list opens one address on a screen of its
/// own — the QR, the label and the whole string — and the chevron is the
/// way back.
#[test]
fn the_address_detail_shows_the_whole_string_and_its_code() {
    for display in [PANEL, TINY, PHONE, DESKTOP] {
        let where_ = format!("{}x{}", display.width, display.height);
        let mut h = loaded(display);
        h.open_single_sig(0);
        h.tap(ids::WALLET_ADDRESSES);
        h.tap(ids::at(ids::ADDR_ROW_BASE, 2));
        assert_eq!(h.app.qr_visible(), Some(true), "the code is drawn");
        assert_chunked_whole(&h, ids::ADDR_TEXT, "the address detail", &where_);
        // The label names which address it is, above the string.
        assert!(
            h.app.texts().iter().any(|t| t == "Receive 2"),
            "the address is named: {:?}",
            h.app.texts()
        );
        assert_one_screenful(&h, "the address detail", display);
        h.tap(ids::BACK);
        assert_eq!(h.app.screen(), ScreenKind::Addresses);
        assert!(h.app.rect_of(ids::at(ids::ADDR_ROW_BASE, 0)).is_some());
    }
}

/// §5 Menu for the wallet export: the format, the script type and the
/// path as rows, the string as a reference row, and the QR one row away.
/// Nothing pages, and the whole screen is one screenful on the panel.
#[test]
fn the_wallet_export_is_a_menu_with_its_string_one_tap_away() {
    for display in [PANEL, TINY, PHONE, DESKTOP] {
        let mut h = loaded(display);
        h.open_single_sig(0);
        h.tap(ids::WALLET_EXPORT);
        assert_eq!(h.app.screen(), ScreenKind::Export);
        for id in [ids::EXPORT_FORMAT, ids::EXPORT_TEXT, ids::EXPORT_SHOW] {
            h.app
                .reveal(id)
                .unwrap_or_else(|| panic!("export row {} at {}", id.0, display.width));
        }
        h.drain();
        assert_document(&h, "wallet export", display);
        h.tap(ids::EXPORT_SHOW);
        assert_one_screenful(&h, "the export code", display);
        h.tap(ids::BACK);
        assert_eq!(h.app.screen(), ScreenKind::Export);
    }
}

/// The lock card holds the pad, the field and the four lines above them
/// with nothing over its edge, before and after a wrong PIN (UX.md I1).
#[test]
fn the_lock_card_fits_the_panel() {
    for display in [PANEL, TINY] {
        let mut h = loaded(display);
        h.tap(ids::STATUS_LOCK);
        assert!(h.app.is_locked());
        assert_eq!(h.app.screen(), ScreenKind::Lock);
        assert_one_screenful(&h, "the lock screen", display);
        let pad = h.app.rect_of(ids::LOCK_KEYBOARD).expect("the PIN pad");
        assert!(
            pad.bottom() <= i32::from(display.height),
            "the pad is on screen at {}x{}",
            display.width,
            display.height
        );
        // A wrong PIN goes on the reserved caption line under the
        // field, and nothing moves for it (§4.3).
        let before = h.app.placed_rects();
        h.type_pin(ids::LOCK_KEYBOARD, "1111");
        assert!(h.app.is_locked());
        let texts = h.app.texts();
        let wrong = strings::fill1(strings::EN.lock_wrong, "7");
        assert!(texts.contains(&wrong), "{texts:?}");
        assert_eq!(h.app.placed_rects(), before, "the wrong PIN moved the card");
        assert_one_screenful(&h, "the lock screen after a wrong PIN", display);
    }
}

#[test]
fn fingerprints_are_never_chunked_and_addresses_are() {
    let mut h = loaded(PANEL);
    let fingerprint = h.app.fingerprints()[0];
    let hex = core::str::from_utf8(&fingerprint.to_hex())
        .unwrap()
        .to_owned();
    assert_eq!(hex.len(), 8);
    assert!(!is_chunked(&hex));
    let address = h
        .app
        .addresses(0, osk_bip::keys::ScriptType::NativeSegwit, false, 1)[0]
        .clone();
    assert!(is_chunked(&address));
    // Sixteen characters is the threshold.
    assert!(!is_chunked(&"0".repeat(16)));
    assert!(is_chunked(&"0".repeat(17)));
    h.open_key(0);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
}

/// Every word the rebuilt screens say comes from `strings/en.rs` and is
/// on the screen the wording was written for: the questions the Choice
/// steps ask, the checksum result and its record labels, and the two
/// lines the reserved caption line carries.
#[test]
fn every_string_is_reachable_through_the_app() {
    let s = &strings::EN;
    let says = |h: &Harness, want: &str, screen: &str| {
        let texts = h.app.texts();
        assert!(
            texts.iter().any(|t| t == want),
            "{screen} never says {want:?}: {texts:?}"
        );
    };
    let mut h = Harness::new(PANEL);
    h.open_load();
    says(&h, s.load_source_title, "the source step");
    says(&h, s.load_source_type, "the source step");
    h.choose(ids::LOAD_SOURCE_TYPE, ids::LOAD_SOURCE_CONTINUE);
    says(&h, s.load_count_title, "the count step");
    says(&h, s.action_continue, "the count step");
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    says(&h, s.load_language_title, "the language step");
    h.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
    says(
        &h,
        &strings::fill(s.load_word_title, &["1", "12"]),
        "word entry",
    );
    for w in ABANDON {
        h.type_text(w);
        h.key(osk_shell_api::Key::Enter);
    }
    says(&h, s.load_checksum_valid, "the checksum result");
    says(&h, s.load_words_row, "the checksum result");
    says(&h, s.load_language_row, "the checksum result");
    h.tap(ids::LOAD_CONTINUE);
    says(&h, s.passphrase_offer_title, "the passphrase offer");
    says(&h, s.passphrase_none, "the passphrase offer");
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    says(&h, s.confirm_title, "the confirm");
    says(&h, s.confirm_key, "the confirm");
    says(&h, s.confirm_network, "the confirm");
    says(&h, s.confirm_add, "the confirm");
    h.tap(ids::LOAD_HOLD);
    says(&h, s.pin_set_title, "the set-PIN step");
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2580");
    says(&h, s.pin_repeat_title, "the repeat-PIN step");
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2581");
    says(&h, s.pin_mismatch, "the set-PIN step after a mismatch");
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2580");
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2580");
    h.open_key(0);
    says(&h, s.detail_backup, "the key menu");
    says(&h, s.detail_backup_unverified, "the key menu");
    h.tap(ids::BACK);
    h.tap(ids::KEYS_ADD);
    says(&h, s.keys_add, "Add a key");
    says(&h, s.home_load_key, "Add a key");
    says(&h, s.home_create_key, "Add a key");
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::STATUS_LOCK);
    says(&h, s.lock_title, "the lock screen");
    h.type_pin(ids::LOCK_KEYBOARD, "1111");
    says(
        &h,
        &strings::fill1(s.lock_wrong, "7"),
        "the lock screen after a wrong PIN",
    );

    // The Create wizard and the Backup flow.
    let mut h = Harness::new(PANEL);
    h.open_create();
    says(&h, s.create_source_title, "the source step");
    says(&h, s.create_source_dice, "the source step");
    says(&h, s.create_source_cards, "the source step");
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 0),
        ids::CREATE_SOURCE_CONTINUE,
    );
    says(&h, s.create_count_title, "the count step");
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    says(&h, s.create_language_title, "the language step");
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    says(&h, s.dice_procedure_title, "the procedure step");
    says(&h, s.dice_procedure_hashed, "the procedure step");
    says(&h, s.dice_procedure_six_as_zero, "the procedure step");
    says(&h, s.dice_procedure_words, "the procedure step");
    says(
        &h,
        &strings::fill1(s.dice_procedure_rolls, "50"),
        "the procedure step",
    );
    says(
        &h,
        &strings::fill1(s.dice_procedure_rolls, "72"),
        "the procedure step",
    );
    h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    says(
        &h,
        &strings::fill(s.create_dice_title, &["1", "50"]),
        "the dice pad",
    );
    says(
        &h,
        &strings::fill(s.create_entry_count, &["0", "50", "0"]),
        "the dice pad",
    );
    h.type_text("66666666351521144121541512665155412152342515356215");
    h.key(osk_shell_api::Key::Enter);
    says(&h, s.create_sanity_title, "the sanity check");
    says(&h, s.create_sanity_uneven, "the sanity check");
    says(&h, s.create_rolls_row, "the sanity check");
    says(&h, s.create_bits_row, "the sanity check");
    says(&h, s.create_chi_square, "the sanity check");
    says(&h, s.create_longest_run, "the sanity check");
    says(&h, s.create_math_row, "the sanity check");
    says(&h, s.create_sanity_again, "the sanity check");
    h.tap(ids::CREATE_MATH);
    says(&h, s.create_math_title, "the math");
    says(&h, s.dice_procedure_row, "the math");
    says(&h, s.create_math_entropy, "the math");
    says(&h, s.create_math_checksum, "the math");
    says(&h, &strings::fill1(s.create_math_bits, "4"), "the math");
    says(&h, &strings::fill1(s.create_math_word, "1"), "the math");
    h.tap(ids::CREATE_MATH_ENTROPY);
    says(&h, s.create_math_entropy, "the entropy screen");
    h.tap(ids::BACK);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::CREATE_CONTINUE);
    // §5 Words: the title is the thing, the pager names the page, and
    // §4.13 shortens the secondary only where the label does not fit.
    says(&h, s.words_title, "the words");
    says(
        &h,
        &strings::fill(s.words_range_title, &["1", "6"]),
        "the words pager",
    );
    says(&h, s.words_numbers_short, "the words on a panel");
    h.tap(ids::CREATE_CONTINUE);
    says(&h, s.quiz_start_title, "the quiz start");
    says(&h, s.quiz_helper_toggle, "the quiz start");
    says(&h, s.quiz_start, "the quiz start");
    says(&h, s.quiz_skip, "the quiz start");
    h.tap(ids::QUIZ_SKIP);
    says(&h, s.quiz_failed_title, "the skip caution");
    says(&h, s.quiz_row, "the skip caution");
    says(&h, s.quiz_skipped, "the skip caution");
    says(&h, s.quiz_skip_confirm, "the skip caution");
    says(&h, s.quiz_skip_cancel, "the skip caution");
    h.tap(ids::QUIZ_SKIP_CANCEL);
    h.tap(ids::QUIZ_HELPER);
    h.tap(ids::QUIZ_START);
    let v = h.app.quiz_view().expect("a quiz");
    let word = format!("{}", v.word_number);
    let count = strings::fill(s.words_page, &["1", "12"]);
    // On a 268 dp panel the title carries the run's progress and there
    // is no progress row; the taller classes keep the question whole.
    says(
        &h,
        &strings::fill1(
            s.quiz_helper_title,
            &strings::fill(s.quiz_question_short, &[&word, &count]),
        ),
        "a quiz question",
    );
    let right = v.choices.iter().position(|c| *c == v.choices[0]).unwrap();
    h.choose(
        ids::at(ids::QUIZ_CHOICE_BASE, (right + 1) % 4),
        ids::QUIZ_CONTINUE,
    );
    if h.app
        .quiz_view()
        .is_some_and(|v| v.state == QuizState::Wrong)
    {
        says(&h, s.quiz_wrong_title, "a wrong answer");
        says(&h, s.quiz_question_row, "a wrong answer");
        says(&h, s.quiz_show_words, "a wrong answer");
        says(&h, s.quiz_retry, "a wrong answer");
        h.tap(ids::QUIZ_RETRY);
    }

    // The Backup flow of a loaded key.
    let mut h = loaded(PANEL);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    says(&h, s.backup_title, "the backup menu");
    says(&h, s.backup_words, "the backup menu");
    says(&h, s.backup_seedqr, "the backup menu");
    says(&h, s.backup_compact, "the backup menu");
    says(&h, s.backup_verify, "the backup menu");
    says(&h, s.detail_backup_unverified, "the backup menu");
    h.tap(ids::BACKUP_SEEDQR);
    says(&h, s.seedqr_title, "the SeedQR");
    h.tap(ids::BACK);
    h.tap(ids::BACKUP_COMPACT);
    says(&h, s.compact_title, "the CompactSeedQR");
    h.tap(ids::BACK);
    h.tap(ids::BACKUP_WORDS);
    says(&h, s.action_done, "the words");
    h.tap(ids::QUIZ_DONE);
    h.tap(ids::BACKUP_VERIFY);
    h.tap(ids::QUIZ_START);
    h.tap(ids::BACK);
    says(&h, s.quiz_failed_title, "a quiz that did not finish");
    says(&h, s.quiz_questions_row, "a quiz that did not finish");
    says(&h, s.quiz_again, "a quiz that did not finish");
    h.tap(ids::QUIZ_AGAIN);
    h.tap(ids::QUIZ_START);
    for _ in 0..12 {
        let v = h.app.quiz_view().expect("a quiz");
        let want = ABANDON[v.word_number - 1];
        let slot = v.choices.iter().position(|c| c == want).expect("the word");
        h.choose(ids::at(ids::QUIZ_CHOICE_BASE, slot), ids::QUIZ_CONTINUE);
    }
    says(&h, s.quiz_passed_title, "a passed quiz");
    says(&h, s.load_words_row, "a passed quiz");

    // A phone has the room for the question whole, the progress row
    // under it and the verdict with its count.
    let mut h = Harness::new(PHONE);
    h.open_create();
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 0),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    h.type_text("66666666351521144121541512665155412152342515356215");
    h.key(osk_shell_api::Key::Enter);
    // The verdict carries the count where the row has the width for it.
    let verdict = strings::fill1(s.create_stat_normal, "50");
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.contains(&verdict)),
        "the sanity check on a phone never says {verdict:?}: {texts:?}"
    );
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::QUIZ_START);
    let v = h.app.quiz_view().expect("a quiz");
    says(
        &h,
        &strings::fill1(s.quiz_question, &format!("{}", v.word_number)),
        "a quiz question on a phone",
    );
    says(
        &h,
        &strings::fill(s.words_page, &["1", "12"]),
        "the progress row on a phone",
    );

    // Coins and hex have their own titles, rows and inline errors.
    let mut h = Harness::new(PANEL);
    h.open_create();
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 1),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    says(
        &h,
        &strings::fill(s.create_coin_title, &["1", "128"]),
        "the coin pad",
    );
    let flips: String = (0..128)
        .map(|i| if i % 10 < 5 { 'h' } else { 't' })
        .collect();
    h.type_text(&flips);
    h.key(osk_shell_api::Key::Enter);
    says(&h, s.create_sanity_random, "the coin sanity check");
    says(&h, s.create_flips_row, "the coin sanity check");
    says(&h, s.create_heads, "the coin sanity check");
    says(&h, s.create_tails, "the coin sanity check");

    let mut h = Harness::new(PANEL);
    h.open_create();
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 2),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    says(&h, s.create_hex_title, "the hex entry");
    h.type_text("0");
    h.key(osk_shell_api::Key::Enter);
    says(&h, s.create_too_short, "the hex entry before the length");
    h.type_text("0000000000000000000000000000000");
    h.key(osk_shell_api::Key::Enter);
    says(&h, s.create_sanity_unchecked, "the hex sanity check");
    says(&h, s.create_digits_row, "the hex sanity check");

    // App pass 4: the scanner, Inspect, Settings, About, the tiers and
    // the two wipes.
    let mut h = loaded(PANEL);
    h.tap(ids::HOME_SCAN);
    says(&h, s.scan_title, "the scanner");
    says(&h, s.scan_looking, "the scanner");
    says(&h, s.scan_from_file, "the scanner");
    h.tap(ids::SCAN_FILE);
    says(&h, s.scan_waiting, "the scanner waiting for a file");
    h.send(osk_shell_api::Event::FileUnavailable {
        kind: osk_shell_api::FileKind::Any,
    });
    h.send(osk_shell_api::Event::CameraUnavailable);
    says(&h, s.scan_no_camera, "the scanner without a camera");
    h.tap(ids::BACK);

    h.tap(ids::HOME_SCAN);
    h.scan_bytes(b"hello, world");
    says(&h, s.scan_unknown_title, "an unknown QR");
    says(&h, s.row_bytes, "an unknown QR");
    says(&h, s.row_hex, "an unknown QR");
    says(&h, s.scan_as_psbt, "an unknown QR");
    says(&h, s.scan_as_text, "an unknown QR");
    h.tap(ids::SCAN_AS_TEXT);
    says(&h, s.inspect_text, "the scanned text");
    says(&h, s.inspect_length_row, "the scanned text");
    says(
        &h,
        &strings::fill1(s.inspect_length, "12"),
        "the scanned text",
    );
    says(&h, s.action_done, "the scanned text");
    h.tap(ids::BACK);

    h.tap(ids::HOME_SCAN);
    h.scan_bytes(ABANDON.join(" ").as_bytes());
    says(&h, s.scan_words_title, "the words caution");
    says(&h, s.load_words_row, "the words caution");
    says(&h, s.scan_words_load, "the words caution");
    h.tap(ids::BACK);
    // Twelve digits per word and none of them a word: not a SeedQR.
    h.scan_bytes(b"999999999999999999999999999999999999999999999999");
    says(&h, s.scan_error_title, "a QR that could not be used");
    says(&h, s.row_qr, "a QR that could not be used");
    says(&h, s.scan_reason_seedqr, "a QR that could not be used");
    says(&h, s.action_again, "a QR that could not be used");
    h.tap(ids::BACK);
    h.tap(ids::BACK);

    // The three titles the expected type gives the scanner (§4.9), on a
    // shell whose camera is still there.
    let mut h = loaded(PANEL);
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    says(
        &h,
        s.scan_title_address,
        "the scanner the address check opened",
    );
    h.tap(ids::BACK);
    h.tap(ids::WALLET_SIGN);
    says(
        &h,
        s.scan_title_psbt,
        "the scanner Sign a transaction opened",
    );
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.open_load();
    h.choose(ids::LOAD_SOURCE_SCAN, ids::LOAD_SOURCE_CONTINUE);
    says(&h, s.scan_title_seed, "the scanner Load opened");
    h.tap(ids::BACK);
    h.tap(ids::BACK);

    // Settings, its Setting Choices, About and the tiers.
    h.open_settings();
    for want in [
        s.settings_title,
        s.settings_network,
        s.settings_unit,
        s.settings_unit_sat,
        s.settings_lock_after,
        s.settings_wipe_after,
        s.settings_camera_rotation,
        s.settings_scramble,
        s.settings_lock_now,
        s.settings_memory_key,
        s.settings_memory_key_ok,
        s.settings_about,
        s.settings_wipe_row,
        s.settings_exit_row,
    ] {
        says(&h, want, "Settings");
    }
    h.tap(ids::SETTINGS_UNIT_ROW);
    says(&h, s.settings_unit_btc, "the unit Choice");
    h.tap(ids::BACK);
    h.tap(ids::SETTINGS_WIPE_AFTER_ROW);
    says(&h, s.settings_never, "the auto-wipe Choice");
    h.tap(ids::BACK);
    h.tap(ids::SETTINGS_ABOUT_ROW);
    let checks = format!("{}", osk_selftest::CHECKS.len());
    for want in [
        s.settings_version,
        s.settings_tier,
        s.settings_core_hash,
        s.settings_core_hash_none,
        s.settings_selftest,
        s.settings_selftest_run,
    ] {
        says(&h, want, "About");
    }
    says(
        &h,
        &strings::fill1(s.settings_selftest_passed, &checks),
        "About",
    );
    h.tap(ids::ABOUT_TIER);
    says(&h, s.tier_title, "the tiers");
    says(
        &h,
        &strings::fill1(s.tier_this_device, s.tier_c),
        "the tiers",
    );
    says(&h, s.tier_c_statement, "the tiers");
    says(&h, s.tier_a, "the tiers");
    says(&h, s.tier_a_statement, "the tiers");
    h.tap(ids::BACK);
    h.tap(ids::BACK);

    // The two wipes and what they leave.
    h.tap(ids::SETTINGS_WIPE_ROW);
    says(&h, s.settings_wipe_row, "the wipe screen");
    says(&h, s.row_keys, "the wipe screen");
    says(&h, s.settings_wipe_hold, "the wipe screen");
    h.hold(ids::SETTINGS_WIPE);
    says(&h, s.settings_wiped_title, "the wipe result");
    h.tap(ids::SETTINGS_WIPED_DONE);

    let mut h = loaded(PANEL);
    h.open_settings();
    h.tap(ids::SETTINGS_EXIT_ROW);
    says(&h, s.settings_exit_title, "the wipe-and-exit screen");
    says(&h, s.settings_exit_hold, "the wipe-and-exit screen");
    h.hold(ids::SETTINGS_EXIT);
    says(&h, s.settings_ended_title, "the session's end");
    says(&h, s.settings_ended_keys, "the session's end");

    // A shell with no entropy and no key: the session key says so, and
    // the rows that need a key carry their reason (§4.11).
    let mut h = Harness::without_entropy(PANEL);
    h.open_settings();
    says(&h, s.settings_memory_key_weak, "a weak memory key");
    says(&h, s.reason_needs_key, "Settings with no key");

    // The Sign screens' terms; what "Replaceable" and "Locktime" mean
    // is a Learn page.
    let mut h = sign_demo(PANEL);
    h.tap(ids::SIGN_CONTINUE);
    h.tap(ids::SIGN_CONTINUE);
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(
        h.app.sign_stage(),
        Some(SignStage::Wizard(SignStep::Inputs))
    );
    says(&h, &strings::fill1(s.sign_input_row, "0"), "the inputs");
    says(&h, s.sign_rbf, "the inputs");
    says(&h, s.sign_locktime, "the inputs");
    h.tap(ids::SIGN_CONTINUE);
    h.hold(ids::SIGN_HOLD);
    h.tap(ids::SIGN_SIGNATURES);
}

/// §5 Compare: "the title names the kind, the label the instance." No
/// Compare screen the snapshot scripts reach says the same thing twice.
#[test]
fn no_compare_screen_says_the_same_text_twice() {
    let unique = |h: &Harness, screen: &str| {
        let texts = h.app.texts();
        for (i, a) in texts.iter().enumerate() {
            for b in texts.iter().skip(i + 1) {
                assert_ne!(a, b, "{screen} says {a:?} twice: {texts:?}");
            }
        }
    };
    let s = &strings::EN;

    // The recipient's address: the kind is "Address", the instance the
    // output it pays.
    let mut h = sign_demo(PANEL);
    h.tap(ids::SIGN_TO);
    assert!(h.app.rect_of(ids::COMPARE_DONE).is_some(), "a Compare");
    unique(&h, "the recipient's address");
    h.tap(ids::COMPARE_DONE);

    // One signature: the kind is "Signature", the instance the input it
    // signs and the key that signed it.
    for _ in 0..4 {
        h.tap(ids::SIGN_CONTINUE);
    }
    h.hold(ids::SIGN_HOLD);
    h.tap(ids::SIGN_SIGNATURES);
    h.tap(ids::at(ids::SIGN_SIG_BASE, 0));
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.sign_signature),
        "the signature Compare is titled by its kind: {texts:?}"
    );
    unique(&h, "a signature");

    // The wallet export: a descriptor as structure, and the account key
    // inside it. Neither is one of a run, so neither carries a label.
    let mut h = loaded(PANEL);
    h.open_single_sig(0);
    h.tap(ids::WALLET_EXPORT);
    h.tap(ids::EXPORT_TEXT);
    unique(&h, "the descriptor");
    h.tap(ids::COMPARE_KEY);
    unique(&h, "the account key");
    h.tap(ids::COMPARE_DONE);

    // The bytes behind an unknown QR.
    let mut h = loaded(PANEL);
    h.tap(ids::HOME_SCAN);
    h.scan_bytes(b"hello, world");
    h.tap(ids::SCAN_HEX);
    unique(&h, "the unknown QR's bytes");
}

/// Fails when anything the last frame placed is outside the region it is
/// drawn in: a row below the fold, a card past the edge.
fn assert_fits(h: &Harness, screen: &str, display: DisplayInfo) {
    let over = h.app.overflow();
    assert!(
        over.is_empty(),
        "{screen} overflows at {}x{}: {over:?}",
        display.width,
        display.height
    );
}

/// Fails when a screen that is a genuine document at this size does not
/// scroll: what does not fit belongs in a scroll region with a
/// scrollbar, not below the fold of a fixed frame (UX.md §5).
fn assert_scrolls(h: &Harness, screen: &str, display: DisplayInfo) {
    let (content, _) = h
        .app
        .scroll_info(ids::SCROLL)
        .unwrap_or_else(|| panic!("{screen} has no scroll region"));
    let view = h.app.rect_of(ids::SCROLL).expect("the scroll region");
    assert!(
        content > view.h,
        "{screen} is not a document at {}x{}",
        display.width,
        display.height
    );
    // Nothing is stranded outside the region that scrolls.
    for r in h.app.overflow() {
        assert!(
            r.x >= view.x && r.right() <= view.right(),
            "{screen} strands {r:?} beside its scroll region at {}x{}",
            display.width,
            display.height
        );
    }
}

/// The offer to keep a key carries "Not now" beside the hold, and both
/// are on the screen with the table above them at the reference panel
/// and the smallest one (§16.65).
#[test]
fn the_keep_offer_fits_beside_its_hold() {
    for display in [PANEL, TINY] {
        let secure = DisplayInfo {
            secure: osk_shell_api::SecureHardware::StrongBox,
            ..display
        };
        let mut h = Harness::kept(secure, Element::default());
        h.start_load(&ABANDON);
        h.tap(ids::LOAD_CONTINUE);
        h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
        h.add_key();
        assert_eq!(h.app.screen(), ScreenKind::Keep);
        let hold = h.app.rect_of(ids::KEEP_HOLD).expect("the hold");
        let not_now = h.app.rect_of(ids::KEEP_NOT_NOW).expect("Not now");
        assert_eq!(hold.y, not_now.y, "the pair shares one row");
        assert!(not_now.right() <= hold.x, "Not now is left of the hold");
        assert!(
            hold.bottom() <= i32::from(secure.height) && not_now.x >= 0,
            "both are on screen at {}x{}",
            secure.width,
            secure.height
        );
        assert_one_screenful(&h, "the offer to keep a key", secure);
    }
}

/// Fails when a screen that is meant to be one screenful scrolls or
/// strands anything: the content is at most the viewport and the frame
/// has nothing outside it (UX.md §5).
fn assert_one_screenful(h: &Harness, screen: &str, display: DisplayInfo) {
    assert_fits(h, screen, display);
    if let Some((content, _)) = h.app.scroll_info(ids::SCROLL) {
        let view = h.app.rect_of(ids::SCROLL).expect("the scroll region");
        assert!(
            content <= view.h,
            "{screen} scrolls at {}x{}: {content} px of content in {} px",
            display.width,
            display.height,
            view.h
        );
    }
}

/// A document either fits or scrolls; either way nothing is below the
/// fold of a frame that cannot be scrolled.
fn assert_document(h: &Harness, screen: &str, display: DisplayInfo) {
    if h.app.overflow().is_empty() {
        return;
    }
    assert_scrolls(h, screen, display);
}

/// A `Small` panel, where a screen with a record, a list and a key chip
/// on it is a document rather than one screenful.
fn is_small(display: DisplayInfo) -> bool {
    display.width == TINY.width || display.width == PANEL.width
}

/// Loads the test key, switches to regtest and signs the demo PSBT up to
/// the overview.
fn sign_demo(display: DisplayInfo) -> Harness {
    let mut h = loaded(display);
    h.set_network(3);
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN);
    h.tap(ids::SCAN_FILE);
    h.send(osk_shell_api::Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    assert_eq!(
        h.app.sign_stage(),
        Some(SignStage::Wizard(SignStep::Summary))
    );
    h
}

/// §5: every step of the Sign review is one screenful for the demo
/// transaction — both outputs, the inputs, the confirm and the result —
/// and nothing on any class is stranded. The summary is the exception on
/// the 268 dp panel: with the change state on it (§16.46) it is a
/// document there, so what does not fit scrolls rather than sitting
/// below the fold.
fn the_sign_review_is_one_screenful_at(display: DisplayInfo) {
    let mut h = sign_demo(display);
    assert!(h.app.rect_of(ids::SIGN_TO).is_some(), "the recipient row");
    if is_small(display) {
        assert_document(&h, "sign review", display);
    } else {
        assert_one_screenful(&h, "sign review", display);
    }
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(
        h.app.sign_stage(),
        Some(SignStage::Wizard(SignStep::Outputs))
    );
    // One output per screen; Continue walks the run.
    assert!(h.app.rect_of(ids::at(ids::SIGN_OUT_BASE, 0)).is_some());
    assert!(h.app.rect_of(ids::at(ids::SIGN_OUT_BASE, 1)).is_none());
    assert_one_screenful(&h, "sign output 1", display);
    h.tap(ids::SIGN_CONTINUE);
    assert!(h.app.rect_of(ids::at(ids::SIGN_OUT_BASE, 1)).is_some());
    assert_one_screenful(&h, "sign output 2", display);
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(
        h.app.sign_stage(),
        Some(SignStage::Wizard(SignStep::Inputs))
    );
    assert_one_screenful(&h, "sign inputs", display);
    h.tap(ids::SIGN_CONTINUE);
    assert_eq!(
        h.app.sign_stage(),
        Some(SignStage::Wizard(SignStep::Confirm))
    );
    assert_one_screenful(&h, "sign confirm", display);
    h.hold(ids::SIGN_HOLD);
    assert_eq!(
        h.app.sign_stage(),
        Some(SignStage::Wizard(SignStep::Result))
    );
    assert_one_screenful(&h, "sign result", display);
}

at_every_size!(
    the_sign_review_is_one_screenful_at =>
        the_sign_review_is_one_screenful_on_tiny,
        the_sign_review_is_one_screenful_on_panel,
        the_sign_review_is_one_screenful_on_phone,
        the_sign_review_is_one_screenful_on_desktop
);

/// §4.5: the recipient's address is a reference row, and the row opens
/// the Compare screen with the whole of it. The same row is on the
/// confirm screen.
#[test]
fn the_recipient_row_opens_the_whole_address() {
    let mut h = sign_demo(PANEL);
    let recipient = h
        .app
        .sign_inspection()
        .expect("inspected")
        .outputs
        .iter()
        .find(|o| !o.is_ours())
        .expect("a recipient")
        .address
        .clone();
    for step in [SignStep::Summary, SignStep::Confirm] {
        while h.app.sign_stage() != Some(SignStage::Wizard(step)) {
            h.tap(ids::SIGN_CONTINUE);
        }
        h.tap(ids::SIGN_TO);
        let texts = h.app.texts();
        assert!(
            texts.iter().any(|t| t.replace(' ', "") == recipient),
            "Compare shows the whole address on {step:?}: {texts:?}"
        );
        assert!(h.app.rect_of(ids::COMPARE_TEXT).is_some());
        h.tap(ids::COMPARE_DONE);
        assert_eq!(h.app.sign_stage(), Some(SignStage::Wizard(step)));
    }
}

/// §2.7: "the word 'irreversible' appears nowhere". The Signatures page
/// is the screen that used to carry the nonce sentence, and it now says
/// of each signature that it verifies and which nonce rule made it
/// (`docs/PLANNING.md` §16.111).
#[test]
fn the_signatures_page_names_the_nonce_rule_and_the_txid() {
    let mut h = sign_demo(PANEL);
    for _ in 0..4 {
        h.tap(ids::SIGN_CONTINUE);
    }
    h.hold(ids::SIGN_HOLD);
    h.tap(ids::SIGN_SIGNATURES);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.contains(strings::EN.sign_sig_valid)
            && t.contains(strings::EN.settings_nonce_low_r)),
        "each signature is checked and named by its nonce rule: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == strings::EN.sign_txid),
        "and the transaction id is a row: {texts:?}"
    );
}

/// The dice pad, the sanity result, the words panel and the quiz rows
/// each fit the screen they are drawn on, and the pad keeps one
/// rectangle from the first roll to the last (§2.5).
#[test]
fn the_words_page_the_quiz_rows_and_the_dice_pad_fit() {
    for display in SIZES {
        let where_ = format!("{}x{}", display.width, display.height);
        let mut h = Harness::new(display);
        h.open_create();
        h.choose(
            ids::at(ids::CREATE_SOURCE_BASE, 0),
            ids::CREATE_SOURCE_CONTINUE,
        );
        h.choose(
            ids::at(ids::CREATE_COUNT_BASE, 0),
            ids::CREATE_COUNT_CONTINUE,
        );
        h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
        assert_eq!(h.app.create_step(), Some(CreateStep::Procedure));
        assert_fits(&h, "dice procedure", display);
        h.tap(ids::CREATE_PROCEDURE_CONTINUE);
        assert_eq!(h.app.create_step(), Some(CreateStep::Entropy));
        assert_fits(&h, "dice pad", display);
        // §2.5: the pad is the same size in the same place whatever the
        // run above it says.
        let empty = h.app.rect_of(ids::PAD_SLOT).expect("the pad rectangle");
        assert!(
            h.app.rect_of(ids::CREATE_ENTRIES).is_some(),
            "the run sits above the pad at {where_}"
        );
        // A run of eight sixes, so the sanity check is in the caution tone.
        h.type_text("66666666351521144121541512665155412152342515356215");
        assert_eq!(
            h.app.rect_of(ids::PAD_SLOT),
            Some(empty),
            "fifty rolls moved the pad at {where_}"
        );
        assert_fits(&h, "dice pad at fifty rolls", display);
        h.key(osk_shell_api::Key::Enter);
        assert_eq!(h.app.create_step(), Some(CreateStep::Sanity));
        // The sanity check is one screenful even with a caution on it:
        // the verdict beside a statistic is one word on a panel, and the
        // row into the math states its label alone.
        assert_one_screenful(&h, "sanity check", display);
        assert!(
            h.app.rect_of(ids::CREATE_MATH).is_some(),
            "the row into the math is on the screen at {where_}"
        );
        h.tap(ids::CREATE_MATH);
        assert_eq!(h.app.create_step(), Some(CreateStep::Math));
        assert_document(&h, "the math", display);
        h.tap(ids::CREATE_MATH_ENTROPY);
        assert_fits(&h, "the entropy", display);
        h.tap(ids::BACK);
        h.tap(ids::CREATE_CONTINUE);
        h.tap(ids::CREATE_CONTINUE);
        assert_eq!(h.app.create_step(), Some(CreateStep::Words));
        assert_fits(&h, "words page 1", display);
        // Six words a page on a panel; a phone and a window hold the
        // whole key, so only a panel has a pager.
        if h.app.class() == osk_ui::geom::SizeClass::Small {
            h.tap(ids::WORDS_NEXT);
            assert_eq!(h.app.create_step(), Some(CreateStep::Words), "two pages");
            assert_fits(&h, "words page 2", display);
        } else {
            assert!(
                h.app.rect_of(ids::WORDS_NEXT).is_none(),
                "no pager for one page at {where_}"
            );
        }
        h.tap(ids::CREATE_CONTINUE);
        assert_eq!(h.app.create_step(), Some(CreateStep::QuizStart));
        assert_fits(&h, "quiz start", display);
        h.tap(ids::QUIZ_START);
        assert_eq!(h.app.create_step(), Some(CreateStep::Quiz));
        // §2.4: one candidate per row, in the order the quiz poses them.
        let rows: Vec<_> = (0..4)
            .map(|i| {
                h.app
                    .rect_of(ids::at(ids::QUIZ_CHOICE_BASE, i))
                    .unwrap_or_else(|| panic!("candidate {i} at {where_}"))
            })
            .collect();
        for pair in rows.windows(2) {
            assert!(
                pair[1].y > pair[0].y && pair[1].x == pair[0].x,
                "the candidates are one list at {where_}: {rows:?}"
            );
        }
        assert_document(&h, "quiz question", display);
    }
}

/// §2.6 and §4.2: on a 268 dp panel the four candidates and the Continue
/// under them are all on the screen, because the progress moved into the
/// title and the row above the list went with it.
#[test]
fn the_quiz_candidates_and_continue_are_all_on_a_panel() {
    for display in [PANEL, TINY] {
        let where_ = format!("{}x{}", display.width, display.height);
        let mut h = loaded(display);
        h.open_key(0);
        h.tap(ids::DETAIL_BACKUP);
        h.tap(ids::BACKUP_VERIFY);
        h.tap(ids::QUIZ_START);
        let bottom = i32::from(display.height);
        for i in 0..4 {
            let r = h
                .app
                .rect_of(ids::at(ids::QUIZ_CHOICE_BASE, i))
                .unwrap_or_else(|| panic!("candidate {i} at {where_}"));
            assert!(
                r.y >= 0 && r.bottom() <= bottom,
                "candidate {i} runs off the screen at {where_}: {r:?}"
            );
        }
        let cont = h.app.rect_of(ids::QUIZ_CONTINUE).expect("Continue");
        assert!(cont.bottom() <= bottom, "Continue is off the screen");
        assert_one_screenful(&h, "a quiz question", display);
    }
}

/// §5 Result: the coloured title is one line on a 268 dp panel. A title
/// that wraps costs the table a row and pushes the block off the screen,
/// which is what "Backup not verified" did.
#[test]
fn every_result_title_is_one_line_on_a_panel() {
    let s = &strings::EN;
    let class = osk_ui::geom::SizeClass::Small;
    let scale = osk_ui::geom::Scale::for_class(PANEL.dpi, class);
    let face = osk_ui::Font::semibold(tokens::TITLE).sized(scale);
    // What §4.11's result row leaves the title: the pane less the screen
    // padding, the icon and the gap beside it.
    let room = i32::from(PANEL.width)
        - 2 * px(PANEL, tokens::PAD)
        - px(PANEL, tokens::ICON_LARGE)
        - px(PANEL, tokens::GAP);
    for title in [
        s.create_sanity_random,
        s.create_sanity_uneven,
        s.create_sanity_unchecked,
        s.quiz_wrong_title,
        s.quiz_passed_title,
        s.quiz_failed_title,
        s.load_checksum_valid,
        s.load_checksum_failed,
    ] {
        let w = osk_ui::text::width(&face, title);
        assert!(
            w <= room,
            "the result title {title:?} is {w} px wide, more than the {room} px a panel gives it"
        );
    }
}

/// The quiz never puts the right word on the screen that says the answer
/// was wrong: not the word, and not any other word of the list either
/// (UX review 2026-09-07, §3.3).
#[test]
fn a_wrong_answer_screen_carries_no_word_of_the_list() {
    let mut h = loaded(PANEL);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_VERIFY);
    h.tap(ids::QUIZ_START);
    let v = h.app.quiz_view().expect("a quiz");
    let right = ABANDON[v.word_number - 1];
    let slot = v.choices.iter().position(|c| c == right).expect("the word");
    h.choose(
        ids::at(ids::QUIZ_CHOICE_BASE, (slot + 1) % 4),
        ids::QUIZ_CONTINUE,
    );
    let texts = h.app.texts();
    // Neither the key's own words nor the candidates that were on the
    // question are anywhere on it.
    let mut secret: Vec<&str> = ABANDON.to_vec();
    secret.extend(v.choices.iter().map(|c| c.as_str()));
    for word in secret {
        for text in &texts {
            assert!(
                !text.contains(word),
                "the wrong-answer screen says {word:?}: {texts:?}"
            );
        }
    }
    assert!(h.app.rect_of(ids::at(ids::QUIZ_CHOICE_BASE, 0)).is_none());
}

#[test]
fn the_scanner_screens_fit() {
    for display in SIZES {
        let mut h = Harness::new(display);
        h.tap(ids::HOME_SCAN);
        assert_eq!(h.app.screen(), ScreenKind::Scan);
        assert_fits(&h, "viewfinder", display);
        h.send(osk_shell_api::Event::CameraUnavailable);
        assert!(h.app.rect_of(ids::SCAN_FILE).is_some(), "the file tile");
        assert_fits(&h, "no camera", display);
        h.tap(ids::BACK);
    }
}

/// §4.9: "the square takes what the band leaves." The ways in are one
/// band of tiles rather than a row each, so the square a person aims by
/// keeps most of the pane's width on the smallest panel it is drawn on.
#[test]
fn the_viewfinder_keeps_most_of_the_width_under_its_band() {
    for display in SIZES {
        let where_ = format!("{}x{}", display.width, display.height);
        // Three ways in is the most any scanner offers: the tool
        // scanners take a file, a paste and a typed string.
        let mut h = loaded(display);
        h.tap(ids::at(ids::HOME_TILE_BASE, 3));
        h.tap(ids::at(ids::TOOLS_CALC_BASE, 0));
        assert_eq!(h.app.screen(), ScreenKind::Scan);
        for id in [ids::SCAN_FILE, ids::SCAN_PASTE, ids::SCAN_TYPE] {
            assert!(
                h.app.rect_of(id).is_some(),
                "the band holds every way in at {where_}"
            );
        }
        let square = h.app.rect_of(ids::VIEWFINDER).expect("the viewfinder");
        // The band spans the column, so its width is the width the
        // square is measured against.
        let column = h.app.rect_of(ids::GROUP).expect("the band").w;
        assert!(
            square.w * 4 >= column * 3,
            "the square is {} of {column} px wide at {where_}",
            square.w
        );
    }
}

/// §4.2 Setting: "The same rows with the check on the current value; no
/// Continue. Tap applies at once and the screen stays."
#[test]
fn a_setting_applies_on_the_tap_and_keeps_its_screen() {
    let mut h = loaded(PANEL);
    h.open_settings();
    h.tap(ids::SETTINGS_UNIT_ROW);
    assert_eq!(h.app.screen(), ScreenKind::Setting);
    // There is no Continue on the screen to press.
    assert!(
        h.app.rect_of(ids::PICK_CONTINUE).is_none(),
        "a Setting has no Continue"
    );
    assert_eq!(h.app.unit(), osk_ui::components::Unit::Sat);
    h.tap(ids::at(ids::SETTINGS_UNIT_BASE, 1));
    assert_eq!(
        h.app.unit(),
        osk_ui::components::Unit::Btc,
        "applied on tap"
    );
    assert_eq!(h.app.screen(), ScreenKind::Setting, "and the screen stays");
    // The chevron closes it and Settings shows the new value.
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    let texts = h.app.texts();
    let btc = strings::EN.settings_unit_btc;
    assert!(texts.iter().any(|t| t == btc), "{texts:?}");
}

/// §4.7 and §2.9: every amount on a Sign screen is in the unit the
/// setting names, and there is no echo in the other.
#[test]
fn the_unit_setting_changes_every_amount_on_the_sign_review() {
    let amounts = |unit: usize| -> Vec<String> {
        let mut h = loaded(PANEL);
        h.set_setting(ids::SETTINGS_UNIT_ROW, ids::SETTINGS_UNIT_BASE, unit);
        h.tap(ids::BACK);
        h.set_network(3);
        h.open_single_sig(0);
        h.tap(ids::WALLET_SIGN);
        h.tap(ids::SCAN_FILE);
        h.send(osk_shell_api::Event::File {
            kind: osk_shell_api::FileKind::Any,
            bytes: DEMO.to_vec(),
        });
        assert_eq!(
            h.app.sign_stage(),
            Some(SignStage::Wizard(SignStep::Summary))
        );
        h.app.texts()
    };
    let sats = amounts(0);
    assert!(
        sats.iter().any(|t| t.ends_with(" sats")),
        "the review in sats: {sats:?}"
    );
    assert!(
        !sats.iter().any(|t| t.contains("BTC")),
        "no echo in the other unit: {sats:?}"
    );
    let btc = amounts(1);
    assert!(
        btc.iter().any(|t| t.ends_with(" BTC")),
        "the review in BTC: {btc:?}"
    );
    assert!(
        !btc.iter().any(|t| t.contains("sats")),
        "no echo in the other unit: {btc:?}"
    );
}

/// §4.14 Terminal state: "Result layout with no action and no back
/// chevron." The session's own end is the one screen a person reaches
/// that has neither.
#[test]
fn the_session_ends_on_a_result_with_no_way_back() {
    let mut h = loaded(PANEL);
    h.open_settings();
    h.tap(ids::SETTINGS_EXIT_ROW);
    h.hold(ids::SETTINGS_EXIT);
    assert_eq!(h.app.screen(), ScreenKind::Ended);
    assert!(h.app.rect_of(ids::SCREEN).is_some(), "built from §5");
    assert!(h.app.rect_of(ids::BACK).is_none(), "no back chevron");
    assert!(h.app.hold_buttons().is_empty(), "nothing to press");
    let texts = h.app.texts();
    for want in [
        strings::EN.settings_ended_title,
        strings::EN.settings_ended_keys,
        strings::EN.row_keys,
    ] {
        assert!(texts.iter().any(|t| t == want), "{want:?}: {texts:?}");
    }
}

fn the_explore_screens_and_the_verify_result_fit_at(display: DisplayInfo) {
    let mut h = loaded(display);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    assert_eq!(h.app.screen(), ScreenKind::Explore);
    // Every Explore screen is one of the sixteen: the two menus are
    // lists that scroll, the Document scrolls, and everything the
    // rows open fits the panel it is drawn on.
    assert_document(&h, "explore keys", display);
    h.tap(ids::EXPLORE_USING);
    assert_fits(&h, "explore key chooser", display);
    h.tap(ids::BACK);
    h.tap(ids::EXPLORE_PATH);
    h.type_text("/");
    assert!(h.app.explore_path_error().is_some());
    // §2.6: five preset rows, a field and a keyboard do not share a
    // 268 dp panel, so the presets are a block that scrolls.
    assert_document(&h, "explore path editor", display);
    h.key(osk_shell_api::Key::Backspace);
    h.key(osk_shell_api::Key::Enter);
    h.tap(ids::EXPLORE_XPRV);
    assert_fits(&h, "explore extended private key", display);
    h.tap(ids::BACK);
    h.tap(ids::EXPLORE_BITS);
    assert_document(&h, "explore words and bits", display);
    h.tap(ids::EXPLORE_WORDS);
    assert_fits(&h, "explore words", display);
    if h.app.rect_of(ids::WORDS_NEXT).is_some() {
        h.tap(ids::WORDS_NEXT);
        assert_fits(&h, "explore words page 2", display);
    }
    h.tap(ids::EXPLORE_WORDS_DONE);
    for row in [
        ids::EXPLORE_ENTROPY,
        ids::EXPLORE_CHECKSUM_BITS,
        ids::EXPLORE_SEED,
        ids::EXPLORE_MASTER_XPRV,
    ] {
        h.tap(row);
        assert_fits(&h, "an explore secret", display);
        h.tap(ids::BACK);
    }
    h.tap(ids::BACK);

    // Verify: the entry menu, the field with the keyboard, then the
    // result.
    h.go_home();
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    // §5 Scanner: the square and its three rows fit the smallest
    // panel.
    assert_fits(&h, "the address scanner", display);
    let ours = h
        .app
        .addresses(0, osk_bip::keys::ScriptType::NativeSegwit, false, 1)[0]
        .clone();
    h.tap(ids::SCAN_TYPE);
    assert_fits(&h, "verify typing", display);
    h.type_text(&ours);
    h.key(osk_shell_api::Key::Enter);
    assert_fits(&h, "verify result", display);
}

at_every_size!(
    the_explore_screens_and_the_verify_result_fit_at =>
        the_explore_screens_and_the_verify_result_fit_on_tiny,
        the_explore_screens_and_the_verify_result_fit_on_panel,
        the_explore_screens_and_the_verify_result_fit_on_phone,
        the_explore_screens_and_the_verify_result_fit_on_desktop
);

// ----- Pass C: the rules the redesign added (UX.md §4-§6) -----

/// Every reference size (`docs/PLANNING.md` §4.3, §13.1).
const SIZES: [DisplayInfo; 4] = [TINY, PANEL, PHONE, DESKTOP];

/// The status line: the tier badge always, the network badge only off
/// mainnet, and the lock only once there is something to lock
/// (UX.md §4 "Status"). Settings is a tile, not a glyph here.
#[test]
fn the_status_line_shows_what_the_state_calls_for() {
    let mut h = Harness::new(PANEL);
    assert!(h.app.rect_of(ids::STATUS_TIER).is_some());
    assert!(
        h.app.rect_of(ids::STATUS_LOCK).is_none(),
        "nothing to lock without keys"
    );
    // The tier badge opens the explanation and names this device's tier.
    h.tap(ids::STATUS_TIER);
    assert_eq!(h.app.screen(), ScreenKind::Tiers);
    h.tap(ids::BACK);
    h.start_load(&ABANDON);
    h.finish_load(None);
    // With a key loaded there is something to lock, so the lock is on
    // the line; the countdown joins it inside the last minute.
    assert!(h.app.rect_of(ids::STATUS_LOCK).is_some());
    assert!(h.app.lock_in_ms().is_some());
    h.tap(ids::STATUS_LOCK);
    assert!(h.app.is_locked());
}

/// A screen with a back chevron never also offers Cancel: the chevron
/// is the cancel (UX.md §5).
#[test]
fn no_screen_offers_both_a_back_chevron_and_a_cancel() {
    let mut h = loaded(PANEL);
    let mut seen = 0;
    let check = |h: &Harness, name: &str| {
        if h.app.rect_of(ids::BACK).is_some() {
            assert!(
                !h.app
                    .labels()
                    .iter()
                    .any(|l| l.eq_ignore_ascii_case("cancel")),
                "{name} has a back chevron and a Cancel"
            );
        }
    };
    // The scanner, which is where a Cancel used to live.
    h.tap(ids::HOME_SCAN);
    check(&h, "scanner");
    seen += 1;
    h.send(osk_shell_api::Event::CameraUnavailable);
    check(&h, "scanner without a camera");
    seen += 1;
    h.tap(ids::BACK);
    // Every wizard step of the Load flow.
    h.open_load();
    for (step, row, cont) in [
        ("source", ids::LOAD_SOURCE_TYPE, ids::LOAD_SOURCE_CONTINUE),
        (
            "count",
            ids::at(ids::LOAD_COUNT_BASE, 0),
            ids::LOAD_COUNT_CONTINUE,
        ),
        (
            "language",
            ids::at(ids::LOAD_LANG_BASE, 0),
            ids::LOAD_LANG_CONTINUE,
        ),
    ] {
        check(&h, step);
        seen += 1;
        h.choose(row, cont);
    }
    for w in ABANDON {
        h.type_text(w);
        h.key(osk_shell_api::Key::Enter);
    }
    check(&h, "checksum");
    seen += 1;
    h.tap(ids::LOAD_CONTINUE);
    check(&h, "passphrase offer");
    seen += 1;
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    check(&h, "confirm");
    seen += 1;
    assert_eq!(seen, 8);
}

/// A secret panel is one touch surface: it reveals while a finger is on
/// it and re-masks on release, and there is no reveal button beside it
/// (UX.md §5).
#[test]
fn secret_panels_reveal_on_press_and_mask_on_release() {
    for display in [PANEL, TINY] {
        let mut h = loaded(display);
        h.open_key(0);
        h.tap(ids::DETAIL_BACKUP);
        h.tap(ids::BACKUP_WORDS);
        let panel = h.app.rect_of(ids::CREATE_REVEAL).expect("the panel");
        assert!(
            panel.w >= px(display, 160.0),
            "the panel is the touch target, not a button: {panel:?}"
        );
        assert!(h.app.hold_buttons().is_empty(), "no hold button beside it");
        let c = panel.center();
        let (x, y) = (c.x as u16, c.y as u16);
        h.send(osk_shell_api::Event::Touch {
            x,
            y,
            phase: osk_shell_api::TouchPhase::Down,
        });
        assert!(h.app.is_held(ids::CREATE_REVEAL), "revealed while pressed");
        h.send(osk_shell_api::Event::Touch {
            x,
            y,
            phase: osk_shell_api::TouchPhase::Up,
        });
        assert!(!h.app.is_held(ids::CREATE_REVEAL), "masked on release");
        assert!(h.app.overflow().is_empty(), "the words page fits");
    }
}

/// Six words to a page, on every size class, so that a page always fits
/// its panel (UX.md D1).
#[test]
fn a_word_page_never_pages_inside_its_panel() {
    for display in SIZES {
        let mut h = loaded(display);
        h.open_key(0);
        h.tap(ids::DETAIL_BACKUP);
        h.tap(ids::BACKUP_WORDS);
        assert!(
            h.app.overflow().is_empty(),
            "the page fits at {}x{}",
            display.width,
            display.height
        );
    }
}

/// A hold is only ever the last step of something that cannot be undone
/// (`docs/PLANNING.md` §16.27).
#[test]
fn only_final_irreversible_actions_use_a_hold() {
    let allowed = [
        ids::SIGN_HOLD,
        ids::DETAIL_FORGET,
        ids::SETTINGS_WIPE,
        ids::SETTINGS_EXIT,
    ];
    let mut h = loaded(PANEL);
    let check = |h: &Harness, name: &str| {
        for id in h.app.hold_buttons() {
            assert!(
                allowed.contains(&id),
                "{name} holds for {}, which is not a final action",
                id.0
            );
        }
    };
    check(&h, "home");
    // Every screen the review scripts walk, in one pass.
    for tile in [0usize, 1, 2] {
        h.tap(ids::at(ids::HOME_TILE_BASE, tile));
        check(&h, "an area");
        h.tap(ids::BACK);
    }
    h.open_single_sig(0);
    check(&h, "a wallet menu");
    for row in [ids::WALLET_ADDRESSES, ids::WALLET_EXPORT, ids::WALLET_KEYS] {
        h.tap(row);
        check(&h, "a wallet screen");
        h.tap(ids::BACK);
    }
    h.go_home();
    h.open_passphrase(0);
    check(&h, "the passphrase a key is opened behind");
    h.tap(ids::BACK);
    h.open_child(0);
    check(&h, "the child's word count");
    h.go_home();
    h.open_key(0);
    check(&h, "key detail");
    h.tap(ids::DETAIL_BACKUP);
    check(&h, "a key screen");
    h.tap(ids::BACK);
    // The Backup flow: the words, the seed codes and the quiz all show
    // a secret, and none of them holds for it.
    h.tap(ids::DETAIL_BACKUP);
    for row in [ids::BACKUP_WORDS, ids::BACKUP_SEEDQR, ids::BACKUP_COMPACT] {
        h.tap(row);
        check(&h, "a backup screen");
        h.tap(ids::BACK);
    }
    h.tap(ids::BACKUP_VERIFY);
    check(&h, "the quiz start");
    h.tap(ids::QUIZ_START);
    check(&h, "a quiz question");
    h.tap(ids::BACK);
    h.tap(ids::QUIZ_DONE);
    h.tap(ids::BACK);
    // Forget, wipe and wipe-and-exit are the holds that are allowed, and
    // each is reached by a tap first.
    h.tap(ids::BACK);
    h.open_key(0);
    h.tap(ids::KEY_FORGET);
    assert_eq!(h.app.hold_buttons(), vec![ids::DETAIL_FORGET]);
    h.tap(ids::BACK);
    h.open_settings();
    check(&h, "settings");
    h.tap(ids::SETTINGS_WIPE_ROW);
    assert_eq!(h.app.hold_buttons(), vec![ids::SETTINGS_WIPE]);
    h.tap(ids::BACK);
    h.tap(ids::SETTINGS_EXIT_ROW);
    assert_eq!(h.app.hold_buttons(), vec![ids::SETTINGS_EXIT]);
}

/// A choice is checked by the tap and confirmed by Continue
/// (`docs/DESIGN.md` §2.7): the tap that checks moves nothing, and
/// Continue is what moves the step.
#[test]
fn a_choice_checks_on_tap_and_advances_on_continue() {
    use opensigner_core::load::Step as LoadStep;
    let mut h = Harness::new(PANEL);
    h.open_load();
    for (step, row, cont, next) in [
        (
            LoadStep::Source,
            ids::LOAD_SOURCE_TYPE,
            ids::LOAD_SOURCE_CONTINUE,
            LoadStep::Count,
        ),
        (
            LoadStep::Count,
            ids::at(ids::LOAD_COUNT_BASE, 0),
            ids::LOAD_COUNT_CONTINUE,
            LoadStep::Language,
        ),
        (
            LoadStep::Language,
            ids::at(ids::LOAD_LANG_BASE, 0),
            ids::LOAD_LANG_CONTINUE,
            LoadStep::Words,
        ),
    ] {
        assert_eq!(h.app.load_step(), Some(step));
        assert!(
            h.app.rect_of(cont).is_some(),
            "a choice step has a Continue"
        );
        h.tap(row);
        assert_eq!(h.app.load_step(), Some(step), "the tap only checks");
        h.tap(cont);
        assert_eq!(h.app.load_step(), Some(next));
    }
    // The passphrase offer and "Which key?" are choices too.
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    assert_eq!(h.app.load_step(), Some(LoadStep::PassphraseOffer));
    h.tap(ids::LOAD_ADD_PASSPHRASE);
    assert_eq!(h.app.load_step(), Some(LoadStep::PassphraseOffer));
    h.tap(ids::LOAD_PASS_CONTINUE);
    assert_eq!(h.app.load_step(), Some(LoadStep::Passphrase));
    h.type_text("TREZOR");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.load_step(), Some(LoadStep::PassphraseConfirm));
    h.tap(ids::LOAD_WHICH_PLAIN);
    assert_eq!(h.app.load_step(), Some(LoadStep::PassphraseConfirm));
    h.tap(ids::LOAD_WHICH_CONTINUE);
    assert_eq!(h.app.load_step(), Some(LoadStep::Confirm));
    // The row the check was on is the key that is added.
    h.add_key();
    assert_eq!(h.app.fingerprints()[0].to_hex(), *b"73c5da0a");
}

/// §4.3: on a 268 dp panel word entry shows the word being typed and
/// nothing else; on a phone the masked panel of the words accepted so
/// far sits above the entry group. Backspace on an empty field steps
/// back to the word before it.
#[test]
fn word_entry_shows_only_the_current_word() {
    use opensigner_core::load::Step as LoadStep;
    let mut h = Harness::new(PANEL);
    h.open_load();
    h.load_choices(0);
    h.type_text("abandon");
    h.key(osk_shell_api::Key::Enter);
    h.type_text("zoo");
    assert_eq!(h.app.load_step(), Some(LoadStep::Words));
    // The field, the candidates and the keyboard: nothing else, and no
    // words panel on a panel.
    assert!(h.app.rect_of(ids::LOAD_KEYBOARD).is_some());
    assert!(h.app.rect_of(ids::FIELD).is_some());
    assert!(
        h.app.rect_of(ids::LOAD_WORDS_PANEL).is_none(),
        "no words panel on a 268 dp panel"
    );
    assert!(h.app.overflow().is_empty());
    // §16.118: "zoo" is typed out in full and waits in the strip until
    // it is taken; Enter takes the one candidate left.
    assert_eq!(h.app.candidates(), vec![String::from("zoo")]);
    h.key(osk_shell_api::Key::Enter);
    assert!(h.app.candidates().is_empty(), "empty field");
    // The backspace on the empty field steps back to word two, which
    // returns to the field for editing: its candidates are on screen.
    h.key(osk_shell_api::Key::Backspace);
    assert!(
        !h.app.candidates().is_empty(),
        "the previous word is back in the field"
    );
    h.key(osk_shell_api::Key::Enter);
    assert!(h.app.candidates().is_empty(), "committed");
    assert!(h.app.overflow().is_empty());

    // On a phone the panel is there and it is masked: the words
    // accepted so far are bullets until a finger is on it.
    let mut h = Harness::new(PHONE);
    h.open_load();
    h.load_choices(0);
    h.type_text("abandon");
    h.key(osk_shell_api::Key::Enter);
    let panel = h
        .app
        .rect_of(ids::LOAD_WORDS_PANEL)
        .expect("the words so far");
    let field = h.app.rect_of(ids::FIELD).expect("the field");
    assert!(
        panel.bottom() <= field.y,
        "the panel is above the entry group"
    );
    let texts = h.app.texts();
    assert!(
        !texts.iter().any(|t| t.contains("abandon")),
        "the words so far are masked: {texts:?}"
    );
    assert!(h.app.overflow().is_empty());
}

/// A keyboard's hit rectangles reach the screen edges, even though the
/// keys people see stand 16 dp clear of the bottom (UX.md §6).
#[test]
fn a_keyboard_reaches_the_screen_edges() {
    for display in [PANEL, TINY, PHONE] {
        let mut h = Harness::new(display);
        h.open_load();
        h.load_choices(0);
        let kb = h.app.rect_of(ids::LOAD_KEYBOARD).expect("the keyboard");
        assert_eq!(kb.x, 0, "left edge");
        assert_eq!(kb.right(), i32::from(display.width), "right edge");
        assert_eq!(kb.bottom(), i32::from(display.height), "bottom edge");
        // The bottom-left corner of the screen types the key above it.
        let corner = h
            .app
            .key_rect(ids::LOAD_KEYBOARD, KeyInput::Backspace)
            .expect("backspace");
        assert_eq!(corner.bottom(), i32::from(display.height));
    }
}

/// Every chip of a segmented row is on the screen and wide enough for
/// its own label, at the smallest supported panel, where a row of four
/// used to run past the right edge (UX review 2026-09-07, finding 5).
#[test]
fn every_setting_option_is_a_row_that_fits_the_smallest_panel() {
    let display = TINY;
    let check = |h: &mut Harness, row: osk_ui::Id, base: u32, count: usize, what: &str| {
        h.open_settings();
        h.tap(row);
        assert_eq!(h.app.screen(), ScreenKind::Setting);
        for i in 0..count {
            let id = ids::at(base, i);
            // A list taller than the panel scrolls from the top (§4.2):
            // bring the row into view, then check it fits the width.
            let r = h
                .app
                .reveal(id)
                .unwrap_or_else(|| panic!("{what} option {i} is not on the screen"));
            assert!(
                r.x >= 0 && r.right() <= i32::from(display.width),
                "{what} option {i} runs off the screen: {r:?}"
            );
        }
        h.tap(ids::BACK);
        h.tap(ids::BACK);
    };

    let mut h = loaded(display);
    check(
        &mut h,
        ids::SETTINGS_NETWORK_ROW,
        ids::SETTINGS_NET_BASE,
        4,
        "network",
    );
    check(
        &mut h,
        ids::SETTINGS_UNIT_ROW,
        ids::SETTINGS_UNIT_BASE,
        2,
        "unit",
    );
    check(
        &mut h,
        ids::SETTINGS_LOCK_AFTER_ROW,
        ids::SETTINGS_LOCK_AFTER_BASE,
        4,
        "lock timer",
    );
    check(
        &mut h,
        ids::SETTINGS_WIPE_AFTER_ROW,
        ids::SETTINGS_WIPE_AFTER_BASE,
        5,
        "wipe timer",
    );
}

// ----- Pass 3: the use of space on every size class -----

/// The `wide` sidebar is on every screen and never moves: the content
/// origin is the same on two consecutive screens of one flow, which is
/// what made `scan-14` to `scan-15` jump 80 px (UX review 2026-09-07,
/// §2b.1).
#[test]
fn the_sidebar_is_in_the_same_place_on_every_wide_screen() {
    let mut h = loaded(DESKTOP);
    let first = h.app.rect_of(ids::SIDEBAR).expect("the sidebar on Home");
    let check = |h: &Harness, screen: &str| {
        let r = h
            .app
            .rect_of(ids::SIDEBAR)
            .unwrap_or_else(|| panic!("no sidebar on {screen}"));
        assert_eq!(r, first, "the sidebar moved on {screen}");
    };
    h.open_single_sig(0);
    check(&h, "a wallet menu");
    h.tap(ids::WALLET_ADDRESSES);
    check(&h, "addresses");
    h.tap(ids::BACK);
    h.go_home();
    h.open_key(0);
    check(&h, "key detail");
    h.tap(ids::KEY_FORGET);
    check(&h, "forget");
    h.tap(ids::BACK);
    h.go_home();
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN);
    check(&h, "the scanner");
    h.tap(ids::BACK);
    h.open_settings();
    check(&h, "settings");
    h.tap(ids::SETTINGS_WIPE_ROW);
    check(&h, "wipe all keys");
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.open_load();
    check(&h, "the load wizard");
    // Dimmed on a wizard: it is drawn, and it has no hit target.
    assert!(
        h.app.rect_of(ids::at(ids::HOME_TILE_BASE, 4)).is_none(),
        "a dimmed sidebar offers no navigation"
    );
}

/// A record's values line up in one column, so the eye runs down them
/// (UX review 2026-09-07, §2b.2 shape 9).
fn the_inputs_record_fits_at(display: DisplayInfo) {
    let where_ = format!("{}x{}", display.width, display.height);
    let mut h = sign_demo(display);
    for _ in 0..3 {
        h.tap(ids::SIGN_CONTINUE);
    }
    assert!(
        h.app.overflow().is_empty(),
        "the inputs record fits at {where_}"
    );
}

at_every_size!(
    the_inputs_record_fits_at =>
        the_inputs_record_fits_on_tiny,
        the_inputs_record_fits_on_panel,
        the_inputs_record_fits_on_phone,
        the_inputs_record_fits_on_desktop
);

// ----- Pass 4: what each screen is called, and which key it acts on -----

/// Every navigable row carries a chevron, and a check mark appears only
/// on the current item of a choice list (UX review 2026-09-07, §2.2).
/// §4.2 and §4.4: the Explore chooser is a Choice of key rows, the
/// current one checked, confirmed with Continue.
#[test]
fn the_key_chooser_checks_the_current_key_and_offers_no_chevron() {
    let mut h = loaded(PANEL);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    h.tap(ids::EXPLORE_USING);
    let row = h
        .app
        .rect_of(ids::at(ids::EXPLORE_KEY_BASE, 0))
        .expect("the loaded key");
    assert!(row.w > 0);
    assert!(
        h.app.texts().iter().any(|t| t == "73c5da0a"),
        "the chooser names the key"
    );
    assert!(
        h.app.rect_of(ids::EXPLORE_CHOOSE_CONTINUE).is_some(),
        "a Choice confirms with Continue"
    );
    // Confirming the key it is already on leaves Explore where it was.
    h.choose(
        ids::at(ids::EXPLORE_KEY_BASE, 0),
        ids::EXPLORE_CHOOSE_CONTINUE,
    );
    assert_eq!(
        h.app.explore_step(),
        Some(opensigner_core::explore::Step::Keys)
    );
}

/// §4.4: "Key context ... only on Sign entry and review, Verify and
/// Explore, where the screen acts on a key the user can switch." The
/// Explore key menu carries the row; the screens its rows open do not,
/// because they are one thing each.
fn the_keys_menu_carries_the_key_context_row_at(display: DisplayInfo) {
    let where_ = format!("{}x{}", display.width, display.height);
    let mut h = loaded(display);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    assert!(
        h.app.rect_of(ids::EXPLORE_USING).is_some(),
        "no key context row on the Explore key menu at {where_}"
    );
    // The row opens the chooser, which names the key too.
    h.tap(ids::EXPLORE_USING);
    assert!(h.app.rect_of(ids::at(ids::EXPLORE_KEY_BASE, 0)).is_some());
    h.tap(ids::EXPLORE_CHOOSE_CONTINUE);
    for row in [ids::EXPLORE_BITS, ids::EXPLORE_PATH] {
        h.tap(row);
        assert!(
            h.app.rect_of(ids::EXPLORE_USING).is_none(),
            "the screen behind row {} carries the key row at {where_}",
            row.0
        );
        h.tap(ids::BACK);
    }
}

at_every_size!(
    the_keys_menu_carries_the_key_context_row_at =>
        the_keys_menu_carries_the_key_context_row_on_tiny,
        the_keys_menu_carries_the_key_context_row_on_panel,
        the_keys_menu_carries_the_key_context_row_on_phone,
        the_keys_menu_carries_the_key_context_row_on_desktop
);

/// §4.4: the review names the keys that sign wherever the class has the
/// height for the row. A 268 dp panel spends its height on the table, so
/// there Confirm's "Sign with" chips are the key's one name.
fn sign_names_its_keys_on_the_screens_that_have_room_at(display: DisplayInfo) {
    let where_ = format!("{}x{}", display.width, display.height);
    let mid = !is_small(display);
    let mut h = sign_demo(display);
    assert_eq!(
        h.app.texts().iter().any(|t| t == "73c5da0a"),
        mid,
        "the key row on the review at {where_}"
    );
    for _ in 0..4 {
        h.tap(ids::SIGN_CONTINUE);
    }
    assert_eq!(
        h.app.sign_stage(),
        Some(SignStage::Wizard(SignStep::Confirm))
    );
    h.hold(ids::SIGN_HOLD);
    assert_eq!(
        h.app.sign_stage(),
        Some(SignStage::Wizard(SignStep::Result))
    );
    assert!(
        h.app.texts().iter().any(|t| t == "73c5da0a"),
        "the result names the key at {where_}"
    );
}

at_every_size!(
    sign_names_its_keys_on_the_screens_that_have_room_at =>
        sign_names_its_keys_on_the_screens_that_have_room_on_tiny,
        sign_names_its_keys_on_the_screens_that_have_room_on_panel,
        sign_names_its_keys_on_the_screens_that_have_room_on_phone,
        sign_names_its_keys_on_the_screens_that_have_room_on_desktop
);

/// A repeat that did not match restarts the entry and says so under the
/// field, in the danger tone, rather than in the app-bar title (UX
/// review 2026-09-07, §2.10).
#[test]
fn a_wrong_pin_repeat_shows_an_inline_error() {
    use opensigner_core::load::Step as LoadStep;
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.tap(ids::LOAD_HOLD);
    assert_eq!(h.app.load_step(), Some(LoadStep::Pin));
    let title = strings::EN.pin_set_title;
    assert!(h.app.texts().iter().any(|t| t == title));
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2580");
    assert_eq!(h.app.load_step(), Some(LoadStep::PinConfirm));
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2581");
    assert_eq!(h.app.load_step(), Some(LoadStep::Pin), "the entry restarts");
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.pin_mismatch),
        "no inline error: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == title),
        "the title still names the step"
    );
}

/// §4.11: "a dead row is a two-line row, like a row with a value" — the
/// reason under the label, in two or three words, on every class (UX
/// review 2026-09-09, §2.2, which found the reason beside the label on
/// `mobile` and `wide`). A row dimmed only because the feature is not
/// built carries nothing: "not built yet" says what the dimming already
/// said.
fn every_dimmed_row_carries_its_reason_at(display: DisplayInfo) {
    fn check(h: &Harness, screen: &str, display: DisplayInfo) {
        let where_ = format!("{}x{}", display.width, display.height);
        let rows = h.app.dimmed_rows();
        assert!(
            !rows.is_empty(),
            "{screen} has no dimmed row to check at {where_}"
        );
        for row in rows {
            let Some(reason) = row.reason else {
                continue;
            };
            let words = reason.split_whitespace().count();
            assert!(
                (1..=5).contains(&words),
                "{screen} at {where_}: {} says {reason:?}, which is {words} words",
                row.label
            );
        }
    }
    let mut h = Harness::new(display);
    h.open_load();
    check(&h, "load sources", display);
    h.choose(ids::LOAD_SOURCE_TYPE, ids::LOAD_SOURCE_CONTINUE);
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    // The unbuilt wordlists are dimmed and say nothing; the list has
    // no other dimmed row, so there is nothing to check here beyond
    // that they carry no text.
    for row in h.app.dimmed_rows() {
        assert_eq!(row.reason, None, "an unbuilt wordlist says {row:?}");
    }

    // Create's sources are all live on a shell with a camera; the
    // camera row is the dimmed one when the shell says it has none.
    let mut h = Harness::new(display);
    h.send(osk_shell_api::Event::CameraUnavailable);
    h.open_create();
    check(&h, "create sources without a camera", display);

    // Load's source list dims the two rows that need a camera when
    // the shell says it has none.
    let mut h = loaded(display);
    h.send(osk_shell_api::Event::CameraUnavailable);
    h.open_load();
    check(&h, "the load sources without a camera", display);

    // Explore at a Taproot account: SLIP-132 has no form for it,
    // and the row says which script types it has.
    let mut h = loaded(display);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    h.tap(ids::EXPLORE_PATH);
    for _ in 0..20 {
        h.pad(ids::EXPLORE_PATH_KEYBOARD, KeyInput::Backspace);
    }
    for c in "86h/0h/0h".chars() {
        h.pad(ids::EXPLORE_PATH_KEYBOARD, KeyInput::Char(c));
    }
    h.pad(ids::EXPLORE_PATH_KEYBOARD, KeyInput::Done);
    check(&h, "explore at a Taproot account", display);

    // A path with no SLIP-132 form dims that row and says which
    // script types have one, in §4.6's one vocabulary.
    h.tap(ids::EXPLORE_PATH);
    h.tap(ids::at(ids::EXPLORE_PRESET_BASE, 3));
    h.key(osk_shell_api::Key::Enter);
    check(&h, "explore at a taproot path", display);
}

at_every_size!(
    every_dimmed_row_carries_its_reason_at =>
        every_dimmed_row_carries_its_reason_on_tiny,
        every_dimmed_row_carries_its_reason_on_panel,
        every_dimmed_row_carries_its_reason_on_phone,
        every_dimmed_row_carries_its_reason_on_desktop
);

/// A chunked address is shown whole, in groups of four, with each line
/// carrying every group its width holds: the string is never paged, no
/// group is broken across two lines, and no line stops a group short of
/// the width.
fn a_chunked_address_fills_its_width_and_is_shown_whole_at(display: DisplayInfo) {
    let where_ = format!("{}x{}", display.width, display.height);
    // Sign, the output's address on the Compare screen its reference
    // row opens (§4.5).
    let mut h = sign_demo(display);
    h.tap(ids::SIGN_CONTINUE);
    h.tap(ids::at(ids::SIGN_OUT_BASE, 0));
    assert_chunked_whole(&h, ids::COMPARE_TEXT, "the sign output", &where_);
    // Key detail, one address on its own screen.
    let mut h = loaded(display);
    h.open_single_sig(0);
    h.tap(ids::WALLET_ADDRESSES);
    h.tap(ids::at(ids::ADDR_ROW_BASE, 0));
    assert_chunked_whole(&h, ids::ADDR_TEXT, "Addresses", &where_);
    // Verify, the address that was checked.
    let address = h
        .app
        .addresses(0, osk_bip::keys::ScriptType::NativeSegwit, false, 1)[0]
        .clone();
    let mut h = loaded(display);
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.tap(ids::SCAN_TYPE);
    h.type_text(&address);
    h.key(osk_shell_api::Key::Enter);
    h.tap(ids::VERIFY_ADDRESS);
    assert_chunked_whole(&h, ids::COMPARE_TEXT, "Verify", &where_);
}

at_every_size!(
    a_chunked_address_fills_its_width_and_is_shown_whole_at =>
        a_chunked_address_fills_its_width_and_is_shown_whole_on_tiny,
        a_chunked_address_fills_its_width_and_is_shown_whole_on_panel,
        a_chunked_address_fills_its_width_and_is_shown_whole_on_phone,
        a_chunked_address_fills_its_width_and_is_shown_whole_on_desktop
);

/// The chunking rule, on one string: the whole of it is on the screen,
/// its lines break between groups and never inside one, and a line
/// carries every group the width holds.
fn assert_chunked_whole(h: &Harness, id: osk_ui::Id, screen: &str, where_: &str) {
    let fit = h
        .app
        .chunk_fit(id)
        .unwrap_or_else(|| panic!("{screen} has no chunked string at {where_}"));
    assert_eq!(fit.pages, 1, "{screen} pages its address at {where_}");
    assert!(
        fit.per_line * fit.lines >= fit.chunks,
        "{screen} leaves {} groups off the screen at {where_}",
        fit.chunks - fit.per_line * fit.lines
    );
    let across = fit.per_line as i32 * fit.chunk_w + (fit.per_line as i32 - 1) * fit.gap;
    assert!(
        across <= fit.width,
        "{screen} splits a group at {where_}: {across} px of groups in {} px",
        fit.width
    );
    assert!(
        fit.per_line >= fit.chunks || across + fit.gap + fit.chunk_w > fit.width,
        "{screen} stops a group short of the width at {where_}: {across} px in {} px",
        fit.width
    );
}

/// §4.9: the QR page opens in the shape a camera can read — one static
/// code only where its modules are wide enough on this display, animated
/// parts otherwise, with the Animated toggle dimmed and the reason
/// beside it when the app chose for itself.
fn the_transaction_qr_opens_in_the_shape_that_scans_at(display: DisplayInfo) {
    let where_ = format!("{}x{}", display.width, display.height);
    let mut h = sign_demo(display);
    for _ in 0..4 {
        h.tap(ids::SIGN_CONTINUE);
    }
    h.hold(ids::SIGN_HOLD);
    h.tap(ids::SIGN_QR);
    let scans = h.app.sign_static_qr_scans();
    let expected = if scans { QrMode::Static } else { QrMode::Ur };
    assert_eq!(h.app.sign_qr_mode(), Some(expected), "the mode at {where_}");
    let s = h.app.strings();
    // A forced toggle is not a control: it is dimmed, has no hit
    // target, and says why in two words (§4.9, §4.11).
    assert_eq!(
        h.app.rect_of(ids::SIGN_QR_ANIMATED).is_some(),
        scans,
        "the toggle at {where_}"
    );
    if !scans {
        let rows = h.app.dimmed_rows();
        assert!(
            rows.iter()
                .any(|r| r.reason.as_deref() == Some(s.sign_qr_dense)),
            "the reason at {where_}: {rows:?}"
        );
    }
    // §3: on `small` the progress row does not fit, so its count
    // folds into the title; the title still starts with the screen's
    // own name.
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.starts_with(s.sign_qr_title)),
        "the title at {where_}: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == s.sign_qr_label),
        "the label under the code at {where_}"
    );
    assert_one_screenful(&h, "the transaction QR", display);
}

at_every_size!(
    the_transaction_qr_opens_in_the_shape_that_scans_at =>
        the_transaction_qr_opens_in_the_shape_that_scans_on_tiny,
        the_transaction_qr_opens_in_the_shape_that_scans_on_panel,
        the_transaction_qr_opens_in_the_shape_that_scans_on_phone,
        the_transaction_qr_opens_in_the_shape_that_scans_on_desktop
);

/// §4.9: "The module pitch floor is a hard floor at the class's side. A
/// payload whose one QR would fall below it is split into as many BC-UR
/// parts as keep every part above the floor: the part count follows from
/// the side and the floor, not from a fixed fragment size."
///
/// So every part of the demo transaction's run, and of the wallet
/// export's, is measured at the class's side on each display; and the
/// wallet export stays static on a phone, where one code clears the floor
/// (re-review 2026-09-09, §2.1).
fn every_ur_part_clears_the_pitch_floor_at(display: DisplayInfo) {
    let where_ = format!("{}x{}", display.width, display.height);
    let side = px(display, tokens::qr_side(class_of(display)));
    // A UR part is upper-case bytewords, so it encodes in
    // alphanumeric mode; a static payload is bytes. Either way the
    // square is the class's side and the pitch is measured in it.
    let floor = |parts: Vec<String>, alnum: bool, what: &str| {
        assert!(!parts.is_empty(), "{what} has no parts at {where_}");
        for part in parts {
            let bytes = part.as_bytes();
            let payload = if alnum {
                osk_codec::qr::Payload::Alphanumeric(bytes)
            } else {
                osk_codec::qr::Payload::Bytes(bytes)
            };
            let matrix =
                osk_codec::qr::encode(payload, osk_codec::qr::Ecc::Low).expect("a part encodes");
            let mm = osk_ui::widgets::qr_module_pitch_mm(side, matrix.size(), display.dpi);
            assert!(
                mm >= tokens::QR_MIN_PITCH_MM,
                "{what} at {where_}: {} modules is {mm} mm a module",
                matrix.size()
            );
        }
    };
    let mut h = sign_demo(display);
    for _ in 0..4 {
        h.tap(ids::SIGN_CONTINUE);
    }
    h.hold(ids::SIGN_HOLD);
    h.tap(ids::SIGN_QR);
    let (animated, version, psbt) = h.app.sign_qr_payload().expect("the QR page");
    assert!(animated, "the demo PSBT needs parts at {where_}");
    floor(
        codes::UrRun::message_parts(codes::UrKind::Psbt, &psbt, version),
        true,
        "the transaction",
    );

    let mut h = loaded(display);
    h.open_single_sig(0);
    h.tap(ids::WALLET_EXPORT);
    h.tap(ids::EXPORT_SHOW);
    let (animated, version, value) = h.app.export_qr_shape().expect("the export QR");
    if animated {
        floor(
            codes::UrRun::message_parts(codes::UrKind::Bytes, value.as_bytes(), version),
            true,
            "the wallet export",
        );
    } else {
        // Static is still the choice when one code clears the floor,
        // which the demo descriptor does at the class's own side on
        // every one of the four renders.
        floor(vec![value.clone()], false, "the wallet export");
    }
    if display.width == PHONE.width {
        assert!(
            !animated,
            "one code clears the floor on a phone, so the export stays static"
        );
    }
}

at_every_size!(
    every_ur_part_clears_the_pitch_floor_at =>
        every_ur_part_clears_the_pitch_floor_on_tiny,
        every_ur_part_clears_the_pitch_floor_on_panel,
        every_ur_part_clears_the_pitch_floor_on_phone,
        every_ur_part_clears_the_pitch_floor_on_desktop
);

/// §4.3: "An address being typed (Verify) is the one value whose tail is
/// not enough: the space above the entry group holds the typed address
/// whole, as a comparison string at the largest size that fits, on every
/// class, and the field under it keeps the tail in view as usual."
///
/// The block keeps the space's rectangle, so the field does not move when
/// the first character arrives (re-review 2026-09-09, finding 2).
fn verify_shows_the_typed_address_whole_at(display: DisplayInfo) {
    const ADDRESS: &str = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
    let where_ = format!("{}x{}", display.width, display.height);
    let mut h = loaded(display);
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.tap(ids::SCAN_TYPE);
    let empty = h.app.rect_of(ids::FIELD).expect("the field");
    h.type_text(ADDRESS);
    assert_eq!(
        h.app.rect_of(ids::FIELD),
        Some(empty),
        "the field moved when the address was typed at {where_}"
    );
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == ADDRESS),
        "the whole address is above the group at {where_}: {texts:?}"
    );
    let fit = h
        .app
        .chunk_fit(ids::VERIFY_TYPED)
        .unwrap_or_else(|| panic!("no comparison string at {where_}"));
    assert_eq!(fit.pages, 1, "the address is paged at {where_}");
    assert!(
        fit.per_line * fit.lines >= fit.chunks,
        "{} groups are off the screen at {where_}",
        fit.chunks - fit.per_line * fit.lines
    );
    assert!(h.app.overflow().is_empty(), "something spills at {where_}");
}

at_every_size!(
    verify_shows_the_typed_address_whole_at =>
        verify_shows_the_typed_address_whole_on_tiny,
        verify_shows_the_typed_address_whole_on_panel,
        verify_shows_the_typed_address_whole_on_phone,
        verify_shows_the_typed_address_whole_on_desktop
);

/// §4.3: the words-so-far panel "is drawn only once a word has been
/// accepted (an empty panel says nothing), at the fixed geometry of the
/// word count, and while it is drawn the app bar carries the eye"
/// (re-review 2026-09-09, finding 4).
#[test]
fn the_words_so_far_panel_arrives_with_the_first_word_and_the_eye() {
    let mut h = Harness::new(PHONE);
    h.open_load();
    h.load_choices(0);
    // Word 1, nothing accepted: no panel and no eye.
    assert!(
        h.app.rect_of(ids::LOAD_WORDS_PANEL).is_none(),
        "an empty panel is drawn at word 1"
    );
    assert!(
        h.app.rect_of(ids::SECRET_EYE).is_none(),
        "the eye is drawn with nothing to show"
    );
    h.type_text("abandon");
    h.key(osk_shell_api::Key::Enter);
    let first = h
        .app
        .rect_of(ids::LOAD_WORDS_PANEL)
        .expect("the panel from the first accepted word");
    assert!(h.app.rect_of(ids::SECRET_EYE).is_some(), "and the eye");
    assert!(
        !h.app.texts().iter().any(|t| t.contains("abandon")),
        "the panel is masked until it is shown"
    );
    // The eye shows it for 30 s with no finger on the panel (§4.10).
    h.tap(ids::SECRET_EYE);
    assert!(
        h.app.texts().iter().any(|t| t.contains("abandon")),
        "the eye reveals the words so far: {:?}",
        h.app.texts()
    );
    // §4.3: accepting a word masks it again.
    h.type_text("abandon");
    h.key(osk_shell_api::Key::Enter);
    assert!(
        !h.app.texts().iter().any(|t| t.contains("abandon")),
        "the next accepted word masks the panel"
    );
    // The rectangle is the word count's, so it does not grow as the
    // entry goes on.
    for _ in 2..11 {
        h.type_text("abandon");
        h.key(osk_shell_api::Key::Enter);
    }
    assert_eq!(
        h.app.rect_of(ids::LOAD_WORDS_PANEL),
        Some(first),
        "the panel grew between word 2 and word 12"
    );
}

/// §4.6: "the presets as two labelled groups, each with its own check
/// ... 'Purpose': four choice rows named in §4.6's vocabulary (Legacy,
/// Nested, SegWit, Taproot) with the path as the value under the name;
/// 'Chain': the Receive | Change pair. A row's check follows the typed
/// path ... Never 'BIP-84' on the screen."
fn the_path_editor_is_two_labelled_groups_with_one_check_each_at(display: DisplayInfo) {
    let where_ = format!("{}x{}", display.width, display.height);
    let mut h = loaded(display);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    h.tap(ids::EXPLORE_PATH);
    let s = h.app.strings();
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.explore_purpose_group)
            && texts.iter().any(|t| t == s.explore_chain_group),
        "both group labels at {where_}: {texts:?}"
    );
    for t in &texts {
        assert!(!t.contains("BIP-"), "the editor says {t:?} at {where_}");
    }
    // One check in the purpose group, and the pair takes the other
    // choice: the two are independent (§4.6).
    assert_eq!(
        h.app.checked_rows().len(),
        1,
        "checks in the purpose group at {where_}: {:?}",
        h.app.checked_rows()
    );
    assert!(
        h.app.rect_of(ids::EXPLORE_RECEIVE).is_some()
            && h.app.rect_of(ids::EXPLORE_CHANGE).is_some(),
        "the Chain pair at {where_}"
    );
    // §4.6: "On `small` the groups scroll from the top and the
    // checked row is scrolled into view when the editor opens."
    let checked = h
        .app
        .rect_of(ids::at(ids::EXPLORE_PRESET_BASE, 2))
        .unwrap_or_else(|| panic!("the checked row is off the screen at {where_}"));
    assert!(checked.h > 0, "the checked row at {where_}");
    // The pair changes the chain and leaves the purpose alone.
    h.tap(ids::EXPLORE_CHANGE);
    assert_eq!(
        h.app.checked_rows().len(),
        1,
        "the purpose check survives the pair at {where_}"
    );
    assert!(
        h.app.texts().iter().any(|t| t == "m/84h/0h/0h/1/0"),
        "the pair writes the chain level at {where_}: {:?}",
        h.app.texts()
    );
    // A path no preset writes leaves the group unchecked.
    for _ in 0.."84h/0h/0h/1/0".len() {
        h.key(osk_shell_api::Key::Backspace);
    }
    h.type_text("1h");
    assert!(
        h.app.checked_rows().is_empty(),
        "an unlisted path still checks a preset at {where_}: {:?}",
        h.app.checked_rows()
    );
}

at_every_size!(
    the_path_editor_is_two_labelled_groups_with_one_check_each_at =>
        the_path_editor_is_two_labelled_groups_with_one_check_each_on_tiny,
        the_path_editor_is_two_labelled_groups_with_one_check_each_on_panel,
        the_path_editor_is_two_labelled_groups_with_one_check_each_on_phone,
        the_path_editor_is_two_labelled_groups_with_one_check_each_on_desktop
);

/// §5 Transcribe: "one panel or string centred above the actions, at the
/// largest type that fits: a comparison string steps up the mono sizes
/// the tokens name until the next step would not fit the block's width
/// and height in groups of four" (re-review 2026-09-09, finding 8).
#[test]
fn a_comparison_string_takes_the_largest_type_that_fits() {
    // A signature on a phone had 200 dp of an 891 dp screen at the mono
    // token; it now steps up the ramp until the next step would not fit.
    let mut h = sign_demo(PHONE);
    for _ in 0..4 {
        h.tap(ids::SIGN_CONTINUE);
    }
    h.hold(ids::SIGN_HOLD);
    h.tap(ids::SIGN_SIGNATURES);
    h.tap(ids::at(ids::SIGN_SIG_BASE, 0));
    let fit = h.app.chunk_fit(ids::COMPARE_TEXT).expect("the signature");
    assert!(
        fit.size_dp > tokens::MONO,
        "the signature is drawn at {} dp, the mono token or below",
        fit.size_dp
    );
    assert_eq!(fit.pages, 1, "and the whole of it is on the screen");

    // The ramp never costs a panel its fit: the seed hex on the 2.8"
    // panel is still one page inside the Secret screen's panel.
    let mut h = Harness::new(PANEL);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    h.tap(ids::EXPLORE_TYPE);
    h.tap(ids::SCAN_TYPE);
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    h.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
    for w in ABANDON {
        h.type_text(w);
        h.key(osk_shell_api::Key::Enter);
    }
    h.tap(ids::LOAD_CONTINUE);
    h.tap(ids::EXPLORE_BITS);
    h.tap(ids::EXPLORE_SEED);
    assert!(
        h.app.paged_chunks().is_empty(),
        "the seed hex pages on the panel: {:?}",
        h.app.paged_chunks()
    );
    assert!(h.app.overflow().is_empty(), "the panel spills");
}

/// A descriptor is shown by its tokens, not chunked in fours: the key
/// origin and the checksum each arrive whole, and the shortened key
/// opens the account key (UX review 2026-09-07, §2.9).
#[test]
fn a_descriptor_keeps_its_origin_and_checksum_whole() {
    let mut h = loaded(PANEL);
    h.open_single_sig(0);
    h.tap(ids::WALLET_EXPORT);
    // §4.5: the export string is a reference row, and Compare shows a
    // descriptor as structure rather than chunked in fours.
    h.tap(ids::EXPORT_TEXT);
    let texts = h.app.texts();
    let origin = texts
        .iter()
        .find(|t| t.starts_with('[') && t.ends_with(']'))
        .unwrap_or_else(|| panic!("the key origin as one token: {texts:?}"));
    assert!(origin.ends_with("/84h/0h/0h]"), "{origin}");
    assert_eq!(origin.len(), "[73c5da0a/84h/0h/0h]".len());
    let checksum = texts
        .iter()
        .find(|t| t.starts_with('#'))
        .unwrap_or_else(|| panic!("the checksum as one token: {texts:?}"));
    assert_eq!(checksum.chars().count(), 9, "{checksum}");
    // Nothing is chunked here: the shortened key is one token too.
    assert!(
        texts.iter().any(|t| t.contains('\u{2026}')),
        "the key is shortened: {texts:?}"
    );
    // The shortened key opens the whole account key, chunked.
    h.tap(ids::COMPARE_KEY);
    assert_chunked_whole(&h, ids::COMPARE_TEXT, "the account key", "480x640");
    assert!(
        h.app.texts().iter().any(|t| t.starts_with("xpub")),
        "the whole account key"
    );
}

// ----- App pass 1: the screens rebuilt from docs/DESIGN.md §5 -----

/// Every screen this pass rebuilt is drawn by a function in
/// `osk_ui::screens`, which is what makes one kind of content look the
/// same everywhere. Each of the sixteen tags the pane it draws with
/// `screens::SCREEN`, and the entry and pad screens also reserve the
/// field and the pad rectangle, so the layout says which screen built
/// it.
#[test]
fn every_screen_kind_is_reachable_by_a_walk() {
    use opensigner_core::load::Step as LoadStep;
    let mut seen: Vec<ScreenKind> = Vec::new();
    let mut h = Harness::new(PANEL);
    macro_rules! built {
        ($h:expr, $name:expr) => {{
            let _ = $name;
            let kind = $h.app.screen();
            if !seen.contains(&kind) {
                seen.push(kind);
            }
        }};
    }
    built!(&h, "home");
    h.open_keys();
    built!(&h, "keys");
    h.tap(ids::KEYS_LOAD);
    for (name, row, cont) in [
        (
            "load source",
            ids::LOAD_SOURCE_TYPE,
            ids::LOAD_SOURCE_CONTINUE,
        ),
        (
            "load count",
            ids::at(ids::LOAD_COUNT_BASE, 0),
            ids::LOAD_COUNT_CONTINUE,
        ),
        (
            "load language",
            ids::at(ids::LOAD_LANG_BASE, 0),
            ids::LOAD_LANG_CONTINUE,
        ),
    ] {
        built!(&h, name);
        h.choose(row, cont);
    }
    built!(&h, "word entry");
    assert!(h.app.rect_of(ids::FIELD).is_some(), "the word field");
    for w in ABANDON {
        h.type_text(w);
        h.key(osk_shell_api::Key::Enter);
    }
    built!(&h, "checksum");
    h.tap(ids::LOAD_CONTINUE);
    built!(&h, "passphrase offer");
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    built!(&h, "load confirm");
    h.tap(ids::LOAD_HOLD);
    assert_eq!(h.app.load_step(), Some(LoadStep::Pin));
    built!(&h, "set a pin");
    assert!(h.app.rect_of(ids::PAD_SLOT).is_some(), "the pad rectangle");
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2580");
    built!(&h, "repeat the pin");
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2580");
    h.open_key(0);
    built!(&h, "the key menu");
    h.tap(ids::DETAIL_BACKUP);
    built!(&h, "the backup menu");
    h.tap(ids::BACKUP_SEEDQR);
    built!(&h, "the SeedQR");
    h.tap(ids::BACK);
    h.tap(ids::BACKUP_WORDS);
    built!(&h, "the words");
    h.tap(ids::QUIZ_DONE);
    h.tap(ids::BACKUP_VERIFY);
    built!(&h, "the quiz start");
    h.tap(ids::QUIZ_START);
    built!(&h, "a quiz question");
    let v = h.app.quiz_view().expect("a quiz");
    let right = ABANDON[v.word_number - 1];
    let slot = v.choices.iter().position(|c| c == right).expect("the word");
    h.choose(
        ids::at(ids::QUIZ_CHOICE_BASE, (slot + 1) % 4),
        ids::QUIZ_CONTINUE,
    );
    built!(&h, "a wrong answer");
    h.tap(ids::BACK);
    built!(&h, "a quiz that did not finish");
    h.tap(ids::QUIZ_DONE);
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::STATUS_LOCK);
    built!(&h, "the lock screen");
    assert!(h.app.rect_of(ids::PAD_SLOT).is_some(), "the pad rectangle");

    // The Create wizard, whose own steps are the rest of the sixteen.
    let mut h = Harness::new(PANEL);
    h.open_create();
    for (name, row, cont) in [
        (
            "create source",
            ids::at(ids::CREATE_SOURCE_BASE, 0),
            ids::CREATE_SOURCE_CONTINUE,
        ),
        (
            "create count",
            ids::at(ids::CREATE_COUNT_BASE, 0),
            ids::CREATE_COUNT_CONTINUE,
        ),
        (
            "create language",
            ids::at(ids::CREATE_LANG_BASE, 0),
            ids::CREATE_LANG_CONTINUE,
        ),
        (
            "create dice procedure",
            ids::at(ids::CREATE_PROCEDURE_BASE, 0),
            ids::CREATE_PROCEDURE_CONTINUE,
        ),
    ] {
        built!(&h, name);
        h.choose(row, cont);
    }
    built!(&h, "the dice pad");
    assert!(h.app.rect_of(ids::PAD_SLOT).is_some(), "the pad rectangle");
    h.type_text("32461151351521144121541512665155412152342515356215");
    h.key(osk_shell_api::Key::Enter);
    built!(&h, "the sanity check");
    h.tap(ids::CREATE_MATH);
    built!(&h, "the math");
    h.tap(ids::CREATE_MATH_ENTROPY);
    built!(&h, "the entropy");
    h.tap(ids::BACK);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::CREATE_CONTINUE);
    built!(&h, "the created words");
    h.tap(ids::CREATE_CONTINUE);
    built!(&h, "the quiz start");
    h.tap(ids::QUIZ_SKIP);
    built!(&h, "the skip caution");

    // App pass 3: Sign, Verify, the address list and its detail, the
    // wallet export, the passphrase and the forget screens, and the
    // Choice and Compare screens their rows open.
    let mut h = loaded(PANEL);
    h.open_wallets();
    built!(&h, "wallets");
    h.tap(ids::WALLETS_ADD);
    built!(&h, "add a wallet");
    h.tap(ids::BUILD_NEW);
    built!(&h, "what kind of wallet");
    h.choose(ids::at(ids::BUILD_KIND_BASE, 0), ids::BUILD_KIND_CONTINUE);
    built!(&h, "which keys");
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 0));
    h.tap(ids::BUILD_CONTINUE);
    built!(&h, "which script type");
    h.choose(
        ids::at(ids::BUILD_SCRIPT_BASE, 2),
        ids::BUILD_SCRIPT_CONTINUE,
    );
    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    h.tap(ids::WALLET_KEYS);
    built!(&h, "a wallet's keys");
    h.tap(ids::BACK);
    h.tap(ids::WALLET_ADDRESSES);
    built!(&h, "the address list");
    h.tap(ids::at(ids::ADDR_ROW_BASE, 0));
    built!(&h, "one address");
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::WALLET_EXPORT);
    built!(&h, "the wallet export");
    h.tap(ids::EXPORT_FORMAT);
    built!(&h, "the format choice");
    h.tap(ids::BACK);
    h.tap(ids::EXPORT_TEXT);
    built!(&h, "the export string");
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::EXPORT_SHOW);
    built!(&h, "the wallet-export QR");
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::WALLET_KEYS);
    built!(&h, "the key behind the wallet");
    h.tap(ids::BACK);
    h.go_home();
    h.open_passphrase(0);
    built!(&h, "the passphrase a key is opened behind");
    h.tap(ids::BACK);
    h.open_child(0);
    built!(&h, "the child's word count");
    h.choose(
        ids::at(ids::OPEN_CHILD_WORDS_BASE, 0),
        ids::OPEN_CHILD_CONTINUE,
    );
    built!(&h, "the child's index");
    h.type_pin(ids::OPEN_CHILD_KEYBOARD, "0");
    built!(&h, "the key that was opened");
    h.go_home();
    // A wallet whose key is gone asks for that one key, so Add a key
    // offers the ways one key opens another and asks which key to
    // start from (§16.104 rule 6).
    h.add_single_sig(1, 2);
    h.open_key(1);
    h.tap(ids::KEY_FORGET);
    h.hold(ids::DETAIL_FORGET);
    h.open_wallets();
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 1));
    h.tap(ids::WALLET_KEYS);
    h.tap(ids::at(ids::WALLET_KEY_ROW_BASE, 0));
    built!(&h, "add a key, for the key a wallet is missing");
    h.tap(ids::ADD_OPEN_PASSPHRASE);
    built!(&h, "which key the passphrase opens");
    h.go_home();
    // BIP-85's other applications: the length, the index and the value
    // (`docs/PLANNING.md` §16.114).
    h.open_key(0);
    h.tap(ids::DETAIL_BIP85);
    built!(&h, "which BIP-85 application");
    h.choose(ids::at(ids::PICK_BASE, 4), ids::PICK_CONTINUE);
    built!(&h, "the password's length");
    h.type_pin(ids::BIP85_LENGTH_PAD, "");
    built!(&h, "the application's index");
    h.type_pin(ids::BIP85_INDEX_PAD, "");
    built!(&h, "the derived value");
    h.tap(ids::BIP85_DONE);
    h.go_home();
    // The vanity grinder: the script type, the prefix, the run and the
    // find (`docs/PLANNING.md` §16.117).
    h.open_key(0);
    h.tap(ids::DETAIL_VANITY);
    built!(&h, "which dial");
    h.choose(ids::at(ids::PICK_BASE, 1), ids::PICK_CONTINUE);
    built!(&h, "the grinder's script type");
    h.choose(
        ids::at(ids::VANITY_SCRIPT_BASE, 2),
        ids::VANITY_SCRIPT_CONTINUE,
    );
    built!(&h, "the prefix");
    h.pad(ids::VANITY_KEYBOARD, KeyInput::Char('q'));
    h.pad(ids::VANITY_KEYBOARD, KeyInput::Done);
    built!(&h, "the grind");
    for _ in 0..200 {
        if h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.vanity_found_title)
        {
            break;
        }
        h.tick(16);
    }
    built!(&h, "the find");
    h.go_home();
    h.open_key(0);
    h.tap(ids::KEY_FORGET);
    built!(&h, "the forget screen");
    h.tap(ids::BACK);
    h.go_home();
    h.open_single_sig(0);
    h.tap(ids::WALLET_CHECK);
    built!(&h, "the address check");
    h.tap(ids::SCAN_TYPE);
    built!(&h, "the address field");
    h.type_text("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4");
    h.key(osk_shell_api::Key::Enter);
    built!(&h, "the verify result");
    h.tap(ids::VERIFY_ADDRESS);
    built!(&h, "the address, whole");
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::BACK);
    h.tap(ids::BACK);

    // The file list a shell that can list its files answers the
    // scanner's "Read a file" with, and the file that is tapped.
    let mut h = loaded(PANEL);
    h.set_network(3);
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN);
    h.tap(ids::SCAN_FILE);
    h.send(osk_shell_api::Event::FileList {
        kind: osk_shell_api::FileKind::Any,
        entries: vec![osk_shell_api::FileEntry {
            name: String::from("demo-regtest.psbt"),
            size: DEMO.len() as u64,
            modified: Some(1_770_000_000),
        }],
        place: Some(String::from("OSKDATA")),
    });
    built!(&h, "the file list");
    h.tap(ids::at(ids::FILES_ROW_BASE, 0));
    h.send(osk_shell_api::Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    built!(&h, "the transaction the file held");

    let mut h = sign_demo(PANEL);
    built!(&h, "the sign review");
    h.tap(ids::SIGN_CONTINUE);
    built!(&h, "an output");
    h.tap(ids::SIGN_CONTINUE);
    h.tap(ids::SIGN_CONTINUE);
    built!(&h, "the inputs");
    h.tap(ids::SIGN_CONTINUE);
    built!(&h, "the confirm");
    h.hold(ids::SIGN_HOLD);
    built!(&h, "the sign result");
    h.tap(ids::SIGN_SIGNATURES);
    built!(&h, "the signatures");
    h.tap(ids::BACK);
    h.tap(ids::SIGN_QR);
    built!(&h, "the transaction QR");

    // Signing a message, and the answer for one that was signed
    // somewhere else.
    let mut h = loaded(PANEL);
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN_MESSAGE);
    h.tap(ids::SCAN_FILE);
    h.send(osk_shell_api::Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: MESSAGE.to_vec(),
    });
    built!(&h, "the message to sign");
    h.tap(ids::MSG_FORMAT);
    built!(&h, "the format choice");
    h.tap(ids::BACK);
    h.tap(ids::MSG_CONTINUE);
    built!(&h, "the hold that signs a message");
    h.hold(ids::MSG_HOLD);
    built!(&h, "the signature");
    h.tap(ids::MSG_SIGNATURE);
    built!(&h, "the signature, whole");
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::MSG_QR);
    built!(&h, "the signature QR");
    h.tap(ids::BACK);
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::MSG_CHECK_ROW);
    h.tap(ids::SCAN_FILE);
    h.send(osk_shell_api::Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: SIGNED_MESSAGE.to_vec(),
    });
    built!(&h, "the checked message");
    h.tap(ids::MSG_READ);
    built!(&h, "the message that was checked");
    h.tap(ids::BACK);
    h.tap(ids::MSG_DONE);

    // App pass 4: the scanner and its three results, Inspect, Settings
    // and its Setting Choices, About, the tiers, and both wipes.
    let mut h = loaded(PANEL);
    h.tap(ids::HOME_SCAN);
    built!(&h, "the scanner");
    assert!(h.app.rect_of(ids::VIEWFINDER).is_some(), "the square");
    h.send(osk_shell_api::Event::CameraUnavailable);
    built!(&h, "the scanner without a camera");
    h.tap(ids::BACK);
    h.tap(ids::HOME_SCAN);
    h.scan_bytes(b"hello, world, and a line too long for one row");
    built!(&h, "an unknown QR");
    h.tap(ids::SCAN_HEX);
    built!(&h, "the unknown code, whole");
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::SCAN_AS_TEXT);
    built!(&h, "a scanned text");
    h.tap(ids::INSPECT_TEXT);
    built!(&h, "the text, whole");
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::BACK);
    h.tap(ids::HOME_SCAN);
    h.scan_bytes(ABANDON.join(" ").as_bytes());
    built!(&h, "the words caution");
    h.tap(ids::BACK);
    h.scan_bytes(b"000000000000000000000000000000000000000000000000");
    built!(&h, "a QR that could not be used");
    h.tap(ids::SCAN_AGAIN);
    h.tap(ids::BACK);

    h.open_settings();
    built!(&h, "settings");
    for row in [
        ids::SETTINGS_NETWORK_ROW,
        ids::SETTINGS_UNIT_ROW,
        ids::SETTINGS_LOCK_AFTER_ROW,
        ids::SETTINGS_WIPE_AFTER_ROW,
    ] {
        h.tap(row);
        built!(&h, "a setting");
        h.tap(ids::BACK);
    }
    h.tap(ids::SETTINGS_ABOUT_ROW);
    built!(&h, "about");
    h.tap(ids::ABOUT_TIER);
    built!(&h, "the tiers");
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::SETTINGS_EXIT_ROW);
    built!(&h, "wipe and exit");
    h.tap(ids::BACK);
    h.tap(ids::SETTINGS_WIPE_ROW);
    built!(&h, "wipe all keys");
    h.hold(ids::SETTINGS_WIPE);
    built!(&h, "the keys removed");
    h.tap(ids::SETTINGS_WIPED_DONE);
    assert_eq!(h.app.screen(), ScreenKind::Home);

    // App pass 5: Explore. The two menus, the chooser, the path and
    // passphrase editors, the Words screen, the five Secret screens and
    // the discard confirm.
    let mut h = loaded(PANEL);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    built!(&h, "the explore key menu");
    h.tap(ids::EXPLORE_USING);
    built!(&h, "the key chooser");
    h.tap(ids::EXPLORE_CHOOSE_CONTINUE);
    h.tap(ids::EXPLORE_PATH);
    built!(&h, "the path editor");
    assert!(h.app.rect_of(ids::FIELD).is_some(), "the path field");
    h.key(osk_shell_api::Key::Enter);
    h.tap(ids::EXPLORE_ACCOUNT_XPUB);
    built!(&h, "the account key, whole");
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::EXPLORE_XPUB);
    built!(&h, "the extended public key, whole");
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::EXPLORE_XPRV);
    built!(&h, "the extended private key");
    h.tap(ids::BACK);
    h.tap(ids::EXPLORE_ADDRESSES);
    built!(&h, "the explored key's addresses");
    h.tap(ids::BACK);
    h.tap(ids::EXPLORE_BITS);
    built!(&h, "words and bits");
    h.tap(ids::EXPLORE_WORDS);
    built!(&h, "the explored words");
    h.tap(ids::EXPLORE_WORDS_DONE);
    for row in [
        ids::EXPLORE_ENTROPY,
        ids::EXPLORE_CHECKSUM_BITS,
        ids::EXPLORE_SEED,
        ids::EXPLORE_MASTER_XPRV,
    ] {
        h.tap(row);
        built!(&h, "an explore secret");
        h.tap(ids::BACK);
    }
    h.tap(ids::BACK);
    h.tap(ids::BACK);

    // Explore over typed words: the passphrase Entry and the confirm
    // that guards the words on the way out.
    let mut h = Harness::new(PANEL);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    built!(&h, "explore with nothing loaded");
    h.tap(ids::EXPLORE_TYPE);
    h.tap(ids::SCAN_TYPE);
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    h.choose(ids::at(ids::LOAD_LANG_BASE, 0), ids::LOAD_LANG_CONTINUE);
    for w in ABANDON {
        h.type_text(w);
        h.key(osk_shell_api::Key::Enter);
    }
    h.tap(ids::LOAD_CONTINUE);
    h.tap(ids::EXPLORE_PASSPHRASE);
    built!(&h, "the explore passphrase");
    h.key(osk_shell_api::Key::Enter);
    h.tap(ids::BACK);
    built!(&h, "the discard confirm");
    h.tap(ids::EXPLORE_DISCARD);

    // Learn: the list of pages and one page, neither of which needs a
    // key or a session.
    let mut h = Harness::new(PANEL);
    h.tap(ids::at(ids::HOME_TILE_BASE, 4));
    built!(&h, "the Learn list");
    h.tap(ids::at(ids::LEARN_ROW_BASE, 0));
    built!(&h, "a Learn page");

    // The session's own ending, which has no way back.
    let mut h = loaded(PANEL);
    h.open_settings();
    h.tap(ids::SETTINGS_EXIT_ROW);
    built!(&h, "wipe and exit");
    h.hold(ids::SETTINGS_EXIT);
    built!(&h, "the session's end");

    // The screens of a key kept on the device, which exist only on a
    // Tier B shell with a secure element: the Hold that keeps it, the
    // Pad that opens it, the duress Pad, the Hold that forgets it, and
    // the Result eight wrong PINs leave.
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    built!(&h, "keep on this device");
    h.hold(ids::KEEP_HOLD);
    let element = h.element.clone().expect("an element");

    let mut h = Harness::kept(SECURE_PHONE, element.clone());
    built!(&h, "the stored key's pad");
    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    h.open_settings();
    h.tap(ids::KEEP_DURESS_ROW);
    built!(&h, "the duress pad");
    h.tap(ids::BACK);

    let mut h = Harness::kept(SECURE_PHONE, element);
    for _ in 0..osk_keep::KEEP_ATTEMPTS {
        h.type_pin(ids::KEEP_PIN_KEYBOARD, "9999");
    }
    built!(&h, "the stored key removed");

    // What a Tier A device opens on, which no other tier reaches.
    let h = Harness::with_tier(PANEL, AssuranceTier::A, true);
    built!(&h, "no secure boot");

    // A phone whose bootloader is unlocked, or whose boot the shell
    // could not verify, opens on the refusal and nothing else.
    let h = Harness::kept(UNVERIFIED_PHONE, Element::default());
    built!(&h, "the boot refusal");

    // A wallet loaded from Add: the review the row reaches, its page,
    // the addresses it derives and the export a coordinator takes.
    let mut h = loaded(PANEL);
    h.set_network(3);
    h.go_home();
    h.open_keys();
    built!(&h, "keys");
    h.tap(ids::KEYS_ADD);
    built!(&h, "add a key");
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.open_wallets();
    built!(&h, "wallets");
    h.tap(ids::WALLETS_ADD);
    built!(&h, "add a wallet");
    h.tap(ids::WALLETS_LOAD);
    h.send(osk_shell_api::Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(osk_shell_api::Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: WALLET_POLICY.to_vec(),
    });
    built!(&h, "the wallet review");
    h.tap(ids::INSPECT_USE_WALLET);
    built!(&h, "the wallet menu");
    h.tap(ids::WALLET_NAME);
    built!(&h, "the wallet's name");
    h.tap(ids::BACK);
    // The wallet's recovery sheet, and the note written on it
    // (`docs/PLANNING.md` §16.112 rule 2).
    h.tap(ids::WALLET_SHEET);
    built!(&h, "the wallet's recovery sheet");
    h.tap(ids::SHEET_NOTE);
    built!(&h, "the sheet's note being typed");
    h.type_text("The third key is with the notary.");
    h.key(osk_shell_api::Key::Enter);
    h.tap(ids::BACK);
    h.tap(ids::WALLET_ADDRESSES);
    built!(&h, "the wallet's addresses");
    h.tap(ids::at(ids::ADDR_ROW_BASE, 0));
    built!(&h, "one of the wallet's addresses");
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::WALLET_EXPORT);
    built!(&h, "the wallet export");
    h.tap(ids::EXPORT_FORMAT);
    built!(&h, "the wallet export's formats");
    h.tap(ids::BACK);
    h.tap(ids::EXPORT_SHOW);
    built!(&h, "the wallet export's QR");
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::WALLET_KEYS);
    built!(&h, "the wallet's keys");
    h.tap(ids::BACK);
    h.tap(ids::WALLET_FORGET);
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
    built!(&h, "wallets again");
    h.go_home();
    built!(&h, "home");

    // A single-sig wallet added by hand: which key, which script type,
    // and the review every wallet ends at.
    h.open_wallets();
    h.tap(ids::WALLETS_ADD);
    h.tap(ids::BUILD_NEW);
    h.choose(ids::at(ids::BUILD_KIND_BASE, 0), ids::BUILD_KIND_CONTINUE);
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 0));
    h.tap(ids::BUILD_CONTINUE);
    h.tap(ids::BUILD_SCRIPT_CONTINUE);
    built!(&h, "the single-sig review");
    h.tap(ids::BUILD_ADD_WALLET);
    h.tap(ids::WALLET_KEYS);
    built!(&h, "a wallet's keys review");
    h.go_home();

    // Add a wallet as a multisig: the kind, the keys it gathers, the
    // script type, the threshold, the review and the confirm on the way
    // out.
    h.go_home();
    h.open_wallets();
    h.tap(ids::WALLETS_ADD);
    h.tap(ids::BUILD_NEW);
    h.choose(ids::at(ids::BUILD_KIND_BASE, 1), ids::BUILD_KIND_CONTINUE);
    built!(&h, "the wizard's keys");
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 0));
    built!(&h, "one key of the wallet being built");
    // A cosigner that is not here is read at the scanner as a file.
    h.tap(ids::BUILD_WHICH_SCAN);
    h.send(osk_shell_api::Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(osk_shell_api::Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: cosigner_key(),
    });
    h.tap(ids::BUILD_CONTINUE);
    built!(&h, "the multisig script type");
    h.tap(ids::BUILD_SCRIPT_CONTINUE);
    built!(&h, "how many must sign");
    h.tap(ids::BUILD_THRESHOLD_CONTINUE);
    built!(&h, "the built wallet's review");
    // Review, threshold, script type, keys, the kind, and then the
    // question that guards the keys on the way out.
    for _ in 0..5 {
        h.tap(ids::BACK);
    }
    built!(&h, "the wizard's discard confirm");
    h.tap(ids::BUILD_DISCARD);
    assert_eq!(h.app.screen(), ScreenKind::AddWallet);
    h.go_home();

    // Tools, and the key explorer behind its first row.
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    built!(&h, "tools");

    // Tools › Word list: the language, the search, the Choice that picks
    // the notation, and one word's record.
    h.tap(ids::TOOLS_WORD_LIST);
    built!(&h, "the word list's language");
    h.tap(ids::WORDLIST_LANG_CONTINUE);
    built!(&h, "the word search");
    h.tap(ids::WORDLIST_BY);
    built!(&h, "search by");
    h.tap(ids::at(ids::PICK_BASE, 1));
    h.tap(ids::PICK_CONTINUE);
    built!(&h, "the search by number");
    h.tap(ids::WORDLIST_BY);
    h.tap(ids::at(ids::PICK_BASE, 4));
    h.tap(ids::PICK_CONTINUE);
    built!(&h, "one word");
    h.tap(ids::WORD_NEXT);
    built!(&h, "the next word");
    // Back from a word is the search, then the language, then Tools.
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Tools);

    // Tools › Dice passphrase: the list, the length, the pad, the
    // passphrase and the confirm that guards the rolls.
    h.tap(ids::TOOLS_DICE);
    built!(&h, "the diceware list");
    h.tap(ids::DICE_LIST_CONTINUE);
    built!(&h, "how many words");
    h.tap(ids::at(ids::DICE_WORDS_BASE, 0));
    h.tap(ids::DICE_WORDS_CONTINUE);
    built!(&h, "the dice pad");
    for i in 0..20 {
        h.pad(
            ids::DICE_PAD,
            KeyInput::Char((b'1' + (i % 6) as u8) as char),
        );
    }
    h.tap(ids::DICE_CONTINUE);
    built!(&h, "the passphrase");
    h.tap(ids::BACK);
    built!(&h, "the dice discard confirm");
    h.tap(ids::DICE_DISCARD);
    assert_eq!(h.app.screen(), ScreenKind::Tools);

    // Tools › the calculators: one Entry each, and the Record its ✓
    // opens.
    h.tap(ids::at(ids::TOOLS_CALC_BASE, 0));
    built!(&h, "the hashes scanner");
    h.tap(ids::SCAN_TYPE);
    built!(&h, "the hashes field");
    h.type_text("abc");
    h.key(Key::Enter);
    built!(&h, "the hashes");
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Tools);

    // Tools › Decode a transaction: the Sign review over a transaction
    // no key here can sign.
    h.tap(ids::TOOLS_DECODE);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    built!(&h, "the decoded transaction");
    h.go_home();

    // Tools › Compare transactions: the Menu that asks for the second
    // one, and the Result that says what differs (§16.111 rule 4).
    h.open_tile(TILE_TOOLS);
    h.tap(ids::TOOLS_COMPARE);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    built!(&h, "the second transaction's way in");
    h.tap(ids::COMPARE_TX_SECOND);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });
    built!(&h, "two transactions compared");
    h.tap(ids::COMPARE_TX_DONE);
    h.go_home();

    // Tools › Notes (§16.112 rule 2): the notes in hand, the entry one
    // is typed on, the Document it lands on, the form it leaves in, the
    // passphrase it is sealed under and the file that makes.
    h.open_tile(TILE_TOOLS);
    h.tap(ids::TOOLS_NOTES);
    built!(&h, "the notes in hand");
    h.tap(ids::NOTES_NEW);
    built!(&h, "a note being typed");
    h.type_text("Keys in the safe.");
    h.key(osk_shell_api::Key::Enter);
    built!(&h, "the note");
    h.tap(ids::NOTE_EXPORT);
    built!(&h, "which form the note leaves in");
    h.tap(ids::FORM_CONTINUE);
    built!(&h, "the note's passphrase");
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    built!(&h, "the sealed note");
    h.tap(ids::SEAL_SHOW_QR);
    built!(&h, "the sealed note as a QR");
    h.go_home();

    // Tools › Lightning node key (§16.116): the source Choice, the
    // passphrase of an LND cipher seed, the Result and the two secrets
    // behind it.
    h.open_tile(TILE_TOOLS);
    h.tap(ids::TOOLS_LIGHTNING);
    built!(&h, "where a node key comes from");
    h.choose(ids::at(ids::PICK_BASE, 0), ids::PICK_CONTINUE);
    let seed_words: Vec<&str> = LND_CIPHER_SEED.split_whitespace().collect();
    for w in &seed_words[..seed_words.len() - 1] {
        h.type_word(w);
    }
    h.type_text(seed_words[seed_words.len() - 1]);
    if h.app.load_step().is_some() {
        h.key(osk_shell_api::Key::Enter);
    }
    built!(&h, "a cipher seed's passphrase");
    h.key(osk_shell_api::Key::Enter);
    built!(&h, "the node key");
    h.tap(ids::LIGHTNING_NODE_KEY);
    built!(&h, "the node key, whole");
    h.tap(ids::COMPARE_DONE);
    h.tap(ids::LIGHTNING_SECRET);
    built!(&h, "the node private key");
    h.tap(ids::LIGHTNING_ENTROPY);
    built!(&h, "the cipher seed's entropy");
    h.tap(ids::LIGHTNING_DONE);
    h.go_home();

    // Add a key › "Load a key" with 24 words, which is what a FROST
    // member is (`docs/PLANNING.md` §16.104 rule 3).
    h.go_home();
    let share_0: Vec<&str> = SHARE_0_WORDS.split_whitespace().collect();
    h.start_load_24(&share_0);
    built!(&h, "a 24-word key's checksum");
    h.tap(ids::BACK);
    h.go_home();

    // Add a wallet › FROST: the counts that come before its keys.
    h.open_add_wallet(ids::BUILD_NEW);
    h.choose(ids::at(ids::BUILD_KIND_BASE, 4), ids::BUILD_KIND_CONTINUE);
    built!(&h, "how many keys");
    h.tap(ids::THRESHOLD_COUNT_CONTINUE);
    built!(&h, "how many of them must sign");
    h.tap(ids::THRESHOLD_QUORUM_CONTINUE);
    built!(&h, "the FROST keys step");
    h.go_home();

    // Add a wallet › Recovery: the second path and the wait, which no
    // other kind has. Two paths need two keys, so the 24-word key goes
    // in first.
    h.start_load_24(&share_0);
    h.finish_load(None);
    h.go_home();
    h.open_add_wallet(ids::BUILD_NEW);
    h.choose(ids::at(ids::BUILD_KIND_BASE, 5), ids::BUILD_KIND_CONTINUE);
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 0));
    h.tap(ids::BUILD_CONTINUE);
    built!(&h, "who can sign later");
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 1));
    h.tap(ids::BUILD_LATER_CONTINUE);
    built!(&h, "after how long");
    h.tap(ids::BUILD_DELAY_TYPE);
    h.tap(ids::BUILD_DELAY_CONTINUE);
    built!(&h, "a wait of its own number of days");
    h.tap(ids::BACK);
    h.tap(ids::at(ids::BUILD_DELAY_BASE, 0));
    h.tap(ids::BUILD_DELAY_CONTINUE);
    built!(&h, "another path later");
    h.tap(ids::BUILD_ANOTHER_CONTINUE);
    built!(&h, "the recovery wallet's script type");
    for _ in 0..6 {
        h.tap(ids::BACK);
    }
    built!(&h, "the recovery wallet's discard confirm");
    h.tap(ids::BUILD_DISCARD);
    h.go_home();

    // The first run, which is a device with no settings at all: the
    // document it opens on, and the Result the key it creates lands on.
    let mut h = Harness::first_run(PANEL);
    built!(&h, "start here");
    h.tap(ids::START_HERE_CONTINUE);
    h.tap(ids::KEYS_CREATE);
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 2),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    h.type_text("00000000000000000000000000000000");
    h.key(osk_shell_api::Key::Enter);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::QUIZ_START);
    for _ in 0..ABANDON.len() {
        let v = h.app.quiz_view().expect("a quiz on screen");
        let want = ABANDON[v.word_number - 1];
        let slot = v
            .choices
            .iter()
            .position(|c| c == want)
            .expect("the word is offered");
        h.choose(ids::at(ids::QUIZ_CHOICE_BASE, slot), ids::QUIZ_CONTINUE);
    }
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    built!(&h, "the created key");

    // The lock screen and the self-test failure are the two screens a
    // walk cannot reach from here: the first needs a session PIN, which
    // the first harness set, and the second is injected by a unit test
    // in `lib.rs` that makes the same assertion.
    for kind in ScreenKind::ALL {
        if kind == ScreenKind::SelfTestFailed {
            continue;
        }
        assert!(
            seen.contains(&kind),
            "{kind:?} is never reached, so nothing proves it comes from §5"
        );
    }
}

/// §2.7: "A hold confirms an action the app cannot undo on its own."
/// Adding a key is undone by forgetting it, so the Load wizard's confirm
/// is a tap.
#[test]
fn the_load_confirm_has_no_hold() {
    use opensigner_core::load::Step as LoadStep;
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    assert_eq!(h.app.load_step(), Some(LoadStep::Confirm));
    assert!(h.app.hold_buttons().is_empty(), "adding a key is a tap");
    assert!(
        h.app.rect_of(ids::LOAD_HOLD).is_some(),
        "the Add key button"
    );
}

/// §2.5 and §4.3: the PIN pad is the same size in the same place on the
/// lock screen and on both set-PIN steps, whatever the screen above it
/// says and however many digits have been typed.
fn the_pin_pad_is_in_one_place_on_every_pin_screen_at(display: DisplayInfo) {
    let where_ = format!("{}x{}", display.width, display.height);
    let mut h = Harness::new(display);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.tap(ids::LOAD_HOLD);
    let set = h.app.rect_of(ids::PAD_SLOT).expect("the pad on Set a PIN");
    h.pad(ids::LOAD_PIN_KEYBOARD, KeyInput::Char('2'));
    assert_eq!(
        h.app.rect_of(ids::PAD_SLOT),
        Some(set),
        "a digit moved the pad at {where_}"
    );
    h.pad(ids::LOAD_PIN_KEYBOARD, KeyInput::Backspace);
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2580");
    let repeat = h.app.rect_of(ids::PAD_SLOT).expect("the pad on Repeat");
    assert_eq!(repeat, set, "the pad moved on Repeat at {where_}");
    // A mismatch restarts the entry and puts "PINs differed" on the
    // reserved caption line; the pad does not move for it.
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2581");
    assert!(h.app.rect_of(ids::INLINE_ERROR).is_some());
    assert_eq!(
        h.app.rect_of(ids::PAD_SLOT),
        Some(set),
        "the error moved the pad at {where_}"
    );
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2580");
    h.type_pin(ids::LOAD_PIN_KEYBOARD, "2580");
    h.tap(ids::STATUS_LOCK);
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    assert_eq!(
        h.app.rect_of(ids::PAD_SLOT),
        Some(set),
        "the pad moved on the lock screen at {where_}"
    );
    h.type_pin(ids::LOCK_KEYBOARD, "1111");
    assert_eq!(
        h.app.rect_of(ids::PAD_SLOT),
        Some(set),
        "a wrong PIN moved the pad at {where_}"
    );
}

at_every_size!(
    the_pin_pad_is_in_one_place_on_every_pin_screen_at =>
        the_pin_pad_is_in_one_place_on_every_pin_screen_on_tiny,
        the_pin_pad_is_in_one_place_on_every_pin_screen_on_panel,
        the_pin_pad_is_in_one_place_on_every_pin_screen_on_phone,
        the_pin_pad_is_in_one_place_on_every_pin_screen_on_desktop
);

/// A key on the smallest panel is big enough to hit. Ten keys across a
/// 240 px panel cannot each be a chip's touch floor wide, so the target
/// is measured against a finger instead: every key of the word keyboard
/// and of the PIN pad clears [`tokens::KEY_MIN_TAP_MM`] in both
/// directions, and a tap in its corner still types it.
#[test]
fn every_key_is_big_enough_to_hit_on_the_smallest_panel() {
    let display = TINY;
    let mm = |px: i32| px as f32 * 25.4 / f32::from(display.dpi);
    let mut h = Harness::new(display);
    h.open_load();
    h.load_choices(0);
    let mut keys = Vec::new();
    for c in 'a'..='z' {
        if let Some(r) = h.app.key_rect(ids::LOAD_KEYBOARD, KeyInput::Char(c)) {
            keys.push((format!("{c}"), r));
        }
    }
    assert!(keys.len() > 20, "the word keyboard has its letters");
    if let Some(r) = h.app.key_rect(ids::LOAD_KEYBOARD, KeyInput::Backspace) {
        keys.push((String::from("backspace"), r));
    }
    for (name, r) in &keys {
        assert!(
            mm(r.w) >= tokens::KEY_MIN_TAP_MM && mm(r.h) >= tokens::KEY_MIN_TAP_MM,
            "the {name} key is {:.1} x {:.1} mm on the smallest panel",
            mm(r.w),
            mm(r.h)
        );
    }
    // A finger that lands in the corner of a key still types that key,
    // so the whole rectangle is the target and not a smaller face
    // inside it.
    let z = h
        .app
        .key_rect(ids::LOAD_KEYBOARD, KeyInput::Char('z'))
        .expect("the z key");
    assert!(h.app.candidates().is_empty(), "nothing typed yet");
    h.tap_at(z.x as u16, (z.bottom() - 1) as u16);
    let candidates = h.app.candidates();
    assert!(
        !candidates.is_empty() && candidates.iter().all(|w| w.starts_with('z')),
        "a tap in the corner of the z key typed {candidates:?}"
    );
}

/// §4.1: on `wide` the sidebar is the six areas of the launcher, in
/// that order, and nothing else.
#[test]
fn the_wide_sidebar_lists_the_six_areas() {
    let h = loaded(DESKTOP);
    let mut previous = h
        .app
        .rect_of(ids::at(ids::HOME_TILE_BASE, 0))
        .expect("the Wallets entry")
        .y;
    for i in 1..6 {
        let r = h
            .app
            .rect_of(ids::at(ids::HOME_TILE_BASE, i))
            .unwrap_or_else(|| panic!("sidebar row {i}"));
        assert!(r.y >= previous, "row {i} is out of order");
        previous = r.y;
    }
    assert!(
        h.app.rect_of(ids::at(ids::HOME_TILE_BASE, 6)).is_none(),
        "six entries and no more"
    );
    let s = &opensigner_core::strings::EN;
    let texts = h.app.texts();
    for label in [
        s.home_wallets,
        s.home_keys,
        s.home_scan,
        s.home_tools,
        s.home_learn,
        s.home_settings,
    ] {
        assert!(texts.iter().any(|t| t == label), "{label} is an area");
    }
}

/// A wallet's screens are the Wallets entry and a key's are the Keys
/// entry: that is where each one's row is.
#[test]
fn the_wide_sidebar_selects_wallets_on_a_wallet_and_keys_on_a_key() {
    let mut h = loaded(DESKTOP);
    h.open_single_sig(0);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    assert_eq!(h.app.selected_area(), Some(String::from("Wallets")));
    h.open_key(0);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    assert_eq!(h.app.selected_area(), Some(String::from("Keys")));
}

/// §4.1: on `wide` the sidebar carries the areas, and a flow that owns
/// the screen dims it. A dimmed entry is not a control: tapping where
/// it is drawn leaves the flow where it was.
#[test]
fn a_dimmed_sidebar_entry_is_not_tappable() {
    let display = DESKTOP;
    let mut h = loaded(display);
    assert_eq!(h.app.class(), osk_ui::SizeClass::Wide);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    // Where the Tools entry is drawn on a live sidebar. The sidebar
    // is on every `Wide` screen, so the point does not move.
    let tools = h
        .app
        .rect_of(ids::at(ids::HOME_TILE_BASE, 3))
        .expect("the Tools entry");
    h.open_load();
    assert_eq!(h.app.screen(), ScreenKind::Load);
    assert!(
        h.app.rect_of(ids::at(ids::HOME_TILE_BASE, 3)).is_none(),
        "the sidebar keeps its targets inside a wizard"
    );
    let c = tools.center();
    h.tap_at(c.x as u16, c.y as u16);
    assert_eq!(
        h.app.screen(),
        ScreenKind::Load,
        "the dimmed sidebar moved the app out of the wizard"
    );
}

/// PLANNING §15.34: what the shell says about the bottom edge is what
/// the core keeps free there. A phone whose shell reports the strip its
/// gesture bar takes draws nothing in it — the bottom action, the PIN
/// pad's last row and the word keyboard's bottom faces all stand above
/// it — while the same phone whose shell reports none puts each of them
/// at least that much lower, down at the screen's own margin.
#[test]
fn a_reported_bottom_inset_is_left_empty_and_no_inset_is_used() {
    const INSET: u16 = 63;
    let reported = DisplayInfo {
        inset_bottom: INSET,
        ..PHONE
    };
    let screens: [(&str, OpenScreen); 3] = [
        ("the bottom action", passphrase_step),
        ("the PIN pad", set_pin_step),
        ("the word keyboard", words_step),
    ];
    let action = |d| {
        passphrase_step(d)
            .app
            .rect_of(ids::LOAD_PASS_CONTINUE)
            .expect("the bottom action")
            .bottom()
    };
    assert_eq!(
        action(PHONE) - action(reported),
        i32::from(INSET),
        "the action moved by something other than the strip the shell reported"
    );
    let strip_top = i32::from(PHONE.height) - i32::from(INSET);
    for (what, open) in screens {
        let lifted = lowest_drawn_row(&mut open(reported));
        let flat = lowest_drawn_row(&mut open(PHONE));
        assert!(
            lifted < strip_top,
            "{what} is drawn in the strip the shell reported: row {lifted} of {}",
            PHONE.height
        );
        assert!(
            flat - lifted >= i32::from(INSET),
            "{what} does not use the pixels of the strip when no strip was reported: \
             row {flat} against row {lifted}"
        );
    }
}

/// A session opened on `display` and walked to one screen.
type OpenScreen = fn(DisplayInfo) -> Harness;

/// The passphrase step of Add key: an app bar, a choice and the bottom
/// action.
fn passphrase_step(display: DisplayInfo) -> Harness {
    let mut h = Harness::new(display);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h
}

/// The step after it: the session PIN, on the pad.
fn set_pin_step(display: DisplayInfo) -> Harness {
    let mut h = passphrase_step(display);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.tap(ids::LOAD_HOLD);
    h
}

/// The first word of Add key, on the word keyboard.
fn words_step(display: DisplayInfo) -> Harness {
    let mut h = Harness::new(display);
    h.open_load();
    h.load_choices(0);
    h
}

/// The lowest row of the drawn frame with anything on it: where the
/// bottom of what a person sees on this screen is. The pixel in the
/// bottom-left corner is the screen's own background, since every screen
/// keeps its margin there.
fn lowest_drawn_row(h: &mut Harness) -> i32 {
    let frame = h.app.frame();
    let (w, rows) = (usize::from(frame.width), usize::from(frame.height));
    let corner = &frame.rgba[(rows - 1) * w * 4..(rows - 1) * w * 4 + 4];
    for y in (0..rows).rev() {
        let row = &frame.rgba[y * w * 4..(y + 1) * w * 4];
        if row.chunks_exact(4).any(|p| p != corner) {
            return y as i32;
        }
    }
    0
}

/// Where two panels differ: the rectangle enclosing every pixel that is
/// not the same in both, `None` when they are the same panel.
fn difference(a: &[u8], b: &[u8], width: i32) -> Option<osk_ui::Rect> {
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for (i, (p, q)) in a.chunks_exact(4).zip(b.chunks_exact(4)).enumerate() {
        if p != q {
            let (x, y) = (i as i32 % width, i as i32 / width);
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    (x1 >= x0).then(|| osk_ui::Rect::new(x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

/// Whether the screen on the panel has more content than fits.
fn scrolls(h: &Harness) -> bool {
    match (h.app.rect_of(ids::SCROLL), h.app.scroll_info(ids::SCROLL)) {
        (Some(rect), Some((content_h, _))) => content_h > rect.h,
        _ => false,
    }
}

/// The panel a scroll left behind and the panel a full draw of the same
/// state gives, for comparing the two.
fn frames_after(h: &mut Harness) -> (Vec<u8>, Vec<u8>) {
    let (_, offset) = h.app.scroll_info(ids::SCROLL).expect("a scroll region");
    assert!(offset > 0, "nothing scrolled");
    let scrolled = h.app.frame().rgba.to_vec();
    h.app.redraw();
    h.drain();
    (scrolled, h.app.frame().rgba.to_vec())
}

/// Drags the list on screen upwards, a finger down and three moves. The
/// frame is read after every event, as a shell blitting each `Draw`
/// reads it, so the panel holds the frame each move starts from.
fn drag(h: &mut Harness, display: DisplayInfo) -> (Vec<u8>, Vec<u8>) {
    let rect = h.app.rect_of(ids::SCROLL).expect("a scroll region");
    let center = rect.center();
    let (x, y) = (center.x as u16, center.y as u16);
    let step = px(display, 20.0) as u16;
    h.send(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    h.blit();
    for i in 1..=3 {
        h.send(Event::Touch {
            x,
            y: y - i * step,
            phase: TouchPhase::Move,
        });
        h.blit();
    }
    frames_after(h)
}

/// Opens the first Learn page with more on it than the panel shows.
fn open_long_learn_page(h: &mut Harness) {
    h.tap(ids::at(ids::HOME_TILE_BASE, 4));
    for i in 0..osk_learn::PAGES {
        h.tap(ids::at(ids::LEARN_ROW_BASE, i));
        if scrolls(h) {
            return;
        }
        h.tap(ids::BACK);
    }
    panic!("no Learn page is longer than the panel");
}

/// A drag on the Settings list shows what a full draw of the same state
/// shows: the pixels that moved are the pixels that would have been
/// drawn.
#[test]
fn a_dragged_settings_list_shows_what_a_full_draw_shows() {
    for display in [PANEL, TINY, PHONE] {
        let mut h = loaded(display);
        h.open_settings();
        assert!(
            scrolls(&h),
            "Settings fits on {}x{}",
            display.width,
            display.height
        );
        let (scrolled, drawn) = drag(&mut h, display);
        assert_eq!(
            difference(&scrolled, &drawn, i32::from(display.width)),
            None,
            "the dragged panel differs from a full draw on {}x{}",
            display.width,
            display.height
        );
    }
}

/// The same on a page of prose.
#[test]
fn a_dragged_learn_page_shows_what_a_full_draw_shows() {
    for display in [PANEL, TINY, PHONE] {
        let mut h = Harness::new(display);
        open_long_learn_page(&mut h);
        let (scrolled, drawn) = drag(&mut h, display);
        assert_eq!(
            difference(&scrolled, &drawn, i32::from(display.width)),
            None,
            "the dragged page differs from a full draw on {}x{}",
            display.width,
            display.height
        );
    }
}

/// A wheel scroll on the desktop window, where scrolling is a wheel and
/// not a finger.
#[test]
fn a_wheeled_list_shows_what_a_full_draw_shows() {
    let mut h = loaded(DESKTOP);
    h.open_settings();
    assert!(scrolls(&h), "Settings fits in the window");
    let rect = h.app.rect_of(ids::SCROLL).expect("a scroll region");
    let center = rect.center();
    h.blit();
    for _ in 0..3 {
        h.send(Event::Scroll {
            x: center.x as u16,
            y: center.y as u16,
            dy: px(DESKTOP, 20.0) as i16,
        });
        h.blit();
    }
    let (scrolled, drawn) = frames_after(&mut h);
    assert_eq!(
        difference(&scrolled, &drawn, i32::from(DESKTOP.width)),
        None,
        "the wheeled panel differs from a full draw"
    );
    // And back up, where the strip comes into view at the top.
    h.blit();
    h.send(Event::Scroll {
        x: center.x as u16,
        y: center.y as u16,
        dy: -px(DESKTOP, 20.0) as i16,
    });
    let (scrolled, drawn) = frames_after(&mut h);
    assert_eq!(
        difference(&scrolled, &drawn, i32::from(DESKTOP.width)),
        None,
        "the panel wheeled back up differs from a full draw"
    );
}

/// A tap after a drag opens the row the reader sees: what the panel
/// shows and what a finger lands on move together.
#[test]
fn a_row_tapped_after_a_drag_is_the_row_on_the_panel() {
    let mut h = loaded(PANEL);
    h.open_settings();
    drag(&mut h, PANEL);
    let view = h.app.rect_of(ids::SCROLL).expect("a scroll region");
    let row = [
        ids::SETTINGS_NETWORK_ROW,
        ids::SETTINGS_UNIT_ROW,
        ids::SETTINGS_LOCK_AFTER_ROW,
        ids::SETTINGS_WIPE_AFTER_ROW,
    ]
    .into_iter()
    .find(|id| {
        h.app
            .rect_of(*id)
            .is_some_and(|r| r.y >= view.y && r.bottom() <= view.bottom())
    })
    .expect("a row wholly on the panel after the drag");
    let center = h.app.rect_of(row).expect("the row").center();
    h.tap_at(center.x as u16, center.y as u16);
    assert_eq!(h.app.screen(), ScreenKind::Setting);
}

/// Glyphs are filled from their outlines on first sight and kept for the
/// next frame. Leaving a screen and coming back must give the reader the
/// screen they left, pixel for pixel.
#[test]
fn a_screen_is_the_same_pixels_when_it_comes_back() {
    let mut h = loaded(PANEL);
    let first = h.app.frame().rgba.to_vec();
    h.open_settings();
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    assert_ne!(h.app.frame().rgba, &first[..], "Settings is another screen");
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.frame().rgba, &first[..]);
}

/// A 24-word backup written in a wordlist whose words are wider than
/// English's: the panel still holds its rows. An Italian list on a phone
/// and a Japanese one in a desktop window are the two that used to run
/// over the panel's edge.
#[test]
fn a_wide_wordlist_keeps_its_words_inside_the_panel() {
    use osk_bip::bip39::{Language, Mnemonic};

    const PASSPHRASE: &str = "a long enough one";
    for (display, lang) in [(PHONE, Language::Italian), (DESKTOP, Language::Japanese)] {
        let m = Mnemonic::from_entropy(lang, &[0x42; 32]).expect("a 24-word key");
        let seed = [7u8; osk_backup::oskb::SEED_LEN];
        let bytes =
            osk_backup::oskb::seal(&m, PASSPHRASE.as_bytes(), osk_backup::DEVICE_PARAMS, &seed)
                .expect("a backup");

        let mut h = Harness::new(display);
        h.go_home();
        h.tap(ids::HOME_SCAN);
        h.scan_bytes(&bytes);
        h.type_text(PASSPHRASE);
        h.key(Key::Enter);
        h.tap(ids::LOAD_CONTINUE);
        h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
        h.add_key();
        h.open_key(0);
        h.tap(ids::DETAIL_BACKUP);
        h.tap(ids::BACKUP_WORDS);
        h.tap(ids::CREATE_REVEAL);

        let panel = h.app.rect_of(ids::CREATE_REVEAL).expect("the words panel");
        let pad = px(display, tokens::SECRET_PAD);
        let rows: Vec<(String, osk_ui::geom::Rect)> = h
            .app
            .text_rects()
            .into_iter()
            .filter(|(t, _)| {
                t.split_once(". ")
                    .is_some_and(|(n, _)| n.trim().parse::<usize>().is_ok())
            })
            .collect();
        assert!(!rows.is_empty(), "{lang:?}: the page has rows");
        for (text, rect) in rows {
            assert!(
                rect.x >= panel.x + pad && rect.right() <= panel.right() - pad,
                "{lang:?} on {}x{}: the row {text:?} at {rect:?} runs past the panel {panel:?}",
                display.width,
                display.height,
            );
        }
    }
}

// ----- §4.3: nothing is drawn over the field -----

/// §4.3: an Entry screen's field and whatever the screen draws above it
/// — the fact or mode row, the typed value, the table of facts — each
/// keep their own rectangle. A panel with no room for both used to draw
/// the row over the field, which put the mode of the Chinese word lists
/// under their own text (`docs/PLANNING.md` §16.121).
fn nothing_is_drawn_over_the_field(h: &Harness, screen: &str, display: DisplayInfo) {
    let field = h
        .app
        .rect_of(osk_ui::screens::FIELD)
        .unwrap_or_else(|| panic!("a field on {screen}"));
    let Some(block) = h.app.rect_of(osk_ui::screens::ABOVE) else {
        return;
    };
    assert!(
        field.intersect(&block).is_empty(),
        "{screen} at {}x{}: the block above the field, at {block:?}, is drawn over the field, at \
         {field:?}",
        display.width,
        display.height,
    );
}

/// Every Entry screen a walk reaches on a short panel: the word list in
/// each of its languages, Load's word entry in each of them, the
/// passphrase, the path editor, the address, the codex32 string, the
/// vanity prefix, the BSMS token and description, a note, and the
/// Lightning cipher seed's passphrase.
fn no_entry_screen_is_drawn_over_its_field(display: DisplayInfo) {
    use osk_bip::bip39::Language;

    // Tools › Word list, whose "Search by" row sits above the field in
    // every language the list has.
    let mut h = Harness::new(display);
    h.open_tile(TILE_TOOLS);
    h.tap(ids::TOOLS_WORD_LIST);
    for (i, lang) in Language::ALL.iter().enumerate() {
        h.tap(ids::at(ids::WORDLIST_LANG_BASE, i));
        h.tap(ids::WORDLIST_LANG_CONTINUE);
        nothing_is_drawn_over_the_field(&h, &format!("the {lang:?} word list"), display);
        h.tap(ids::BACK);
    }

    // Load a key › the word entry, in each of the same languages.
    for (i, lang) in Language::ALL.iter().enumerate() {
        let mut h = Harness::new(display);
        h.open_load();
        h.choose(ids::LOAD_SOURCE_TYPE, ids::LOAD_SOURCE_CONTINUE);
        h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
        h.choose(ids::at(ids::LOAD_LANG_BASE, i), ids::LOAD_LANG_CONTINUE);
        nothing_is_drawn_over_the_field(&h, &format!("Load a key in {lang:?}"), display);
    }

    // Load a key › Codex32, whose typed string is the block above.
    let mut h = Harness::new(display);
    h.open_codex32();
    nothing_is_drawn_over_the_field(&h, "the codex32 string", display);

    // The passphrase the wizard asks for, on a key being loaded.
    let mut h = Harness::new(display);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_ADD_PASSPHRASE, ids::LOAD_PASS_CONTINUE);
    nothing_is_drawn_over_the_field(&h, "the passphrase", display);

    let mut h = loaded(display);

    // Tools › Explore › the path editor, whose block is the key it walks.
    h.open_tile(TILE_TOOLS);
    h.tap(ids::TOOLS_EXPLORER);
    h.tap(ids::EXPLORE_PATH);
    nothing_is_drawn_over_the_field(&h, "the path editor", display);
    h.go_home();

    // A wallet's "Check an address", whose block is the typed address.
    h.add_single_sig(0, 2);
    h.tap(ids::WALLET_CHECK);
    h.tap(ids::SCAN_TYPE);
    nothing_is_drawn_over_the_field(&h, "the address", display);
    h.go_home();

    // The vanity grinder's prefix.
    h.open_key(0);
    h.tap(ids::DETAIL_VANITY);
    h.choose(ids::at(ids::PICK_BASE, 1), ids::PICK_CONTINUE);
    h.choose(
        ids::at(ids::VANITY_SCRIPT_BASE, 2),
        ids::VANITY_SCRIPT_CONTINUE,
    );
    nothing_is_drawn_over_the_field(&h, "the vanity prefix", display);
    h.go_home();

    // The BSMS round's token, then its description.
    h.open_key(0);
    h.tap(ids::DETAIL_ACCOUNT);
    h.choose(ids::at(ids::PICK_BASE, 5), ids::PICK_CONTINUE);
    let bsms = opensigner_core::ExportFormat::ALL
        .iter()
        .position(|f| *f == opensigner_core::ExportFormat::BsmsSigner)
        .expect("the signer record is a format");
    h.tap(ids::EXPORT_FORMAT);
    h.choose(ids::at(ids::PICK_BASE, bsms), ids::PICK_CONTINUE);
    nothing_is_drawn_over_the_field(&h, "the BSMS token", display);
    h.tap(ids::BSMS_TOKEN_NONE);
    h.pad(ids::BSMS_TOKEN_KEYBOARD, KeyInput::Done);
    nothing_is_drawn_over_the_field(&h, "the BSMS description", display);
    h.go_home();

    // Tools › Notes › a new note.
    h.open_tile(TILE_TOOLS);
    h.tap(ids::TOOLS_NOTES);
    h.tap(ids::NOTES_NEW);
    nothing_is_drawn_over_the_field(&h, "a note", display);
    h.go_home();

    // Tools › Lightning: the cipher seed's words, then its passphrase.
    h.open_tile(TILE_TOOLS);
    h.tap(ids::TOOLS_LIGHTNING);
    h.choose(ids::at(ids::PICK_BASE, 0), ids::PICK_CONTINUE);
    nothing_is_drawn_over_the_field(&h, "the cipher seed's words", display);
    let words: Vec<&str> = LND_CIPHER_SEED.split_whitespace().collect();
    for w in &words[..words.len() - 1] {
        h.type_word(w);
    }
    h.type_text(words[words.len() - 1]);
    if h.app.load_step().is_some() {
        h.key(Key::Enter);
    }
    nothing_is_drawn_over_the_field(&h, "the cipher seed's passphrase", display);
}

/// Both `small` reference sizes: the 240 × 320 panel and the 480 × 640
/// one.
#[test]
fn no_entry_screen_is_drawn_over_its_field_on_the_smallest_panel() {
    no_entry_screen_is_drawn_over_its_field(TINY);
}

#[test]
fn no_entry_screen_is_drawn_over_its_field_on_the_panel() {
    no_entry_screen_is_drawn_over_its_field(PANEL);
}

/// Every string the screen draws cut off at the side of its clip, the
/// list scrolled from top to bottom so that each row is on the panel
/// once.
fn cut_while_scrolling(h: &mut Harness) -> Vec<String> {
    let mut cut = h.app.clipped_texts();
    let Some(rect) = h.app.rect_of(ids::SCROLL) else {
        return cut;
    };
    let center = rect.center();
    loop {
        let before = h.app.scroll_info(ids::SCROLL).map(|(_, o)| o);
        h.send(Event::Scroll {
            x: center.x as u16,
            y: center.y as u16,
            dy: (rect.h / 2) as i16,
        });
        for t in h.app.clipped_texts() {
            if !cut.contains(&t) {
                cut.push(t);
            }
        }
        if h.app.scroll_info(ids::SCROLL).map(|(_, o)| o) == before {
            return cut;
        }
    }
}

/// On the smallest panel every row label of the menus — a key's page,
/// Add a key, Wallets, Settings and Tools — is drawn whole: one that does
/// not fit steps down a size and then takes a second line rather than
/// running under its chevron (§16.130, §16.133).
#[test]
fn every_menu_label_is_drawn_whole_on_the_smallest_panel() {
    let mut h = loaded(TINY);
    let mut screens: Vec<(&str, Vec<String>)> = Vec::new();
    h.open_key(0);
    screens.push(("the key page", cut_while_scrolling(&mut h)));
    h.open_keys();
    h.tap(ids::KEYS_ADD);
    screens.push(("Add a key", cut_while_scrolling(&mut h)));
    h.open_wallets();
    screens.push(("Wallets", cut_while_scrolling(&mut h)));
    h.tap(ids::WALLETS_ADD);
    screens.push(("Add a wallet", cut_while_scrolling(&mut h)));
    h.open_settings();
    screens.push(("Settings", cut_while_scrolling(&mut h)));
    h.open_tile(TILE_TOOLS);
    screens.push(("Tools", cut_while_scrolling(&mut h)));
    for (screen, cut) in screens {
        assert!(cut.is_empty(), "{screen} cuts {cut:?}");
    }
}
