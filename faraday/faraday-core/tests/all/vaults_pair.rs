//! Vaults on the wide layout: this session's keys at the left, the vault
//! files and the row Make a new vault in the tile at the right.

use faraday_core::strings::EN;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const WORDS: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                      abandon abandon about";

fn shown(width: u16, height: u16) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// A 1366 by 768 window holding one locked vault file.
fn with_a_vault() -> Faraday {
    let mut app = shown(1366, 768);
    let file = testkit::kdbx_vault("Main", "pair phrase", 1, Vec::new()).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("main.kdbx".to_string(), file)]),
        outbox: faraday_core::files_from(Vec::<(String, Vec<u8>)>::new()),
        kept: faraday_core::kept_from(Vec::<(String, Vec<u8>)>::new()),
    });
    app
}

#[test]
fn the_pane_lists_the_sessions_keys_and_the_tile_ends_in_make_a_new_vault() {
    let c = faraday_core::testkit::compute();
    let mut app = with_a_vault();
    app.session.keys.add_words(&c, WORDS, "Seed", None).unwrap();
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == EN.session_group_keys), "Keys");
    assert!(
        texts.iter().any(|t| t == EN.vaults_make_a_vault),
        "Make row"
    );
    assert!(
        texts.iter().any(|t| t == EN.vault_unlock),
        "the file's Unlock"
    );
}

#[test]
fn make_a_new_vault_in_the_tile_opens_the_wizard_and_the_bar_has_no_button() {
    let mut app = with_a_vault();
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::Create)));
    // One press target for it: the row, not the row and a bar button.
    let n = app
        .drawn_texts()
        .iter()
        .filter(|t| *t == EN.vaults_make_a_vault)
        .count();
    assert_eq!(n, 1, "the row only");
    app.press(Action::Vault(V::Create));
    assert_eq!(app.screen, Screen::CreateVault);
}
