//! The backup comes before spending (`docs/WALLETS.md` §13): a key
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
use faraday_core::testkit::Plain;
use faraday_core::testkit::backup_as;
use faraday_core::testkit::{self, Kit};
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::fp_text;
use faraday_core::{Action, BStage, Faraday, Screen, StorageCommand, StorageEvent, cstep, qstep};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const WARNING: &str = "Back up before you receive";

/// One file of the test stick, made alone: building the whole stick costs
/// every kit's spend and the test vault.
fn kit_file(name: &str) -> (String, Vec<u8>) {
    if name == "vault.kdbx" {
        return (name.to_string(), testkit::test_vault().unwrap());
    }
    let id = name
        .strip_suffix("-wallet.txt")
        .unwrap_or_else(|| panic!("no {name} in the kit"));
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == id)
        .unwrap_or_else(|| panic!("no {name} in the kit"));
    (name.to_string(), testkit::wallet_file(&kit).1)
}

fn device(inbox: Vec<(String, Vec<u8>)>, height: u16) -> Faraday {
    device_with_memory(inbox, height, None)
}

/// As [`device`], the machine saying its free memory when `free_mib` is set.
fn device_with_memory(
    inbox: Vec<(String, Vec<u8>)>,
    height: u16,
    free_mib: Option<u32>,
) -> Faraday {
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
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    if let Some(available_mib) = free_mib {
        app.storage(StorageEvent::Memory { available_mib });
    }
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
    crate::common::ckind(app, at);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 7 + i / 5) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    // Lock in goes back to Create by itself.
    assert!(app.keygen().is_none(), "the key did not lock in");
    assert_eq!(app.screen, Screen::Create);
    let w = app.create().and_then(|c| c.built).expect("built");
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
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
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
    if app.backup().unwrap().open != Some(Item::Envelopes) {
        app.press(Action::BStep(Item::Envelopes));
    }
    assert_eq!(app.backup().unwrap().open, Some(Item::Envelopes));
    app.press(Action::BNext(Item::Envelopes));
}

fn kit_of(app: &Faraday, w: usize) -> Kit {
    Kit {
        id: "made",
        name: "Made here",
        descriptor: app.session.wallets[w]
            .policy()
            .to_descriptor_checksummed()
            .as_str()
            .into(),
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
        .inbox()
        .iter()
        .position(|i| i.name == "spend.psbt")
        .unwrap();
    app.press(Action::StartSpend(at));
    // Every step passed the way a person passes them: the refusal is Sign's.
    for _ in 0..8 {
        match app.spend().and_then(|s| s.open) {
            Some(n) if n != faraday_core::wallet::step::SIGN => {
                app.press(Action::StepNext(n));
            }
            _ => break,
        }
    }
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
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
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    check_the_copy(&mut app, fp);
    assert!(app.key_held(&c, fp), "held before the envelopes");
    the_envelopes(&mut app);
    assert!(!app.key_held(&c, fp), "still held with the checklist done");
    assert_ne!(app.spend_button(&c, w).0, "Finish the backup first");
    app.press(Action::StartSpend(at));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(
        app.spend().unwrap().spend.finished.is_some(),
        "{:?}",
        app.spend().unwrap().error
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
    let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n).unwrap();
    let wallet_at = at(&app, "spending-wallet.txt");
    testkit::load_wallet(&mut app, wallet_at);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys[0].fingerprint;
    assert!(!app.key_held(&c, fp));
    assert_ne!(app.spend_button(&c, 0).0, "Finish the backup first");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.finished.is_some());
}

/// Locks `app` and returns what it queued for the stick.
fn locked(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    app.press(Action::Lock);
    let mut out = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = c {
            out = Some(faraday_core::testkit::plain_files(outbox));
        }
    }
    out.expect("nothing queued on locking")
}

fn unlock(app: &mut Faraday) {
    faraday_core::testkit::vault_speed(app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the test vault did not unlock");
}

#[test]
fn after_a_lock_and_an_unlock_from_the_vault_the_held_key_is_still_held() {
    held_key_after_a_lock_and_an_unlock(None);
}

/// The same flow on a machine that says its free memory: the KeePass-format
/// vault is sealed and opened again at the strong cost.
#[test]
fn after_a_lock_and_an_unlock_from_the_vault_the_held_key_is_still_held_at_strong_cost() {
    held_key_after_a_lock_and_an_unlock(Some(15_000));
}

fn held_key_after_a_lock_and_an_unlock(free_mib: Option<u32>) {
    let c = testkit::compute();
    let mut app = device_with_memory(vec![kit_file("vault.kdbx")], 768, free_mib);
    let (_, fp) = made_here(&mut app);
    checklist(&mut app, Preset::PaperVault);
    check_the_copy(&mut app, fp);
    // The copy checked, the checklist has moved on to the vault's item.
    if app.backup().unwrap().open != Some(Item::Vault(0)) {
        app.press(Action::BStep(Item::Vault(0)));
    }
    assert_eq!(app.backup().unwrap().open, Some(Item::Vault(0)));
    unlock(&mut app);
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    assert!(app.key_held(&c, fp), "the checklist is not done");
    let vault: Vec<_> = locked(&mut app)
        .into_iter()
        .filter(|(n, _)| n == "vault.kdbx")
        .collect();
    assert_eq!(vault.len(), 1, "the vault was not sealed");
    let mut next = device_with_memory(vault, 768, free_mib);
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
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
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
        .map(|&(change, i)| {
            app.session
                .address_shown(&c, wl, change, i)
                .as_str()
                .to_string()
        })
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
    // Create's address check: Lock in opened it.
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::CHECK));
    let _ = app.frame();
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
            (faraday_text::Text::of(WARNING), true)
        );
    }
    app.press(Action::Backup(w));
    backup_as(&mut app, Preset::Paper);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
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
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    check_the_copy(&mut app, fp);
    // The checklist is on show with its last item waiting, drawn.
    let _ = app.drawn_texts();
    assert!(app.receive_held(&c, &app.session.wallets[w]));
    app.press(Action::BStep(Item::Envelopes));
    app.press(Action::BNext(Item::Envelopes));
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
    testkit::load_wallet(&mut app, 0);
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
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
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
    assert!(app.offers(Action::QrWallet(0)), "Show wallet QR");
    assert!(app.offers(Action::OpenWallet(0)), "Open the wallet");
    assert!(app.offers(Action::LockAsk), "Lock");
    assert!(app.offers(Action::PowerAsk), "Power off");
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.starts_with("Single key · ")),
        "the card says what the wallet is"
    );
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
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    assert_eq!(app.backup().unwrap().open, Some(Item::Vault(0)));
    let _ = app.frame();
    let make = Action::Vault(V::CreateFrom(Screen::Backup));
    assert!(app.offers(make), "Make a vault");
    assert!(!app.offers(Action::BVaultSave(0)));
    assert!(!app.offers(Action::BNext(Item::Vault(0))), "a Continue");
    // The next item does not open, nor does Continue pass it.
    app.press(Action::BStep(Item::Sheets));
    app.press(Action::BNext(Item::Vault(0)));
    assert_eq!(app.backup().unwrap().open, Some(Item::Vault(0)));
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
        if !app.vaults().open.is_empty() && app.screen == Screen::Backup {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.screen, Screen::Backup);
    let _ = app.frame();
    assert!(app.offers(Action::BVaultSave(0)), "Save into the vault");
    assert!(!app.offers(make));
    assert!(!app.offers(Action::BNext(Item::Vault(0))));
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    assert!(app.key_held(&c, fp), "the rest of the checklist waits");
    // Saved into, the item is done and moves on by itself
    // (`docs/BACKUP-FLOW.md` §17.23).
    let _ = app.frame();
    assert!(!app.offers(Action::BVaultSave(0)));
    assert_ne!(app.backup().unwrap().open, Some(Item::Vault(0)));
}

#[test]
fn changing_to_paper_only_after_the_vault_keeps_its_copy_on_the_map_and_the_hold() {
    let c = testkit::compute();
    let mut app = device(vec![kit_file("vault.kdbx")], 2400);
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
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    assert!(!app.backup_items_now(&c).contains(&Item::Vault(0)));
    let line = format!("vault.kdbx holds seed {}", String::from(&*fp_text(fp)));
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
    let nowhere = format!("Seed {} is kept nowhere", String::from(&*fp_text(fp)));
    assert!(app.drawn_texts().plain().contains(&nowhere));
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    assert_eq!(app.backup().unwrap().stage, BStage::Plan);
}

/// While a key made here waits for its backup, no receive address opens a
/// code: not on Create's Check, not on the wallet's card, and a press for
/// one shows none (`docs/QR.md` §7, F31).
#[test]
fn a_key_made_here_opens_no_receive_code_until_its_backup_is_done() {
    use faraday_core::Code;
    let mut app = device(Vec::new(), 2400);
    let (w, _) = made_here(&mut app);
    for card in [false, true] {
        if card {
            app.press(Action::OpenWallet(w));
        }
        for i in 0..2 {
            let a = Action::ShowCode(Code::Receive(w, i));
            assert!(!crate::common::offered(&mut app, a), "{a:?}, card {card}");
        }
    }
    app.press(Action::ShowCode(Code::Receive(w, 0)));
    assert!(app.qr().is_none(), "a held address showed a code");
}

/// A wallet made from a new key with no backup yet hands its descriptor
/// (and so its addresses) to the watch-only software with a warning, and
/// Explore of that key opens no address code; after the backup both show as
/// they do for any key.
#[test]
fn a_key_made_here_warns_on_its_descriptor_and_explore_opens_no_code_until_its_backup_is_done() {
    use faraday_core::Code;
    let c = testkit::compute();
    let mut app = device(Vec::new(), 2400);
    let (w, fp) = made_here(&mut app);
    let warned = |app: &Faraday| {
        let v = app
            .code_view(&c, Code::Descriptor(w))
            .expect("a descriptor");
        v.label.iter().any(|l| l == WARNING)
    };
    assert!(warned(&app), "the descriptor's code carries no warning");
    app.press(Action::Explore);
    for code in [
        Code::ExploreAddress,
        Code::ExplorePublicKey,
        Code::ExploreXpub,
    ] {
        assert!(app.code_view(&c, code).is_err(), "{code:?} opened");
    }
    app.press(Action::Back);
    app.press(Action::Backup(w));
    backup_as(&mut app, Preset::Paper);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    check_the_copy(&mut app, fp);
    the_envelopes(&mut app);
    assert!(!warned(&app), "the warning stays after the backup");
    app.press(Action::Explore);
    assert!(app.code_view(&c, Code::ExploreAddress).is_ok());
}

/// A key kept nowhere else makes Power off a hold.
#[test]
fn power_off_holds_for_a_key_kept_nowhere() {
    let mut app = device(Vec::new(), 2400);
    let _ = made_here(&mut app);
    app.press(Action::PowerAsk);
    let _ = app.frame();
    assert!(app.offers(Action::HoldPowerOff) && !app.offers(Action::PowerOff));
}

/// A key loaded from SLIP-39 shares or a master seed has no words to
/// write: a "Paper and vault" plan puts it in the vault alone, the plan
/// counts it there, the vault saves it, and a later device restores the
/// same key from that vault.
#[test]
fn a_key_loaded_without_words_goes_into_the_plans_vault_and_the_vault_restores_it() {
    use faraday_core::plan::{At, What, check, map};
    use std::str::FromStr;
    let c = testkit::compute();
    let mut app = device(vec![kit_file("vault.kdbx")], 2400);
    let fp = app.session.keys.add_seed(&c, &[7u8; 32], "").unwrap();
    let path = faraday_bip::bitcoin::bip32::DerivationPath::from_str("m/84'/1'/0'").unwrap();
    let pos = app.session.keys.position(fp).unwrap();
    let x = app.session.keys.xpub_at(&c, pos, &path).unwrap();
    let hex = String::from_utf8_lossy(&fp.to_hex()).into_owned();
    let d = format!("wpkh([{hex}/84'/1'/0']{x}/<0;1>/*)");
    let w = app
        .session
        .add_wallet(
            &c,
            "Test",
            &d,
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .expect("reads");
    app.press(Action::Backup(w));
    backup_as(&mut app, Preset::PaperVault);
    let (shape, a) = app.board_plan().expect("a board");
    let m = map(&c, &shape, &a);
    let vault = m.iter().find(|s| s.at == At::Vault(0)).expect("a vault");
    assert!(vault.holds.iter().any(|(h, _)| *h == What::Seed(0)));
    assert!(
        !m.iter()
            .flat_map(|s| s.holds.iter())
            .any(|(h, _)| matches!(h, What::Words(_) | What::SeedFile(_))),
        "no words to write"
    );
    assert!(m.iter().all(|s| s.at != At::Away));
    assert_ne!(
        check(&c, &shape, &a).lost,
        faraday_core::plan::Lost::Yes,
        "the one place holds the vault with the only copy"
    );
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    unlock(&mut app);
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    let vault: Vec<_> = locked(&mut app)
        .into_iter()
        .filter(|(n, _)| n == "vault.kdbx")
        .collect();
    assert_eq!(vault.len(), 1, "the vault was not sealed");
    let mut next = device(vault, 768);
    unlock(&mut next);
    next.press(Action::Vault(V::LoadChosen(0)));
    assert!(
        next.session.keys.all().any(|k| k.fingerprint == fp),
        "the vault did not restore the key"
    );
}

/// A multisig whose steps include Signers: both keys are held, so Signers
/// passes by itself, and a key made here and not backed up still keeps Sign
/// from signing.
#[test]
fn signers_passed_by_itself_does_not_pass_the_backup_refusal() {
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .unwrap();
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut app = device(
        vec![
            ("spend.psbt".to_string(), unsigned.to_bytes()),
            testkit::wallet_file(&k),
        ],
        768,
    );
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    let wallet_at = app
        .inbox()
        .iter()
        .position(|i| i.name.ends_with("-wallet.txt"))
        .unwrap();
    testkit::load_wallet(&mut app, wallet_at);
    for s in ["zebra", "bacon"] {
        crate::common::add_key(&mut app, s);
    }
    let fp = app.session.keys.last().unwrap().fingerprint;
    testkit::mark_made_here(&mut app, fp);
    let at = app
        .inbox()
        .iter()
        .position(|i| i.name == "spend.psbt")
        .unwrap();
    app.press(Action::StartSpend(at));
    for _ in 0..8 {
        match app.spend().and_then(|s| s.open) {
            Some(n) if n != faraday_core::wallet::step::SIGN => {
                app.press(Action::StepNext(n));
            }
            _ => break,
        }
    }
    let s = app.spend().unwrap();
    assert_eq!(s.open, Some(faraday_core::wallet::step::SIGN));
    assert!(s.done[faraday_core::wallet::step::SIGNERS as usize]);
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert!(s.spend.signed_here.is_empty(), "a held key signed");
    assert!(
        s.error.as_deref().is_some_and(|e| e.contains("backup")),
        "{:?}",
        s.error
    );
}
