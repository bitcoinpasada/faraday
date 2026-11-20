//! My seed makes my wallet: what the seed makes at each account, against
//! the published test accounts (BIP-49, BIP-84, BIP-86), BIP-39's vectors
//! for the fingerprints and BIP-32's for a path typed by hand; and the
//! seed held for the check alone (kept as Add a key keeps a key, refused
//! with a stick in, gone when dropped).

#![expect(
    clippy::disallowed_types,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use faraday_bip::bitcoin::bip32::Xpriv;
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_core::seed_check::{ACCOUNTS, Account, Source, make};
use faraday_core::strings::EN;
use faraday_core::testkit::compute;
use faraday_keys::Keys;
use serde_json::Value;
use std::str::FromStr;

const ADDRESSES: &str = include_str!("../../../../tools/vectors/addresses/vectors.json");
const BIP39: &str = include_str!("../../../../tools/vectors/bip39/vectors.json");
const BIP32: &str = include_str!("../../../../tools/vectors/bip32/vectors.json");

const CORE: &str = include_str!("../../../../tools/vectors/seed-check/vectors.json");

const ABOUT: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// The master fingerprint of the extended private key a BIP-39 vector lists.
fn fingerprint_of(c: &faraday_crypto::work::Compute, xprv: &str) -> String {
    let key = MasterKey::from_xpriv(c, Xpriv::from_str(xprv).unwrap()).unwrap();
    key.fingerprint().to_string()
}

fn script_for(purpose: u64) -> ScriptType {
    match purpose {
        84 => ScriptType::NativeSegwit,
        86 => ScriptType::Taproot,
        49 => ScriptType::NestedSegwit,
        _ => ScriptType::Legacy,
    }
}

#[test]
fn each_published_account_gives_its_xpub_and_first_addresses() {
    let c = compute();
    let v: Value = serde_json::from_str(ADDRESSES).unwrap();
    for a in v["accounts"].as_array().unwrap() {
        let purpose = a["purpose"].as_u64().unwrap();
        let network = if a["network"] == "testnet" {
            Network::Testnet
        } else {
            Network::Mainnet
        };
        let mut keys = Keys::on(network);
        keys.check_read(&c, a["mnemonic"].as_str().unwrap(), "")
            .unwrap();
        let coin = u32::from(network != Network::Mainnet);
        let account = ACCOUNTS
            .iter()
            .find(|r| r.bip() == format!("BIP-{purpose}"))
            .unwrap();
        let made = make(
            &c,
            &keys,
            Source::Held,
            &account.path(coin),
            script_for(purpose),
            network,
        )
        .unwrap();
        let xpub = a["xpub"].as_str().unwrap();
        let shown = made.spelled.as_ref().unwrap_or(&made.plain);
        assert_eq!(**shown, *xpub, "BIP-{purpose} xpub");
        for x in a["addresses"].as_array().unwrap() {
            if x["index"] != 0 {
                continue;
            }
            let want = x["address"].as_str().unwrap();
            let got = if x["change"] == 0 {
                &made.receive
            } else {
                &made.change
            };
            assert_eq!(got.as_deref(), Some(want), "BIP-{purpose}");
        }
    }
}

#[test]
fn bip44_and_bip48_give_what_bitcoin_core_makes_of_the_same_phrase() {
    let c = compute();
    let v: Value = serde_json::from_str(CORE).unwrap();
    let mut keys = Keys::on(Network::Mainnet);
    keys.check_read(&c, v["phrase"].as_str().unwrap(), "")
        .unwrap();
    let legacy = make(
        &c,
        &keys,
        Source::Held,
        &Account::Legacy.path(0),
        ScriptType::Legacy,
        Network::Mainnet,
    )
    .unwrap();
    assert_eq!(*legacy.plain, *v["bip44"]["xpub"].as_str().unwrap());
    assert_eq!(
        legacy.receive.as_deref(),
        v["bip44"]["receive"].as_str(),
        "BIP-44's first receive address"
    );
    assert_eq!(
        legacy.change.as_deref(),
        v["bip44"]["change"].as_str(),
        "BIP-44's first change address"
    );
    let cosigner = make(
        &c,
        &keys,
        Source::Held,
        &Account::Multisig.path(0),
        ScriptType::NativeSegwit,
        Network::Mainnet,
    )
    .unwrap();
    assert_eq!(*cosigner.plain, *v["bip48"]["xpub"].as_str().unwrap());
}

#[test]
fn legacy_gives_the_widely_published_first_address() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.check_read(&c, ABOUT, "").unwrap();
    let made = make(
        &c,
        &keys,
        Source::Held,
        &Account::Legacy.path(0),
        ScriptType::Legacy,
        Network::Mainnet,
    )
    .unwrap();
    assert_eq!(made.script, "Legacy");
    assert!(made.spelled.is_none(), "BIP-44 has the xpub alone");
    assert_eq!(
        made.receive.as_deref(),
        Some("1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA")
    );
}

#[test]
fn a_multisig_account_gives_its_cosigner_key_and_no_addresses() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.check_read(&c, ABOUT, "").unwrap();
    let made = make(
        &c,
        &keys,
        Source::Held,
        &Account::Multisig.path(0),
        ScriptType::NativeSegwit,
        Network::Mainnet,
    )
    .unwrap();
    assert_eq!(made.script, "Multisig · Native SegWit");
    assert!(made.receive.is_none() && made.change.is_none());
    assert!(made.spelled.as_ref().unwrap().starts_with("Zpub"));
    assert!(made.origin.ends_with("/48h/0h/0h/2h]"));
}

#[test]
fn the_fingerprints_with_and_without_the_passphrase_are_bip39s() {
    let c = compute();
    // The published master fingerprint of the phrase without a passphrase.
    let mut about = Keys::on(Network::Mainnet);
    let f = about.check_read(&c, ABOUT, "").unwrap();
    assert_eq!(f.fingerprint.to_string(), "73c5da0a");
    let v: Value = serde_json::from_str(BIP39).unwrap();
    let want = |xprv: &str| fingerprint_of(&c, xprv);
    for row in v["english"].as_array().unwrap() {
        let words = row[1].as_str().unwrap();
        let mut keys = Keys::on(Network::Mainnet);
        let facts = keys.check_read(&c, words, "TREZOR").unwrap();
        assert_eq!(
            facts.fingerprint.to_string(),
            want(row[3].as_str().unwrap())
        );
        let none = keys.check_read(&c, words, "").unwrap();
        assert_eq!(facts.without, Some(none.fingerprint));
        assert_eq!(none.without, None);
    }
}

#[test]
fn a_path_typed_by_hand_gives_bip32s_xpub() {
    let c = compute();
    let v: Value = serde_json::from_str(BIP32).unwrap();
    for case in v.as_array().unwrap() {
        let seed: Vec<u8> = (0..case["seed"].as_str().unwrap().len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&case["seed"].as_str().unwrap()[i..i + 2], 16).unwrap())
            .collect();
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_seed(&c, &seed, "").unwrap();
        for chain in case["chains"].as_array().unwrap() {
            let path = chain["path"].as_str().unwrap();
            if path == "m" {
                continue;
            }
            let made = make(
                &c,
                &keys,
                Source::Loaded(0),
                path,
                ScriptType::Legacy,
                Network::Mainnet,
            )
            .unwrap();
            assert_eq!(*made.plain, *chain["xpub"].as_str().unwrap(), "{path}");
        }
    }
}

#[test]
fn a_path_that_is_not_one_is_refused_with_the_reader_s_line() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.check_read(&c, ABOUT, "").unwrap();
    for bad in ["84h//0h", "84h/x", "84h/0h/"] {
        assert!(
            make(
                &c,
                &keys,
                Source::Held,
                bad,
                ScriptType::NativeSegwit,
                Network::Mainnet
            )
            .is_err(),
            "{bad}"
        );
    }
}

#[test]
fn keep_this_key_loads_it_as_add_a_key_does_and_drop_leaves_none() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    let facts = keys.check_read(&c, ABOUT, "TREZOR").unwrap();
    assert!(
        keys.is_empty(),
        "held for the check alone, not among the keys"
    );
    assert!(keys.holds_secret());
    let kept = keys.check_keep(&c, "").unwrap();
    assert_eq!(kept, facts.fingerprint);
    assert!(keys.check_facts().is_none());
    let info = keys.all().next().unwrap();
    assert!(info.has_passphrase && info.has_words());
    // Dropped: nothing held.
    let mut other = Keys::on(Network::Mainnet);
    other.check_read(&c, ABOUT, "").unwrap();
    other.check_forget();
    assert!(other.check_facts().is_none() && !other.holds_secret());
    // A lock wipes it with the session's keys.
    other.check_read(&c, ABOUT, "").unwrap();
    other.clear();
    assert!(other.check_facts().is_none());
}

#[test]
fn keep_this_key_is_refused_with_a_stick_in_and_for_a_key_loaded() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.check_read(&c, ABOUT, "").unwrap();
    keys.set_stick_attached(Some("stick"));
    assert!(keys.check_keep(&c, "").is_err());
    assert!(keys.check_facts().is_some(), "the check still holds it");
    keys.set_stick_attached(None);
    keys.check_keep(&c, "").unwrap();
    keys.check_read(&c, ABOUT, "").unwrap();
    assert!(keys.check_keep(&c, "").is_err(), "not loaded twice");
}

#[test]
fn reading_the_seed_is_refused_with_a_stick_in() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.set_stick_attached(Some("stick"));
    assert!(keys.check_read(&c, ABOUT, "").is_err());
    assert!(!keys.holds_secret());
}

// ---------------------------------------------------------------------
// The flow, from the Tools tile
// ---------------------------------------------------------------------

use crate::common::texts;
use faraday_core::seed_check::SeedAction as S;
use faraday_core::{Action, ChoicesPress, Faraday, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, Event, Key};

/// The BIP-84 test account of `ADDRESSES`: its first receive address and
/// extended public key.
fn bip84() -> (String, String) {
    let v: Value = serde_json::from_str(ADDRESSES).unwrap();
    let a = v["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["purpose"] == 84 && a["network"] != "testnet")
        .unwrap();
    let first = a["addresses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["change"] == 0 && x["index"] == 0)
        .unwrap();
    (
        first["address"].as_str().unwrap().to_string(),
        a["xpub"].as_str().unwrap().to_string(),
    )
}

/// Whether the screen shows `want`, whatever the spaces between groups.
fn shows(app: &mut Faraday, want: &str) -> bool {
    let all: String = texts(app).concat().split_whitespace().collect();
    all.contains(want)
}

fn type_chars(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// The tile pressed, the words typed and taken.
fn with_words(words: &str) -> Faraday {
    with_words_on(Network::Mainnet, words)
}

/// [`with_words`] on `network`: the checks read Mainnet's published
/// vectors, and reading a seed refuses nothing, but keeping a public sample
/// is refused on Mainnet, so the tests that keep go to the test network.
fn with_words_on(network: Network, words: &str) -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, network).unwrap();
    app.press(Action::Nav(faraday_core::Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "My seed makes my wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::SeedCheck(S::Type));
    testkit::type_words(&mut app, words);
    app.press(Action::SeedCheck(S::Words));
    app
}

#[test]
fn the_words_and_an_account_give_the_fingerprint_and_the_first_address() {
    let (address, xpub) = bip84();
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    assert!(shows(&mut app, &address), "the first receive address");
    assert!(shows(&mut app, &xpub), "the account's xpub");
    // The BIP-39 vector's fingerprint, with its passphrase typed twice.
    let v: Value = serde_json::from_str(BIP39).unwrap();
    let row = &v["english"].as_array().unwrap()[0];
    let want = fingerprint_of(&compute(), row[3].as_str().unwrap());
    let mut app = with_words(row[1].as_str().unwrap());
    app.press(Action::SeedCheck(S::AddPass));
    app.press(Action::SeedCheck(S::PassField(0)));
    type_chars(&mut app, "TREZOR");
    app.press(Action::SeedCheck(S::PassField(1)));
    type_chars(&mut app, "TREZOR");
    app.press(Action::PassEye);
    app.press(Action::SeedCheck(S::UsePass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    assert!(
        shows(&mut app, &want),
        "the fingerprint with the passphrase"
    );
}

/// The line that says keys are read as a US keyboard, as `shows` reads text
/// (its words run together).
const US_LINE: &str = "KeysarereadasaUSkeyboard.CapsLockisignored.";

/// The frame once the step's fade through has finished.
fn at_rest(app: &mut Faraday) {
    for i in 1..=30u64 {
        app.event(Event::Tick {
            now_ms: 100_000 + i * 16,
        });
        let _ = app.frame();
    }
}

#[test]
fn a_passphrase_typed_with_hardware_keys_in_the_seed_check_says_so_and_waits_for_the_eye() {
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::AddPass));
    app.press(Action::SeedCheck(S::PassField(0)));
    assert!(!shows(&mut app, US_LINE));
    type_chars(&mut app, "TREZOR");
    app.press(Action::SeedCheck(S::PassField(1)));
    type_chars(&mut app, "TREZOR");
    assert!(shows(&mut app, US_LINE));
    app.press(Action::SeedCheck(S::UsePass));
    assert!(
        !app.offers(Action::SeedCheck(S::Pick(Account::NativeSegwit))),
        "the passphrase waits for the eye"
    );
    app.press(Action::PassEye);
    app.press(Action::SeedCheck(S::UsePass));
    testkit::settle(&mut app);
    at_rest(&mut app);
    assert!(app.offers(Action::SeedCheck(S::Pick(Account::NativeSegwit))));
}

#[test]
fn a_passphrase_typed_on_the_on_screen_keyboard_in_the_seed_check_has_no_line_and_no_wait() {
    use faraday_ui::keyboard::OskPress;
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::AddPass));
    for field in [0, 1] {
        app.press(Action::SeedCheck(S::PassField(field)));
        for c in "one".chars() {
            app.press(Action::Osk(OskPress::Char(c)));
        }
    }
    assert!(!shows(&mut app, US_LINE));
    app.press(Action::SeedCheck(S::UsePass));
    testkit::settle(&mut app);
    at_rest(&mut app);
    assert!(app.offers(Action::SeedCheck(S::Pick(Account::NativeSegwit))));
}

#[test]
fn another_account_of_the_same_key_is_picked_from_the_result() {
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    app.press(Action::SeedCheck(S::Another));
    app.press(Action::SeedCheck(S::Pick(Account::Legacy)));
    let v: Value = serde_json::from_str(CORE).unwrap();
    assert!(
        shows(&mut app, v["bip44"]["receive"].as_str().unwrap()),
        "BIP-44's first address"
    );
    assert!(
        shows(&mut app, v["bip44"]["xpub"].as_str().unwrap()),
        "BIP-44's xpub"
    );
    assert!(app.session.keys.check_facts().is_some());
}

#[test]
fn done_cancel_and_a_lock_each_leave_no_key() {
    for end in 0..3 {
        let mut app = with_words(ABOUT);
        app.press(Action::SeedCheck(S::NoPass));
        assert!(app.session.keys.check_facts().is_some(), "held at Account");
        match end {
            0 => {
                app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
                app.press(Action::SeedCheck(S::Done));
            }
            1 => app.press(Action::SeedCheck(S::Cancel)),
            _ => {
                app.press(Action::Lock);
            }
        }
        let _ = app.frame();
        assert!(app.session.keys.check_facts().is_none(), "way out {end}");
        assert!(!app.session.keys.holds_secret(), "way out {end}");
    }
}

#[test]
fn keep_this_key_loads_the_key_and_is_refused_with_a_stick_in() {
    let mut app = with_words_on(testkit::net(), ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    app.press(Action::SeedCheck(S::Keep));
    assert_eq!(
        app.session.keys.all().count(),
        1,
        "loaded as Add a key does"
    );
    let info = app.session.keys.all().next().unwrap();
    assert!(info.has_words() && !info.has_passphrase);

    let mut app = with_words_on(testkit::net(), ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    app.storage(StorageEvent::Sticks(vec![faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    app.press(Action::SeedCheck(S::Keep));
    assert_eq!(app.session.keys.all().count(), 0, "refused with a stick in");
    assert!(
        app.session.keys.check_facts().is_some(),
        "the check still holds its key"
    );
}

#[test]
fn back_and_forward_reopen_the_steps_as_they_were() {
    let (address, _) = bip84();
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    assert!(shows(&mut app, &address));
    app.event(Event::Back);
    app.event(Event::Forward);
    assert!(shows(&mut app, &address), "the same Result again");
    app.event(Event::Back);
    app.event(Event::Back);
    assert!(
        app.session.keys.check_facts().is_some(),
        "Passphrase keeps the key"
    );
    app.event(Event::Forward);
    app.event(Event::Forward);
    assert!(shows(&mut app, &address), "and the Result after it");
}

/// A stick comes in.
fn stick_in(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(vec![faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
}

#[test]
fn a_passphrase_taken_after_back_replaces_the_result_of_the_key_before() {
    let (address, _) = bip84();
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    assert!(shows(&mut app, &address));
    let before = app.session.keys.check_facts().unwrap();
    app.event(Event::Back);
    app.event(Event::Back);
    app.press(Action::SeedCheck(S::AddPass));
    type_chars(&mut app, "secret");
    app.press(Action::SeedCheck(S::PassField(1)));
    type_chars(&mut app, "secret");
    app.press(Action::PassEye);
    app.press(Action::SeedCheck(S::UsePass));
    assert_ne!(app.session.keys.check_facts().unwrap(), before);
    assert!(!shows(&mut app, &address), "the old key's address is gone");
    // The Result header does not reopen the old key's Result.
    app.press(Action::SeedCheck(S::Step(3)));
    assert!(!shows(&mut app, &address), "and no Result stands for it");
    let old_fp = faraday_core::wallet::fp_text(before.fingerprint);
    assert!(!shows(&mut app, old_fp.as_str()), "nor its fingerprint");
    app.press(Action::SeedCheck(S::Keep));
    assert_eq!(app.session.keys.all().count(), 0, "nothing is kept");
}

#[test]
fn enter_in_the_passphrase_field_is_drawn_first_with_a_working_line() {
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::AddPass));
    type_chars(&mut app, "secret");
    app.press(Action::SeedCheck(S::PassField(1)));
    type_chars(&mut app, "secret");
    app.press(Action::PassEye);
    app.event(Event::Key(Key::Enter));
    assert!(
        app.session.keys.check_facts().is_none(),
        "the key waits for the frame"
    );
    let _ = app.frame();
    assert_eq!(app.toast_text(), Some("Working out the key"));
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(&mut app);
    assert!(app.session.keys.check_facts().is_some(), "then it is read");
}

#[test]
fn new_words_after_back_to_seed_leave_no_result_of_the_old_ones() {
    let (address, _) = bip84();
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    assert!(shows(&mut app, &address));
    for _ in 0..3 {
        app.event(Event::Back);
    }
    app.press(Action::SeedCheck(S::Type));
    testkit::type_words(
        &mut app,
        "legal winner thank year wave sausage worth useful legal winner thank yellow",
    );
    app.press(Action::SeedCheck(S::Words));
    assert!(!shows(&mut app, &address));
    app.press(Action::SeedCheck(S::Step(3)));
    assert!(!shows(&mut app, &address), "the Result is gone");
    app.press(Action::SeedCheck(S::Keep));
    assert_eq!(app.session.keys.all().count(), 0, "nothing is kept");
}

#[test]
fn with_a_stick_in_add_a_passphrase_is_refused_and_no_field_opens() {
    let mut app = with_words(ABOUT);
    stick_in(&mut app);
    app.press(Action::SeedCheck(S::AddPass));
    assert!(!shows(&mut app, "Passphraseagain"), "no field takes focus");
}

#[test]
fn the_passphrase_card_taken_twice_after_the_key_is_read_keeps_the_key() {
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::Step(1)));
    app.press(Action::SeedCheck(S::Step(1)));
    app.press(Action::SeedCheck(S::NoPass));
    assert!(app.session.keys.check_facts().is_some(), "the key is read");
    assert!(!faraday_core::testkit::secret_typed(&app), "no words held");
}

#[test]
fn a_refused_press_with_a_stick_in_keeps_the_passphrase_card_once_the_key_is_read() {
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Step(1)));
    assert!(shows(&mut app, "Addapassphrase"), "the card opens");
    stick_in(&mut app);
    app.press(Action::SeedCheck(S::AddPass));
    assert!(
        !shows(&mut app, "Continue") && !shows(&mut app, "12words"),
        "the Seed card does not take its place"
    );
}

#[test]
fn back_to_passphrase_for_a_loaded_key_opens_seed() {
    let mut app = with_words_on(testkit::net(), ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    app.press(Action::SeedCheck(S::Keep));
    faraday_core::testkit::keep_published(&mut app);
    app.press(Action::Nav(faraday_core::Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "My seed makes my wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.session.keys.all().count(), 1, "the key is loaded");
    app.press(Action::SeedCheck(S::Loaded(0)));
    assert!(shows(&mut app, "NativeSegWit"), "the Account card");
    app.event(Event::Back);
    assert!(
        !shows(&mut app, "Addapassphrase"),
        "Passphrase is not offered for a key with no words"
    );
    app.event(Event::Forward);
    assert!(
        shows(&mut app, "NativeSegWit"),
        "Forward returns to Account"
    );
}

#[test]
fn a_seedqr_that_is_not_a_seed_closes_the_camera() {
    let mut app = with_words(ABOUT);
    app.press(Action::ScanSeed);
    // Twelve times word 0000: a SeedQR whose checksum does not hold.
    app.event(Event::Scanned {
        bytes: "0000".repeat(12).into_bytes(),
    });
    let _ = app.frame();
    // The camera went off at the read card; what follows is its own.
    let _ = std::iter::from_fn(|| app.poll_command()).count();
    // The read card offers Add this key; the Seed card says it is no seed.
    app.press(Action::ReadGo(0));
    let _ = app.frame();
    assert_eq!(app.sheet, None, "sheet");
    let sent: Vec<_> = std::iter::from_fn(|| app.poll_command()).collect();
    assert!(
        sent.contains(&faraday_shell_api::Command::CameraOff),
        "off {sent:?}"
    );
    assert!(
        shows(&mut app, "NotaBIP-39seed"),
        "line {:?}",
        texts(&mut app)
    );
}

#[test]
fn the_master_key_is_origin_and_path_with_nothing_after_the_fingerprint() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.check_read(&c, ABOUT, "").unwrap();
    let made = make(
        &c,
        &keys,
        Source::Held,
        "m",
        ScriptType::NativeSegwit,
        Network::Mainnet,
    )
    .unwrap();
    assert_eq!(&*made.origin, "[73c5da0a]");
    assert_eq!(&*made.path, "m");
}

// ---------------------------------------------------------------------
// Compare with the wallet app's
// ---------------------------------------------------------------------

const COMPARE: &str = include_str!("../../../../tools/vectors/seed-check/compare.json");

/// Whether the screen shows `want`, the page scrolled down to it.
fn shows_down(app: &mut Faraday, want: &str) -> bool {
    for _ in 0..20 {
        if shows(app, want) {
            return true;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    false
}

/// The Result of `ABOUT` at the native segwit account, Compare opened.
fn at_compare(pass: bool) -> Faraday {
    let mut app = with_words(ABOUT);
    if pass {
        app.press(Action::SeedCheck(S::AddPass));
        app.press(Action::SeedCheck(S::PassField(0)));
        type_chars(&mut app, "TREZOR");
        app.press(Action::SeedCheck(S::PassField(1)));
        type_chars(&mut app, "TREZOR");
        app.press(Action::PassEye);
        app.press(Action::SeedCheck(S::UsePass));
    } else {
        app.press(Action::SeedCheck(S::NoPass));
    }
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    app.press(Action::SeedCheck(S::Compare));
    app
}

/// The value typed into the comparison's field and compared.
fn compare_typed(app: &mut Faraday, value: &str) {
    app.press(Action::SeedCheck(S::CompareField));
    type_chars(app, value);
    app.press(Action::SeedCheck(S::CompareGo));
    testkit::settle(app);
}

/// The published extended public key of the BIP-86 account of `ABOUT`.
fn bip86_xpub() -> String {
    let v: Value = serde_json::from_str(ADDRESSES).unwrap();
    let a = v["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["purpose"] == 86 && a["network"] != "testnet")
        .unwrap();
    a["xpub"].as_str().unwrap().to_string()
}

#[test]
fn the_wallet_apps_extended_key_of_this_account_matches() {
    let (_, zpub) = bip84();
    let mut app = at_compare(false);
    compare_typed(&mut app, &format!("[73c5da0a/84h/0h/0h]{zpub}"));
    assert!(shows_down(&mut app, "Matches"));
    assert!(!shows(&mut app, "Doesnotmatch"));
}

#[test]
fn an_address_of_the_second_chain_is_found_with_its_index() {
    let v: Value = serde_json::from_str(COMPARE).unwrap();
    let mut app = at_compare(false);
    compare_typed(&mut app, v["bip84_change_7"].as_str().unwrap());
    assert!(shows_down(&mut app, "Matches"));
    assert!(
        shows_down(&mut app, "Changeaddress7"),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_two_of_three_wallet_file_names_which_key_this_seed_is() {
    let v: Value = serde_json::from_str(COMPARE).unwrap();
    let mut app = at_compare(false);
    testkit::inbox_file_at(
        &mut app,
        0,
        "wallet.txt",
        v["wallet"].as_str().unwrap().as_bytes().to_vec(),
    );
    assert!(shows_down(&mut app, "FromFiles·wallet.txt"), "its button");
    app.press(Action::SeedCheck(S::CompareFile(0)));
    testkit::settle(&mut app);
    assert!(shows_down(&mut app, "Matches"));
    assert!(
        shows_down(&mut app, "Thisseediskey2ofthiswallet·2of3·m/48h/0h/0h/2h"),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_key_with_this_seeds_fingerprint_and_another_key_warns_though_another_key_matches() {
    let v: Value = serde_json::from_str(COMPARE).unwrap();
    // The first key carries this seed's fingerprint on a foreign key; the
    // second is this seed's own.
    let wallet = v["wallet"]
        .as_str()
        .unwrap()
        .replace("b8688df1", "73c5da0a");
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::Multisig)));
    app.press(Action::SeedCheck(S::Compare));
    testkit::inbox_file_at(&mut app, 0, "wallet.txt", wallet.into_bytes());
    app.press(Action::SeedCheck(S::CompareFile(0)));
    testkit::settle(&mut app);
    assert!(
        shows_down(&mut app, "Thefingerprintisthisseed's"),
        "{:?}",
        texts(&mut app)
    );
    assert!(!shows(&mut app, "Matches"));
}

#[test]
fn a_value_over_the_size_limit_from_files_is_refused_and_gone() {
    let v: Value = serde_json::from_str(COMPARE).unwrap();
    let mut file = v["wallet"].as_str().unwrap().to_string();
    file.push('\n');
    file.push_str(&"#".repeat(20 * 1024));
    let mut app = at_compare(false);
    testkit::inbox_file_at(&mut app, 0, "wallet.txt", file.into_bytes());
    app.press(Action::SeedCheck(S::CompareFile(0)));
    testkit::settle(&mut app);
    assert!(
        shows_down(&mut app, "Over16.4KB:nottakenhere"),
        "{:?}",
        texts(&mut app)
    );
    assert!(!shows(&mut app, "wsh(sortedmulti"), "the field is empty");
}

#[test]
fn an_edit_in_the_middle_of_a_value_with_line_breaks_keeps_the_rest_and_runs_nothing() {
    let v: Value = serde_json::from_str(COMPARE).unwrap();
    let file = v["wallet"].as_str().unwrap().replace("/*,", "/*,\n");
    let mut app = at_compare(false);
    testkit::inbox_file_at(&mut app, 0, "wallet.txt", file.into_bytes());
    app.press(Action::SeedCheck(S::CompareFile(0)));
    testkit::settle(&mut app);
    let asked = testkit::jobs_asked(&app);
    app.press(Action::SeedCheck(S::CompareField));
    for _ in 0..200 {
        app.event(Event::Key(Key::Left));
    }
    app.event(Event::Key(Key::Backspace));
    assert!(shows_down(&mut app, "28645006"), "{:?}", texts(&mut app));
    assert_eq!(testkit::jobs_asked(&app), asked, "no comparison started");
}

#[test]
fn back_while_the_comparison_runs_drops_its_answer() {
    let (_, zpub) = bip84();
    let mut app = at_compare(false);
    app.press(Action::SeedCheck(S::CompareField));
    type_chars(&mut app, &format!("[73c5da0a/84h/0h/0h]{zpub}"));
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::SeedCheck(S::CompareGo));
    app.press(Action::Back);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    app.event(Event::Forward);
    assert!(
        shows_down(&mut app, &format!("[73c5da0a/84h/0h/0h]{zpub}")),
        "the field keeps what was typed: {:?}",
        texts(&mut app)
    );
    assert!(!shows(&mut app, "Matches"), "{:?}", texts(&mut app));
}

#[test]
fn a_wallet_key_at_the_account_picked_names_no_path() {
    let v: Value = serde_json::from_str(COMPARE).unwrap();
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::Multisig)));
    app.press(Action::SeedCheck(S::Compare));
    testkit::inbox_file_at(
        &mut app,
        0,
        "wallet.txt",
        v["wallet"].as_str().unwrap().as_bytes().to_vec(),
    );
    app.press(Action::SeedCheck(S::CompareFile(0)));
    testkit::settle(&mut app);
    assert!(shows_down(&mut app, "Thisseediskey2ofthiswallet·2of3"));
    assert!(!shows(&mut app, "·2of3·m/"), "{:?}", texts(&mut app));
}

#[test]
fn another_accounts_key_does_not_match() {
    let mut app = at_compare(false);
    compare_typed(&mut app, &format!("[73c5da0a/86h/0h/0h]{}", bip86_xpub()));
    assert!(
        shows_down(&mut app, "Doesnotmatch"),
        "{:?}",
        texts(&mut app)
    );
    assert!(
        shows_down(&mut app, "Thekeyisatm/86h/0h/0h"),
        "its own path is named"
    );
    assert!(!shows(&mut app, "Matches"));
    assert!(!shows(&mut app, "Itmatcheswithoutthepassphrase"));
}

#[test]
fn the_comparison_is_a_job_with_a_busy_line_and_its_answer_comes_after() {
    let (_, zpub) = bip84();
    let mut app = at_compare(false);
    app.press(Action::SeedCheck(S::CompareField));
    type_chars(&mut app, &format!("[73c5da0a/84h/0h/0h]{zpub}"));
    testkit::press_on_worker(&mut app, Action::SeedCheck(S::CompareGo));
    assert!(app.busy().is_some());
    assert!(!shows(&mut app, "Matches"));
    testkit::settle(&mut app);
    assert!(app.busy().is_none());
    assert!(shows_down(&mut app, "Matches"));
}

#[test]
fn a_key_with_no_origin_is_refused() {
    let (_, zpub) = bip84();
    for bare in [zpub, bip86_xpub()] {
        let mut app = at_compare(false);
        compare_typed(&mut app, &bare);
        assert!(
            shows_down(&mut app, "itskeyhasnofingerprintandderivationpath"),
            "{:?}",
            texts(&mut app)
        );
        assert!(!shows(&mut app, "Matches"));
        assert!(!shows(&mut app, "Doesnotmatch"));
    }
}

#[test]
fn a_private_key_or_a_seed_typed_is_refused_and_gone() {
    let v: Value = serde_json::from_str(COMPARE).unwrap();
    for secret in [
        "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi",
        v["phrase"].as_str().unwrap(),
    ] {
        let mut app = at_compare(false);
        app.press(Action::SeedCheck(S::CompareField));
        type_chars(&mut app, secret);
        app.press(Action::SeedCheck(S::CompareGo));
        testkit::settle(&mut app);
        assert!(
            shows_down(&mut app, "Aprivatekey:nottakenhere"),
            "{:?}",
            texts(&mut app)
        );
        let first: String = secret.split_whitespace().next().unwrap().chars().collect();
        let all: String = texts(&mut app).concat().split_whitespace().collect();
        assert!(!all.contains(&first[..first.len().min(12)]), "{all}");
        assert!(!shows(&mut app, "Matches"));
    }
}

#[test]
fn a_key_made_without_the_passphrase_says_so() {
    let (_, zpub) = bip84();
    let mut app = at_compare(true);
    compare_typed(&mut app, &format!("[73c5da0a/84h/0h/0h]{zpub}"));
    assert!(
        shows_down(&mut app, "Doesnotmatch"),
        "{:?}",
        texts(&mut app)
    );
    assert!(
        shows_down(&mut app, "Itmatcheswithoutthepassphrase"),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn an_answer_for_one_account_is_not_shown_after_taking_another() {
    let (_, zpub) = bip84();
    let mut app = at_compare(false);
    compare_typed(&mut app, &format!("[73c5da0a/84h/0h/0h]{zpub}"));
    assert!(shows_down(&mut app, "Matches"));
    app.press(Action::SeedCheck(S::Another));
    app.press(Action::SeedCheck(S::Pick(Account::Legacy)));
    assert!(!shows_down(&mut app, "Matches"), "{:?}", texts(&mut app));
    assert!(!shows(&mut app, "Doesnotmatch"));
}

#[test]
fn an_answer_for_one_passphrase_is_not_shown_after_retaking_it() {
    let (_, zpub) = bip84();
    let mut app = at_compare(false);
    compare_typed(&mut app, &format!("[73c5da0a/84h/0h/0h]{zpub}"));
    assert!(shows_down(&mut app, "Matches"));
    app.press(Action::SeedCheck(S::Step(1)));
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    assert!(!shows_down(&mut app, "Matches"), "{:?}", texts(&mut app));
}

/// Practice with the public samples set, My seed makes my wallet open.
fn practice_check(practice: bool) -> Faraday {
    let mut app = testkit::started();
    if practice {
        app.online = true;
        app.session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Testnet);
        app.session.set_demo(testkit::demo_samples());
        app.set_modes(faraday_core::DESKTOP_MODES);
        app.samples = Some(testkit::demo_samples());
        app.set_hosted(true);
    }
    app.press(Action::Nav(faraday_core::Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "My seed makes my wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app
}

/// Whether the host strip is handed a button pressing `action`.
fn strip_offers(app: &mut Faraday, action: Action) -> bool {
    let _ = app.frame();
    app.sample_area()
        .aids()
        .iter()
        .any(|i| matches!(i, faraday_ui::demo::DemoItem::Button(_, a) if *a == action))
}

#[test]
fn in_practice_the_seed_card_offers_a_sample_seed_and_it_fills_the_seed() {
    let mut app = practice_check(true);
    assert!(strip_offers(&mut app, Action::SeedCheck(S::Sample)));
    app.press(Action::SeedCheck(S::Sample));
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    assert!(shows_down(&mut app, "Fingerprint"));
    assert!(!strip_offers(&mut app, Action::SeedCheck(S::Sample)));
}

#[test]
fn in_practice_the_compare_sample_answers_matches() {
    let mut app = practice_check(true);
    app.press(Action::SeedCheck(S::Sample));
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    app.press(Action::SeedCheck(S::Compare));
    assert!(strip_offers(&mut app, Action::SeedCheck(S::CompareSample)));
    app.press(Action::SeedCheck(S::CompareSample));
    assert!(!strip_offers(&mut app, Action::SeedCheck(S::CompareSample)));
    app.press(Action::SeedCheck(S::CompareGo));
    testkit::settle(&mut app);
    assert!(shows_down(&mut app, "Matches"));
    assert!(!shows(&mut app, "Doesnotmatch"));
}

#[test]
fn outside_practice_no_sample_is_offered() {
    let mut app = practice_check(false);
    assert!(!strip_offers(&mut app, Action::SeedCheck(S::Sample)));
    app.press(Action::SeedCheck(S::Sample));
    assert!(shows(&mut app, "Typethewords"));
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    app.press(Action::SeedCheck(S::Compare));
    assert!(!strip_offers(&mut app, Action::SeedCheck(S::CompareSample)));
}

#[test]
fn a_descriptor_of_this_account_matches() {
    let (_, zpub) = bip84();
    let mut app = at_compare(false);
    compare_typed(
        &mut app,
        &format!("wpkh([73c5da0a/84h/0h/0h]{zpub}/<0;1>/*)"),
    );
    assert!(shows_down(&mut app, "Matches"), "{:?}", texts(&mut app));
    assert!(!shows(&mut app, "Doesnotmatch"));
}

#[test]
fn the_first_receive_address_matches_with_its_index() {
    let (first, _) = bip84();
    let mut app = at_compare(false);
    compare_typed(&mut app, &first);
    assert!(shows_down(&mut app, "Matches"), "{:?}", texts(&mut app));
    assert!(
        shows_down(&mut app, "Receiveaddress0"),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn an_origin_of_this_seed_with_a_key_that_is_not_its_own_does_not_match() {
    let v: Value = serde_json::from_str(COMPARE).unwrap();
    let foreign = v["cosigners"][0]["xpub"].as_str().unwrap();
    let mut app = at_compare(false);
    compare_typed(&mut app, &format!("[73c5da0a/84h/0h/0h]{foreign}"));
    assert!(
        shows_down(&mut app, "Doesnotmatch"),
        "{:?}",
        texts(&mut app)
    );
    assert!(
        shows_down(
            &mut app,
            "Thefingerprintisthisseed's;thexpubatm/84h/0h/0hisnot"
        ),
        "{:?}",
        texts(&mut app)
    );
    assert!(!shows(&mut app, "Matches"));
}

/// The tool opened, on its Seed card.
fn seed_card() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "My seed makes my wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app
}

/// The Seed card's row for `form` pressed, and `parts` typed with Enter
/// after each.
fn parts_typed(form: faraday_core::forms::Form, parts: &[&str]) -> Faraday {
    let mut app = seed_card();
    let at = faraday_core::seed_check::FORMS
        .iter()
        .position(|f| *f == form)
        .unwrap();
    app.press(Action::SeedCheck(S::Form(at as u8)));
    for part in parts {
        type_chars(&mut app, part);
        app.event(Event::Key(Key::Enter));
    }
    app
}

fn held_fingerprint(app: &Faraday) -> String {
    app.session
        .keys
        .check_facts()
        .expect("the check holds a key")
        .fingerprint
        .to_string()
}

#[test]
fn seed_xor_parts_are_checked_as_the_words_they_make_are() {
    let mut app = parts_typed(
        faraday_core::forms::Form::Xor,
        &[
            "accident stock globe super snake hero sample law fit blade scale silly",
            "",
            "insect shoe rescue viable grab then adult rigid funny forget peace napkin",
            "",
        ],
    );
    app.press(Action::SeedCheck(S::NoPass));
    let mut typed =
        with_words("inner away stamp cliff position merge rubber series antique era gift force");
    typed.press(Action::SeedCheck(S::NoPass));
    assert_eq!(held_fingerprint(&app), held_fingerprint(&typed));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    let fingerprint = held_fingerprint(&app);
    assert!(shows(&mut app, &fingerprint), "the Result shows it");
}

#[test]
fn a_codex32_secret_is_checked_with_no_words_and_no_passphrase() {
    // BIP 93 vector 1.
    let mut app = parts_typed(
        faraday_core::forms::Form::Codex32,
        &["ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw", ""],
    );
    let want = fingerprint_of(
        &compute(),
        "xprv9s21ZrQH143K3taPNekMd9oV5K6szJ8ND7vVh6fxicRUMDcChr3bFFzuxY8qP3xFFBL6DWc2uEYCfBFZ2nFWbAqKPhtCLRjgv78EZJDEfpL",
    );
    assert_eq!(held_fingerprint(&app), want);
    assert_eq!(app.session.keys.check_facts().unwrap().words, None);
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    assert!(shows(&mut app, &want), "the Result shows the fingerprint");
}

#[test]
fn slip39_shares_wait_for_the_passphrase_and_then_are_checked() {
    // SLIP-39 vector 4, passphrase TREZOR.
    let mut app = parts_typed(
        faraday_core::forms::Form::Slip39,
        &[
            "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed",
            "shadow pistol academic acid actress prayer class unknown daughter sweater depict flip twice unkind craft early superior advocate guest smoking",
            "",
        ],
    );
    assert!(
        app.session.keys.check_facts().is_none(),
        "waits for the passphrase"
    );
    app.press(Action::SeedCheck(S::AddPass));
    app.press(Action::SeedCheck(S::PassField(0)));
    type_chars(&mut app, "TREZOR");
    app.press(Action::SeedCheck(S::PassField(1)));
    type_chars(&mut app, "TREZOR");
    app.press(Action::PassEye);
    app.press(Action::SeedCheck(S::UsePass));
    let want = fingerprint_of(
        &compute(),
        "xprv9s21ZrQH143K2nNuAbfWPHBtfiSCS14XQgb3otW4pX655q58EEZeC8zmjEUwucBu9dPnxdpbZLCn57yx45RBkwJHnwHFjZK4XPJ8SyeYjYg",
    );
    assert_eq!(held_fingerprint(&app), want);
    assert!(
        testkit::entry(&app)
            .parts
            .lines(faraday_core::forms::Form::Slip39)
            .is_empty(),
        "the shares are gone once read"
    );
}

#[test]
fn a_codex32_secret_scanned_here_fills_this_tool_and_not_add_a_key() {
    let mut app = seed_card();
    app.press(Action::ScanSeed);
    app.event(Event::Scanned {
        bytes: b"MS10TESTSXXXXXXXXXXXXXXXXXXXXXXXXXX4NZVCA9CMCZLW".to_vec(),
    });
    assert_eq!(app.screen, faraday_core::Screen::SeedCheck);
    assert!(app.scan().is_none(), "the camera closed");
    assert!(app.session.keys.is_empty(), "no key was loaded");
    assert!(
        app.session.keys.check_facts().is_some(),
        "the check holds it"
    );
}

const SHARE_1: &str = "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed";
const SHARE_2: &str = "shadow pistol academic acid actress prayer class unknown daughter sweater depict flip twice unkind craft early superior advocate guest smoking";

/// SLIP-39 vector 4's two shares typed and Enter on nothing: the shares
/// wait on the Passphrase card.
fn shares_waiting() -> Faraday {
    let app = parts_typed(faraday_core::forms::Form::Slip39, &[SHARE_1, SHARE_2, ""]);
    assert!(app.session.keys.check_facts().is_none(), "they wait");
    app
}

#[test]
fn words_typed_after_back_from_waiting_shares_are_read_as_words() {
    for form in [None, Some(faraday_core::forms::Form::Xor)] {
        let mut app = shares_waiting();
        app.event(Event::Back);
        match form {
            None => {
                app.press(Action::SeedCheck(S::Type));
                testkit::type_words(&mut app, ABOUT);
                app.press(Action::SeedCheck(S::Words));
            }
            Some(_) => {
                // Seed XOR's two parts, typed as words are, make words.
                let at = faraday_core::seed_check::FORMS
                    .iter()
                    .position(|f| *f == faraday_core::forms::Form::Xor)
                    .unwrap();
                app.press(Action::SeedCheck(S::Form(at as u8)));
                for part in [
                    "accident stock globe super snake hero sample law fit blade scale silly",
                    "",
                    "insect shoe rescue viable grab then adult rigid funny forget peace napkin",
                    "",
                ] {
                    type_chars(&mut app, part);
                    app.event(Event::Key(Key::Enter));
                }
            }
        }
        app.press(Action::SeedCheck(S::NoPass));
        let mut typed = if form.is_none() {
            with_words(ABOUT)
        } else {
            with_words("inner away stamp cliff position merge rubber series antique era gift force")
        };
        typed.press(Action::SeedCheck(S::NoPass));
        assert_eq!(held_fingerprint(&app), held_fingerprint(&typed), "{form:?}");
    }
}

#[test]
fn back_then_forward_keeps_the_shares_waiting() {
    let mut app = shares_waiting();
    app.event(Event::Back);
    app.event(Event::Forward);
    app.press(Action::SeedCheck(S::NoPass));
    assert!(
        app.session.keys.check_facts().is_some(),
        "the shares waiting are read"
    );
}

#[test]
fn a_key_loaded_after_shares_wait_is_the_key_checked() {
    let result = |shares: bool| {
        let mut app = with_words_on(testkit::net(), ABOUT);
        app.press(Action::SeedCheck(S::NoPass));
        app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
        app.press(Action::SeedCheck(S::Keep));
        faraday_core::testkit::keep_published(&mut app);
        app.press(Action::Nav(faraday_core::Screen::Catalog));
        let tile = faraday_core::catalog::TILES
            .iter()
            .position(|t| t.name == "My seed makes my wallet")
            .unwrap();
        app.press(Action::Catalog(tile as u8));
        if shares {
            let at = faraday_core::seed_check::FORMS
                .iter()
                .position(|f| *f == faraday_core::forms::Form::Slip39)
                .unwrap();
            app.press(Action::SeedCheck(S::Form(at as u8)));
            for part in [SHARE_1, SHARE_2, ""] {
                type_chars(&mut app, part);
                app.event(Event::Key(Key::Enter));
            }
            app.event(Event::Back);
        }
        app.press(Action::SeedCheck(S::Loaded(0)));
        app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
        assert!(app.session.keys.check_facts().is_none(), "no shares read");
        texts(&mut app)
    };
    assert_eq!(result(true), result(false));
}

#[test]
fn a_seedqr_scanned_after_back_from_waiting_shares_is_the_key_checked() {
    let mut app = shares_waiting();
    app.event(Event::Back);
    app.press(Action::ScanSeed);
    // Eleven times word 0000 and word 0003: the twelve words of ABOUT.
    app.event(Event::Scanned {
        bytes: format!("{}0003", "0000".repeat(11)).into_bytes(),
    });
    let _ = app.frame();
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
    let _ = app.frame();
    let mut typed = with_words(ABOUT);
    typed.press(Action::SeedCheck(S::NoPass));
    assert_eq!(held_fingerprint(&app), held_fingerprint(&typed));
    // The Passphrase header takes the SeedQR's key back to the Seed card;
    // no shares wait to be read in its place.
    app.press(Action::SeedCheck(S::Step(1)));
    app.press(Action::SeedCheck(S::NoPass));
    assert!(
        app.session.keys.check_facts().is_none(),
        "no old shares read"
    );
}

#[test]
fn keep_this_key_on_a_codex32_key_keeps_the_set_s_facts_and_strings() {
    // BIP 93 vector 2: shares A and C of a 2 of 3 set, on the test network.
    let mut app = seed_card();
    testkit::move_to(&mut app.session, testkit::net()).unwrap();
    let at = faraday_core::seed_check::FORMS
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Codex32)
        .unwrap();
    app.press(Action::SeedCheck(S::Form(at as u8)));
    for s in [
        "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
        "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
    ] {
        type_chars(&mut app, s);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    app.press(Action::SeedCheck(S::Keep));
    assert_eq!(app.session.keys.len(), 1, "kept");
    let facts = app.session.keys.info(0).unwrap().codex32.clone();
    let facts = facts.expect("the set's facts are kept");
    assert_eq!(facts.threshold, 2);
    assert_eq!(facts.strings.len(), 2, "the strings typed");
    for (i, s) in [
        (b'a', "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm"),
        (b'c', "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn"),
    ] {
        assert_eq!(
            app.session.keys.codex32_reads_as(0, i, s.as_bytes()),
            Some(true),
            "the string at {} is kept with the key",
            i as char
        );
    }
}

#[test]
fn leaving_drops_the_parts_and_the_shares_waiting() {
    for end in 0..3 {
        let mut app = if end == 0 {
            parts_typed(faraday_core::forms::Form::Slip39, &[SHARE_1])
        } else {
            shares_waiting()
        };
        assert!(testkit::entry(&app).parts.holds_any(), "end {end}");
        if end == 2 {
            app.press(Action::Lock);
        } else {
            app.press(Action::SeedCheck(S::Cancel));
        }
        let _ = app.frame();
        assert!(!testkit::entry(&app).parts.holds_any(), "way out {end}");
    }
}

#[test]
fn a_codex32_secret_read_with_a_stick_in_is_refused_on_the_check() {
    let mut app = seed_card();
    let at = faraday_core::seed_check::FORMS
        .iter()
        .position(|f| *f == faraday_core::forms::Form::Codex32)
        .unwrap();
    app.press(Action::SeedCheck(S::Form(at as u8)));
    // The stick comes in once the first of two strings is typed: the
    // strings typed stay, and the set is finished with it in.
    type_chars(&mut app, "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm");
    app.event(Event::Key(Key::Enter));
    stick_in(&mut app);
    type_chars(&mut app, "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn");
    app.event(Event::Key(Key::Enter));
    app.event(Event::Key(Key::Enter));
    assert!(app.session.keys.check_facts().is_none(), "no key is held");
    // The stick's sheet is put away to read the card under it.
    app.press(Action::Cancel);
    let shown = texts(&mut app).join(" ");
    let en = &faraday_strings::EN;
    let said = [en.medium_stick_inline, en.medium_card_or_stick_inline]
        .map(|n| en.stick_keys_keys_load.replacen("{}", n, 1));
    assert!(
        said.iter().any(|l| shown.contains(l.as_str())),
        "the check's own refusal is shown: {shown}"
    );
}

/// The window resized to `width` × `height` px at 160 dpi.
fn resize(app: &mut Faraday, width: u16, height: u16) {
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
}

#[test]
fn the_open_step_is_alone_with_your_choices_beside_it_or_in_the_sheet() {
    let mut app = with_words(ABOUT);
    app.press(Action::SeedCheck(S::NoPass));
    // Wide: the pane lists the steps with the answers given, and a press
    // on an earlier row opens that step as it was.
    let wide = texts(&mut app);
    for line in ["Your choices", "Step 3 of 4", "12 words"] {
        assert!(wide.iter().any(|t| t == line), "{line}: {wide:?}");
    }
    app.press(Action::Choices(ChoicesPress::Row(0)));
    assert!(texts(&mut app).iter().any(|t| t == "Step 1 of 4"));
    app.press(Action::SeedCheck(S::Show(2)));
    // A panel: one card, and the bar's button opens the sheet.
    resize(&mut app, 480, 640);
    let panel = texts(&mut app);
    assert!(
        !panel.iter().any(|t| t == EN.seed_check_card_seed),
        "{panel:?}"
    );
    app.press(Action::Choices(ChoicesPress::Open));
    assert_eq!(app.sheet, Some(Sheet::Choices));
    assert!(texts(&mut app).iter().any(|t| t == "12 words"));
}
