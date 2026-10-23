//! The Spend tab (`docs/FAMILY.md`): someone spending for the first time
//! goes from what is in the envelope to a signed transaction on one tab.
//! What do you have? (`docs/history/SCAN-TO-SIGN.md` §4) takes whatever is in
//! the envelope, scanned or typed in any order (a vault, seeds, a
//! description, split sheets, a transaction), works out the wallet and
//! asks only the next question; a wallet worked out from seeds alone is
//! checked kind by kind until the money shows (§5). The tab's place
//! survives a lock, and what it signs is byte for byte what Wallets ›
//! Sign a transaction signs.

use crate::common::type_text;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::family::{CardId, FamilyAction as F, Open, Question, Route, page};
use faraday_core::seeds::read::Choice;
use faraday_core::testkit;
use faraday_core::wallet::{FileKind, step};
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// A fresh process whose Inbox holds these files.
fn with_files(files: Vec<(String, Vec<u8>)>, kept: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept,
    });
    app
}

/// A fresh process on a display, so work that waits for a drawn frame
/// runs.
fn shown() -> Faraday {
    shown_at(768)
}

/// A fresh process on a display `height` high: tall enough, Check the
/// money's buttons show under its addresses without scrolling.
fn shown_at(height: u16) -> Faraday {
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height,
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

/// A descriptor as text, hardened steps written one way.
fn desc(p: &faraday_bip::policy::WalletPolicy) -> String {
    p.to_descriptor().replace('\'', "h")
}

fn vault_file() -> (String, Vec<u8>) {
    ("vault.ofv".to_string(), testkit::test_vault().unwrap())
}

fn savings_psbt() -> (String, Vec<u8>) {
    let (u, _) = testkit::spend_of(&kit("savings")).unwrap();
    ("savings-unsigned.psbt".to_string(), u.to_bytes())
}

/// Frames and ticks, so work waiting for a drawn frame (Argon2id) runs.
fn settle(app: &mut Faraday) {
    for t in 1..20u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn inbox_at(app: &Faraday, name: &str) -> usize {
    app.inbox().iter().position(|i| i.name == name).unwrap()
}

fn wallet_at(app: &Faraday, name: &str) -> usize {
    app.session
        .wallets
        .iter()
        .position(|w| w.name == name)
        .unwrap()
}

/// The texts on screen once what moves has come to rest.
fn drawn(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.settle();
    let _ = app.frame();
    app.drawn_texts().join("\n")
}

/// The tab on the testing network, past the map and the stick: What do
/// you have? open.
fn on_intake(app: &mut Faraday) {
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Next(page::MAP)));
    app.press(Action::Family(F::Next(page::SAFE)));
    assert_eq!(app.family().open, Some(Open::Page(page::HOLDING)));
}

/// The camera, opened by Scan the QR codes you have, reads this code.
fn scan(app: &mut Faraday, bytes: Vec<u8>) {
    if app.scan().is_none() {
        app.press(Action::Family(F::Scan));
    }
    assert_eq!(app.sheet, Some(Sheet::Scan), "the camera is not on");
    app.event(Event::Scanned { bytes });
    let _ = app.frame();
}

/// A seed's SeedQR, as the camera reads it.
fn seedqr(words: &str) -> Vec<u8> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, words).unwrap();
    faraday_codec::seedqr::to_digits(&m)
        .expose()
        .as_bytes()
        .to_vec()
}

/// The SeedQR of test seed `word`, read by the camera; its sheet's Use
/// without a passphrase, after which the camera comes back on.
fn scan_seed(app: &mut Faraday, word: &str) {
    scan(app, seedqr(&testkit::test_words(word)));
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "no passphrase sheet");
    app.press(Action::SeedRead(Choice::Plain));
    assert_eq!(app.sheet, Some(Sheet::Scan), "the camera did not come back");
}

/// The camera's note.
fn note(app: &Faraday) -> String {
    app.scan().and_then(|s| s.note.clone()).unwrap_or_default()
}

/// The first receive address of the test kit's wallet `id`.
fn kit_address(id: &str) -> String {
    let c = faraday_core::testkit::compute();
    let mut s = testkit::session();
    let i = s
        .add_wallet(
            &c,
            "kit",
            &kit(id).descriptor,
            "kit",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the kit's wallet reads");
    s.address(&c, &s.wallets[i], false, 0)
}

/// The first receive address of the wallet the tab chose.
fn chosen_address(app: &Faraday) -> String {
    let c = faraday_core::testkit::compute();
    let w = app.family_wallet().expect("no wallet chosen");
    app.session.address(&c, &app.session.wallets[w], false, 0)
}

/// Lock, and the next process restored from what the lock kept.
fn after_lock(mut app: Faraday) -> Faraday {
    app.press(Action::Lock);
    assert!(app.restart_requested());
    let mut inbox = Vec::new();
    let mut outbox = Vec::new();
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox: i,
            outbox: o,
            kept: k,
        } = c
        {
            (inbox, outbox, kept) = (i, o, k);
        }
    }
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    app
}

/// The vault as far as the wallet: the tab, What do you have?'s vault,
/// the passphrase, Unlock, Load.
fn unlock_on_tab(app: &mut Faraday) {
    let c = testkit::compute();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Next(page::MAP)));
    app.press(Action::Family(F::Next(page::SAFE)));
    let text = drawn(app);
    assert!(text.contains("Unlock vault.ofv"), "{text}");
    app.press(Action::Family(F::Pick(0)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Family(F::Unlock));
    settle(app);
    // Unlocked: everything is ticked, nothing loaded until Load.
    assert!(app.session.keys.is_empty(), "the unlock loaded before Load");
    assert!(app.vault_rows_now(&c, 0).iter().all(|r| r.chosen));
    app.press(Action::Vault(faraday_core::vault::VaultAction::LoadChosen(
        0,
    )));
}

/// An unlock the Spend tab asks for returns to it with no Load sheet
/// (F62): what the spend needs is loaded from the tab.
#[test]
fn an_unlock_the_spend_asks_for_shows_no_load_sheet() {
    let mut app = with_files(vec![vault_file(), savings_psbt()], Vec::new());
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Next(page::MAP)));
    app.press(Action::Family(F::Next(page::SAFE)));
    app.press(Action::Family(F::Pick(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Family(F::Unlock));
    settle(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "the vault did not open");
    assert_eq!(app.screen, Screen::Family);
    assert_ne!(app.sheet, Some(Sheet::UnlockLoad));
}

#[test]
fn the_vault_route_unlocks_loads_and_signs_without_leaving_the_tab() {
    let mut app = with_files(vec![vault_file(), savings_psbt()], Vec::new());
    unlock_on_tab(&mut app);
    assert_eq!(app.screen, Screen::Family, "unlocking left the tab");
    assert_eq!(app.vaults.open.len(), 1, "the vault did not open");
    assert!(!app.session.keys.is_empty(), "the vault's key did not load");
    // The test vault holds every test wallet; the PSBT waiting in Files
    // says which one it spends from.
    assert!(app.session.wallets.len() > 1);
    let savings = wallet_at(&app, "Savings");
    assert_eq!(app.family_wallet(), Some(savings));
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
    app.press(Action::Family(F::Next(page::CHECK)));
    app.press(Action::Family(F::Next(page::WRITE)));
    assert_eq!(app.family().open, Some(Open::Page(page::BRING)));

    let at = inbox_at(&app, "savings-unsigned.psbt");
    app.press(Action::Family(F::UsePsbt(at)));
    assert_eq!(app.screen, Screen::Family, "the spend left the tab");
    assert_eq!(app.family().open, Some(Open::Spend));
    let s = app.spend().unwrap();
    assert_eq!(s.wallet, Some(savings), "the spend's wallet is not Savings");
    assert_eq!(s.open, Some(step::TRANSACTION), "the review is not first");

    app.press(Action::SignHere);
    let here = app.spend().unwrap().spend.psbt.to_bytes();
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());

    // Wallets › Sign a transaction, from the same files, signs the same
    // bytes.
    let mut other = with_files(vec![vault_file(), savings_psbt()], Vec::new());
    other.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    type_text(&mut other, testkit::VAULT_PASSPHRASES[0]);
    other.press(Action::Vault(faraday_core::vault::VaultAction::Unlock));
    settle(&mut other);
    other.press(Action::Vault(faraday_core::vault::VaultAction::LoadAll(0)));
    let at = inbox_at(&other, "savings-unsigned.psbt");
    other.press(Action::StartSpend(at));
    other.press(Action::SignHere);
    assert_eq!(here, other.spend().unwrap().spend.psbt.to_bytes());
}

#[test]
fn a_vault_with_the_wallet_and_one_key_takes_the_second_key_from_its_words() {
    let mut app = with_files(vec![vault_file(), savings_psbt()], Vec::new());
    unlock_on_tab(&mut app);
    let savings = wallet_at(&app, "Savings");
    app.press(Action::Family(F::Choose(savings)));
    let at = inbox_at(&app, "savings-unsigned.psbt");
    app.press(Action::Family(F::UsePsbt(at)));
    app.press(Action::Step(step::SIGNERS));
    // Who signs' Scan a key, then Type the words instead.
    let _ = app.frame();
    app.settle();
    let _ = app.frame();
    assert!(app.offers(Action::ScanKey(savings)));
    app.press(Action::ScanKey(savings));
    let _ = app.frame();
    app.press(Action::EntryWallet(savings));
    assert_eq!(app.screen, Screen::Entry);
    type_text(&mut app, &testkit::test_words("zebra"));
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Family, "adding the key left the tab");
    assert_eq!(app.session.keys.len(), 2);
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert_eq!(s.spend.signed_here.len(), 2, "both keys did not sign");
    assert!(s.complete, "two of three did not complete the spend");
}

#[test]
fn a_key_from_another_wallet_is_refused_on_the_signing_card() {
    let mut app = with_files(vec![vault_file(), savings_psbt()], Vec::new());
    unlock_on_tab(&mut app);
    let savings = wallet_at(&app, "Savings");
    app.press(Action::Family(F::Choose(savings)));
    let at = inbox_at(&app, "savings-unsigned.psbt");
    app.press(Action::Family(F::UsePsbt(at)));
    app.press(Action::Step(step::SIGNERS));
    app.press(Action::ScanKey(savings));
    app.press(Action::EntryWallet(savings));
    let unrelated = ["abandon"; 11].join(" ") + " about";
    type_text(&mut app, &unrelated);
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Entry, "a key for no slot was taken");
    let error = testkit::entry(&app).error.clone().unwrap_or_default();
    assert!(error.ends_with("not a key of Savings"), "{error}");
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_lock_comes_back_to_the_same_page_and_wallet_after_unlocking_again() {
    let mut app = with_files(vec![vault_file()], Vec::new());
    unlock_on_tab(&mut app);
    let savings = wallet_at(&app, "Savings");
    // Choosing the wallet is the card done.
    app.press(Action::Family(F::Choose(savings)));
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
    app.press(Action::Family(F::Next(page::CHECK)));
    app.press(Action::Family(F::Next(page::WRITE)));

    // The next process: the tab, waiting for the vault again.
    let mut app = after_lock(app);
    assert_eq!(
        app.screen,
        Screen::Family,
        "the lock did not return to the tab"
    );
    assert_eq!(app.family().route, Some(Route::Vault));
    assert_eq!(app.family().open, Some(Open::Page(page::HOLDING)));
    // The PSBT copied in during the visit.
    let (name, bytes) = savings_psbt();
    app.storage(StorageEvent::Restored {
        inbox: {
            let mut v: Vec<(String, Vec<u8>)> = app
                .inbox()
                .iter()
                .map(|i| (i.name.clone(), i.bytes.clone()))
                .collect();
            v.push((name, bytes));
            v
        },
        outbox: app
            .outbox
            .iter()
            .map(|i| (i.name.clone(), i.bytes.clone()))
            .collect(),
        kept: Vec::new(),
    });
    app.press(Action::Family(F::Pick(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Family(F::Unlock));
    settle(&mut app);
    app.press(Action::Vault(faraday_core::vault::VaultAction::LoadChosen(
        0,
    )));
    assert_eq!(app.family_wallet(), Some(wallet_at(&app, "Savings")));
    assert_eq!(
        app.family().open,
        Some(Open::Page(page::BRING)),
        "the pages done before the lock were asked for again"
    );
}

#[test]
fn typed_words_open_the_kind_of_wallet_the_transaction_spends_from() {
    let c = faraday_core::testkit::compute();
    let taproot = kit("taproot");
    let (u, _) = testkit::spend_of(&taproot).unwrap();
    let mut app = with_files(
        vec![("taproot-unsigned.psbt".to_string(), u.to_bytes())],
        Vec::new(),
    );
    on_intake(&mut app);
    app.press(Action::Family(F::TypeWords));
    assert_eq!(app.screen, Screen::Entry);
    type_text(&mut app, &testkit::test_words("bacon"));
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Family);
    // That's everything: native SegWit first, the Spending test wallet.
    app.press(Action::Family(F::Everything));
    let w = app.family_wallet().expect("no wallet opened");
    let spending = faraday_core::wallet::read_wallet(&c, &kit("spending").descriptor).unwrap();
    assert_eq!(desc(app.session.wallets[w].policy()), desc(&spending));
    // A Taproot spend opens the Taproot kind of the same words.
    app.press(Action::Family(F::UsePsbt(0)));
    let s = app.spend().unwrap();
    let w = s.wallet.expect("no wallet matched the Taproot spend");
    let tr = faraday_core::wallet::read_wallet(&c, &taproot.descriptor).unwrap();
    assert_eq!(desc(app.session.wallets[w].policy()), desc(&tr));
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().complete);
}

/// Savings' split sheet `k` of three.
fn sheet(k: usize) -> Vec<u8> {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| *n == format!("savings-share-{k}-of-3.txt"))
        .unwrap()
        .1
}

#[test]
fn split_sheets_scanned_in_any_order_rebuild_the_wallet() {
    let mut app = shown();
    on_intake(&mut app);
    scan(&mut app, sheet(3));
    assert!(app.family_wallet().is_none());
    assert!(note(&app).contains("2 of 3 keys in hand"), "{}", note(&app));
    scan(&mut app, sheet(1));
    assert!(
        app.family_wallet().is_some(),
        "two sheets did not rebuild it"
    );
    assert_eq!(chosen_address(&app), kit_address("savings"));
    app.press(Action::Cancel);
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
}

#[test]
fn one_sheet_says_what_is_still_missing() {
    let mut app = shown();
    on_intake(&mut app);
    scan(&mut app, sheet(2));
    app.press(Action::Cancel);
    assert!(app.family_wallet().is_none());
    let text = drawn(&mut app);
    assert!(text.contains("Part 1 · 2 of 3 keys in hand"), "{text}");
}

#[test]
fn a_description_in_files_is_offered_on_the_card_and_opens_the_wallet() {
    let file = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "savings-wallet.txt")
        .unwrap();
    let mut app = with_files(vec![file], Vec::new());
    on_intake(&mut app);
    let text = drawn(&mut app);
    assert!(text.contains("Open savings-wallet.txt"), "{text}");
    assert!(text.contains("Copy files in from a stick"), "{text}");
    let k = inbox_at(&app, "savings-wallet.txt");
    assert!(app.offers(Action::Family(F::UseWallet(k))));
    app.press(Action::Family(F::UseWallet(k)));
    assert_eq!(chosen_address(&app), kit_address("savings"));
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
}

#[test]
fn with_several_wallets_and_no_transaction_the_person_chooses() {
    let mut app = with_files(vec![vault_file()], Vec::new());
    unlock_on_tab(&mut app);
    assert!(app.family_wallet().is_none());
    assert_eq!(app.family().open, Some(Open::Page(page::HOLDING)));
    let savings = wallet_at(&app, "Savings");
    app.press(Action::Family(F::Choose(savings)));
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
}

#[test]
fn no_page_past_the_wallet_opens_without_one() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Card(page::BRING)));
    assert_eq!(app.family().open, Some(Open::Page(page::HOLDING)));
    app.press(Action::Family(F::Card(page::CHECK)));
    assert_eq!(app.family().open, Some(Open::Page(page::HOLDING)));
    // Open the wallet is one card with What do you have?.
    assert!(!app.family_cards().contains(&CardId::Page(page::OPEN)));
}

#[test]
fn with_no_vault_in_files_the_card_offers_no_unlock() {
    let mut app = shown();
    on_intake(&mut app);
    let text = drawn(&mut app);
    assert!(text.contains("What do you have?"), "{text}");
    assert!(text.contains("Scan the QR codes you have"), "{text}");
    assert!(text.contains("Type the words you have"), "{text}");
    assert!(!text.contains("Unlock"), "{text}");
}

#[test]
fn one_seed_asks_for_anything_else_and_thats_everything_opens_native_segwit() {
    let mut app = shown();
    on_intake(&mut app);
    scan_seed(&mut app, "bacon");
    app.press(Action::Cancel);
    // No wallet yet: the next question.
    assert!(app.family_wallet().is_none());
    let text = drawn(&mut app);
    assert!(text.contains("Anything else in the envelope?"), "{text}");
    assert!(text.contains("That's everything"), "{text}");
    app.press(Action::Family(F::Everything));
    assert_eq!(chosen_address(&app), kit_address("spending"));
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
}

#[test]
fn two_seeds_then_the_description_open_the_wallet_with_both_keys_here() {
    let mut app = shown();
    on_intake(&mut app);
    scan_seed(&mut app, "bacon");
    scan_seed(&mut app, "zebra");
    // The camera's sheet says what a description looks like.
    let text = drawn(&mut app);
    assert!(text.contains("Scan what you have"), "{text}");
    assert!(text.contains("Its description says how they fit"), "{text}");
    app.press(Action::Cancel);
    let text = drawn(&mut app);
    assert!(
        text.contains("These seeds are one wallet with several keys."),
        "{text}"
    );
    assert!(text.contains("Scan the description"), "{text}");
    scan(&mut app, testkit::savings().into_bytes());
    assert_eq!(note(&app), "Wallet description read");
    let w = app.family_wallet().expect("the description did not open");
    let wl = &app.session.wallets[w];
    let here = app
        .session
        .slots(wl)
        .iter()
        .filter(|s| s.held_by.is_some())
        .count();
    assert_eq!(here, 2, "both seeds are not keys of the wallet here");
    app.press(Action::Cancel);
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
}

#[test]
fn a_seed_of_another_wallet_after_the_description_is_not_one_of_its_keys() {
    let mut app = shown();
    on_intake(&mut app);
    scan(&mut app, testkit::savings().into_bytes());
    let w = app.family_wallet().expect("the description did not open");
    let name = app.session.wallets[w].name.clone();
    // A seed of no key of the wallet, on the same camera.
    let other = ["abandon"; 11].join(" ") + " about";
    scan(&mut app, seedqr(&other));
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    let text = drawn(&mut app);
    assert!(
        text.contains(&format!("is not one of {name}'s keys")),
        "{text}"
    );
    // Its plain choice is refused there.
    app.press(Action::SeedRead(Choice::Plain));
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert!(app.session.keys.is_empty());
    // Not this one: the words go and the camera comes back.
    let _ = drawn(&mut app);
    assert!(app.offers(Action::SeedRead(Choice::Skip)));
    app.press(Action::SeedRead(Choice::Skip));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.seed_read().is_none());
    assert!(app.session.keys.is_empty());
}

#[test]
fn two_seeds_and_no_description_make_the_two_of_three_from_all_three_seeds() {
    let mut app = shown();
    on_intake(&mut app);
    scan_seed(&mut app, "bacon");
    scan_seed(&mut app, "zebra");
    app.press(Action::Cancel);
    app.press(Action::Family(F::NoDescription));
    // The wallet's shape: M of N, the cosigners, the kind and the path.
    let text = drawn(&mut app);
    assert!(text.contains("2 of 2"), "{text}");
    assert!(!text.contains("I can't find one"), "{text}");
    // The third seed, typed.
    app.press(Action::Family(F::TypeWords));
    type_text(&mut app, &testkit::test_words("summer"));
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Family);
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    assert_eq!(chosen_address(&app), kit_address("savings"));
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
}

#[test]
fn a_transaction_scanned_on_the_card_is_kept_for_bring() {
    let mut app = shown();
    on_intake(&mut app);
    let (_, psbt) = savings_psbt();
    scan(&mut app, psbt);
    assert_eq!(note(&app), "A transaction to sign · kept for later");
    assert!(app.spend().is_none(), "the spend started before the wallet");
    scan(&mut app, testkit::savings().into_bytes());
    app.press(Action::Cancel);
    assert!(app.family_ready());
    assert!(app.spend().is_none(), "the spend started before Bring");
    app.press(Action::Family(F::Next(page::CHECK)));
    app.press(Action::Family(F::Next(page::WRITE)));
    assert_eq!(app.family().open, Some(Open::Page(page::BRING)));
    let _ = drawn(&mut app);
    let k = app
        .inbox()
        .iter()
        .position(|i| i.kind == FileKind::Psbt)
        .expect("the transaction is not in Files");
    assert!(app.offers(Action::Family(F::UsePsbt(k))));
    app.press(Action::Family(F::UsePsbt(k)));
    assert_eq!(app.spend().unwrap().wallet, app.family_wallet());
}

#[test]
fn a_lock_on_the_card_comes_back_with_what_came_in() {
    let mut app = shown();
    on_intake(&mut app);
    let (_, psbt) = savings_psbt();
    scan(&mut app, psbt);
    scan_seed(&mut app, "bacon");
    app.press(Action::Cancel);
    let text = drawn(&mut app);
    assert!(text.contains("Transaction · kept for later"), "{text}");
    assert!(text.contains("Seed · "), "{text}");
    let mut app = after_lock(app);
    assert_eq!(app.screen, Screen::Family);
    assert_eq!(app.family().open, Some(Open::Page(page::HOLDING)));
    let text = drawn(&mut app);
    assert!(text.contains("Transaction · kept for later"), "{text}");
    // Keys come back by a vault or not at all.
    assert!(!text.contains("Seed · "), "{text}");
}

#[test]
fn a_seed_already_loaded_comes_in_again_after_start_over() {
    let mut app = shown();
    on_intake(&mut app);
    scan_seed(&mut app, "bacon");
    app.press(Action::Cancel);
    app.press(Action::Family(F::StartOver));
    assert_eq!(app.family_question(), Question::None);
    scan_seed(&mut app, "bacon");
    app.press(Action::Cancel);
    assert_eq!(app.screen, Screen::Family);
    assert_eq!(app.family_question(), Question::Anything);
}

/// One seed and That's everything: Check the money, guessed.
fn guessed_single() -> Faraday {
    let mut app = shown_at(2000);
    on_intake(&mut app);
    scan_seed(&mut app, "bacon");
    app.press(Action::Cancel);
    app.press(Action::Family(F::Everything));
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
    app
}

#[test]
fn no_money_shows_walks_through_the_passphrase_taproot_nested_and_legacy() {
    let mut app = guessed_single();
    let text = drawn(&mut app);
    assert!(
        text.contains("Check the money is there, on the online computer"),
        "{text}"
    );
    assert!(
        text.contains("Trying: native SegWit, no passphrase · 1 of 5"),
        "{text}"
    );
    assert!(text.contains("Open this in the wallet on the online computer"));
    assert_eq!(chosen_address(&app), kit_address("spending"));
    app.press(Action::Family(F::NoMoney));
    let text = drawn(&mut app);
    assert!(
        text.contains("Is there a word or sentence kept apart from the list?"),
        "{text}"
    );
    assert!(text.contains("native SegWit · no money"), "{text}");
    app.press(Action::Family(F::Passphrase(false)));
    assert_eq!(chosen_address(&app), kit_address("taproot"));
    let text = drawn(&mut app);
    assert!(
        text.contains("Trying: Taproot, no passphrase · 3 of 5"),
        "{text}"
    );
    assert!(
        text.contains("Open this one on the online computer too"),
        "{text}"
    );
    app.press(Action::Family(F::NoMoney));
    assert_eq!(chosen_address(&app), kit_address("nested"));
    app.press(Action::Family(F::NoMoney));
    assert_eq!(chosen_address(&app), kit_address("legacy"));
    // No money shows is never the way on.
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
    app.press(Action::Family(F::NoMoney));
    let text = drawn(&mut app);
    assert!(text.contains("None of these show money."), "{text}");
    assert!(text.contains("legacy · no money"), "{text}");
    assert!(!text.contains("Yes, my money shows"), "{text}");
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
    // Scan more: back to What do you have?, the camera on.
    app.press(Action::Family(F::Again(true)));
    assert_eq!(app.family().open, Some(Open::Page(page::HOLDING)));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.family_wallet().is_none());
}

#[test]
fn yes_my_money_shows_is_the_way_on() {
    let mut app = guessed_single();
    app.press(Action::Family(F::NoMoney));
    app.press(Action::Family(F::Passphrase(false)));
    let text = drawn(&mut app);
    assert!(text.contains("Yes, my money shows"), "{text}");
    app.press(Action::Family(F::Next(page::CHECK)));
    assert_eq!(app.family().open, Some(Open::Page(page::WRITE)));
    // The Taproot wallet is the one spent from.
    assert_eq!(chosen_address(&app), kit_address("taproot"));
}

#[test]
fn the_passphrase_question_tries_native_segwit_with_the_passphrase() {
    let mut app = guessed_single();
    app.press(Action::Family(F::NoMoney));
    app.press(Action::Family(F::Passphrase(true)));
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    for f in [0u8, 1] {
        app.press(Action::SeedRead(Choice::Field(f)));
        type_text(&mut app, "kept apart");
    }
    app.press(Action::SeedRead(Choice::WithPass));
    assert_eq!(app.sheet, None);
    let text = drawn(&mut app);
    assert!(
        text.contains("Trying: native SegWit, with a passphrase · 2 of 5"),
        "{text}"
    );
    assert_ne!(chosen_address(&app), kit_address("spending"));
}

#[test]
fn a_described_wallets_check_has_no_no_money_shows() {
    let mut app = shown_at(2000);
    on_intake(&mut app);
    scan(&mut app, testkit::savings().into_bytes());
    app.press(Action::Cancel);
    assert_eq!(app.family().open, Some(Open::Page(page::CHECK)));
    let text = drawn(&mut app);
    assert!(text.contains("Yes, my money shows"), "{text}");
    assert!(!text.contains("No money shows"), "{text}");
    assert!(!text.contains("Trying:"), "{text}");
}

/// The Spend tab left the sidebar and the small Home grid
/// (`docs/history/SIMPLIFY.md` §6.2); it is still reached from the Wallets empty
/// state's fourth way in.
#[test]
fn the_spend_tab_opens_from_the_wallets_empty_state() {
    let mut app = shown();
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    assert!(app.offers(Action::Nav(Screen::Family)));
    app.press(Action::Nav(Screen::Family));
    assert_eq!(app.screen, Screen::Family);
}

/// The Learn sheet's first row is the other way in, from any screen that
/// is not the Spend tab itself.
#[test]
fn the_spend_tab_opens_from_learn() {
    let mut app = shown();
    app.press(Action::Learn);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Learn));
    assert!(app.offers(Action::LearnSpend));
    app.press(Action::LearnSpend);
    assert_eq!(app.screen, Screen::Family);
    assert_eq!(app.sheet, None);
}
