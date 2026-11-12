//! The head of an open vault names the settings of a file a KeePass app
//! saved, and the version where the version is what differs.

use crate::common::type_text;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};

fn settings_of(file: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/kdbx")
        .join(file);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("vault.kdbx".into(), bytes)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.event(Event::Entropy(EntropyBytes::new([7; 32])));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "kdbx vector passphrase");
    app.press(Action::Vault(V::Unlock));
    for t in 1..120u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    app.vaults().open[0].settings_text().to_string()
}

#[test]
fn a_4_1_file_is_saved_in_a_named_version() {
    assert_eq!(
        settings_of("pykeepass-aes-argon2id-gzip-v41.kdbx"),
        "Saved in KDBX 4.1 / AES-256 / Argon2id / gzip"
    );
}

#[test]
fn a_4_0_file_names_no_version() {
    assert_eq!(
        settings_of("pykeepass-aes-argon2d-gzip.kdbx"),
        "Saved in AES-256 / Argon2d / gzip"
    );
}
