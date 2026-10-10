//! New key, as a person making one sees it: the same rolls or flips give
//! the key other signers make from them, too few entries make nothing, a
//! caution does not stop anyone, rolls that name words roll the last word
//! too and the checksum replaces its low bits, this device's generator is asked for fresh
//! bytes and the key is those bytes alone, a key made for a Create slot
//! fills it, and leaving the screen forgets everything.

use faraday_core::keygen::{Group, Source, Way};
use faraday_core::{Action, Faraday, Screen};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::hashes::{Hash, sha256};
use osk_entropy::DiceProcedure;
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

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

/// The app on Add a key, with New key opened from there and carried past
/// Length and Randomness to the entries, each on its default
/// (`docs/NEW-WALLET.md` §2.1: New key opens on Length, not the entries).
fn opened() -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    let words = app.keygen.as_ref().unwrap().words as u8;
    app.press(Action::KWords(words));
    app.press(Action::KNext);
    app
}

/// [`opened`] on a display tall enough for a whole card.
fn opened_tall() -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
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
    app.press(Action::KWay(Way::DiceHashed.index()));
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
    app.press(Action::KWay(Way::Coins.index()));
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
fn dice_rolled_in_flip_mode_makes_the_key_the_same_flips_make() {
    let faces: Vec<u8> = (0..128u32)
        .map(|i| ((i * 7 + i / 5) % 6) as u8 + 1)
        .collect();

    let mut by_dice = opened();
    by_dice.press(Action::KWords(12));
    by_dice.press(Action::KWay(Way::DiceFlips.index()));
    by_dice.press(Action::KNext);
    for &f in &faces {
        by_dice.press(Action::KRoll(f));
    }

    let mut by_coin = opened();
    by_coin.press(Action::KWords(12));
    by_coin.press(Action::KWay(Way::Coins.index()));
    by_coin.press(Action::KNext);
    for &f in &faces {
        by_coin.press(Action::KFlip(f >= 4));
    }

    {
        let a = by_dice.keygen.as_ref().unwrap();
        let b = by_coin.keygen.as_ref().unwrap();
        assert!(a.coins.flips().eq(b.coins.flips()));
        assert_eq!(&a.live_words()[..], &b.live_words()[..]);
    }

    for app in [&mut by_dice, &mut by_coin] {
        app.press(Action::KNext);
        app.press(Action::KNext);
        pass_quiz(app);
        app.press(Action::KAdd);
    }
    assert_eq!(added(&by_dice), added(&by_coin));
}

#[test]
fn words_chosen_by_the_dice_are_chosen_from_the_start_and_one_option_replaces_another() {
    let mut app = opened_tall();
    app.press(Action::KWords(12));
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.way(), Some(Way::DiceWords));
    assert!(k.groups_open[0] && !k.groups_open[1] && !k.groups_open[2]);
    let _ = app.frame();
    assert!(app.offers(Action::KWay(Way::DiceFlips.index())));
    assert!(
        !app.offers(Action::KWay(Way::DiceHashed.index())),
        "second group closed"
    );
    assert!(
        !app.offers(Action::KWay(Way::Device.index())),
        "this device's group closed"
    );
    app.press(Action::KGroup(2));
    let _ = app.frame();
    assert!(app.offers(Action::KWay(Way::Device.index())));

    app.press(Action::KGroup(1));
    let _ = app.frame();
    assert!(app.offers(Action::KWay(Way::DiceSixAsZero.index())));
    app.press(Action::KWay(Way::DiceSixAsZero.index()));
    let k = app.keygen.as_ref().unwrap();
    assert!(!k.by_die);
    assert_eq!(k.procedure, DiceProcedure::SixAsZero);

    app.press(Action::KWay(Way::DiceFlips.index()));
    let k = app.keygen.as_ref().unwrap();
    assert!(k.by_die);
    assert_eq!(k.way(), Some(Way::DiceFlips));
}

#[test]
fn shares_offer_nothing_as_checkable_by_hand_and_no_words_chosen_by_rolls() {
    let mut app = opened_tall();
    app.press(Action::KForm(true));
    app.press(Action::KWords(20));
    let _ = app.frame();
    let k = app.keygen.as_ref().unwrap();
    for w in [Way::DiceFlips, Way::Coins, Way::DiceHashed, Way::Hex] {
        assert_eq!(w.group(k.slip39), Group::Computed);
        assert!(app.offers(Action::KWay(w.index())), "{w:?} on offer");
    }
    assert!(!app.offers(Action::KWay(Way::DiceWords.index())));
    app.press(Action::KWay(Way::DiceWords.index()));
    assert_ne!(app.keygen.as_ref().unwrap().way(), Some(Way::DiceWords));
}

#[test]
fn every_length_opensigner_offers_can_be_chosen() {
    for n in [12u8, 15, 18, 21, 24] {
        let mut app = opened();
        app.press(Action::KWords(n));
        app.press(Action::KWay(Way::Hex.index()));
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
    app.press(Action::KWay(Way::DiceHashed.index()));
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
    app.press(Action::KWay(Way::DiceHashed.index()));
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

/// EntropyLab's BitBox transcript for twelve words, the last included,
/// and the entropy embit's `mnemonic_to_bytes(..., ignore_checksum=True)`
/// makes of the words it names, as SeedSigner's `calculate_checksum`
/// completes a last word (`tools/reference/dice/README.md`, §16.139).
const BITBOX_12: &str = "123411234122341233412344123415234126341231412342123413234124341235432141";
const BITBOX_12_ENTROPY: [u8; 16] = [
    0x1b, 0x0d, 0x8a, 0xc6, 0x63, 0x71, 0xb2, 0xd8, 0xec, 0x66, 0x36, 0x1b, 0x0d, 0x8e, 0xc6, 0xf2,
];

#[test]
fn rolls_that_name_words_stop_at_the_last_words_kept_bits_and_the_checksum_ends_it() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::DiceWords.index()));
    app.press(Action::KNext);
    // A 5 is rolled again in a word's first five places: refused.
    app.press(Action::KRoll(5));
    assert_eq!(app.keygen.as_ref().unwrap().dice.len(), 0);
    for c in BITBOX_12.bytes() {
        app.press(Action::KRoll(c - b'0'));
    }
    // Six rolls a word, and for the last only the four that cover the
    // seven bits it keeps: the transcript's last two are not taken, and
    // the last word, checksum and all, is there at once.
    let expected = Mnemonic::from_entropy(Language::English, &BITBOX_12_ENTROPY).unwrap();
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.dice.len(), 70);
    assert_eq!(k.checksum_word(), Some(expected.indices()[11]));
    app.press(Action::KNext);
    let k = app.keygen.as_ref().unwrap();
    let m = k.mnemonic.as_ref().expect("the words are made");
    assert_eq!(m.indices(), expected.indices());
    // The words before the last are the rolls' own.
    let named: Vec<u16> = k.dice.word_indices().collect();
    assert_eq!(&m.indices()[..11], &named[..11]);
    // The last keeps the high seven of the eight bits its four rolls
    // name.
    let high = k.dice.rolls()[66..70]
        .iter()
        .fold(0u16, |n, &f| n * 4 + u16::from(f - 1));
    assert_eq!(m.indices()[11] >> 4, high >> 1);
}

#[test]
fn three_one_four_two_two_five_is_miracle() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::DiceWords.index()));
    app.press(Action::KNext);
    for f in [3, 1, 4, 2, 2, 5] {
        app.press(Action::KRoll(f));
    }
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(&k.live_words()[..], &[1131]);
    assert_eq!(Language::English.word(1131), "miracle");
}

/// A device key from the given answer, through the whole flow.
fn device_key(answer: [u8; 32]) -> Faraday {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Device.index()));
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

/// New key with handed-out keys: the key is made from the generator's
/// answer as ever, and the session adds its next handed-out key in its
/// place; without them, as on the device, the key made is the key added.
#[test]
fn a_session_that_hands_out_keys_makes_the_next_one_not_loaded() {
    let words = |w: &str| vec![w; 24].join(" ");
    let handing = |answer: [u8; 32], loaded: &[&str]| {
        let mut app = faraday_core::testkit::started();
        app.session.handout = vec![words("bacon"), words("flag"), words("gas")];
        for w in loaded {
            app.session.add_words(&words(w), "", None).unwrap();
        }
        app.press(Action::Entry(None));
        app.press(Action::KeyGen(None));
        app.press(Action::KWords(24));
        app.press(Action::KWay(Way::Device.index()));
        app.press(Action::KNext);
        app.press(Action::KNext);
        app.event(Event::Entropy(EntropyBytes::new(answer)));
        app.press(Action::KNext);
        // The words made are the generator's.
        let made = app.keygen.as_ref().and_then(|k| k.phrase()).unwrap();
        assert_ne!(made.as_str(), words("bacon"));
        app.press(Action::KNext);
        pass_quiz(&mut app);
        app.press(Action::KAdd);
        added(&app)
    };
    let seed = |w: &str| fingerprint(&Mnemonic::parse(Language::English, &words(w)).unwrap());
    assert_eq!(handing([0x5a; 32], &[]), vec![seed("bacon")]);
    assert_eq!(handing([0x5b; 32], &[]), vec![seed("bacon")]);
    assert_eq!(
        handing([0x5a; 32], &["bacon"]),
        vec![seed("bacon"), seed("flag")]
    );
    // The device hands nothing out.
    assert_ne!(added(&device_key([0x5a; 32])), vec![seed("bacon")]);
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
    app.press(Action::KWay(Way::Mix.index()));
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
    let mut app = faraday_core::testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(4));
    app.press(Action::CNext(0));
    app.press(Action::CNext(1));
    app.press(Action::KeyGen(Some(1)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
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
    app.press(Action::KWay(Way::DiceFlips.index()));
    app.press(Action::KNext);
    app.press(Action::KRoll(4));
    app.press(Action::Nav(Screen::Entry));
    assert!(app.keygen.is_none());
    app.press(Action::KeyGen(None));
    assert_eq!(app.keygen.as_ref().unwrap().dice.len(), 0);
}

#[test]
fn a_key_is_added_only_after_the_quiz_or_skipping_it_twice() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
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

#[test]
fn fresh_new_key_opens_on_length_and_two_continues_reach_the_entries() {
    use faraday_core::keygen::kstep;
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(
        k.open,
        Some(kstep::LENGTH),
        "New key opens on Length (`docs/NEW-WALLET.md` §2.1)"
    );
    let words = k.words as u8;
    let way = k.way();
    assert!(way.is_some(), "Randomness has a default");
    app.press(Action::KWords(words));
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(
        k.open,
        Some(kstep::SOURCE),
        "Length's Continue takes the count shown and opens Randomness"
    );
    app.press(Action::KNext);
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(
        k.open,
        Some(kstep::ENTER),
        "Randomness's Continue opens the entry card"
    );
}

#[test]
fn change_opens_length_keeping_what_follows() {
    use faraday_core::keygen::kstep;
    let mut app = opened();
    let k = app.keygen.as_ref().unwrap();
    let way = k.way();
    for _ in 0..7 {
        app.press(Action::KRoll(3));
    }
    let rolled = app.keygen.as_ref().unwrap().dice.len();
    assert!(rolled > 0, "the rolls were taken");
    app.press(Action::KStep(kstep::LENGTH));
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.open, Some(kstep::LENGTH), "Change opens Length");
    assert_eq!(k.words, 12, "the length is kept");
    assert_eq!(k.way(), way, "the randomness is kept");
    assert_eq!(k.dice.len(), rolled, "the rolls are kept");
}
