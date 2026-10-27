//! A computer that does not say how much memory it has free
//! (`docs/VAULT.md` §3.1): a vault whose unlock memory is above Light's
//! is not unlocked and not made there, and Unlock and Create say why;
//! Light's, which opens anywhere, is.

use crate::common::type_text;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

const UNKNOWN: &str = "This computer does not say how much memory it has free";

/// A Light vault (64 MiB) of the test passphrase, and the same file with
/// its header stating `mib`: Unlock reads the cost from the header before
/// anything is decrypted.
fn vault_at(mib: u32) -> Vec<u8> {
    let c = testkit::compute();
    let mut bytes = faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        faraday_vault::Cost {
            memory_kib: 64 * 1024,
            passes: 1,
            lanes: 1,
        },
        &[b"memory phrase"],
        &[faraday_vault::Contents {
            records: Vec::new(),
        }],
        &[0x3c; 32],
    )
    .unwrap();
    bytes[5..9].copy_from_slice(&(mib * 1024).to_le_bytes());
    bytes
}

/// A computer that never says how much memory it has free.
fn silent(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
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
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::vault_speed(&mut app, Some(180));
    let _ = app.frame();
    app
}

fn try_unlock(app: &mut Faraday, name: &str) {
    app.press(Action::Nav(Screen::Vaults));
    let n = app
        .vault_files()
        .iter()
        .position(|f| f.name == name)
        .expect("the vault is listed");
    app.press(Action::Vault(V::Open(n)));
    type_text(app, "memory phrase");
    app.press(Action::Vault(V::Unlock));
    for t in 1..30u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

#[test]
fn memory_unknown_a_vault_above_light_is_not_unlocked_and_says_why() {
    let mut app = silent(vec![("big.ofv".into(), vault_at(256))]);
    try_unlock(&mut app, "big.ofv");
    assert!(app.vaults().open.is_empty(), "unlocked with memory unknown");
    let e = app.vaults().unlock_error.clone().unwrap_or_default();
    assert!(e.starts_with(UNKNOWN), "{e}");
}

#[test]
fn memory_unknown_a_light_vault_unlocks() {
    let mut app = silent(vec![("light.ofv".into(), vault_at(64))]);
    try_unlock(&mut app, "light.ofv");
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
}

#[test]
fn memory_unknown_a_vault_above_light_is_not_made_and_light_is_offered() {
    let mut app = silent(Vec::new());
    app.press(Action::Vault(V::Create));
    assert_eq!(app.screen, Screen::CreateVault);
    // Standard, 256 MiB.
    app.press(Action::Vault(V::CPreset(1)));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "memory phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, "memory phrase");
    app.press(Action::Vault(V::CGo));
    for t in 1..20u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert!(app.vaults().just_made.is_none(), "made with memory unknown");
    let e = app
        .vaults()
        .create
        .as_ref()
        .and_then(|c| c.error.clone())
        .unwrap_or_default();
    assert!(e.starts_with(UNKNOWN), "{e}");
    assert_eq!(
        app.vaults().suggested(),
        0,
        "Light is the size suggested for this computer"
    );
}
