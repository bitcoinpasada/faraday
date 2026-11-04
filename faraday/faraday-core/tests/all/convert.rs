//! Convert a seed: a loaded key reaches its master seed and its xprv by
//! BIP-39's and BIP-32's published vectors, a row not offered takes no
//! press, and nothing stays once the flow is left.

use faraday_core::convert::{self, ConvertAction as C, ROWS, cstep};
use faraday_core::wallet::Origin;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, Event, Key};

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
    app.press(Action::Convert(C::Save));
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
    app.press(Action::Convert(C::Save));
    app.press(Action::SecretUnprotected);
    assert_eq!(app.outbox.len(), waiting, "not before the hold");
    app.press(Action::SecretHold);
    let item = app.outbox.last().expect("the file is written");
    assert_eq!(&item.bytes[..], ABOUT_XPRV.as_bytes());
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
    app.press(Action::Convert(C::Save));
    assert_eq!(
        testkit::secret_out(&app).map(|o| o.bytes.to_vec()),
        Some(value.into_bytes())
    );
}
