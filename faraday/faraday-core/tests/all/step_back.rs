//! Back and Forward within a flow (F104), and a choice that decides its
//! step moving the flow on (F103). Back (the arrow, the mouse's back
//! button, Escape where it is a way back) opens the step before the open
//! one with its choice as it was made, and the arrow is named after that
//! step; Forward opens again the step Back left; the first step's Back
//! leaves the flow for where it was opened; a step that is running takes
//! no Back; once something is made that the flow cannot take back, Back
//! goes no further back than the step after it. A single choice moves
//! on by itself, and pressing the option already chosen moves on too.

use crate::common::{add_key, type_text};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::bip85::{Bip85App, pstep};
use faraday_core::create::NewKind;
use faraday_core::family::{CardId, FamilyAction as F, page};
use faraday_core::rstep::{CHECK, DESCRIPTION, KIND, QUORUM, SEEDS, WHERE};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::silent::sstep;
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
        .add_wallet(
            &c,
            "Wallet",
            &format!("wpkh({text}/<0;1>/*)"),
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
        drawn_count(&mut app, "Kind"),
        2,
        "the way back is named after Kind, beside Kind's own card"
    );
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

#[test]
fn restore_the_answer_already_chosen_moves_on_again() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::RestoreWallet);
    // The kind ticked to begin with is taken by pressing it.
    app.press(Action::RKind(0));
    assert_eq!(restore_open(&app), Some(DESCRIPTION));
    app.press(Action::RSeeds);
    assert_eq!(restore_open(&app), Some(WHERE));
    app.press(Action::RVaults(false));
    assert_eq!(restore_open(&app), Some(SEEDS), "No moves on to the keys");
    back(&mut app);
    assert_eq!(restore_open(&app), Some(WHERE));
    assert_eq!(app.restore().unwrap().vaults.answer, Some(false));
    app.press(Action::RVaults(false));
    assert_eq!(restore_open(&app), Some(SEEDS));
}

#[test]
fn restore_once_the_wallet_is_made_back_leaves_from_check() {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::keep_time(&mut app);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::RestoreWallet);
    app.press(Action::RKind(0));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RSlotWords);
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
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
fn create_a_vault_customised_cost_and_space_move_on() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    back(&mut app);
    app.press(Action::Vault(V::CCustomise));
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    assert_eq!(vault_open(&app), Some(vstep::COST));
    app.press(Action::Vault(V::CPreset(0)));
    assert_eq!(vault_open(&app), Some(vstep::SIZE), "a cost moves on");
    app.press(Action::Vault(V::CSize(0)));
    assert_eq!(vault_open(&app), Some(vstep::PHRASES), "a space moves on");
    back(&mut app);
    back(&mut app);
    assert_eq!(vault_open(&app), Some(vstep::COST));
}

#[test]
fn a_vault_being_made_takes_no_back() {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    let phrase = "orange harbour lantern quietly";
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, phrase);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, phrase);
    app.press(Action::Vault(V::CGo));
    assert!(app.vaults().working.is_some(), "the vault is being made");
    app.event(Event::Back);
    assert_eq!(app.screen, Screen::CreateVault);
    assert_eq!(vault_open(&app), Some(vstep::PHRASES));
}

#[test]
fn sign_a_transaction_steps_back_and_once_signed_leaves() {
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = testkit::unsigned(&k).unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("spend.psbt".to_string(), psbt.to_bytes()),
            ("spending-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, name: &str| app.inbox().iter().position(|i| i.name == name).unwrap();
    app.press(Action::LoadWallet(at(&app, "spending-wallet.txt")));
    add_key(&mut app, "bacon");
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    assert_eq!(app.screen, Screen::Spend);
    let s = app.spend().unwrap();
    let open = s.open.unwrap();
    let place = s.steps.iter().position(|&k| k == open).unwrap();
    assert!(place > 0, "the spend opens past its first step");
    let before = s.steps[place - 1];
    back(&mut app);
    assert_eq!(app.spend().unwrap().open, Some(before));
    forward(&mut app);
    assert_eq!(app.spend().unwrap().open, Some(open));
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
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Next(page::MAP)));
    assert_eq!(app.family_open_card(), Some(CardId::Page(page::HOLDING)));
    let _ = app.frame();
    assert!(
        !app.drawn_texts()
            .iter()
            .any(|t| t.starts_with("Start from the")),
        "Starting from the stick is no card of its own"
    );
    back(&mut app);
    assert_eq!(app.family_open_card(), Some(CardId::Page(page::MAP)));
    forward(&mut app);
    assert_eq!(app.family_open_card(), Some(CardId::Page(page::HOLDING)));
}
