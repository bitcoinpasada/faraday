//! The Spend tab (`docs/FAMILY.md`): someone spending for the first time
//! goes from what is in the envelope to a signed transaction on one tab,
//! through the vault's unlock and load, typed words, or split sheets; the
//! tab's place survives a lock; and what it signs is byte for byte what
//! Wallets › Sign a transaction signs.

use faraday_core::family::{FamilyAction as F, Open, Route, page};
use faraday_core::testkit;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

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
    app
}

/// A descriptor as text, hardened steps written one way.
fn desc(p: &osk_bip::policy::WalletPolicy) -> String {
    p.to_descriptor().replace('\'', "h")
}

fn vault_file() -> (String, Vec<u8>) {
    ("vault.ofv".to_string(), testkit::test_vault().unwrap())
}

fn savings_psbt() -> (String, Vec<u8>) {
    let (u, _) = testkit::spend_of(&kit("savings")).unwrap();
    ("savings-unsigned.psbt".to_string(), u.to_bytes())
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Frames and ticks, so work waiting for a drawn frame (Argon2id) runs.
fn settle(app: &mut Faraday) {
    for t in 1..20u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn inbox_at(app: &Faraday, name: &str) -> usize {
    app.inbox.iter().position(|i| i.name == name).unwrap()
}

fn wallet_at(app: &Faraday, name: &str) -> usize {
    app.session
        .wallets
        .iter()
        .position(|w| w.name == name)
        .unwrap()
}

/// The vault route as far as the wallet: the tab, the answer, the
/// passphrase, Unlock.
fn unlock_on_tab(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Next(page::MAP)));
    app.press(Action::Family(F::Next(page::SAFE)));
    app.press(Action::Family(F::Holding(Route::Vault)));
    app.press(Action::Family(F::Pick(0)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Family(F::Unlock));
    settle(app);
    // Unlocked: everything is ticked, nothing loaded until Load.
    assert!(app.session.keys.is_empty(), "the unlock loaded before Load");
    assert!(app.vault_rows(0).iter().all(|r| r.chosen));
    app.press(Action::Vault(
        faraday_core::vaults::VaultAction::LoadChosen(0),
    ));
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
    assert_eq!(app.family.open, Some(Open::Page(page::CHECK)));
    app.press(Action::Family(F::Next(page::CHECK)));
    app.press(Action::Family(F::Next(page::WRITE)));
    assert_eq!(app.family.open, Some(Open::Page(page::BRING)));

    let at = inbox_at(&app, "savings-unsigned.psbt");
    app.press(Action::Family(F::UsePsbt(at)));
    assert_eq!(app.screen, Screen::Family, "the spend left the tab");
    assert_eq!(app.family.open, Some(Open::Spend));
    let s = app.spend.as_ref().unwrap();
    assert_eq!(s.wallet, Some(savings), "the spend's wallet is not Savings");
    assert_eq!(s.open, Some(step::TRANSACTION), "the review is not first");

    app.press(Action::SignHere);
    let here = app.spend.as_ref().unwrap().spend.psbt.to_bytes();
    assert!(!app.spend.as_ref().unwrap().spend.signed_here.is_empty());

    // Wallets › Sign a transaction, from the same files, signs the same
    // bytes.
    let mut other = with_files(vec![vault_file(), savings_psbt()], Vec::new());
    other.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
    type_text(&mut other, testkit::VAULT_PASSPHRASES[0]);
    other.press(Action::Vault(faraday_core::vaults::VaultAction::Unlock));
    settle(&mut other);
    other.press(Action::Vault(faraday_core::vaults::VaultAction::LoadAll(0)));
    let at = inbox_at(&other, "savings-unsigned.psbt");
    other.press(Action::StartSpend(at));
    other.press(Action::SignHere);
    assert_eq!(here, other.spend.as_ref().unwrap().spend.psbt.to_bytes());
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
    // Test key 2's slot, from the signing card.
    let fp = {
        let w = &app.session.wallets[savings];
        app.session
            .slots(w)
            .iter()
            .find(|s| s.held_by.is_none())
            .and_then(|s| s.fingerprint)
            .unwrap()
    };
    app.press(Action::Entry(Some(fp.0)));
    assert_eq!(app.screen, Screen::Entry);
    type_text(&mut app, &testkit::test_words("zebra"));
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Family, "adding the key left the tab");
    assert_eq!(app.session.keys.len(), 2);
    app.press(Action::SignHere);
    let s = app.spend.as_ref().unwrap();
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
    let fp = {
        let w = &app.session.wallets[savings];
        app.session
            .slots(w)
            .iter()
            .find(|s| s.held_by.is_none())
            .and_then(|s| s.fingerprint)
            .unwrap()
    };
    app.press(Action::Entry(Some(fp.0)));
    let unrelated = ["abandon"; 11].join(" ") + " about";
    type_text(&mut app, &unrelated);
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Entry, "a key for no slot was taken");
    assert!(app.entry.error.is_some());
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_lock_comes_back_to_the_same_page_and_wallet_after_unlocking_again() {
    let mut app = with_files(vec![vault_file()], Vec::new());
    unlock_on_tab(&mut app);
    let savings = wallet_at(&app, "Savings");
    app.press(Action::Family(F::Choose(savings)));
    app.press(Action::Family(F::Next(page::OPEN)));
    app.press(Action::Family(F::Next(page::CHECK)));
    app.press(Action::Family(F::Next(page::WRITE)));
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

    // The next process: the tab, waiting for the vault again.
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    assert_eq!(
        app.screen,
        Screen::Family,
        "the lock did not return to the tab"
    );
    assert_eq!(app.family.route, Some(Route::Vault));
    assert_eq!(app.family.open, Some(Open::Page(page::OPEN)));
    // The PSBT copied in during the visit.
    let (name, bytes) = savings_psbt();
    app.storage(StorageEvent::Restored {
        inbox: {
            let mut v: Vec<(String, Vec<u8>)> = app
                .inbox
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
    app.press(Action::Vault(
        faraday_core::vaults::VaultAction::LoadChosen(0),
    ));
    assert_eq!(app.family_wallet(), Some(wallet_at(&app, "Savings")));
    assert_eq!(
        app.family.open,
        Some(Open::Page(page::BRING)),
        "the pages done before the lock were asked for again"
    );
}

#[test]
fn typed_words_open_the_kind_of_wallet_the_transaction_spends_from() {
    let taproot = kit("taproot");
    let (u, _) = testkit::spend_of(&taproot).unwrap();
    let mut app = with_files(
        vec![("taproot-unsigned.psbt".to_string(), u.to_bytes())],
        Vec::new(),
    );
    app.press(Action::Network(testkit::NET));
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Holding(Route::Words)));
    app.press(Action::Family(F::TypeWords));
    assert_eq!(app.screen, Screen::Entry);
    type_text(&mut app, &testkit::test_words("bacon"));
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Family);
    // Native SegWit first, as the Spend tab opens it: the Spending
    // test wallet.
    let w = app.family_wallet().expect("no wallet opened");
    let spending = faraday_core::wallet::read_wallet(&kit("spending").descriptor).unwrap();
    assert_eq!(desc(&app.session.wallets[w].policy), desc(&spending));
    // A Taproot spend opens the Taproot kind of the same words.
    app.press(Action::Family(F::UsePsbt(0)));
    let s = app.spend.as_ref().unwrap();
    let w = s.wallet.expect("no wallet matched the Taproot spend");
    let tr = faraday_core::wallet::read_wallet(&taproot.descriptor).unwrap();
    assert_eq!(desc(&app.session.wallets[w].policy), desc(&tr));
    app.press(Action::SignHere);
    assert!(app.spend.as_ref().unwrap().complete);
}

#[test]
fn split_sheets_rebuild_the_wallet_on_the_tab() {
    let files: Vec<(String, Vec<u8>)> = testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| n.starts_with("savings-share-"))
        .take(2)
        .collect();
    assert_eq!(files.len(), 2);
    let mut app = with_files(files, Vec::new());
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Holding(Route::Paper)));
    app.press(Action::Family(F::Rebuild));
    let w = app.family_wallet().expect("two sheets did not rebuild it");
    let savings = faraday_core::wallet::read_wallet(&testkit::savings()).unwrap();
    assert_eq!(desc(&app.session.wallets[w].policy), desc(&savings));
    assert_eq!(app.family.open, Some(Open::Page(page::CHECK)));
}

#[test]
fn one_sheet_says_what_is_still_missing() {
    let files: Vec<(String, Vec<u8>)> = testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| n.starts_with("savings-share-"))
        .take(1)
        .collect();
    let mut app = with_files(files, Vec::new());
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Holding(Route::Paper)));
    app.press(Action::Family(F::Rebuild));
    assert!(app.family_wallet().is_none());
    assert!(
        app.family
            .error
            .as_deref()
            .is_some_and(|e| e.contains("of 3 keys in hand"))
    );
}

#[test]
fn with_several_wallets_and_no_transaction_the_person_chooses() {
    let mut app = with_files(vec![vault_file()], Vec::new());
    unlock_on_tab(&mut app);
    assert!(app.family_wallet().is_none());
    assert_eq!(app.family.open, Some(Open::Page(page::OPEN)));
    let savings = wallet_at(&app, "Savings");
    app.press(Action::Family(F::Choose(savings)));
    app.press(Action::Family(F::Next(page::OPEN)));
    assert_eq!(app.family.open, Some(Open::Page(page::CHECK)));
}

#[test]
fn no_page_past_the_wallet_opens_without_one() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Card(page::BRING)));
    assert_eq!(app.family.open, Some(Open::Page(page::HOLDING)));
    app.press(Action::Family(F::Holding(Route::Vault)));
    app.press(Action::Family(F::Card(page::CHECK)));
    assert_eq!(app.family.open, Some(Open::Page(page::OPEN)));
}
