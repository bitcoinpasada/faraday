//! A vault entry exported for KeePass: sealed under the passphrase typed
//! for it, listed with the sealed files, and a KDBX 4 file that passphrase
//! opens.

use faraday_core::secrets::Exposure;
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

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

#[test]
fn an_entry_exported_for_keepass_is_sealed_under_its_passphrase() {
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
    // The answers a shell gives to the requests for randomness.
    for i in 0..6u8 {
        app.event(Event::Entropy(osk_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    assert_eq!(app.vaults.open.len(), 1);
    app.press(Action::Nav(Screen::VaultContents));
    // An entry of its own, then exported.
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Mail");
    app.press(Action::Vault(V::FocusField(2)));
    type_text(&mut app, "hunter22");
    app.press(Action::Vault(V::FormSave));
    let entries = app.vaults.open[0]
        .contents
        .of(faraday_vault::records::kind::ENTRY)
        .count();
    app.press(Action::Vault(V::Item(entries - 1)));
    app.press(Action::Vault(V::ExportKdbx));
    type_text(&mut app, "keepass passphrase");
    app.press(Action::Vault(V::PromptGo));
    settle(&mut app, 30);
    let file = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with(".kdbx"))
        .expect("no KeePass file");
    assert_eq!(file.kind.exposure(), Exposure::Sealed);
    assert!(osk_backup::kdbx::verify(&file.bytes, b"keepass passphrase"));
    assert!(!osk_backup::kdbx::verify(&file.bytes, b"another"));
}
