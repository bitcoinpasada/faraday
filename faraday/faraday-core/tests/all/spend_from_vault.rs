//! From the vault to a spend (`docs/history/NEW-WALLET.md` §11): the boot
//! stick's vault unlocked after the pull lands on Wallets with the
//! wallet that signs here and a toast; the card's first button opens the
//! Spend tab on the first page the wallet still needs; a PSBT made in
//! Sparrow is scanned at once into its review; a 2-of-3 with one key here
//! offers adding a key and collecting a signature; and the spend resumes
//! where it was.

use faraday_core::boot_import::ImportAction as I;
use faraday_core::family::{CardId, FamilyAction as F, Open, page};
use faraday_core::testkit;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageCommand, StorageEvent};
use faraday_shell_api::{App, Event, Key};

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// Answers the app's reads from the test vault, the one file on the
/// boot stick.
fn pump(app: &mut Faraday, vault: &[u8]) {
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: vault.to_vec(),
            });
        }
    }
}

/// Started from the boot stick, the stick pulled, the vault's passphrase
/// typed on the import sheet, Unlock pressed and its Load sheet's Load
/// ticked, with everything ticked.
fn unlocked() -> Faraday {
    let vault = testkit::test_vault().unwrap();
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(100);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        files: vec![("vault.ofv".into(), vault.len() as u64)],
    }]));
    pump(&mut app, &vault);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    app.press(Action::Import(I::Go));
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Import(I::Submit));
    let _ = app.frame();
    app.event(Event::Tick { now_ms: 1000 });
    let _ = app.frame();
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::UnlockLoad));
    app.press(Action::Vault(faraday_core::vault::VaultAction::LoadTicked));
    let _ = app.frame();
    app
}

fn wallet_at(app: &Faraday, name: &str) -> usize {
    app.session
        .wallets
        .iter()
        .position(|w| w.name == name)
        .unwrap()
}

fn keys_here(app: &Faraday, w: usize) -> usize {
    let wl = &app.session.wallets[w];
    app.session
        .slots(wl)
        .iter()
        .filter(|s| s.held_by.is_some())
        .count()
}

/// The texts on screen once what moves has come to rest.
fn drawn(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.settle();
    let _ = app.frame();
    app.drawn_texts().join("\n")
}

/// Unlocked, on the card of the wallet called `name`.
fn on_card(name: &str) -> (Faraday, usize) {
    let mut app = unlocked();
    let w = wallet_at(&app, name);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    (app, w)
}

/// The PSBT Sparrow shows, scanned on the camera.
fn scan_psbt(app: &mut Faraday, psbt: &[u8]) {
    app.press(Action::Scan);
    for part in faraday_qr::bbqr::encode('P', psbt, 60).unwrap() {
        app.event(Event::Scanned {
            bytes: part.into_bytes(),
        });
    }
}

#[test]
fn unlocking_the_boot_sticks_vault_lands_on_wallets_with_the_one_that_signs_and_a_toast() {
    let app = unlocked();
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.sheet, None);
    let n = app.session.wallets.len();
    assert!(n > 1, "the test vault holds every test wallet");
    assert_eq!(
        app.toast_text(),
        Some(format!("{n} wallets loaded from vault.ofv").as_str())
    );
    // The first wallet with a key here is the one shown.
    let first = (0..n).find(|&w| keys_here(&app, w) > 0).unwrap();
    assert_eq!(app.wallet, first);
}

#[test]
fn spend_from_this_wallet_opens_on_load_the_wallet_in_sparrow() {
    let (mut app, w) = on_card("Spending");
    let text = drawn(&mut app);
    assert!(text.contains("Spend from this wallet"), "{text}");
    assert!(app.offers(Action::Family(F::SpendFrom(w))));
    app.press(Action::Family(F::SpendFrom(w)));
    assert_eq!(app.screen, Screen::Family);
    assert_eq!(app.family.open, Some(Open::Page(page::CHECK)));
    // Past What do you have?.
    let cards = app.family_cards();
    assert_eq!(cards.first(), Some(&CardId::Page(page::CHECK)));
    assert!(!cards.contains(&CardId::Page(page::HOLDING)));
    assert!(!cards.contains(&CardId::Page(page::OPEN)));
    let text = drawn(&mut app);
    assert!(text.contains("Load the wallet in Sparrow"), "{text}");
    assert!(app.offers(Action::QrWallet(w)), "the wallet QR");
    // It is loaded: Write the payment in Sparrow, with the PSBT's ways in.
    app.press(Action::Family(F::Next(page::CHECK)));
    assert_eq!(app.family.open, Some(Open::Page(page::WRITE)));
    let text = drawn(&mut app);
    assert!(text.contains("Write the payment in Sparrow"), "{text}");
    assert!(text.contains("Scan the PSBT"), "{text}");
    assert!(text.contains("On a stick"), "{text}");
}

#[test]
fn the_shortcut_scans_the_psbt_from_sparrow_and_opens_its_review() {
    let (mut app, w) = on_card("Spending");
    app.press(Action::Family(F::SpendFrom(w)));
    let text = drawn(&mut app);
    assert!(
        text.contains("I have the PSBT from Sparrow ready: Scan it"),
        "{text}"
    );
    assert!(app.offers(Action::Scan));
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    scan_psbt(&mut app, &psbt);
    assert_eq!(app.screen, Screen::Family);
    assert_eq!(app.family.open, Some(Open::Spend));
    let s = app.spend.as_ref().expect("no spend");
    assert_eq!(s.wallet, Some(w));
    assert_eq!(s.open, Some(step::TRANSACTION), "the review is not next");
}

#[test]
fn with_its_psbt_in_files_the_button_signs_it_and_opens_its_review() {
    let (mut app, w) = on_card("Spending");
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    app.storage(StorageEvent::Restored {
        inbox: vec![("spend.psbt".to_string(), psbt)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::OpenWallet(w));
    let text = drawn(&mut app);
    assert!(text.contains("Sign the PSBT from the stick"), "{text}");
    app.press(Action::Family(F::SpendFrom(w)));
    assert_eq!(app.screen, Screen::Family);
    assert_eq!(app.family.open, Some(Open::Spend));
    let s = app.spend.as_ref().expect("no spend");
    assert_eq!(s.wallet, Some(w));
    assert_eq!(s.open, Some(step::TRANSACTION));
}

#[test]
fn a_two_of_three_with_one_key_here_offers_scanning_a_key_and_collecting_a_signature() {
    let (mut app, w) = on_card("Savings");
    assert_eq!(keys_here(&app, w), 1);
    app.press(Action::Family(F::SpendFrom(w)));
    let (psbt, _) = testkit::spend_of(&kit("savings")).unwrap();
    scan_psbt(&mut app, &psbt.to_bytes());
    assert_eq!(app.spend.as_ref().and_then(|s| s.wallet), Some(w));
    app.press(Action::Step(step::SIGNERS));
    let text = drawn(&mut app);
    assert!(
        text.contains("1 of 2 signatures here · 1 more needed"),
        "{text}"
    );
    assert!(text.contains("Not here"), "{text}");
    assert!(!text.contains("Type its words"), "{text}");
    assert!(app.offers(Action::ScanKey(w)), "Scan a key");
    assert!(!app.offers(Action::ScanSeed), "Scan a SeedQR");
    assert!(text.contains("Collect a signature"), "{text}");
    assert!(
        app.offers(Action::Step(step::COLLECT)),
        "Collect a signature"
    );
    app.press(Action::Step(step::COLLECT));
    assert_eq!(app.family.open, Some(Open::Spend));
    assert_eq!(app.spend.as_ref().unwrap().open, Some(step::COLLECT));
}

/// On Savings' Who signs, with Test key 1 here.
fn who_signs() -> (Faraday, usize) {
    let (mut app, w) = on_card("Savings");
    app.press(Action::Family(F::SpendFrom(w)));
    let (psbt, _) = testkit::spend_of(&kit("savings")).unwrap();
    scan_psbt(&mut app, &psbt.to_bytes());
    app.press(Action::Step(step::SIGNERS));
    let _ = app.frame();
    (app, w)
}

/// The camera reads these words' SeedQR.
fn scan_seedqr(app: &mut Faraday, words: &str) {
    let c = testkit::compute();
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, words)
        .unwrap();
    app.event(Event::Scanned {
        bytes: faraday_codec::seedqr::to_digits(&m)
            .expose()
            .as_bytes()
            .to_vec(),
    });
    let _ = app.frame();
}

#[test]
fn scan_a_key_on_who_signs_reaches_can_spend_and_the_buttons_go() {
    let (mut app, w) = who_signs();
    app.press(Action::ScanKey(w));
    scan_seedqr(&mut app, &testkit::test_words("zebra"));
    assert_eq!(app.sheet, None, "the camera stayed open");
    assert_eq!(app.screen, Screen::Family);
    let text = drawn(&mut app);
    assert!(text.contains("2 of 2 signatures here"), "{text}");
    assert!(!text.contains("more needed"), "{text}");
    assert!(text.contains("Can spend"), "{text}");
    assert!(!app.offers(Action::ScanKey(w)), "Scan a key");
    assert!(!app.offers(Action::ScanSeed), "Scan a SeedQR");
    assert!(!text.contains("Collect a signature"), "{text}");
    assert!(app.offers(Action::StepNext(step::SIGNERS)), "Continue");
}

#[test]
fn a_key_scanned_with_no_name_shows_its_fingerprint_once() {
    let c = testkit::compute();
    let (mut app, w) = who_signs();
    app.press(Action::ScanKey(w));
    scan_seedqr(&mut app, &testkit::test_words("zebra"));
    let fp = {
        let mut s = testkit::session();
        faraday_core::wallet::fp_text(
            s.keys
                .add_words(&c, &testkit::test_words("zebra"), "", None)
                .unwrap(),
        )
    };
    // Its row: the fingerprint, then Signs here, with no name between.
    let text = drawn(&mut app);
    assert!(text.contains(&format!("{fp}\nSigns here")), "{text}");
    assert!(!text.contains(&format!("{fp}\n{fp}")), "{text}");
}

#[test]
fn leaving_the_spend_and_coming_back_resumes_it() {
    let (mut app, w) = on_card("Spending");
    app.press(Action::Family(F::SpendFrom(w)));
    // Before the PSBT: back on the page it was on.
    app.press(Action::Family(F::Next(page::CHECK)));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    app.press(Action::Family(F::SpendFrom(w)));
    assert_eq!(app.family.open, Some(Open::Page(page::WRITE)));

    // With the PSBT: the card and Home say how far it is, and resume it.
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    scan_psbt(&mut app, &psbt);
    app.press(Action::StepNext(step::TRANSACTION));
    let open = app.spend.as_ref().unwrap().open;
    assert_ne!(open, Some(step::TRANSACTION));
    app.press(Action::OpenWallet(w));
    let text = drawn(&mut app);
    assert!(
        text.contains("Carry on the spend · Signatures 0 of 1"),
        "{text}"
    );
    app.press(Action::Nav(Screen::Home));
    let text = drawn(&mut app);
    assert!(text.contains("Carry on the spend"), "{text}");
    assert!(app.offers(Action::Family(F::SpendFrom(w))));
    app.press(Action::Family(F::SpendFrom(w)));
    assert_eq!(app.screen, Screen::Family);
    assert_eq!(app.family.open, Some(Open::Spend));
    assert_eq!(app.spend.as_ref().unwrap().open, open);
}

#[test]
fn wallets_sign_step_says_how_many_more_and_offers_scan_a_key() {
    let (mut app, w) = on_card("Savings");
    let (psbt, _) = testkit::spend_of(&kit("savings")).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![("savings.psbt".to_string(), psbt.to_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::StartSpend(0));
    assert_eq!(app.screen, Screen::Spend);
    app.press(Action::Step(step::SIGNERS));
    let text = drawn(&mut app);
    assert!(
        text.contains("1 of 2 signatures here · 1 more needed"),
        "{text}"
    );
    assert!(!text.contains("Add its key"), "{text}");
    assert!(app.offers(Action::ScanKey(w)), "Scan a key");
    app.press(Action::ScanKey(w));
    scan_seedqr(&mut app, &testkit::test_words("summer"));
    assert_eq!(app.screen, Screen::Spend);
    let text = drawn(&mut app);
    assert!(text.contains("2 of 2 signatures here"), "{text}");
    assert!(text.contains("Can spend"), "{text}");
    assert!(!app.offers(Action::ScanKey(w)), "Scan a key");
}
