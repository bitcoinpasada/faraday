//! The vault_show tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::vault::{SearchAct, SearchIn, VaultAction as V};
use faraday_core::{Action, Screen, StorageEvent};
use faraday_shell_api::{App, Event, TouchPhase};

/// The test vault's key and wallet shown and saved (`docs/BACKUP-FLOW.md`
/// §17.7): the key masked, then its words and SeedQR shown by their eyes,
/// its Save to a stick, and the Spending wallet's Save to a stick; then
/// the wallets searched, with a match and with none.
pub(crate) fn vault_show_tour(t: &mut Tour) -> Result<(), String> {
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.fvlt")
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
    // Masked, each secret behind its eye; then the words and the SeedQR
    // shown by a press on each eye, and hidden again by the next.
    t.shot("vault-key")?;
    t.press(Action::Vault(V::Reveal(0)));
    t.press(Action::Vault(V::ShowSeedQr));
    t.shot("vault-key-seedqr")?;
    t.press(Action::Vault(V::Reveal(0)));
    t.press(Action::Vault(V::ShowSeedQr));
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
    t.shot("vault-wallet-save")?;
    // The wallets searched (`docs/VAULT.md` §12).
    t.press(Action::Cancel);
    t.press(Action::Vault(V::Category(1)));
    // A tap on the bar, as a person makes it: on the small panel the page
    // is scrolled to it first, and the keyboard rising keeps it in view.
    let bar = Action::Vault(V::Search(SearchAct::Focus(SearchIn::Items)));
    let mut seen = 0;
    for _ in 0..40 {
        let _ = t.app.frame();
        if t.app.where_offered(bar).is_some() {
            seen += 1;
            if seen > 2 || t.app.hit_box(bar).is_some_and(|b| b.1 < 200) {
                break;
            }
        }
        t.app.event(Event::Scroll {
            x: 100,
            y: 200,
            dy: 30,
        });
        t.app.settle();
    }
    let (x, y) = t
        .app
        .where_offered(bar)
        .ok_or("the search bar is not on screen")?;
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        t.app.event(Event::Touch { x, y, phase });
    }
    type_text(t, "sav");
    t.shot("vault-wallets-search")?;
    type_text(t, "x");
    t.shot("vault-wallets-search-none")
}
