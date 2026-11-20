//! Whatever needs a vault leads into making or unlocking one, and back.
//! With no vault file, Make a vault opens Create a vault; making one
//! goes straight to Unlock with it picked, and unlocking it returns to
//! the step that asked. With a vault file and none unlocked, Unlock
//! opens with it picked, after the stick is pulled. Going back on the
//! way returns to the flow and drops a secret held for it. Leaving Unlock,
//! by its back link or any other way, drops what was typed there. Unlock
//! is refused, before any work, on a computer without the memory free that
//! the vault's unlock memory, twice its file and the app need.

use crate::common::type_text;
use faraday_core::catalog::TILES;
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_vault::records::kind;

const NEW_PHRASE: &str = "test phrase";

/// The Tools tiles that open a vault category, with the category. The GPG
/// and Secure Boot tiles are not among them: they start the backup
/// flow, which decides where the key will be kept (F31).
const VAULT_TILES: [(&str, usize); 1] = [("KeePass export", 2)];

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

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
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
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x20 + i; 32])));
    }
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    let _ = app.frame();
    app
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

fn tile(name: &str) -> u8 {
    TILES.iter().position(|t| t.name == name).unwrap() as u8
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(app);
}

/// On Create a vault: the default size, the passphrase twice, Create. The
/// vault opens at once with what was typed.
fn make_the_vault(app: &mut Faraday) {
    assert_eq!(
        app.screen,
        Screen::CreateVault,
        "Create a vault did not open"
    );
    type_text(app, NEW_PHRASE);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, NEW_PHRASE);
    app.press(Action::Vault(V::CGo));
    settle(app, |a| a.screen == Screen::Unlock);
    assert!(app.vaults().open.is_empty(), "the vault is made locked");
    unlock_with(app, NEW_PHRASE);
}

/// The session locked: every open vault sealed and closed.
fn lock_it(app: &mut Faraday) {
    app.press(Action::Lock);
    settle(app, |a| a.vaults().open.is_empty());
    assert!(app.vaults().open.is_empty(), "the vault did not lock");
}

/// On Unlock: the passphrase, Unlock, and the wait.
fn unlock_with(app: &mut Faraday, phrase: &str) {
    assert_eq!(app.screen, Screen::Unlock);
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults().open.is_empty());
    assert_eq!(app.vaults().open.len(), 1, "the vault did not unlock");
}

/// Whether the page offers `action` anywhere down it, scrolled to as a
/// person would.
fn offered(app: &mut Faraday, action: Action) -> bool {
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(action) {
            return true;
        }
        app.event(Event::Scroll {
            x: 683,
            y: 384,
            dy: 300,
        });
    }
    false
}

/// Presses what the page offers in place of a vault's button: the
/// detour that comes back to `back`.
fn press_make(app: &mut Faraday, back: Screen) {
    let make = Action::Vault(V::CreateFrom(back));
    assert!(offered(app, make), "Make a vault is not offered");
    app.press(make);
}

#[test]
fn a_vault_tile_with_no_vault_says_make_one_and_makes_it_then_opens_its_category() {
    for (name, category) in VAULT_TILES {
        let mut app = device(Vec::new());
        app.press(Action::Nav(Screen::Catalog));
        let i = tile(name);
        assert_eq!(
            app.tile_need(TILES[usize::from(i)].go),
            Some("Make a new vault first")
        );
        app.press(Action::Catalog(i));
        make_the_vault(&mut app);
        assert_eq!(app.screen, Screen::VaultContents, "{name}");
        assert_eq!(app.vaults().category, category, "{name}");
        let made = app.vaults().just_made.clone();
        assert!(made.is_none(), "unlocking clears the new vault's mark");
        assert_eq!(app.vaults().current, 0);
    }
}

#[test]
fn a_vault_tile_with_a_locked_vault_unlocks_it_then_opens_its_category() {
    for (name, category) in VAULT_TILES {
        let mut app = device(vec![kit_file("vault.kdbx")]);
        app.press(Action::Nav(Screen::Catalog));
        let i = tile(name);
        assert_eq!(
            app.tile_need(TILES[usize::from(i)].go),
            Some("Unlock a vault first")
        );
        app.press(Action::Catalog(i));
        assert_eq!(app.screen, Screen::Unlock, "{name}");
        assert_eq!(app.vaults().pick, 0, "the only vault is picked");
        unlock_with(&mut app, testkit::VAULT_PASSPHRASE);
        assert_eq!(app.screen, Screen::VaultContents, "{name}");
        assert_eq!(app.vaults().category, category, "{name}");
    }
}

#[test]
fn a_vault_tile_with_a_stick_in_asks_for_the_stick_before_unlock() {
    let mut app = device(vec![kit_file("vault.kdbx")]);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("KeePass export")));
    assert_eq!(app.sheet, Some(Sheet::Pull), "the Pull sheet asks first");
    assert_ne!(app.screen, Screen::Unlock);
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Unlock, "Unlock opens once it is out");
    unlock_with(&mut app, testkit::VAULT_PASSPHRASE);
    assert_eq!(app.screen, Screen::VaultContents);
    assert_eq!(app.vaults().category, 2);
}

#[test]
fn going_back_from_create_returns_to_tools_and_a_later_unlock_is_an_ordinary_one() {
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("KeePass export")));
    assert_eq!(app.screen, Screen::CreateVault);
    app.press(Action::Vault(V::Back));
    assert_eq!(app.screen, Screen::Catalog, "back returns to Tools");
    assert_eq!(app.vaults().back_to, None);

    // A vault made and unlocked from Vaults afterwards does not open GPG.
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    type_text(&mut app, NEW_PHRASE);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, NEW_PHRASE);
    app.press(Action::Vault(V::CGo));
    settle(&mut app, |a| a.screen == Screen::Unlock);
    unlock_with(&mut app, NEW_PHRASE);
    assert_eq!(
        app.screen,
        Screen::VaultContents,
        "made from Vaults, it opens once unlocked"
    );
    assert!(
        !(app.screen == Screen::VaultContents && app.vaults().category == 2),
        "an unlock from Vaults does not open the KeePass export"
    );
}

#[test]
fn bip85_makes_a_vault_and_comes_back_to_save_the_value() {
    let mut app = device(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    press_make(&mut app, Screen::Bip85);
    make_the_vault(&mut app);
    assert_eq!(app.screen, Screen::Bip85);
    let _ = app.frame();
    assert!(app.offers(Action::PVault), "Save into the vault is live");
    app.press(Action::PVault);
    assert_eq!(
        app.vaults()
            .records(app.vaults().open[0].id)
            .of(kind::ENTRY)
            .count(),
        1
    );
}

#[test]
fn lightning_makes_a_vault_and_comes_back_to_save_the_node_key() {
    let mut app = device(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Lightning);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::LKey(fp));
    press_make(&mut app, Screen::Lightning);
    make_the_vault(&mut app);
    assert_eq!(app.screen, Screen::Lightning);
    let _ = app.frame();
    assert!(app.offers(Action::LVault), "Save into the vault is live");
    let notes = app
        .vaults()
        .records(app.vaults().open[0].id)
        .of(kind::NOTE)
        .count();
    app.press(Action::LVault);
    assert_eq!(
        app.vaults()
            .records(app.vaults().open[0].id)
            .of(kind::NOTE)
            .count(),
        notes + 1
    );
}

#[test]
fn lightning_keeps_its_passphrase_with_its_node_on_the_way_to_a_vault_and_a_lock_drops_both() {
    const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon \
        abandon abandon abandon about";
    let read = |app: &mut Faraday| {
        app.press(Action::LPassphrase);
        app.event(Event::Key(faraday_shell_api::Key::Enter));
        let _ = app.frame();
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        testkit::settle(app);
        let l = app.lightning().unwrap();
        let n = l.node.as_ref().unwrap_or_else(|| panic!("{:?}", l.error));
        n.public().to_vec()
    };
    let mut app = device(Vec::new());
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    type_text(&mut app, ABANDON);
    app.press(Action::LPassphrase);
    type_text(&mut app, "pw");
    let first = read(&mut app);
    press_make(&mut app, Screen::Lightning);
    make_the_vault(&mut app);
    assert_eq!(app.screen, Screen::Lightning);
    assert_eq!(app.lightning().unwrap().passphrase(), "pw");
    assert_eq!(read(&mut app), first, "the same passphrase, the same node");
    lock_it(&mut app);
    assert!(
        app.lightning()
            .is_none_or(|l| l.passphrase().is_empty() && l.node.is_none()),
        "the node and its passphrase went with the lock"
    );
}

#[test]
fn silent_payments_makes_a_vault_and_comes_back_to_save_the_scan_key() {
    let mut app = device(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Silent);
    app.press(Action::SStep(faraday_core::silent::sstep::SCAN));
    press_make(&mut app, Screen::Silent);
    make_the_vault(&mut app);
    assert_eq!(app.screen, Screen::Silent);
    let _ = app.frame();
    assert!(
        app.offers(Action::SScanVault),
        "Save into the vault is live"
    );
    let notes = app
        .vaults()
        .records(app.vaults().open[0].id)
        .of(kind::NOTE)
        .count();
    app.press(Action::SScanVault);
    assert_eq!(
        app.vaults()
            .records(app.vaults().open[0].id)
            .of(kind::NOTE)
            .count(),
        notes + 1
    );
}

/// A one-key wallet from test key 2, its backup's plan of paper and a
/// vault made, open on the seed's copy.
fn on_the_seeds_step(app: &mut Faraday) {
    add_key(app, 1);
    let fp = app.session.keys[0].fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(app, faraday_core::plan::Preset::PaperVault);
    app.press(Action::BChecklist);
    app.press(Action::BStep(faraday_core::plan::Item::Copy(0)));
    app.press(Action::BReveal);
    let _ = app.frame();
}

#[test]
fn the_seeds_vault_item_makes_a_vault_and_comes_back_with_save_live() {
    let mut app = device(Vec::new());
    on_the_seeds_step(&mut app);
    // No paper words: the vault's item comes first.
    app.press(Action::BPlan);
    crate::common::no_words(&mut app);
    app.press(Action::BChecklist);
    let step = app.backup().unwrap().open;
    assert_eq!(step, Some(faraday_core::plan::Item::Vault(0)));
    press_make(&mut app, Screen::Backup);
    make_the_vault(&mut app);
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup().unwrap().open, step, "the same step");
    assert!(
        offered(&mut app, Action::BVaultSave(0)),
        "Save into the vault is live"
    );
}

#[test]
fn the_secret_sheet_waits_while_a_vault_is_made_and_saves_into_it() {
    let mut app = device(Vec::new());
    on_the_seeds_step(&mut app);
    app.press(Action::BFile);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    press_make(&mut app, Screen::Backup);
    assert_eq!(app.sheet, None, "the sheet closes on the way");
    assert!(testkit::secret_out(&app).is_some(), "its secret is kept");
    make_the_vault(&mut app);
    assert!(testkit::secret_out(&app).is_some(), "its secret is kept");
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.sheet, Some(Sheet::SecretOut), "the sheet is back");
    let _ = app.frame();
    assert!(
        app.offers(Action::SecretVault),
        "Save into the vault is live"
    );
    app.press(Action::SecretVault);
    assert_eq!(
        app.vaults()
            .records(app.vaults().open[0].id)
            .of(kind::KEY)
            .count(),
        1
    );
    assert!(testkit::secret_out(&app).is_none());
    assert!(
        app.outbox.iter().all(|i| !i.secret),
        "nothing went out bare"
    );
}

#[test]
fn the_secret_sheet_does_not_offer_saving_a_seed_the_vault_holds() {
    let mut app = device(Vec::new());
    on_the_seeds_step(&mut app);
    app.press(Action::BFile);
    press_make(&mut app, Screen::Backup);
    make_the_vault(&mut app);
    let _ = app.frame();
    app.press(Action::SecretVault);
    let id = app.vaults().open[0].id;
    let keys = |app: &Faraday| app.vaults().records(id).of(kind::KEY).count();
    assert_eq!(keys(&app), 1);
    app.press(Action::BFile);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(
        !app.offers(Action::SecretVault),
        "the vault holds the seed already"
    );
    app.press(Action::SecretVault);
    assert_eq!(keys(&app), 1, "nothing is added");
    assert!(
        testkit::secret_out(&app).is_some(),
        "the sheet keeps its secret"
    );
}

#[test]
fn going_back_on_the_way_drops_the_secret() {
    let mut app = device(Vec::new());
    on_the_seeds_step(&mut app);
    app.press(Action::BFile);
    press_make(&mut app, Screen::Backup);
    app.press(Action::Vault(V::Back));
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.sheet, None);
    assert!(testkit::secret_out(&app).is_none(), "the secret is dropped");

    // Leaving the way for another page drops it too.
    app.press(Action::BFile);
    press_make(&mut app, Screen::Backup);
    app.press(Action::Nav(Screen::Home));
    assert!(testkit::secret_out(&app).is_none(), "the secret is dropped");
    assert_eq!(app.vaults().back_to, None);
}

/// An entries file in Files with no vault offers no way to one on its row
/// (F60); once a vault is made and open, its Import is live.
#[test]
fn an_entries_file_in_files_offers_import_once_a_vault_is_open() {
    let name = "entries.txt".to_string();
    let mut app = device(vec![(
        name.clone(),
        testkit::ENTRIES_FILE.as_bytes().to_vec(),
    )]);
    app.press(Action::Nav(Screen::Files));
    let k = app
        .inbox()
        .iter()
        .position(|i| i.name.as_str() == name.as_str())
        .unwrap();
    assert!(!offered(
        &mut app,
        Action::Vault(V::CreateFrom(Screen::Files))
    ));
    assert!(!offered(&mut app, Action::Vault(V::ImportEntries(k))));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    make_the_vault(&mut app);
    app.press(Action::Nav(Screen::Files));
    assert!(
        offered(&mut app, Action::Vault(V::ImportEntries(k))),
        "Import into the vault is live"
    );
}

// ---------------------------------------------------------------------
// §3.3: after creation, the vault is open; an
// empty vault's next steps.
// ---------------------------------------------------------------------

#[test]
fn leaving_unlock_drops_a_half_typed_passphrase() {
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    make_the_vault(&mut app);
    lock_it(&mut app);
    for way_out in [Action::Nav(Screen::Vaults), Action::Nav(Screen::Home)] {
        app.press(Action::Nav(Screen::Vaults));
        app.press(Action::Vault(V::Open(0)));
        assert_eq!(app.screen, Screen::Unlock);
        type_text(&mut app, "test phr");
        app.press(way_out);
        assert!(
            app.vaults().passphrase.text.is_empty(),
            "{way_out:?} kept the typed passphrase"
        );
    }
    // Typed in full and unlocked, it opens: nothing left over was in the way.
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    unlock_with(&mut app, NEW_PHRASE);
}

#[test]
fn unlock_counts_the_file_twice_in_the_memory_it_needs() {
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    make_the_vault(&mut app);
    lock_it(&mut app);
    // The default's 256 MiB, twice the file and 100 MiB for the app.
    const MIB: usize = 1024 * 1024;
    let file = app
        .outbox
        .last()
        .expect("the vault was written")
        .bytes
        .len();
    let need = 256 + ((2 * file + MIB / 2) / MIB) as u32 + 100;
    app.storage(StorageEvent::Memory {
        available_mib: need - 1,
    });
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, NEW_PHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults().open.is_empty());
    assert!(app.vaults().open.is_empty(), "it unlocked short of memory");
    assert_eq!(
        app.vaults().unlock_error,
        Some(faraday_text::Text::of(&format!(
            "Needs {need} MiB. This computer has {} MiB free.",
            need - 1
        )))
    );
    app.storage(StorageEvent::Memory {
        available_mib: need,
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    unlock_with(&mut app, NEW_PHRASE);
}

#[test]
fn create_a_vault_leaves_it_locked_and_its_unlock_takes_the_passphrase_once() {
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    type_text(&mut app, NEW_PHRASE);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, NEW_PHRASE);
    // As a person presses it: the make runs on the worker, with frames
    // drawn between.
    testkit::press_on_worker(&mut app, Action::Vault(V::CGo));
    for t in 1..60u64 {
        testkit::settle(&mut app);
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
        if app.screen == Screen::Unlock {
            break;
        }
    }
    assert_eq!(app.screen, Screen::Unlock, "its Unlock opens");
    assert!(app.vaults().open.is_empty(), "the new vault is locked");
    // A wrong passphrase is refused as at any unlock.
    type_text(&mut app, "not the passphrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| a.vaults().unlock_error.is_some());
    assert!(
        app.vaults().open.is_empty(),
        "a wrong passphrase opens nothing"
    );
    app.press(Action::Vault(V::FocusPassphrase));
    unlock_with(&mut app, NEW_PHRASE);
    assert_eq!(app.screen, Screen::VaultContents);
}

#[test]
fn an_empty_open_vault_offers_every_add_and_a_stick() {
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    make_the_vault(&mut app);
    assert_eq!(
        app.screen,
        Screen::VaultContents,
        "the vault just made opens on its contents"
    );
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Nothing in it yet"),
        "it says it is empty"
    );
    // Every way to add, with no press first.
    for add in [
        "Save a bitcoin key",
        "Save a wallet",
        "Add an entry",
        "Add a note",
        "Make a GPG key",
        "Make Secure Boot keys",
    ] {
        assert!(texts.iter().any(|t| t == add), "{add}: {texts:?}");
    }
    let _ = app.frame();
    for k in 0..faraday_core::vault::CATEGORIES.len() {
        assert!(app.offers(Action::Vault(V::AddKind(k))), "add kind {k}");
    }
    assert!(app.offers(Action::WriteAsk), "Write it to a stick");
    assert!(texts.iter().any(|t| t == "Write it to a stick"));
    app.press(Action::WriteAsk);
    assert_eq!(
        app.sheet,
        Some(faraday_core::Sheet::WriteOut),
        "the write asks to lock first, as an open vault holds a secret"
    );
}

#[test]
fn vaults_with_no_vault_in_files_opens_new_vault_and_back_returns_to_where_it_was_pressed() {
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Nav(Screen::Vaults));
    assert_eq!(app.screen, Screen::CreateVault);
    leave_create(&mut app);
    assert_eq!(
        app.screen,
        Screen::Home,
        "Back is not sent into New vault again"
    );

    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Nav(Screen::Vaults));
    assert_eq!(app.screen, Screen::CreateVault);
    leave_create(&mut app);
    assert_eq!(app.screen, Screen::Catalog);
}

/// Back out of New vault: a step each press, then the page itself.
fn leave_create(app: &mut Faraday) {
    for _ in 0..4 {
        if app.screen != Screen::CreateVault {
            break;
        }
        app.press(Action::Back);
    }
}

#[test]
fn vaults_with_a_vault_file_in_files_still_shows_the_page() {
    let mut app = device(vec![kit_file("vault.kdbx")]);
    app.press(Action::Nav(Screen::Vaults));
    assert_eq!(app.screen, Screen::Vaults);
}

#[test]
fn vaults_with_samples_on_keeps_the_page_and_the_demo_bar_pulses_its_sample() {
    let mut app = device(Vec::new());
    app.session.set_demo(testkit::demo_samples());
    app.samples = Some(testkit::demo_samples());
    app.press(Action::Nav(Screen::Vaults));
    assert_eq!(app.screen, Screen::Vaults);
    assert!(
        app.vaults_offer_pulses(),
        "Use a sample pulses on the empty page"
    );

    let mut app = device(vec![kit_file("vault.kdbx")]);
    app.session.set_demo(testkit::demo_samples());
    app.samples = Some(testkit::demo_samples());
    app.press(Action::Nav(Screen::Vaults));
    assert!(
        !app.vaults_offer_pulses(),
        "nothing to pulse with a vault in Files"
    );
}

#[test]
fn the_boot_stick_pill_asks_for_the_stick_out_on_unlock_and_stops_when_it_is_out() {
    let mut app = device(vec![kit_file("vault.kdbx")]);
    app.shell_sticks = true;
    let boot = |app: &Faraday| app.stick_pills()[0].asked;
    app.press(Action::Nav(Screen::Vaults));
    assert!(!boot(&app), "no stick in, nothing to pull");
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of(faraday_core::BOOT_FOLDER),
        boot: true,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    assert!(!boot(&app), "the Vaults list asks for nothing");
    app.press(Action::Vault(V::Open(0)));
    assert_eq!(app.screen, Screen::Unlock);
    assert!(boot(&app), "Unlock waits for the stick to be pulled");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert!(!boot(&app), "the pulse stops as soon as the stick is out");
}
