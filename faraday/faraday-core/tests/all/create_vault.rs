//! Pausing a new wallet's backup to make a vault for its keys: the
//! backup checklist's vault item, reached from Create's Back up card,
//! offers Make a vault, which opens the vault wizard; finishing it lands
//! on Unlock with the new vault picked and a way back; unlocking it, or
//! going back, returns to the backup still under way.

use crate::common::type_text;
use faraday_core::keygen::Way;
use faraday_core::testkit::Plain;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

fn settle(app: &mut Faraday, from: u64) {
    for t in from..from + 20 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// A single-key wallet, made and added to the session from the Create
/// flow's own New key, its backup opened on Paper and vault, stopping on
/// the checklist's vault item.
fn create_to_vault_step() -> Faraday {
    let c = faraday_core::testkit::compute();
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
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x20 + i; 32])));
    }
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KStart(
        faraday_core::keygen::Start::Make(faraday_core::forms::Form::Words).index(),
    ));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip(i % 3 != 1));
    }
    app.press(Action::KNext); // entries -> key
    app.press(Action::KLock); // adds the key and makes the wallet
    app.press(Action::KNext); // back to Create, on Back up
    assert_eq!(app.screen, Screen::Create, "New key returns to Create");
    assert!(
        app.create().unwrap().built.is_some(),
        "the wallet did not build"
    );
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    assert_eq!(app.screen, Screen::Backup);
    // No paper words: the vault's item comes first and opens.
    crate::common::no_words(&mut app);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::plan::Item::Templates)
    {
        app.press(Action::BNext(faraday_core::plan::Item::Templates));
    }
    assert_eq!(
        app.backup().unwrap().open,
        Some(faraday_core::plan::Item::Vault(0))
    );
    let _ = app.frame();
    assert!(
        app.offers(Action::Vault(V::CreateFrom(Screen::Backup))),
        "the vault item offers Make a vault"
    );
    app
}

fn make_a_vault(app: &mut Faraday) {
    app.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    assert_eq!(app.screen, Screen::CreateVault);
    // The size has its default: the flow opens on the passphrases.
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CGo));
    settle(app, 30);
    assert!(
        app.vaults().open.is_empty(),
        "the vault was made and is locked"
    );
    assert_eq!(app.screen, Screen::Unlock, "its Unlock opens");
}

#[test]
fn a_vault_made_from_the_backup_is_locked_and_unlocked_once_before_it_returns_to_it() {
    let mut app = create_to_vault_step();
    make_a_vault(&mut app);
    assert!(
        app.vaults().unlock_text().is_empty(),
        "no passphrase is typed for it"
    );
    type_text(&mut app, "wrong phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 30);
    assert!(
        app.vaults().open.is_empty(),
        "a wrong passphrase is refused"
    );
    assert_eq!(app.screen, Screen::Unlock);
    assert!(crate::common::unlock_error(&app).is_some(), "and says so");
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 30);
    assert_eq!(app.vaults().open.len(), 1, "the right one opens it");
    assert_eq!(
        app.screen,
        Screen::Backup,
        "the vault just unlocked returns to the backup still under way"
    );
}

#[test]
fn a_new_vaults_passphrases_wait_until_the_stick_is_pulled() {
    let mut app = create_to_vault_step();
    app.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    app.storage(faraday_core::StorageEvent::Sticks(vec![
        faraday_core::StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("a"),
            label: faraday_text::Text::of("STICK"),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        },
    ]));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "typed with a stick in");
    app.press(Action::Vault(V::Dice(0)));
    type_text(&mut app, "12345");
    let c = app.vaults().create.as_ref().unwrap();
    assert!(
        (0..c.phrase_count())
            .all(|i| faraday_core::vault::Vaults::phrase_text(c, i) == Some(("", ""))),
        "nothing secret is typed while a stick is attached"
    );
    assert!(
        app.vaults().dice.open().is_none(),
        "no dice are rolled with a stick in"
    );

    app.storage(faraday_core::StorageEvent::Sticks(Vec::new()));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    assert_eq!(
        faraday_core::vault::Vaults::phrase_text(app.vaults().create.as_ref().unwrap(), 0)
            .unwrap()
            .0,
        "test phrase"
    );
}

#[test]
fn a_finished_backup_offers_a_stick_and_asks_to_lock_first() {
    let c = faraday_core::testkit::compute();
    let mut app = create_to_vault_step();
    let wallet = app.create().unwrap().built.unwrap();
    app.press(Action::Backup(wallet));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::plan::Item::Templates)
    {
        app.press(Action::BNext(faraday_core::plan::Item::Templates));
    }
    app.press(Action::BOut(0));
    app.press(Action::BOut(3));
    let pdfs = app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".pdf"))
        .count();
    assert_eq!(
        pdfs, 1,
        "the backup sheet goes out; no template is made here"
    );

    // A seed is held, so the stick comes after a lock.
    app.press(Action::WriteAsk);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WriteOut));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 1, "Not now keeps the session");
}

// ---------------------------------------------------------------------
// §3.1: four sizes, then Customise.
// ---------------------------------------------------------------------

/// The app with nothing loaded, memory to spare and the system's
/// randomness in, on Create a vault from Vaults.
fn on_create_vault() -> Faraday {
    on_create_vault_free(Some(15_000))
}

/// Create a vault opened on a computer with `free` MiB free, or one that
/// does not say.
fn on_create_vault_free(free: Option<u32>) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 1000,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    if let Some(available_mib) = free {
        app.storage(faraday_core::StorageEvent::Memory { available_mib });
    }
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    app
}

/// The memory of the size the form stands on, MiB.
fn form_memory(app: &Faraday) -> u32 {
    app.vaults().form_cost().unwrap().1
}

/// Create starts on Light, 64 MiB, which opens everywhere: on a computer
/// with plenty free, on one with 400 MiB free, on one that does not say,
/// and when opened from the Backup page after another row was chosen.
#[test]
fn create_starts_on_64_mib_whatever_this_computer_has_free() {
    for free in [Some(2_000), Some(400), Some(300), None] {
        let app = on_create_vault_free(free);
        let (k, memory, passes) = app.vaults().form_cost().unwrap();
        assert_eq!((memory, passes), (64, 3), "{free:?}");
        assert_eq!(faraday_core::vault::PRESETS[k].0, "Light");
        assert_eq!(app.vaults().size_row(), Some(0), "the default row");
    }
    let mut app = on_create_vault_free(Some(2_000));
    app.press(Action::Vault(V::CSizeRow(3)));
    app.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    assert_eq!(form_memory(&app), 64, "a new form starts on 64 MiB again");
}

/// A size this computer cannot take can be picked, but the vault is not
/// made: Create says to choose a cost it can allocate.
#[test]
fn a_size_this_computer_cannot_take_is_chosen_but_not_made() {
    let mut app = on_create_vault_free(Some(300));
    app.press(Action::Vault(V::CSizeRow(2)));
    assert_eq!(form_memory(&app), 256);
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::CGo));
    settle(&mut app, 1);
    assert!(app.vaults().just_made.is_none());
    let e = app.vaults().create.as_ref().unwrap().error.clone();
    assert_eq!(
        e.unwrap().as_str(),
        "Choose a cost this computer can allocate"
    );
}

/// The Pi Zero 2 W row is 128 MiB at 3 passes, written as KDBX memory
/// 134,217,728, and the vault opens.
#[test]
fn the_pi_zero_row_makes_a_128_mib_vault_that_opens() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "PCs and a Pi Zero 2 W"),
        "{:?}",
        app.drawn_texts()
    );
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "PCs and a Raspberry Pi 3")
    );
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "PCs and every Raspberry Pi")
    );
    app.press(Action::Vault(V::CSizeRow(1)));
    assert_eq!(app.vaults().form_cost().map(|c| (c.1, c.2)), Some((128, 3)));
    let (cost, file) = create_and_read(&mut app);
    assert_eq!(cost, (128, 3));
    // KDBX stores the memory in bytes: the file holds 134,217,728 as its
    // little-endian u64.
    let m = 134_217_728u64.to_le_bytes();
    assert!(file.windows(8).any(|w| w == m), "KDF M in the header");
}

/// Types one passphrase twice, presses Create vault and waits: the cost
/// of the vault made, MiB and passes.
fn create_and_read_cost(app: &mut Faraday) -> (u32, u32) {
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CGo));
    settle(app, 30);
    assert!(app.vaults().open.is_empty(), "the vault is made locked");
    crate::common::unlock(app, "test phrase");
    assert_eq!(app.vaults().open.len(), 1, "and opens with its passphrase");
    let made = app.vaults().open[0].name.clone();
    let f = app
        .vault_files()
        .into_iter()
        .find(|f| f.name == made)
        .unwrap();
    let h = &f.facts.header;
    (h.cost.memory_kib / 1024, h.cost.passes)
}

#[test]
fn a_fresh_create_a_vault_opens_on_name_and_passphrases_with_64_mib_chosen() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    let c = app.vaults().create.as_ref().unwrap();
    assert_eq!(c.open, Some(vstep::PHRASES), "it opens on the passphrases");
    assert!(
        app.offers(Action::Vault(V::CStep(vstep::PRESET))),
        "the size is a card of its own, closed"
    );
    assert!(
        !app.offers(Action::Vault(V::CStep(vstep::COST))),
        "the unlock cost is not a card until Customise"
    );
    assert_eq!(create_and_read_cost(&mut app), (64, 3));
}

/// A new vault goes by the name typed for it.
#[test]
fn a_new_vault_goes_by_the_vaults_name() {
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CName));
    type_text(&mut app, "Savings");
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "first phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, "first phrase");
    app.press(Action::Vault(V::CGo));
    settle(&mut app, 1);
    settle(&mut app, 30);
    crate::common::unlock(&mut app, "first phrase");
    assert_eq!(app.vaults().open.len(), 1, "it opened with its passphrase");
    assert_eq!(app.vaults().open[0].label(app.vaults().store()), "Savings");
}

#[test]
fn pcs_only_makes_a_512_mib_vault() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::CSizeRow(3))), "PCs only");
    assert!(
        app.offers(Action::Vault(V::CSizeRow(0))),
        "PCs and every Raspberry Pi"
    );
    app.press(Action::Vault(V::CSizeRow(3)));
    app.press(Action::Vault(V::CNext(vstep::PRESET)));
    assert_eq!(
        app.vaults().create.as_ref().unwrap().open,
        Some(vstep::PHRASES),
        "Continue goes on to the passphrases"
    );
    assert_eq!(create_and_read_cost(&mut app), (512, 3));
}

#[test]
fn customise_opens_the_two_cards_and_a_custom_memory_survives_continue() {
    use faraday_core::vault::{CUSTOM_MEMORY, vstep};
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::CCustomise)));
    app.press(Action::Vault(V::CCustomise));
    let _ = app.frame();
    // Customise opens the first of them; the other is a row of Your choices.
    assert_eq!(
        app.vaults().create.as_ref().unwrap().open,
        Some(vstep::WHERE)
    );
    assert!(
        !app.offers(Action::Vault(V::CStep(vstep::COST))),
        "the unlock cost, not reached yet, takes no press"
    );
    assert!(
        !app.offers(Action::Vault(V::CStep(vstep::PRESET))),
        "in place of the size"
    );
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    let k = CUSTOM_MEMORY.iter().position(|m| *m == 128).unwrap();
    app.press(Action::Vault(V::CMemory(k)));
    app.press(Action::Vault(V::CNext(vstep::COST)));
    assert_eq!(create_and_read_cost(&mut app).0, 128);
}

/// The cost step tags no row as suggested, even with plenty free, and a
/// row reads its memory, passes and bits in that order.
#[test]
fn the_cost_step_tags_no_row_suggested_and_reads_memory_passes_bits() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    app.press(Action::Vault(V::CCustomise));
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    let _ = app.frame();
    let text: Vec<String> = app.drawn_texts().iter().map(|t| t.to_string()).collect();
    assert!(
        text.iter().any(|t| t == "Light") && text.iter().any(|t| t == "Fits"),
        "on the cost step: {text:?}"
    );
    assert!(!text.iter().any(|t| t == "Suggested"), "{text:?}");
    assert!(
        text.iter()
            .any(|t| t.starts_with("64 MiB \u{b7} 3 passes \u{b7} \u{2248}")),
        "{text:?}"
    );
}

/// The vault's memory is one figure: the machines choice, the cost and
/// the summary all say 64 MiB for PCs and every Raspberry Pi, never the app's
/// share on top of it.
#[test]
fn the_unlock_memory_reads_the_same_on_the_machines_and_the_summary() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let text = app.drawn_texts();
    assert!(
        !text.iter().any(|t| t == "Unlock memory"),
        "the default's figures stay behind Customize: {text:?}"
    );
    app.press(Action::Vault(V::CCustomise));
    let text = app.drawn_texts();
    assert!(text.iter().any(|t| t == "Unlock memory"), "{text:?}");
    assert!(text.iter().any(|t| t == "64 MiB"), "{text:?}");
    assert!(!text.iter().any(|t| t.contains("164 MiB")), "{text:?}");
}

/// Every memory Create offers is the memory the file is written with, at
/// 3 passes, and an outside reader (KeePassXC's `db-info`) reads the same
/// cost from the file.
#[test]
fn every_cost_create_offers_is_written_as_stated() {
    use faraday_core::vault::{CUSTOM_MEMORY, PRESETS, vstep};
    let mut offered: Vec<(u32, u32)> = PRESETS.iter().map(|p| (p.1, p.2)).collect();
    offered.extend(CUSTOM_MEMORY.iter().map(|m| (*m, 3)));
    offered.sort_unstable();
    offered.dedup();
    for (memory, passes) in offered {
        let mut app = on_create_vault();
        app.press(Action::Vault(V::CStep(vstep::PRESET)));
        let _ = app.frame();
        app.press(Action::Vault(V::CCustomise));
        let _ = app.frame();
        app.press(Action::Vault(V::CNext(vstep::WHERE)));
        let k = CUSTOM_MEMORY.iter().position(|m| *m == memory).unwrap();
        app.press(Action::Vault(V::CMemory(k)));
        app.press(Action::Vault(V::CNext(vstep::COST)));
        let stated = app.vaults().form_cost().map(|c| (c.1, c.2)).unwrap();
        assert_eq!(stated, (memory, passes));
        let (written, file) = create_and_read(&mut app);
        assert_eq!(written, stated, "the file's own cost");
        outside_reader_agrees(&file, stated);
    }
}

/// The vault made, as `create_and_read_cost` reads it, with its bytes.
fn create_and_read(app: &mut Faraday) -> ((u32, u32), Vec<u8>) {
    let cost = create_and_read_cost(app);
    let bytes = app
        .outbox
        .iter()
        .find(|i| i.kind == faraday_core::wallet::FileKind::Vault)
        .map(|i| i.bytes.to_vec())
        .expect("the vault is in the Outbox");
    (cost, bytes)
}

/// `keepassxc-cli db-info` on the file reads the stated memory and passes
/// (`KDF: Argon2id (3 rounds, 65536 KB)`); skipped, and said, only where
/// the program is not installed.
fn outside_reader_agrees(file: &[u8], (memory, passes): (u32, u32)) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    // A folder of its own, made fresh and removed after, which is also the
    // reader's home.
    let dir = std::env::temp_dir().join(format!(
        "faraday-cost-{}-{memory}-{passes}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    let path = dir.join("v.kdbx");
    std::fs::write(&path, file).unwrap();
    let child = Command::new("keepassxc-cli")
        .args(["db-info", "-q"])
        .arg(&path)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("XDG_CACHE_HOME", &home)
        .env("XDG_DATA_HOME", &home)
        .env("XDG_RUNTIME_DIR", &dir)
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("LC_ALL", "C.UTF-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("keepassxc-cli is not installed: skipped");
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        Err(e) => panic!("keepassxc-cli did not start: {e}"),
    };
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"test phrase\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text} {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let kdf = format!(
        "Argon2id ({passes} rounds, {} KB)",
        u64::from(memory) * 1024
    );
    assert!(text.contains(&kdf), "{text}");
}

/// Customise's This computer is ticked and can be unticked; its row reads
/// the most unlock memory it gives a vault, at most 1 GiB, with what it
/// has free, and the cost suggested follows the machines chosen.
#[test]
fn this_computer_gives_what_it_has_free_and_can_be_unticked() {
    use faraday_core::vault::{PRESETS, vstep};
    let mut app = on_create_vault();
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 1229,
    });
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    app.press(Action::Vault(V::CCustomise));
    let _ = app.frame();
    let text = app.drawn_texts();
    assert!(
        text.iter()
            .any(|t| t == "1\u{a0}GiB (1.2\u{a0}GiB\u{a0}free)"),
        "{text:?}"
    );
    let suggested = |app: &Faraday| PRESETS[app.vaults().suggested()].1;
    // The default row ticks other PCs and the Zero 2 W, which opens 128 MiB
    // at most: Suggested is the 128 MiB row, and 256 MiB is too large.
    assert_eq!(suggested(&app), 128);
    assert_eq!(app.vaults().weakest().1, 128, "256 MiB is too large");
    // This computer alone: all it gives.
    app.press(Action::Vault(V::CMachine(0)));
    app.press(Action::Vault(V::CMachine(4)));
    assert_eq!(suggested(&app), 1024);
    // Unticked, with the Pi 3 alone.
    app.press(Action::Vault(V::CHere));
    app.press(Action::Vault(V::CMachine(3)));
    assert_eq!(suggested(&app), 512);
    let _ = app.frame();
    let text = app.drawn_texts();
    assert!(
        text.iter().any(|t| t == "Pi 3"),
        "opens on the Pi 3 alone: {text:?}"
    );

    // With plenty free, it gives 1 GiB at most.
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "1\u{a0}GiB (14.6\u{a0}GiB\u{a0}free)")
    );
}

/// A vault added with + new vault reads "made in the checklist" on the
/// board until the checklist has made it, and "sealed" once it has
/// (`docs/BACKUP-FLOW.md` §17.21 item 6).
#[test]
fn a_new_vault_reads_made_in_the_checklist_until_the_checklist_makes_it() {
    let band = |app: &mut Faraday| -> String {
        app.press(Action::BPlan);
        let _ = app.frame();
        app.drawn_texts()
            .plain()
            .into_iter()
            .find(|t| t.starts_with("Vault 1 ·") && (t.contains("sealed") || t.ends_with(" · new")))
            .unwrap_or_else(|| panic!("no band: {:?}", app.drawn_texts()))
    };
    let mut app = create_to_vault_step();
    let before = band(&mut app);
    assert!(
        before == "Vault 1 · sealed · made in the checklist" || before == "Vault 1 · new",
        "{before}"
    );
    app.press(Action::BChecklist);
    make_a_vault(&mut app);
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 30);
    assert_eq!(app.vaults().open.len(), 1, "the new vault is open");
    let after = band(&mut app);
    assert!(
        after.ends_with(" · sealed") && !after.contains("checklist"),
        "{after}"
    );
}

#[test]
fn creating_a_vault_with_too_few_answers_says_the_randomness_has_not_come() {
    // Three answers wait; a vault takes more.
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 1000,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    for i in 0..3u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::CGo));
    settle(&mut app, 1);
    assert_eq!(app.screen, Screen::CreateVault, "no vault was made");
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == faraday_strings::EN.no_randomness),
        "the form says why"
    );
}

#[test]
fn a_vault_waits_for_its_passphrase_typed_twice_alike() {
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "one phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, "another phrase");
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == faraday_core::strings::EN.pass_pair_differ),
        "the line says the two differ"
    );
    assert!(!app.offers(Action::Vault(V::CGo)), "Create waits");
    app.press(Action::Vault(V::CGo));
    assert_ne!(app.screen, Screen::Unlock, "no vault was made");
    // The second typed as the first is.
    app.press(Action::Vault(V::CFocus(0, true)));
    for _ in 0.."another phrase".len() {
        app.event(Event::Key(faraday_shell_api::Key::Backspace));
    }
    type_text(&mut app, "one phrase");
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::CGo)), "Create is on");
}

#[test]
fn dice_rolled_for_a_vaults_passphrase_keep_the_app_from_counting_clean_until_the_form_closes() {
    let mut app = on_create_vault();
    assert!(app.clean());
    app.press(Action::Vault(V::Dice(0)));
    type_text(&mut app, "12345");
    assert!(app.vaults().dice.open().is_some());
    assert!(!app.clean(), "rolls are held");
    app.press(Action::CancelFlow);
    app.press(Action::CancelFlowNow);
    assert!(app.vaults().create.is_none(), "the form closed");
    assert!(app.vaults().dice.open().is_none(), "the rolls are gone");
}

// ---------------------------------------------------------------------
// The flow container: one step's card at one place, Your choices at its
// right.
// ---------------------------------------------------------------------

/// The window made `width` × `height` at `dpi`.
fn window(app: &mut Faraday, (width, height): (u16, u16), dpi: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.settle();
    let _ = app.frame();
}

/// Where `text` is drawn, the first of it: (x, y, w, h).
fn drawn_at(app: &mut Faraday, text: &str) -> Option<(i32, i32, i32, i32)> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .plain()
        .into_iter()
        .find(|(t, _)| t == text)
        .map(|b| b.1)
}

#[test]
fn a_wide_window_shows_one_step_at_one_place() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    window(&mut app, (1366, 768), 160);
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    assert!(drawn_at(&mut app, "Step 1 of 2").is_some(), "the heading");
    assert!(drawn_at(&mut app, "Step 2 of 2").is_none(), "one card only");
    app.press(Action::Vault(V::CCustomise));
    let first = drawn_at(&mut app, "Step 1 of 3").expect("Where will you open it? is step 1");
    assert!(drawn_at(&mut app, "Where will you open it?").is_some());
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    let second = drawn_at(&mut app, "Step 2 of 3").expect("Unlock cost is step 2");
    assert!(
        drawn_at(&mut app, "Where will you open it?").is_none(),
        "the finished step is no longer drawn"
    );
    assert!(drawn_at(&mut app, "Step 1 of 3").is_none());
    assert_eq!(first.1, second.1, "the head stands at the same height");
    assert_eq!(first.0, second.0, "and at the same place across");
}

#[test]
fn your_choices_stands_at_the_right_on_a_wide_window_and_a_row_opens_its_step() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    window(&mut app, (1366, 768), 160);
    for text in [
        "Your choices",
        "Opens on",
        "Unlock cost",
        "Name and passphrase",
    ] {
        assert!(drawn_at(&mut app, text).is_some(), "{text} is drawn");
    }
    assert!(drawn_at(&mut app, "Step 2 of 2").is_some());
    let at = drawn_at(&mut app, "Your choices").unwrap();
    assert!(at.0 > 1366 / 2, "at the right: {at:?}");
    assert!(
        app.offers(Action::Vault(V::CStep(vstep::PRESET))),
        "a row opens its step"
    );
    assert!(
        !app.offers(Action::Vault(V::CStep(vstep::PHRASES))),
        "the step open takes no press on its row"
    );
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let _ = app.frame();
    assert!(drawn_at(&mut app, "Step 1 of 2").is_some());
    app.press(Action::Vault(V::CCustomise));
    let _ = app.frame();
    assert!(
        drawn_at(&mut app, "Not chosen yet").is_some(),
        "a step not reached yet says so"
    );
}

#[test]
fn a_small_panel_has_no_your_choices_pane() {
    use faraday_core::ChoicesPress;
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    window(&mut app, (480, 640), 160);
    assert!(drawn_at(&mut app, "Step 2 of 2").is_some());
    assert!(
        !app.offers(Action::Vault(V::CStep(vstep::PRESET))),
        "no pane: no row of Your choices opens a step"
    );
    assert!(
        app.offers(Action::Choices(ChoicesPress::Open)),
        "the bar's button opens Your choices instead"
    );
}

#[test]
fn create_vault_ends_the_last_card_and_only_there() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    window(&mut app, (1366, 768), 160);
    app.press(Action::Vault(V::CCustomise));
    app.press(Action::Vault(V::CStep(vstep::PHRASES)));
    let _ = app.frame();
    // The card is taller than the window: Create vault ends it.
    let count = drawn_at(&mut app, "Step 3 of 3").expect("the card's step count");
    let (x, y) = (
        u16::try_from(count.0).unwrap(),
        u16::try_from(count.1 + 40).unwrap(),
    );
    for _ in 0..40 {
        if app.offers(Action::Vault(V::CGo)) {
            break;
        }
        app.event(Event::Hover { x, y });
        app.event(Event::Scroll { x, y, dy: 200 });
        app.event(Event::ScrollEnd { x, y });
        app.settle();
        let _ = app.frame();
    }
    assert!(app.offers(Action::Vault(V::CGo)), "on Name and passphrase");
    app.event(Event::Scroll { x, y, dy: -4000 });
    app.event(Event::ScrollEnd { x, y });
    app.settle();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let _ = app.frame();
    assert!(
        !app.offers(Action::Vault(V::CGo)),
        "not in the pane nor on another step"
    );
}

#[test]
fn on_a_low_wide_window_your_choices_scrolls_within_itself() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    window(&mut app, (1366, 600), 160);
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    app.press(Action::Vault(V::CCustomise));
    let head = drawn_at(&mut app, "Your choices").expect("the pane is still shown");
    let (x, y) = (
        u16::try_from(head.0 + 20).unwrap(),
        u16::try_from(head.1 + 80).unwrap(),
    );
    let page = app.vaults().create.as_ref().unwrap().scroll.y;
    app.event(Event::Hover { x, y });
    app.event(Event::Scroll { x, y, dy: 800 });
    app.event(Event::ScrollEnd { x, y });
    app.settle();
    let _ = app.frame();
    let c = app.vaults().create.as_ref().unwrap();
    assert!(
        app.choices_offset() > 0.0,
        "the pane scrolled within itself"
    );
    assert_eq!(c.scroll.y, page, "the page did not move");
    let last = drawn_at(&mut app, "Goes to").expect("the pane's last line is on screen");
    assert!(last.1 + last.3 <= 600, "inside the window: {last:?}");
    // Opened anew, the pane starts at its top.
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    assert_eq!(app.choices_offset(), 0.0, "the form opened anew");
}

#[test]
fn a_cost_stays_while_it_fits_the_machines_and_is_cleared_when_it_does_not() {
    use faraday_core::vault::{CUSTOM_MEMORY, vstep};
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    app.press(Action::Vault(V::CCustomise));
    // The default row ticks the Zero 2 W: PCs alone first.
    app.press(Action::Vault(V::CMachine(4)));
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    let k = CUSTOM_MEMORY.iter().position(|m| *m == 512).unwrap();
    app.press(Action::Vault(V::CMemory(k)));
    app.press(Action::Vault(V::CNext(vstep::COST)));
    // Back to Opens on.
    app.press(Action::Vault(V::CStep(vstep::WHERE)));
    let _ = app.frame();
    app.press(Action::Vault(V::CHere));
    app.press(Action::Vault(V::CMachine(3)));
    let form = app.vaults().create.as_ref().unwrap();
    assert!(
        form.done[usize::from(vstep::COST)],
        "512 MiB still fits PCs"
    );
    // A Raspberry Pi Zero 2 W opens 128 MiB at most.
    app.press(Action::Vault(V::CMachine(4)));
    let form = app.vaults().create.as_ref().unwrap();
    assert!(
        !form.done[usize::from(vstep::COST)],
        "512 MiB no longer fits a Pi Zero 2 W: its step is to be gone through again"
    );
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    assert_eq!(
        app.vaults().create.as_ref().unwrap().open,
        Some(vstep::COST),
        "the cost is asked again"
    );
}

#[test]
fn a_new_size_keeps_the_passphrases_typed() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let _ = app.frame();
    app.press(Action::Vault(V::CSizeRow(3)));
    let form = app.vaults().create.as_ref().unwrap();
    assert_eq!(
        faraday_core::vault::Vaults::phrase_text(form, 0).unwrap().0,
        "test phrase"
    );
    assert_eq!(
        faraday_core::vault::Vaults::phrase_text(form, 0).unwrap().1,
        "test phrase"
    );
}

fn a_stick() -> faraday_core::StorageEvent {
    faraday_core::StorageEvent::Sticks(vec![faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }])
}

#[test]
fn a_stick_put_in_empties_a_new_vaults_passphrases() {
    let mut app = create_to_vault_step();
    app.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, "test phrase");
    app.storage(a_stick());
    let c = app.vaults().create.as_ref().unwrap();
    assert_eq!(
        faraday_core::vault::Vaults::phrase_text(c, 0),
        Some(("", "")),
        "nothing typed stays with a stick in"
    );
}

fn drawn_now(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts().plain()
}

#[test]
fn one_eye_shows_and_hides_a_new_vaults_passphrases() {
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    assert!(
        !drawn_now(&mut app).iter().any(|t| t == "test phrase"),
        "the passphrase is masked"
    );
    app.press(Action::PassEye);
    assert!(drawn_now(&mut app).iter().any(|t| t == "test phrase"));
    app.press(Action::PassEye);
    assert!(!drawn_now(&mut app).iter().any(|t| t == "test phrase"));
}

#[test]
fn an_eye_opened_on_create_a_vault_does_not_show_another_screens_passphrase() {
    for leave in [Action::Nav(Screen::Home), Action::Nav(Screen::Vaults)] {
        let mut app = on_create_vault();
        app.press(Action::Vault(V::CFocus(0, false)));
        type_text(&mut app, "test phrase");
        app.press(Action::PassEye);
        let _ = app.frame();
        app.press(leave);
        // The next event is where the app checks which fields are up.
        app.event(Event::Key(faraday_shell_api::Key::BackTab));
        app.press(Action::Lightning);
        app.press(Action::LAezeed);
        app.press(Action::LPassphrase);
        type_text(&mut app, "kept apart");
        assert!(
            !drawn_now(&mut app).iter().any(|t| t == "kept apart"),
            "the eye stayed open after leaving Create a vault"
        );
    }
}

#[test]
fn a_lock_empties_a_new_vaults_passphrases() {
    let mut app = create_to_vault_step();
    app.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    app.press(Action::Lock);
    assert!(
        app.vaults()
            .create
            .as_ref()
            .is_none_or(|c| faraday_core::vault::Vaults::phrase_text(c, 0)
                .is_none_or(|(a, b)| a.is_empty() && b.is_empty())),
        "the passphrase does not survive a lock"
    );
}

/// A 256 MiB cost chosen for other PCs alone is cleared by ticking the
/// Pi Zero 2 W, and the Unlock cost card then reads the larger rows too
/// large for it; no row is tagged Suggested.
#[test]
fn ticking_the_zero_2_w_clears_a_256_mib_cost_and_suggests_128() {
    use faraday_core::vault::vstep;
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    app.press(Action::Vault(V::CCustomise));
    app.press(Action::Vault(V::CMachine(4)));
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    app.press(Action::Vault(V::CPreset(2)));
    assert_eq!(form_memory(&app), 256);
    app.press(Action::Vault(V::CNext(vstep::COST)));
    app.press(Action::Vault(V::CStep(vstep::WHERE)));
    let _ = app.frame();
    app.press(Action::Vault(V::CMachine(4)));
    assert!(
        !app.vaults().create.as_ref().unwrap().done[usize::from(vstep::COST)],
        "256 MiB does not fit a Pi Zero 2 W: the cost is asked again"
    );
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    let _ = app.frame();
    let text = app.drawn_texts();
    assert!(!text.iter().any(|t| t == "Suggested"), "{text:?}");
    assert_eq!(
        text.iter()
            .filter(|t| *t == "Too large for Pi Zero 2 W")
            .count(),
        3,
        "256 MiB, 512 MiB and 1 GiB: {text:?}"
    );
    assert_eq!(
        faraday_core::vault::PRESETS[app.vaults().suggested()].1,
        128
    );
}
