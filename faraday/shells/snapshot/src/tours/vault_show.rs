//! The vault_show tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};
use faraday_shell_api::{App, Event};

/// The test vault's key and wallet shown and saved (`docs/BACKUP-FLOW.md`
/// §17.7): the key with its SeedQR held, its Save to a stick, and the
/// Spending wallet's Save to a stick.
pub(crate) fn vault_show_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_shell_api::TouchPhase;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    t.tick();
    crate::tour::not_now_load(t);
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(0)));
    t.press(Action::Vault(V::Item(0)));
    let show = Action::Vault(V::ShowSeedQr);
    let _ = t.app.frame();
    let (x, y) = t
        .app
        .where_offered(show)
        .ok_or("Show SeedQR is not on screen")?;
    t.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    t.shot("vault-key-seedqr")?;
    t.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    t.press(Action::Vault(V::SaveToStick));
    t.shot("vault-key-save")?;
    t.press(Action::Cancel);
    t.press(Action::Vault(V::Category(1)));
    let k = t
        .app
        .vaults()
        .records(t.app.vaults().open[0].id)
        .iter()
        .filter(|(_, r)| {
            [
                faraday_vault::records::kind::WALLET,
                faraday_vault::records::kind::WALLET_SHARE,
            ]
            .contains(&r.kind())
        })
        .position(|(_, r)| {
            r.public_text(faraday_vault::records::field::WALLET_NAME) == Some("Spending")
        })
        .ok_or("no Spending in the test vault")?;
    t.press(Action::Vault(V::Item(k)));
    t.press(Action::Vault(V::SaveToStick));
    t.shot("vault-wallet-save")
}
