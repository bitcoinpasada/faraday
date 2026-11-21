//! Back and Forward within a flow (F104), and a choice that decides its
//! step moving the flow on (F103). Back (the arrow, the mouse's back
//! button, Escape where it is a way back) opens the step before the open
//! one with its choice as it was made, and the arrow is named after that
//! step; Forward opens again the step Back left; the first step's Back
//! leaves the flow for where it was opened; a step that is running takes
//! no Back; once something is made that the flow cannot take back, Back
//! goes no further back than the step after it. A single choice moves
//! on by itself, and pressing the option already chosen moves on too.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::{add_key, type_text};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::bip85::{Bip85App, pstep};
use faraday_core::create::NewKind;
use faraday_core::rstep::{CHECK, DONE, KIND, QUORUM, SEEDS};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::silent::sstep;
use faraday_core::spend_backup::{CardId, SpendBackupAction as F, page};
use faraday_core::testkit::{self, KindText};
use faraday_core::vanity::{VanityAction as VA, vstep as vastep};
use faraday_core::vault::{VaultAction as V, vstep};
use faraday_core::{Action, Faraday, Screen, StorageEvent, mstep};
use faraday_shell_api::{App, Event, Key};

/// Multisig · native SegWit, by its place on Kind.
const MULTI: u8 = 4;

/// The mouse's back button, on the screen as drawn.
fn back(app: &mut Faraday) {
    let _ = app.frame();
    app.event(Event::Back);
    let _ = app.frame();
}

/// The mouse's forward button.
fn forward(app: &mut Faraday) {
    let _ = app.frame();
    app.event(Event::Forward);
    let _ = app.frame();
}

/// How many times `text` is drawn on the screen.
fn drawn_count(app: &mut Faraday, text: &str) -> usize {
    let _ = app.frame();
    app.drawn_texts().iter().filter(|t| *t == text).count()
}

/// The app with a seed and one single-key wallet made from it.
fn with_a_wallet() -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    let m = Mnemonic::from_entropy(Language::English, &[7; 16]).unwrap();
    let words: Vec<&str> = m
        .indices()
        .iter()
        .map(|&w| Language::English.words()[usize::from(w)])
        .collect();
    app.session
        .keys
        .add_words(&c, &words.join(" "), "Seed", None)
        .unwrap();
    let text = NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    app.session
        .add_wallet_confirmed(
            &c,
            "Wallet",
            &format!("wpkh({}/<0;1>/*)", &*text),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app
}

fn restore_open(app: &Faraday) -> Option<u8> {
    app.restore().unwrap().open
}

#[test]
fn restore_back_opens_the_kind_as_chosen_and_forward_the_quorum_again() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::RestoreWallet);
    // A kind pressed moves on (F103).
    app.press(Action::RKind(MULTI));
    assert_eq!(restore_open(&app), Some(QUORUM));
    assert_eq!(
        drawn_count(&mut app, "Back to Kind"),
        1,
        "the way back reads Back to Kind (F146)"
    );
    assert_eq!(drawn_count(&mut app, "Kind"), 1, "Kind's own card");
    back(&mut app);
    assert_eq!(restore_open(&app), Some(KIND));
    assert!(app.restore().unwrap().kind().multi(), "the kind as chosen");
    forward(&mut app);
    assert_eq!(restore_open(&app), Some(QUORUM));
    back(&mut app);
    back(&mut app);
    assert_eq!(
        app.screen,
        Screen::WalletsStart,
        "Kind's Back leaves for where Restore was opened"
    );
}

/// Restore up to Done: a single key made from one seed.
fn restore_at_check(app: &mut Faraday) {
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(0));
    app.press(Action::RSeeds);
    assert_eq!(
        restore_open(app),
        Some(SEEDS),
        "no question before the seeds"
    );
    app.press(Action::RSlotWords);
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(app);
    app.press(Action::Seeds(S::Make));
    assert_eq!(restore_open(app), Some(CHECK));
    app.press(Action::RNext(CHECK));
    assert_eq!(restore_open(app), Some(DONE), "Done follows Check");
}

#[test]
fn restore_asks_nothing_about_vaults_after_check_and_done_draws_no_continue() {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::keep_time(&mut app);
    restore_at_check(&mut app);
    let _ = app.frame();
    assert!(
        !app.drawn_texts().iter().any(|t| t == "Continue"),
        "Done has no Continue"
    );
    back(&mut app);
    assert_eq!(restore_open(&app), Some(CHECK));
}

#[test]
fn restore_a_closed_step_with_no_answer_shows_no_change() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::RestoreWallet);
    assert_eq!(restore_open(&app), Some(KIND));
    let _ = app.frame();
    assert!(
        !app.drawn_texts().iter().any(|t| t == "Change"),
        "nothing is answered yet: no Change beside it"
    );
}

#[test]
fn restore_once_the_wallet_is_made_back_leaves_from_check() {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::keep_time(&mut app);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(0));
    app.press(Action::RSeeds);
    app.press(Action::RSlotWords);
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    app.press(Action::Seeds(S::Make));
    assert_eq!(restore_open(&app), Some(CHECK), "the wallet is made");
    back(&mut app);
    assert_eq!(
        app.screen,
        Screen::WalletsStart,
        "the seeds of a made wallet are not opened again"
    );
}

#[test]
fn sign_a_message_a_format_moves_on_and_back_shows_it_chosen() {
    let mut app = with_a_wallet();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::SignMessage);
    app.press(Action::MNext(mstep::ADDRESS));
    app.press(Action::MNext(mstep::TEXT));
    assert_eq!(testkit::message(&app).unwrap().open, Some(mstep::FORMAT));
    app.press(Action::MFormat(false));
    assert_eq!(testkit::message(&app).unwrap().open, Some(mstep::SIGN));
    back(&mut app);
    let m = testkit::message(&app).unwrap();
    assert_eq!(m.open, Some(mstep::FORMAT));
    assert!(!m.bip322, "BIP-137, as chosen");
    app.press(Action::MFormat(false));
    assert_eq!(testkit::message(&app).unwrap().open, Some(mstep::SIGN));
    for open in [mstep::FORMAT, mstep::TEXT, mstep::ADDRESS] {
        back(&mut app);
        assert_eq!(testkit::message(&app).unwrap().open, Some(open));
    }
    back(&mut app);
    assert_eq!(app.screen, Screen::WalletsStart);
}

fn vault_open(app: &Faraday) -> Option<u8> {
    app.vaults().create.as_ref().unwrap().open
}

#[test]
fn create_a_vault_back_opens_the_size_and_a_size_pressed_moves_on() {
    let mut app = testkit::started();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "vault.kdbx".to_string(),
            faraday_core::testkit::test_vault().unwrap(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    assert_eq!(app.screen, Screen::CreateVault);
    assert_eq!(vault_open(&app), Some(vstep::PHRASES));
    back(&mut app);
    assert_eq!(vault_open(&app), Some(vstep::PRESET));
    app.press(Action::Vault(V::CSizeRow(1)));
    assert_eq!(vault_open(&app), Some(vstep::PHRASES), "a size moves on");
    let preset = app.vaults().create.as_ref().unwrap().preset;
    back(&mut app);
    assert_eq!(vault_open(&app), Some(vstep::PRESET));
    assert_eq!(app.vaults().create.as_ref().unwrap().preset, preset);
    forward(&mut app);
    assert_eq!(vault_open(&app), Some(vstep::PHRASES));
    back(&mut app);
    back(&mut app);
    assert_eq!(app.screen, Screen::Vaults);
}

#[test]
fn create_a_vault_customised_cost_moves_on() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    back(&mut app);
    app.press(Action::Vault(V::CCustomise));
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    assert_eq!(vault_open(&app), Some(vstep::COST));
    app.press(Action::Vault(V::CPreset(0)));
    assert_eq!(vault_open(&app), Some(vstep::PHRASES), "a cost moves on");
    back(&mut app);
    assert_eq!(vault_open(&app), Some(vstep::COST));
}

#[test]
fn back_while_a_vault_is_made_makes_none() {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    let phrase = "orange harbour lantern quietly";
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, phrase);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, phrase);
    testkit::press_on_worker(&mut app, Action::Vault(V::CGo));
    assert!(app.vaults().working.is_some(), "the vault is being made");
    // Back while it is made drops it: no vault file comes of it.
    app.event(Event::Back);
    assert!(app.vaults().working.is_none());
    testkit::settle(&mut app);
    assert!(app.vault_files().is_empty(), "a vault was made after Back");
}

/// A transaction to sign, opened on its spend as a person opens it.
fn spend_opened() -> Faraday {
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = testkit::unsigned(&k).unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
            (
                faraday_text::Text::of("spending-wallet.txt"),
                k.descriptor.into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, name: &str| app.inbox().iter().position(|i| i.name == name).unwrap();
    let wallet_at = at(&app, "spending-wallet.txt");
    testkit::load_wallet(&mut app, wallet_at);
    add_key(&mut app, "bacon");
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    assert_eq!(app.screen, Screen::Spend);
    app
}

#[test]
fn sign_a_transaction_steps_back_and_once_signed_leaves() {
    let mut app = spend_opened();
    let s = app.spend().unwrap();
    let open = s.open.unwrap();
    let place = s.steps.iter().position(|&k| k == open).unwrap();
    assert!(place > 0, "the spend opens past its first step");
    let before = s.steps[place - 1];
    back(&mut app);
    assert_eq!(app.spend().unwrap().open, Some(before));
    forward(&mut app);
    assert_eq!(app.spend().unwrap().open, Some(open));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.finished.is_some(), "signed here");
    back(&mut app);
    assert_ne!(
        app.screen,
        Screen::Spend,
        "Back does not open the steps before the signature"
    );
}

#[test]
fn bip85_an_application_with_no_length_moves_on_and_escape_steps_back() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    app.press(Action::Bip85);
    assert_eq!(app.bip85().unwrap().open, Some(pstep::APP));
    app.press(Action::PApp(1));
    assert_eq!(app.bip85().unwrap().app, Bip85App::Wif);
    assert_eq!(app.bip85().unwrap().open, Some(pstep::INDEX));
    let _ = app.frame();
    app.event(Event::Key(Key::Escape));
    let b = app.bip85().unwrap();
    assert_eq!(b.open, Some(pstep::APP), "Escape is Back");
    assert_eq!(b.app, Bip85App::Wif);
}

#[test]
fn silent_payments_back_opens_the_key_and_forward_the_address() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    app.press(Action::Silent);
    assert_eq!(app.silent().unwrap().open, Some(sstep::ADDRESS));
    back(&mut app);
    assert_eq!(app.silent().unwrap().open, Some(sstep::KEY));
    forward(&mut app);
    assert_eq!(app.silent().unwrap().open, Some(sstep::ADDRESS));
}

#[test]
fn a_vanity_search_under_way_takes_no_back() {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    add_key(&mut app, "bacon");
    app.press(Action::Vanity(VA::Open));
    assert_eq!(app.vanity().unwrap().open, Some(vastep::DIAL));
    // A dial and a kind of address each move on (F103).
    app.press(Action::Vanity(VA::Dial(0)));
    assert_eq!(app.vanity().unwrap().open, Some(vastep::SCRIPT));
    back(&mut app);
    assert_eq!(app.vanity().unwrap().open, Some(vastep::DIAL));
    forward(&mut app);
    assert_eq!(app.vanity().unwrap().open, Some(vastep::SCRIPT));
    app.press(Action::Vanity(VA::Script(2)));
    assert_eq!(app.vanity().unwrap().open, Some(vastep::PREFIX));
    app.event(Event::Key(Key::Char('q')));
    app.press(Action::Vanity(VA::Next(vastep::PREFIX)));
    app.press(Action::Vanity(VA::Start));
    assert!(app.vanity().unwrap().running);
    app.event(Event::Back);
    assert_eq!(app.screen, Screen::Vanity);
    assert_eq!(app.vanity().unwrap().open, Some(vastep::RUN));
    app.press(Action::Vanity(VA::Stop));
    back(&mut app);
    assert_eq!(app.vanity().unwrap().open, Some(vastep::PREFIX));
}

#[test]
fn the_spend_tabs_start_opens_what_do_you_have_and_back_the_map() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::Next(page::MAP)));
    assert_eq!(
        app.spend_backup_open_card(),
        Some(CardId::Page(page::HOLDING))
    );
    let _ = app.frame();
    assert!(
        !app.drawn_texts()
            .iter()
            .any(|t| t.starts_with("Start from the")),
        "Starting from the stick is no card of its own"
    );
    back(&mut app);
    assert_eq!(app.spend_backup_open_card(), Some(CardId::Page(page::MAP)));
    forward(&mut app);
    assert_eq!(
        app.spend_backup_open_card(),
        Some(CardId::Page(page::HOLDING))
    );
}

/// The arrow, Escape or the mouse's back, on the screen as drawn.
fn leave_by(app: &mut Faraday, way: usize) {
    let _ = app.frame();
    match way {
        0 => app.press(Action::Back),
        1 => app.event(Event::Key(Key::Escape)),
        _ => app.event(Event::Back),
    }
    let _ = app.frame();
}

#[test]
fn sign_a_transaction_steps_back_by_the_arrow_escape_and_the_mouse_back_alike() {
    for way in 0..3 {
        let mut app = spend_opened();
        let s = app.spend().unwrap();
        let open = s.open.unwrap();
        let place = s.steps.iter().position(|&k| k == open).unwrap();
        assert!(place > 0, "the spend opens past its first step");
        let before = s.steps[place - 1];
        leave_by(&mut app, way);
        assert_eq!(app.screen, Screen::Spend, "way {way}");
        assert_eq!(app.spend().unwrap().open, Some(before), "way {way}");
    }
}

#[test]
fn restore_steps_back_to_its_first_step_by_the_arrow_escape_and_the_mouse_back_alike() {
    for way in 0..3 {
        let mut app = testkit::started();
        app.press(Action::Nav(Screen::WalletsStart));
        app.press(Action::RestoreWallet);
        app.press(Action::RKind(MULTI));
        assert_eq!(restore_open(&app), Some(QUORUM));
        leave_by(&mut app, way);
        assert_eq!(app.screen, Screen::Restore, "way {way}");
        assert_eq!(restore_open(&app), Some(KIND), "way {way}");
    }
}

/// Create a vault on a wide window, where Your choices stands beside the
/// step.
fn create_a_vault_wide() -> Faraday {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width: 1366,
        height: 1000,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    app
}

#[test]
fn a_row_of_your_choices_opens_its_step_and_the_name_row_comes_back_with_the_passphrase() {
    let mut app = create_a_vault_wide();
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "orange harbour");
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::CStep(vstep::PRESET))));
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    assert_eq!(vault_open(&app), Some(vstep::PRESET));
    let _ = app.frame();
    assert!(
        app.offers(Action::Vault(V::CStep(vstep::PHRASES))),
        "the name and passphrase row goes straight forward"
    );
    app.press(Action::Vault(V::CStep(vstep::PHRASES)));
    assert_eq!(vault_open(&app), Some(vstep::PHRASES));
    let form = app.vaults().create.as_ref().unwrap();
    assert_eq!(
        faraday_core::vault::Vaults::phrase_text(form, 0).unwrap().0,
        "orange harbour"
    );
}

#[test]
fn the_row_of_the_step_open_takes_no_press_and_no_step_closes() {
    let mut app = create_a_vault_wide();
    assert!(!app.offers(Action::Vault(V::CStep(vstep::PHRASES))));
    app.press(Action::Vault(V::CStep(vstep::PHRASES)));
    assert_eq!(vault_open(&app), Some(vstep::PHRASES));
}

#[test]
fn while_a_vault_is_made_no_row_takes_a_press() {
    let mut app = create_a_vault_wide();
    let phrase = "orange harbour lantern quietly";
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, phrase);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, phrase);
    testkit::press_on_worker(&mut app, Action::Vault(V::CGo));
    assert!(app.vaults().working.is_some(), "the vault is being made");
    let _ = app.frame();
    assert!(!app.offers(Action::Vault(V::CStep(vstep::PRESET))));
    assert_eq!(vault_open(&app), Some(vstep::PHRASES));
}
