//! A vault entry exported for KeePass: sealed under the passphrase typed
//! for it, listed with the sealed files, and a KDBX 4 file that passphrase
//! opens.

use crate::common::type_text;
use faraday_core::secrets::Exposure;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn settle(app: &mut Faraday, from: u64) {
    for t in from..from + 20 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

#[test]
fn an_entry_exported_for_keepass_is_sealed_under_its_passphrase() {
    let c = testkit::compute();
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
    // The answers a shell gives to the requests for randomness: the
    // session's seed, then eleven pooled.
    for i in 0..12u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
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
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Nav(Screen::VaultContents));
    // An entry of its own, then exported.
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Mail");
    app.press(Action::Vault(V::FocusField(2)));
    type_text(&mut app, "hunter22");
    app.press(Action::Vault(V::FormSave));
    let entries = app
        .vaults()
        .records(app.vaults().open[0].id)
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
    assert!(faraday_backup::kdbx::verify(
        &c,
        &file.bytes,
        b"keepass passphrase"
    ));
    assert!(!faraday_backup::kdbx::verify(&c, &file.bytes, b"another"));
    // Its randomness is the system's fresh bytes as they came, an answer
    // for every 32, the latest first: the master seed is the last
    // answer's, 0x4b × 32, taken as SHA-256("faraday fresh" ‖ "kdbx" ‖
    // answer), and the encryption IV the first 12 bytes of the one
    // before's (computed with Python's `hashlib`).
    assert_eq!(
        hex(header_field(&file.bytes, 4)),
        "3c654f400a8e486b60e50c4487b18f5f5e263fad2f571fbf1a4ff9460315d3f5"
    );
    assert_eq!(
        hex(header_field(&file.bytes, 7)),
        "7e8f080f045fb896678ccbee"
    );
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The value of KDBX 4 outer header field `id`: after the two signatures
/// and the version, each field is its id, a four-byte little-endian
/// length and the value.
fn header_field(file: &[u8], id: u8) -> &[u8] {
    let mut at = 12;
    loop {
        let len = u32::from_le_bytes(file[at + 1..at + 5].try_into().unwrap()) as usize;
        if file[at] == id {
            return &file[at + 5..at + 5 + len];
        }
        assert_ne!(file[at], 0, "no field {id}");
        at += 5 + len;
    }
}
