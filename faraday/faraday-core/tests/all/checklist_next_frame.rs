//! The checklist's cards show what changed on the very next frame: a
//! vault opened while the checklist waits on it, holding what the plan
//! puts there, ticks its card (`docs/SPEED.md` §3.12).

use crate::common::{CLOCK, Shell, no_words, process, type_text};
use faraday_core::plan::Item;
use faraday_core::testkit::Plain;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// Test keys 1 and 2 typed in and the 2-of-3 over the three test keys
/// loaded: two of its seeds here, the third a cosigner's. Returns the
/// wallet's index.
fn two_of_three_two_here(app: &mut Faraday) -> usize {
    let c = faraday_core::testkit::compute();
    for k in 0..2 {
        app.press(Action::Entry(None));
        type_text(
            app,
            &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[k].0),
        );
        app.press(Action::EntryAdd);
    }
    app.session
        .add_wallet_confirmed(
            &c,
            "Savings",
            &faraday_core::testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap()
}

/// A later power-on with `inbox` on its sticks, its own randomness, the
/// clock known; tall enough that every card is drawn.
fn later(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
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
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x60 + i; 32],
        )));
    }
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock { unix_secs: CLOCK });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app
}

/// The vault Create vault is open on made with `phrase`, then unlocked;
/// back where it was made from.
fn make_vault(app: &mut Faraday, phrase: &str) {
    assert_eq!(app.screen, Screen::CreateVault);
    let before = app.vaults().open.len();
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(app, phrase);
    }
    app.press(Action::Vault(V::CGo));
    for t in 1..60u64 {
        if app.screen == Screen::Unlock {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    for t in 60..120u64 {
        if app.vaults().open.len() > before {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        before + 1,
        "{phrase}: {:?} on {:?}",
        app.vaults().unlock_error,
        app.screen
    );
}

/// A drawn string and its box in pixels.
type TextBox = (String, (i32, i32, i32, i32));

/// The strings the next frame draws, with their boxes.
fn next_frame(app: &mut Faraday) -> Vec<TextBox> {
    app.settle();
    app.drawn_text_boxes().plain()
}

/// Whether Your choices says the item titled `title` is done.
fn ticked(boxes: &[TextBox], title: &str) -> bool {
    let (_, (px, _, _, _)) = boxes
        .iter()
        .find(|(t, _)| t == "Your choices")
        .unwrap_or_else(|| panic!("no Your choices in {boxes:?}"));
    let (_, (x, y, _, _)) = boxes
        .iter()
        .find(|(t, (bx, _, _, _))| t == title && bx >= px)
        .unwrap_or_else(|| panic!("no row titled {title:?} in {boxes:?}"));
    boxes
        .iter()
        .filter(|(_, (bx, by, _, _))| (bx - x).abs() < 4 && *by > *y && *by - *y < 30)
        .min_by_key(|(_, (_, by, _, _))| *by)
        .is_some_and(|(t, _)| t == "Done")
}

#[test]
fn a_vault_opened_mid_checklist_holding_the_wallet_ticks_the_vault_2_card_on_the_next_frame() {
    let c = faraday_core::testkit::compute();
    // A vault made on an earlier power-on, holding seed 2 and the wallet.
    let mut shell = Shell::new();
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    let w = two_of_three_two_here(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    make_vault(&mut app, "earlier vault phrase");
    app.press(Action::Vault(V::SaveKey(1)));
    app.press(Action::Vault(V::SaveWallet(w)));
    let app = shell.lock(&mut app);
    let earlier: Vec<(String, Vec<u8>)> = app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".kdbx"))
        .map(|i| (String::from(&*i.name), i.bytes.to_vec()))
        .collect();
    assert_eq!(earlier.len(), 1, "one vault sealed at the lock");
    let earlier_name = earlier[0].0.clone();

    // A later power-on, the vault's file on a stick: the same wallet's
    // checklist, Vault 1 made and filled here, Vault 2 waiting.
    let mut app = later(earlier);
    let w = two_of_three_two_here(&mut app);
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
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
    app.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    make_vault(&mut app, "first vault phrase");
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    app.press(Action::BNext(faraday_core::plan::Item::Vault(0)));
    let seed2 = String::from(&*app.backup_seed_list_now(w)[1].0);
    let title = format!("Vault 2 · {seed2}");
    let boxes = next_frame(&mut app);
    assert!(!ticked(&boxes, &title), "{boxes:?}");
    let into = format!("Into {earlier_name}");
    assert!(!boxes.iter().any(|(t, _)| *t == into), "{boxes:?}");
    assert!(
        app.offers(Action::BNewVault(1)),
        "the open vault holds Vault 1's seed: lock it and make a new one"
    );

    // The earlier vault opened from the Vaults page, then back to the
    // checklist: the very next frame ticks Vault 2's card, names the
    // vault on it and offers to go on.
    app.press(Action::Nav(Screen::Vaults));
    let i = app
        .vault_files()
        .iter()
        .position(|f| f.name.as_str() == earlier_name)
        .expect("the earlier vault is listed");
    app.press(Action::Vault(V::Open(i)));
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, "earlier vault phrase");
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() == 2 {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        2,
        "{:?}",
        app.vaults().unlock_error
    );
    // Its Load sheet put off: the vault stays open (F62).
    app.press(Action::Vault(V::LoadNotNow));
    app.press(Action::Nav(Screen::Backup));
    let boxes = next_frame(&mut app);
    assert!(ticked(&boxes, &title), "{boxes:?}");
    assert!(app.backup_item_done_now(&c, Item::Vault(1)));
    // Done, the item has moved on by itself (`docs/BACKUP-FLOW.md` §17.23).
    assert_ne!(
        app.backup().unwrap().open,
        Some(faraday_core::plan::Item::Vault(1))
    );
}
