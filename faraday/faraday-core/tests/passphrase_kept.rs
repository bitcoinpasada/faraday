//! A BIP-39 passphrase is never kept in the clear: not in what the
//! device asks the shell to keep, and not in a file queued for the
//! stick (`docs/NEW-WALLET.md` §3.3, the passphrase wording batch).

use faraday_core::create::NewKind;
use faraday_core::keygen::Way;
use faraday_core::plan::Preset;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent, bstep, cstep, testkit};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

const PASSPHRASE: &str = "lantern-orchid-41";

/// What the shell was asked to keep, and what waits for the stick.
type Boxes = Vec<(String, Vec<u8>)>;

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

/// A device on a tall display, with a vault kit on its stick, so a
/// backup's checklist can save a key into it.
fn device() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
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
        inbox: vec![kit_file("vault.ofv")],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    let _ = app.frame();
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Flips enough for twelve words.
fn flip_all(app: &mut Faraday) {
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
}

fn unlock_vault(app: &mut Faraday) {
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
    app.press(Action::Nav(Screen::Backup));
}

/// A single-key wallet made from a Create slot with a known passphrase,
/// its backup plan set to paper and vault, the checklist open and the
/// seed saved into the vault without its passphrase, then the device
/// locked. Returns what the shell was asked to keep and queue for the
/// stick.
fn locked_after_backup() -> (Boxes, Boxes) {
    let mut app = device();
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
    flip_all(&mut app);
    app.press(Action::KNext);
    app.press(Action::KPassField(0));
    type_text(&mut app, PASSPHRASE);
    app.press(Action::KPassField(1));
    type_text(&mut app, PASSPHRASE);
    app.press(Action::KLock);
    assert!(
        app.keygen.as_ref().is_some_and(|k| k.locked),
        "the key did not lock in"
    );
    app.press(Action::KNext);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::BACKUP),
        "a single-key Create opens on Back up"
    );
    let k = Preset::ALL
        .iter()
        .position(|p| *p == Preset::PaperVault)
        .expect("Paper and vault") as u8;
    app.press(Action::CBackup(k));
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BChecklist);
    app.press(Action::BStep(bstep::COPY));
    app.press(Action::BReveal);
    let _ = app.frame();
    unlock_vault(&mut app);
    app.press(Action::BVaultSave(0));
    let _ = app.frame();

    app.press(Action::Lock);
    let mut found = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept, outbox, .. } = c {
            found = Some((kept, outbox));
        }
    }
    found.expect("the device did not ask to keep anything on locking")
}

fn holds(bytes: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && bytes.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn locking_after_a_vault_backup_never_keeps_the_passphrase_in_the_clear() {
    let (kept, outbox) = locked_after_backup();
    let needle = PASSPHRASE.as_bytes();
    for (name, bytes) in &kept {
        assert!(
            !holds(bytes, needle),
            "{name}, kept on locking, holds the passphrase"
        );
    }
    for (name, bytes) in &outbox {
        assert!(
            !holds(bytes, needle),
            "{name}, queued for the stick, holds the passphrase"
        );
    }
}

/// A key loaded with a known passphrase and a one-key wallet over it,
/// planned on paper; another copy of the passphrase added at a new place
/// on the chart, then checked there, typed wrong and then right.
#[test]
fn a_passphrase_checked_by_its_fingerprint_leaves_no_trace_but_the_result() {
    use faraday_core::glance::{Backup, Press};
    use faraday_core::glance_sheet::{ChartAction as C, Target};
    use faraday_core::plan::{At, Edit, What};
    use osk_bip::bip39::{Language, Mnemonic};

    let mut app = device();
    let m = Mnemonic::parse(
        Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[0].0),
    )
    .unwrap();
    app.session
        .add_mnemonic(&m, PASSPHRASE, "Key", None)
        .unwrap();
    let text = NewKind::NativeSegwit
        .key_text(&app.session.keys[0].master)
        .unwrap();
    let w = app
        .session
        .add_wallet("Spending", &format!("wpkh({text}/<0;1>/*)"), "test")
        .unwrap();
    app.press(Action::Backup(w));
    app.press(Action::BPreset(0));
    app.press(Action::BChecklist);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    let press = Press::Loaded(w);
    // Another copy of the passphrase, at a new place: it is shown to be
    // copied, then checked.
    app.press(Action::Chart(C::Open(
        press,
        Target::CopyTo(What::Passphrase(0)),
    )));
    app.press(Action::Chart(C::Edit(
        press,
        Edit::Add(At::Place(2), What::Passphrase(0)),
    )));
    app.press(Action::Chart(C::Confirm(press)));
    let Some((_, Target::PassShow(k, n, j))) = app.chart else {
        panic!("the passphrase is not shown to copy: {:?}", app.chart);
    };
    app.press(Action::Chart(C::Open(press, Target::PassCheck(k, n, j))));
    type_text(&mut app, "not-the-passphrase");
    app.press(Action::Chart(C::PassTry(press)));
    assert_eq!(app.chart_work.result, Some(false));
    type_text(&mut app, PASSPHRASE);
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.chart_work.result, Some(true));
    assert!(
        app.chart_work.secret.is_empty(),
        "the typed passphrase is kept"
    );
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t.starts_with("Matches")), "{texts:?}");
    let g = faraday_core::glance::of(&app, w).unwrap();
    let Backup::Plan { nodes, .. } = &g.backup else {
        panic!("no plan");
    };
    assert!(
        nodes[usize::from(n)].holds[usize::from(j)].starts_with("Passphrase"),
        "{nodes:?}"
    );

    app.press(Action::Lock);
    let mut found = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept, outbox, .. } = c {
            found = Some((kept, outbox));
        }
    }
    let (kept, outbox) = found.expect("the device did not ask to keep anything on locking");
    for (name, bytes) in kept.iter().chain(&outbox) {
        assert!(
            !holds(bytes, PASSPHRASE.as_bytes()),
            "{name} holds the passphrase checked"
        );
    }
}
