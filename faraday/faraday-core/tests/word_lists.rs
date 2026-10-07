//! Seeing which words the randomness makes, and finding them in their
//! lists. On New key, coin flips (pressed as heads and tails, or read
//! off a die, 1 to 3 as tails and 4 to 6 as heads) name a BIP-39 word for
//! every whole 11 flips as they come in; the last word is never shown
//! before the words are made, because its last bits are the checksum. A
//! die read as a coin makes the key the same flips make, and rolls typed
//! as one string make the key the same presses make. Every word links to
//! the word-list sheet, marked among its neighbours, and Back returns to
//! the card as it was; the EFF passphrase dice link to their list the
//! same way. The sheet holds the real lists, each word with its number
//! and the bits or dice that name it.

use faraday_core::keygen::{Source, kstep, source_index};
use faraday_core::vaults::VaultAction as V;
use faraday_core::wordlist::{LISTS, WordList, WordListAction as WL};
use faraday_core::{Action, Faraday, Screen, Sheet};
use osk_bip::bip39::Language;
use osk_bip::diceware::List;
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

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
    app.press(Action::KWords(words));
    app.press(Action::KSource(source_index(Source::Coins)));
    app.press(Action::KNext);
    assert_eq!(app.keygen.as_ref().unwrap().open, Some(kstep::ENTER));
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
        let k = app.keygen.as_ref().unwrap();
        assert_eq!(&k.live_words()[..], &want[..want.len().min(11)]);
        for &i in &want {
            assert!(!Language::English.word(i).is_empty());
        }
    }
    // The eleven whole words are on screen, each linked to its list.
    assert_eq!(linked(&mut app), sorted(groups(&f)));
}

#[test]
fn the_last_word_is_not_shown_until_the_words_are_made() {
    for words in [12u8, 15, 18, 21, 24] {
        let bits = usize::from(words) * 32 / 3;
        let f = flips(bits);
        let mut app = coins(words);
        for &heads in &f {
            app.press(Action::KFlip(heads));
            let k = app.keygen.as_ref().unwrap();
            assert!(k.live_words().len() < usize::from(words));
            assert_eq!(k.checksum_word(), None);
        }
        // Every flip is in, and still only the words before the last.
        let before = groups(&f);
        assert_eq!(before.len(), usize::from(words) - 1);
        assert_eq!(linked(&mut app), sorted(before.clone()));

        // Made, the last word is the checksum's, and the card shows it.
        app.press(Action::KNext);
        app.press(Action::KStep(kstep::ENTER));
        let k = app.keygen.as_ref().unwrap();
        let m = k.mnemonic.as_ref().unwrap();
        assert_eq!(&m.indices()[..before.len()], &before[..]);
        let last = *m.indices().last().unwrap();
        assert_eq!(k.checksum_word(), Some(last));
        let mut all = before;
        all.push(last);
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
    let a = by_die.keygen.as_ref().unwrap();
    let b = by_coin.keygen.as_ref().unwrap();
    assert!(a.coins.flips().eq(b.coins.flips()));
    assert_eq!(&a.live_words()[..], &b.live_words()[..]);
    for app in [&mut by_die, &mut by_coin] {
        app.press(Action::KNext);
    }
    let a = by_die.keygen.as_ref().unwrap().phrase().unwrap();
    let b = by_coin.keygen.as_ref().unwrap().phrase().unwrap();
    assert_eq!(*a, *b);
}

#[test]
fn die_faces_typed_on_the_keyboard_count_as_flips_too() {
    let mut app = coins(12);
    app.press(Action::KByDie(true));
    for c in ['1', '3', '4', '6', '2', '5'] {
        app.event(Event::Key(Key::Char(c)));
    }
    let k = app.keygen.as_ref().unwrap();
    let got: Vec<bool> = k.coins.flips().collect();
    assert_eq!(got, [false, false, true, true, false, true]);
}

/// Rolls typed into the box as one string, taken with Enter.
fn typed(app: &mut Faraday, s: &str) {
    app.press(Action::KTyping(true));
    for c in s.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Enter));
}

#[test]
fn rolls_typed_as_a_string_make_the_key_the_same_presses_make() {
    let rolls: String = (0..99u32)
        .map(|i| char::from(b'1' + ((i * 5 + i / 3) % 6) as u8))
        .collect();
    let dice = |app: &mut Faraday, procedure: u8| {
        app.press(Action::KeyGen(None));
        app.press(Action::KWords(24));
        app.press(Action::KSource(source_index(Source::Dice)));
        app.press(Action::KProc(procedure));
        app.press(Action::KNext);
    };
    // The hashed procedure: every face counts.
    let mut pressed = shown();
    dice(&mut pressed, 0);
    for c in rolls.chars() {
        pressed.press(Action::KRoll(c as u8 - b'0'));
    }
    let mut boxed = shown();
    dice(&mut boxed, 0);
    typed(&mut boxed, &rolls);
    assert_eq!(
        pressed.keygen.as_ref().unwrap().dice.rolls(),
        boxed.keygen.as_ref().unwrap().dice.rolls()
    );
    for app in [&mut pressed, &mut boxed] {
        app.press(Action::KNext);
    }
    assert_eq!(
        *pressed.keygen.as_ref().unwrap().phrase().unwrap(),
        *boxed.keygen.as_ref().unwrap().phrase().unwrap()
    );

    // Choosing words directly, where a 5 or 6 is rolled again in most
    // places: the box refuses the same faces the pad does.
    let mut pressed = shown();
    dice(&mut pressed, 2);
    for c in rolls.chars() {
        pressed.press(Action::KRoll(c as u8 - b'0'));
    }
    let mut boxed = shown();
    dice(&mut boxed, 2);
    typed(&mut boxed, &rolls);
    let (a, b) = (
        pressed.keygen.as_ref().unwrap(),
        boxed.keygen.as_ref().unwrap(),
    );
    assert!(a.dice.len() < rolls.len());
    assert_eq!(a.dice.rolls(), b.dice.rolls());
    assert_eq!(&a.live_words()[..], &b.live_words()[..]);
}

#[test]
fn words_chosen_directly_show_only_the_words_rolled() {
    let mut app = shown();
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KSource(source_index(Source::Dice)));
    app.press(Action::KProc(2));
    app.press(Action::KNext);
    for i in 0..66u32 {
        let face = if i % 6 == 5 {
            (i % 3) as u8 + 4
        } else {
            (i * 3 % 4) as u8 + 1
        };
        app.press(Action::KRoll(face));
    }
    let k = app.keygen.as_ref().unwrap();
    let named: Vec<u16> = k.dice.word_indices().collect();
    assert_eq!(named.len(), 11);
    assert_eq!(&k.live_words()[..], &named[..]);
    assert_eq!(k.checksum_word(), None);
}

#[test]
fn hashed_dice_show_no_words_as_they_come_in() {
    let mut app = shown();
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KSource(source_index(Source::Dice)));
    app.press(Action::KNext);
    for i in 0..50u8 {
        app.press(Action::KRoll(i % 6 + 1));
    }
    assert!(app.keygen.as_ref().unwrap().live_words().is_empty());
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
    let st = app.wordlist.as_ref().unwrap();
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
    assert_eq!(app.keygen.as_ref().unwrap().coins.len(), 70);
    let _ = app.frame();
    assert!(app.offers(Action::WordList(WL::Close)));
    app.press(Action::WordList(WL::Close));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
    let k = app.keygen.as_ref().unwrap();
    assert_eq!(k.open, Some(kstep::ENTER));
    assert!(k.coins.flips().eq(f.iter().copied()));
    // Escape closes it the same way.
    app.press(open);
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(app.keygen.as_ref().unwrap().coins.len(), 70);
}

#[test]
fn a_passphrase_word_s_link_marks_it_in_its_eff_list_and_back_keeps_the_rolls() {
    let mut app = shown();
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::CNext(0)));
    app.press(Action::Vault(V::CPreset(0)));
    app.press(Action::Vault(V::CNext(1)));
    app.press(Action::Vault(V::CNext(2)));
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
    let st = app.wordlist.as_ref().unwrap();
    assert_eq!(st.list, WordList::Eff(List::Short1));
    assert_eq!(st.list.words()[st.mark.unwrap()], word);
    let _ = app.frame();
    assert!(app.offers(Action::WordList(WL::Mark(place))));
    // Keys are the sheet's: these letters find a word, not rolls.
    app.event(Event::Key(Key::Char('z')));
    app.press(Action::WordList(WL::Close));
    assert_eq!(app.screen, Screen::CreateVault);
    let (_, typed, list) = app.vaults.dice.as_ref().unwrap();
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
        assert_eq!(app.wordlist.as_ref().unwrap().list, *list);
        // The first word is on screen, and a press marks it.
        let _ = app.frame();
        assert!(app.offers(Action::WordList(WL::Mark(0))));
        app.press(Action::WordList(WL::Mark(0)));
        assert_eq!(app.wordlist.as_ref().unwrap().mark, Some(0));
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
    // EFF: the dice shown for a word roll that word in osk-bip's list.
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
    let i = app.wordlist.as_ref().unwrap().mark.unwrap();
    assert_eq!(Language::English.word(i as u16), "zone");
    let _ = app.frame();
    assert!(app.offers(Action::WordList(WL::Mark(i as u16))));
}
