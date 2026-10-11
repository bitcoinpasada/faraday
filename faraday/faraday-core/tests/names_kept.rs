//! A place's name never leaves the vault: a backup plan's map (record
//! type 11, field 4) names a place by number only, "Place n", the same
//! as its blank template and its locked summary (`docs/VAULT.md` §9,
//! §11). The name itself stays in field 3, read back only while the
//! vault that keeps it is open. A vault written before this field 4
//! named the place is read with that name replaced on the way into the
//! kept summary.

use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent, qstep};
use faraday_vault::records::{Record, field, kind};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

const NAME: &str = "Grandmas attic";

type Files = Vec<(String, Vec<u8>)>;

fn device(inbox: Files) -> Faraday {
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
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    let _ = app.frame();
    app
}

fn shown(inbox: Files, outbox: Files, kept: Files) -> Faraday {
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
    app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    let _ = app.frame();
    app
}

fn vault_file() -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .unwrap()
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn settle(app: &mut Faraday, until: impl Fn(&Faraday) -> bool) {
    for t in 1..60u64 {
        if until(app) {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn unlock(app: &mut Faraday) {
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
}

/// Test key 2 typed in and a one-key wallet made from it; returns the
/// wallet's index.
fn one_key(app: &mut Faraday) -> usize {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().master.fingerprint();
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    app.session.wallets.len() - 1
}

fn press_offered(app: &mut Faraday, action: Action) {
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(action) {
            app.press(action);
            return;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    panic!("{action:?} is not offered");
}

/// What the lock saved: the Inbox, the Outbox and the kept state.
fn saved(app: &mut Faraday) -> (Files, Files, Files) {
    let mut last = (Vec::new(), Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            last = (inbox, outbox, kept);
        }
    }
    last
}

fn holds(bytes: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && bytes.windows(needle.len()).any(|w| w == needle)
}

const VAULT_PHRASE: &str = "test phrase";

/// A fresh, empty vault made here and unlocked, holding nothing of its
/// own, so a wallet saved into it can be found again with no other
/// wallet to confuse it with.
fn made_and_unlocked(app: &mut Faraday) {
    app.vaults.ms_per_unit = Some(180);
    for i in 0..8u8 {
        app.event(Event::Entropy(osk_shell_api::EntropyBytes::new(
            [0x50 + i; 32],
        )));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(app, VAULT_PHRASE);
    }
    app.press(Action::Vault(V::CGo));
    settle(app, |a| a.screen == Screen::Unlock);
    type_text(app, VAULT_PHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not unlock");
}

fn unlock_made(app: &mut Faraday) {
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, VAULT_PHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not unlock again");
}

/// A place named "Grandmas attic" with a vault open, a plan written for
/// it by the checklist, and the device locked: nothing kept across the
/// lock, and nothing queued for the stick, holds the name; it is shown
/// again once the same vault is unlocked.
#[test]
fn a_place_names_name_never_leaves_the_vault() {
    let mut app = device(Vec::new());
    made_and_unlocked(&mut app);
    let w = one_key(&mut app);
    app.press(Action::Backup(w));
    press_offered(&mut app, Action::BPreset(1));
    press_offered(&mut app, Action::BQ(qstep::PLACES));
    press_offered(&mut app, Action::BName(0));
    type_text(&mut app, NAME);
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.place_name(0), NAME);
    press_offered(&mut app, Action::BChecklist);
    app.press(Action::BReveal);
    app.storage(StorageEvent::Cameras(Vec::new()));
    press_offered(&mut app, Action::BCheck);
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let digits = osk_codec::seedqr::to_digits(
        &osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, &words).unwrap(),
    );
    type_text(
        &mut app,
        std::str::from_utf8(digits.expose().as_bytes()).unwrap(),
    );
    press_offered(&mut app, Action::BStep(faraday_core::bstep::VAULT));
    press_offered(&mut app, Action::BVaultSave(0));
    // The wallet itself into the vault too, so the next process can load
    // it back and find the plan kept for it.
    app.press(Action::Vault(V::SaveWallet(w)));
    // The record kept in the vault holds the name in field 3 alone.
    let plan = app.vaults.open[0]
        .contents
        .of(kind::PLAN)
        .next()
        .expect("a plan record")
        .1
        .clone();
    assert_eq!(plan.text(field::PLAN_PLACE), Some(NAME));
    for f in plan.fields.iter().filter(|f| f.number == field::PLAN_HOLDS) {
        assert!(
            !holds(&f.bytes, NAME.as_bytes()),
            "field 4 holds the place's name: {:?}",
            String::from_utf8_lossy(&f.bytes)
        );
    }

    app.press(Action::Lock);
    let (_, outbox, kept) = saved(&mut app);
    for (name, bytes) in &kept {
        assert!(
            !holds(bytes, NAME.as_bytes()),
            "{name}, kept on locking, holds the place's name"
        );
    }
    for (name, bytes) in &outbox {
        assert!(
            !holds(bytes, NAME.as_bytes()),
            "{name}, queued for the stick, holds the place's name"
        );
    }

    // Unlocked again, with the wallet loaded back from the vault, the
    // name is shown as before. Locking seals the vault's new bytes into
    // the outbox, as if written back to the stick; the next process
    // meets them again as its Inbox.
    let sealed_vault = outbox
        .iter()
        .find(|(n, _)| n == "vault.ofv")
        .cloned()
        .expect("the locked vault was sealed for the stick");
    let mut app = shown(vec![sealed_vault], Vec::new(), kept);
    unlock_made(&mut app);
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(
        app.session.wallets.len(),
        1,
        "the wallet did not load back from the vault"
    );
    app.press(Action::Backup(0));
    assert_eq!(
        app.backup.as_ref().map(|b| b.q),
        Some(None),
        "a fresh backup flow started instead of reading the kept plan"
    );
    assert_eq!(app.place_name(0), NAME, "the name is not shown once open");
}

/// A vault whose plan record was written before field 4 named the place
/// by number: read into the kept summary, the name is replaced by
/// "Place n", not carried into the kept state as a name.
#[test]
fn a_plan_written_the_old_way_is_read_with_its_place_named_by_number() {
    let mut app = device(vec![vault_file()]);
    unlock(&mut app);
    let mut record = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, b"legacy-wallet")
        .with(field::PLAN_ANSWERS, b"places 1\n");
    record.push(field::PLAN_PLACE, NAME.as_bytes());
    record.push(field::PLAN_HOLDS, format!("{NAME}: 24 words").as_bytes());
    app.vaults.open[0].contents.records.push(record);

    app.press(Action::Lock);
    let (_, outbox, kept) = saved(&mut app);
    for (name, bytes) in &kept {
        assert!(
            !holds(bytes, NAME.as_bytes()),
            "{name}, kept on locking, holds the legacy place's name"
        );
    }
    for (name, bytes) in &outbox {
        assert!(
            !holds(bytes, NAME.as_bytes()),
            "{name}, queued for the stick, holds the legacy place's name"
        );
    }
    // The summary keeps the map line, its place named by number.
    let summary = kept
        .iter()
        .find(|(name, _)| name == "vault-summaries")
        .expect("a kept vault summary");
    assert!(
        holds(&summary.1, b"Place 1"),
        "the map line is not kept at all, not just renamed"
    );
}
