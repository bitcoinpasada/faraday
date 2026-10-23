//! Restore a wallet made elsewhere (`docs/history/NEW-WALLET.md` §12): what kind
//! it is, its quorum, its description or none, then a slot per key for its
//! seed, with a passphrase if it has one, or a cosigner's xpub; the
//! wallet, its first addresses, and the backup or a spend after.

use crate::common::type_text;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::backup_flow::steps::FlowAction;
use faraday_core::create::{NewKind, Source};
use faraday_core::family::FamilyAction as F;
use faraday_core::rstep::{CHECK, DESCRIPTION, DONE, HOW_MANY, KIND, QUORUM, SEEDS, WHERE};
use faraday_core::seeds::read::Choice;
use faraday_core::seeds::{Focus, SeedsAction as S};
use faraday_core::{Action, Faraday, NOT_A_KEY, Screen, Sheet, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

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
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app
}

/// The open slot's Type the words, with these words and, on Add a key,
/// this passphrase.
fn type_seed_with(app: &mut Faraday, words: &str, pass: &str) {
    app.press(Action::RSlotWords);
    assert_eq!(app.screen, Screen::Entry);
    type_text(app, words);
    app.press(Action::EntryPassphrase);
    type_text(app, pass);
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Restore, "Add a key did not return");
}

/// The open slot's Scan a SeedQR, reading these words' SeedQR: the sheet
/// asks how it loads.
fn scan_seed(app: &mut Faraday, words: &str) {
    let c = testkit::compute();
    app.press(Action::RSlotScan);
    let m = Mnemonic::parse(&c, Language::English, words).unwrap();
    app.event(Event::Scanned {
        bytes: faraday_codec::seedqr::to_digits(&m)
            .expose()
            .as_bytes()
            .to_vec(),
    });
    let _ = app.frame();
    assert_eq!(
        app.sheet,
        Some(Sheet::SeedRead),
        "the seed asks how it loads"
    );
}

/// The SeedQR sheet's two passphrase fields, typed.
fn sheet_passphrase(app: &mut Faraday, first: &str, again: &str) {
    app.press(Action::SeedRead(Choice::AddPass));
    app.press(Action::SeedRead(Choice::Field(0)));
    type_text(app, first);
    app.press(Action::SeedRead(Choice::Field(1)));
    type_text(app, again);
    // The key the passphrase makes shows once typing pauses.
    testkit::pause(app);
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
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));
    app
}

fn made(app: &Faraday) -> usize {
    let r = app.restore().unwrap();
    assert_eq!(r.open, Some(CHECK), "the Check card is not next");
    r.wallet.expect("no wallet made")
}

fn desc(app: &Faraday, w: usize) -> String {
    app.session.wallets[w]
        .policy()
        .to_descriptor_checksummed()
        .replace('\'', "h")
}

fn plain(text: &str) -> String {
    let c = faraday_core::testkit::compute();
    faraday_core::wallet::read_wallet(&c, text)
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
    assert_eq!(app.restore().unwrap().open, Some(KIND));
    let text = drawn(&mut app);
    for card in ["Kind", "Description", "Use vaults", "Keys", "Check"] {
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
    assert_eq!(app.restore().unwrap().open, Some(QUORUM));
    let s = app.restore().unwrap().seeds.as_ref().unwrap();
    assert_eq!((s.m, s.n), (2, 3));
    assert!(drawn(&mut app).contains("2 of 3"));
}

/// A single key's backup is usually its seed alone: Description offers
/// **I have the seed**, which opens Seeds; a multisig's says **I do not
/// have it**.
#[test]
fn a_single_key_goes_on_from_description_with_the_seed_alone() {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(KIND));
    assert_eq!(app.restore().unwrap().open, Some(DESCRIPTION));
    let text = drawn(&mut app);
    assert!(text.lines().any(|l| l == "I have the seed"), "{text}");
    assert!(!text.contains("I do not have it"), "{text}");
    assert!(app.offers(Action::RSeeds));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));

    let mut app = shown();
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    app.press(Action::RNext(QUORUM));
    let text = drawn(&mut app);
    assert!(text.lines().any(|l| l == "I do not have it"), "{text}");
    assert!(!text.contains("I have the seed"), "{text}");
}

#[test]
fn three_typed_seeds_one_with_a_passphrase_make_the_wallet_sparrow_makes() {
    let mut app = two_of_three_from_seeds();
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    type_seed_with(&mut app, &testkit::test_words("summer"), "TREZOR");
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    assert_eq!(desc(&app, w), PASSPHRASE_VECTOR);
}

/// The third seed scanned as a SeedQR, its passphrase typed twice on the
/// scan's sheet: the same wallet Sparrow makes.
#[test]
fn a_seedqr_with_its_passphrase_on_the_sheet_makes_the_wallet_sparrow_makes() {
    let mut app = two_of_three_from_seeds();
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    let keys = app.session.keys.len();
    scan_seed(&mut app, &testkit::test_words("summer"));
    assert_eq!(
        app.session.keys.len(),
        keys,
        "nothing loads before the choice"
    );
    sheet_passphrase(&mut app, "TREZOR", "TREZOR");
    // The key the passphrase makes, before it loads.
    assert!(drawn(&mut app).contains("9f071687"), "its key is shown");
    app.press(Action::SeedRead(Choice::WithPass));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    assert_eq!(desc(&app, w), PASSPHRASE_VECTOR);
}

/// A key already loaded from its words, with no passphrase, taken into a
/// slot asks whether it has one, as a vault keeping a seed apart from its
/// passphrase needs: with TREZOR it is the wallet Sparrow makes.
#[test]
fn a_loaded_key_taken_into_a_slot_asks_for_its_passphrase() {
    let mut app = shown();
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words("summer"));
    app.press(Action::EntryAdd);
    let summer = app.session.keys[0].fingerprint;
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    app.press(Action::RNext(QUORUM));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    app.press(Action::Seeds(S::UseLoaded(summer.0)));
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "it asks first");
    sheet_passphrase(&mut app, "TREZOR", "TREZOR");
    app.press(Action::SeedRead(Choice::WithPass));
    assert_eq!(app.sheet, None);
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    assert_eq!(desc(&app, w), PASSPHRASE_VECTOR);
}

#[test]
fn passphrases_that_differ_on_the_sheet_load_no_seed() {
    let mut app = two_of_three_from_seeds();
    let keys = app.session.keys.len();
    scan_seed(&mut app, &testkit::test_words("summer"));
    sheet_passphrase(&mut app, "TREZOR", "TREZRO");
    app.press(Action::SeedRead(Choice::WithPass));
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "the sheet stays");
    assert_eq!(
        app.seed_read().unwrap().error.as_deref(),
        Some("The two passphrases differ")
    );
    assert_eq!(app.session.keys.len(), keys);
    // Cancel wipes what was read and loads nothing.
    app.press(Action::Cancel);
    assert!(app.seed_read().is_none());
    assert_eq!(app.session.keys.len(), keys);
}

#[test]
fn the_description_scanned_first_fills_the_quorum_and_refuses_what_is_not_its_key() {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: testkit::savings().into_bytes(),
    });
    let r = app.restore().unwrap();
    assert_eq!(r.open, Some(WHERE), "the question of vaults is next");
    assert!(r.done[KIND as usize] && r.done[QUORUM as usize] && r.done[DESCRIPTION as usize]);
    let s = r.seeds.as_ref().unwrap();
    assert_eq!((s.m, s.n), (2, 3));
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));
    let keys = app.session.keys.len();

    // A seed of another wallet.
    type_seed(&mut app, ABANDON);
    assert_eq!(app.restore().unwrap().error.as_deref(), Some(NOT_A_KEY));
    assert_eq!(app.session.keys.len(), keys, "the stranger was taken");
    assert!(drawn(&mut app).contains(NOT_A_KEY));

    // The right seed scanned, with a passphrase the wallet's key does
    // not have: the sheet says so and stays, nothing loaded.
    scan_seed(&mut app, &testkit::test_words("zebra"));
    sheet_passphrase(&mut app, "wrong", "wrong");
    app.press(Action::SeedRead(Choice::WithPass));
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert_eq!(app.seed_read().unwrap().error.as_deref(), Some(NOT_A_KEY));
    assert_eq!(
        app.session.keys.len(),
        keys,
        "the wrong passphrase was taken"
    );

    // The same seed without a passphrase: its key of the wallet.
    app.press(Action::SeedRead(Choice::Plain));
    assert_eq!(app.sheet, None);
    let r = app.restore().unwrap();
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
    assert!(
        summary_is(&mut app, "Keys", "3 of 3 filled"),
        "{}",
        drawn(&mut app)
    );
}

/// Whether a closed card titled `title` reads `summary` under it.
fn summary_is(app: &mut Faraday, title: &str, summary: &str) -> bool {
    app.drawn_texts()
        .windows(2)
        .any(|w| w[0] == title && w[1] == summary)
}

/// Made from three typed seeds, the wallet's Keys card reads every slot
/// filled, at Check and at Done.
#[test]
fn keys_reads_every_slot_filled_once_the_wallet_is_made_from_typed_seeds() {
    let mut app = two_of_three_from_seeds();
    for word in ["bacon", "zebra", "summer"] {
        type_seed(&mut app, &testkit::test_words(word));
    }
    app.press(Action::Seeds(S::Make));
    made(&app);
    assert!(
        summary_is(&mut app, "Keys", "3 of 3 filled"),
        "{}",
        drawn(&mut app)
    );
    app.press(Action::RNext(CHECK));
    assert_eq!(app.restore().unwrap().open, Some(DONE));
    assert!(
        summary_is(&mut app, "Keys", "3 of 3 filled"),
        "{}",
        drawn(&mut app)
    );
}

/// The description read before the question of vaults: the question is
/// asked in its turn, before the seeds, and reads its answer at Done.
#[test]
fn a_description_read_first_asks_about_vaults_before_the_seeds() {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: testkit::savings().into_bytes(),
    });
    assert_eq!(app.restore().unwrap().open, Some(WHERE));
    // The seeds wait for the answer.
    app.press(Action::RStep(SEEDS));
    assert_ne!(app.restore().unwrap().open, Some(SEEDS));
    app.press(Action::RStep(WHERE));
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));
    type_seed(&mut app, &testkit::test_words("zebra"));
    let w = app.restore().unwrap().wallet.unwrap();
    let held = app
        .session
        .slots(&app.session.wallets[w])
        .iter()
        .position(|s| s.held_by.is_some())
        .unwrap();
    for k in (0..3u8).filter(|&k| usize::from(k) != held) {
        app.press(Action::RAbsent(k));
    }
    app.press(Action::Seeds(S::Make));
    made(&app);
    app.press(Action::RNext(CHECK));
    let r = app.restore().unwrap();
    assert_eq!(r.open, Some(DONE));
    assert!(r.done[WHERE as usize], "the question is answered");
    assert!(
        summary_is(&mut app, "Use vaults", "No"),
        "{}",
        drawn(&mut app)
    );
    assert!(
        summary_is(&mut app, "Keys", "3 of 3 filled"),
        "{}",
        drawn(&mut app)
    );
}

/// The description read first and one vault set up after it: Back up
/// this wallet resumes the wallet's backup at the plan, with that vault.
#[test]
fn a_description_read_first_then_a_vault_resumes_the_backup_with_the_vault() {
    let mut shell = crate::common::Shell::new();
    let mut d = crate::common::Disk::empty("S1", "KEYSTICK");
    d.serial = Some(1);
    shell.add(d);
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::keep_time(&mut app);
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: testkit::savings().into_bytes(),
    });
    app.press(Action::RVaults(true));
    app.press(Action::RNext(WHERE));
    app.press(Action::RVaultN(1));
    app.press(Action::RNext(HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, FlowAction::Tap);
    shell.pull(&mut app, "S1");
    crate::common::create_vault(&mut shell, &mut app, 0, "first vault phrase");
    assert_eq!(app.screen, Screen::Restore, "back in Restore for the seeds");
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));
    type_seed(&mut app, &testkit::test_words("zebra"));
    let w = app.restore().unwrap().wallet.unwrap();
    let held = app
        .session
        .slots(&app.session.wallets[w])
        .iter()
        .position(|s| s.held_by.is_some())
        .unwrap();
    for k in (0..3u8).filter(|&k| usize::from(k) != held) {
        app.press(Action::RAbsent(k));
    }
    app.press(Action::Seeds(S::Make));
    made(&app);
    app.press(Action::RNext(CHECK));
    shell.pump(&mut app);
    let _ = app.frame();
    assert_eq!(app.restore().unwrap().open, Some(DONE));
    assert!(
        summary_is(&mut app, "Use vaults", "Yes"),
        "{}",
        drawn(&mut app)
    );
    app.press(Action::Backup(w));
    assert_eq!(app.screen, Screen::Backup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(
        app.backup.as_ref().unwrap().answers.vaults.len(),
        1,
        "the vault set up is the plan's"
    );
}

/// Done offers the four ways on, each doing what it says
/// (`docs/history/NEW-WALLET.md` §12.3).
#[test]
fn done_offers_a_spend_the_backup_another_wallet_and_wallets() {
    let mut app = two_of_three_from_seeds();
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    type_seed(&mut app, &testkit::test_words("summer"));
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    app.press(Action::RNext(CHECK));
    assert_eq!(app.restore().unwrap().open, Some(DONE));
    let text = drawn(&mut app);
    for said in [
        "Spend from this wallet",
        "Back up this wallet",
        "Another wallet from these seeds",
    ] {
        assert!(text.contains(said), "no {said}: {text}");
    }
    assert!(app.offers(Action::Family(F::SpendFrom(w))));
    assert!(app.offers(Action::Backup(w)));
    assert!(app.offers(Action::NewWalletFrom(w)));
    assert!(app.offers(Action::OpenWallet(w)));
    // Restored seeds are backed up already: the spend opens at once.
    app.press(Action::Family(F::SpendFrom(w)));
    assert_eq!(app.screen, Screen::Family);
}

/// Done, after It matches on a single key restored with no vaults.
fn single_key_done() -> (Faraday, usize) {
    let mut shell = crate::common::Shell::new();
    let (mut app, w) = single_key_at_check(&mut shell, 0);
    app.press(Action::RNext(CHECK));
    let _ = app.frame();
    (app, w)
}

#[test]
fn back_up_this_wallet_opens_back_up_on_it() {
    let (mut app, w) = single_key_done();
    app.press(Action::Backup(w));
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup.as_ref().unwrap().wallet, w);
}

#[test]
fn wallets_opens_the_wallet() {
    let (mut app, w) = single_key_done();
    app.press(Action::OpenWallet(w));
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.wallet_index(), w);
}

/// Another wallet from these seeds is New wallet from this one: Create
/// filled from the restored wallet, at the next account (§16.2).
#[test]
fn another_wallet_from_these_seeds_opens_create_at_the_next_account() {
    let (mut app, w) = single_key_done();
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::NewWalletFrom(w));
    assert_eq!(app.screen, Screen::Create);
    let c = app.create().unwrap();
    assert_eq!(c.kind, NewKind::NativeSegwit);
    assert_eq!(c.account, 1, "the next account the seed does not use");
    assert_eq!(c.slots, vec![Source::Here(fp)], "the restored seed");
}

/// Restored with two vaults, at Done.
fn two_vaults_done() -> (crate::common::Shell, Faraday, usize) {
    let mut shell = crate::common::Shell::new();
    let (mut app, w) = single_key_at_check(&mut shell, 2);
    app.press(Action::RNext(CHECK));
    let _ = app.frame();
    (shell, app, w)
}

/// With vaults set up before the seeds, Back up this wallet resumes the
/// wallet's backup at the plan, with those vaults.
#[test]
fn with_vaults_back_up_this_wallet_resumes_at_the_plan_with_its_vaults() {
    let (_shell, mut app, w) = two_vaults_done();
    app.press(Action::Backup(w));
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup.as_ref().unwrap().wallet, w);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(
        app.backup.as_ref().unwrap().answers.vaults.len(),
        2,
        "the vaults set up are the plan's"
    );
    assert_eq!(app.vaults.open.len(), 2);
}

/// With vaults, Spend from this wallet opens the spend and the restored
/// key signs at once (F6).
#[test]
fn with_vaults_spend_from_this_wallet_opens_at_once() {
    let c = testkit::compute();
    let (_shell, mut app, w) = two_vaults_done();
    assert!(
        app.held_keys_now(&c).is_empty(),
        "a restored key signs at once"
    );
    let (label, action) = app.spend_button(&c, w);
    assert_eq!(label, "Spend from this wallet");
    app.press(action);
    assert_eq!(app.screen, Screen::Family);
}

/// Left for Wallets, the wallet's card and Backups offer the backup left
/// part way; it resumes at the plan.
#[test]
fn the_wallets_card_and_backups_offer_the_unfinished_backup() {
    let (_shell, mut app, w) = two_vaults_done();
    app.press(Action::OpenWallet(w));
    assert!(drawn(&mut app).contains("Continue the backup"));
    assert!(app.offers(Action::Backup(w)));
    app.press(Action::Nav(Screen::Backups));
    assert!(drawn(&mut app).contains("Continue the backup"));
    assert!(app.offers(Action::Backup(w)));
    app.press(Action::Backup(w));
    assert_eq!(app.screen, Screen::Backup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(app.backup.as_ref().unwrap().answers.vaults.len(), 2);

    // With no vaults, nothing is left part way.
    let (mut app, w) = single_key_done();
    app.press(Action::OpenWallet(w));
    assert!(!drawn(&mut app).contains("Continue the backup"));
}

/// A single key restored from its typed seed, with vaults (`count`) or
/// none, up to Check.
fn single_key_at_check(shell: &mut crate::common::Shell, count: usize) -> (Faraday, usize) {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::keep_time(&mut app);
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(KIND));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(count > 0));
    app.press(Action::RNext(WHERE));
    if count > 0 {
        for k in 1..=count {
            let mut d = crate::common::Disk::empty(&format!("S{k}"), "KEYSTICK");
            d.serial = Some(k as u32);
            shell.add(d);
        }
        app.press(Action::RVaultN(count as u16));
        app.press(Action::RNext(HOW_MANY));
        assert_eq!(app.screen, Screen::BackupFlow);
        for k in 1..=count {
            let d = format!("S{k}");
            shell.plug(&mut app, &d);
            shell.flow(&mut app, FlowAction::Tap);
            shell.pull(&mut app, &d);
        }
        for (v, phrase) in ["first vault phrase", "second vault phrase"]
            .iter()
            .enumerate()
            .take(count)
        {
            crate::common::create_vault(shell, &mut app, v, phrase);
        }
        assert_eq!(app.screen, Screen::Restore, "back in Restore for the seeds");
        assert_eq!(app.vaults.open.len(), count);
    }
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));
    type_seed(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    (app, w)
}

/// Restore ends on Done, whatever the question of vaults was answered
/// (`docs/history/NEW-WALLET.md` §12.3).
#[test]
fn it_matches_opens_done_with_no_vaults() {
    let mut shell = crate::common::Shell::new();
    let (mut app, _) = single_key_at_check(&mut shell, 0);
    app.press(Action::RNext(CHECK));
    assert_eq!(app.screen, Screen::Restore);
    assert_eq!(app.restore().unwrap().open, Some(DONE));
}

#[test]
fn it_matches_opens_done_with_two_vaults() {
    let mut shell = crate::common::Shell::new();
    let (mut app, _) = single_key_at_check(&mut shell, 2);
    app.press(Action::RNext(CHECK));
    shell.pump(&mut app);
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Restore);
    assert_eq!(app.restore().unwrap().open, Some(DONE));
}

/// A wallet restored from the test vault kept under the file name
/// `file`, then its Spend tab opened from the wallet's card.
fn restored_from_vault_named(file: &str) -> Faraday {
    use faraday_core::StorageEvent;
    use faraday_core::vault::VaultAction as V;
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox: vec![(file.to_string(), testkit::test_vault().unwrap())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(100);
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..20u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    let record = {
        use faraday_vault::records::{field, kind};
        app.vaults
            .store()
            .contents_for_tests(app.vaults.open[0].id)
            .of(kind::WALLET)
            .find(|(_, r)| r.text(field::WALLET_NAME) == Some("Spending"))
            .map(|(i, _)| i)
            .expect("the vault's Spending wallet")
    };
    app.press(Action::RFromVault(0, record));
    let w = app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Spending")
        .expect("the wallet was not restored");
    app.press(Action::Family(F::SpendFrom(w)));
    assert_eq!(app.screen, Screen::Family);
    app
}

/// A wallet restored from a vault is a vault's wallet, whatever the
/// vault is called.
#[test]
fn a_wallet_restored_from_a_vault_with_any_name_goes_the_vault_route() {
    use faraday_core::family::Route;
    let app = restored_from_vault_named("Savings vault.ofv");
    assert_eq!(app.family().route, Some(Route::Vault));
}

#[test]
fn a_wallet_restored_from_a_vault_named_vault_goes_the_vault_route() {
    use faraday_core::family::Route;
    let app = restored_from_vault_named("Vault");
    assert_eq!(app.family().route, Some(Route::Vault));
}
