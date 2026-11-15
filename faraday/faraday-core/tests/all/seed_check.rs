//! My seed makes my wallet: what the seed makes at each account, against
//! the published test accounts (BIP-49, BIP-84, BIP-86), BIP-39's vectors
//! for the fingerprints and BIP-32's for a path typed by hand; and the
//! seed held for the check alone (kept as Add a key keeps a key, refused
//! with a stick in, gone when dropped).

use faraday_bip::bitcoin::bip32::Xpriv;
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_core::seed_check::{ACCOUNTS, Account, Source, make};
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
use faraday_core::{Action, Faraday, StorageEvent, testkit};
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
    let mut app = testkit::started();
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
    app.press(Action::SeedCheck(S::UsePass));
    app.press(Action::SeedCheck(S::Pick(Account::NativeSegwit)));
    assert!(
        shows(&mut app, &want),
        "the fingerprint with the passphrase"
    );
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
    let mut app = with_words(ABOUT);
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

    let mut app = with_words(ABOUT);
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
fn back_to_passphrase_for_a_loaded_key_opens_seed() {
    let mut app = with_words(ABOUT);
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
