//! A secret saved as a plain file, outside any vault, asks for the hold on
//! **Save without encryption?** (F129): from the secret sheet's
//! **Unprotected**, when Back up's plan keeps a seed's file on a stick at
//! a place, and when Transfer's Receive reads one. Into a vault, no hold.

use crate::common::*;
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::plan::What;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

fn display(app: &mut Faraday) {
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
}

/// Test key 1 loaded, with `inbox` in the Inbox.
fn with_key(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    display(&mut app);
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    add_key(&mut app, testkit::TEST_SEEDS[0].0);
    app
}

/// A BIP-85 password on the secret sheet, on its way out.
fn password_on_the_sheet(app: &mut Faraday) {
    app.press(Action::Bip85);
    for a in [Action::PApp(4), Action::PNext, Action::PNext, Action::POut] {
        app.press(a);
    }
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
}

/// Touches `action` where the frame draws it at `t0` ms and lets go at
/// `t1`, a tick between.
fn held(app: &mut Faraday, action: Action, t0: u64, t1: u64) {
    let _ = app.frame();
    let (x, y) = app.where_offered(action).expect("on screen");
    app.event(Event::Tick { now_ms: t0 });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    app.event(Event::Tick { now_ms: t1 });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
}

fn shown(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts()
}

#[test]
fn unprotected_writes_nothing_until_the_hold_is_held_to_its_end() {
    let mut app = with_key(Vec::new());
    password_on_the_sheet(&mut app);
    app.press(Action::SecretUnprotected);
    assert!(app.outbox.is_empty(), "nothing out at the press");
    let t = shown(&mut app);
    for line in [
        "Save without encryption?",
        "Hold to save without encryption",
        "Cancel",
    ] {
        assert!(t.iter().any(|s| s == line), "{line}: {t:?}");
    }
    assert!(t.iter().any(|s| s.contains("as a plain file")), "{t:?}");
    assert!(!app.offers(Action::SecretVault), "no vault is open");
    // Let go early: nothing is written, and the hold is still asked.
    held(&mut app, Action::SecretHold, 1_000, 1_400);
    assert!(app.outbox.is_empty(), "let go early");
    assert!(testkit::plain_asked(&app).is_some());
    // Held to its end: the file waits for the stick, a secret.
    held(&mut app, Action::SecretHold, 2_000, 3_400);
    assert_eq!(app.outbox.len(), 1);
    assert!(app.outbox[0].secret);
    assert_eq!(app.sheet, None);
}

#[test]
fn cancel_on_the_hold_sends_nothing_out() {
    let mut app = with_key(Vec::new());
    password_on_the_sheet(&mut app);
    app.press(Action::SecretUnprotected);
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert!(app.outbox.is_empty());
    assert!(testkit::secret_out(&app).is_none(), "the secret is dropped");
}

/// The test vault unlocked.
fn vault_open(app: &mut Faraday) {
    testkit::vault_speed(app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
}

fn test_vault() -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .unwrap()
}

#[test]
fn a_secret_saved_into_a_vault_asks_no_hold() {
    let mut app = with_key(vec![test_vault()]);
    vault_open(&mut app);
    let notes = |app: &Faraday| {
        let v = &app.vaults().open[0];
        app.vaults()
            .store()
            .contents_for_tests(v.id)
            .of(faraday_vault::records::kind::NOTE)
            .count()
    };
    let before = notes(&app);
    password_on_the_sheet(&mut app);
    app.press(Action::SecretVault);
    assert_eq!(testkit::plain_asked(&app), None, "no hold");
    assert_eq!(app.sheet, None);
    assert_eq!(notes(&app), before + 1, "in the vault");
    assert!(app.outbox.is_empty());
    // On the hold's sheet a vault open is offered instead of Cancel, and
    // takes it there with no hold.
    password_on_the_sheet(&mut app);
    app.press(Action::SecretUnprotected);
    let t = shown(&mut app);
    assert!(t.iter().any(|s| s == "Put it in a vault"), "{t:?}");
    assert!(app.offers(Action::SecretVault));
    app.press(Action::SecretVault);
    assert_eq!(notes(&app), before + 2);
    assert!(app.outbox.is_empty());
}

/// A key made here, its backup open on paper, the words off the paper.
fn backing_up() -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 3 + i / 7) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    app.press(Action::KNext);
    app.press(Action::CBackup);
    testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    no_words(&mut app);
    app
}

/// Seed 1's words file dropped on Place 1's **+ stick**, by hand.
fn drop_seed_file(app: &mut Faraday) {
    app.press(Action::Board(BP::Chip(Chip::List(Thing::Seed(0)))));
    app.press(Action::Board(BP::Zone(Zone::NewStick(0))));
}

fn seed_files(app: &Faraday) -> usize {
    backup_map(app)
        .iter()
        .flat_map(|s| s.holds.iter())
        .filter(|(w, _)| *w == What::SeedFile(0))
        .count()
}

#[test]
fn a_seed_file_placed_on_a_stick_asks_the_hold_once_and_cancel_leaves_it_off() {
    let mut app = backing_up();
    assert_eq!(seed_files(&app), 0);
    drop_seed_file(&mut app);
    let (name, on) = testkit::plain_asked(&app).expect("the hold, at the drop");
    assert!(
        name.starts_with("seed-") && name.ends_with(".txt"),
        "{name}"
    );
    assert_eq!(on, "stick at Place 1");
    // Cancel: the plan as it was, the file on no stick.
    app.press(Action::Cancel);
    assert_eq!(testkit::plain_asked(&app), None);
    assert_eq!(app.sheet, None);
    assert_eq!(seed_files(&app), 0, "the file left the plan");
    // Dropped again and held: on the stick.
    drop_seed_file(&mut app);
    assert!(testkit::plain_asked(&app).is_some(), "asked again");
    app.press(Action::SecretHold);
    assert_eq!(seed_files(&app), 1);
    // Taken off and put back: the same file asks no more.
    let p = files_stick(&app, 0).expect("Place 1's stick of files");
    take_off(&mut app, Zone::Files(0, p), Thing::Seed(0));
    assert_eq!(seed_files(&app), 0);
    drop_seed_file(&mut app);
    assert_eq!(testkit::plain_asked(&app), None, "once per file");
    assert_eq!(seed_files(&app), 1);
}

/// The online app's Transfer, Receive on.
fn receiving() -> Faraday {
    let mut app = Faraday::new();
    app.online = true;
    display(&mut app);
    app.press(Action::Nav(Screen::Transfer));
    app.press(Action::TransferReceive);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    while app.poll_storage().is_some() {}
    app
}

fn saves(app: &mut Faraday) -> Vec<String> {
    let mut out = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveDownload { name, .. } = c {
            out.push(name);
        }
    }
    out
}

#[test]
fn a_received_secret_is_saved_only_past_the_hold() {
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let mut app = receiving();
    app.event(Event::Scanned {
        bytes: words.clone().into_bytes(),
    });
    assert!(saves(&mut app).is_empty(), "not saved before the hold");
    let (name, _) = testkit::plain_asked(&app).expect("the hold");
    let t = shown(&mut app);
    assert!(t.iter().any(|s| s.contains("as a plain file")), "{t:?}");
    app.press(Action::SecretHold);
    assert_eq!(saves(&mut app), vec![name]);
    assert_eq!(
        app.sheet,
        Some(Sheet::Scan),
        "the camera's sheet, for the next"
    );
    // Another, cancelled: not saved.
    let mut app = receiving();
    app.event(Event::Scanned {
        bytes: words.into_bytes(),
    });
    app.press(Action::Cancel);
    assert!(saves(&mut app).is_empty(), "cancelled");
}
