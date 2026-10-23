//! New key, as a person making one sees it: the same rolls or flips give
//! the key other signers make from them, too few entries make nothing, a
//! caution does not stop anyone, rolls that name words roll the last word
//! too and the checksum replaces its low bits, this device's generator is asked for fresh
//! bytes and the key is those bytes alone, a key made for a Create slot
//! fills it, and leaving the screen forgets everything. The randomness
//! check is on the entry card, Lock in adds the key, with a passphrase
//! for a Create slot, and a Create slot's flow ends there
//! (`docs/history/NEW-WALLET.md` §3). Add a key is the same flow opening on
//! Start, where words already held are typed or scanned and loaded as
//! Add a key loads them (§15).

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::hashes::{Hash, sha256};
use faraday_core::keygen::{Group, Source, Way};
use faraday_core::{Action, Faraday, Screen};
use faraday_entropy::DiceProcedure;
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

/// From the Key card of a key on its own: Lock in, Continue to Words,
/// the quiz passed, and Done.
fn finish(app: &mut Faraday) {
    app.press(Action::KLock);
    app.press(Action::KNext);
    pass_quiz(app);
    app.press(Action::KAdd);
}

/// From the Words card: the quiz, every word answered rightly.
fn pass_quiz(app: &mut Faraday) {
    use faraday_core::quiz::QuizState;
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
/// (`docs/history/NEW-WALLET.md` §2.1: New key opens on Length, not the entries).
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
    let c = faraday_core::testkit::compute();
    let phrase: Vec<&str> = words
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    let mut probe = faraday_core::wallet::Session::default();
    probe
        .keys
        .add_words_with(&c, &phrase.join(" "), "", "", None)
        .unwrap()
        .0
}

fn added(app: &Faraday) -> Vec<[u8; 4]> {
    app.session.keys.all().map(|k| k.fingerprint.0).collect()
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
    finish(&mut app);

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
    finish(&mut app);

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
        finish(app);
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
fn every_length_on_offer_can_be_chosen() {
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
    finish(&mut app);
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

/// This device's randomness is named as the shell names it, as it
/// reads it: Windows' `ProcessPrng`, the stick's `/dev/random`; a shell
/// that names none leaves "operating system" alone, naming no call.
#[test]
fn this_device_names_the_source_the_shell_reads() {
    let device = |app: &mut Faraday| {
        app.press(Action::KWords(12));
        app.press(Action::KWay(Way::Device.index()));
        app.press(Action::KNext);
        texts(app)
    };

    let mut app = faraday_core::testkit::started();
    app.event(Event::EntropySource("ProcessPrng"));
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KNext);
    let shown = device(&mut app);
    assert!(shown.contains("operating system · ProcessPrng"), "{shown}");
    assert!(!shown.contains("getrandom(2)"), "{shown}");

    // Named again while the flow is open, the new name shows.
    app.event(Event::EntropySource("/dev/random"));
    let shown = texts(&mut app);
    assert!(shown.contains("operating system · /dev/random"), "{shown}");

    let shown = device(&mut opened());
    assert!(shown.contains("operating system"), "{shown}");
    assert!(!shown.contains("operating system ·"), "{shown}");
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
    finish(&mut app);
    app
}

/// New key in the demo on mainnet (F20): the key is made from the
/// generator's answer as ever, and the session adds the next public seed
/// not loaded in its place; without the demo, as on the device, the key
/// made is the key added.
#[test]
fn the_demo_adds_the_next_sample_of_its_length_not_loaded() {
    let c = faraday_core::testkit::compute();
    let words = faraday_core::testkit::test_words;
    let handing = |answer: [u8; 32], loaded: &[&str]| {
        let mut app = faraday_core::testkit::started();
        faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
        app.session.set_demo(faraday_core::testkit::demo_samples());
        for w in loaded {
            app.session.keys.add_words(&c, &words(w), "", None).unwrap();
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
        finish(&mut app);
        added(&app)
    };
    let seed = |w: &str| fingerprint(&Mnemonic::parse(&c, Language::English, &words(w)).unwrap());
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
    // The definition: SHA-256 of the 32 bytes, truncated.
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
    app.press(Action::KLock);
    app.press(Action::KNext);
    assert_eq!(app.screen, Screen::Create);
    let fp = app.session.keys[0].fingerprint.0;
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
fn a_key_on_its_own_is_added_at_lock_in_and_done_waits_for_the_quiz_or_skipping_it_twice() {
    let mut app = opened();
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip(i % 3 != 1));
    }
    app.press(Action::KNext);
    assert!(app.session.keys.is_empty(), "nothing added before Lock in");
    app.press(Action::KLock);
    assert_eq!(app.session.keys.len(), 1, "Lock in adds the key");
    app.press(Action::KNext);
    app.press(Action::KNext);
    // On the quiz: a wrong pick does not pass it, and Done does nothing.
    let q = app.keygen.as_ref().unwrap().quiz.as_ref().unwrap();
    let wrong = (q.correct_slot() as u8 + 1) % 4;
    app.press(Action::KQuiz(wrong));
    app.press(Action::KAdd);
    assert!(app.keygen.is_some(), "still on the quiz");
    // Skipping asks first.
    app.press(Action::KSkip);
    app.press(Action::KAdd);
    assert!(app.keygen.is_some());
    app.press(Action::KSkip);
    app.press(Action::KAdd);
    assert!(app.keygen.is_none(), "Done leaves the flow");
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(app.session.keys.len(), 1, "the key is added once");
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
        "New key opens on Length (`docs/history/NEW-WALLET.md` §2.1)"
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

/// A tall display, so a whole card is drawn at once.
fn tall(app: &mut Faraday) {
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
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
}

/// Make a new wallet, single key or a 2-of-3 multisig, with New key opened
/// for its first slot and carried to the entry card on coin flips.
fn slot_flow(multi: bool) -> Faraday {
    use faraday_core::create::NewKind;
    let mut app = faraday_core::testkit::started();
    tall(&mut app);
    app.press(Action::CreateWallet);
    let kind = if multi {
        NewKind::Multi
    } else {
        NewKind::NativeSegwit
    };
    let at = NewKind::ALL.iter().position(|k| *k == kind).unwrap() as u8;
    app.press(Action::CKind(at));
    app.press(Action::CNext(faraday_core::cstep::KIND));
    if multi {
        app.press(Action::CNext(faraday_core::cstep::QUORUM));
    }
    app.press(Action::KeyGen(Some(0)));
    // Start, Make a new key ticked: its Continue opens Length.
    app.press(Action::KNext);
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    app
}

/// Flips enough for twelve words.
fn flip_all(app: &mut Faraday) {
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
}

fn texts(app: &mut Faraday) -> String {
    app.drawn_texts().join(" ")
}

#[test]
fn a_create_slot_new_key_shows_no_check_words_or_quiz_card() {
    use faraday_core::keygen::kstep;
    let mut app = slot_flow(true);
    let _ = app.frame();
    assert!(
        !app.offers(Action::KNext),
        "the foot button waits for the last entry"
    );
    assert!(!texts(&mut app).contains("Randomness check"));
    flip_all(&mut app);
    let _ = app.frame();
    assert!(app.offers(Action::KNext));
    for s in [kstep::LENGTH, kstep::SOURCE, kstep::ENTER, kstep::KEY] {
        assert!(app.offers(Action::KStep(s)), "card {s} is shown");
    }
    for s in [kstep::CHECK, kstep::WORDS, kstep::QUIZ] {
        assert!(!app.offers(Action::KStep(s)), "card {s} is not shown");
    }
}

#[test]
fn with_the_last_flip_in_the_entry_card_shows_the_checks_verdict() {
    let mut app = slot_flow(true);
    flip_all(&mut app);
    let shown = texts(&mut app);
    assert!(shown.contains("Randomness check"));
    let cautions = app.keygen.as_ref().unwrap().caution_lines();
    if cautions.is_empty() {
        assert!(shown.contains("Fair: no cautions"));
    }
    for c in cautions {
        assert!(shown.contains(c), "{c}");
    }
    assert!(
        shown.contains("No need to write the words down yet"),
        "a Create slot's words come back in Back up"
    );
}

#[test]
fn roll_again_on_a_caution_clears_the_entries_and_adds_no_key() {
    use faraday_core::keygen::kstep;
    let mut app = opened_tall();
    app.press(Action::KWords(12));
    app.press(Action::KGroup(1));
    app.press(Action::KWay(Way::DiceHashed.index()));
    app.press(Action::KNext);
    for _ in 0..50 {
        app.press(Action::KRoll(6));
    }
    assert!(!app.keygen.as_ref().unwrap().caution_lines().is_empty());
    let _ = app.frame();
    assert!(
        app.offers(Action::KAgain),
        "Roll again beside the foot button"
    );
    assert!(app.offers(Action::KNext), "a caution is not a refusal");
    app.press(Action::KAgain);
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.dice.len(), 0);
    assert_eq!(k.open, Some(kstep::ENTER));
    assert!(app.session.keys.is_empty());
}

#[test]
fn lock_in_with_a_passphrase_adds_the_key_of_the_words_with_that_passphrase() {
    let c = faraday_core::testkit::compute();
    let mut app = slot_flow(true);
    flip_all(&mut app);
    app.press(Action::KNext);
    let phrase = app.keygen.as_ref().unwrap().phrase().unwrap();
    app.press(Action::KPassField(0));
    type_text(&mut app, "Ride the 7 bus");
    app.press(Action::KPassField(1));
    type_text(&mut app, "Ride the 7 bus");
    app.press(Action::KLock);
    let mut probe = faraday_core::wallet::Session::default();
    let with = probe
        .keys
        .add_words_with(&c, &phrase, "Ride the 7 bus", "", None)
        .unwrap()
        .0;
    let without = faraday_core::wallet::Session::default()
        .keys
        .add_words_with(&c, &phrase, "", "", None)
        .unwrap()
        .0;
    assert_ne!(with, without);
    assert_eq!(added(&app), vec![with]);
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.slots[0], faraday_core::create::Source::Here(with));
    let shown = texts(&mut app);
    assert!(shown.contains("Locked in"));
    assert!(shown.contains(&faraday_core::wallet::fp_text(
        faraday_bip::keys::Fingerprint(with)
    )));
    // Continue returns to Create on Keys, with the slot filled.
    app.press(Action::KNext);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(
        app.create.as_ref().unwrap().open,
        Some(faraday_core::cstep::KEYS)
    );
}

#[test]
fn unequal_passphrases_lock_nothing_in() {
    let mut app = slot_flow(true);
    flip_all(&mut app);
    app.press(Action::KNext);
    app.press(Action::KPassField(0));
    type_text(&mut app, "one");
    app.press(Action::KPassField(1));
    type_text(&mut app, "two");
    app.press(Action::KLock);
    assert!(app.session.keys.is_empty());
    assert!(texts(&mut app).contains("The two passphrases differ"));
    // Enter in a field locks in once the two agree.
    app.event(Event::Key(faraday_shell_api::Key::Backspace));
    app.event(Event::Key(faraday_shell_api::Key::Backspace));
    app.event(Event::Key(faraday_shell_api::Key::Backspace));
    type_text(&mut app, "one");
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    assert_eq!(app.session.keys.len(), 1);
}

/// The demo on a Create slot (F20, `docs/MODES.md` §7): the passphrase is
/// `faraday` once the chip on each field is pressed; Lock in adds the next
/// sample with it, saying so before and after.
#[test]
fn the_demo_locks_in_its_sample_with_faraday_when_a_passphrase_is_chosen() {
    use faraday_core::wallet::fp_text;
    let c = faraday_core::testkit::compute();
    let mut app = slot_flow(true);
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.session.set_demo(faraday_core::testkit::demo_samples());
    flip_all(&mut app);
    app.press(Action::KNext);
    let n = app.keygen.as_ref().unwrap().handout_words();
    let (word, fp) = app.session.keys.next_handout(n).unwrap();
    assert!(texts(&mut app).contains(&format!(
        "Lock in adds sample {word} ×{n} ({}) in place of these words",
        fp_text(fp)
    )));
    let _ = texts(&mut app);
    for f in 0..2 {
        let chip = Action::PassChip(faraday_core::demo::PassField::KeyGen(f));
        assert!(app.offers(chip), "each field offers faraday");
        app.press(chip);
    }
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.passphrase.as_str(), "faraday");
    assert_eq!(k.passphrase2.as_str(), "faraday");
    app.press(Action::KLock);
    let with = faraday_core::wallet::Session::default()
        .keys
        .add_words_with(
            &c,
            &faraday_core::samples::repeated(word, n),
            "faraday",
            "",
            None,
        )
        .unwrap()
        .0;
    assert_eq!(added(&app), vec![with]);
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.slots[0], faraday_core::create::Source::Here(with));
    let shown = texts(&mut app);
    assert!(shown.contains(&format!(
        "Added sample {word} ×{n} ({}) in place of the key made",
        fp_text(faraday_bip::keys::Fingerprint(with))
    )));
    assert!(!shown.contains("Not set"), "faraday went with it");
}

/// The passphrase stays optional in Demo: left empty, the sample goes in
/// as it is.
#[test]
fn the_demo_locks_in_its_sample_with_no_passphrase_unless_one_is_chosen() {
    use faraday_core::wallet::fp_text;
    let mut app = slot_flow(true);
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.session.set_demo(faraday_core::testkit::demo_samples());
    flip_all(&mut app);
    app.press(Action::KNext);
    let n = app.keygen.as_ref().unwrap().handout_words();
    let (word, fp) = app.session.keys.next_handout(n).unwrap();
    app.press(Action::KPassField(0));
    app.press(Action::KLock);
    assert_eq!(added(&app), vec![fp.0]);
    let shown = texts(&mut app);
    assert!(shown.contains(&format!(
        "Added sample {word} ×{n} ({}) in place of the key made",
        fp_text(fp)
    )));
    assert!(shown.contains("Not set"), "no passphrase went with it");
}

/// Outside Demo, on a test network too, New key adds the key it made.
#[test]
fn on_a_test_network_outside_demo_new_key_adds_the_key_made() {
    let c = faraday_core::testkit::compute();
    let mut app = faraday_core::testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(24));
    app.press(Action::KWay(Way::Device.index()));
    app.press(Action::KNext);
    app.press(Action::KNext);
    app.event(Event::Entropy(EntropyBytes::new([0x5a; 32])));
    app.press(Action::KNext);
    let made = app.keygen.as_ref().and_then(|k| k.phrase()).unwrap();
    finish(&mut app);
    let made = Mnemonic::parse(&c, Language::English, &made).unwrap();
    assert_eq!(added(&app), vec![fingerprint(&made)]);
}

#[test]
fn after_lock_in_pressing_a_count_on_length_changes_nothing() {
    use faraday_core::keygen::kstep;
    let mut app = opened_tall();
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    flip_all(&mut app);
    app.press(Action::KNext);
    app.press(Action::KLock);
    let before = added(&app);
    app.press(Action::KStep(kstep::LENGTH));
    assert_eq!(
        app.keygen.as_ref().unwrap().open,
        Some(kstep::LENGTH),
        "Length opens to be read"
    );
    let _ = app.frame();
    assert!(!app.offers(Action::KWords(24)), "no counts to press");
    app.press(Action::KWords(24));
    app.press(Action::KFlip(true));
    app.press(Action::KAgain);
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.words, 12);
    assert_eq!(k.mnemonic.as_ref().map(|m| m.indices().len()), Some(12));
    assert!(k.locked);
    assert_eq!(added(&app), before);
}

#[test]
fn a_single_key_create_opens_on_back_up_and_the_key_card_holds_the_address_back() {
    let mut app = slot_flow(false);
    flip_all(&mut app);
    app.press(Action::KNext);
    app.press(Action::KLock);
    let on_key = texts(&mut app);
    // The key is new, so no address is shown until it is backed up.
    assert!(!on_key.contains("First address"));
    assert!(on_key.contains("Back up before you receive"));
    app.press(Action::KNext);
    assert_eq!(app.screen, Screen::Create);
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.open, Some(faraday_core::cstep::BACKUP));
    assert!(c.done[faraday_core::cstep::KEYS as usize]);
    assert!(c.done[faraday_core::cstep::CHECK as usize]);
    let w = c.built.expect("the wallet is made");
    let wallet = &app.session.wallets[w];
    let k = faraday_core::testkit::compute();
    let first = faraday_core::ui::grouped(&app.session.address_shown(&k, wallet, false, 0));
    assert!(!on_key.contains(&first), "{first} shown while held");
}

#[test]
fn a_multisig_slots_key_card_shows_its_key_and_no_address() {
    let mut app = slot_flow(true);
    flip_all(&mut app);
    app.press(Action::KNext);
    app.press(Action::KLock);
    let shown = texts(&mut app);
    assert!(!shown.contains("First address"));
    assert!(!shown.contains("tb1") && !shown.contains("bc1"));
    // The key as cosigners take it: at the multisig path.
    assert!(shown.contains("/48h/"), "{shown}");
}

#[test]
fn new_key_on_its_own_asks_for_no_passphrase_and_adds_the_key_without_one() {
    let c = faraday_core::testkit::compute();
    let mut app = opened_tall();
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    flip_all(&mut app);
    app.press(Action::KNext);
    let _ = app.frame();
    assert!(!app.offers(Action::KPassField(0)));
    assert!(!texts(&mut app).contains("Passphrase again"));
    let phrase = app.keygen.as_ref().unwrap().phrase().unwrap();
    let m = Mnemonic::parse(&c, Language::English, &phrase).unwrap();
    app.press(Action::KLock);
    assert_eq!(added(&app), vec![fingerprint(&m)]);
    assert!(!app.session.keys[0].has_passphrase);
}

/// The colour drawn just inside the left end of what `action` presses,
/// at its middle.
fn fill_of(app: &mut Faraday, action: Action) -> [u8; 4] {
    let (x, y, _, h) = app.hit_box(action).expect("drawn");
    let (x, y) = (usize::from(x) + 4, usize::from(y + h / 2));
    let frame = app.frame();
    let at = (y * frame.width as usize + x) * 4;
    frame.rgba[at..at + 4].try_into().unwrap()
}

#[test]
fn a_fresh_new_key_draws_twelve_words_as_chosen() {
    let mut app = opened_tall();
    assert_eq!(
        app.keygen.as_ref().unwrap().open,
        Some(faraday_core::keygen::kstep::LENGTH)
    );
    let _ = app.frame();
    let twelve = fill_of(&mut app, Action::KWords(12));
    let fifteen = fill_of(&mut app, Action::KWords(15));
    let eighteen = fill_of(&mut app, Action::KWords(18));
    assert_ne!(twelve, fifteen, "12 words is drawn chosen before any press");
    assert_eq!(fifteen, eighteen, "the others are not");
}

#[test]
fn an_empty_key_card_passphrase_field_says_no_passphrase_until_a_character_is_typed() {
    let mut app = slot_flow(true);
    flip_all(&mut app);
    app.press(Action::KNext);
    let count = |app: &mut Faraday, s: &str| app.drawn_texts().iter().filter(|t| *t == s).count();
    assert_eq!(count(&mut app, "No passphrase"), 2, "one in each field");
    assert_eq!(count(&mut app, "None"), 0);
    assert_eq!(count(&mut app, "Not set"), 0);
    // Focused and empty, it stays.
    app.press(Action::KPassField(0));
    assert_eq!(count(&mut app, "No passphrase"), 2);
    type_text(&mut app, "a");
    assert_eq!(count(&mut app, "No passphrase"), 1, "the dots in its place");
}

// ---------------------------------------------------------------------
// Add a key is New key, opening on Start (`docs/history/NEW-WALLET.md` §15)
// ---------------------------------------------------------------------

use faraday_core::keygen::{Start, kstep};

/// Types `text` on the keyboard.
fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
}

/// Add a key pressed on the Wallets page.
fn add_a_key() -> Faraday {
    let mut app = faraday_core::testkit::started();
    tall(&mut app);
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    assert!(app.offers(Action::KeyGen(None)), "Wallets offers Add a key");
    app.press(Action::KeyGen(None));
    app
}

/// Whether Start offers each of the seven ways in, Other forms opened.
fn offers_every_start(app: &mut Faraday) {
    let _ = app.frame();
    for s in [Start::Make, Start::Words, Start::Scan] {
        assert!(app.offers(Action::KStart(s.index())), "{s:?} is offered");
    }
    let others = [Start::MakeShares, Start::Shares, Start::Codex32, Start::Xor];
    for s in others {
        assert!(!app.offers(Action::KStart(s.index())), "{s:?} waits");
    }
    app.press(Action::KOthers);
    let _ = app.frame();
    for s in others {
        assert!(app.offers(Action::KStart(s.index())), "{s:?} is offered");
    }
}

#[test]
fn add_a_key_opens_on_start_offering_every_way_in_and_a_create_slot_offers_the_same() {
    let mut app = add_a_key();
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.open, Some(kstep::START));
    assert_eq!(k.start, Start::Make, "Make a new key is ticked");
    offers_every_start(&mut app);

    use faraday_core::create::NewKind;
    let mut app = faraday_core::testkit::started();
    tall(&mut app);
    app.press(Action::CreateWallet);
    let at = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CKind(at));
    app.press(Action::CNext(faraday_core::cstep::KIND));
    app.press(Action::CNext(faraday_core::cstep::QUORUM));
    app.press(Action::KeyGen(Some(0)));
    assert_eq!(app.keygen.as_ref().unwrap().open, Some(kstep::START));
    offers_every_start(&mut app);
}

/// The Randomness card's options a flow offers, every group open.
fn ways_offered(app: &mut Faraday) -> Vec<u8> {
    for g in 1..3 {
        app.press(Action::KGroup(g));
    }
    let _ = app.frame();
    (0..faraday_core::keygen::WAYS.len() as u8)
        .filter(|&i| app.offers(Action::KWay(i)))
        .collect()
}

#[test]
fn make_a_new_key_from_add_a_key_reaches_length_then_every_source_create_offers() {
    let mut app = add_a_key();
    app.press(Action::KNext);
    assert_eq!(app.keygen.as_ref().unwrap().open, Some(kstep::LENGTH));
    app.press(Action::KWords(12));
    assert_eq!(app.keygen.as_ref().unwrap().open, Some(kstep::SOURCE));
    let ours = ways_offered(&mut app);
    let mut slot = slot_flow(true);
    slot.press(Action::KStep(kstep::SOURCE));
    let theirs = ways_offered(&mut slot);
    assert!(!ours.is_empty());
    assert_eq!(ours, theirs, "the same sources as Create's New key");
}

/// From Start: Type the words, Continue, the words typed, Continue.
fn type_the_words(app: &mut Faraday, words: &str) {
    app.press(Action::KStart(Start::Words.index()));
    app.press(Action::KNext);
    assert_eq!(app.keygen.as_ref().unwrap().open, Some(kstep::TYPED));
    let _ = app.frame();
    typed(app, words);
    let _ = app.frame();
    assert!(app.offers(Action::KNext), "the Words card's Continue");
    app.press(Action::KNext);
    assert_eq!(app.keygen.as_ref().unwrap().open, Some(kstep::KEY));
}

/// Types `pass` into both passphrase fields of the Key card.
fn passphrase_twice(app: &mut Faraday, pass: &str) {
    app.press(Action::KPassField(0));
    typed(app, pass);
    app.press(Action::KPassField(1));
    typed(app, pass);
}

#[test]
fn twelve_words_typed_with_a_passphrase_load_the_key_add_a_key_loads_not_made_here() {
    let c = faraday_core::testkit::compute();
    let words = faraday_core::testkit::test_words("bacon");
    let mut app = add_a_key();
    type_the_words(&mut app, &words);
    let _ = app.frame();
    assert!(
        app.offers(Action::KPassField(0)),
        "a key typed takes a passphrase"
    );
    passphrase_twice(&mut app, "Ride the 7 bus");
    app.press(Action::KLock);
    let fp = app
        .session
        .keys
        .last()
        .map(|k| k.fingerprint)
        .expect("loaded");
    let want = faraday_core::wallet::Session::default()
        .keys
        .add_words_with(&c, &words, "Ride the 7 bus", "", None)
        .unwrap();
    assert_eq!(fp, want);
    assert!(!app.held_keys_now(&c).contains(&fp), "not marked made here");

    // The same words and passphrase on the Entry screen give that key.
    let mut entry = faraday_core::testkit::started();
    entry.press(Action::Entry(None));
    typed(&mut entry, &words);
    entry.press(Action::EntryPassphrase);
    typed(&mut entry, "Ride the 7 bus");
    entry.press(Action::EntryAdd);
    assert_eq!(entry.session.keys.last().map(|k| k.fingerprint), Some(fp));

    // Continue leaves the flow: no Words or Quiz for words held already.
    app.press(Action::KNext);
    assert!(app.keygen.is_none());
}

#[test]
fn typed_words_whose_checksum_fails_do_not_open_key() {
    let mut app = add_a_key();
    app.press(Action::KStart(Start::Words.index()));
    app.press(Action::KNext);
    typed(&mut app, &["abandon"; 12].join(" "));
    app.press(Action::KNext);
    assert_eq!(app.keygen.as_ref().unwrap().open, Some(kstep::TYPED));
    assert!(app.entry.error.is_some(), "Add a key's own message stands");
}

#[test]
fn words_typed_for_a_create_slot_fill_the_slot_with_that_key() {
    let c = faraday_core::testkit::compute();
    use faraday_core::create::{NewKind, Source};
    let words = faraday_core::testkit::test_words("bacon");
    let mut app = faraday_core::testkit::started();
    tall(&mut app);
    app.press(Action::CreateWallet);
    let at = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CKind(at));
    app.press(Action::CNext(faraday_core::cstep::KIND));
    app.press(Action::CNext(faraday_core::cstep::QUORUM));
    app.press(Action::KeyGen(Some(0)));
    type_the_words(&mut app, &words);
    app.press(Action::KLock);
    let fp = faraday_core::wallet::Session::default()
        .keys
        .add_words_with(&c, &words, "", "", None)
        .unwrap();
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.slots[0], Source::Here(fp.0));
    app.press(Action::KNext);
    assert_eq!(app.screen, Screen::Create, "back to Create");
}

#[test]
fn a_scanned_seedqr_opens_key_with_its_words() {
    let mut app = add_a_key();
    app.press(Action::KStart(Start::Scan.index()));
    app.press(Action::KNext);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Scan));
    // "abandon" eleven times and "about".
    app.event(Event::Scanned {
        bytes: b"000000000000000000000000000000000000000000000003".to_vec(),
    });
    assert_eq!(app.sheet, None);
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.open, Some(kstep::KEY));
    let m = k.mnemonic.as_ref().expect("the words");
    let words: Vec<&str> = m
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    assert_eq!(
        words.join(" "),
        format!("{} about", ["abandon"; 11].join(" "))
    );
    app.press(Action::KLock);
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_key_wanted_by_its_fingerprint_still_opens_add_a_key() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(Some([0x5d, 0x38, 0x83, 0x76])));
    assert_eq!(app.screen, Screen::Entry);
    assert!(app.keygen.is_none());
}

#[test]
fn a_threshold_slot_offers_only_make_a_new_key_and_type_the_words() {
    use faraday_core::create::NewKind;
    let Some(at) = NewKind::ALL.iter().position(|k| k.threshold()) else {
        return;
    };
    let mut app = faraday_core::testkit::started();
    tall(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(at as u8));
    app.press(Action::CNext(faraday_core::cstep::KIND));
    app.press(Action::CNext(faraday_core::cstep::QUORUM));
    app.press(Action::KeyGen(Some(0)));
    let _ = app.frame();
    assert!(app.offers(Action::KStart(Start::Make.index())));
    assert!(app.offers(Action::KStart(Start::Words.index())));
    assert!(!app.offers(Action::KStart(Start::Scan.index())));
    assert!(!app.offers(Action::KOthers));
}
