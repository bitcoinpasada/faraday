//! The checklist's cards show what changed on the very next frame: a
//! vault opened while the checklist waits on it, holding what the plan
//! puts there, ticks its card (`docs/SPEED.md` §3.12).

use crate::common::{CLOCK, Shell, no_words, process, type_text};
use faraday_core::plan::Item;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent, bstep};
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
        .add_wallet(
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
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock { unix_secs: CLOCK });
    app.vaults.ms_per_unit = Some(180);
    app
}

/// The vault Create vault is open on made with `phrase`, then unlocked;
/// back where it was made from.
fn make_vault(app: &mut Faraday, phrase: &str) {
    assert_eq!(app.screen, Screen::CreateVault);
    let before = app.vaults.open.len();
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
        if app.vaults.open.len() > before {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults.open.len(),
        before + 1,
        "{phrase}: {:?} on {:?}",
        app.vaults.unlock_error,
        app.screen
    );
}

/// A drawn string and its box in pixels.
type TextBox = (String, (i32, i32, i32, i32));

/// The strings the next frame draws, with their boxes.
fn next_frame(app: &mut Faraday) -> Vec<TextBox> {
    app.settle();
    app.drawn_text_boxes()
}

/// Whether the card titled `title` shows a tick: its number, drawn left
/// of its title, is gone.
fn ticked(boxes: &[TextBox], title: &str, number: usize) -> bool {
    let (_, (x, y, _, _)) = boxes
        .iter()
        .find(|(t, _)| t == title)
        .unwrap_or_else(|| panic!("no card titled {title:?} in {boxes:?}"));
    !boxes
        .iter()
        .any(|(t, (bx, by, _, _))| *t == number.to_string() && bx < x && (by - y).abs() < 24)
}

#[test]
fn a_vault_opened_mid_checklist_holding_the_wallet_ticks_the_vault_2_card_on_the_next_frame() {
    let c = faraday_core::testkit::compute();
    // A vault made on an earlier power-on, holding seed 2 and the wallet.
    let mut shell = Shell::new();
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
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
        .filter(|i| i.name.ends_with(".ofv"))
        .map(|i| (i.name.clone(), i.bytes.clone()))
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
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    assert_eq!(app.backup.as_ref().unwrap().open, Some(bstep::VAULT));
    app.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    make_vault(&mut app, "first vault phrase");
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done_now(&c, Item::Vault(0)));
    app.press(Action::BNext(bstep::VAULT));
    let seed2 = app.backup_seed_list_now(w)[1].0.clone();
    let title = format!("Vault 2 · {seed2}");
    let number = app
        .backup_items_now(&c)
        .iter()
        .position(|&n| n == bstep::VAULT + 1)
        .expect("a Vault 2 item")
        + 1;
    let boxes = next_frame(&mut app);
    assert!(!ticked(&boxes, &title, number), "{boxes:?}");
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
        .position(|f| f.name == earlier_name)
        .expect("the earlier vault is listed");
    app.press(Action::Vault(V::Open(i)));
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, "earlier vault phrase");
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults.open.len() == 2 {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 2, "{:?}", app.vaults.unlock_error);
    // Its Load sheet put off: the vault stays open (F62).
    app.press(Action::Vault(V::LoadNotNow));
    app.press(Action::Nav(Screen::Backup));
    let boxes = next_frame(&mut app);
    assert!(ticked(&boxes, &title, number), "{boxes:?}");
    assert!(boxes.iter().any(|(t, _)| *t == into), "{boxes:?}");
    assert!(app.offers(Action::BNext(bstep::VAULT + 1)), "Continue");
    assert!(app.backup_item_done_now(&c, Item::Vault(1)));
}
