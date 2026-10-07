//! New key, as a person making one sees it: the same rolls or flips give
//! the key other signers make from them, too few entries make nothing, a
//! caution does not stop anyone, the direct-selection procedure ends in a
//! last word the person picks, this device's generator is asked for fresh
//! bytes and the key is those bytes alone, a key made for a Create slot
//! fills it, and leaving the screen forgets everything.

use faraday_core::keygen::{SOURCE_ROWS, Source, source_index};
use faraday_core::{Action, Faraday, Screen};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::hashes::{Hash, sha256};
use osk_shell_api::{App, EntropyBytes, Event};

/// From the Words card: the quiz, every word answered rightly.
fn pass_quiz(app: &mut Faraday) {
    use opensigner_core::quiz::QuizState;
    app.press(Action::KNext);
    for _ in 0..30 {
        let Some(q) = app.keygen.as_ref().and_then(|k| k.quiz.as_ref()) else {
            return;
        };
        if q.state() == QuizState::Passed {
            return;
        }
        let slot = q.correct_slot() as u8;
        app.press(Action::KQuiz(slot));
    }
}

fn index(s: Source) -> u8 {
    source_index(s)
}

/// The app on Add a key, with New key opened from there.
fn opened() -> Faraday {
    let mut app = Faraday::new();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    app
}

/// A fingerprint of `words` with no passphrase.
fn fingerprint(words: &Mnemonic) -> [u8; 4] {
    let phrase: Vec<&str> = words
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    let mut probe = faraday_core::wallet::Session::default();
    probe
        .add_words_with(&phrase.join(" "), "", "", None)
        .unwrap()
        .0
}

fn added(app: &Faraday) -> Vec<[u8; 4]> {
    app.session
        .keys
        .iter()
        .map(|k| k.master.fingerprint().0)
        .collect()
}

#[test]
fn ninety_nine_rolls_make_the_key_coldcard_and_seedsigner_make() {
    let rolls: Vec<u8> = (0..99u32)
        .map(|i| ((i * 7 + i / 5) % 6) as u8 + 1)
        .collect();
    let mut app = opened();
    app.press(Action::KWords(24));
    app.press(Action::KSource(index(Source::Dice)));
    app.press(Action::KNext);
    for r in &rolls {
        app.press(Action::KRoll(*r));
    }
    app.press(Action::KNext);
    app.press(Action::KNext);
    pass_quiz(&mut app);
    app.press(Action::KAdd);

    // `echo -n 3246… | sha256sum`, as those signers document it.
    let ascii: String = rolls.iter().map(|r| char::from(b'0' + r)).collect();
    let entropy = sha256::Hash::hash(ascii.as_bytes()).to_byte_array();
    let expected = Mnemonic::from_entropy(Language::English, &entropy).unwrap();
    assert_eq!(added(&app), vec![fingerprint(&expected)]);
    assert_eq!(app.screen, Screen::Entry, "back where New key was opened");
    assert!(app.keygen.is_none());
}

#[test]
fn one_hundred_twenty_eight_flips_are_a_twelve_word_key_bit_for_bit() {
    let flips: Vec<bool> = (0..128).map(|i| (i * 5 + i / 3) % 2 == 0).collect();
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KSource(index(Source::Coins)));
    app.press(Action::KNext);
    for f in &flips {
        app.press(Action::KFlip(*f));
    }
    app.press(Action::KNext);
    app.press(Action::KNext);
    pass_quiz(&mut app);
    app.press(Action::KAdd);

    let mut bytes = [0u8; 16];
    for (i, f) in flips.iter().enumerate() {
        if *f {
            bytes[i / 8] |= 0x80 >> (i % 8);
        }
    }
    let expected = Mnemonic::from_entropy(Language::English, &bytes).unwrap();
    assert_eq!(expected.indices().len(), 12);
    assert_eq!(added(&app), vec![fingerprint(&expected)]);
}

#[test]
fn every_length_opensigner_offers_can_be_chosen() {
    for n in [12u8, 15, 18, 21, 24] {
        let mut app = opened();
        app.press(Action::KWords(n));
        app.press(Action::KSource(index(Source::Hex)));
        app.press(Action::KNext);
        let digits = usize::from(n) * 11 * 32 / 33 / 4;
        for i in 0..digits {
            app.press(Action::KHex((i * 7 % 16) as u8));
        }
        app.press(Action::KNext);
        let k = app.keygen.as_ref().unwrap();
        assert_eq!(
            k.mnemonic.as_ref().map(|m| m.indices().len()),
            Some(usize::from(n))
        );
    }
}

#[test]
fn too_few_rolls_make_no_words() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KSource(index(Source::Dice)));
    app.press(Action::KNext);
    for _ in 0..10 {
        app.press(Action::KRoll(3));
    }
    app.press(Action::KNext);
    let k = app.keygen.as_ref().unwrap();
    assert!(k.mnemonic.is_none());
    assert_eq!(k.note.as_deref(), Some("10 of 50"));
    app.press(Action::KAdd);
    assert!(app.session.keys.is_empty());
}

#[test]
fn a_run_of_one_face_is_cautioned_and_does_not_stop_the_key() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KSource(index(Source::Dice)));
    app.press(Action::KNext);
    for _ in 0..50 {
        app.press(Action::KRoll(6));
    }
    assert!(!app.keygen.as_ref().unwrap().caution_lines().is_empty());
    app.press(Action::KNext);
    app.press(Action::KNext);
    pass_quiz(&mut app);
    app.press(Action::KAdd);
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn rolls_that_name_words_end_in_a_last_word_the_person_picks() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KSource(index(Source::Dice)));
    // Direct word selection (BitBox), the third procedure.
    app.press(Action::KProc(2));
    app.press(Action::KNext);
    // A 5 is rolled again in a word's first five places: refused.
    app.press(Action::KRoll(5));
    assert_eq!(app.keygen.as_ref().unwrap().dice.len(), 0);
    for i in 0..66u32 {
        let face = if i % 6 == 5 {
            (i % 3) as u8 + 4
        } else {
            (i * 3 % 4) as u8 + 1
        };
        app.press(Action::KRoll(face));
    }
    app.press(Action::KNext);
    assert!(
        app.keygen.as_ref().unwrap().mnemonic.is_none(),
        "no last word yet"
    );
    let cands = app.keygen.as_ref().unwrap().last_word_candidates();
    assert_eq!(cands.len(), 128, "seven free bits at 12 words");
    app.press(Action::KLast(cands[5]));
    app.press(Action::KNext);
    let k = app.keygen.as_ref().unwrap();
    let m = k.mnemonic.as_ref().unwrap();
    assert_eq!(m.indices()[11], cands[5]);
    let named: Vec<u16> = k.dice.word_indices().collect();
    assert_eq!(&m.indices()[..11], &named[..]);
}

/// A device key from the given answer, through the whole flow.
fn device_key(answer: [u8; 32]) -> Faraday {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KSource(index(Source::Device)));
    app.press(Action::KNext);
    // Nothing is made before the generator answers.
    app.press(Action::KNext);
    assert!(app.keygen.as_ref().unwrap().mnemonic.is_none());
    app.event(Event::Entropy(EntropyBytes::new(answer)));
    app.press(Action::KNext);
    app.press(Action::KNext);
    pass_quiz(&mut app);
    app.press(Action::KAdd);
    app
}

#[test]
fn this_devices_key_is_its_generators_fresh_answer_hashed_and_nothing_else() {
    let answer = [0x5au8; 32];
    let a = device_key(answer);
    let b = device_key(answer);
    // OpenSigner's definition: SHA-256 of the 32 bytes, truncated.
    let hash = sha256::Hash::hash(&answer).to_byte_array();
    let expected = Mnemonic::from_entropy(Language::English, &hash[..16]).unwrap();
    assert_eq!(added(&a), vec![fingerprint(&expected)]);
    assert_eq!(added(&a), added(&b));
    let other = device_key([0x5bu8; 32]);
    assert_ne!(added(&a), added(&other));
}

#[test]
fn a_mix_of_dice_and_coins_is_the_hash_of_both_commitments() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KSource(index(Source::Mix)));
    let mix = faraday_core::keygen::MIX_SOURCES;
    let pos = |s: Source| mix.iter().position(|m| *m == s).unwrap() as u8;
    app.press(Action::KMix(pos(Source::Dice)));
    app.press(Action::KNext);
    let k = app.keygen.as_ref().unwrap();
    assert!(k.note.is_some(), "one source is not a mix");
    app.press(Action::KMix(pos(Source::Coins)));
    app.press(Action::KNext);
    let rolls: Vec<u8> = (0..50u32).map(|i| (i * 5 % 6) as u8 + 1).collect();
    for r in &rolls {
        app.press(Action::KRoll(*r));
    }
    app.press(Action::KNext);
    let flips: Vec<bool> = (0..128).map(|i| i % 3 == 0).collect();
    for f in &flips {
        app.press(Action::KFlip(*f));
    }
    app.press(Action::KNext);

    let ascii: String = rolls.iter().map(|r| char::from(b'0' + r)).collect();
    let dice = sha256::Hash::hash(ascii.as_bytes()).to_byte_array();
    // A flip is committed as the letter H or T.
    let letters: String = flips.iter().map(|f| if *f { 'H' } else { 'T' }).collect();
    let coins = sha256::Hash::hash(letters.as_bytes()).to_byte_array();
    let both = sha256::Hash::hash(&[dice, coins].concat()).to_byte_array();
    let expected = Mnemonic::from_entropy(Language::English, &both[..16]).unwrap();
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.mnemonic.as_ref().unwrap().indices(), expected.indices());
}

#[test]
fn a_key_made_for_a_create_slot_fills_it() {
    let mut app = Faraday::new();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(4));
    app.press(Action::CNext(0));
    app.press(Action::CNext(1));
    app.press(Action::KeyGen(Some(1)));
    app.press(Action::KWords(12));
    app.press(Action::KSource(index(Source::Coins)));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip(i % 3 != 1));
    }
    app.press(Action::KNext);
    app.press(Action::KNext);
    pass_quiz(&mut app);
    app.press(Action::KAdd);
    assert_eq!(app.screen, Screen::Create);
    let fp = app.session.keys[0].master.fingerprint().0;
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.slots[1], faraday_core::create::Source::Here(fp));
}

#[test]
fn leaving_new_key_forgets_the_entries() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KSource(index(Source::Dice)));
    app.press(Action::KNext);
    app.press(Action::KRoll(4));
    app.press(Action::Nav(Screen::Entry));
    assert!(app.keygen.is_none());
    app.press(Action::KeyGen(None));
    assert_eq!(app.keygen.as_ref().unwrap().dice.len(), 0);
    assert!(SOURCE_ROWS.contains(&Source::Device));
}

#[test]
fn a_key_is_added_only_after_the_quiz_or_skipping_it_twice() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KSource(index(Source::Coins)));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip(i % 3 != 1));
    }
    app.press(Action::KNext);
    app.press(Action::KNext);
    app.press(Action::KNext);
    // On the quiz: a wrong pick does not pass it, and Add does nothing.
    let q = app.keygen.as_ref().unwrap().quiz.as_ref().unwrap();
    let wrong = (q.correct_slot() as u8 + 1) % 4;
    app.press(Action::KQuiz(wrong));
    app.press(Action::KAdd);
    assert!(app.session.keys.is_empty());
    // Skipping asks first.
    app.press(Action::KSkip);
    app.press(Action::KAdd);
    assert!(app.session.keys.is_empty());
    app.press(Action::KSkip);
    app.press(Action::KAdd);
    assert_eq!(app.session.keys.len(), 1);
}
