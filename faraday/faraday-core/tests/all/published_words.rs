//! Words that are published, typed on a real device on Mainnet: the
//! wallet is not shown before a caution says anyone can spend from it,
//! and the person removes the key or keeps it for testing.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use faraday_bip::keys::Network;
use faraday_core::seeds::read::Choice;
use faraday_core::spend_backup::SpendBackupAction as F;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, Event, Key};

/// Words that are published but are none of Faraday's samples: Mainnet
/// cautions and lets the person keep them for testing (BIP-39's vector).
const ABANDON: &str = "legal winner thank year wave sausage worth useful legal winner thank yellow";

/// BIP-39's first vector, also one of Faraday's public samples: Mainnet
/// refuses it and offers only Remove.
const SAMPLE: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// Twelve words nobody published: one made from entropy of no vector.
fn fresh_words() -> String {
    use faraday_bip::bip39::{Language, Mnemonic};
    let m = Mnemonic::from_entropy(
        Language::English,
        &[
            0x5a, 0x17, 0xc3, 0x09, 0x71, 0xee, 0x2b, 0x84, 0x30, 0xd5, 0x6f, 0x12, 0xa8, 0x4c,
            0x93, 0xfb,
        ],
    )
    .unwrap();
    let words: Vec<String> = m
        .indices()
        .iter()
        .map(|&i| {
            Language::English
                .typed(i)
                .unwrap()
                .as_chars()
                .iter()
                .collect()
        })
        .collect();
    words.join(" ")
}

/// The app on Mainnet, where the cautions are: a test starts on the test
/// wallets' network.
fn mainnet() -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Mainnet).unwrap();
    app
}

fn typed(app: &mut Faraday, words: &str) {
    app.press(Action::Entry(None));
    for c in words.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

#[test]
fn the_abandon_words_on_mainnet_bring_the_caution_before_anything_else() {
    let mut app = mainnet();
    typed(&mut app, ABANDON);
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    assert_eq!(app.sheet, Some(Sheet::Published));
}

#[test]
fn typing_a_codex32_vector_on_mainnet_brings_the_caution() {
    let mut app = mainnet();
    app.press(Action::Entry(None));
    let codex32 = faraday_core::forms::Form::ALL
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Codex32)
        .unwrap() as u8;
    app.press(Action::EntryForm(codex32));
    // BIP-93's vector 4, a 256-bit master seed.
    for c in "ms10leetsllhdmn9m42vcsamx24zrxgs3qrl7ahwvhw4fnzrhve25gvezzyqqtum9pgv99ycma".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Enter));
    testkit::pause(&mut app);
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    assert_eq!(app.sheet, Some(Sheet::Published));
}

#[test]
fn typing_a_bip85_vectors_child_words_on_mainnet_brings_the_caution() {
    let mut app = mainnet();
    typed(
        &mut app,
        "girl mad pet galaxy egg matter matrix prison refuse sense ordinary nose",
    );
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    assert_eq!(app.sheet, Some(Sheet::Published));
}

/// A codex32 string typed on the Codex32 form of Entry, as a person does.
fn typed_codex32(app: &mut Faraday, string: &str) {
    app.press(Action::Entry(None));
    let codex32 = faraday_core::forms::Form::ALL
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Codex32)
        .unwrap() as u8;
    app.press(Action::EntryForm(codex32));
    for c in string.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Enter));
    testkit::pause(app);
}

#[test]
fn typing_a_bip85_vectors_hex_or_a_bip32_vectors_seed_as_codex32_on_mainnet_brings_the_caution() {
    use faraday_bip::codex32::Codex32;
    for seed in [
        // BIP-32's test vector 1, 16 bytes.
        "000102030405060708090a0b0c0d0e0f",
        // BIP-85's hex vector, 64 bytes.
        "492db4698cf3b73a5a24998aa3e9d7fa96275d85724a91e71aa2d645442f878555d078fd1f1f67e368976f04137b1f7a0d19232136ca50c44614af72b5582a5c",
    ] {
        let bytes: Vec<u8> = (0..seed.len() / 2)
            .map(|i| u8::from_str_radix(&seed[2 * i..2 * i + 2], 16).unwrap())
            .collect();
        let string = Codex32::from_seed(0, *b"test", &bytes).unwrap().encode();
        let mut app = mainnet();
        typed_codex32(&mut app, string.as_str());
        assert_eq!(
            app.session.keys.len(),
            1,
            "{seed}: {:?}",
            testkit::entry(&app).error
        );
        let _ = app.frame();
        assert_eq!(app.sheet, Some(Sheet::Published), "{seed}");
    }
}

#[test]
fn a_bip85_child_of_a_published_key_kept_for_testing_brings_the_caution_too() {
    let mut app = mainnet();
    typed(&mut app, ABANDON);
    app.press(Action::PublishedKeep);
    assert_eq!(app.sheet, None);
    app.press(Action::Bip85);
    app.press(Action::PApp(0));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::PLoad);
    app.press(Action::PRecord(false));
    assert_eq!(app.session.keys.len(), 2);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Published));
}

#[test]
fn a_bip85_child_of_a_key_nobody_published_brings_no_caution() {
    let mut app = mainnet();
    typed(&mut app, &fresh_words());
    app.press(Action::Bip85);
    app.press(Action::PApp(0));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::PLoad);
    app.press(Action::PRecord(false));
    assert_eq!(app.session.keys.len(), 2);
    let _ = app.frame();
    assert_eq!(app.sheet, None);
}

#[test]
fn remove_drops_the_key_and_keep_for_testing_leaves_it_alone() {
    let mut app = mainnet();
    typed(&mut app, ABANDON);
    app.press(Action::PublishedRemove);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 0);

    typed(&mut app, ABANDON);
    assert_eq!(app.sheet, Some(Sheet::Published));
    app.press(Action::PublishedKeep);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 1);
    app.press(Action::Entry(None));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None, "the caution is not asked again");
}

#[test]
fn a_fresh_key_and_a_test_network_bring_no_caution() {
    let mut app = mainnet();
    typed(&mut app, &fresh_words());
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    assert_eq!(app.sheet, None);

    let mut test = mainnet();
    testkit::move_to(&mut test.session, Network::Testnet).unwrap();
    typed(&mut test, ABANDON);
    assert_eq!(test.session.keys.len(), 1);
    assert_eq!(test.sheet, None);
}

const ABANDON_KEY: &str = "b8688df1";

fn drawn(app: &mut Faraday) -> String {
    app.drawn_texts().join("\n")
}

#[test]
fn removing_a_key_typed_into_restore_takes_it_out_of_restore() {
    let mut app = mainnet();
    app.press(Action::RestoreWallet);
    app.press(Action::RSlotWords);
    crate::common::type_text(&mut app, ABANDON);
    app.press(Action::EntryAdd);
    assert_eq!(app.sheet, Some(Sheet::Published));
    assert!(
        app.restore()
            .and_then(|r| r.seeds.as_ref())
            .is_some_and(|s| !s.keys.is_empty()),
        "Restore took no seed"
    );
    app.press(Action::PublishedRemove);
    assert_eq!(app.session.keys.len(), 0);
    assert_eq!(app.screen, Screen::Restore);
    let text = drawn(&mut app);
    assert!(!text.contains(ABANDON_KEY), "{text}");
    assert!(
        app.restore()
            .and_then(|r| r.seeds.as_ref())
            .is_none_or(|s| s.keys.is_empty())
    );
}

/// Spend from a backup with no description: the words typed, the guess
/// begun, then the passphrase asked for and typed. A wallet is made from
/// the key with the passphrase as it is added.
fn spend_from_backup_with_pass(app: &mut Faraday, words: &str, pass: &str, keep: bool) {
    app.press(Action::Nav(Screen::SpendBackup));
    typed(app, words);
    if keep {
        assert_eq!(app.sheet, Some(Sheet::Published));
        app.press(Action::PublishedKeep);
    }
    assert_eq!(app.sheet, None);
    app.press(Action::SpendBackup(F::Everything));
    app.press(Action::SpendBackup(F::Passphrase(true)));
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::SeedRead(Choice::AddPass));
    app.press(Action::SeedRead(Choice::Field(0)));
    crate::common::type_text(app, pass);
    app.press(Action::SeedRead(Choice::Field(1)));
    crate::common::type_text(app, pass);
    testkit::pause(app);
    app.press(Action::SeedRead(Choice::WithPass));
}

/// What is drawn with the Check the money card open.
fn check_the_money_drawn(app: &mut Faraday) -> String {
    use faraday_core::spend_backup::{Open, page};
    if app.spend_backup().open != Some(Open::Page(page::CHECK)) {
        app.press(Action::SpendBackup(F::Card(page::CHECK)));
    }
    assert_eq!(app.spend_backup().open, Some(Open::Page(page::CHECK)));
    drawn(app)
}

#[test]
fn removing_the_key_a_spend_from_a_backup_guess_made_leaves_no_wallet_of_it() {
    let mut app = mainnet();
    spend_from_backup_with_pass(&mut app, ABANDON, "TREZOR", true);
    assert_eq!(app.sheet, Some(Sheet::Published));
    assert!(
        !app.session.wallets.is_empty(),
        "the passphrase made no wallet"
    );
    let text = check_the_money_drawn(&mut app);
    assert!(!text.contains("bc1"), "{text}");
    app.press(Action::PublishedRemove);
    // The words stay, kept; the key their passphrase made goes with its wallet.
    assert_eq!(app.session.keys.len(), 1);
    assert!(app.session.wallets.is_empty());
    assert_eq!(app.spend_backup_wallet(), None);
    // The column is not left on an empty Check the money, and the typed
    // words are listed again.
    assert_ne!(
        app.spend_backup().open,
        Some(faraday_core::spend_backup::Open::Page(
            faraday_core::spend_backup::page::CHECK
        ))
    );
    assert!(
        app.spend_backup()
            .seeds
            .keys
            .contains(&app.session.keys[0].fingerprint.0),
        "the typed words are not listed"
    );
}

#[test]
fn removing_the_key_made_in_create_empties_its_slot() {
    let mut app = mainnet();
    typed(&mut app, ABANDON);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::PublishedKeep);
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CPassOpen(0));
    crate::common::type_text(&mut app, "TREZOR");
    app.press(Action::CPassField(1));
    crate::common::type_text(&mut app, "TREZOR");
    app.press(Action::PassEye);
    app.press(Action::CPassLock);
    assert_eq!(app.sheet, Some(Sheet::Published));
    app.press(Action::PublishedRemove);
    assert_eq!(
        app.create().unwrap().slots[0],
        faraday_core::create::Source::Empty
    );
}

#[test]
fn a_wallet_of_a_key_nobody_published_shows_its_addresses() {
    let mut app = mainnet();
    spend_from_backup_with_pass(&mut app, &fresh_words(), "TREZOR", false);
    assert_eq!(app.sheet, None);
    assert!(!app.session.wallets.is_empty());
    let text = check_the_money_drawn(&mut app);
    assert!(text.contains("bc1"), "{text}");
}

#[test]
fn practice_brings_no_caution_for_a_handout_words() {
    let mut app = mainnet();
    testkit::into_demo(&mut app);
    testkit::move_to(&mut app.session, Network::Mainnet).unwrap();
    typed(&mut app, &["twelve"; 12].join(" "));
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    assert_eq!(app.sheet, None);
}

#[test]
fn a_public_sample_typed_on_mainnet_can_only_be_removed() {
    let mut app = mainnet();
    typed(&mut app, SAMPLE);
    assert_eq!(app.sheet, Some(Sheet::Published));
    app.press(Action::PublishedKeep);
    assert_eq!(app.sheet, Some(Sheet::Published), "it cannot be kept");
    assert_eq!(app.session.keys.len(), 1);
    app.press(Action::PublishedRemove);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 0);
}

#[test]
fn a_public_sample_typed_on_testnet_is_taken() {
    let mut app = mainnet();
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    typed(&mut app, SAMPLE);
    let _ = app.frame();
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 1);
}
