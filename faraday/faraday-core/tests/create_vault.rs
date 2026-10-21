//! Pausing a new wallet's backup to make a vault for its keys: the
//! backup checklist's vault item, reached from Create's Back up card,
//! offers Make a vault, which opens the vault wizard; finishing it lands
//! on Unlock with the new vault picked and a way back; unlocking it, or
//! going back, returns to the backup still under way.

mod common;

use faraday_core::keygen::Way;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

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
    app.press(Action::CNext(faraday_core::cstep::KIND));
    app.press(Action::KeyGen(Some(0)));
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
        app.create.as_ref().unwrap().built.is_some(),
        "the wallet did not build"
    );
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    assert_eq!(app.screen, Screen::Backup);
    // No paper words: the vault's item comes first and opens.
    common::no_words(&mut app);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    assert_eq!(
        app.backup.as_ref().unwrap().open,
        Some(faraday_core::bstep::VAULT)
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
    settle(app, 1);
    assert_eq!(app.screen, Screen::Unlock);
    let name = app
        .vaults
        .just_made
        .clone()
        .expect("the vault was not made");
    assert_eq!(app.vault_files()[app.vaults.pick].name, name);
}

#[test]
fn going_back_from_unlock_leaves_the_new_vault_locked_and_returns_to_the_backup() {
    let mut app = create_to_vault_step();
    make_a_vault(&mut app);
    app.press(Action::Vault(V::Back));
    assert_eq!(
        app.screen,
        Screen::Backup,
        "the way back should return to the backup still under way"
    );
    assert!(app.vaults.open.is_empty(), "the new vault stays locked");
}

#[test]
fn opening_a_just_made_vault_needs_its_passphrase_typed_again() {
    let mut app = create_to_vault_step();
    make_a_vault(&mut app);
    settle(&mut app, 30);
    assert_eq!(
        app.screen,
        Screen::Unlock,
        "nothing typed while making the vault should open it on its own"
    );
    assert!(
        app.vaults.passphrase.text.is_empty(),
        "the passphrase must not be filled in for the person"
    );
}

#[test]
fn opening_a_just_made_vault_with_its_passphrase_returns_to_the_backup() {
    let mut app = create_to_vault_step();
    make_a_vault(&mut app);
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 30);
    assert_eq!(
        app.screen,
        Screen::Backup,
        "unlocking the vault just made should return to the backup still under way"
    );
}

#[test]
fn a_new_vaults_passphrases_wait_until_the_stick_is_pulled() {
    let mut app = create_to_vault_step();
    app.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    app.storage(faraday_core::StorageEvent::Sticks(vec![
        faraday_core::StickInfo {
            serial: None,
            id: "a".into(),
            label: "STICK".into(),
            boot: false,
            files: Vec::new(),
        },
    ]));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "typed with a stick in");
    app.press(Action::Vault(V::Dice(0)));
    type_text(&mut app, "12345");
    let c = app.vaults.create.as_ref().unwrap();
    assert!(
        c.phrases
            .iter()
            .all(|(a, b)| a.text.is_empty() && b.text.is_empty()),
        "nothing secret is typed while a stick is attached"
    );
    assert!(
        app.vaults.dice.is_none(),
        "no dice are rolled with a stick in"
    );

    app.storage(faraday_core::StorageEvent::Sticks(Vec::new()));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    assert_eq!(
        &*app.vaults.create.as_ref().unwrap().phrases[0].0.text,
        "test phrase"
    );
}

#[test]
fn a_finished_backup_offers_a_stick_and_asks_to_lock_first() {
    let c = faraday_core::testkit::compute();
    let mut app = create_to_vault_step();
    let wallet = app.create.as_ref().unwrap().built.unwrap();
    app.press(Action::Backup(wallet));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&c)
        .contains(&faraday_core::bstep::BLANK)
    {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
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
// §3.1 (`docs/SIMPLIFY.md`): two sizes, then Customise.
// ---------------------------------------------------------------------

/// The app with nothing loaded, memory to spare and the system's
/// randomness in, on Create a vault from Vaults.
fn on_create_vault() -> Faraday {
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
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    app
}

/// Types one passphrase twice, presses Create vault and waits: the cost
/// of the vault made, MiB and passes.
fn create_and_read_cost(app: &mut Faraday) -> (u32, u32) {
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CGo));
    settle(app, 1);
    assert_eq!(app.screen, Screen::Unlock, "the vault was made");
    let made = app.vaults.just_made.clone().expect("the vault was made");
    let f = app
        .vault_files()
        .into_iter()
        .find(|f| f.name == made)
        .unwrap();
    (f.header.cost.memory_kib / 1024, f.header.cost.passes)
}

#[test]
fn a_fresh_create_a_vault_opens_on_name_and_passphrases_with_64_mib_chosen() {
    use faraday_core::vaults::vstep;
    let mut app = on_create_vault();
    let c = app.vaults.create.as_ref().unwrap();
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

/// Every passphrase of a new vault opens a slot named after the vault:
/// the slot one passphrase opens says nothing of the others.
#[test]
fn every_slot_of_a_new_vault_goes_by_the_vaults_name() {
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CName));
    type_text(&mut app, "Savings");
    for (i, p) in ["first phrase", "second phrase"].iter().enumerate() {
        if i > 0 {
            app.press(Action::Vault(V::CAddPhrase));
        }
        app.press(Action::Vault(V::CFocus(i, false)));
        type_text(&mut app, p);
        app.press(Action::Vault(V::CFocus(i, true)));
        type_text(&mut app, p);
    }
    app.press(Action::Vault(V::CGo));
    settle(&mut app, 1);
    assert_eq!(app.screen, Screen::Unlock, "the vault was made");
    // The second passphrase's slot reads as the vault, not as a second.
    type_text(&mut app, "second phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 30);
    assert_eq!(app.vaults.open.len(), 1, "it unlocked");
    assert_eq!(app.vaults.open[0].label(), "Savings");
}

#[test]
fn pcs_only_makes_a_512_mib_vault() {
    use faraday_core::vaults::vstep;
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::CSizeRow(0))), "PCs only");
    assert!(
        app.offers(Action::Vault(V::CSizeRow(1))),
        "PCs and a Raspberry Pi"
    );
    app.press(Action::Vault(V::CSizeRow(0)));
    app.press(Action::Vault(V::CNext(vstep::PRESET)));
    assert_eq!(
        app.vaults.create.as_ref().unwrap().open,
        Some(vstep::PHRASES),
        "Continue goes on to the passphrases"
    );
    assert_eq!(create_and_read_cost(&mut app), (512, 3));
}

#[test]
fn customise_opens_the_three_cards_and_a_custom_memory_survives_continue() {
    use faraday_core::vaults::{CUSTOM_MEMORY, vstep};
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::CCustomise)));
    app.press(Action::Vault(V::CCustomise));
    let _ = app.frame();
    for step in [vstep::WHERE, vstep::COST, vstep::SIZE] {
        assert!(
            app.offers(Action::Vault(V::CStep(step))),
            "Customise shows step {step}"
        );
    }
    assert!(
        !app.offers(Action::Vault(V::CStep(vstep::PRESET))),
        "in place of the size"
    );
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    let k = CUSTOM_MEMORY.iter().position(|m| *m == 128).unwrap();
    app.press(Action::Vault(V::CMemory(k)));
    app.press(Action::Vault(V::CNext(vstep::COST)));
    app.press(Action::Vault(V::CNext(vstep::SIZE)));
    assert_eq!(create_and_read_cost(&mut app).0, 128);
}

/// The vault's memory is one figure: the size, the cost and the summary
/// all say 64 MiB for PCs and a Raspberry Pi, never the app's share on
/// top of it.
#[test]
fn the_unlock_memory_reads_the_same_on_the_size_and_the_summary() {
    use faraday_core::vaults::vstep;
    let mut app = on_create_vault();
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let text = app.drawn_texts();
    assert!(text.iter().any(|t| t == "Unlock memory"), "{text:?}");
    assert!(text.iter().any(|t| t == "64 MiB"), "{text:?}");
    assert!(!text.iter().any(|t| t.contains("164 MiB")), "{text:?}");
}

/// Customise's This computer is ticked and can be unticked; its row reads
/// the most unlock memory it gives a vault, at most 4 GiB, with what it
/// has free, and the cost suggested follows the machines chosen.
#[test]
fn this_computer_gives_what_it_has_free_and_can_be_unticked() {
    use faraday_core::vaults::{PRESETS, vstep};
    let mut app = on_create_vault();
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 1229,
    });
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    app.press(Action::Vault(V::CCustomise));
    let _ = app.frame();
    let text = app.drawn_texts();
    assert!(text.iter().any(|t| t == "1 GiB (1.2 GiB free)"), "{text:?}");
    let suggested = |app: &Faraday| PRESETS[app.vaults.suggested()].1;
    // PCs and a Raspberry Pi chose other PCs and the Pi 3: half the
    // Pi's 1 GB.
    assert_eq!(suggested(&app), 512);
    // This computer alone: all it gives.
    app.press(Action::Vault(V::CMachine(0)));
    app.press(Action::Vault(V::CMachine(3)));
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

    // With plenty free, it gives 4 GiB at most.
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "4 GiB (14.6 GiB free)")
    );
}

/// A vault added with + new vault reads "made in the checklist" on the
/// board until the checklist has made it, and "sealed" once it has
/// (`docs/BACKUP-FLOW.md` §19.21 item 6).
#[test]
fn a_new_vault_reads_made_in_the_checklist_until_the_checklist_makes_it() {
    let band = |app: &mut Faraday| -> String {
        app.press(Action::BPlan);
        let _ = app.frame();
        app.drawn_texts()
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
    assert_eq!(app.vaults.open.len(), 1, "the new vault is open");
    let after = band(&mut app);
    assert!(
        after.ends_with(" · sealed") && !after.contains("checklist"),
        "{after}"
    );
}
