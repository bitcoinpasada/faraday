//! Use a sample (`docs/MODES.md` §6, §10 M7): in Demo, each step that takes
//! a key offers a chip inside its empty field, or on the scanner a Sample
//! tile, and a press puts the next sample in as typed or scanned. The tests
//! drive a `Faraday` with the real samples set, as the desktop does.

use crate::common::{demo_app, reference_key_text, show};
use faraday_bip::keys::Fingerprint;
use faraday_core::forms::Form;
use faraday_core::samples::SampleKey;
use faraday_core::seeds::read::Choice;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_samples::SAMPLES;
use faraday_shell_api::{App, Event, Key};

/// The sample made of `word`.
fn words_of(word: &str) -> &'static SampleKey {
    SAMPLES
        .keys
        .iter()
        .find(|k| k.word == word)
        .unwrap_or_else(|| panic!("no sample of {word}"))
}

fn named(name: &str) -> &'static SampleKey {
    SAMPLES
        .keys
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("no sample named {name}"))
}

fn held(app: &Faraday, k: &SampleKey) -> bool {
    app.session
        .keys
        .position(Fingerprint(k.plain.expect("a key")))
        .is_some()
}

fn form_at(form: Form) -> Action {
    Action::EntryForm(Form::ALL.iter().position(|f| *f == form).unwrap() as u8)
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Add a key open, drawn.
fn add_a_key(app: &mut Faraday) {
    app.press(Action::Entry(None));
    let _ = app.frame();
}

fn words_typed(app: &Faraday) -> String {
    testkit::entry(app).typed.trim().to_string()
}

#[test]
fn add_a_key_with_twelve_words_takes_the_first_sample_and_a_second_press_the_next() {
    let mut app = demo_app();
    add_a_key(&mut app);
    assert!(app.offers(Action::UseSample), "the chip is in the field");
    app.press(Action::UseSample);
    assert_eq!(words_typed(&app), words_of("twelve").parts[0]);
    // Filled: the chip is gone, and a press again puts the next one in its
    // place.
    let _ = app.frame();
    assert!(!app.offers(Action::UseSample));
    app.press(Action::UseSample);
    assert_eq!(words_typed(&app), words_of("ocean").parts[0]);
    app.press(Action::EntryAdd);
    assert!(
        held(&app, words_of("ocean")),
        "{:?}",
        testkit::entry(&app).error
    );
    assert!(!held(&app, words_of("twelve")));
}

#[test]
fn the_chip_follows_the_word_count_chosen() {
    let mut app = demo_app();
    add_a_key(&mut app);
    app.press(Action::EntryWordCount(24));
    app.press(Action::UseSample);
    assert_eq!(words_typed(&app), words_of("bacon").parts[0]);
    app.press(Action::EntryClear);
    app.press(Action::EntryWordCount(15));
    let _ = app.frame();
    assert!(app.offers(Action::UseSample), "Clear brings the chip back");
    app.press(Action::UseSample);
    assert_eq!(words_typed(&app), words_of("lucky").parts[0]);
}

#[test]
fn a_sample_is_not_offered_again_once_it_is_loaded() {
    let mut app = demo_app();
    add_a_key(&mut app);
    app.press(Action::UseSample);
    app.press(Action::EntryAdd);
    assert!(held(&app, words_of("twelve")));
    add_a_key(&mut app);
    app.press(Action::UseSample);
    assert_eq!(
        words_typed(&app),
        words_of("ocean").parts[0],
        "twelve is loaded"
    );
}

#[test]
fn a_small_panel_fills_its_word_entry_from_the_chip() {
    let mut app = demo_app();
    show(&mut app, 480, 2400, 286);
    add_a_key(&mut app);
    assert!(app.offers(Action::UseSample));
    app.press(Action::UseSample);
    let w = testkit::entry(&app)
        .keys
        .as_ref()
        .expect("the word keyboard");
    assert_eq!(w.committed(), 12);
    app.press(Action::EntryAdd);
    assert!(
        held(&app, words_of("twelve")),
        "{:?}",
        testkit::entry(&app).error
    );
}

#[test]
fn pressing_in_the_field_or_typing_hides_the_chip_and_keeps_what_was_typed() {
    // A press in the field.
    let mut app = demo_app();
    add_a_key(&mut app);
    assert!(app.offers(Action::UseSample));
    app.press(Action::EntryField);
    let _ = app.frame();
    assert!(!app.offers(Action::UseSample));
    type_in(&mut app, "abandon ");
    assert_eq!(words_typed(&app), "abandon");
    // Emptied again, the chip stays away for the visit.
    for _ in 0..8 {
        app.event(Event::Key(Key::Backspace));
    }
    let _ = app.frame();
    assert!(testkit::entry(&app).typed.is_empty());
    assert!(!app.offers(Action::UseSample));
    // Typing without a press.
    let mut app = demo_app();
    add_a_key(&mut app);
    type_in(&mut app, "zebra ");
    let _ = app.frame();
    assert!(!app.offers(Action::UseSample));
    assert_eq!(words_typed(&app), "zebra");
    // Leaving the step and opening it again brings it back.
    app.press(Action::Nav(Screen::Wallets));
    add_a_key(&mut app);
    assert!(app.offers(Action::UseSample));
}

#[test]
fn a_slip39_share_fills_then_the_set_s_second_share_and_the_key_loads() {
    let mut app = demo_app();
    add_a_key(&mut app);
    app.press(form_at(Form::Slip39));
    let _ = app.frame();
    assert!(app.offers(Action::UseSample));
    // The first set is the one-share backup; a press again takes the next.
    app.press(Action::UseSample);
    assert_eq!(
        &*testkit::entry(&app).typed,
        named("sample_slip39_one").parts[0]
    );
    app.press(Action::UseSample);
    let set = named("sample_slip39_two_of_three");
    assert_eq!(&*testkit::entry(&app).typed, set.parts[0]);
    // The share is drawn as it is, not masked.
    let shown = app.drawn_texts().join("\n");
    assert!(shown.contains(&set.parts[0][..24]), "{shown}");
    app.press(Action::EntryPart);
    let _ = app.frame();
    assert!(testkit::entry(&app).typed.is_empty());
    assert!(app.offers(Action::UseSample), "the second share's chip");
    app.press(Action::UseSample);
    assert_eq!(
        &*testkit::entry(&app).typed,
        set.parts[1],
        "the same set's next share"
    );
    app.press(Action::EntryPart);
    let _ = app.frame();
    app.press(Action::EntryRecover);
    assert!(held(&app, set), "{:?}", testkit::entry(&app).error);
}

#[test]
fn a_codex32_secret_loads_at_once_and_its_shares_after_two() {
    let mut app = demo_app();
    add_a_key(&mut app);
    app.press(form_at(Form::Codex32));
    let _ = app.frame();
    app.press(Action::UseSample);
    let secret = named("sample_codex32_secret");
    assert_eq!(&*testkit::entry(&app).typed, secret.parts[0]);
    app.press(Action::EntryPart);
    assert!(held(&app, secret), "{:?}", testkit::entry(&app).error);

    let mut app = demo_app();
    add_a_key(&mut app);
    app.press(form_at(Form::Codex32));
    let shares = named("sample_codex32_shares");
    // The secret is the first set; a press again takes the shares.
    app.press(Action::UseSample);
    app.press(Action::UseSample);
    assert_eq!(&*testkit::entry(&app).typed, shares.parts[0]);
    app.press(Action::EntryPart);
    app.press(Action::UseSample);
    assert_eq!(&*testkit::entry(&app).typed, shares.parts[1]);
    app.press(Action::EntryPart);
    let _ = app.frame();
    app.press(Action::EntryRecover);
    assert!(held(&app, shares), "{:?}", testkit::entry(&app).error);
}

#[test]
fn seed_xor_parts_fill_one_after_the_other_and_make_the_key() {
    let mut app = demo_app();
    add_a_key(&mut app);
    app.press(form_at(Form::Xor));
    let _ = app.frame();
    let set = named("sample_xor");
    app.press(Action::UseSample);
    assert_eq!(&*testkit::entry(&app).typed, set.parts[0]);
    app.press(Action::EntryPart);
    app.press(Action::UseSample);
    assert_eq!(&*testkit::entry(&app).typed, set.parts[1]);
    app.press(Action::EntryPart);
    let _ = app.frame();
    app.press(Action::EntryRecover);
    assert!(held(&app, set), "{:?}", testkit::entry(&app).error);
}

#[test]
fn new_key_typed_and_the_spend_tab_s_words_offer_the_chip_too() {
    let mut app = demo_app();
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Restore from SLIP-39")
        .expect("a Restore from SLIP-39 tile");
    app.press(Action::Catalog(i as u8));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::KeyGen);
    assert!(app.offers(Action::UseSample), "New key's typed card");
    app.press(Action::UseSample);
    assert_eq!(
        &*testkit::entry(&app).typed,
        named("sample_slip39_one").parts[0]
    );

    // The Spend tab's Type the words.
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(
        faraday_core::family::FamilyAction::TypeWords,
    ));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Entry);
    assert!(app.offers(Action::UseSample));
    app.press(Action::UseSample);
    app.press(Action::EntryAdd);
    assert!(
        held(&app, words_of("twelve")),
        "{:?}",
        testkit::entry(&app).error
    );
}

#[test]
fn the_seed_scanner_reads_a_sample_through_its_seedqr() {
    let mut app = demo_app();
    app.press(Action::ScanSeed);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.offers(Action::ScanSample), "the Sample tile");
    app.press(Action::ScanSample);
    assert_eq!(
        app.sheet,
        Some(Sheet::SeedRead),
        "the seed asks how it loads"
    );
    app.press(Action::SeedRead(Choice::Plain));
    assert!(
        held(&app, words_of("bacon")),
        "{:?}",
        testkit::entry(&app).error
    );
}

#[test]
fn a_wallet_s_missing_key_scanner_loads_the_matching_samples_and_the_wallet_signs() {
    let mut app = demo_app();
    let c = testkit::compute();
    let savings = &SAMPLES.wallets[0];
    app.session
        .add_wallet(
            &c,
            "Savings",
            savings.text,
            "t",
            faraday_core::wallet::Origin::Named,
        )
        .expect("Savings");
    assert!(!app.can_spend(0));
    app.press(Action::ScanKey(0));
    let _ = app.frame();
    assert!(app.offers(Action::ScanSample));
    app.press(Action::ScanSample);
    assert_eq!(app.session.keys.len(), 1, "one of Savings' keys");
    let _ = app.frame();
    assert!(app.offers(Action::ScanSample), "Savings is short of more");
    app.press(Action::ScanSample);
    assert_eq!(app.session.keys.len(), 2);
    assert!(app.can_spend(0), "two of three sign");
}

#[test]
fn restore_by_file_loads_the_next_sample_wallet() {
    let mut app = demo_app();
    app.press(Action::RestoreWallet);
    let _ = app.frame();
    app.press(Action::Scan);
    let _ = app.frame();
    assert!(app.offers(Action::ScanSample));
    app.press(Action::ScanSample);
    assert!(app.restore().is_some_and(|r| r.described));
    assert_eq!(app.session.wallets.len(), 1, "Savings, the first sample");
}

#[test]
fn the_envelope_takes_the_spend_of_the_wallet_loaded() {
    use faraday_core::family::{FamilyAction as F, page};
    let mut app = demo_app();
    // Savings and its keys loaded, as the Load samples sheet loads them.
    app.press(Action::Samples);
    app.press(Action::SampleTick(faraday_core::samples::Pick::Wallet(0)));
    app.press(Action::SamplesLoad);
    let _ = app.frame();
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Next(page::MAP)));
    app.press(Action::Family(F::Next(page::SAFE)));
    app.press(Action::Family(F::Scan));
    let _ = app.frame();
    assert!(app.offers(Action::ScanSample));
    app.press(Action::ScanSample);
    let _ = app.frame();
    // The transaction is kept for Bring, which starts the spend on the
    // wallet it is from: the review.
    app.press(Action::Cancel);
    app.press(Action::Family(F::Next(page::CHECK)));
    app.press(Action::Family(F::Next(page::WRITE)));
    let _ = app.frame();
    let k = app
        .inbox()
        .iter()
        .position(|i| i.kind == faraday_core::wallet::FileKind::Psbt)
        .expect("the spend is in the Inbox");
    app.press(Action::Family(F::UsePsbt(k)));
    let spend = app.spend().expect("the spend started");
    assert_eq!(spend.wallet, app.family_wallet(), "Savings' own spend");
}

#[test]
fn a_cosigner_s_box_takes_the_account_key_of_the_next_sample_seed_at_the_path() {
    use faraday_core::rstep::{KIND, QUORUM};
    use faraday_core::seeds::{SLIDE_N, SeedsAction as S};
    let mut app = demo_app();
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(4));
    app.press(Action::RNext(KIND));
    app.press(Action::RNext(QUORUM));
    app.press(Action::RSeeds);
    // Two of the keys here, the third a cosigner's.
    for word in ["bacon", "zebra"] {
        app.press(Action::Entry(None));
        type_in(&mut app, &format!("{} ", words_of(word).parts[0]));
        app.press(Action::EntryAdd);
        assert!(
            held(&app, words_of(word)),
            "{:?}",
            testkit::entry(&app).error
        );
    }
    app.press(Action::Slide(SLIDE_N, 2));
    app.event(Event::Key(Key::Right));
    // No vaults, so the Keys card is the one open.
    app.press(Action::RVaults(false));
    app.press(Action::RNext(faraday_core::rstep::WHERE));
    // Its xpub opens the box, with the chip in it.
    app.press(Action::Seeds(S::Open(
        faraday_core::seeds::Focus::Cosigner(0),
    )));
    let _ = app.frame();
    assert!(app.offers(Action::Seeds(S::Sample(0))), "the box's chip");
    app.press(Action::Seeds(S::Sample(0)));
    let s = app.seeds().expect("the seeds");
    // The first sample seed not among the wallet's: flag, at BIP-48's
    // multisig account.
    assert_eq!(
        s.cosigners[0],
        reference_key_text(words_of("flag").parts[0], "48h/1h/0h/2h")
    );
    let _ = app.frame();
    assert!(!app.offers(Action::Seeds(S::Sample(0))), "filled");
}

#[test]
fn the_files_scanner_on_spend_takes_the_sample_wallet_s_unsigned_spend() {
    let mut app = demo_app();
    app.press(Action::Samples);
    app.press(Action::SampleTick(faraday_core::samples::Pick::Wallet(0)));
    app.press(Action::SamplesLoad);
    app.screen = Screen::Spend;
    app.press(Action::Scan);
    let _ = app.frame();
    assert!(app.offers(Action::ScanSample));
    app.press(Action::ScanSample);
    assert!(
        app.inbox()
            .iter()
            .any(|i| i.kind == faraday_core::wallet::FileKind::Psbt),
        "the spend is in Files"
    );
}

/// The copies of seed `seed` made a SeedQR alone, on the backup board.
fn seedqr_copy_only(app: &mut Faraday, seed: usize) {
    use faraday_core::board::{BoardPress as BP, Thing, Zone};
    use faraday_core::plan;
    let c = testkit::compute();
    if app.board_plan().is_some_and(|(_, a)| !a.custom) {
        app.press(Action::Board(BP::Customize));
    }
    let place = {
        let (shape, a) = app.board_plan().expect("a board");
        plan::map(&c, &shape, &a)
            .iter()
            .find_map(|s| {
                match s.at {
                plan::At::Place(p)
                    if s.holds.iter().any(|(w, _)| {
                        matches!(w, plan::What::Words(i) | plan::What::SeedQr(i) if *i == seed)
                    }) =>
                {
                    Some(p)
                }
                _ => None,
            }
            })
            .expect("a place keeps the seed")
    };
    let toggle = |app: &mut Faraday, k: u8| {
        app.press(Action::Board(BP::Form(
            Zone::Paper(place),
            Thing::Seed(seed),
            k,
        )));
    };
    let forms = |app: &Faraday| {
        let (shape, a) = app.board_plan().expect("a board");
        plan::copy_forms(&c, &shape, &a, seed)
    };
    if !forms(app).seedqr {
        toggle(app, 1);
    }
    if forms(app).words {
        toggle(app, 0);
    }
    let got = forms(app);
    assert_eq!((got.words, got.seedqr), (false, true), "the board's copies");
}

#[test]
fn the_copy_check_s_scanner_reads_the_seedqr_of_a_sample_key() {
    use faraday_core::seeds::SeedsAction as S;
    use faraday_core::{ScanPurpose, StorageEvent, bstep};
    let c = testkit::compute();
    let mut app = demo_app();
    app.storage(StorageEvent::Cameras(vec![(
        "/dev/video0".into(),
        "Integrated Camera".into(),
    )]));
    add_a_key(&mut app);
    app.press(Action::UseSample);
    app.press(Action::EntryAdd);
    let key = words_of("twelve");
    assert!(held(&app, key), "{:?}", testkit::entry(&app).error);
    app.press(Action::KeyWallet(key.plain.unwrap(), 1));
    app.press(Action::Seeds(S::Make));
    app.press(Action::Backup(0));
    testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    seedqr_copy_only(&mut app, 0);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    if app.backup().and_then(|b| b.open) != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BReveal);
    let _ = app.frame();
    app.press(Action::BScan);
    let _ = app.frame();
    assert!(matches!(
        app.scan().map(|s| s.purpose),
        Some(ScanPurpose::CheckCopy(_))
    ));
    assert!(app.offers(Action::ScanSample), "the Sample tile");
    app.press(Action::ScanSample);
    assert!(
        app.scan().is_none() && app.sheet.is_none(),
        "the scan read it"
    );
    let line = faraday_core::wallet_backup::copy_scan_line(
        app.backup().unwrap().scanned.as_ref().unwrap(),
    );
    assert_eq!(line, "Your copy scans and matches the seed");
}

#[test]
fn no_button_is_offered_on_testnet_or_where_no_samples_are_set() {
    use faraday_bip::keys::Network;
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Testnet).expect("testnet");
    app.samples = Some(&SAMPLES);
    add_a_key(&mut app);
    assert!(!app.offers(Action::UseSample), "Testnet");
    app.press(Action::UseSample);
    assert!(
        testkit::entry(&app).typed.is_empty(),
        "the press does nothing"
    );
    app.press(Action::ScanSeed);
    let _ = app.frame();
    assert!(!app.offers(Action::ScanSample));

    let mut app = demo_app();
    app.samples = None;
    add_a_key(&mut app);
    assert!(!app.offers(Action::UseSample), "the stick has no samples");
}

/// A twelve-word sample key loaded, its backup open on the copy item,
/// with the typed words check open.
fn open_words_check(app: &mut Faraday) {
    use faraday_core::bstep;
    let c = testkit::compute();
    add_a_key(app);
    app.press(Action::UseSample);
    app.press(Action::EntryAdd);
    let key = words_of("twelve");
    assert!(held(app, key), "{:?}", testkit::entry(app).error);
    app.press(Action::KeyWallet(key.plain.unwrap(), 1));
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    app.press(Action::Backup(0));
    testkit::backup_as(app, faraday_core::plan::Preset::Paper);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    if app.backup().and_then(|b| b.open) != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BCheck);
    let _ = app.frame();
    assert!(app.backup().is_some_and(|b| b.check.is_some()));
}

fn copy_checked(app: &Faraday) -> bool {
    let fp = words_of("twelve").plain.unwrap();
    app.backup()
        .is_some_and(|b| b.checked.iter().any(|(f, _)| *f == fp))
}

#[test]
fn the_copy_check_offers_fill_words_in_an_empty_grid_and_it_completes_the_check() {
    let mut app = demo_app();
    open_words_check(&mut app);
    assert!(app.drawn_texts().iter().any(|t| t == "Fill words"));
    assert!(app.offers(Action::UseSample));
    app.press(Action::UseSample);
    assert!(copy_checked(&app), "the words were checked as typed");
    assert!(app.backup().unwrap().check.is_none());
}

#[test]
fn typing_or_pressing_in_the_grid_hides_fill_words() {
    let mut app = demo_app();
    open_words_check(&mut app);
    app.event(Event::Key(Key::Char('a')));
    let _ = app.frame();
    assert!(!app.drawn_texts().iter().any(|t| t == "Fill words"));
    let mut app = demo_app();
    open_words_check(&mut app);
    app.press(Action::EntryField);
    let _ = app.frame();
    assert!(!app.drawn_texts().iter().any(|t| t == "Fill words"));
}

#[test]
fn outside_demo_the_copy_check_has_no_fill_words() {
    let mut app = demo_app();
    open_words_check(&mut app);
    app.samples = None;
    let _ = app.frame();
    assert!(!app.drawn_texts().iter().any(|t| t == "Fill words"));
    app.press(Action::UseSample);
    assert!(!copy_checked(&app));
}

#[test]
fn the_template_offered_first_is_the_one_the_seeds_fit() {
    let mut app = demo_app();
    open_words_check(&mut app);
    assert_eq!(app.backup().unwrap().words, 12);
}
