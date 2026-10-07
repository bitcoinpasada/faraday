//! The vanity address grinder through its screens (`docs/PLANNING.md`
//! §16.117): the key page's row, the two dials, the prefix on the
//! address keyboard, the run, and what "Use it" does with what it
//! found.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_ui::widgets::keyboard::KeyInput;

/// The dials, in the order the Choice lists them.
const PASSPHRASE: usize = 0;
const ACCOUNT: usize = 1;

/// Native SegWit, in `ScriptType::ALL` order.
const SEGWIT: usize = 2;

/// A prefix of one free character past `bc1q`, which the first handful
/// of candidates reaches on either dial.
const PREFIX: &str = "bc1qq";

fn device() -> Harness {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h
}

fn words() -> Mnemonic {
    Mnemonic::parse(Language::English, &ABANDON.join(" ")).expect("the test words")
}

/// The key's page › Vanity address › dial › Continue › SegWit ›
/// Continue › the prefix › ✓, then ticks until the run ends.
fn grind(h: &mut Harness, dial: usize) {
    h.open_key(0);
    h.tap(ids::DETAIL_VANITY);
    h.choose(ids::at(ids::PICK_BASE, dial), ids::PICK_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Vanity);
    h.choose(
        ids::at(ids::VANITY_SCRIPT_BASE, SEGWIT),
        ids::VANITY_SCRIPT_CONTINUE,
    );
    for c in PREFIX.chars().skip("bc1q".len()) {
        h.pad(ids::VANITY_KEYBOARD, KeyInput::Char(c));
    }
    h.pad(ids::VANITY_KEYBOARD, KeyInput::Done);
    let found = strings::EN.vanity_found_title;
    for _ in 0..200 {
        if h.app.texts().iter().any(|t| t == found) {
            return;
        }
        h.tick(16);
    }
    panic!("the grind found nothing: {:?}", h.app.texts());
}

/// The address the found key or wallet pays to first, which is what the
/// person asked for.
fn first_address(h: &Harness, key: usize) -> String {
    h.app
        .addresses(key, ScriptType::NativeSegwit, false, 1)
        .first()
        .cloned()
        .expect("an address")
}

#[test]
fn a_loaded_key_offers_the_grinder_and_both_its_dials() {
    let s = &strings::EN;
    let mut h = device();
    h.open_key(0);
    assert!(
        h.app.texts().iter().any(|t| t == s.key_vanity_row),
        "the Vanity address row: {:?}",
        h.app.texts()
    );
    h.tap(ids::DETAIL_VANITY);
    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);
    assert!(says(s.vanity_how_title), "the question: {texts:?}");
    assert!(says(s.vanity_how_passphrase), "the first dial: {texts:?}");
    assert!(says(s.vanity_how_account), "the second dial: {texts:?}");
}

#[test]
fn the_keyboard_refuses_a_character_no_address_of_the_kind_carries() {
    let mut h = device();
    h.open_key(0);
    h.tap(ids::DETAIL_VANITY);
    h.choose(ids::at(ids::PICK_BASE, ACCOUNT), ids::PICK_CONTINUE);
    h.choose(
        ids::at(ids::VANITY_SCRIPT_BASE, SEGWIT),
        ids::VANITY_SCRIPT_CONTINUE,
    );
    for refused in ['b', 'i', 'o', '1'] {
        assert!(
            h.app
                .key_rect(ids::VANITY_KEYBOARD, KeyInput::Char(refused))
                .is_none(),
            "{refused} is offered after bc1q"
        );
    }
    for offered in ['q', 'z', '9'] {
        assert!(
            h.app
                .key_rect(ids::VANITY_KEYBOARD, KeyInput::Char(offered))
                .is_some(),
            "{offered} is not offered after bc1q"
        );
    }
}

#[test]
fn the_passphrase_dial_finds_an_address_and_use_it_opens_that_key() {
    let mut h = device();
    grind(&mut h, PASSPHRASE);
    h.tap(ids::VANITY_USE);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    assert_eq!(h.app.fingerprints().len(), 2, "the opened key was added");
    assert!(
        first_address(&h, 1).starts_with(PREFIX),
        "the opened key pays to {}",
        first_address(&h, 1)
    );

    // The key that was opened is the words with the counter the grind
    // named, and nothing else: the same fingerprint the engine's own
    // counter reaches.
    let find = osk_bip::vanity::grind(
        &osk_bip::vanity::Key::Words(&words()),
        &osk_bip::vanity::Method::Passphrase { base: b"" },
        ScriptType::NativeSegwit,
        PREFIX,
        Network::Mainnet,
        0,
        1000,
    )
    .find
    .expect("a find");
    let seed = words().to_seed(find.suffix.as_bytes()).expect("a seed");
    let expected = MasterKey::from_seed(&seed, Network::Mainnet).fingerprint();
    assert_eq!(h.app.fingerprints()[1], expected);
}

#[test]
fn the_account_dial_finds_an_address_and_use_it_adds_that_wallet() {
    let mut h = device();
    grind(&mut h, ACCOUNT);
    assert_eq!(h.app.wallet_count(), 0, "no wallet before Use it");
    h.tap(ids::VANITY_USE);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    assert_eq!(h.app.wallet_count(), 1);
    let descriptor = h.app.wallet_policy(0).expect("the wallet").to_descriptor();
    assert!(
        descriptor.starts_with("wpkh("),
        "a single-sig SegWit wallet: {descriptor}"
    );

    // The wallet is at the account the grind found, so its first
    // address is the one the Result stated.
    let find = osk_bip::vanity::grind(
        &osk_bip::vanity::Key::Master(&MasterKey::from_seed(
            &words().to_seed(b"").expect("a seed"),
            Network::Mainnet,
        )),
        &osk_bip::vanity::Method::Account,
        ScriptType::NativeSegwit,
        PREFIX,
        Network::Mainnet,
        0,
        1000,
    )
    .find
    .expect("a find");
    assert!(
        descriptor.contains(&format!("/{}']", find.account)),
        "the account the grind found: {descriptor}"
    );
}

#[test]
fn leaving_the_run_discards_what_it_found() {
    let mut h = device();
    grind(&mut h, ACCOUNT);
    h.tap(ids::BACK);
    let texts = h.app.texts();
    assert!(
        !texts.iter().any(|t| t == strings::EN.vanity_found_title),
        "the find is still on screen: {texts:?}"
    );
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    assert_eq!(h.app.wallet_count(), 0);
    assert_eq!(h.app.fingerprints().len(), 1);
}

#[test]
fn a_key_that_carries_a_passphrase_cannot_have_a_counter_appended() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(Some("correct horse"));
    h.open_key(0);
    h.tap(ids::DETAIL_VANITY);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.vanity_has_passphrase),
        "the reason the dial is dimmed: {texts:?}"
    );
}
