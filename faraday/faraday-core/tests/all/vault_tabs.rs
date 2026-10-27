//! Vault contents' kinds (Bitcoin keys, Wallets, Entries, Notes), shown
//! one at a time as tabs on the Pi's panel (`docs/DESIGN.md` §5
//! VaultContents): each kind keeps where its list was scrolled to. A
//! person who scrolls the wallets, looks at the entries and comes back
//! finds the same place.

use crate::common::type_text;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

/// The test vault unlocked on the Pi's panel, Vault contents open.
fn contents() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    app.press(Action::ReduceMotion(true));
    app.storage(StorageEvent::Restored {
        inbox: vec![("vault.ofv".into(), testkit::test_vault().unwrap())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    crate::common::settle(&mut app, 100);
    if app.sheet == Some(faraday_core::Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadTicked));
    }
    assert_eq!(app.vaults().open.len(), 1, "the test vault is open");
    app.press(Action::Nav(Screen::VaultContents));
    app.settle();
    app
}

fn shown(app: &mut Faraday) -> Vec<(String, (i32, i32, i32, i32))> {
    let _ = app.frame();
    app.drawn_text_boxes()
}

#[test]
fn on_the_pi_each_kind_keeps_where_its_list_was_scrolled_to() {
    let mut app = contents();
    // Wallets, the longest list, scrolled down.
    app.press(Action::Vault(V::Category(1)));
    app.settle();
    let top = shown(&mut app);
    for _ in 0..3 {
        app.event(Event::Scroll {
            x: 240,
            y: 400,
            dy: 120,
        });
        app.settle();
    }
    let scrolled = shown(&mut app);
    assert_ne!(scrolled, top, "the wallets' list scrolls");
    // Entries, then Wallets again.
    app.press(Action::Vault(V::Category(2)));
    app.settle();
    let entries = shown(&mut app);
    app.press(Action::Vault(V::Category(1)));
    app.settle();
    assert_eq!(shown(&mut app), scrolled, "back on Wallets, the same place");
    // Entries, never scrolled, shows its top as it did.
    app.press(Action::Vault(V::Category(2)));
    app.settle();
    assert_eq!(shown(&mut app), entries, "Entries where it was left");
}
