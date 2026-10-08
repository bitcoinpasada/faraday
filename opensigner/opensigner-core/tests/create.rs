//! The Create wizard, the word screens and the backup quiz, driven through
//! the shell contract (`docs/PLANNING.md` §4.2, §16.7): `Event`s in,
//! `Command`s and the non-secret inspection API out.

mod common;

use common::{ABANDON, Harness, PANEL, PHONE};
use opensigner_core::backup::BackupStep;
use opensigner_core::create::Step;
use opensigner_core::ids;
use opensigner_core::quiz::QuizState;
use opensigner_core::{QuizView, ScreenKind, strings};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{Fingerprint, MasterKey, Network};
use osk_entropy::{CoinFlips, DiceRolls, Strength};
use osk_shell_api::{BootState, Event, Key, SecureHardware, TouchPhase};

/// Fifty plausible rolls (no sanity flags; see `osk-entropy`'s tests).
const RANDOM_50: &str = "32461151351521144121541512665155412152342515356215";

/// The English words of a mnemonic built from `entropy`.
fn words_of(entropy: &[u8]) -> Vec<&'static str> {
    let m = Mnemonic::from_entropy(Language::English, entropy).unwrap();
    m.indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect()
}

fn fingerprint_of(entropy: &[u8]) -> Fingerprint {
    let m = Mnemonic::from_entropy(Language::English, entropy).unwrap();
    MasterKey::from_seed(&m.to_seed(b"").unwrap(), Network::Mainnet).fingerprint()
}

/// Home → Create → source `source` → 12 words → English → entry step.
fn start_create(h: &mut Harness, source: usize) {
    h.open_create();
    assert_eq!(h.app.screen(), ScreenKind::Create);
    assert_eq!(h.app.create_step(), Some(Step::Source));
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, source),
        ids::CREATE_SOURCE_CONTINUE,
    );
    assert_eq!(h.app.create_step(), Some(Step::Count));
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    assert_eq!(h.app.create_step(), Some(Step::Language));
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    skip_procedure(h);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
}

/// Passes the dice source's "Which procedure?" Choice on the procedure
/// it opens with, which is the hashed one every other test takes.
fn skip_procedure(h: &mut Harness) {
    if h.app.create_step() == Some(Step::Procedure) {
        h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    }
}

/// Fails when `word` is anywhere in what the screen draws. The quiz
/// never prints the right answer, in either mode (UX review
/// 2026-09-07, §3.3).
fn assert_no_word(h: &Harness, word: &str) {
    for text in h.app.texts() {
        assert!(
            !text.contains(word),
            "the screen names the right word {word:?} in {text:?}"
        );
    }
}

/// Answers every question correctly, reading the screen through
/// `quiz_view` and matching against `words`.
fn pass_quiz(h: &mut Harness, words: &[&str]) {
    for _ in 0..words.len() {
        let v: QuizView = h.app.quiz_view().expect("a quiz on screen");
        assert_eq!(v.state, QuizState::Asking);
        let want = words[v.word_number - 1];
        let slot = v
            .choices
            .iter()
            .position(|c| c == want)
            .unwrap_or_else(|| panic!("word {} not offered in {:?}", v.word_number, v.choices));
        h.choose(ids::at(ids::QUIZ_CHOICE_BASE, slot), ids::QUIZ_CONTINUE);
    }
}

/// Words → quiz → passphrase skip → hold → Home.
fn finish_create(h: &mut Harness, words: &[&str]) {
    assert_eq!(h.app.create_step(), Some(Step::Words));
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::QuizStart));
    h.tap(ids::QUIZ_START);
    assert_eq!(h.app.create_step(), Some(Step::Quiz));
    pass_quiz(h, words);
    assert_eq!(h.app.create_step(), Some(Step::PassphraseOffer));
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Confirm));
    h.add_key();
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

#[test]
fn keys_offers_load_and_create_when_empty() {
    let mut h = Harness::new(PANEL);
    h.open_keys();
    assert!(h.app.rect_of(ids::KEYS_LOAD).is_some());
    assert!(h.app.rect_of(ids::KEYS_CREATE).is_some());
    h.tap(ids::KEYS_CREATE);
    assert_eq!(h.app.screen(), ScreenKind::Create);
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Keys);
}

#[test]
fn fifty_dice_rolls_by_physical_keys_create_the_expected_key() {
    let mut h = Harness::new(PANEL);
    start_create(&mut h, 0);
    h.type_text(&RANDOM_50[..49]);
    assert_eq!(h.app.create_entries(), Some(49));
    // Enter before the count is reached does nothing.
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    // Digits outside 1–6 and letters are ignored.
    h.type_text("0x7");
    assert_eq!(h.app.create_entries(), Some(49));
    h.type_text(&RANDOM_50[49..]);
    assert_eq!(h.app.create_entries(), Some(50));
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    assert!(!h.app.create_warnings().unwrap().any());
    // The math screen reveals only while held.
    h.tap(ids::CREATE_MATH);
    assert_eq!(h.app.create_step(), Some(Step::Math));
    assert!(!h.app.is_held(ids::CREATE_REVEAL));
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    h.tap(ids::CREATE_CONTINUE);

    let mut dice = DiceRolls::new();
    for c in RANDOM_50.bytes() {
        dice.push(c - b'0');
    }
    let entropy = dice.entropy(Strength::Bits128).unwrap();
    let words = words_of(entropy.as_bytes());
    finish_create(&mut h, &words);
    let fps = h.app.fingerprints();
    assert_eq!(fps.len(), 1);
    assert_eq!(fps[0], fingerprint_of(entropy.as_bytes()));
    assert_eq!(h.app.backup_verified(0), Some(true));
    assert_eq!(h.app.has_mnemonic(0), Some(true));
    // Key detail shows the backup section.
    h.open_key(0);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    assert!(h.app.rect_of(ids::DETAIL_BACKUP).is_some());
    h.tap(ids::DETAIL_BACKUP);
    assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
    assert!(h.app.rect_of(ids::BACKUP_VERIFY).is_some());
    assert!(h.app.rect_of(ids::BACKUP_WORDS).is_some());
}

#[test]
fn a_run_of_rolls_draws_a_caution_but_does_not_block() {
    let mut h = Harness::new(PANEL);
    start_create(&mut h, 0);
    // Eight sixes then plausible rolls.
    h.type_text("66666666");
    h.type_text(&RANDOM_50[8..]);
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    let w = h.app.create_warnings().unwrap();
    assert!(w.long_run && !w.too_few, "{w:?}");
    // "Roll again" clears the rolls.
    h.tap(ids::CREATE_AGAIN);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    assert_eq!(h.app.create_entries(), Some(0));
    h.type_text("66666666");
    h.type_text(&RANDOM_50[8..]);
    h.key(Key::Enter);
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Words), "continue anyway");
}

#[test]
fn a_wrong_quiz_answer_is_reported_and_retried() {
    let mut h = Harness::new(PANEL);
    start_create(&mut h, 0);
    h.type_text(RANDOM_50);
    h.key(Key::Enter);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::QUIZ_START);
    assert_eq!(h.app.create_step(), Some(Step::Quiz));
    let mut dice = DiceRolls::new();
    for c in RANDOM_50.bytes() {
        dice.push(c - b'0');
    }
    let entropy = dice.entropy(Strength::Bits128).unwrap();
    let words = words_of(entropy.as_bytes());

    let v = h.app.quiz_view().unwrap();
    let right = words[v.word_number - 1];
    let right_slot = v.choices.iter().position(|c| c == right).unwrap();
    let wrong_slot = (right_slot + 1) % 4;
    h.choose(
        ids::at(ids::QUIZ_CHOICE_BASE, wrong_slot),
        ids::QUIZ_CONTINUE,
    );
    let v2 = h.app.quiz_view().unwrap();
    assert_eq!(v2.state, QuizState::Wrong);
    assert_eq!(v2.done, 0, "the question number does not advance");
    assert_no_word(&h, right);
    // The candidates are gone until retry; retry asks the same word.
    assert!(h.app.rect_of(ids::at(ids::QUIZ_CHOICE_BASE, 0)).is_none());
    h.tap(ids::QUIZ_RETRY);
    let v3 = h.app.quiz_view().unwrap();
    assert_eq!(v3.state, QuizState::Asking);
    assert_eq!(v3.word_number, v.word_number);
    assert_eq!(v3.message, None);
    let slot = v3.choices.iter().position(|c| c == right).unwrap();
    // The tap checks; Continue answers.
    h.tap(ids::at(ids::QUIZ_CHOICE_BASE, slot));
    assert_eq!(h.app.quiz_view().unwrap().done, 0, "the tap only checks");
    h.tap(ids::QUIZ_CONTINUE);
    assert_eq!(h.app.quiz_view().unwrap().done, 1);
}

/// A wrong answer sends the user to the words, and the words come back
/// to the same question (UX review 2026-09-07, §3.3).
#[test]
fn a_wrong_answer_offers_the_words_and_returns_to_the_question() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_VERIFY);
    h.tap(ids::QUIZ_START);
    let v = h.app.quiz_view().unwrap();
    let right = ABANDON[v.word_number - 1];
    let right_slot = v.choices.iter().position(|c| c == right).unwrap();
    h.choose(
        ids::at(ids::QUIZ_CHOICE_BASE, (right_slot + 1) % 4),
        ids::QUIZ_CONTINUE,
    );
    assert_eq!(h.app.quiz_view().unwrap().state, QuizState::Wrong);
    assert_no_word(&h, right);
    h.tap(ids::QUIZ_SHOW_WORDS);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Words));
    // The pager walks the pages; Done goes back to the question that
    // was missed.
    h.tap(ids::WORDS_NEXT);
    h.tap(ids::QUIZ_DONE);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Quiz));
    let after = h.app.quiz_view().unwrap();
    assert_eq!(after.state, QuizState::Asking);
    assert_eq!(after.word_number, v.word_number, "the same question");
    assert_eq!(after.done, 0);
    pass_quiz(&mut h, &ABANDON);
    assert_eq!(h.app.backup_step(), Some(BackupStep::QuizResult));
}

#[test]
fn skipping_the_quiz_leaves_the_backup_unverified() {
    let mut h = Harness::new(PANEL);
    start_create(&mut h, 0);
    h.type_text(RANDOM_50);
    h.key(Key::Enter);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::QuizStart));
    // The confirm is only reachable behind the caution.
    assert!(h.app.rect_of(ids::QUIZ_SKIP_CONFIRM).is_none());
    h.tap(ids::QUIZ_SKIP);
    assert_eq!(h.app.create_step(), Some(Step::QuizSkip));
    assert!(h.app.rect_of(ids::QUIZ_SKIP_CONFIRM).is_some());
    h.tap(ids::QUIZ_SKIP_CANCEL);
    assert_eq!(h.app.create_step(), Some(Step::QuizStart));
    assert!(h.app.rect_of(ids::QUIZ_START).is_some());
    h.tap(ids::QUIZ_SKIP);
    h.tap(ids::QUIZ_SKIP_CONFIRM);
    assert_eq!(h.app.create_step(), Some(Step::PassphraseOffer));
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.backup_verified(0), Some(false));
}

#[test]
fn cancelling_mid_way_leaves_no_keys() {
    let mut h = Harness::new(PANEL);
    start_create(&mut h, 0);
    h.type_text(&RANDOM_50[..30]);
    // Back keeps the rolls only while looking at the sanity step; from the
    // entry step it clears them.
    h.tap(ids::BACK);
    assert_eq!(h.app.create_step(), Some(Step::Procedure));
    h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    assert_eq!(h.app.create_entries(), Some(0));
    h.type_text(RANDOM_50);
    h.key(Key::Enter);
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(Step::Words));
    h.key(Key::Escape);
    assert_eq!(h.app.screen(), ScreenKind::Keys, "back to where it started");
    assert!(h.app.fingerprints().is_empty());
    assert_eq!(h.app.create_step(), None);
}

#[test]
fn coin_flips_yield_the_packed_entropy() {
    let mut h = Harness::new(PANEL);
    start_create(&mut h, 1);
    // A pattern with 65 heads and runs of five.
    let flips: String = (0..128)
        .map(|i| if i % 10 < 5 { 'h' } else { 't' })
        .collect();
    h.type_text(&flips);
    assert_eq!(h.app.create_entries(), Some(128));
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    assert!(!h.app.create_warnings().unwrap().any());
    h.tap(ids::CREATE_CONTINUE);
    let mut coins = CoinFlips::new();
    for c in flips.chars() {
        coins.push(c == 'h');
    }
    let entropy = coins.entropy(Strength::Bits128).unwrap();
    assert_eq!(entropy.as_bytes()[0], 0b1111_1000);
    let words = words_of(entropy.as_bytes());
    finish_create(&mut h, &words);
    assert_eq!(h.app.fingerprints()[0], fingerprint_of(entropy.as_bytes()));
}

#[test]
fn all_zero_hex_yields_abandon_about() {
    let mut h = Harness::new(PANEL);
    start_create(&mut h, 2);
    h.type_text("0000000000000000000000000000000");
    assert_eq!(h.app.create_entries(), Some(31));
    h.key(Key::Enter);
    assert_eq!(
        h.app.create_step(),
        Some(Step::Entropy),
        "31 digits is not enough"
    );
    h.type_text("00");
    assert_eq!(
        h.app.create_entries(),
        Some(32),
        "capped at 32 for 12 words"
    );
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    h.tap(ids::CREATE_CONTINUE);
    finish_create(&mut h, &ABANDON);
    assert_eq!(h.app.fingerprints()[0].to_hex(), *b"73c5da0a");
}

// ----- the three middle word counts (`docs/PLANNING.md` §8.1 item 9) -----

/// The strength a word count asks for.
fn strength_for(count: usize) -> Strength {
    Strength::for_words(count).expect("a BIP-39 word count")
}

/// `n` rolls that show no sanity flag, `RANDOM_50` repeated.
fn rolls(n: usize) -> String {
    RANDOM_50.chars().cycle().take(n).collect()
}

/// Sixty-two rolls make a 15-word key, and the words are the ones
/// SHA-256 of those rolls, cut to twenty bytes, spells.
#[test]
fn sixty_two_dice_rolls_make_a_fifteen_word_key() {
    let mut h = Harness::new(PANEL);
    start_create_with(&mut h, 0, 2);
    let typed = rolls(62);
    h.type_text(&typed[..61]);
    h.key(Key::Enter);
    assert_eq!(
        h.app.create_step(),
        Some(Step::Entropy),
        "sixty-one rolls is short of 15 words"
    );
    h.type_text(&typed[61..]);
    assert_eq!(h.app.create_entries(), Some(62));
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    h.tap(ids::CREATE_CONTINUE);

    let mut dice = DiceRolls::new();
    for c in typed.bytes() {
        dice.push(c - b'0');
    }
    let entropy = dice.entropy(strength_for(15)).unwrap();
    assert_eq!(entropy.as_bytes().len(), 20);
    let words = words_of(entropy.as_bytes());
    assert_eq!(words.len(), 15);
    finish_create(&mut h, &words);
    assert_eq!(h.app.fingerprints()[0], fingerprint_of(entropy.as_bytes()));
}

/// A hundred and ninety-two flips make an 18-word key.
#[test]
fn a_hundred_and_ninety_two_coin_flips_make_an_eighteen_word_key() {
    let mut h = Harness::new(PANEL);
    start_create_with(&mut h, 1, 3);
    let flips: String = (0..192)
        .map(|i| if i % 10 < 5 { 'h' } else { 't' })
        .collect();
    h.type_text(&flips[..191]);
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    h.type_text(&flips[191..]);
    assert_eq!(h.app.create_entries(), Some(192));
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    h.tap(ids::CREATE_CONTINUE);

    let mut coins = CoinFlips::new();
    for c in flips.chars() {
        coins.push(c == 'h');
    }
    let entropy = coins.entropy(strength_for(18)).unwrap();
    let words = words_of(entropy.as_bytes());
    assert_eq!(words.len(), 18);
    finish_create(&mut h, &words);
    assert_eq!(h.app.fingerprints()[0], fingerprint_of(entropy.as_bytes()));
}

/// Fifty-six hex digits make a 21-word key, and the field takes no
/// fifty-seventh.
#[test]
fn fifty_six_hex_digits_make_a_twenty_one_word_key() {
    let mut h = Harness::new(PANEL);
    start_create_with(&mut h, 2, 4);
    let digits = "0123456789abcdef0123456789abcdef0123456789abcdef01234567";
    h.type_text(&digits[..55]);
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    h.type_text(&digits[55..]);
    h.type_text("f");
    assert_eq!(
        h.app.create_entries(),
        Some(56),
        "capped at 56 for 21 words"
    );
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    h.tap(ids::CREATE_CONTINUE);

    let entropy = osk_entropy::RawHex::parse(digits).unwrap();
    assert_eq!(entropy.as_bytes().len(), 28);
    let words = words_of(entropy.as_bytes());
    assert_eq!(words.len(), 21);
    finish_create(&mut h, &words);
    assert_eq!(h.app.fingerprints()[0], fingerprint_of(entropy.as_bytes()));
}

/// "How many words?" lists all five BIP-39 counts, and each row leads to
/// a run of dice of its own length.
#[test]
fn every_word_count_row_leads_to_a_key_of_that_length() {
    for (row, count) in [(0, 12), (1, 24), (2, 15), (3, 18), (4, 21)] {
        let mut h = Harness::new(PANEL);
        h.open_create();
        h.choose(
            ids::at(ids::CREATE_SOURCE_BASE, 0),
            ids::CREATE_SOURCE_CONTINUE,
        );
        assert_eq!(h.app.create_step(), Some(Step::Count));
        let texts = h.app.texts();
        for n in ["12", "15", "18", "21", "24"] {
            assert!(
                texts.iter().any(|t| t == n),
                "{n} words is not offered: {texts:?}"
            );
        }
        h.choose(
            ids::at(ids::CREATE_COUNT_BASE, row),
            ids::CREATE_COUNT_CONTINUE,
        );
        h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
        skip_procedure(&mut h);
        let need = DiceRolls::needed(strength_for(count));
        let typed = rolls(need);
        h.type_text(&typed[..need - 1]);
        h.key(Key::Enter);
        assert_eq!(
            h.app.create_step(),
            Some(Step::Entropy),
            "{count} words takes {need} rolls"
        );
        h.type_text(&typed[need - 1..]);
        h.key(Key::Enter);
        assert_eq!(h.app.create_step(), Some(Step::Sanity));
        h.tap(ids::CREATE_CONTINUE);
        assert_eq!(h.app.create_step(), Some(Step::Words));

        let mut dice = DiceRolls::new();
        for c in typed.bytes() {
            dice.push(c - b'0');
        }
        let entropy = dice.entropy(strength_for(count)).unwrap();
        let words = words_of(entropy.as_bytes());
        assert_eq!(words.len(), count);
        finish_create(&mut h, &words);
        assert_eq!(h.app.fingerprints()[0], fingerprint_of(entropy.as_bytes()));
    }
}

#[test]
fn the_quiz_from_key_detail_passes_for_a_loaded_key() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    assert_eq!(h.app.backup_verified(0), Some(false));
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_VERIFY);
    assert_eq!(h.app.screen(), ScreenKind::Backup);
    assert_eq!(h.app.backup_step(), Some(BackupStep::QuizStart));
    h.tap(ids::QUIZ_START);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Quiz));
    assert!(!h.app.quiz_view().unwrap().helper);
    pass_quiz(&mut h, &ABANDON);
    assert_eq!(h.app.backup_step(), Some(BackupStep::QuizResult));
    assert_eq!(h.app.quiz_view().unwrap().state, QuizState::Passed);
    h.tap(ids::QUIZ_DONE);
    assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
    assert_eq!(h.app.backup_verified(0), Some(true));
}

#[test]
fn helper_mode_hides_the_correct_word_on_a_wrong_answer() {
    let mut h = Harness::new(PHONE);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_VERIFY);
    h.tap(ids::QUIZ_HELPER);
    h.tap(ids::QUIZ_START);
    let v = h.app.quiz_view().unwrap();
    assert!(v.helper);
    let helper_shown = |h: &Harness| {
        h.app
            .texts()
            .iter()
            .any(|t| t.ends_with(&strings::fill1(strings::EN.quiz_helper_title, "")))
    };
    assert!(helper_shown(&h), "the mode is named while the quiz runs");
    let right = ABANDON[v.word_number - 1];
    let right_slot = v.choices.iter().position(|c| c == right).unwrap();
    let wrong_slot = (right_slot + 2) % 4;
    h.choose(
        ids::at(ids::QUIZ_CHOICE_BASE, wrong_slot),
        ids::QUIZ_CONTINUE,
    );
    let v2 = h.app.quiz_view().unwrap();
    assert_eq!(v2.state, QuizState::Wrong);
    assert_no_word(&h, right);
    assert!(helper_shown(&h), "and on the wrong-answer screen too");
    h.tap(ids::QUIZ_RETRY);
    pass_quiz(&mut h, &ABANDON);
    assert_eq!(h.app.backup_step(), Some(BackupStep::QuizResult));
    h.tap(ids::QUIZ_DONE);
    assert_eq!(h.app.backup_verified(0), Some(true));
}

#[test]
fn word_screens_reveal_only_while_held() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_WORDS);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Words));
    assert!(!h.app.is_held(ids::CREATE_REVEAL));
    let (x, y) = h.center(ids::CREATE_REVEAL);
    h.send(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    h.tick(100);
    assert!(h.app.is_held(ids::CREATE_REVEAL));
    // Still held after the hold duration completes.
    h.tick(2000);
    assert!(h.app.is_held(ids::CREATE_REVEAL));
    h.send(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    assert!(!h.app.is_held(ids::CREATE_REVEAL));
    h.tap(ids::QUIZ_DONE);
    assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
}

/// §4.10's mask policy: "whatever the entropy becomes is a secret ...
/// they mask together". The word and its place in the wordlist are one
/// row, bullets of their own widths while the panel is masked and both
/// values while it is held.
#[test]
fn the_words_panel_masks_the_wordlist_numbers_with_the_words() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_WORDS);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Words));
    let row_of = |h: &Harness, prefix: &str| -> String {
        h.app
            .texts()
            .into_iter()
            .find(|t| t.starts_with(prefix))
            .unwrap_or_else(|| panic!("no row starting {prefix:?}"))
    };
    // The row's position is always there; the word is bullets until the
    // panel is held.
    assert_eq!(row_of(&h, " 1."), " 1. ••••••••");
    let point = h.press(ids::CREATE_REVEAL);
    assert_eq!(row_of(&h, " 1."), " 1. abandon ");
    h.release(point);
    assert_eq!(row_of(&h, " 1."), " 1. ••••••••", "masked again on release");
    // "Numbers" adds each word's place in the wordlist, masked with the
    // word: "abandon" is number 1, and four bullets stand for it.
    h.tap(ids::WORDS_NUMBERS);
    let masked = row_of(&h, " 1.");
    assert_eq!(masked, " 1. •••••••• ••••", "both columns are bullets");
    let point = h.press(ids::CREATE_REVEAL);
    let held = row_of(&h, " 1.");
    assert_eq!(held, " 1. abandon     1", "both columns carry a value");
    assert_eq!(held.chars().count(), masked.chars().count());
    h.release(point);
    assert_eq!(row_of(&h, " 1."), masked, "masked again on release");
}

/// The app bar's eye shows a transcription panel without a finger on
/// it, for 30 s, and never a moment longer or a screen wider (UX review
/// 2026-09-07, §2.7).
#[test]
fn the_eye_reveals_for_thirty_seconds_and_ends_on_leaving() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_WORDS);
    let shows_words = |h: &Harness| h.app.texts().iter().any(|t| t.contains("abandon"));
    assert!(!shows_words(&h), "masked on arrival");
    assert_eq!(h.app.reveal_left_s(), None);
    h.tap(ids::SECRET_EYE);
    assert_eq!(h.app.reveal_left_s(), Some(30));
    assert!(shows_words(&h), "the eye needs no finger");
    // The countdown runs down and the page turn keeps it.
    h.tick(10_000);
    assert_eq!(h.app.reveal_left_s(), Some(20));
    h.tap(ids::WORDS_NEXT);
    assert!(shows_words(&h), "page two is still open");
    h.tick(opensigner_core::REVEAL_MS);
    assert_eq!(h.app.reveal_left_s(), None, "it expires on its own");
    assert!(!shows_words(&h));
    // A second look ends when the screen does, and does not come back.
    h.tap(ids::SECRET_EYE);
    assert!(shows_words(&h));
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
    assert_eq!(h.app.reveal_left_s(), None);
    h.tap(ids::BACKUP_WORDS);
    assert!(!shows_words(&h), "never persisted across re-entry");
}

/// The four reference sizes (UX.md §2).
const SIZES: [osk_shell_api::DisplayInfo; 4] = [
    osk_shell_api::DisplayInfo {
        width: 240,
        height: 320,
        dpi: 143,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 1,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    },
    PANEL,
    PHONE,
    osk_shell_api::DisplayInfo {
        width: 960,
        height: 640,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    },
];

/// §4.10: "The geometry is fixed per class and per word count,
/// identical masked and revealed." The Words panel keeps one rectangle
/// masked, revealed, with the numbers on and off, on the Create wizard
/// and in the Backup flow, so nothing moves under the finger that
/// reveals it.
#[test]
fn the_words_panel_is_one_rectangle_in_every_state() {
    for display in SIZES {
        for created in [false, true] {
            let mut h = Harness::new(display);
            if created {
                start_create(&mut h, 0);
                h.type_text(RANDOM_50);
                h.key(Key::Enter);
                h.tap(ids::CREATE_CONTINUE);
                assert_eq!(h.app.create_step(), Some(Step::Words));
            } else {
                h.start_load(&ABANDON);
                h.finish_load(None);
                h.open_key(0);
                h.tap(ids::DETAIL_BACKUP);
                h.tap(ids::BACKUP_WORDS);
                assert_eq!(h.app.backup_step(), Some(BackupStep::Words));
            }
            let panel = ids::CREATE_REVEAL;
            let where_ = format!(
                "{} at {}x{}",
                if created { "create" } else { "backup" },
                display.width,
                display.height
            );
            let numbered = h.app.rect_of(panel).expect("the panel");
            let outside = |h: &Harness, p: osk_ui::Rect| -> Vec<osk_ui::Rect> {
                h.app
                    .placed_rects()
                    .into_iter()
                    .filter(|r| !p.contains(r.x, r.y) || *r == p)
                    .collect()
            };
            let masked = outside(&h, numbered);
            let point = h.press(panel);
            assert!(h.app.is_held(panel));
            assert_eq!(
                h.app.rect_of(panel),
                Some(numbered),
                "the panel resizes on reveal, {where_}"
            );
            assert_eq!(
                outside(&h, numbered),
                masked,
                "the screen moves on reveal, {where_}"
            );
            h.release(point);
            // The fourth state: the wordlist numbers beside the words,
            // masked and then held.
            h.tap(ids::WORDS_NUMBERS);
            assert_eq!(
                h.app.rect_of(panel),
                Some(numbered),
                "the panel resizes with the numbers on, {where_}"
            );
            assert_eq!(
                outside(&h, numbered),
                masked,
                "the screen moves with the numbers on, {where_}"
            );
            let point = h.press(panel);
            assert_eq!(
                h.app.rect_of(panel),
                Some(numbered),
                "the numbered panel resizes on reveal, {where_}"
            );
            assert_eq!(
                outside(&h, numbered),
                masked,
                "the numbered screen moves on reveal, {where_}"
            );
            h.release(point);
        }
    }
}

// ----- the four sources of `docs/PLANNING.md` §8.1 items 3, 4, 5 and 7 -----

/// Home → Create → source `source` → `count` words → English, without
/// saying which step the source then opens.
fn start_create_with(h: &mut Harness, source: usize, count: usize) {
    h.open_create();
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, source),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, count),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    skip_procedure(h);
}

/// `n` cards of a deck in index order, as the pad takes them: the rank
/// and then the suit.
fn card_keys(n: usize) -> String {
    (0..n)
        .map(|i| {
            let (rank, suit) = osk_entropy::card_parts(i as u8);
            let mut s = String::new();
            s.push(osk_entropy::RANKS[usize::from(rank)]);
            s.push(osk_entropy::SUITS[usize::from(suit)]);
            s
        })
        .collect()
}

/// A frame of `n` bytes whose values vary, which is what a camera
/// pointed at anything gives.
fn noisy_frame(seed: u8) -> Vec<u8> {
    (0..4096u32)
        .map(|i| (i.wrapping_mul(37).wrapping_add(u32::from(seed)) % 251) as u8)
        .collect()
}

fn send_frame(h: &mut Harness, luma: Vec<u8>) {
    h.send(Event::CameraFrame {
        width: 64,
        height: 64,
        luma,
        chroma: None,
    });
}

#[test]
fn cards_make_a_key_and_the_deck_tracker_refuses_a_repeat() {
    let mut h = Harness::new(PANEL);
    start_create_with(&mut h, 3, 0);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    let keys = card_keys(24);
    h.type_text(&keys);
    assert_eq!(h.app.create_entries(), Some(24));
    // The ace of spades is already out; naming it again changes nothing
    // and the screen says so.
    h.type_text("A\u{2660}");
    assert_eq!(h.app.create_entries(), Some(24));
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.create_already_drawn),
        "the caption line does not say the card is out: {:?}",
        h.app.texts()
    );
    // A card that is still in the deck lands, and twenty-five is enough.
    h.type_text(&card_keys(25)[48..]);
    assert_eq!(h.app.create_entries(), Some(25));
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));

    let mut deck = osk_entropy::CardDraws::new();
    for i in 0..25u8 {
        assert!(deck.push(i));
    }
    let entropy = deck.entropy(Strength::Bits128).unwrap();
    h.tap(ids::CREATE_CONTINUE);
    let words = words_of(entropy.as_bytes());
    finish_create(&mut h, &words);
    assert_eq!(h.app.fingerprints()[0], fingerprint_of(entropy.as_bytes()));
}

#[test]
fn twenty_four_words_from_cards_asks_for_a_second_deck() {
    let mut h = Harness::new(PHONE);
    start_create_with(&mut h, 3, 1);
    h.type_text(&card_keys(52));
    assert_eq!(h.app.create_entries(), Some(52));
    // The whole deck is out and the count is still short, so the pad
    // asks for a second one and states which deck it is on.
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    let deck_2 = strings::EN.create_card_deck.replace("{}", "2");
    assert!(
        h.app.texts().contains(&deck_2),
        "the pad does not ask for a second deck: {:?}",
        h.app.texts()
    );
    // The first deck is back in play, so the ace of spades lands again.
    h.type_text(&card_keys(6));
    assert_eq!(h.app.create_entries(), Some(58));
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
}

#[test]
fn a_covered_lens_is_refused_and_six_frames_make_a_24_word_key() {
    let mut h = Harness::new(PHONE);
    start_create_with(&mut h, 4, 1);
    assert_eq!(h.app.create_step(), Some(Step::Camera));
    assert!(h.seen.contains(&osk_shell_api::Command::CameraOn));

    // A lens with a cap on it: one luma value, and nothing to hash.
    send_frame(&mut h, vec![9u8; 4096]);
    h.tap(ids::CREATE_SHUTTER);
    assert_eq!(h.app.create_entries(), Some(0));
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.create_too_little_variation),
        "the viewfinder does not say why: {:?}",
        h.app.texts()
    );

    let mut camera = osk_entropy::CameraNoise::new();
    for seed in 0..6u8 {
        let frame = noisy_frame(seed);
        send_frame(&mut h, frame.clone());
        h.tap(ids::CREATE_SHUTTER);
        assert!(camera.push(&frame).1);
    }
    // The sixth frame is the last 256 bits needs, so the step is over.
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    assert!(h.seen.contains(&osk_shell_api::Command::CameraOff));
    let entropy = camera.entropy(Strength::Bits256).unwrap();
    h.tap(ids::CREATE_CONTINUE);
    let words = words_of(entropy.as_bytes());
    finish_create(&mut h, &words);
    assert_eq!(h.app.fingerprints()[0], fingerprint_of(entropy.as_bytes()));
}

#[test]
fn the_devices_own_randomness_makes_a_different_key_every_run() {
    /// Create → this device → skip the quiz → add the key.
    fn run(h: &mut Harness) {
        start_create_with(h, 6, 0);
        assert_eq!(h.app.create_step(), Some(Step::Device));
        assert!(
            h.app
                .texts()
                .iter()
                .any(|t| t == strings::EN.create_trust_device),
            "the result does not say what it rests on: {:?}",
            h.app.texts()
        );
        h.tap(ids::CREATE_CONTINUE);
        // The device's own result is its sanity statement: there is
        // nothing to count and no statistic that would say anything, so
        // the words come next.
        assert_eq!(h.app.create_step(), Some(Step::Words));
        h.tap(ids::CREATE_CONTINUE);
        h.tap(ids::QUIZ_SKIP);
        h.tap(ids::QUIZ_SKIP_CONFIRM);
        h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
        h.add_key();
    }
    let mut h = Harness::new(PANEL);
    run(&mut h);
    run(&mut h);
    let fps = h.app.fingerprints();
    assert_eq!(fps.len(), 2);
    assert_ne!(
        fps[0], fps[1],
        "two runs of the device source produced the same key"
    );
}

#[test]
fn a_mix_lists_a_commitment_per_source_and_hashes_them_in_order() {
    let mut h = Harness::new(PHONE);
    start_create_with(&mut h, 5, 0);
    assert_eq!(h.app.create_step(), Some(Step::MixChoose));
    // Dice and cards: the first and the third row.
    h.tap(ids::at(ids::CREATE_MIX_BASE, 0));
    h.tap(ids::at(ids::CREATE_MIX_BASE, 2));
    h.tap(ids::CREATE_MIX_CONTINUE);

    // Each chosen source runs its own step, in the order of the rows.
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    h.type_text(RANDOM_50);
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Entropy), "then the cards");
    h.type_text(&card_keys(25));
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::MixResult));

    let mut dice = DiceRolls::new();
    for c in RANDOM_50.bytes() {
        dice.push(c - b'0');
    }
    let mut deck = osk_entropy::CardDraws::new();
    for i in 0..25u8 {
        deck.push(i);
    }
    let mut mixed = osk_entropy::Mixed::new();
    assert!(mixed.push(dice.commitment()) && mixed.push(deck.commitment()));
    // The screen states both commitments, so they can be written down
    // before the words are seen.
    let texts = h.app.texts();
    for c in [dice.commitment(), deck.commitment()] {
        // A reference row shows the head and the tail of a long string,
        // in groups of four (§4.5).
        let hex: String = c.iter().map(|b| format!("{b:02x}")).collect();
        let head = format!("{} {}", &hex[..4], &hex[4..8]);
        assert!(
            texts.iter().any(|t| t.contains(&head)),
            "commitment {hex} is not on the screen: {texts:?}"
        );
    }
    let entropy = mixed.entropy(Strength::Bits128).unwrap();
    h.tap(ids::CREATE_CONTINUE);
    let words = words_of(entropy.as_bytes());
    finish_create(&mut h, &words);
    assert_eq!(h.app.fingerprints()[0], fingerprint_of(entropy.as_bytes()));
}

// ----- the other published dice procedures (`docs/PLANNING.md` §16.115)

/// The rolls of a twelve-word key under direct selection: five faces
/// of 1–4 and a sixth roll read as the coin for each word. The first 66
/// are EntropyLab's BitBox diceware transcript; the last six name
/// "tooth" (`tools/reference/dice/README.md`).
const BITBOX_72: &str = "123411234122341233412344123415234126341231412342123413234124341235432141";

/// The words of that key: the eleven EntropyLab's `hodlBitBoxRolls`
/// names, and "tooth" with its low four bits replaced by the checksum,
/// as SeedSigner's `calculate_checksum` completes it.
const BITBOX_WORDS: [&str; 12] = [
    "brand", "hobby", "ranch", "shoulder", "brass", "hockey", "ranch", "short", "brand", "hockey",
    "random", "torch",
];

/// The entropy EntropyLab's "Dice [1-6] / Hashed rolls" makes of
/// [`RANDOM_50`]: every 6 written as a 0, then SHA-256.
const SIX_AS_ZERO_50: [u8; 16] = [
    0x8b, 0xc1, 0x9a, 0x48, 0x8e, 0xa4, 0x1d, 0xa2, 0x54, 0xda, 0x5f, 0x35, 0xc0, 0x41, 0xba, 0x3a,
];

/// Home → Create → dice → 12 words → English → the procedure Choice.
fn to_procedure(h: &mut Harness) {
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
    assert_eq!(h.app.create_step(), Some(Step::Procedure));
}

/// "Which procedure?" states the rolls each procedure takes at the
/// length that was chosen: 50 for the two that hash, 72 for the one that
/// names words, and 99 and 144 at twenty-four words.
#[test]
fn the_procedure_choice_states_what_each_one_costs_in_rolls() {
    let s = &strings::EN;
    for (row, hashed, direct) in [(0usize, "50", "72"), (1, "99", "144")] {
        let mut h = Harness::new(PANEL);
        h.open_create();
        h.choose(
            ids::at(ids::CREATE_SOURCE_BASE, 0),
            ids::CREATE_SOURCE_CONTINUE,
        );
        h.choose(
            ids::at(ids::CREATE_COUNT_BASE, row),
            ids::CREATE_COUNT_CONTINUE,
        );
        h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
        let texts = h.app.texts();
        for want in [
            strings::fill1(s.dice_procedure_rolls, hashed),
            strings::fill1(s.dice_procedure_rolls, direct),
        ] {
            assert!(
                texts.contains(&want),
                "the procedure step never says {want:?}: {texts:?}"
            );
        }
    }
}

/// The hashed procedure and the one that writes every six as a zero take
/// the same fifty rolls to two different keys, each the key its own
/// source computes.
#[test]
fn the_two_hashing_procedures_take_the_same_rolls_to_their_own_keys() {
    for (row, entropy) in [
        (0usize, {
            let mut d = DiceRolls::new();
            for c in RANDOM_50.bytes() {
                d.push(c - b'0');
            }
            d.entropy(Strength::Bits128).unwrap().as_bytes().to_vec()
        }),
        (1, SIX_AS_ZERO_50.to_vec()),
    ] {
        let mut h = Harness::new(PANEL);
        to_procedure(&mut h);
        h.choose(
            ids::at(ids::CREATE_PROCEDURE_BASE, row),
            ids::CREATE_PROCEDURE_CONTINUE,
        );
        assert_eq!(h.app.create_step(), Some(Step::Entropy));
        h.type_text(RANDOM_50);
        h.key(Key::Enter);
        assert_eq!(h.app.create_step(), Some(Step::Sanity));
        h.tap(ids::CREATE_CONTINUE);
        let words = words_of(&entropy);
        finish_create(&mut h, &words);
        assert_eq!(h.app.fingerprints(), vec![fingerprint_of(&entropy)]);
    }
}

/// Direct selection: the rolls name every word as they go, the last
/// included, and a 5 or a 6 in a word's first five places is rolled
/// again and says so. The checksum takes the last word's low bits.
#[test]
fn the_dice_name_every_word_and_the_checksum_completes_the_last() {
    let mut h = Harness::new(PANEL);
    to_procedure(&mut h);
    h.choose(
        ids::at(ids::CREATE_PROCEDURE_BASE, 2),
        ids::CREATE_PROCEDURE_CONTINUE,
    );
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    // A 5 and a 6 in a word's first five places are not kept, and the
    // caption line under the pad says so.
    h.type_text("56");
    assert_eq!(h.app.create_entries(), Some(0));
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.create_rerolled),
        "the pad never says the face was rolled again: {texts:?}"
    );
    // Eleven words' rolls do not finish a twelve-word key.
    h.type_text(&BITBOX_72[..66]);
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Entropy));
    h.type_text(&BITBOX_72[66..]);
    assert_eq!(h.app.create_entries(), Some(72));
    h.key(Key::Enter);
    assert_eq!(h.app.create_step(), Some(Step::Sanity));
    h.tap(ids::CREATE_CONTINUE);
    finish_create(&mut h, &BITBOX_WORDS);
    let expected = Mnemonic::parse(Language::English, &BITBOX_WORDS.join(" ")).unwrap();
    assert_eq!(
        h.app.fingerprints(),
        vec![MasterKey::from_seed(&expected.to_seed(b"").unwrap(), Network::Mainnet).fingerprint()]
    );
}
