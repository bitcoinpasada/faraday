//! A file is a vault by its contents, whatever its name: the magic and
//! one of the vault lengths (`docs/VAULT-HARDENING.md` §2.1). A vault
//! under a short name or no extension opens from Files; a file named
//! `.fvlt` that holds other bytes is not a vault.

use crate::common::type_text;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, Event};

fn vault_bytes() -> Vec<u8> {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.fvlt")
        .unwrap()
        .1
}

fn with_inbox(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(180));
    app
}

fn kind_of(app: &Faraday, name: &str) -> FileKind {
    app.inbox().iter().find(|i| i.name == name).unwrap().kind
}

#[test]
fn a_vault_is_a_vault_under_any_name_and_a_fvlt_file_of_other_bytes_is_not() {
    let v = vault_bytes();
    let mut short_len = v.clone();
    short_len.truncate(v.len() - 1);
    let app = with_inbox(vec![
        ("VAULT~1.FVL".into(), v.clone()),
        ("vault".into(), v.clone()),
        ("x.fvlt".into(), b"some other bytes".to_vec()),
        ("short.fvlt".into(), short_len),
    ]);
    assert_eq!(kind_of(&app, "VAULT~1.FVL"), FileKind::Vault);
    assert_eq!(kind_of(&app, "vault"), FileKind::Vault);
    assert_ne!(kind_of(&app, "x.fvlt"), FileKind::Vault);
    assert_ne!(kind_of(&app, "short.fvlt"), FileKind::Vault);
}

#[test]
fn a_vault_with_no_extension_opens_from_files() {
    let mut app = with_inbox(vec![("vault".into(), vault_bytes())]);
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault did not unlock");
}
