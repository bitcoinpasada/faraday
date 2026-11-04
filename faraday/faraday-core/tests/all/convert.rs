//! Convert a seed: a loaded key reaches its master seed and its xprv by
//! BIP-39's and BIP-32's published vectors, a row not offered takes no
//! press, and nothing stays once the flow is left.

use faraday_core::convert::{self, ConvertAction as C, ROWS, cstep};
use faraday_core::forms::Form;
use faraday_core::wallet::Origin;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

/// BIP-39's first twelve-word vector.
const ABOUT: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
    abandon abandon about";
const ABOUT_SEED: &str = "5eb00bbddcf069084889a8ab9155568165f5c453ccb85e70811aaed6f6da5fc1\
    9a5ac40b389cd370d086206dec8aa6c43daea6690f20ad3d8d48b2d2ce9e38e4";
const ABOUT_XPRV: &str = "xprv9s21ZrQH143K3GJpoapnV8SFfukcVBSfeCficPSGfubmSFDxo1kuHnLisriDvSnRRuL2Qrg5ggqHKNVpxR86QEC8w35uxmGoggxtQTPvfUu";
const ABOUT_TPRV: &str = "tprv8ZgxMBicQKsPe5YMU9gHen4Ez3ApihUfykaqUorj9t6FDqy3nP6eoXiAo2ssvpAjoLroQxHqr3R5nE3a5dU3DHTjTgJDd7zrbniJr6nrCzd";
/// The same words with the passphrase "TREZOR", BIP-39's vector.
const TREZOR_XPRV: &str = "xprv9s21ZrQH143K3h3fDYiay8mocZ3afhfULfb5GX8kCBdno77K4HiA15Tg23wpbeF1pLfs1c5SPmYHrEpTuuRhxMwvKDwqdKiGJS9XFKzUsAF";
/// BIP-32's test vector 1.
const SEED_1: &str = "000102030405060708090a0b0c0d0e0f";
const SEED_1_XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";

/// The rows by name, as a test presses them.
fn place(name: &str) -> u8 {
    let at = ROWS.iter().position(|r| r.name == name);
    at.unwrap_or_else(|| panic!("no row {name}")) as u8
}

fn mainnet() -> Faraday {
    testkit::started()
}

fn test_net() -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::net()).unwrap();
    app
}

/// `app` with the words loaded, the flow opened from Tools' tile.
fn opened(mut app: Faraday, words: &str) -> Faraday {
    let c = testkit::compute();
    app.session.keys.add_words(&c, words, "Test", None).unwrap();
    open(&mut app);
    app
}

fn open(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Convert a seed")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::Convert);
}

/// Presses Save, its work on the tick after the frame.
fn save(app: &mut Faraday) {
    app.press(Action::Convert(C::Save));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(app);
}

/// Presses a row, its work on the tick after the frame (`docs/SPEED.md`
/// §9), and opens the value's eye.
fn make(app: &mut Faraday, name: &str) -> Option<String> {
    app.press(Action::Convert(C::To(place(name))));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(app);
    assert_eq!(convert::shown(app), None, "the value starts masked");
    app.press(Action::Convert(C::Show));
    let _ = app.frame();
    convert::shown(app).map(str::to_string)
}

#[test]
fn a_key_of_words_gives_bip_39s_seed_and_the_master_xprv() {
    let mut app = opened(mainnet(), ABOUT);
    assert_eq!(
        make(&mut app, "Master seed, hex").as_deref(),
        Some(ABOUT_SEED)
    );
    assert_eq!(
        convert::state(&app).unwrap().open,
        Some(cstep::RESULT),
        "Result opens on the press"
    );
    app.press(Action::Convert(C::Step(cstep::TO)));
    assert_eq!(make(&mut app, "xprv").as_deref(), Some(ABOUT_XPRV));
}

#[test]
fn a_test_network_writes_a_tprv() {
    let mut app = opened(test_net(), ABOUT);
    assert_eq!(make(&mut app, "xprv").as_deref(), Some(ABOUT_TPRV));
}

#[test]
fn a_passphrase_is_folded_into_the_result() {
    let mut app = mainnet();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    let fp = app
        .session
        .keys
        .add_with_passphrase(&c, 0, "TREZOR", "Test 2")
        .unwrap();
    open(&mut app);
    app.press(Action::Convert(C::Key(fp.0)));
    assert_eq!(make(&mut app, "xprv").as_deref(), Some(TREZOR_XPRV));
    assert!(app.session.keys[1].has_passphrase);
}

#[test]
fn a_key_of_seed_bytes_gives_those_bytes_and_bip_32s_xprv() {
    let mut app = mainnet();
    let c = testkit::compute();
    let seed: Vec<u8> = (0..16).collect();
    app.session.keys.add_seed(&c, &seed, "Test").unwrap();
    assert_eq!(convert::source(&app.session.keys[0]), "Master seed");
    open(&mut app);
    assert_eq!(make(&mut app, "Master seed, hex").as_deref(), Some(SEED_1));
    app.press(Action::Convert(C::Step(cstep::TO)));
    assert_eq!(make(&mut app, "xprv").as_deref(), Some(SEED_1_XPRV));
}

#[test]
fn from_says_what_a_key_of_words_was_loaded_from() {
    let app = opened(mainnet(), ABOUT);
    assert_eq!(convert::source(&app.session.keys[0]), "BIP-39 words, 12");
}

#[test]
fn a_row_not_offered_does_nothing() {
    let mut app = opened(mainnet(), ABOUT);
    for (i, r) in ROWS.iter().enumerate().filter(|(_, r)| r.target.is_none()) {
        app.press(Action::Convert(C::To(i as u8)));
        let _ = app.frame();
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        let s = convert::state(&app).unwrap();
        assert_eq!(s.target, None, "{} made something", r.name);
        assert_eq!(s.open, Some(cstep::TO), "{} moved on", r.name);
    }
}

#[test]
fn a_key_of_words_and_a_key_of_seed_bytes_list_their_own_rows() {
    let mut app = mainnet();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    let seed: Vec<u8> = (0..16).collect();
    app.session.keys.add_seed(&c, &seed, "Test 2").unwrap();
    let listed = |i: usize| -> Vec<&str> {
        let key = &app.session.keys[i];
        ROWS.iter()
            .filter(|r| r.applies(key))
            .map(|r| r.name)
            .collect()
    };
    assert!(listed(0).contains(&"SLIP-39 or codex32"));
    assert!(!listed(0).contains(&"BIP-39 words"));
    assert!(listed(1).contains(&"BIP-39 words"));
    assert!(!listed(1).contains(&"SLIP-39 or codex32"));
}

#[test]
fn leaving_drops_the_result() {
    let mut app = opened(mainnet(), ABOUT);
    assert!(make(&mut app, "xprv").is_some());
    // Back to To drops it, and it does not come back on going forward.
    app.press(Action::Convert(C::Step(cstep::TO)));
    assert_eq!(convert::shown(&app), None);
    app.press(Action::Convert(C::Step(cstep::RESULT)));
    let s = convert::state(&app).unwrap();
    assert_eq!(s.open, Some(cstep::TO));
    assert_eq!(s.target, None);
    // Done leaves the flow, and nothing is kept.
    assert!(make(&mut app, "xprv").is_some());
    app.press(Action::Convert(C::Done));
    assert_eq!(app.screen, Screen::Catalog);
    assert!(convert::state(&app).is_none());
    assert_eq!(convert::shown(&app), None);
    // Opened again, it starts masked and with no result.
    open(&mut app);
    assert_eq!(convert::state(&app).unwrap().target, None);
}

#[test]
fn a_key_added_from_the_tool_returns_to_it() {
    let mut app = mainnet();
    open(&mut app);
    assert_eq!(convert::state(&app).unwrap().open, Some(cstep::FROM));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    assert!(convert::state(&app).is_some(), "the detour keeps the tool");
    // New key's Back returns as its own flow does.
    let back = app.keygen().map(|k| k.back);
    assert_eq!(back, Some(Screen::Convert));
    app.press(Action::Nav(Screen::Convert));
    assert_eq!(app.screen, Screen::Convert);
    assert!(convert::state(&app).is_some());
    // Anywhere else ends it.
    app.press(Action::Nav(Screen::Catalog));
    assert!(convert::state(&app).is_none());
}

#[test]
fn the_paths_fact_names_the_wallets_that_hold_the_key() {
    let mut app = test_net();
    let c = testkit::compute();
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    app.session
        .keys
        .add_words(&c, &words, "Test", None)
        .unwrap();
    let descriptor = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/0'"));
    app.session
        .add_wallet(&c, "Spending", &descriptor, "Test wallet", Origin::Named)
        .unwrap();
    open(&mut app);
    assert!(make(&mut app, "xprv").is_some());
    assert_eq!(
        convert::state(&app).unwrap().paths,
        ["Wallet 'Spending' · m/84'/1'/0'"]
    );
}

/// The copy typed on the keyboard.
fn type_copy(app: &mut Faraday, copy: &str) {
    for c in copy.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Check my copy on `value`'s row: wrong, then right, each saying so where
/// the person can read it.
fn checks(row: &str, right: &str) {
    let mut app = opened(mainnet(), ABOUT);
    assert_eq!(make(&mut app, row).as_deref(), Some(right));
    app.press(Action::Convert(C::Check));
    // One character short, and one changed.
    type_copy(&mut app, &right[..right.len() - 1]);
    app.press(Action::Convert(C::Compare));
    assert_eq!(convert::found(&app), Some(false));
    let _ = app.frame();
    assert!(app.drawn_texts().iter().any(|t| t == "Your copy differs"));
    assert_eq!(convert::typed(&app), Some(""), "what differs is wiped");
    let mut changed = right.to_string();
    changed.replace_range(..1, if right.starts_with('0') { "1" } else { "0" });
    type_copy(&mut app, &changed);
    assert_eq!(convert::found(&app), None, "typing clears the finding");
    app.event(Event::Key(Key::Enter));
    assert_eq!(convert::found(&app), Some(false));
    type_copy(&mut app, right);
    app.event(Event::Key(Key::Enter));
    assert_eq!(convert::found(&app), Some(true));
    let _ = app.frame();
    assert!(app.drawn_texts().iter().any(|t| t == "Your copy matches"));
    assert_eq!(convert::typed(&app), None, "a match closes the check");
}

#[test]
fn check_my_copy_says_a_copy_of_the_hex_is_right_or_wrong() {
    checks("Master seed, hex", ABOUT_SEED);
}

#[test]
fn check_my_copy_says_a_copy_of_the_xprv_is_right_or_wrong() {
    checks("xprv", ABOUT_XPRV);
}

#[test]
fn check_my_copy_works_with_the_value_masked() {
    let mut app = opened(mainnet(), ABOUT);
    assert!(make(&mut app, "xprv").is_some());
    app.press(Action::Convert(C::Show));
    assert_eq!(convert::shown(&app), None);
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, ABOUT_XPRV);
    app.press(Action::Convert(C::Compare));
    assert_eq!(convert::found(&app), Some(true));
    let _ = app.frame();
    assert!(app.drawn_texts().iter().any(|t| t == "Your copy matches"));
}

#[test]
fn leaving_result_drops_what_was_typed() {
    let mut app = opened(mainnet(), ABOUT);
    assert!(make(&mut app, "xprv").is_some());
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, "xprv");
    assert_eq!(convert::typed(&app), Some("xprv"));
    // Escape closes the check, and the flow stays.
    app.event(Event::Key(Key::Escape));
    assert_eq!(convert::typed(&app), None);
    assert_eq!(app.screen, Screen::Convert);
    // Back to To drops it.
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, "xprv");
    app.press(Action::Convert(C::Step(cstep::TO)));
    assert_eq!(convert::typed(&app), None);
    // So does leaving the flow.
    assert!(make(&mut app, "xprv").is_some());
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, "xprv");
    app.press(Action::Convert(C::Done));
    assert_eq!(convert::typed(&app), None);
    open(&mut app);
    assert_eq!(convert::typed(&app), None);
    assert_eq!(convert::found(&app), None);
}

#[test]
fn save_offers_exactly_the_value_and_writes_nothing_until_answered() {
    let mut app = opened(mainnet(), ABOUT);
    let waiting = app.outbox.len();
    assert_eq!(make(&mut app, "xprv").as_deref(), Some(ABOUT_XPRV));
    assert_eq!(app.outbox.len(), waiting, "a conversion writes nothing");
    save(&mut app);
    let out = testkit::secret_out(&app).expect("the Secret out sheet is up");
    assert_eq!(out.bytes.as_slice(), ABOUT_XPRV.as_bytes());
    assert_eq!(
        app.outbox.len(),
        waiting,
        "nothing until the sheet is answered"
    );
    // Cancel writes nothing, and the result stays.
    app.press(Action::Cancel);
    assert_eq!(app.outbox.len(), waiting);
    assert_eq!(convert::shown(&app), Some(ABOUT_XPRV));
    // Answered: the file holds the value and nothing else.
    save(&mut app);
    app.press(Action::SecretUnprotected);
    assert_eq!(app.outbox.len(), waiting, "not before the hold");
    app.press(Action::SecretHold);
    let item = app.outbox.last().expect("the file is written");
    assert_eq!(&item.bytes[..], ABOUT_XPRV.as_bytes());
}

/// BIP-39's vector of all ones, whose words are three letters and five.
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
/// BIP-39's second twelve-word vector.
const LEGAL: &str = "legal winner thank year wave sausage worth useful legal winner thank yellow";
/// BIP-39's Japanese vector of all zeros (`tools/vectors/bip39/test_JP_BIP39.json`).
const JAPANESE: &str = "あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　\
    あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あおぞら";

/// The numbers 1 to 2048 alone, as a copy of the numbers is typed.
const ABOUT_NUMBERS: &str = "1 1 1 1 1 1 1 1 1 1 1 4";
const ABOUT_ENTROPY: &str = "00000000000000000000000000000000";

#[test]
fn a_key_of_words_is_written_in_each_form_by_bip_39() {
    let mut app = opened(mainnet(), ABOUT);
    let next = |app: &mut Faraday, name| {
        let value = make(app, name);
        app.press(Action::Convert(C::Step(cstep::TO)));
        value
    };
    let about = ABOUT.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(next(&mut app, "The words"), Some(about));
    assert_eq!(
        next(&mut app, "Hex entropy").as_deref(),
        Some(ABOUT_ENTROPY)
    );
    let numbers = next(&mut app, "Word numbers").unwrap();
    assert_eq!(numbers.lines().next(), Some("1 (0)"));
    assert_eq!(numbers.lines().last(), Some("4 (3)"));
    assert_eq!(numbers.lines().count(), 12);
    assert_eq!(
        next(&mut app, "4-letter starts").as_deref(),
        Some("aban aban aban aban aban aban aban aban aban aban aban abou")
    );
    // A word of three letters is its own start, and a list's last word
    // has the highest numbers.
    let mut app = opened(mainnet(), ZOO);
    assert_eq!(
        next(&mut app, "4-letter starts").as_deref(),
        Some("zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wron")
    );
    let numbers = next(&mut app, "Word numbers").unwrap();
    assert_eq!(numbers.lines().next(), Some("2048 (2047)"));
    assert_eq!(numbers.lines().last(), Some("2038 (2037)"));
    assert_eq!(
        next(&mut app, "Hex entropy").as_deref(),
        Some("ffffffffffffffffffffffffffffffff")
    );
    let mut app = opened(mainnet(), LEGAL);
    assert_eq!(
        next(&mut app, "4-letter starts").as_deref(),
        Some("lega winn than year wave saus wort usef lega winn than yell")
    );
    assert_eq!(
        next(&mut app, "Hex entropy").as_deref(),
        Some("7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f")
    );
}

/// Presses the code's row and opens its eye; what a scanner reads from it.
fn scanned(app: &mut Faraday, name: &str) -> Vec<u8> {
    assert_eq!(make(app, name), None, "a code is not text");
    let code = convert::shown_code(app).expect("the code shows");
    let (w, h, luma) = code.to_luma(6, 4);
    let c = testkit::compute();
    let read = faraday_codec::decode::decode_luma(&c, w, h, &luma);
    assert_eq!(read.len(), 1, "one code");
    read[0].bytes.to_vec()
}

#[test]
fn a_seedqr_and_a_compact_seedqr_read_back_to_the_same_words() {
    let mut app = opened(mainnet(), LEGAL);
    let digits = scanned(&mut app, "SeedQR");
    let lang = faraday_bip::bip39::Language::English;
    let m = faraday_keys::words::from_seedqr_digits(&digits, lang).unwrap();
    let text: Vec<&str> = m.indices().iter().map(|&i| lang.word(i)).collect();
    assert_eq!(text.join(" "), LEGAL);
    app.press(Action::Convert(C::Step(cstep::TO)));
    let bytes = scanned(&mut app, "CompactSeedQR");
    assert_eq!(bytes, [0x7f; 16], "the vector's entropy");
}

#[test]
fn a_code_masked_shows_nothing_and_has_no_check_of_a_copy() {
    let mut app = opened(mainnet(), ABOUT);
    let _ = scanned(&mut app, "SeedQR");
    assert!(!app.drawn_texts().iter().any(|t| t == "Check my copy"));
    app.press(Action::Convert(C::Check));
    assert_eq!(convert::typed(&app), None, "no check opens");
    app.press(Action::Convert(C::Show));
    assert!(convert::shown_code(&app).is_none());
}

#[test]
fn save_writes_a_codes_labelled_picture_with_the_backups_name() {
    let mut app = opened(mainnet(), ABOUT);
    let _ = scanned(&mut app, "SeedQR");
    save(&mut app);
    let out = testkit::secret_out(&app).expect("the Secret out sheet is up");
    assert_eq!(out.name, "seedqr-73c5da0a.png");
    assert!(out.bytes.starts_with(b"\x89PNG"));
    app.press(Action::Cancel);
    app.press(Action::Convert(C::Step(cstep::TO)));
    let _ = scanned(&mut app, "CompactSeedQR");
    save(&mut app);
    let out = testkit::secret_out(&app).expect("the Secret out sheet is up");
    assert_eq!(out.name, "compactseedqr-73c5da0a.png");
}

#[test]
fn save_writes_each_text_form() {
    let mut app = opened(mainnet(), ABOUT);
    for (row, end) in [
        ("The words", "words"),
        ("Hex entropy", "entropy"),
        ("Word numbers", "word-numbers"),
        ("4-letter starts", "starts"),
    ] {
        let value = make(&mut app, row).unwrap();
        save(&mut app);
        let out = testkit::secret_out(&app).expect("the Secret out sheet is up");
        assert_eq!(out.name, format!("73c5da0a-{end}.txt"));
        assert_eq!(out.bytes.as_slice(), value.as_bytes());
        app.press(Action::Cancel);
        app.press(Action::Convert(C::Step(cstep::TO)));
    }
}

#[test]
fn check_my_copy_says_a_copy_of_each_written_form_is_right_or_wrong() {
    let about = ABOUT.split_whitespace().collect::<Vec<_>>().join(" ");
    checks("The words", &about);
    checks("Hex entropy", ABOUT_ENTROPY);
    checks(
        "4-letter starts",
        "aban aban aban aban aban aban aban aban aban aban aban abou",
    );
}

#[test]
fn a_copy_of_the_numbers_is_compared_on_the_numbers_alone() {
    let mut app = opened(mainnet(), ABOUT);
    assert!(make(&mut app, "Word numbers").is_some());
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, "1 1 1 1 1 1 1 1 1 1 1 3");
    app.press(Action::Convert(C::Compare));
    assert_eq!(convert::found(&app), Some(false));
    // The numbers 0 to 2047 are the ones beside: not the copy asked for.
    type_copy(&mut app, "0 0 0 0 0 0 0 0 0 0 0 3");
    app.press(Action::Convert(C::Compare));
    assert_eq!(convert::found(&app), Some(false));
    type_copy(&mut app, ABOUT_NUMBERS);
    app.press(Action::Convert(C::Compare));
    assert_eq!(convert::found(&app), Some(true));
}

#[test]
fn a_key_on_another_list_is_not_offered_the_codes_or_the_starts() {
    let mut app = opened(mainnet(), JAPANESE);
    let key = &app.session.keys[0];
    let listed: Vec<&str> = ROWS
        .iter()
        .filter(|r| r.applies(key) && r.target.is_some())
        .map(|r| r.name)
        .collect();
    for name in ["SeedQR", "CompactSeedQR", "4-letter starts"] {
        assert!(!listed.contains(&name), "{name} is offered");
    }
    // A press on one, were it made, does nothing.
    app.press(Action::Convert(C::To(place("SeedQR"))));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    assert_eq!(convert::state(&app).unwrap().target, None);
    // The rest read in its own list; the words have no typed check.
    assert_eq!(
        make(&mut app, "Hex entropy").as_deref(),
        Some(ABOUT_ENTROPY)
    );
    app.press(Action::Convert(C::Step(cstep::TO)));
    let numbers = make(&mut app, "Word numbers").unwrap();
    assert_eq!(numbers.lines().last(), Some("4 (3)"));
    app.press(Action::Convert(C::Step(cstep::TO)));
    let words = make(&mut app, "The words").unwrap();
    assert!(words.starts_with("あいこくしん "));
    assert!(words.ends_with("あおぞら"));
    assert!(!app.drawn_texts().iter().any(|t| t == "Check my copy"));
    app.press(Action::Convert(C::Check));
    assert_eq!(convert::typed(&app), None, "no check opens");
}

#[test]
fn a_key_with_no_words_is_offered_none_of_the_words_forms() {
    let mut app = mainnet();
    let c = testkit::compute();
    let seed: Vec<u8> = (0..16).collect();
    app.session.keys.add_seed(&c, &seed, "Test").unwrap();
    let key = &app.session.keys[0];
    for name in [
        "The words",
        "Seed XOR parts",
        "SeedQR",
        "CompactSeedQR",
        "Hex entropy",
        "Word numbers",
        "4-letter starts",
    ] {
        let row = ROWS.iter().find(|r| r.name == name).unwrap();
        assert!(!row.applies(key), "{name} is offered");
    }
}

#[test]
fn a_passphrase_is_said_to_stay_apart_from_the_words() {
    let mut app = mainnet();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    let fp = app
        .session
        .keys
        .add_with_passphrase(&c, 0, "TREZOR", "Test 2")
        .unwrap();
    open(&mut app);
    app.press(Action::Convert(C::Key(fp.0)));
    assert!(make(&mut app, "The words").is_some());
    let says = "Not in this form: restore it with the passphrase";
    assert!(app.drawn_texts().iter().any(|t| t.contains(says)));
}

/// The way in `from`, then Done and Back each land on the screen it was
/// pressed from, and nothing is held after.
fn returns_to(app: &mut Faraday, press_in: fn(&mut Faraday), from: Screen, fp: [u8; 4]) {
    press_in(app);
    assert_eq!(app.screen, Screen::Convert);
    let s = convert::state(app).unwrap();
    assert_eq!((s.key, s.open), (Some(fp), Some(cstep::TO)));
    assert!(make(app, "Hex entropy").is_some());
    app.press(Action::Convert(C::Done));
    assert_eq!(app.screen, from);
    assert!(convert::state(app).is_none());
    assert_eq!(convert::shown(app), None);
    assert!(app.sheet.is_none());
    // Back, by the key.
    press_in(app);
    assert_eq!(app.screen, Screen::Convert);
    // Back goes a card at a time (To, From), then out.
    for _ in 0..2 {
        let _ = app.frame();
        app.event(Event::Key(Key::Escape));
    }
    assert_eq!(app.screen, from);
    assert!(convert::state(app).is_none());
}

#[test]
fn explore_opens_the_flow_on_its_key_and_done_returns_there() {
    let mut app = mainnet();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    let fp = app
        .session
        .keys
        .add_words(&c, LEGAL, "Test 2", None)
        .unwrap();
    app.press(Action::Explore);
    app.press(Action::XKey(fp.0));
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Write it in another form")
    );
    fn way_in(app: &mut Faraday) {
        let fp = app.explore().and_then(|e| e.key).unwrap();
        app.press(Action::Convert(C::OpenOn(fp)));
    }
    returns_to(&mut app, way_in, Screen::Explore, fp.0);
    assert_eq!(
        app.explore().and_then(|e| e.key),
        Some(fp.0),
        "Explore is as it was"
    );
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Write it in another form"),
        "Explore draws again"
    );
}

#[test]
fn a_key_not_loaded_opens_nothing() {
    let mut app = opened(mainnet(), ABOUT);
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Convert(C::OpenOn([1, 2, 3, 4])));
    assert_eq!(app.screen, Screen::Catalog);
    assert!(convert::state(&app).is_none());
}

#[test]
fn practice_converts_a_public_seed() {
    let mut app = testkit::started();
    testkit::into_demo(&mut app);
    let c = testkit::compute();
    let (word, count) = testkit::SAMPLE_WORDS[0];
    let words = vec![word; usize::from(count)].join(" ");
    app.session
        .keys
        .add_words(&c, &words, "Sample", None)
        .unwrap();
    open(&mut app);
    let value = make(&mut app, "xprv").unwrap();
    // Check my copy and the sheet stay open there, the seed being public.
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, &value);
    app.press(Action::Convert(C::Compare));
    assert_eq!(convert::found(&app), Some(true));
    save(&mut app);
    assert_eq!(
        testkit::secret_out(&app).map(|o| o.bytes.to_vec()),
        Some(value.into_bytes())
    );
}

/// A right copy of a long value is answered where the person reads it, on
/// a small panel as on a large one.
#[test]
fn the_answer_to_a_compared_copy_is_on_the_screen() {
    let c = testkit::compute();
    for (w, h) in [(480u16, 640u16), (240, 320)] {
        for row in ["Word numbers", "The words"] {
            let words24 = ["abandon "; 23].concat() + "art";
            let mut app = mainnet();
            app.event(Event::Display(DisplayInfo {
                width: w,
                height: h,
                dpi: 160,
                inset_bottom: 0,
                inset_top: 0,
                buttons: 0,
                camera_fixed: false,
                secure: SecureHardware::None,
                boot: BootState::Unknown,
                memory_mib: None,
            }));
            app.session
                .keys
                .add_words(&c, &words24, "Test", None)
                .unwrap();
            open(&mut app);
            let shown = make(&mut app, row).unwrap();
            let right = if row == "Word numbers" {
                ["1 "; 23].concat() + "103"
            } else {
                shown
            };
            app.press(Action::Convert(C::Check));
            type_copy(&mut app, &right);
            app.press(Action::Convert(C::Compare));
            assert_eq!(convert::found(&app), Some(true), "{w}x{h} {row}: {right}");
            for _ in 0..40 {
                let _ = app.frame();
            }
            let at = app
                .drawn_text_boxes()
                .into_iter()
                .find(|(t, _)| t == "Your copy matches")
                .unwrap_or_else(|| panic!("{w}x{h} {row}: the answer is not drawn"));
            let (_, y, _, bh) = at.1;
            assert!(
                y >= 0 && y + bh <= i32::from(h),
                "{w}x{h} {row}: the answer is at {:?}",
                at.1
            );
        }
    }
}

/// BIP-39's 18-word vector of entropy 7f.
const EIGHTEEN: &str = "legal winner thank year wave sausage worth useful legal winner thank \
    year wave sausage worth useful legal will";
/// BIP-39's 24-word vector of all ones.
const ZOO_24: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo \
    zoo zoo zoo zoo zoo vote";

/// What is drawn on the screen, as one list.
fn drawn(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts().to_vec()
}

#[test]
fn a_word_list_is_named_for_numbers_and_entropy_of_a_key_not_on_the_english_list() {
    let mut app = opened(mainnet(), JAPANESE);
    for (row, file) in [
        ("Word numbers", "word-numbers-japanese.txt"),
        ("Hex entropy", "entropy-japanese.txt"),
    ] {
        assert!(make(&mut app, row).is_some());
        let texts = drawn(&mut app);
        assert!(texts.iter().any(|t| t == "Word list"), "{row}");
        assert!(texts.iter().any(|t| t == "Japanese"), "{row}");
        save(&mut app);
        let out = testkit::secret_out(&app).expect("the Secret out sheet is up");
        assert!(out.name.ends_with(file), "{row}: {}", out.name);
        app.press(Action::Cancel);
        app.press(Action::Convert(C::Step(cstep::TO)));
    }
    // The words carry their list; an English key has no such fact.
    assert!(make(&mut app, "The words").is_some());
    assert!(!drawn(&mut app).iter().any(|t| t == "Word list"));
    let mut app = opened(mainnet(), ABOUT);
    assert!(make(&mut app, "Word numbers").is_some());
    assert!(!drawn(&mut app).iter().any(|t| t == "Word list"));
    save(&mut app);
    let out = testkit::secret_out(&app).expect("the Secret out sheet is up");
    assert!(!out.name.contains("english"));
}

#[test]
fn a_key_of_eighteen_words_is_not_offered_a_seedqr() {
    let app = opened(mainnet(), EIGHTEEN);
    let key = &app.session.keys[0];
    for name in ["SeedQR", "CompactSeedQR"] {
        let row = ROWS.iter().find(|r| r.name == name).unwrap();
        assert!(!row.applies(key), "{name} is offered");
    }
    for name in ["The words", "Hex entropy", "4-letter starts"] {
        let row = ROWS.iter().find(|r| r.name == name).unwrap();
        assert!(row.applies(key), "{name} is not offered");
    }
}

#[test]
fn a_code_of_twelve_or_twenty_four_words_is_scanned_and_shown() {
    for (words, bytes) in [(LEGAL, 16), (ZOO_24, 32)] {
        let mut app = opened(mainnet(), words);
        let digits = scanned(&mut app, "SeedQR");
        assert_eq!(digits.len(), bytes * 3, "{words}");
        app.press(Action::Convert(C::Step(cstep::TO)));
        let entropy = scanned(&mut app, "CompactSeedQR");
        assert_eq!(entropy.len(), bytes, "{words}");
        assert!(convert::state(&app).unwrap().refused.is_none());
    }
}

#[test]
fn the_flow_through_add_a_key_returns_to_an_explore_that_draws() {
    let mut app = mainnet();
    let c = testkit::compute();
    let fp = app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    app.press(Action::Explore);
    app.press(Action::XKey(fp.0));
    app.press(Action::Convert(C::OpenOn(fp.0)));
    app.press(Action::Convert(C::Step(cstep::TO)));
    app.press(Action::Convert(C::Step(cstep::FROM)));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    app.press(Action::Nav(Screen::Convert));
    assert_eq!(app.screen, Screen::Convert);
    app.press(Action::Convert(C::Done));
    assert_eq!(app.screen, Screen::Explore);
    assert!(
        drawn(&mut app)
            .iter()
            .any(|t| t == "Write it in another form")
    );
}

#[test]
fn the_secret_sheet_says_a_form_without_the_passphrase_opens_another_wallet() {
    let mut app = mainnet();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    let fp = app
        .session
        .keys
        .add_with_passphrase(&c, 0, "TREZOR", "Test 2")
        .unwrap();
    open(&mut app);
    app.press(Action::Convert(C::Key(fp.0)));
    let every = "Whoever has it can spend from every wallet of this key";
    for (row, apart) in [("The words", true), ("xprv", false)] {
        assert!(make(&mut app, row).is_some());
        save(&mut app);
        let sheet = drawn(&mut app);
        assert_eq!(sheet.iter().any(|t| t == every), !apart, "{row}");
        assert!(
            sheet.iter().any(|t| t.contains("with no passphrase")) == apart,
            "{row}"
        );
        app.press(Action::Cancel);
        app.press(Action::Convert(C::Step(cstep::TO)));
    }
}

#[test]
fn a_copy_of_the_numbers_typed_as_shown_matches_only_when_each_pair_agrees() {
    let mut app = opened(mainnet(), ABOUT);
    assert!(make(&mut app, "Word numbers").is_some());
    app.press(Action::Convert(C::Check));
    let shown = "1 (0) 1 (0) 1 (0) 1 (0) 1 (0) 1 (0) 1 (0) 1 (0) 1 (0) 1 (0) 1 (0) 4 (3)";
    for (copy, same) in [
        (ABOUT_NUMBERS, true),
        (shown, true),
        // A pair whose two numbers do not agree.
        (&shown.replace("4 (3)", "4 (2)"), false),
        // Half of the pairs only.
        ("1 (0) 1 1 1 1 1 1 1 1 1 4", false),
    ] {
        type_copy(&mut app, copy);
        app.press(Action::Convert(C::Compare));
        assert_eq!(convert::found(&app), Some(same), "{copy}");
        if same {
            app.press(Action::Convert(C::Check));
        }
    }
}

#[test]
fn the_idle_warning_closes_the_eye_of_the_result() {
    let mut app = mainnet();
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    let mut app = opened(app, ABOUT);
    assert!(make(&mut app, "Hex entropy").is_some());
    assert!(convert::shown(&app).is_some());
    let last = app.now_ms();
    app.event(Event::Tick {
        now_ms: last + 26 * 60_000,
    });
    assert_eq!(app.sheet, Some(faraday_core::Sheet::IdleWarn));
    let _ = app.frame();
    assert_eq!(convert::shown(&app), None);
}

const SLIP39: &str = include_str!("../../../../tools/vectors/slip39/vectors.json");
const BIP93: &str = include_str!("../../../../tools/vectors/bip93/vectors.json");

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// The system's randomness answered, as the shell does.
fn feed(app: &mut Faraday) {
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x70 + i; 32])));
    }
}

/// `app` with a key of these seed bytes loaded, the flow open on it.
fn seed_opened(seed: &[u8]) -> Faraday {
    let mut app = mainnet();
    let c = testkit::compute();
    app.session.keys.add_seed(&c, seed, "Test").unwrap();
    open(&mut app);
    app
}

/// Runs the work asked for by a press: on the tick after the frame.
fn settle(app: &mut Faraday) {
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(app);
}

/// Presses a row, then Make where it has a Settings card, and opens the
/// eye. The strings of a set, if it made one.
fn make_set(app: &mut Faraday, name: &str) -> Option<Vec<String>> {
    app.press(Action::Convert(C::To(place(name))));
    settle(app);
    if convert::state(app).unwrap().open == Some(cstep::SETTINGS) {
        app.press(Action::Convert(C::Make));
        settle(app);
    }
    app.press(Action::Convert(C::Show));
    let _ = app.frame();
    convert::shown_parts(app).map(|p| p.into_iter().map(str::to_string).collect())
}

fn rows_listed(key: &faraday_keys::KeyInfo) -> Vec<&'static str> {
    ROWS.iter()
        .filter(|r| r.applies(key))
        .map(|r| r.name)
        .collect()
}

/// A key of seed bytes is offered SLIP-39 1 of 1 at 16 and 32 bytes, and
/// codex32 at every length; 64 bytes lists SLIP-39 dimmed with its line;
/// 20, 24 and 28 bytes list no SLIP-39 at all; a key of words is offered
/// Seed XOR and neither.
#[test]
fn the_rows_for_seed_bytes_follow_the_seeds_length() {
    let c = testkit::compute();
    for (len, slip, dimmed) in [
        (16, true, false),
        (20, false, false),
        (24, false, false),
        (28, false, false),
        (32, true, false),
        (64, false, true),
    ] {
        let mut app = mainnet();
        app.session
            .keys
            .add_seed(&c, &vec![5u8; len], "Test")
            .unwrap();
        let rows = rows_listed(&app.session.keys[0]);
        assert_eq!(rows.contains(&"SLIP-39, 1 of 1"), slip, "{len} bytes");
        assert_eq!(rows.contains(&"SLIP-39"), dimmed, "{len} bytes");
        assert!(rows.contains(&"codex32"), "{len} bytes");
        assert!(!rows.contains(&"Seed XOR parts"), "{len} bytes");
    }
    let line = ROWS.iter().find(|r| r.name == "SLIP-39").unwrap();
    assert_eq!(
        line.line,
        "SLIP-39 backups of 64 bytes are read by no wallet"
    );
    assert_eq!(line.target, None);
    let mut app = mainnet();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    let rows = rows_listed(&app.session.keys[0]);
    assert!(rows.contains(&"Seed XOR parts"));
    assert!(!rows.contains(&"SLIP-39, 1 of 1") && !rows.contains(&"codex32"));
}

/// Every published SLIP-39 secret of one share, written as SLIP-39 1 of 1,
/// is read by SLIP-39's decoder as the same secret, and written twice gives
/// the same words.
#[test]
fn slip39_one_of_one_reads_back_as_the_published_secret() {
    let v: serde_json::Value = serde_json::from_str(SLIP39).unwrap();
    let mut seen = 0;
    for e in v.as_array().unwrap() {
        let (words, secret) = (e[1].as_array().unwrap(), e[2].as_str().unwrap());
        if words.len() != 1 || secret.is_empty() {
            continue;
        }
        let seed = unhex(secret);
        let mut app = seed_opened(&seed);
        let made = make(&mut app, "SLIP-39, 1 of 1").unwrap();
        let share = faraday_bip::slip39::Share::parse(&made).unwrap();
        let back = faraday_keys::slip39_seed(&testkit::compute(), &[share], b"").unwrap();
        assert_eq!(back.as_slice(), seed.as_slice(), "{}", e[0]);
        app.press(Action::Convert(C::Step(cstep::TO)));
        assert_eq!(make(&mut app, "SLIP-39, 1 of 1"), Some(made), "twice");
        seen += 1;
    }
    assert!(seen > 0);
}

/// Every published codex32 master seed, written as codex32: the secret
/// alone, and a set of 2 of 3, each decoded by BIP-93's reader to the same
/// seed; the settings step through the thresholds BIP-93 allows.
#[test]
fn codex32_reads_back_as_the_published_seed_alone_and_as_a_set() {
    let v: serde_json::Value = serde_json::from_str(BIP93).unwrap();
    for e in v["secrets"].as_array().unwrap() {
        let seed = unhex(e["seed"].as_str().unwrap());
        let mut app = seed_opened(&seed);
        feed(&mut app);
        let alone = make_set(&mut app, "codex32").unwrap();
        assert_eq!(alone.len(), 1);
        let part = faraday_bip::codex32::Codex32::parse(&alone[0]).unwrap();
        assert_eq!(*faraday_keys::codex32_seed(&[part]).unwrap(), seed);
        assert!(
            drawn(&mut app)
                .iter()
                .any(|t| t == "BIP-93 codex32, the secret")
        );
        // The card again: 0 steps to 2 and back; n follows k.
        app.press(Action::Convert(C::Step(cstep::SETTINGS)));
        app.press(Action::Convert(C::K(1)));
        let s = convert::state(&app).unwrap();
        assert_eq!((s.k, s.n), (2, 2));
        app.press(Action::Convert(C::N(1)));
        assert_eq!(convert::state(&app).unwrap().n, 3);
        app.press(Action::Convert(C::Make));
        settle(&mut app);
        app.press(Action::Convert(C::Show));
        let _ = app.frame();
        let set = convert::shown_parts(&app).unwrap();
        assert_eq!(set.len(), 3);
        let parsed: Vec<_> = set
            .iter()
            .map(|s| faraday_bip::codex32::Codex32::parse(s).unwrap())
            .collect();
        for two in [&parsed[..2], &parsed[1..]] {
            assert_eq!(*faraday_keys::codex32_seed(two).unwrap(), seed);
        }
        assert!(
            drawn(&mut app)
                .iter()
                .any(|t| t == "BIP-93 codex32, 2 of 3 shares")
        );
    }
}

/// The parts of Seed XOR typed into Add a key give the same key.
fn xor_added(parts: &[String]) -> [u8; 4] {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    let form = Form::ALL.iter().position(|f| *f == Form::Xor).unwrap();
    app.press(Action::EntryForm(form as u8));
    for p in parts {
        type_copy(&mut app, p);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    app.session.keys[0].fingerprint.0
}

#[test]
fn seed_xor_parts_typed_into_add_a_key_give_the_same_key() {
    for parts in [2u8, 3, 4] {
        let mut app = opened(mainnet(), LEGAL);
        feed(&mut app);
        app.press(Action::Convert(C::To(place("Seed XOR parts"))));
        assert_eq!(convert::state(&app).unwrap().open, Some(cstep::SETTINGS));
        for _ in 2..parts {
            app.press(Action::Convert(C::Parts(1)));
        }
        app.press(Action::Convert(C::Make));
        settle(&mut app);
        app.press(Action::Convert(C::Show));
        let _ = app.frame();
        let set = convert::shown_parts(&app).unwrap();
        assert_eq!(set.len(), usize::from(parts));
        assert!(set.iter().all(|p| p.split(' ').count() == 12));
        let set: Vec<String> = set.into_iter().map(str::to_string).collect();
        assert_eq!(xor_added(&set), app.session.keys[0].fingerprint.0);
        assert!(
            drawn(&mut app)
                .iter()
                .any(|t| t == &format!("Seed XOR (Coldcard's), {parts} parts"))
        );
    }
}

/// The parts stay between 2 and 4, whatever is pressed.
#[test]
fn the_parts_stay_between_two_and_four() {
    let mut app = opened(mainnet(), ABOUT);
    app.press(Action::Convert(C::To(place("Seed XOR parts"))));
    for _ in 0..5 {
        app.press(Action::Convert(C::Parts(1)));
    }
    assert_eq!(convert::state(&app).unwrap().parts, 4);
    for _ in 0..5 {
        app.press(Action::Convert(C::Parts(-1)));
    }
    assert_eq!(convert::state(&app).unwrap().parts, 2);
}

/// What is drawn once a card that has just opened has finished growing: the
/// clock moves only on a tick, and a card opened by the last tick shows
/// nothing of its body yet.
fn opened_up(app: &mut Faraday) -> Vec<String> {
    for _ in 0..40 {
        let _ = app.frame();
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
    }
    drawn(app)
}

/// On a small panel, where the open card is the page, the answer to Check my
/// copy is on it.
#[test]
fn a_small_panel_shows_which_part_a_copy_matched() {
    let mut app = mainnet();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let mut app = opened(app, LEGAL);
    feed(&mut app);
    let set = make_set(&mut app, "Seed XOR parts").unwrap();
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, &set[1]);
    app.event(Event::Key(Key::Enter));
    assert!(
        opened_up(&mut app)
            .iter()
            .any(|t| t == "Your copy matches part 2")
    );
}

/// Check my copy takes one part and says which it matched, or that it
/// differs; Save writes every part; leaving drops them all.
#[test]
fn a_set_is_checked_part_by_part_saved_whole_and_dropped_on_leaving() {
    let mut app = opened(mainnet(), LEGAL);
    feed(&mut app);
    let set = make_set(&mut app, "Seed XOR parts").unwrap();
    assert_eq!(set.len(), 2);
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, &set[1]);
    app.event(Event::Key(Key::Enter));
    assert_eq!(convert::found(&app), Some(true));
    assert_eq!(convert::found_part(&app), Some(2));
    assert!(
        opened_up(&mut app)
            .iter()
            .any(|t| t == "Your copy matches part 2")
    );
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, &set[0]);
    app.event(Event::Key(Key::Enter));
    assert_eq!(convert::found_part(&app), Some(1));
    assert!(
        opened_up(&mut app)
            .iter()
            .any(|t| t == "Your copy matches part 1")
    );
    app.press(Action::Convert(C::Check));
    type_copy(&mut app, ABOUT);
    app.event(Event::Key(Key::Enter));
    assert_eq!(convert::found(&app), Some(false));
    assert!(opened_up(&mut app).iter().any(|t| t == "Your copy differs"));
    save(&mut app);
    let out = testkit::secret_out(&app).unwrap();
    assert!(out.name.ends_with("seed-xor.txt"), "{}", out.name);
    assert_eq!(out.bytes.to_vec(), (set.join("\n") + "\n").into_bytes());
    app.press(Action::Convert(C::Step(cstep::TO)));
    app.press(Action::Convert(C::Show));
    assert_eq!(convert::shown_parts(&app), None);
    assert_eq!(convert::state(&app).unwrap().target, None);
}

/// With no randomness answered yet, no set is made: the card stays and
/// says so; once answered, Make gives one.
#[test]
fn no_randomness_gives_no_set() {
    let mut app = opened(mainnet(), ABOUT);
    app.press(Action::Convert(C::To(place("Seed XOR parts"))));
    app.press(Action::Convert(C::Make));
    settle(&mut app);
    let s = convert::state(&app).unwrap();
    assert_eq!(s.open, Some(cstep::SETTINGS));
    app.press(Action::Convert(C::Show));
    assert_eq!(convert::shown_parts(&app), None);
    feed(&mut app);
    app.press(Action::Convert(C::Make));
    settle(&mut app);
    app.press(Action::Convert(C::Show));
    assert!(convert::shown_parts(&app).is_some());
}

/// In Practice Seed XOR is dimmed and makes nothing.
#[test]
fn practice_does_not_make_seed_xor() {
    let mut app = testkit::started();
    testkit::into_demo(&mut app);
    let c = testkit::compute();
    let (word, count) = testkit::SAMPLE_WORDS[0];
    let words = vec![word; usize::from(count)].join(" ");
    app.session
        .keys
        .add_words(&c, &words, "Sample", None)
        .unwrap();
    open(&mut app);
    feed(&mut app);
    app.press(Action::Convert(C::To(place("Seed XOR parts"))));
    let s = convert::state(&app).unwrap();
    assert_eq!((s.target, s.open), (None, Some(cstep::TO)));
}
