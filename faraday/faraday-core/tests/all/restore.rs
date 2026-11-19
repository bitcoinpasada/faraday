//! Restore a wallet made elsewhere: what kind
//! it is, its quorum, its description or none, then a slot per key for its
//! seed, with a passphrase if it has one, or a cosigner's xpub; the
//! wallet, its first addresses, and the backup or a spend after.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::RecordsOf as _;
use crate::common::type_text;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::backup_flow::steps::FlowAction;
use faraday_core::create::{NewKind, Source};
use faraday_core::rstep::{CHECK, DESCRIPTION, DONE, HOW_MANY, KIND, QUORUM, SEEDS, WHERE};
use faraday_core::seeds::read::Choice;
use faraday_core::seeds::{Focus, SLIDE_N, SeedsAction as S};
use faraday_core::spend_backup::SpendBackupAction as F;
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
    app.press(Action::PassEye);
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(app);
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
    faraday_core::testkit::keep_published(app);
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
    assert!(text.lines().any(|l| l == "Kind"), "no Kind card: {text}");
    for card in ["Description", "Use vaults", "Keys"] {
        assert!(
            !text.lines().any(|l| l == card),
            "{card} before it is reached: {text}"
        );
    }
    assert!(!text.contains("transaction"), "{text}");
    // The shortcut, and the two everyday kinds with More kinds.
    assert!(text.contains("I have the descriptor"), "{text}");
    assert!(app.offers(Action::ScanFor(faraday_core::Ask::Description)));
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
    faraday_core::testkit::confirm_wallets(&mut app);
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
    faraday_core::testkit::confirm_wallets(&mut app);
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
    faraday_core::testkit::keep_published(&mut app);
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
    faraday_core::testkit::confirm_wallets(&mut app);
    let w = made(&app);
    assert_eq!(desc(&app, w), PASSPHRASE_VECTOR);
}

#[test]
fn the_sheets_button_waits_and_the_line_shows_while_the_passphrases_differ() {
    use crate::common::{has, offered};
    let mut app = two_of_three_from_seeds();
    scan_seed(&mut app, &testkit::test_words("summer"));
    sheet_passphrase(&mut app, "TREZOR", "TREZRO");
    assert!(has(&mut app, "The two passphrases differ"));
    assert!(!offered(&mut app, Action::SeedRead(Choice::WithPass)));
    app.press(Action::SeedRead(Choice::Field(1)));
    app.event(Event::Key(Key::Backspace));
    app.event(Event::Key(Key::Backspace));
    type_text(&mut app, "OR");
    assert!(!has(&mut app, "The two passphrases differ"));
    assert!(offered(&mut app, Action::SeedRead(Choice::WithPass)));
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
    faraday_core::testkit::confirm_wallets(&mut app);
    let r = app.restore().unwrap();
    assert_eq!(r.open, Some(WHERE), "the question of vaults is next");
    assert!(r.done[KIND as usize] && r.done[QUORUM as usize] && r.done[DESCRIPTION as usize]);
    let s = r.seeds.as_ref().unwrap();
    assert_eq!((s.m, s.n), (2, 3));
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));
    let keys = app.session.keys.len();

    // A seed of another wallet: refused on Add a key, words kept.
    app.press(Action::RSlotWords);
    type_text(&mut app, ABANDON);
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(app.session.keys.len(), keys, "the stranger was taken");
    assert!(drawn(&mut app).contains(NOT_A_KEY));
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.screen, Screen::Restore);

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
    faraday_core::testkit::confirm_wallets(&mut app);
    let w = made(&app);
    assert_eq!(desc(&app, w), plain(&testkit::savings()));
}

#[test]
fn two_seeds_and_a_cosigners_xpub_make_the_two_of_three() {
    let mut app = two_of_three_from_seeds();
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    // The arrow keys move the slider last used: one more key.
    app.press(Action::Slide(SLIDE_N, 2));
    app.event(Event::Key(Key::Right));
    app.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(&mut app, &testkit::key(2, MULTI_PATH));
    app.event(Event::Key(Key::Enter));
    app.press(Action::Seeds(S::Make));
    // The cosigner's xpub is no loaded key's own: the wallet waits for its
    // sheet, and Restore goes on from it once held.
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WalletConfirm));
    assert_eq!(app.restore().unwrap().wallet, None);
    assert_eq!(app.restore().unwrap().error, None);
    app.press(Action::WalletHold);
    let w = made(&app);
    assert_eq!(desc(&app, w), plain(&testkit::savings()));
    assert!(
        summary_is(&mut app, "Keys", "3 of 3 filled"),
        "{}",
        drawn(&mut app)
    );
}

/// A key ticked on the sheet of a wallet made from typed seeds is kept by
/// the wallet added.
#[test]
fn a_key_ticked_on_the_restore_sheet_stays_ticked_on_the_wallet() {
    let mut app = two_of_three_from_seeds();
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    app.press(Action::Slide(SLIDE_N, 2));
    app.event(Event::Key(Key::Right));
    app.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(&mut app, &testkit::key(2, MULTI_PATH));
    app.event(Event::Key(Key::Enter));
    app.press(Action::Seeds(S::Make));
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WalletConfirm));
    for key in 0..3 {
        app.press(Action::KeyTick(
            faraday_core::wallet::KeyBlockOf::Asked,
            key,
        ));
    }
    app.press(Action::WalletHold);
    let w = made(&app);
    let ticks = app.session.wallets[w].key_ticks();
    assert_eq!(ticks.iter().filter(|t| **t).count(), 1, "{ticks:?}");
    assert_eq!(app.session.wallets[w].keys_unchecked(), 0);
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
    faraday_core::testkit::confirm_wallets(&mut app);
    let w = made(&app);
    assert_eq!(desc(&app, w), plain(&testkit::savings()));
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
    faraday_core::testkit::confirm_wallets(&mut app);
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
    faraday_core::testkit::confirm_wallets(&mut app);
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

/// A seed loaded for a wallet that has its key in two slots fills both:
/// neither reads "Not here" after it.
#[test]
fn a_seed_filling_two_slots_of_the_wallet_clears_both_not_here_marks() {
    let c = faraday_core::testkit::compute();
    let d = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(0, MULTI_PATH),
        testkit::key(0, "m/48'/1'/1'/2'"),
        testkit::key(1, MULTI_PATH)
    );
    let d = faraday_bip::policy::WalletPolicy::parse_any(&c, &d)
        .map(|w| String::from(&*w.to_descriptor_checksummed()))
        .unwrap_or(d);
    let mut app = shown();
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: d.into_bytes(),
    });
    faraday_core::testkit::confirm_wallets(&mut app);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    let w = app.restore().unwrap().wallet.unwrap();
    let slots = app.session.slots(&app.session.wallets[w]).len();
    assert_eq!(slots, 3);
    for k in 0..3u8 {
        app.press(Action::RAbsent(k));
    }
    assert_eq!(app.restore().unwrap().absent.len(), 3);
    type_seed(&mut app, &testkit::test_words("bacon"));
    assert_eq!(app.restore().unwrap().error, None);
    assert_eq!(
        app.restore().unwrap().absent,
        vec![2],
        "only the slot of the other key is Not here"
    );
}

/// Restore asked for 129 vaults sets up 128.
#[test]
fn restore_asked_for_129_vaults_gets_128() {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(KIND));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(true));
    app.press(Action::RNext(WHERE));
    app.press(Action::RVaultN(129));
    assert_eq!(app.restore().unwrap().vaults.count, 128);
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
    faraday_core::testkit::confirm_wallets(&mut app);
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
    faraday_core::testkit::confirm_wallets(&mut app);
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
        app.backup().unwrap().answers.vaults.len(),
        1,
        "the vault set up is the plan's"
    );
}

/// Done is the one end card: the wallet restored, where it came from, Show
/// wallet QR filled, then a spend, Back up this wallet while it has none,
/// Open the wallet, Another wallet, Lock and Power off; no button reads
/// "Wallets".
#[test]
fn done_is_the_end_card_with_a_spend_the_backup_and_another_wallet() {
    let mut app = two_of_three_from_seeds();
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    type_seed(&mut app, &testkit::test_words("summer"));
    app.press(Action::Seeds(S::Make));
    faraday_core::testkit::confirm_wallets(&mut app);
    let w = made(&app);
    app.press(Action::RNext(CHECK));
    assert_eq!(app.restore().unwrap().open, Some(DONE));
    let text = drawn(&mut app);
    for said in [
        "Wallet restored",
        "Restored from",
        "Done",
        "not recorded here",
        "Show wallet QR",
        "Spend from this wallet",
        "Back up this wallet",
        "Open the wallet",
        "Another wallet from these seeds",
    ] {
        assert!(text.contains(said), "no {said}: {text}");
    }
    assert!(app.offers(Action::QrWallet(w)));
    assert!(app.offers(Action::LockAsk));
    assert!(app.offers(Action::SpendBackup(F::SpendFrom(w))));
    assert!(app.offers(Action::BQuick(
        faraday_core::wallet_backup::quick::Quick::Open(w)
    )));
    assert!(app.offers(Action::NewWalletFrom(w)));
    assert!(app.offers(Action::OpenWallet(w)));
    // Restored seeds are backed up already: the spend opens at once.
    app.press(Action::SpendBackup(F::SpendFrom(w)));
    assert_eq!(app.screen, Screen::SpendBackup);
}

/// Back from a Restore opened from Backups, Tools or Home goes to that
/// screen, and so does Done on the end card.
#[test]
fn back_and_done_return_to_where_restore_was_opened() {
    for from in [Screen::Backups, Screen::Catalog, Screen::Home] {
        let mut app = shown();
        app.press(Action::Nav(from));
        assert_eq!(app.screen, from);
        app.press(Action::RestoreWallet);
        assert_eq!(app.screen, Screen::Restore);
        let _ = app.frame();
        assert!(app.offers(Action::Nav(from)), "no Back to {from:?}");
        app.press(Action::Nav(from));
        assert_eq!(app.screen, from);
    }
    let mut app = two_of_three_from_seeds();
    app.press(Action::Nav(Screen::Backups));
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    app.press(Action::RNext(QUORUM));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    type_seed(&mut app, &testkit::test_words("bacon"));
    type_seed(&mut app, &testkit::test_words("zebra"));
    type_seed(&mut app, &testkit::test_words("summer"));
    app.press(Action::Seeds(S::Make));
    faraday_core::testkit::confirm_wallets(&mut app);
    app.press(Action::RNext(CHECK));
    assert_eq!(app.restore().unwrap().open, Some(DONE));
    assert!(drawn(&mut app).contains("Done"));
    app.press(Action::Nav(Screen::Backups));
    assert_eq!(app.screen, Screen::Backups);
}

/// A wallet made from a loaded key is made, not restored.
#[test]
fn a_wallet_from_a_loaded_key_ends_on_wallet_made() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Wallets));
    app.press(Action::Entry(None));
    for c in ABANDON.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    let _ = app.frame();
    let fp = app.session.keys.all().next().unwrap().fingerprint.0;
    app.press(Action::Nav(Screen::Wallets));
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    faraday_core::testkit::confirm_wallets(&mut app);
    app.press(Action::RNext(CHECK));
    assert_eq!(app.restore().unwrap().open, Some(DONE));
    let text = drawn(&mut app);
    assert!(text.contains("Wallet made"), "{text}");
    assert!(!text.contains("Restored from"), "{text}");
    assert!(!text.contains("Wallet restored"), "{text}");
    assert!(!text.contains("not recorded here"), "{text}");
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
    app.press(Action::BQuick(
        faraday_core::wallet_backup::quick::Quick::Open(w),
    ));
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup().unwrap().wallet, w);
}

#[test]
fn open_the_wallet_opens_the_wallet() {
    let (mut app, w) = single_key_done();
    app.press(Action::OpenWallet(w));
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.wallet_index(), w);
}

/// Another wallet from these seeds is New wallet from this one: Create
/// filled from the restored wallet, at the next account (NEW-WALLET §16.2).
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
    assert_eq!(app.backup().unwrap().wallet, w);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(
        app.backup().unwrap().answers.vaults.len(),
        2,
        "the vaults set up are the plan's"
    );
    assert_eq!(app.vaults().open.len(), 2);
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
    assert_eq!(app.screen, Screen::SpendBackup);
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
    assert_eq!(app.backup().unwrap().answers.vaults.len(), 2);

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
        assert_eq!(app.vaults().open.len(), count);
    }
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));
    type_seed(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::Seeds(S::Make));
    faraday_core::testkit::confirm_wallets(&mut app);
    let w = made(&app);
    (app, w)
}

/// Restore on one seed with no description, the seed typed, then `presses`
/// and Make: the Check card.
fn seed_alone_at_check(kind: Option<u8>, presses: &[Action]) -> Faraday {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    if let Some(k) = kind {
        app.press(Action::RKind(k));
    }
    app.press(Action::RNext(KIND));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    type_seed(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    for a in presses {
        app.press(*a);
    }
    app.press(Action::Seeds(S::Make));
    made(&app);
    app
}

/// A seed alone gives Native SegWit at account 0 and the Check card says
/// so; the Keys card offers another account, and the wallet it makes says
/// the account it assumed and that its backup needs the description.
#[test]
fn a_restore_from_the_seed_alone_says_the_account_and_script_it_assumed() {
    let mut app = seed_alone_at_check(None, &[]);
    let text = drawn(&mut app);
    assert!(
        text.contains(
            "Made from the seed alone: Single key · native SegWit, account 0, m/84'/1'/0'"
        ),
        "{text}"
    );
    assert!(text.contains("It matches my record"), "{text}");
    assert!(text.contains("I have nothing to compare"), "{text}");
    let w = made(&app);
    assert!(!app.plan_shape_now(w).seed_alone_fails);
    let zero = desc(&app, w);

    let mut app = seed_alone_at_check(None, &[Action::Seeds(S::Account(true))]);
    let text = drawn(&mut app);
    assert!(text.contains("account 1, m/84'/1'/1'"), "{text}");
    let w = made(&app);
    // The account-1 key of the zebra seed, derived a second time outside
    // the repository by a plain Python BIP-32 (no shared code; the same
    // derivation gives the published 2-of-3 vector's tpub for this seed at
    // m/48'/1'/0'/2'). No other BIP-32 implementation was to hand.
    assert!(
        desc(&app, w).contains(
            "[5d388376/84h/1h/1h]tpubDCiB1iLoNxaajbgCAJYPcfs9ZwFdUmAmRzLAnTeYCQ6RqcFd1MBAp2paCByiNfRdyC7Vgw8WhT4zpcExsGZtCQwZMKQAHHjEY91NxihBLDS"
        ),
        "the key is taken at account 1: {}",
        desc(&app, w)
    );
    assert_ne!(desc(&app, w), zero, "another account is another wallet");
    assert!(
        app.plan_shape_now(w).seed_alone_fails,
        "the seed alone does not give account 1 back"
    );

    // Taproot at account 0 is a script the seed does not say either.
    let taproot = kind_place(NewKind::Taproot);
    let mut app = seed_alone_at_check(Some(taproot), &[]);
    let text = drawn(&mut app);
    assert!(text.contains("Taproot, account 0"), "{text}");
    let w = made(&app);
    assert!(desc(&app, w).starts_with("tr(["), "{}", desc(&app, w));
    assert!(app.plan_shape_now(w).seed_alone_fails);
}

/// The Check card reads the wallet made, not the Keys card: changing the
/// account after Make leaves the wallet and its line as they were.
#[test]
fn the_line_on_the_check_card_is_the_wallet_made_not_the_keys_card() {
    let mut app = seed_alone_at_check(None, &[Action::Seeds(S::Account(true))]);
    app.press(Action::Seeds(S::Account(true)));
    let text = drawn(&mut app);
    assert!(text.contains("account 1, m/84'/1'/1'"), "{text}");
    assert!(!text.contains("account 2"), "{text}");
}

/// A mainnet wallet at the testnet account `84'/1'/0'` is not the one a
/// seed alone gives on mainnet (`84'/0'/0'`): it is not counted as rebuilt
/// from the seed alone.
#[test]
fn a_mainnet_wallet_at_the_testnet_path_is_not_rebuilt_from_the_seed_alone() {
    let mut app = shown();
    testkit::move_to(&mut app.session, faraday_bip::keys::Network::Mainnet).unwrap();
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(KIND));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    // BIP-39's second vector, published but no sample, so Mainnet takes it.
    type_seed(
        &mut app,
        "legal winner thank year wave sausage worth useful legal winner thank yellow",
    );
    app.press(Action::Seeds(S::Custom));
    for _ in 0..5 {
        app.event(Event::Key(Key::Backspace));
    }
    type_text(&mut app, "1'/0'");
    app.press(Action::Seeds(S::Make));
    let w = made(&app);
    assert!(desc(&app, w).contains("/84h/1h/0h]"), "{}", desc(&app, w));
    assert!(app.plan_shape_now(w).seed_alone_fails);
}

/// The blank seed sheet of a wallet prints the path its keys are taken at,
/// which the seed alone does not say.
#[test]
fn the_blank_sheet_of_an_account_one_wallet_prints_its_path() {
    let app = seed_alone_at_check(None, &[Action::Seeds(S::Account(true))]);
    let w = made(&app);
    let net = app.session.network();
    let text = faraday_core::wallet_backup::sheet_blank(
        24,
        faraday_core::wallet_backup::BlankCopy::Both,
        Some(&app.session.wallets[w]),
        net,
        2,
    );
    assert!(text.contains("\npath: m/84'/1'/1'\n"), "{}", &*text);
    let c = testkit::compute();
    let pdf = faraday_paper::pdf::sheet(&c, &text).expect("a PDF");
    let path = b"m/84'/1'/1'";
    assert!(
        pdf.windows(path.len()).any(|x| x == path),
        "path in the PDF"
    );
}

/// A path that gives no wallet says why, where Make would be.
#[test]
fn a_bad_custom_path_says_why_there_is_no_make() {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(KIND));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    type_seed(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::Seeds(S::Custom));
    for _ in 0..20 {
        app.event(Event::Key(Key::Backspace));
    }
    let text = drawn(&mut app);
    assert!(text.contains("Type a derivation path"), "{text}");
}

/// The place of `kind` among Restore's single-key kinds.
fn kind_place(kind: NewKind) -> u8 {
    faraday_core::seeds::SeedsState::default()
        .kinds()
        .iter()
        .position(|k| *k == kind)
        .expect("a single-key kind") as u8
}

/// Restore ends on Done, whatever the question of vaults was answered.
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
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of(file),
            testkit::test_vault().unwrap(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..20u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    let record = {
        use faraday_vault::records::{field, kind};
        app.vaults()
            .store()
            .contents_for_tests(app.vaults().open[0].id)
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
    app.press(Action::SpendBackup(F::SpendFrom(w)));
    assert_eq!(app.screen, Screen::SpendBackup);
    app
}

/// A wallet restored from a vault is a vault's wallet, whatever the
/// vault is called.
#[test]
fn a_wallet_restored_from_a_vault_with_any_name_goes_the_vault_route() {
    use faraday_core::spend_backup::Route;
    let app = restored_from_vault_named("Savings vault.kdbx");
    assert_eq!(app.spend_backup().route, Some(Route::Vault));
}

#[test]
fn a_wallet_restored_from_a_vault_named_vault_goes_the_vault_route() {
    use faraday_core::spend_backup::Route;
    let app = restored_from_vault_named("Vault");
    assert_eq!(app.spend_backup().route, Some(Route::Vault));
}

/// The person's own record in an open vault is restored with no sheet.
#[test]
fn a_wallet_restored_from_a_vault_asks_for_no_sheet() {
    let app = restored_from_vault_named("Vault");
    assert_ne!(app.sheet, Some(faraday_core::Sheet::WalletConfirm));
    assert!(app.session.wallet_asked().is_none());
    let w = app
        .session
        .wallets
        .iter()
        .find(|w| w.name == "Spending")
        .expect("the wallet was restored");
    assert_eq!(w.origin, faraday_core::wallet::Origin::Vault);
}

/// A seed whose fingerprint a description names, but whose key at that
/// slot's path is not the slot's key, is not that key: the whole key is
/// matched, not its four bytes.
#[test]
fn a_seed_with_a_slots_fingerprint_but_not_its_key_is_refused() {
    let mut app = shown();
    let k = |n: usize, account: u32| testkit::key(n, &format!("m/48'/1'/{account}'/2'"));
    // Test key 2's origin, with its key at account 1 written in place of
    // its key at account 0.
    let (origin, _) = k(1, 0)
        .split_once(']')
        .map(|(o, x)| (o.to_string(), x.to_string()))
        .unwrap();
    let (_, other) = k(1, 1)
        .split_once(']')
        .map(|(o, x)| (o.to_string(), x.to_string()))
        .unwrap();
    let forged = format!("{origin}]{other}");
    let description = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{forged}/<0;1>/*,{}/<0;1>/*))",
        k(0, 0),
        k(2, 0)
    );
    app.press(Action::RestoreWallet);
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: description.into_bytes(),
    });
    faraday_core::testkit::confirm_wallets(&mut app);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));
    let keys = app.session.keys.len();
    app.press(Action::RSlotWords);
    type_text(&mut app, &testkit::test_words("zebra"));
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(testkit::entry(&app).error.as_deref(), Some(NOT_A_KEY));
    assert_eq!(app.session.keys.len(), keys, "the seed was taken");
}

/// Check's two receive rows each open their address's code, the address
/// as the row shows it, with no picture (`docs/QR.md` §7).
#[test]
fn checks_receive_rows_open_their_codes() {
    let c = testkit::compute();
    let mut shell = crate::common::Shell::new();
    let (mut app, w) = single_key_at_check(&mut shell, 0);
    for i in 0..2 {
        let want = app
            .session
            .address_shown(&c, &app.session.wallets[w], false, i);
        crate::common::opens_its_code(&mut app, faraday_core::Code::Receive(w, i), &want);
    }
}

/// A seed read at Restore's own Scan, past the seed sheet, goes into its
/// slot, and Restore stays on screen.
#[test]
fn a_seed_read_at_restores_scan_goes_into_its_slot_and_restore_stays() {
    let c = testkit::compute();
    let mut app = two_of_three_from_seeds();
    app.press(Action::ScanFor(faraday_core::Ask::Description));
    let m = Mnemonic::parse(&c, Language::English, &testkit::test_words("zebra")).unwrap();
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
    app.press(Action::SeedRead(Choice::Plain));
    assert_eq!(app.screen, Screen::Restore, "Restore was left");
    assert_eq!(app.session.keys.len(), 1);
    let fp = app.session.keys[0].fingerprint.0;
    let s = app.restore().unwrap().seeds.as_ref().unwrap();
    assert_eq!(s.keys, vec![fp], "the slot does not hold the key");
}

/// A codex32 secret read at Restore's own Scan goes into its slot as a
/// seed does, with no card between, and Restore stays on screen
/// (`docs/SCANNING.md` §4.3, §10 item 7). BIP 93's vector 1.
#[test]
fn a_codex32_secret_read_at_restores_scan_goes_into_its_slot() {
    let mut app = two_of_three_from_seeds();
    app.press(Action::ScanFor(faraday_core::Ask::Description));
    app.event(Event::Scanned {
        bytes: b"ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw".to_vec(),
    });
    faraday_core::testkit::confirm_wallets(&mut app);
    let _ = app.frame();
    assert!(app.scan().is_none(), "the scanner is still open");
    assert_eq!(app.screen, Screen::Restore, "Restore was left");
    assert_eq!(app.session.keys.len(), 1);
    let fp = app.session.keys[0].fingerprint.0;
    let s = app.restore().unwrap().seeds.as_ref().unwrap();
    assert_eq!(s.keys, vec![fp], "the slot does not hold the key");
}

/// A step's count never states a total that a later step changes: the
/// small panel's Restore reads "Step 2", not "Step 2 of 6".
#[test]
fn a_restore_s_step_count_states_no_total() {
    let mut app = shown();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    let shown = crate::common::texts(&mut app);
    assert!(
        shown.iter().any(|t| t.starts_with("Step ")),
        "a step is counted: {shown:?}"
    );
    assert!(
        !shown
            .iter()
            .any(|t| t.starts_with("Step ") && t.contains(" of ")),
        "a step count states a total: {shown:?}"
    );
}

#[test]
fn check_names_what_it_matches_compares() {
    let mut shell = crate::common::Shell::new();
    let (mut app, _) = single_key_at_check(&mut shell, 0);
    let texts = drawn(&mut app);
    assert!(
        texts.contains("Same in your wallet software?"),
        "the addresses are compared against the wallet software: {texts}"
    );
    assert!(texts.contains("It matches"), "{texts}");
}

/// Restore on the saved 2-of-3, open on its seeds.
fn described_to_seeds() -> Faraday {
    let mut app = shown();
    app.press(Action::RestoreWallet);
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: testkit::savings().into_bytes(),
    });
    faraday_core::testkit::confirm_wallets(&mut app);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    assert_eq!(app.restore().unwrap().open, Some(SEEDS));
    app
}

#[test]
fn restore_refusing_typed_words_stays_on_add_a_key_with_the_words_and_passphrase() {
    let mut app = described_to_seeds();
    let keys = app.session.keys.len();
    app.press(Action::RSlotWords);
    type_text(&mut app, ABANDON);
    app.press(Action::EntryPassphrase);
    type_text(&mut app, "TREZOR");
    app.press(Action::PassEye);
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Entry, "the refusal stays on Add a key");
    let e = testkit::entry(&app);
    assert_eq!(e.typed.as_str().trim(), ABANDON, "the words are kept");
    assert_eq!(e.passphrase.as_str(), "TREZOR", "the passphrase is kept");
    assert_eq!(e.error.as_deref(), Some(NOT_A_KEY));
    assert_eq!(app.session.keys.len(), keys);
}

#[test]
fn restore_refusing_seed_xor_parts_loads_no_key() {
    let mut app = described_to_seeds();
    let keys = app.session.keys.len();
    app.press(Action::RSlotWords);
    app.press(Action::EntryForm(3));
    for part in [
        "accident stock globe super snake hero sample law fit blade scale silly",
        "insect shoe rescue viable grab then adult rigid funny forget peace napkin",
    ] {
        type_text(&mut app, part);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.screen, Screen::Entry, "the refusal stays on Add a key");
    assert_eq!(testkit::entry(&app).error.as_deref(), Some(NOT_A_KEY));
    assert_eq!(app.session.keys.len(), keys, "the refused key was loaded");
}

#[test]
fn a_key_already_loaded_typed_into_a_slot_fills_it() {
    let mut app = shown();
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words("bacon"));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    let bacon = app.session.keys[0].fingerprint;
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(MULTI));
    app.press(Action::RNext(KIND));
    app.press(Action::RNext(QUORUM));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(WHERE));
    type_seed(&mut app, &testkit::test_words("bacon"));
    assert_eq!(app.session.keys.len(), 1, "no second copy");
    let s = app.restore().unwrap().seeds.as_ref().unwrap();
    assert!(s.keys.contains(&bacon.0), "the slot is filled");
}

/// Restore's slot, one-column panel: the window is 480 by 640.
fn compact_slot() -> Faraday {
    let mut app = two_of_three_from_seeds();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

#[test]
fn the_slots_shares_button_opens_add_a_key_with_the_other_forms_shown() {
    let mut app = two_of_three_from_seeds();
    assert!(
        drawn(&mut app).contains("Shares or another form"),
        "{}",
        drawn(&mut app)
    );
    app.press(Action::RSlotForms);
    assert_eq!(app.screen, Screen::Entry);
    let text = drawn(&mut app);
    for name in ["SLIP-39 shares", "Seed XOR parts"] {
        assert!(text.contains(name), "{name} is not shown: {text}");
    }
}

#[test]
fn add_a_key_from_a_slot_names_its_key_and_offers_no_new_key() {
    for compact in [false, true] {
        let mut app = if compact {
            compact_slot()
        } else {
            two_of_three_from_seeds()
        };
        app.press(Action::RSlotWords);
        assert_eq!(app.screen, Screen::Entry);
        let text = drawn(&mut app);
        assert!(
            !text.contains("Make a new key"),
            "compact {compact}: {text}"
        );
        assert!(
            text.contains("Add a key · 1") || text.contains("For key 1"),
            "compact {compact}: {text}"
        );
    }
}

#[test]
fn a_one_column_words_page_always_ends_with_the_passphrase_and_add_a_key() {
    let mut app = compact_slot();
    app.press(Action::RSlotForms);
    // Back to the words chip: the page is reset, with no word keyboard.
    app.press(Action::EntryForm(1));
    app.press(Action::EntryForm(0));
    let text = drawn(&mut app);
    for line in ["BIP-39 passphrase", "Add a key"] {
        assert!(text.contains(line), "{line} is missing: {text}");
    }
}
