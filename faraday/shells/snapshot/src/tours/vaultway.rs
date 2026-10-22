//! The vaultway tour.

use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_shell_api::{App, Event, Key};

/// Tools' GPG key with no vault file, Create a vault made for it, and
/// back; then with the test vault locked, the tile again, Unlock for it,
/// and the GPG keys it comes back to.
pub(crate) fn vaultway_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::catalog::TILES;
    let gpg = TILES
        .iter()
        .position(|tile| tile.name == "GPG key")
        .ok_or("no GPG key tile")? as u8;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    // On a small panel the tile is found by typing, as far down as it is.
    let find = |t: &mut Tour| {
        t.press(Action::Nav(Screen::Catalog));
        if t.app.is_compact() {
            t.app.event(Event::Key(Key::Escape));
            type_text(t, "gpg");
        } else {
            scroll_to(t, Action::Catalog(gpg));
        }
    };
    find(t);
    t.shot("tools-no-vault")?;
    t.press(Action::Catalog(gpg));
    t.shot("create-then-gpg")?;
    t.press(Action::Vault(V::Back));
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    find(t);
    t.shot("tools-vault-locked")?;
    t.press(Action::Catalog(gpg));
    t.shot("unlock-for-gpg")?;
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    if t.app.screen != Screen::VaultContents {
        return Err("unlocking did not come back to the GPG keys".into());
    }
    t.shot("unlocked-gpg")?;
    Ok(())
}
