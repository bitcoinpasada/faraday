//! Pausing a new wallet's backup to make a vault for its keys: the
//! backup checklist's vault item, reached from Create's Back up card,
//! offers Make a vault, which opens the vault wizard; finishing it lands
//! on Unlock with the new vault picked and a way back; unlocking it, or
//! going back, returns to the backup still under way.

use faraday_core::keygen::Way;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

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
    let paper_and_vault = faraday_core::plan::Preset::ALL
        .iter()
        .position(|p| *p == faraday_core::plan::Preset::PaperVault)
        .unwrap() as u8;
    app.press(Action::CBackup(paper_and_vault));
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BChecklist);
    app.press(Action::BStep(faraday_core::bstep::VAULT));
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
    let mut app = create_to_vault_step();
    let wallet = app.create.as_ref().unwrap().built.unwrap();
    app.press(Action::Backup(wallet));
    app.press(Action::BPreset(0));
    app.press(Action::BChecklist);
    app.press(Action::BOut(0));
    app.press(Action::BOut(3));
    let pdfs = app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".pdf"))
        .count();
    assert_eq!(pdfs, 2, "the template and the backup sheet go out");

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
