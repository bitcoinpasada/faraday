//! The backup comes before spending (`docs/NEW-WALLET.md` §14): a key
//! made here signs nothing until its wallet's backup checklist is
//! complete, while a key restored from words signs at once; the hold
//! survives a lock, in the vault beside the seed; the wallet's address is
//! shown with a warning until then; a plan that keeps no copy of the key
//! is not made into a checklist; a plan changed part way keeps what was
//! done on the map and the hold until its own checklist is done; and the
//! done card leads with the three checks, above the chart.

use faraday_core::create::NewKind;
use faraday_core::keygen::Way;
use faraday_core::plan::{CHECK_LINES, Item, Preset};
use faraday_core::testkit::{self, Kit};
use faraday_core::vaults::VaultAction as V;
use faraday_core::wallet::fp_text;
use faraday_core::{
    Action, BStage, Faraday, Screen, StorageCommand, StorageEvent, bstep, cstep, plan, qrow,
};
use osk_bip::bip39::Mnemonic;
use osk_bip::keys::Fingerprint;
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

const WARNING: &str = "Back up before you receive";

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

fn device(inbox: Vec<(String, Vec<u8>)>, height: u16) -> Faraday {
    let mut app = Faraday::new();
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
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    // The system's randomness, which a new vault draws on.
    for i in 0..8u8 {
        app.event(Event::Entropy(osk_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    let _ = app.frame();
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn preset(p: Preset) -> u8 {
    Preset::ALL.iter().position(|x| *x == p).unwrap() as u8
}

/// A single-key wallet made in Create from a key New key makes from
/// coin flips, with no passphrase, back on Create's Back up card. The
/// same flips make the same key. Returns the wallet and the key's
/// fingerprint.
fn made_here(app: &mut Faraday) -> (usize, Fingerprint) {
    app.press(Action::CreateWallet);
    let at = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::NativeSegwit)
        .unwrap() as u8;
    app.press(Action::CKind(at));
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 7 + i / 5) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    assert!(
        app.keygen.as_ref().is_some_and(|k| k.locked),
        "the key did not lock in"
    );
    app.press(Action::KNext);
    assert_eq!(app.screen, Screen::Create);
    let w = app.create.as_ref().and_then(|c| c.built).expect("built");
    let fp = app.session.keys.last().unwrap().master.fingerprint();
    (w, fp)
}

/// The backup of wallet `w` from Create with preset `p`, and its
/// checklist made.
fn checklist(app: &mut Faraday, p: Preset) {
    app.press(Action::CBackup(preset(p)));
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
}

/// The copy by hand of the key's seed, checked by its numbers typed back.
fn check_the_copy(app: &mut Faraday, fp: Fingerprint) {
    let key = app
        .session
        .keys
        .iter()
        .find(|k| k.master.fingerprint() == fp)
        .unwrap();
    let words = key.words.as_ref().unwrap().to_string();
    let lang = key.language;
    let digits = osk_codec::seedqr::to_digits(&Mnemonic::parse(lang, &words).unwrap());
    app.press(Action::BCheck);
    type_text(
        app,
        std::str::from_utf8(digits.expose().as_bytes()).unwrap(),
    );
    assert!(
        app.backup_item_done(Item::Copy(0)),
        "the copy did not check"
    );
}

/// The rest of a Paper only checklist after the copy: the envelopes.
fn the_envelopes(app: &mut Faraday) {
    if app.backup.as_ref().unwrap().open != Some(bstep::ENVELOPE) {
        app.press(Action::BStep(bstep::ENVELOPE));
    }
    assert_eq!(app.backup.as_ref().unwrap().open, Some(bstep::ENVELOPE));
    app.press(Action::BNext(bstep::ENVELOPE));
}

fn kit_of(app: &Faraday, w: usize) -> Kit {
    Kit {
        id: "made",
        name: "Made here",
        descriptor: app.session.wallets[w].policy.to_descriptor_checksummed(),
        cosigner: None,
        fee: 1_000,
    }
}

#[test]
fn a_key_from_new_key_cannot_sign_until_its_wallets_checklist_is_complete_and_can_after() {
    // The key's wallet, made once to build a spend of it.
    let mut first = device(Vec::new(), 768);
    let (w, fp) = made_here(&mut first);
    let psbt = testkit::unsigned(&kit_of(&first, w)).unwrap();
    // The same flips on a device with that spend in its Inbox.
    let mut app = device(vec![("spend.psbt".to_string(), psbt.to_bytes())], 768);
    let (w, again) = made_here(&mut app);
    assert_eq!(again, fp, "the same flips made another key");
    assert!(app.key_held(fp));
    assert_eq!(app.spend_button(w).0, "Finish the backup first");
    // Signing is refused, and Who signs says why.
    let at = app
        .inbox
        .iter()
        .position(|i| i.name == "spend.psbt")
        .unwrap();
    app.press(Action::StartSpend(at));
    app.press(Action::SignHere);
    let s = app.spend.as_ref().unwrap();
    assert!(s.spend.finished.is_none(), "a held key signed");
    assert!(s.spend.signed_here.is_empty());
    assert!(
        s.error.as_deref().is_some_and(|e| e.contains("backup")),
        "{:?}",
        s.error
    );
    // Its checklist done, Paper only: it signs.
    app.press(Action::BackupFirst(w));
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BPreset(preset(Preset::Paper)));
    app.press(Action::BChecklist);
    check_the_copy(&mut app, fp);
    assert!(app.key_held(fp), "held before the envelopes");
    the_envelopes(&mut app);
    assert!(!app.key_held(fp), "still held with the checklist done");
    assert_ne!(app.spend_button(w).0, "Finish the backup first");
    app.press(Action::StartSpend(at));
    app.press(Action::SignHere);
    assert!(
        app.spend.as_ref().unwrap().spend.finished.is_some(),
        "{:?}",
        app.spend.as_ref().unwrap().error
    );
}

#[test]
fn a_restored_key_signs_at_once() {
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = testkit::unsigned(&k).unwrap();
    let mut app = device(
        vec![
            ("spend.psbt".to_string(), psbt.to_bytes()),
            ("spending-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ],
        768,
    );
    let at = |app: &Faraday, n: &str| app.inbox.iter().position(|i| i.name == n).unwrap();
    app.press(Action::LoadWallet(at(&app, "spending-wallet.txt")));
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys[0].master.fingerprint();
    assert!(!app.key_held(fp));
    assert_ne!(app.spend_button(0).0, "Finish the backup first");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    app.press(Action::SignHere);
    assert!(app.spend.as_ref().unwrap().spend.finished.is_some());
}

/// Locks `app` and returns what it queued for the stick.
fn locked(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    app.press(Action::Lock);
    let mut out = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = c {
            out = Some(outbox);
        }
    }
    out.expect("nothing queued on locking")
}

fn unlock(app: &mut Faraday) {
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
}

#[test]
fn after_a_lock_and_an_unlock_from_the_vault_the_held_key_is_still_held() {
    let mut app = device(vec![kit_file("vault.ofv")], 768);
    let (_, fp) = made_here(&mut app);
    checklist(&mut app, Preset::PaperVault);
    check_the_copy(&mut app, fp);
    app.press(Action::BStep(bstep::VAULT));
    assert_eq!(app.backup.as_ref().unwrap().open, Some(bstep::VAULT));
    unlock(&mut app);
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done(Item::Vault(0)));
    assert!(app.key_held(fp), "the checklist is not done");
    let vault: Vec<_> = locked(&mut app)
        .into_iter()
        .filter(|(n, _)| n == "vault.ofv")
        .collect();
    assert_eq!(vault.len(), 1, "the vault was not sealed");
    let mut next = device(vault, 768);
    unlock(&mut next);
    next.press(Action::Vault(V::LoadChosen(0)));
    assert!(
        next.session
            .keys
            .iter()
            .any(|k| k.master.fingerprint() == fp),
        "the key did not load"
    );
    assert!(next.key_held(fp), "the key signs after a lock");
}

#[test]
fn the_cards_address_carries_the_warning_until_the_backup_is_done() {
    let mut app = device(Vec::new(), 2400);
    let (w, fp) = made_here(&mut app);
    app.press(Action::OpenWallet(w));
    assert!(app.drawn_texts().iter().any(|t| t == WARNING));
    app.press(Action::Backup(w));
    app.press(Action::BPreset(preset(Preset::Paper)));
    app.press(Action::BChecklist);
    check_the_copy(&mut app, fp);
    the_envelopes(&mut app);
    app.press(Action::OpenWallet(w));
    assert!(!app.drawn_texts().iter().any(|t| t == WARNING));
}

#[test]
fn with_every_item_done_the_done_card_shows_the_three_checks_above_the_chart() {
    let mut app = device(Vec::new(), 2400);
    let (_, fp) = made_here(&mut app);
    checklist(&mut app, Preset::Paper);
    check_the_copy(&mut app, fp);
    the_envelopes(&mut app);
    let texts = app.drawn_texts();
    // A key node of the chart says where the key is.
    let chart = texts
        .iter()
        .position(|t| t == "Can sign here")
        .unwrap_or_else(|| panic!("no chart: {texts:?}"));
    for line in CHECK_LINES {
        // A line may wrap: its first words.
        let start: String = line.split(' ').take(3).collect::<Vec<_>>().join(" ");
        let at = texts
            .iter()
            .position(|t| t.starts_with(&start))
            .unwrap_or_else(|| panic!("{line} is not shown: {texts:?}"));
        assert!(at < chart, "{line} is under the chart");
    }
    assert!(app.offers(Action::WriteAsk));
    assert!(app.offers(Action::BPlan));
}

#[test]
fn a_vault_item_offers_its_one_action_at_a_time_and_holds_the_items_after_it() {
    let mut app = device(Vec::new(), 2400);
    let (_, fp) = made_here(&mut app);
    app.press(Action::CBackup(preset(Preset::PaperVault)));
    // No paper: the vault's item comes first.
    app.press(Action::BAnswer(qrow::SEEDS, plan::seeds::WORDS as u8));
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().open, Some(bstep::VAULT));
    let _ = app.frame();
    let make = Action::Vault(V::CreateFrom(Screen::Backup));
    assert!(app.offers(make), "Make a vault");
    assert!(!app.offers(Action::BVaultSave(0)));
    assert!(!app.offers(Action::BNext(bstep::VAULT)), "a Continue");
    // The next item does not open, nor does Continue pass it.
    app.press(Action::BStep(bstep::SHEETS));
    app.press(Action::BNext(bstep::VAULT));
    assert_eq!(app.backup.as_ref().unwrap().open, Some(bstep::VAULT));
    // The vault made: one button saves everything.
    app.press(make);
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(&mut app, "test phrase");
    }
    app.press(Action::Vault(V::CGo));
    for t in 1..60u64 {
        if app.screen == Screen::Unlock {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    for t in 60..120u64 {
        if !app.vaults.open.is_empty() && app.screen == Screen::Backup {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.screen, Screen::Backup);
    let _ = app.frame();
    assert!(app.offers(Action::BVaultSave(0)), "Save into the vault");
    assert!(!app.offers(make));
    assert!(!app.offers(Action::BNext(bstep::VAULT)));
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done(Item::Vault(0)));
    assert!(app.key_held(fp), "the rest of the checklist waits");
    let _ = app.frame();
    assert!(app.offers(Action::BNext(bstep::VAULT)), "Continue");
    assert!(!app.offers(Action::BVaultSave(0)));
    app.press(Action::BNext(bstep::VAULT));
    assert_ne!(app.backup.as_ref().unwrap().open, Some(bstep::VAULT));
}

#[test]
fn changing_to_paper_only_after_the_vault_keeps_its_copy_on_the_map_and_the_hold() {
    let mut app = device(vec![kit_file("vault.ofv")], 2400);
    let (_, fp) = made_here(&mut app);
    checklist(&mut app, Preset::PaperVault);
    check_the_copy(&mut app, fp);
    unlock(&mut app);
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done(Item::Vault(0)));
    // Paper only, part way.
    app.press(Action::BPlan);
    app.press(Action::BPreset(preset(Preset::Paper)));
    app.press(Action::BChecklist);
    assert!(!app.backup_items().contains(&bstep::VAULT));
    let line = format!("vault.ofv holds seed {}", fp_text(fp));
    let shown = app.drawn_texts().join(" ");
    assert!(shown.contains(&line), "{shown}");
    assert!(app.key_held(fp), "the paper items are not done");
    the_envelopes(&mut app);
    assert!(!app.key_held(fp));
}

#[test]
fn a_plan_with_the_seed_unticked_everywhere_offers_no_make_the_checklist() {
    let mut app = device(Vec::new(), 2400);
    let (w, fp) = made_here(&mut app);
    app.press(Action::Backup(w));
    // The plan says so first.
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.contains("A key made here signs only once this backup is done."))
    );
    app.press(Action::BPreset(preset(Preset::Paper)));
    app.press(Action::BAnswer(qrow::SEEDS, plan::seeds::WORDS as u8));
    let _ = app.frame();
    assert!(!app.offers(Action::BChecklist));
    let nowhere = format!("Seed {} is kept nowhere", fp_text(fp));
    assert!(app.drawn_texts().contains(&nowhere));
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Plan);
}
