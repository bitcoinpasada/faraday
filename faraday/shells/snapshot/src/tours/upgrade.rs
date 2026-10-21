//! The upgrade tour.

use faraday_core::{Action, BootPart, Screen, StickInfo, StorageEvent};
use crate::tour::Tour;

/// The Spend tab from a fresh start: the map, the envelope, the vault and
/// the PSBT copied in from a stick, the unlock, the wallet chosen, the
/// check, the transaction, test key 2 typed from the signing card, the
/// signature and the result.
/// Each theme on three screens, with the test kit loaded: the Wallets
/// tab's first page, the wallets, and Settings.
/// The desktop app's Transfer over a Downloads folder in the output
/// directory: the list newest first, a PSBT sent as codes, a file over
/// 256 KiB refused, and Receive saving a descriptor and a seed's words
/// that a device sent, each said as saved, the words as a secret.
/// Upgrade a Faraday stick (`docs/history/PLAN.md` §5.5), the boot copier's answers
/// given by hand.
pub(crate) fn upgrade_tour(t: &mut Tour) -> Result<(), String> {
    const RUNNING: &str = "6.6.84-faraday-0.2.0+4d0680b1a2b3";
    const MB: u64 = 1 << 20;
    let part = |id: &str, size: u64, release: Option<&str>, source: bool| BootPart {
        id: id.into(),
        size,
        release: release.map(str::to_string),
        source,
    };
    let source = part("sda1@sda#1", 48 * MB, Some(RUNNING), true);
    t.press(Action::Nav(Screen::Settings));
    t.shot("settings")?;
    // With a key loaded, the lock comes first.
    t.press(Action::Entry(None));
    t.type_key(0);
    t.press(Action::Nav(Screen::Settings));
    t.press(Action::UpgradeOpen);
    t.shot("upgrade-lock")?;
    t.press(Action::Lock);
    // The fresh process, given what the lock kept.
    t.app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: vec![("upgrade".into(), b"1".to_vec())],
    });
    t.pump();
    if t.app.screen != Screen::Upgrade {
        return Err("the fresh process did not open on the upgrade".into());
    }
    t.shot("upgrade-insert")?;
    t.app.storage(StorageEvent::Boots(vec![part(
        "sda1@sda#1",
        48 * MB,
        Some(RUNNING),
        false,
    )]));
    t.shot("upgrade-reading")?;
    t.app.storage(StorageEvent::BootSource {
        id: "sda1@sda#1".into(),
        release: RUNNING.into(),
        size: 48 * MB,
    });
    t.app.storage(StorageEvent::Boots(vec![source.clone()]));
    t.shot("upgrade-source-read")?;
    // The vault stick goes in: an older Faraday, its data partition with
    // a vault and the settings file.
    t.app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "sdb2@sdb#2".into(),
        label: "FARADAYDATA".into(),
        boot: false,
        files: vec![
            ("vault.ofv".into(), 16_778_240),
            ("faraday-settings.txt".into(), 120),
        ],
    }]));
    let older = part("sdb1@sdb#2", 48 * MB, None, false);
    t.app
        .storage(StorageEvent::Boots(vec![source.clone(), older.clone()]));
    t.shot("upgrade-older")?;
    // A newer one beside it.
    let newer = part(
        "sdc1@sdc#3",
        48 * MB,
        Some("6.6.84-faraday-0.3.0+0123456789ab"),
        false,
    );
    t.app.storage(StorageEvent::Boots(vec![
        source.clone(),
        older.clone(),
        newer,
    ]));
    t.press(Action::UpgradePick(1));
    t.shot("upgrade-newer")?;
    // One too small.
    t.app.storage(StorageEvent::Boots(vec![
        source.clone(),
        part("sdd1@sdd#4", 32 * MB, None, false),
    ]));
    t.shot("upgrade-too-small")?;
    // Pulled during the write.
    t.app
        .storage(StorageEvent::Boots(vec![source.clone(), older.clone()]));
    t.press(Action::UpgradeWrite);
    t.shot("upgrade-writing")?;
    t.app.storage(StorageEvent::BootWriteFailed {
        id: older.id.clone(),
        reason: "the stick was removed".into(),
        pulled: true,
    });
    t.shot("upgrade-pulled")?;
    // Put back, and written.
    t.press(Action::UpgradeWrite);
    t.app.storage(StorageEvent::BootWritten {
        id: older.id.clone(),
        release: RUNNING.into(),
    });
    t.app.storage(StorageEvent::Boots(vec![
        source,
        part("sdb1@sdb#2", 48 * MB, Some(RUNNING), false),
    ]));
    t.shot("upgrade-done")?;
    Ok(())
}
