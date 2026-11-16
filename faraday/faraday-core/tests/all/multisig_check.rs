//! Check a multisig setup (`docs/FLOWS.md`): a descriptor, a policy or a
//! setup file read by Files' reader and shown as its quorum, checksum and
//! cosigners, with nothing added to the session.

use crate::common::{has_part, texts};
use faraday_bip::policy::WalletPolicy;
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::multisig_check::MsAction as M;
use faraday_core::seed_check::SeedAction as S;
use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, Event, Key};

fn kit(id: &str) -> String {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == id)
        .unwrap_or_else(|| panic!("{id}"))
        .descriptor
}

/// The check open with its scanner, `text` read on it.
fn scanned(text: &str) -> Faraday {
    let mut app = testkit::started();
    app.press(Action::MultisigCheck(M::Open));
    app.press(Action::MultisigCheck(M::Scan));
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
    app
}

fn checksummed(descriptor: &str) -> String {
    let c = testkit::compute();
    WalletPolicy::parse_any(&c, descriptor)
        .unwrap()
        .to_descriptor_checksummed()
        .to_string()
}

#[test]
fn a_sorted_multisig_descriptor_shows_its_quorum_and_every_cosigner() {
    let d = kit("savings");
    let mut app = scanned(d.split('#').next().unwrap());
    assert!(has_part(&mut app, "2 of 3 · native SegWit multisig"));
    for n in 1..=3 {
        assert!(has_part(&mut app, &format!("Key {n}")));
    }
    // Each cosigner's fingerprint and origin path are shown.
    assert!(has_part(&mut app, "48h/1h/0h/2h"));
    assert!(has_part(&mut app, "Descriptor checksum"));
    assert!(has_part(&mut app, "worked out here, none given"));
    assert!(has_part(&mut app, "First receive address"));
    assert!(app.session.wallets.is_empty());
}

#[test]
fn a_checksum_that_holds_is_said_to_hold() {
    let mut app = scanned(&checksummed(&kit("savings")));
    assert!(has_part(&mut app, "holds"));
    assert!(!has_part(&mut app, "worked out here"));
}

#[test]
fn a_checksum_that_fails_is_refused_and_no_key_is_shown() {
    let good = checksummed(&kit("savings"));
    let (body, sum) = good.split_once('#').unwrap();
    let last = if sum.ends_with('q') { 'p' } else { 'q' };
    let bad = format!("{body}#{}{last}", &sum[..sum.len() - 1]);
    let mut app = scanned(&bad);
    assert!(has_part(&mut app, "The descriptor checksum does not hold"));
    assert!(!has_part(&mut app, "Key 1"));
}

/// Bitcoin Core's own documented multisig example and its checksum
/// (doc/descriptors.md): the checksum is read as given, not worked out here.
const CORE_EXAMPLE: &str = "wsh(multi(1,xpub661MyMwAqRbcFW31YEwpkMuc5THy2PSt5bDMsktWQcFF8syAmRUapSCGu8ED9W6oDMSgv6Zz8idoc4a6mr8BDzTJY47LJhkJ8UB7WEGuduB/1/0/*,xpub69H7F5d8KSRgmmdJg2KhpAK8SR3DjMwAdkxj3ZuxV27CprR9LgpeyGmXUbC6wb7ERfvrnKZjXoUmmDznezpbZb7ap6r1D3tgFxHmwMkQTPH/0/0/*))";

#[test]
fn a_checksum_from_bitcoin_core_is_accepted_and_a_changed_one_refused() {
    // Its keys have no origin, so it is refused, but not for its checksum.
    let mut app = scanned(&format!("{CORE_EXAMPLE}#t2zpj2eu"));
    assert!(!has_part(&mut app, "The descriptor checksum does not hold"));
    assert!(!has_part(&mut app, "Key 1"));
    let mut app = scanned(&format!("{CORE_EXAMPLE}#t2zpj2ev"));
    assert!(has_part(&mut app, "The descriptor checksum does not hold"));
}

#[test]
fn a_key_without_its_origin_is_refused() {
    let mut bare = String::new();
    let mut inside = false;
    for ch in kit("savings").split('#').next().unwrap().chars() {
        match ch {
            '[' => inside = true,
            ']' => inside = false,
            c if !inside => bare.push(c),
            _ => {}
        }
    }
    let mut app = scanned(&bare);
    assert!(has_part(&mut app, "key is not [fingerprint/path]xpub"));
    assert!(!has_part(&mut app, "Key 1"));
}

#[test]
fn a_wallet_of_one_key_is_not_a_setup() {
    let mut app = scanned(&kit("spending"));
    assert!(has_part(&mut app, "One key: not a multisig setup"));
}

#[test]
fn a_musig2_wallet_is_every_key_never_one_of_n() {
    let c = testkit::compute();
    let _ = c;
    let d = testkit::musig_policy().unwrap().to_descriptor_checksummed();
    let mut app = scanned(&d);
    assert!(has_part(&mut app, "3 of 3 · MuSig2"));
    assert!(!has_part(&mut app, "1 of 3"));
}

#[test]
fn a_script_tree_lists_its_ways_to_spend() {
    let mut app = scanned(&kit("tree"));
    assert!(has_part(&mut app, "Ways to spend"));
    assert!(has_part(&mut app, "Key path"));
}

#[test]
fn a_key_loaded_here_is_marked_yours_on_its_cosigner() {
    let mut app = testkit::started();
    let c = testkit::compute();
    app.session
        .keys
        .add_words(&c, &testkit::test_words("bacon"), "", None)
        .unwrap();
    app.press(Action::MultisigCheck(M::Open));
    app.press(Action::MultisigCheck(M::Scan));
    app.event(Event::Scanned {
        bytes: kit("savings").into_bytes(),
    });
    let _ = app.frame();
    assert!(has_part(&mut app, "Yours"));
}

#[test]
fn the_mark_is_the_loaded_key_after_the_way_back_in() {
    let mut app = testkit::started();
    let c = testkit::compute();
    app.session
        .keys
        .add_words(&c, &testkit::test_words("bacon"), "", None)
        .unwrap();
    app.press(Action::MultisigCheck(M::Open));
    app.press(Action::Nav(Screen::Home));
    app.session.wipe();
    app.press(Action::MultisigCheck(M::Open));
    app.press(Action::MultisigCheck(M::Scan));
    app.event(Event::Scanned {
        bytes: kit("savings").into_bytes(),
    });
    let _ = app.frame();
    assert!(!has_part(&mut app, "Yours"));
}

#[test]
fn a_file_of_files_is_checked_from_its_menu() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "savings.txt",
            checksummed(&kit("savings")).into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::File(FilePress::Menu(Row::In(0))));
    let lines = app
        .files_menu()
        .menu
        .as_ref()
        .expect("its menu")
        .lines
        .clone();
    let (_, check) = lines
        .iter()
        .find(|(l, _)| l == EN.files_check_setup)
        .unwrap_or_else(|| panic!("Check this setup is in {lines:?}"));
    let _ = app.frame();
    app.press(*check);
    assert_eq!(app.screen, Screen::MultisigCheck);
    assert!(has_part(&mut app, "2 of 3 · native SegWit multisig"));
    assert!(has_part(&mut app, "savings.txt"));
}

const PATH: &str = "m/48'/1'/0'/2'";
const ABOUT: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// A 2-of-3 over test keys 1 and 3 and `middle`.
fn with_middle(middle: &str) -> String {
    format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{middle}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(0, PATH),
        testkit::key(2, PATH)
    )
}

/// `[fingerprint/path]tpub` of the key `words` and `pass` make.
fn key_with(words: &str, pass: &str) -> String {
    let mut app = testkit::started();
    let c = testkit::compute();
    let fp = app
        .session
        .keys
        .add_words_with(&c, words, pass, "", None)
        .unwrap();
    let path = std::str::FromStr::from_str(PATH).unwrap();
    let mut xpub = app.session.keys.xpub_at(&c, 0, &path).unwrap();
    // The wallet's keys are the test network's.
    xpub.network = faraday_bip::bitcoin::NetworkKind::Test;
    format!("[{fp}/48'/1'/0'/2']{xpub}")
}

fn type_chars(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Check a seed against it pressed, the words typed and taken, and the
/// passphrase (none, or `pass` typed twice) given.
fn give(app: &mut Faraday, words: &str, pass: Option<&str>) {
    app.press(Action::MultisigCheck(M::CheckSeed));
    app.press(Action::SeedCheck(S::Type));
    testkit::type_words(app, words);
    app.press(Action::SeedCheck(S::Words));
    match pass {
        None => app.press(Action::SeedCheck(S::NoPass)),
        Some(p) => {
            app.press(Action::SeedCheck(S::AddPass));
            app.press(Action::SeedCheck(S::PassField(0)));
            type_chars(app, p);
            app.press(Action::SeedCheck(S::PassField(1)));
            type_chars(app, p);
            app.press(Action::SeedCheck(S::UsePass));
        }
    }
    testkit::settle(app);
}

/// Whether the page shows `part`, scrolled down to it.
fn shows(app: &mut Faraday, part: &str) -> bool {
    for _ in 0..20 {
        if has_part(app, part) {
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

#[test]
fn a_seed_that_is_a_cosigner_is_named_with_its_number_and_marks_its_row() {
    let mut app = scanned(&kit("savings"));
    give(&mut app, &testkit::test_words("zebra"), None);
    assert!(shows(&mut app, "This seed is key 2 of this wallet"));
    assert!(texts(&mut app).iter().any(|t| t == "This seed"));
    assert!(!has_part(&mut app, "Not a key of this wallet"));
    // It is only checked: nothing is loaded.
    assert!(app.session.keys.all().next().is_none());
}

#[test]
fn a_seed_with_its_passphrase_is_a_cosigner_and_without_it_is_not() {
    let d = with_middle(&key_with(&testkit::test_words("zebra"), "TREZOR"));
    let mut app = scanned(&d);
    give(&mut app, &testkit::test_words("zebra"), Some("TREZOR"));
    assert!(shows(&mut app, "This seed is key 2 of this wallet"));
    // The passphrase not given is never searched for.
    give(&mut app, &testkit::test_words("zebra"), None);
    assert!(shows(&mut app, "Not a key of this wallet"));
    assert!(!has_part(&mut app, "It matches without the passphrase"));
}

#[test]
fn a_passphrase_that_the_cosigner_does_not_use_is_said_to_match_without_it() {
    let mut app = scanned(&kit("savings"));
    give(&mut app, &testkit::test_words("zebra"), Some("faraday"));
    assert!(shows(&mut app, "Not a key of this wallet"));
    assert!(shows(&mut app, "It matches without the passphrase"));
}

#[test]
fn a_cosigner_s_fingerprint_on_another_key_gets_the_warning() {
    // Key 2 carries the zebra seed's fingerprint on another seed's key.
    let (zebra, about) = (testkit::key(1, PATH), key_with(ABOUT, ""));
    let (fp, _) = zebra.split_once(']').unwrap();
    let (_, other) = about.split_once(']').unwrap();
    let mut app = scanned(&with_middle(&format!("{fp}]{other}")));
    give(&mut app, &testkit::test_words("zebra"), None);
    assert!(shows(&mut app, "Not a key of this wallet"));
    assert!(shows(&mut app, "The fingerprint is "));
    assert!(shows(&mut app, "is not"));
    assert!(!has_part(&mut app, "This seed is key"));
}

#[test]
fn an_unrelated_seed_is_not_a_key_of_the_wallet() {
    let mut app = scanned(&kit("savings"));
    give(&mut app, ABOUT, None);
    assert!(shows(
        &mut app,
        "Not a key of this wallet: check the words or the passphrase"
    ));
    assert!(!has_part(&mut app, "The fingerprint is "));
    assert!(!has_part(&mut app, "It matches without the passphrase"));
}

#[test]
fn the_seed_is_gone_after_done_another_setup_and_a_lock() {
    for end in 0..3 {
        let mut app = scanned(&kit("savings"));
        give(&mut app, &testkit::test_words("zebra"), None);
        assert!(app.session.keys.check_facts().is_some(), "held");
        match end {
            0 => app.press(Action::Nav(Screen::Tools)),
            1 => app.press(Action::MultisigCheck(M::Another)),
            _ => crate::common::lock(&mut app),
        }
        let _ = app.frame();
        assert!(app.session.keys.check_facts().is_none(), "way out {end}");
        assert!(!app.session.keys.holds_secret(), "way out {end}");
    }
}

/// `[fingerprint/path]tpub` of BIP-39's vector "abandon ... about" with the
/// passphrase "TREZOR" (seed `c55257c3…`, the `xprv9s21ZrQH143K3h3f…` root
/// of `convert.rs`) at BIP-48 account 0 and 1, derived with the Python
/// library embit 0.8.0 (testnet keys).
const TREZOR_KEYS: [&str; 2] = [
    "[b4e3f5ed/48'/1'/0'/2']tpubDELhgCXumKjTuYRLqt7faiBC44qPDPXzi5HXyzNZwiAhDx8qZDFf3hSLwKCkgN7HnqY1E6DgsTdbc37HR1yVeD7Gwd2eL62rnZDmQdNYqGd",
    "[b4e3f5ed/48'/1'/1'/2']tpubDDxMRPETHGXPjyZ2wRbSgMMzb3d4VpkbvEqGXG3F7sY2fP97765KNN1rCcDsANihsLrdgpWEpndtDAk2aywozCmrE9xvhFXXj4MkVGzgtnG",
];

fn setup(keys: [&str; 3]) -> String {
    format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        keys[0], keys[1], keys[2]
    )
}

#[test]
fn a_published_vector_s_passphrase_key_is_a_cosigner_only_with_its_passphrase() {
    let d = setup([
        TREZOR_KEYS[0],
        &testkit::key(0, PATH),
        &testkit::key(2, PATH),
    ]);
    let mut app = scanned(&d);
    give(&mut app, ABOUT, Some("TREZOR"));
    assert!(shows(&mut app, "This seed is key 1 of this wallet"));
    give(&mut app, ABOUT, None);
    assert!(shows(&mut app, "Not a key of this wallet"));
    // The fingerprint differs without the passphrase, so no warning.
    assert!(!has_part(&mut app, "The fingerprint is "));
}

#[test]
fn a_cosigner_seed_whose_fingerprint_is_on_another_key_is_named_and_warned() {
    let (zebra, about) = (testkit::key(1, PATH), key_with(ABOUT, ""));
    let (fp, _) = zebra.split_once(']').unwrap();
    let (_, other) = about.split_once(']').unwrap();
    let d = setup([&testkit::key(0, PATH), &zebra, &format!("{fp}]{other}")]);
    let mut app = scanned(&d);
    give(&mut app, &testkit::test_words("zebra"), None);
    assert!(shows(&mut app, "This seed is key 2 of this wallet"));
    assert!(texts(&mut app).iter().any(|t| t == "This seed"));
    assert!(shows(&mut app, "The fingerprint is "));
    assert!(!has_part(&mut app, "Not a key of this wallet"));
}

#[test]
fn a_seed_that_is_several_cosigners_names_and_marks_every_one() {
    let d = setup([TREZOR_KEYS[0], &testkit::key(0, PATH), TREZOR_KEYS[1]]);
    let mut app = scanned(&d);
    give(&mut app, ABOUT, Some("TREZOR"));
    assert!(shows(&mut app, "This seed is keys 1, 3 of this wallet"));
    let marks = texts(&mut app).iter().filter(|t| *t == "This seed").count();
    assert_eq!(marks, 2);
}

#[test]
fn keys_typed_with_the_result_closed_reach_nothing_and_the_seed_is_gone() {
    for (back, given) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut app = scanned(&kit("savings"));
        if given {
            give(&mut app, &testkit::test_words("zebra"), None);
            assert!(app.session.keys.check_facts().is_some(), "held");
        } else {
            app.press(Action::MultisigCheck(M::CheckSeed));
            app.press(Action::SeedCheck(S::Type));
            testkit::type_words(&mut app, &testkit::test_words("zebra"));
            app.press(Action::SeedCheck(S::Words));
            app.press(Action::SeedCheck(S::AddPass));
            app.press(Action::SeedCheck(S::PassField(0)));
        }
        let step = if back {
            faraday_core::multisig_check::mstep::SETUP
        } else {
            faraday_core::multisig_check::mstep::RESULT
        };
        app.press(Action::MultisigCheck(M::Step(step)));
        let _ = app.frame();
        type_chars(&mut app, "TREZOR");
        app.event(Event::Key(Key::Enter));
        testkit::settle(&mut app);
        assert!(app.keyboard_top().is_none(), "no keyboard, back {back}");
        // The Result opened again starts fresh: no seed flow, no field
        // holding what was typed.
        app.press(Action::MultisigCheck(M::Step(
            faraday_core::multisig_check::mstep::RESULT,
        )));
        let _ = app.frame();
        let shown = texts(&mut app);
        assert!(!shown.iter().any(|t| t.contains("TREZOR")), "back {back}");
        assert!(
            !shown.iter().any(|t| t == EN.seed_check_passphrase),
            "back {back}"
        );
        assert!(app.keyboard_top().is_none(), "reopened, back {back}");
        assert!(app.session.keys.check_facts().is_none(), "back {back}");
        assert!(!app.session.keys.holds_secret(), "back {back}");
        assert!(app.session.keys.all().next().is_none());
    }
}

#[test]
fn add_this_wallet_asks_the_sheet_and_the_hold_loads_it_here() {
    let mut app = scanned(&kit("savings"));
    assert!(has_part(&mut app, "Add this wallet"));
    app.press(Action::MultisigCheck(M::Add));
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WalletConfirm));
    assert!(app.session.wallets.is_empty());
    app.press(Action::WalletHold);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.screen, Screen::MultisigCheck);
    let name = app.session.wallets[0].name.to_string();
    assert!(has_part(&mut app, &format!("Loaded here as {name}")));
    assert!(!has_part(&mut app, "Add this wallet"));
}

#[test]
fn cancelling_the_sheet_adds_nothing_and_the_page_still_offers_it() {
    let mut app = scanned(&kit("savings"));
    app.press(Action::MultisigCheck(M::Add));
    app.press(Action::WalletCancel);
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.is_empty());
    assert!(has_part(&mut app, "Add this wallet"));
}

#[test]
fn a_wallet_already_loaded_reads_loaded_at_once() {
    let mut app = scanned(&kit("savings"));
    app.press(Action::MultisigCheck(M::Add));
    app.press(Action::WalletHold);
    app.press(Action::MultisigCheck(M::Another));
    app.press(Action::MultisigCheck(M::Scan));
    app.event(Event::Scanned {
        bytes: kit("savings").into_bytes(),
    });
    let _ = app.frame();
    assert!(has_part(&mut app, "Loaded here as"));
    assert!(!has_part(&mut app, "Add this wallet"));
    assert_eq!(app.session.wallets.len(), 1);
}
