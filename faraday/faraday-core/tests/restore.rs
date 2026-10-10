//! Restore a wallet made elsewhere (`docs/NEW-WALLET.md` §12): what kind
//! it is, its quorum, its description or none, then a slot per key for its
//! seed, with a passphrase if it has one, or a cosigner's xpub; the
//! wallet, its first addresses, and the backup or a spend after.

use faraday_core::family::FamilyAction as F;
use faraday_core::rstep::{CHECK, DESCRIPTION, DONE, KIND, QUORUM, SEEDS};
use faraday_core::seeds::{Focus, SeedsAction as S};
use faraday_core::{Action, Faraday, NOT_A_KEY, Screen, testkit};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

const MULTI_PATH: &str = "m/48'/1'/0'/2'";
/// Multisig · native SegWit, by its place on Kind.
const MULTI: u8 = 4;
/// BIP-39's test words, whose key is in none of the test wallets.
const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// The 2-of-3 over the three test seeds at BIP-48 account 0 on testnet,
/// the third seed (summer ×24) with the BIP-39 passphrase `TREZOR`, as
/// Sparrow writes a wallet descriptor: each key with its origin,
/// `<0;1>/*`, `h` for hardened, and the BIP-380 checksum. Its three
/// fingerprints and tpubs were derived a second time outside the
/// repository, by a plain Python BIP-39 (PBKDF2) and BIP-32 derivation
/// with no shared code, and agree; the summer key without the
/// passphrase is cf2e083d.
const PASSPHRASE_VECTOR: &str = "wsh(sortedmulti(2,\
[9a6a2580/48h/1h/0h/2h]tpubDFZk17LmJpm9HZWfxoggu7yC1KeUm26JA2gCHnCXdo9oVPnpGZb9ErVXmiRiDMBgTiQ9oNdC3TDE4PvVy7eMvubhZqkcrPyCkiZvzWBVqrr/<0;1>/*,\
[5d388376/48h/1h/0h/2h]tpubDFcsPThm5auRC835VVQsJ4U3cwqobwd2NUsMBHh7ngRtyRVFtKcWrWMUK5jQ6nFmUVfC5aLckrGaPSkRkxDYZL2Qxf3weKxbYb4uSvdc5fb/<0;1>/*,\
[9f071687/48h/1h/0h/2h]tpubDEks6YLhzDBuCX4UVsPNw2dBLFGmnRsPBSeR2HjHRMYeWF9hHrzpfcESBYFCUVTatnviXTnjy7JSJy8N8NwGZ5piMgwf6BoV8w7FFDH2ohz/<0;1>/*))#jrauekm9";

fn shown() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Network(testkit::NET));
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// The open slot's two passphrase fields, typed.
fn passphrase(app: &mut Faraday, first: &str, again: &str) {
    app.press(Action::RPassField(0));
    type_text(app, first);
    app.press(Action::RPassField(1));
    type_text(app, again);
}

/// The open slot's Type the words, with these words.
fn type_seed(app: &mut Faraday, words: &str) {
    app.press(Action::RSlotWords);
    assert_eq!(app.screen, Screen::Entry);
    type_text(app, words);
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Restore, "Add a key did not return");
}

/// Restore on a multisig at its default quorum, past the description.
fn two_of_three_from_seeds() -> Faraday {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    app.press(Action::RNext(QUORUM));
    app.press(Action::RSeeds);
    assert_eq!(app.restore.as_ref().unwrap().open, Some(SEEDS));
    app
}

fn made(app: &Faraday) -> usize {
    let r = app.restore.as_ref().unwrap();
    assert_eq!(r.open, Some(CHECK), "the Check card is not next");
    r.wallet.expect("no wallet made")
}

fn desc(app: &Faraday, w: usize) -> String {
    app.session.wallets[w]
        .policy
        .to_descriptor_checksummed()
        .replace('\'', "h")
}

fn plain(text: &str) -> String {
    faraday_core::wallet::read_wallet(text)
        .unwrap()
        .to_descriptor_checksummed()
        .replace('\'', "h")
}

fn drawn(app: &mut Faraday) -> String {
    app.drawn_texts().join("\n")
}

#[test]
fn restore_opens_on_kind_with_no_transaction_card() {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    assert_eq!(app.screen, Screen::Restore);
    assert_eq!(app.restore.as_ref().unwrap().open, Some(KIND));
    let text = drawn(&mut app);
    for card in ["Kind", "Description", "Seeds", "Check", "Done"] {
        assert!(text.lines().any(|l| l == card), "no {card} card: {text}");
    }
    assert!(!text.contains("transaction"), "{text}");
    // The shortcut, and the two everyday kinds with More kinds.
    assert!(text.contains("I have the wallet description"), "{text}");
    assert!(app.offers(Action::Scan));
    assert!(app.offers(Action::RMoreKinds));
    assert!(text.contains("Single key · native SegWit"), "{text}");
    assert!(text.contains("Multisig · native SegWit"), "{text}");
    // A multisig has a Quorum card, 2 of 3 to start.
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    assert_eq!(app.restore.as_ref().unwrap().open, Some(QUORUM));
    let s = app.restore.as_ref().unwrap().seeds.as_ref().unwrap();
    assert_eq!((s.m, s.n), (2, 3));
    assert!(drawn(&mut app).contains("2 of 3"));
}

#[test]
fn three_typed_seeds_one_with_a_passphrase_make_the_wallet_sparrow_makes() {
    let mut app = two_of_three_from_seeds();
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    passphrase(&mut app, "TREZOR", "TREZOR");
    type_seed(&mut app, &testkit::test_words("summer"));
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    assert_eq!(desc(&app, w), PASSPHRASE_VECTOR);
}

#[test]
fn passphrases_that_differ_take_no_seed() {
    let mut app = two_of_three_from_seeds();
    passphrase(&mut app, "TREZOR", "TREZRO");
    app.press(Action::RSlotWords);
    assert_eq!(app.screen, Screen::Restore);
    assert_eq!(
        app.restore.as_ref().unwrap().error.as_deref(),
        Some("The two passphrases differ")
    );
}

#[test]
fn the_description_scanned_first_fills_the_quorum_and_refuses_what_is_not_its_key() {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: testkit::savings().into_bytes(),
    });
    let r = app.restore.as_ref().unwrap();
    assert_eq!(r.open, Some(SEEDS), "the flow is not at Seeds");
    assert!(r.done[KIND as usize] && r.done[QUORUM as usize] && r.done[DESCRIPTION as usize]);
    let s = r.seeds.as_ref().unwrap();
    assert_eq!((s.m, s.n), (2, 3));
    let keys = app.session.keys.len();

    // A seed of another wallet.
    type_seed(&mut app, ABANDON);
    assert_eq!(
        app.restore.as_ref().unwrap().error.as_deref(),
        Some(NOT_A_KEY)
    );
    assert_eq!(app.session.keys.len(), keys, "the stranger was taken");
    assert!(drawn(&mut app).contains(NOT_A_KEY));

    // The right words with a passphrase the wallet's key does not have.
    passphrase(&mut app, "wrong", "wrong");
    type_seed(&mut app, &testkit::test_words("zebra"));
    assert_eq!(
        app.restore.as_ref().unwrap().error.as_deref(),
        Some(NOT_A_KEY)
    );
    assert_eq!(
        app.session.keys.len(),
        keys,
        "the wrong passphrase was taken"
    );

    // The right words alone: its key of the wallet.
    for f in 0..2 {
        app.press(Action::RPassField(f));
        for _ in 0.."wrong".len() {
            app.event(Event::Key(Key::Backspace));
        }
    }
    type_seed(&mut app, &testkit::test_words("zebra"));
    let r = app.restore.as_ref().unwrap();
    assert_eq!(r.error, None);
    let place = app
        .session
        .slots(&app.session.wallets[r.wallet.unwrap()])
        .iter()
        .position(|s| s.held_by.is_some())
        .unwrap();
    let said = format!("Key {} of the wallet", place + 1);
    assert!(drawn(&mut app).contains(&said));

    // The two others are not here: the wallet is made.
    for k in 0..3u8 {
        if usize::from(k) != place {
            app.press(Action::RAbsent(k));
        }
    }
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    assert_eq!(desc(&app, w), plain(&testkit::savings()));
}

#[test]
fn two_seeds_and_a_cosigners_xpub_make_the_two_of_three() {
    let mut app = two_of_three_from_seeds();
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    app.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(&mut app, &testkit::key(2, MULTI_PATH));
    app.event(Event::Key(Key::Enter));
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    assert_eq!(desc(&app, w), plain(&testkit::savings()));
}

#[test]
fn done_offers_the_backup_and_a_spend_from_this_wallet() {
    let mut app = two_of_three_from_seeds();
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    type_seed(&mut app, &testkit::test_words("summer"));
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    app.press(Action::RNext(CHECK));
    assert_eq!(app.restore.as_ref().unwrap().open, Some(DONE));
    let text = drawn(&mut app);
    assert!(text.contains("Back up this wallet"), "{text}");
    assert!(text.contains("Spend from this wallet"), "{text}");
    assert!(app.offers(Action::Backup(w)));
    assert!(app.offers(Action::Family(F::SpendFrom(w))));
    assert!(app.offers(Action::OpenWallet(w)));
    // Restored seeds are backed up already: the spend opens at once.
    app.press(Action::Family(F::SpendFrom(w)));
    assert_eq!(app.screen, Screen::Family);
}
