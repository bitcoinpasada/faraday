//! The backup comes before spending (`docs/history/NEW-WALLET.md` §14): a key
//! made here signs nothing until its wallet's backup checklist is
//! complete, while a key restored from words signs at once; the hold
//! survives a lock, in the vault beside the seed; the wallet's address is
//! shown with a warning until then; a plan that keeps no copy of the key
//! is not made into a checklist; a plan changed part way keeps what was
//! done on the map and the hold until its own checklist is done; and the
//! done card leads with the three checks, above the chart.

use crate::common::{no_paper_description, no_words, type_text};
use faraday_bip::keys::Fingerprint;
use faraday_core::create::NewKind;
use faraday_core::keygen::Way;
use faraday_core::plan::{CHECK_LINES, Item, Preset};
use faraday_core::testkit::backup_as;
use faraday_core::testkit::{self, Kit};
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::fp_text;
use faraday_core::{
    Action, BStage, Faraday, Screen, StorageCommand, StorageEvent, bstep, cstep, qstep,
};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

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
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    let _ = app.frame();
    app
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
    let fp = app.session.keys.last().unwrap().fingerprint;
    (w, fp)
}

/// The backup of wallet `w` from Create made into the plan preset `p`
/// builds, by presses, and its checklist made.
fn checklist(app: &mut Faraday, p: Preset) {
    let c = faraday_core::testkit::compute();
    app.press(Action::CBackup);
    backup_as(app, p);
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
}

/// The copy by hand of the key's seed, checked by its words typed back.
fn check_the_copy(app: &mut Faraday, fp: Fingerprint) {
    let c = testkit::compute();
    let key = app.session.keys.position(fp).unwrap();
    let words = app
        .session
        .keys
        .reveal(key, faraday_keys::Reveal::Words)
        .to_string();
    app.press(Action::BCheck);
    testkit::type_words(app, &words);
    assert!(
        app.backup_item_done_now(&c, Item::Copy(0)),
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
        descriptor: app.session.wallets[w].policy().to_descriptor_checksummed(),
        cosigner: None,
        fee: 1_000,
    }
}

#[test]
fn a_key_from_new_key_cannot_sign_until_its_wallets_checklist_is_complete_and_can_after() {
    let c = testkit::compute();
    // The key's wallet, made once to build a spend of it.
    let mut first = device(Vec::new(), 768);
    let (w, fp) = made_here(&mut first);
    let psbt = testkit::unsigned(&kit_of(&first, w)).unwrap();
    // The same flips on a device with that spend in its Inbox.
    let mut app = device(vec![("spend.psbt".to_string(), psbt.to_bytes())], 768);
    let (w, again) = made_here(&mut app);
    assert_eq!(again, fp, "the same flips made another key");
    assert!(app.key_held(&c, fp));
    assert_eq!(app.spend_button(&c, w).0, "Finish the backup first");
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
    backup_as(&mut app, Preset::Paper);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    check_the_copy(&mut app, fp);
    assert!(app.key_held(&c, fp), "held before the envelopes");
    the_envelopes(&mut app);
    assert!(!app.key_held(&c, fp), "still held with the checklist done");
    assert_ne!(app.spend_button(&c, w).0, "Finish the backup first");
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
    let c = testkit::compute();
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
    let fp = app.session.keys[0].fingerprint;
    assert!(!app.key_held(&c, fp));
    assert_ne!(app.spend_button(&c, 0).0, "Finish the backup first");
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
    let c = testkit::compute();
    let mut app = device(vec![kit_file("vault.ofv")], 768);
    let (_, fp) = made_here(&mut app);
    checklist(&mut app, Preset::PaperVault);
    check_the_copy(&mut app, fp);
    app.press(Action::BStep(bstep::VAULT));
    assert_eq!(app.backup.as_ref().unwrap().open, Some(bstep::VAULT));
    unlock(&mut app);
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    assert!(app.key_held(&c, fp), "the checklist is not done");
    let vault: Vec<_> = locked(&mut app)
        .into_iter()
        .filter(|(n, _)| n == "vault.ofv")
        .collect();
    assert_eq!(vault.len(), 1, "the vault was not sealed");
    let mut next = device(vault, 768);
    unlock(&mut next);
    next.press(Action::Vault(V::LoadChosen(0)));
    assert!(
        next.session.keys.all().any(|k| k.fingerprint == fp),
        "the key did not load"
    );
    assert!(next.key_held(&c, fp), "the key signs after a lock");
}

#[test]
fn the_cards_address_carries_the_warning_until_the_backup_is_done() {
    let c = faraday_core::testkit::compute();
    let mut app = device(Vec::new(), 2400);
    let (w, fp) = made_here(&mut app);
    app.press(Action::OpenWallet(w));
    assert!(app.drawn_texts().iter().any(|t| t == WARNING));
    app.press(Action::Backup(w));
    backup_as(&mut app, Preset::Paper);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    check_the_copy(&mut app, fp);
    the_envelopes(&mut app);
    app.press(Action::OpenWallet(w));
    assert!(!app.drawn_texts().iter().any(|t| t == WARNING));
}

/// Whether the frame draws any of the wallet's first receive or change
/// addresses, grouped or not.
fn draws_an_address(app: &mut Faraday, w: usize) -> bool {
    let c = faraday_core::testkit::compute();
    let wl = &app.session.wallets[w];
    let addrs: Vec<String> = [(false, 0), (false, 1), (true, 0)]
        .iter()
        .map(|&(change, i)| app.session.address_shown(&c, wl, change, i))
        .collect();
    app.drawn_texts().iter().any(|t| {
        let t = t.replace(' ', "");
        addrs.iter().any(|a| t.contains(&a[..a.len().min(20)]))
    })
}

#[test]
fn a_key_made_here_shows_no_receive_address_until_its_backup_is_done() {
    let c = testkit::compute();
    let mut app = device(Vec::new(), 2400);
    let (w, fp) = made_here(&mut app);
    // Create's address check.
    app.press(Action::CStep(cstep::CHECK));
    assert!(!draws_an_address(&mut app, w), "Create's check");
    assert!(app.drawn_texts().iter().any(|t| t == WARNING));
    // The wallet's card.
    app.press(Action::OpenWallet(w));
    assert!(!draws_an_address(&mut app, w), "the card");
    // Every address line and the chart's move target.
    let wl = &app.session.wallets[w];
    for (change, i) in [(false, 0), (false, 7), (true, 3)] {
        assert_eq!(
            app.receive_line(&c, wl, change, i),
            (WARNING.to_string(), true)
        );
    }
    app.press(Action::Backup(w));
    backup_as(&mut app, Preset::Paper);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    check_the_copy(&mut app, fp);
    the_envelopes(&mut app);
    app.press(Action::OpenWallet(w));
    assert!(draws_an_address(&mut app, w), "after the backup");
    let wl = &app.session.wallets[w];
    assert!(!app.receive_line(&c, wl, false, 0).1);
}

#[test]
fn the_press_that_completes_the_last_item_shows_the_receive_address_on_the_next_frame() {
    let c = testkit::compute();
    let mut app = device(Vec::new(), 2400);
    let (w, fp) = made_here(&mut app);
    app.press(Action::Backup(w));
    backup_as(&mut app, Preset::Paper);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    check_the_copy(&mut app, fp);
    // The checklist is on show with its last item waiting, drawn.
    let _ = app.drawn_texts();
    assert!(app.receive_held(&c, &app.session.wallets[w]));
    app.press(Action::BStep(bstep::ENVELOPE));
    app.press(Action::BNext(bstep::ENVELOPE));
    // No frame has been drawn since the press: the key is already
    // released, and the address is not held back.
    assert!(!app.key_held(&c, fp));
    assert!(!app.receive_held(&c, &app.session.wallets[w]));
    app.press(Action::OpenWallet(w));
    assert!(!app.drawn_texts().iter().any(|t| t == WARNING));
    assert!(draws_an_address(&mut app, w));
}

#[test]
fn a_restored_key_shows_its_addresses_at_once() {
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = device(
        vec![("spending-wallet.txt".to_string(), k.descriptor.into_bytes())],
        2400,
    );
    app.press(Action::LoadWallet(0));
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    app.press(Action::OpenWallet(0));
    assert!(draws_an_address(&mut app, 0));
}

#[test]
fn with_every_item_done_the_done_card_shows_the_three_checks_above_the_chart() {
    let c = faraday_core::testkit::compute();
    let mut app = device(Vec::new(), 2400);
    let (_, fp) = made_here(&mut app);
    // Paper alone, with no sheet: nothing is written to a stick, so the
    // card is the end.
    app.press(Action::CBackup);
    backup_as(&mut app, Preset::Paper);
    no_paper_description(&mut app);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
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
    assert!(
        !app.offers(Action::WriteAsk),
        "nothing waits For the stick after the checklist"
    );
    assert!(app.offers(Action::BPlan));
}

#[test]
fn a_vault_item_offers_its_one_action_at_a_time_and_holds_the_items_after_it() {
    let c = testkit::compute();
    let mut app = device(Vec::new(), 2400);
    let (_, fp) = made_here(&mut app);
    app.press(Action::CBackup);
    backup_as(&mut app, Preset::PaperVault);
    // No paper: the vault's item comes first.
    no_words(&mut app);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
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
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    assert!(app.key_held(&c, fp), "the rest of the checklist waits");
    let _ = app.frame();
    assert!(app.offers(Action::BNext(bstep::VAULT)), "Continue");
    assert!(!app.offers(Action::BVaultSave(0)));
    app.press(Action::BNext(bstep::VAULT));
    assert_ne!(app.backup.as_ref().unwrap().open, Some(bstep::VAULT));
}

#[test]
fn changing_to_paper_only_after_the_vault_keeps_its_copy_on_the_map_and_the_hold() {
    let c = testkit::compute();
    let mut app = device(vec![kit_file("vault.ofv")], 2400);
    let (_, fp) = made_here(&mut app);
    checklist(&mut app, Preset::PaperVault);
    check_the_copy(&mut app, fp);
    unlock(&mut app);
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    // Paper only, part way: the vault taken off on the board, by its ×
    // and Take off, as a person does it (§19.21).
    app.press(Action::BPlan);
    use faraday_core::board::BoardPress;
    app.press(Action::Board(BoardPress::RemoveVault(0)));
    app.press(Action::Board(BoardPress::RemoveVaultSure(0)));
    let (shape, a) = app.board_plan().expect("Change the plan opens the board");
    assert!(
        !faraday_core::plan::map(&c, &shape, &a)
            .iter()
            .any(|s| matches!(s.at, faraday_core::plan::At::Vault(_))),
        "no vault on the plan"
    );
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    assert!(!app.backup_items_now(&c).contains(&bstep::VAULT));
    let line = format!("vault.ofv holds seed {}", fp_text(fp));
    let shown = app.drawn_texts().join(" ");
    assert!(shown.contains(&line), "{shown}");
    assert!(app.key_held(&c, fp), "the paper items are not done");
    the_envelopes(&mut app);
    assert!(!app.key_held(&c, fp));
}

#[test]
fn a_plan_with_the_seed_unticked_everywhere_offers_no_make_the_checklist() {
    let c = faraday_core::testkit::compute();
    let mut app = device(Vec::new(), 2400);
    let (w, fp) = made_here(&mut app);
    app.press(Action::Backup(w));
    // The plan says so first.
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.contains("A key made here signs only once this backup is done."))
    );
    backup_as(&mut app, Preset::Paper);
    no_words(&mut app);
    app.press(Action::BQ(qstep::PLAN));
    let _ = app.frame();
    assert!(!app.offers(Action::BChecklist));
    let nowhere = format!("Seed {} is kept nowhere", fp_text(fp));
    assert!(app.drawn_texts().contains(&nowhere));
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Plan);
}

/// The backup's last card, in Guided, says extra copies are made outside
/// the plan and links the Learn page on them; Steps only shows neither
/// (`docs/BACKUP-FLOW.md` §17.19 item 7).
#[test]
fn the_done_card_says_extra_copies_are_made_outside_the_plan_in_guided_only() {
    let c = faraday_core::testkit::compute();
    let mut app = device(Vec::new(), 2400);
    let (_, fp) = made_here(&mut app);
    app.press(Action::CBackup);
    backup_as(&mut app, Preset::Paper);
    no_paper_description(&mut app);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    check_the_copy(&mut app, fp);
    the_envelopes(&mut app);
    app.press(Action::Guided(true));
    let _ = app.frame();
    let t = app.drawn_texts().join(" ");
    assert!(
        t.contains("More copies can be made once the wallet is made"),
        "{t}"
    );
    assert!(app.offers(Action::LearnExtraCopies));
    app.press(Action::LearnExtraCopies);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Learn));
    assert_eq!(app.learn().pages[app.learn().page].title, "Extra copies");
    app.press(Action::Cancel);
    app.press(Action::Guided(false));
    let _ = app.frame();
    assert!(!app.offers(Action::LearnExtraCopies), "not in Steps only");
}
