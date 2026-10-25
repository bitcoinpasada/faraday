//! The panes tour: the lists and the one chosen (`docs/LAYOUT.md` §5,
//! §17), Wallets, Files, Vaults with a vault's Unlock, Backups and
//! Settings, nothing chosen and one chosen.

use crate::tour::Tour;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};

/// Every list screen with the test wallets, the test vault and the test
/// stick's files.
pub(crate) fn panes_tour(t: &mut Tour) -> Result<(), String> {
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.load_kit()?;
    t.press(Action::Nav(Screen::Wallets));
    t.shot("panes-wallets")?;
    t.press(Action::PickWallet(2));
    t.shot("panes-wallets-chosen")?;
    t.press(Action::Nav(Screen::Files));
    t.shot("panes-files")?;
    t.press(Action::Nav(Screen::Vaults));
    t.shot("panes-vaults")?;
    t.press(Action::Vault(V::Open(0)));
    t.shot("panes-vault-unlock")?;
    t.press(Action::Nav(Screen::Backups));
    t.shot("panes-backups")?;
    t.press(Action::BackupsPage(1));
    t.shot("panes-backups-chosen")?;
    t.press(Action::Nav(Screen::Settings));
    t.shot("panes-settings")?;
    t.press(Action::SettingsSection(4));
    t.shot("panes-settings-appearance")
}
