//! Seeing which words the randomness makes, and finding them in their
//! lists. On New key, coin flips (pressed as heads and tails, or read
//! off a die, 1 to 3 as tails and 4 to 6 as heads) name a BIP-39 word for
//! every whole 11 flips as they come in, and the last word, whose last
//! bits are the checksum, as soon as every flip is in. A die read as a
//! coin makes the key the same flips make, and rolls typed are taken as
//! they are typed, the key the same presses make. Every word links to
//! the word-list sheet, marked among its neighbours, and Back returns to
//! the card as it was; the EFF passphrase dice link to their list the
//! same way. The sheet holds the real lists, each word with its number
//! and the bits or dice that name it.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::diceware::List;
use faraday_core::keygen::{Way, kstep};
use faraday_core::vault::VaultAction as V;
use faraday_core::wordlist::{LISTS, WordList, WordListAction as WL};
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

/// A tall display, so that a whole New key card is on screen at once.
fn shown() -> Faraday {
    let mut app = Faraday::new();
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
    let _ = app.frame();
    app
}

/// New key at `words` words from coin flips, on the Flips card.
fn coins(words: u8) -> Faraday {
    let mut app = shown();
    app.press(Action::KeyGen(None));
    // Make a new key, pressed on Start, opens Length.
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    app.press(Action::KWords(words));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    assert_eq!(app.keygen().unwrap().open, Some(kstep::ENTER));
    app
}

/// Flips no one would type by hand: a fixed run from a small generator.
fn flips(n: usize) -> Vec<bool> {
    let mut x: u32 = 0x9e37_79b9;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x & 1 == 1
        })
        .collect()
}

/// The BIP-39 index each whole 11 flips spell, first flip the highest
/// bit, worked out here from the flips alone.
fn groups(flips: &[bool]) -> Vec<u16> {
    flips
        .chunks_exact(11)
        .map(|g| {
            let bits: String = g.iter().map(|&h| if h { '1' } else { '0' }).collect();
            u16::from_str_radix(&bits, 2).unwrap()
        })
        .collect()
}

/// The BIP-39 words the New key screen links to the list, by index.
fn linked(app: &mut Faraday) -> Vec<u16> {
    let _ = app.frame();
    (0..2048u16)
        .filter(|&i| app.offers(Action::WordList(WL::Open(0, Some(i)))))
        .collect()
}

fn sorted(mut v: Vec<u16>) -> Vec<u16> {
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn each_eleven_flips_show_the_word_they_spell_as_they_come_in() {
    let f = flips(128);
    let mut app = coins(12);
    for (n, &heads) in f.iter().enumerate() {
        app.press(Action::KFlip(heads));
        let want = groups(&f[..=n]);
        let k = app.keygen().unwrap();
        assert_eq!(&k.live_words()[..], &want[..want.len().min(11)]);
        for &i in &want {
            assert!(!Language::English.word(i).is_empty());
        }
    }
    // The eleven whole words are on screen, and with every flip in the
    // checksum's last word, each linked to its list.
    let mut all = groups(&f);
    all.push(last_word(&f));
    assert_eq!(linked(&mut app), sorted(all));
}

/// The last word the checksum makes of these flips, worked out here: the
/// flips packed first flip highest, spelled as BIP-39 words.
fn last_word(f: &[bool]) -> u16 {
    let mut bytes = vec![0u8; f.len() / 8];
    for (i, &h) in f.iter().enumerate() {
        if h {
            bytes[i / 8] |= 0x80 >> (i % 8);
        }
    }
    *Mnemonic::from_entropy(Language::English, &bytes)
        .unwrap()
        .indices()
        .last()
        .unwrap()
}

#[test]
fn the_last_word_shows_once_every_flip_is_in() {
    for words in [12u8, 15, 18, 21, 24] {
        let bits = usize::from(words) * 32 / 3;
        let f = flips(bits);
        let mut app = coins(words);
        let (before_last, last_flip) = f.split_at(f.len() - 1);
        for &heads in before_last {
            app.press(Action::KFlip(heads));
            let k = app.keygen().unwrap();
            assert!(k.live_words().len() < usize::from(words));
            assert_eq!(k.checksum_word(), None);
        }
        // The last flip in, the last word is the checksum's, on screen
        // before the words are made.
        app.press(Action::KFlip(last_flip[0]));
        let before = groups(&f);
        assert_eq!(before.len(), usize::from(words) - 1);
        let last = last_word(&f);
        assert_eq!(app.keygen().unwrap().checksum_word(), Some(last));
        let mut all = before.clone();
        all.push(last);
        assert_eq!(linked(&mut app), sorted(all.clone()));

        // Made, the words are those.
        app.press(Action::KNext);
        app.press(Action::KStep(kstep::ENTER));
        let k = app.keygen().unwrap();
        assert_eq!(k.mnemonic.as_ref().unwrap().indices(), &all[..]);
        assert_eq!(linked(&mut app), sorted(all));
    }
}

#[test]
fn a_die_read_as_a_coin_makes_the_key_the_same_flips_make() {
    let faces: Vec<u8> = (0..128u32)
        .map(|i| ((i * 7 + i / 5) % 6) as u8 + 1)
        .collect();
    let mut by_die = coins(12);
    by_die.press(Action::KByDie(true));
    for &f in &faces {
        by_die.press(Action::KDie(f));
    }
    let mut by_coin = coins(12);
    for &f in &faces {
        by_coin.press(Action::KFlip(f >= 4));
    }
    let a = by_die.keygen().unwrap();
    let b = by_coin.keygen().unwrap();
    assert!(a.coins.flips().eq(b.coins.flips()));
    assert_eq!(&a.live_words()[..], &b.live_words()[..]);
    for app in [&mut by_die, &mut by_coin] {
        app.press(Action::KNext);
    }
    let a = by_die.keygen().unwrap().phrase().unwrap();
    let b = by_coin.keygen().unwrap().phrase().unwrap();
    assert_eq!(*a, *b);
}

#[test]
fn die_faces_typed_on_the_keyboard_count_as_flips_too() {
    let mut app = coins(12);
    app.press(Action::KByDie(true));
    app.press(Action::KTyping(false));
    for c in ['1', '3', '4', '6', '2', '5'] {
        app.event(Event::Key(Key::Char(c)));
    }
    let k = app.keygen().unwrap();
    let got: Vec<bool> = k.coins.flips().collect();
    assert_eq!(got, [false, false, true, true, false, true]);
}

#[test]
fn dice_open_on_flip_mode_and_every_way_takes_each_entry_as_it_is_typed() {
    // Dice: Flip mode, 1 to 3 a tails and 4 to 6 a heads.
    let mut dice = shown();
    dice.press(Action::KeyGen(None));
    dice.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    dice.press(Action::KWords(12));
    dice.press(Action::KWay(Way::DiceFlips.index()));
    dice.press(Action::KNext);
    let faces = "1346251436251";
    for (n, c) in faces.char_indices() {
        dice.event(Event::Key(Key::Char(c)));
        let k = dice.keygen().unwrap();
        assert_eq!(k.coins.len(), n + 1, "taken as typed");
        assert_eq!(
            k.entered.as_str(),
            &faces[..=n],
            "the box shows what was typed"
        );
    }
    let f: Vec<bool> = faces.bytes().map(|b| b >= b'4').collect();
    assert_eq!(dice.keygen().unwrap().coins.flips().collect::<Vec<_>>(), f);
    assert_eq!(linked(&mut dice), groups(&f));

    // Coins: heads and tails.
    let mut coin = coins(12);
    let sides = "HTTHHHTHTTHT";
    for c in sides.chars() {
        coin.event(Event::Key(Key::Char(c)));
    }
    let f: Vec<bool> = sides.chars().map(|c| c == 'H').collect();
    assert_eq!(coin.keygen().unwrap().coins.flips().collect::<Vec<_>>(), f);
    assert_eq!(linked(&mut coin), groups(&f));

    // Every dice procedure is typed too.
    for way in [Way::DiceHashed, Way::DiceSixAsZero, Way::DiceWords] {
        let mut app = shown();
        app.press(Action::KeyGen(None));
        app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
        app.press(Action::KWords(12));
        app.press(Action::KWay(way.index()));
        app.press(Action::KNext);
        for c in "1234".chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        assert_eq!(app.keygen().unwrap().dice.rolls(), &[1, 2, 3, 4]);
        // Backspace takes the last one back, from the box too.
        app.event(Event::Key(Key::Backspace));
        let k = app.keygen().unwrap();
        assert_eq!(k.dice.rolls(), &[1, 2, 3]);
        assert_eq!(k.entered.as_str(), "123");
    }
}

/// Rolls typed into the box, each taken as it is typed.
fn typed(app: &mut Faraday, s: &str) {
    app.press(Action::KTyping(true));
    for c in s.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn rolls_typed_as_a_string_make_the_key_the_same_presses_make() {
    let rolls: String = (0..99u32)
        .map(|i| char::from(b'1' + ((i * 5 + i / 3) % 6) as u8))
        .collect();
    let dice = |app: &mut Faraday, way: Way| {
        app.press(Action::KeyGen(None));
        app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
        app.press(Action::KWords(24));
        app.press(Action::KWay(way.index()));
        app.press(Action::KNext);
    };
    // The hashed procedure: every face counts.
    let mut pressed = shown();
    dice(&mut pressed, Way::DiceHashed);
    for c in rolls.chars() {
        pressed.press(Action::KRoll(c as u8 - b'0'));
    }
    let mut boxed = shown();
    dice(&mut boxed, Way::DiceHashed);
    typed(&mut boxed, &rolls);
    assert_eq!(
        pressed.keygen().unwrap().dice.rolls(),
        boxed.keygen().unwrap().dice.rolls()
    );
    for app in [&mut pressed, &mut boxed] {
        app.press(Action::KNext);
    }
    assert_eq!(
        *pressed.keygen().unwrap().phrase().unwrap(),
        *boxed.keygen().unwrap().phrase().unwrap()
    );

    // Choosing words directly, where a 5 or 6 is rolled again in most
    // places: the box refuses the same faces the pad does.
    let mut pressed = shown();
    dice(&mut pressed, Way::DiceWords);
    for c in rolls.chars() {
        pressed.press(Action::KRoll(c as u8 - b'0'));
    }
    let mut boxed = shown();
    dice(&mut boxed, Way::DiceWords);
    typed(&mut boxed, &rolls);
    let (a, b) = (pressed.keygen().unwrap(), boxed.keygen().unwrap());
    assert!(a.dice.len() < rolls.len());
    assert_eq!(a.dice.rolls(), b.dice.rolls());
    assert_eq!(&a.live_words()[..], &b.live_words()[..]);
}

#[test]
fn words_chosen_directly_show_only_the_words_rolled() {
    let mut app = shown();
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::DiceWords.index()));
    app.press(Action::KNext);
    for i in 0..66u32 {
        let face = if i % 6 == 5 {
            (i % 3) as u8 + 4
        } else {
            (i * 3 % 4) as u8 + 1
        };
        app.press(Action::KRoll(face));
    }
    let k = app.keygen().unwrap();
    let named: Vec<u16> = k.dice.word_indices().collect();
    assert_eq!(named.len(), 11);
    assert_eq!(&k.live_words()[..], &named[..]);
    assert_eq!(k.checksum_word(), None);
}

#[test]
fn hashed_dice_show_no_words_as_they_come_in() {
    let mut app = shown();
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::DiceHashed.index()));
    app.press(Action::KNext);
    for i in 0..50u8 {
        app.press(Action::KRoll(i % 6 + 1));
    }
    assert!(app.keygen().unwrap().live_words().is_empty());
    assert!(linked(&mut app).is_empty());
}

#[test]
fn a_word_s_link_marks_it_in_the_list_and_back_returns_to_the_card() {
    let f = flips(70);
    let mut app = coins(12);
    for &h in &f {
        app.press(Action::KFlip(h));
    }
    let word = groups(&f)[3];
    let open = Action::WordList(WL::Open(0, Some(word)));
    let _ = app.frame();
    assert!(app.offers(open));
    app.press(open);
    assert_eq!(app.sheet, Some(Sheet::WordList));
    assert_eq!(app.screen, Screen::KeyGen);
    let st = testkit::wordlist(&app).unwrap();
    assert_eq!(st.list, WordList::Bip39);
    assert_eq!(st.mark, Some(usize::from(word)));
    // The marked word's row is on screen, among its neighbours.
    let _ = app.frame();
    assert!(app.offers(Action::WordList(WL::Mark(word))));
    if word > 0 {
        assert!(app.offers(Action::WordList(WL::Mark(word - 1))));
    }
    // Keys are the sheet's: a 1 typed here is not a flip.
    app.event(Event::Key(Key::Char('1')));
    assert_eq!(app.keygen().unwrap().coins.len(), 70);
    let _ = app.frame();
    assert!(app.offers(Action::WordList(WL::Close)));
    app.press(Action::WordList(WL::Close));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
    let k = app.keygen().unwrap();
    assert_eq!(k.open, Some(kstep::ENTER));
    assert!(k.coins.flips().eq(f.iter().copied()));
    // Escape closes it the same way.
    app.press(open);
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(app.keygen().unwrap().coins.len(), 70);
    // So does a press beside the sheet; a press on it does not.
    app.press(open);
    let _ = app.frame();
    let (x, y) = app.where_offered(Action::WordList(WL::Mark(word))).unwrap();
    tap(&mut app, x, y);
    assert_eq!(app.sheet, Some(Sheet::WordList));
    tap(&mut app, 4, 600);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
    let k = app.keygen().unwrap();
    assert_eq!(k.open, Some(kstep::ENTER));
    assert_eq!(k.coins.len(), 70);
}

/// A finger down and up at (x, y), in pixels.
fn tap(app: &mut Faraday, x: u16, y: u16) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
}

#[test]
fn a_passphrase_word_s_link_marks_it_in_its_eff_list_and_back_keeps_the_rolls() {
    let mut app = shown();
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    app.press(Action::Vault(V::DiceList(1)));
    let rolls = "6512";
    for c in rolls.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let word = List::Short1.word(&[6, 5, 1, 2]).unwrap();
    let place = List::Short1
        .words()
        .iter()
        .position(|w| *w == word)
        .unwrap() as u16;
    let open = Action::WordList(WL::Open(2, Some(place)));
    let _ = app.frame();
    assert!(app.offers(open));
    app.press(open);
    let st = testkit::wordlist(&app).unwrap();
    assert_eq!(st.list, WordList::Eff(List::Short1));
    assert_eq!(st.list.words()[st.mark.unwrap()], word);
    let _ = app.frame();
    assert!(app.offers(Action::WordList(WL::Mark(place))));
    // Keys are the sheet's: these letters find a word, not rolls.
    app.event(Event::Key(Key::Char('z')));
    app.press(Action::WordList(WL::Close));
    assert_eq!(app.screen, Screen::CreateVault);
    let (_, typed, list) = app.vaults().dice.as_ref().unwrap();
    assert_eq!(*typed.text, *rolls);
    assert_eq!(*list, List::Short1);
}

#[test]
fn the_lists_are_the_real_lists_with_their_numbers_and_codes() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Catalog));
    for (k, list) in LISTS.iter().enumerate() {
        let tile = faraday_core::catalog::TILES
            .iter()
            .position(|t| t.go == faraday_core::catalog::Go::WordList(k as u8))
            .unwrap();
        app.press(Action::Catalog(tile as u8));
        assert_eq!(app.sheet, Some(Sheet::WordList));
        assert_eq!(testkit::wordlist(&app).unwrap().list, *list);
        // The first word is on screen, and a press marks it.
        let _ = app.frame();
        assert!(app.offers(Action::WordList(WL::Mark(0))));
        app.press(Action::WordList(WL::Mark(0)));
        assert_eq!(testkit::wordlist(&app).unwrap().mark, Some(0));
        app.press(Action::WordList(WL::Close));
        assert_eq!(app.screen, Screen::Catalog);
    }

    // BIP-39: every word, its number one more than its index, and its 11
    // bits read back to that index.
    let bip = WordList::Bip39;
    assert_eq!(bip.words(), &Language::English.words()[..]);
    for i in [0usize, 1, 961, 1024, 2047] {
        assert_eq!(usize::from_str_radix(&bip.code(i), 2).unwrap(), i);
        assert_eq!(bip.code(i).len(), 11);
        assert_eq!(bip.words()[i], Language::English.word(i as u16));
    }
    assert_eq!(bip.words()[0], "abandon");
    // EFF: the dice shown for a word roll that word in faraday-bip's list.
    for l in List::ALL {
        let eff = WordList::Eff(l);
        assert_eq!(eff.words(), l.words());
        for i in [0usize, 1, 7, l.len() / 2, l.len() - 1] {
            let dice: Vec<u8> = eff.code(i).bytes().map(|b| b - b'0').collect();
            assert_eq!(dice.len(), l.dice_per_word());
            assert_eq!(l.word(&dice), Some(eff.words()[i]));
        }
    }
}

#[test]
fn typing_on_the_sheet_marks_the_first_word_that_begins_so() {
    let mut app = shown();
    app.press(Action::WordList(WL::Open(0, None)));
    for c in "zo".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let i = testkit::wordlist(&app).unwrap().mark.unwrap();
    assert_eq!(Language::English.word(i as u16), "zone");
    let _ = app.frame();
    assert!(app.offers(Action::WordList(WL::Mark(i as u16))));
}

/// The list `k` of [`LISTS`] open on the sheet, `search` typed into its
/// field: the words it finds, the one marked, and the sheet's texts.
fn searched(k: u8, search: &str) -> (Vec<u16>, Option<usize>, Vec<String>) {
    let mut app = shown();
    app.press(Action::WordList(WL::Open(k, None)));
    for c in search.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let st = testkit::wordlist(&app).unwrap();
    let (found, mark) = (st.found.clone(), st.mark);
    let texts = app.drawn_texts();
    for &i in &found {
        assert!(
            app.offers(Action::WordList(WL::Mark(i))),
            "{search}: row {i} shown"
        );
    }
    (found, mark, texts)
}

#[test]
fn the_search_field_says_what_it_finds_and_shows_what_is_typed() {
    let mut app = shown();
    app.press(Action::WordList(WL::Open(0, None)));
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Word, number or bits")
    );
    app.press(Action::WordList(WL::List(1)));
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Word, number or dice")
    );
    let (_, _, texts) = searched(0, "zoo");
    assert!(
        texts.iter().any(|t| t.starts_with("zoo")),
        "the search shown: {texts:?}"
    );
    assert!(texts.iter().any(|t| t == "1 found"), "{texts:?}");
}

#[test]
fn the_search_finds_a_word_by_its_number_its_bits_or_its_dice() {
    // BIP-39 by number: word 1024 is at index 1023; no bits begin "1024".
    let (found, mark, _) = searched(0, "1024");
    assert_eq!((found, mark), (vec![1023], Some(1023)));
    // By its 11 bits: the last word.
    let (found, mark, _) = searched(0, "11111111111");
    assert_eq!((found, mark), (vec![2047], Some(2047)));
    assert_eq!(Language::English.word(2047), "zoo");
    // EFF large by its dice: 11111 is the first word, 66666 the last.
    let large = LISTS
        .iter()
        .position(|l| *l == WordList::Eff(List::Large))
        .unwrap() as u8;
    let (found, _, _) = searched(large, "11111");
    assert_eq!(found, vec![0]);
    let (found, _, _) = searched(large, "66666");
    assert_eq!(found, vec![7775]);
    // A short list: 1111 is both word 1111 and the dice of the first word,
    // and both are shown, in the list's order.
    let short = LISTS
        .iter()
        .position(|l| *l == WordList::Eff(List::Short1))
        .unwrap() as u8;
    let (found, mark, _) = searched(short, "1111");
    assert_eq!((found, mark), (vec![0, 1110], Some(0)));
    // Letters find words that begin so, and nothing else.
    let (found, _, _) = searched(0, "zo");
    let words: Vec<&str> = found.iter().map(|&i| Language::English.word(i)).collect();
    assert_eq!(words, ["zone", "zoo"]);
}

#[test]
fn backspace_takes_the_search_back_until_every_word_shows_again() {
    let mut app = shown();
    app.press(Action::WordList(WL::Open(0, None)));
    for c in "2048".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert_eq!(testkit::wordlist(&app).unwrap().found, vec![2047]);
    for _ in 0..4 {
        app.event(Event::Key(Key::Backspace));
    }
    let st = testkit::wordlist(&app).unwrap();
    assert_eq!(st.found.len(), 2048, "every word");
    // "2", the last search, found word 2 first: it stays marked.
    assert_eq!(st.mark, Some(1));
    let _ = app.frame();
    assert!(
        app.offers(Action::WordList(WL::Mark(2))),
        "the whole list, at the word marked, among its neighbours"
    );
}
