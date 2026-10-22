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
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, bstep};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_vault::records::kind;

const NEW_PHRASE: &str = "test phrase";

/// The Tools tiles that open a vault category, with the category. The GPG,
/// Secure Boot and APK tiles are not among them: they start the backup
/// flow, which decides where the key will be kept (F31).
const VAULT_TILES: [(&str, usize); 1] = [("KeePass export", 2)];

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
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
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x20 + i; 32])));
    }
    app.vaults.ms_per_unit = Some(180);
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
}

/// On Create a vault: the default size, one passphrase, Create. Lands on
/// Unlock with the new vault picked.
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
    settle(app, |a| a.screen != Screen::CreateVault);
    assert_eq!(
        app.screen,
        Screen::Unlock,
        "a vault made for another flow goes straight to Unlock"
    );
    let made = app
        .vaults
        .just_made
        .clone()
        .expect("the vault was not made");
    assert_eq!(
        app.vault_files()[app.vaults.pick].name,
        made,
        "Unlock has the new vault picked"
    );
}

/// On Unlock: the passphrase, Unlock, and the wait.
fn unlock_with(app: &mut Faraday, phrase: &str) {
    assert_eq!(app.screen, Screen::Unlock);
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not unlock");
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
            Some("Make a vault first")
        );
        app.press(Action::Catalog(i));
        make_the_vault(&mut app);
        unlock_with(&mut app, NEW_PHRASE);
        assert_eq!(app.screen, Screen::VaultContents, "{name}");
        assert_eq!(app.vaults.category, category, "{name}");
        let made = app.vaults.just_made.clone();
        assert!(made.is_none(), "unlocking clears the new vault's mark");
        assert_eq!(app.vaults.current, 0);
    }
}

#[test]
fn a_vault_tile_with_a_locked_vault_unlocks_it_then_opens_its_category() {
    for (name, category) in VAULT_TILES {
        let mut app = device(vec![kit_file("vault.ofv")]);
        app.press(Action::Nav(Screen::Catalog));
        let i = tile(name);
        assert_eq!(
            app.tile_need(TILES[usize::from(i)].go),
            Some("Unlock a vault first")
        );
        app.press(Action::Catalog(i));
        assert_eq!(app.screen, Screen::Unlock, "{name}");
        assert_eq!(app.vaults.pick, 0, "the only vault is picked");
        unlock_with(&mut app, testkit::VAULT_PASSPHRASES[0]);
        assert_eq!(app.screen, Screen::VaultContents, "{name}");
        assert_eq!(app.vaults.category, category, "{name}");
    }
}

#[test]
fn a_vault_tile_with_a_stick_in_asks_for_the_stick_before_unlock() {
    let mut app = device(vec![kit_file("vault.ofv")]);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "a".into(),
        label: "STICK".into(),
        boot: false,
        files: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("KeePass export")));
    assert_eq!(app.sheet, Some(Sheet::Pull), "the Pull sheet asks first");
    assert_ne!(app.screen, Screen::Unlock);
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Unlock, "Unlock opens once it is out");
    unlock_with(&mut app, testkit::VAULT_PASSPHRASES[0]);
    assert_eq!(app.screen, Screen::VaultContents);
    assert_eq!(app.vaults.category, 2);
}

#[test]
fn going_back_from_create_returns_to_tools_and_a_later_unlock_is_an_ordinary_one() {
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("KeePass export")));
    assert_eq!(app.screen, Screen::CreateVault);
    app.press(Action::Vault(V::Back));
    assert_eq!(app.screen, Screen::Catalog, "back returns to Tools");
    assert_eq!(app.vaults.back_to, None);

    // A vault made and unlocked from Vaults afterwards does not open GPG.
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    type_text(&mut app, NEW_PHRASE);
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, NEW_PHRASE);
    app.press(Action::Vault(V::CGo));
    settle(&mut app, |a| a.screen != Screen::CreateVault);
    assert_eq!(
        app.screen,
        Screen::Unlock,
        "made from Vaults, it goes to Unlock with the new vault picked"
    );
    app.press(Action::Vault(V::Open(0)));
    unlock_with(&mut app, NEW_PHRASE);
    assert!(
        !(app.screen == Screen::VaultContents && app.vaults.category == 2),
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
    unlock_with(&mut app, NEW_PHRASE);
    assert_eq!(app.screen, Screen::Bip85);
    let _ = app.frame();
    assert!(app.offers(Action::PVault), "Save into the vault is live");
    app.press(Action::PVault);
    assert_eq!(app.vaults.open[0].contents.of(kind::ENTRY).count(), 1);
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
    unlock_with(&mut app, NEW_PHRASE);
    assert_eq!(app.screen, Screen::Lightning);
    let _ = app.frame();
    assert!(app.offers(Action::LVault), "Save into the vault is live");
    let notes = app.vaults.open[0].contents.of(kind::NOTE).count();
    app.press(Action::LVault);
    assert_eq!(
        app.vaults.open[0].contents.of(kind::NOTE).count(),
        notes + 1
    );
}

#[test]
fn silent_payments_makes_a_vault_and_comes_back_to_save_the_scan_key() {
    let mut app = device(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Silent);
    app.press(Action::SNext);
    press_make(&mut app, Screen::Silent);
    make_the_vault(&mut app);
    unlock_with(&mut app, NEW_PHRASE);
    assert_eq!(app.screen, Screen::Silent);
    let _ = app.frame();
    assert!(
        app.offers(Action::SScanVault),
        "Save into the vault is live"
    );
    let notes = app.vaults.open[0].contents.of(kind::NOTE).count();
    app.press(Action::SScanVault);
    assert_eq!(
        app.vaults.open[0].contents.of(kind::NOTE).count(),
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
    app.press(Action::BStep(bstep::COPY));
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
    let step = app.backup.as_ref().unwrap().open;
    assert_eq!(step, Some(bstep::VAULT));
    press_make(&mut app, Screen::Backup);
    make_the_vault(&mut app);
    unlock_with(&mut app, NEW_PHRASE);
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup.as_ref().unwrap().open, step, "the same step");
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
    assert!(app.secret_out.is_some(), "its secret is kept");
    make_the_vault(&mut app);
    assert!(app.secret_out.is_some(), "its secret is kept");
    unlock_with(&mut app, NEW_PHRASE);
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.sheet, Some(Sheet::SecretOut), "the sheet is back");
    let _ = app.frame();
    assert!(
        app.offers(Action::SecretVault),
        "Save into the vault is live"
    );
    app.press(Action::SecretVault);
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 1);
    assert!(app.secret_out.is_none());
    assert!(
        app.outbox.iter().all(|i| !i.secret),
        "nothing went out bare"
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
    assert!(app.secret_out.is_none(), "the secret is dropped");

    // Leaving the way for another page drops it too.
    app.press(Action::BFile);
    press_make(&mut app, Screen::Backup);
    app.press(Action::Nav(Screen::Home));
    assert!(app.secret_out.is_none(), "the secret is dropped");
    assert_eq!(app.vaults.back_to, None);
}

/// An `.oskb` in Files with no vault offers no way to one on its row
/// (F60); once a vault is made and open, its Import is live.
#[test]
fn an_oskb_in_files_offers_import_once_a_vault_is_open() {
    let (name, bytes) = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n.ends_with(".oskb"))
        .unwrap();
    let mut app = device(vec![(name.clone(), bytes)]);
    app.press(Action::Nav(Screen::Files));
    let k = app.inbox.iter().position(|i| i.name == name).unwrap();
    assert!(!offered(
        &mut app,
        Action::Vault(V::CreateFrom(Screen::Files))
    ));
    assert!(!offered(&mut app, Action::Vault(V::ImportBackup(k))));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    make_the_vault(&mut app);
    unlock_with(&mut app, NEW_PHRASE);
    app.press(Action::Nav(Screen::Files));
    assert!(
        offered(&mut app, Action::Vault(V::ImportBackup(k))),
        "Import into the vault is live"
    );
}

// ---------------------------------------------------------------------
// §3.3 (`docs/history/SIMPLIFY.md`): after creation, once more to open it; an
// empty vault's next steps.
// ---------------------------------------------------------------------

#[test]
fn leaving_unlock_drops_a_half_typed_passphrase() {
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    make_the_vault(&mut app);
    for way_out in [Action::Nav(Screen::Vaults), Action::Nav(Screen::Home)] {
        app.press(Action::Nav(Screen::Vaults));
        app.press(Action::Vault(V::Open(0)));
        assert_eq!(app.screen, Screen::Unlock);
        type_text(&mut app, "test phr");
        app.press(way_out);
        assert!(
            app.vaults.passphrase.text.is_empty(),
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
    // Light's 64 MiB, twice the 1 MiB file and 100 MiB for the app.
    app.storage(StorageEvent::Memory { available_mib: 165 });
    type_text(&mut app, NEW_PHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults.open.is_empty());
    assert!(app.vaults.open.is_empty(), "it unlocked short of memory");
    assert_eq!(
        app.vaults.unlock_error.as_deref(),
        Some("Needs 166 MiB. This computer has 165 MiB free.")
    );
    app.storage(StorageEvent::Memory { available_mib: 166 });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    unlock_with(&mut app, NEW_PHRASE);
}

#[test]
fn create_a_vault_lands_on_unlock_asking_for_the_passphrase_once_more() {
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    make_the_vault(&mut app);
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Type the passphrase once more to open it"),
        "Unlock says to type the passphrase once more"
    );
}

#[test]
fn an_empty_open_vault_offers_every_add_and_a_stick() {
    let mut app = device(Vec::new());
    app.apk_signing = true;
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    make_the_vault(&mut app);
    unlock_with(&mut app, NEW_PHRASE);
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
    assert!(
        app.offers(Action::Vault(V::WriteOut)),
        "Write it to a stick"
    );
    assert!(texts.iter().any(|t| t == "Write it to a stick"));
    app.press(Action::Vault(V::WriteOut));
    assert_eq!(app.screen, Screen::Files);
}
